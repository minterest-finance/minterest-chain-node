#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode};
use frame_support::{decl_error, decl_event, decl_module, decl_storage, ensure, traits::Get};
use frame_system::{self as system, ensure_root};
use minterest_primitives::{Balance, CurrencyId, Rate};
use orml_traits::MultiCurrency;
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};
use sp_runtime::{
	traits::{CheckedDiv, CheckedMul, Zero},
	DispatchError, DispatchResult, FixedPointNumber, RuntimeDebug,
};
use sp_std::{cmp::Ordering, convert::TryInto, result};

#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(Encode, Decode, RuntimeDebug, Eq, PartialEq, Default)]
pub struct ControllerData<BlockNumber> {
	/// Block number that interest was last accrued at.
	pub timestamp: BlockNumber,
	pub borrow_rate: Rate,
	pub insurance_factor: Rate,
	pub max_borrow_rate: Rate,
	/// Determines how much a user can borrow.
	pub collateral_factor: Rate,
}

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

type LiquidityPools<T> = liquidity_pools::Module<T>;
type Oracle<T> = oracle::Module<T>;

pub trait Trait: liquidity_pools::Trait + system::Trait + oracle::Trait {
	/// The overarching event type.
	type Event: From<Event> + Into<<Self as system::Trait>::Event>;

	/// Start exchange rate
	type InitialExchangeRate: Get<Rate>;

	/// Wrapped currency IDs.
	type MTokensId: Get<Vec<CurrencyId>>;
}

decl_event! {
	pub enum Event {
		InsuranceFactorChanged,
	}
}

decl_storage! {
	trait Store for Module<T: Trait> as ControllerStorage {
		pub ControllerDates get(fn controller_dates) config(): map hasher(blake2_128_concat) CurrencyId => ControllerData<T::BlockNumber>;
	}
}

decl_error! {
	pub enum Error for Module<T: Trait> {
		/// Number overflow in calculation.
		NumOverflow,

		/// Operations with this underlying assets are locked by the administrator
		OperationsLocked,

		/// Borrow rate is absurdly high.
		BorrowRateIsTooHight,

		/// The currency is not enabled in protocol.
		NotValidUnderlyingAssetId,

		/// The currency is not enabled in wrapped protocol.
		NotValidWrappedTokenId,

		/// An internal Oracle error has occurred.
		OraclePriceError,

		/// There is not enough liquidity available in the pool.
		InsufficientLiquidity
	}
}

decl_module! {
	pub struct Module<T: Trait> for enum Call where origin: T::Origin {
		type Error = Error<T>;
		fn deposit_event() = default;

		#[weight = 10_000]
		pub fn set_insurance_factor(origin, pool_id: CurrencyId, new_amount_n: u128, new_amount_d: u128) -> DispatchResult {
			ensure_root(origin)?;
			ensure!(new_amount_d > 0, Error::<T>::NumOverflow);
			let new_insurance_factor = Rate::saturating_from_rational(new_amount_n, new_amount_d);
			ControllerDates::<T>::mutate(pool_id, |r| r.insurance_factor = new_insurance_factor);
			Self::deposit_event(Event::InsuranceFactorChanged);
			Ok(())
		}

		#[weight = 10_000]
		pub fn set_max_borrow_rate(origin, pool_id: CurrencyId, new_amount_n: u128, new_amount_d: u128) -> DispatchResult {
			ensure_root(origin)?;
			ensure!(new_amount_d > 0, Error::<T>::NumOverflow);
			let new_max_borow_rate = Rate::saturating_from_rational(new_amount_n, new_amount_d);
			ControllerDates::<T>::mutate(pool_id, |r| r.max_borrow_rate = new_max_borow_rate);
			Self::deposit_event(Event::InsuranceFactorChanged);
			Ok(())
		}
	}
}

type RateResult = result::Result<Rate, DispatchError>;
type BalanceResult = result::Result<Balance, DispatchError>;
type LiquidityResult = result::Result<(Balance, Balance), DispatchError>;
type CurrencyIdResult = result::Result<CurrencyId, DispatchError>;

impl<T: Trait> Module<T> {
	// Used in controller: do_deposit, do_redeem, do_borrow, do_repay
	pub fn accrue_interest_rate(underlying_asset_id: CurrencyId) -> DispatchResult {
		ensure!(
			!<LiquidityPools<T>>::pools(&underlying_asset_id).is_lock,
			Error::<T>::OperationsLocked
		);

		//Remember the initial block number
		let current_block_number = <frame_system::Module<T>>::block_number();
		let accrual_block_number_previous = Self::controller_dates(underlying_asset_id).timestamp;

		//Short-circuit accumulating 0 interest
		if current_block_number == accrual_block_number_previous {
			return Ok(());
		}

		let current_total_balance = <LiquidityPools<T>>::get_pool_available_liquidity(underlying_asset_id);
		let current_total_borrowed_balance = <LiquidityPools<T>>::get_pool_total_borrowed(underlying_asset_id);
		let current_total_insurance = <LiquidityPools<T>>::get_pool_total_insurance(underlying_asset_id);
		let _current_borrow_index: Rate; // FIXME: how can i use it?

		// Calculate the current borrow interest rate
		let current_borrow_interest_rate = Self::calculate_borrow_interest_rate(
			current_total_balance,
			current_total_borrowed_balance,
			current_total_insurance,
		)?;

		let max_borrow_rate = ControllerDates::<T>::get(underlying_asset_id).max_borrow_rate;
		let insurance_factor = ControllerDates::<T>::get(underlying_asset_id).insurance_factor;

		ensure!(
			current_borrow_interest_rate <= max_borrow_rate,
			Error::<T>::BorrowRateIsTooHight
		);

		// Calculate the number of blocks elapsed since the last accrual
		let block_delta = Self::calculate_block_delta(current_block_number, accrual_block_number_previous)?;

		/*
		Calculate the interest accumulated into borrows and insurance and the new index:
			*  simpleInterestFactor = borrowRate * blockDelta
			*  interestAccumulated = simpleInterestFactor * totalBorrows
			*  totalBorrowsNew = interestAccumulated + totalBorrows
			*  totalInsuranceNew = interestAccumulated * insuranceFactor + totalInsurance
			*  borrowIndexNew = simpleInterestFactor * borrowIndex + borrowIndex
		*/

		let simple_interest_factor = Self::calculate_interest_factor(current_borrow_interest_rate, &block_delta)?; //FIXME: function returns 0, unusual behavior
		let interest_accumulated =
			Self::calculate_interest_accumulated(simple_interest_factor, current_total_borrowed_balance)?;
		let new_total_borrow_balance =
			Self::calculate_new_total_borrow(interest_accumulated, current_total_borrowed_balance)?;
		let new_total_insurance =
			Self::calculate_new_total_insurance(interest_accumulated, insurance_factor, current_total_insurance)?;
		let _new_borrow_index: Rate; // FIXME: how can i use it?

		// Save new params
		ControllerDates::<T>::mutate(underlying_asset_id, |x| x.timestamp = current_block_number);
		<LiquidityPools<T>>::set_accrual_interest_params(
			underlying_asset_id,
			new_total_borrow_balance,
			new_total_insurance,
		)?;

		Ok(())
	}

	// Used in controller: do_deposit, do_redeem
	pub fn convert_to_wrapped(underlying_asset_id: CurrencyId, underlying_amount: Balance) -> BalanceResult {
		let exchange_rate = Self::get_exchange_rate(underlying_asset_id)?;

		let wrapped_amount = Rate::from_inner(underlying_amount)
			.checked_div(&exchange_rate)
			.map(|x| x.into_inner())
			.ok_or(Error::<T>::NumOverflow)?;

		Ok(wrapped_amount)
	}

	// Used in controller: do_redeem
	pub fn convert_from_wrapped(wrapped_id: CurrencyId, wrapped_amount: Balance) -> BalanceResult {
		let underlying_asset_id = Self::get_underlying_asset_id_by_wrapped_id(&wrapped_id)?;
		let exchange_rate = Self::get_exchange_rate(underlying_asset_id)?;

		let underlying_amount = Rate::from_inner(wrapped_amount)
			.checked_mul(&exchange_rate)
			.map(|x| x.into_inner())
			.ok_or(Error::<T>::NumOverflow)?;

		Ok(underlying_amount)
	}

	// Not used yet
	pub fn calculate_interest_rate(_underlying_asset_id: CurrencyId) -> RateResult {
		//FIXME
		Ok(Rate::saturating_from_rational(1, 1)) //100%
	}

	/// Return the borrow balance of account based on stored data.
	///
	/// - `who`: the address whose balance should be calculated.
	/// - `currency_id`: id of the currency, the balance of borrowing of which we calculate.
	pub fn borrow_balance_stored(who: &T::AccountId, underlying_asset_id: CurrencyId) -> BalanceResult {
		let user_borrow_balance = <LiquidityPools<T>>::get_user_total_borrowed(&who, underlying_asset_id);

		// If borrow_balance = 0 then borrow_index is likely also 0.
		// Rather than failing the calculation with a division by 0, we immediately return 0 in this case.
		if user_borrow_balance == 0 {
			return Ok(Balance::zero());
		};

		let pool_borrow_index = <LiquidityPools<T>>::get_pool_borrow_index(underlying_asset_id);
		let user_borrow_index = <LiquidityPools<T>>::get_user_borrow_index(&who, underlying_asset_id);

		// Calculate new borrow balance using the borrow index:
		// recent_borrow_balance = user_borrow_balance * pool_borrow_index / user_borrow_index
		let principal_times_index = Rate::from_inner(user_borrow_balance)
			.checked_mul(&pool_borrow_index)
			.map(|x| x.into_inner())
			.ok_or(Error::<T>::NumOverflow)?;

		let result = Rate::from_inner(principal_times_index)
			.checked_div(&user_borrow_index)
			.map(|x| x.into_inner())
			.ok_or(Error::<T>::NumOverflow)?;

		Ok(result)
	}

	/// Determine what the account liquidity would be if the given amounts were redeemed/borrowed.
	///
	/// - `account`: The account to determine liquidity.
	/// - `underlying_asset_id`: The pool to hypothetically redeem/borrow.
	/// - `redeem_amount`: The number of tokens to hypothetically redeem.
	/// - `borrow_amount`: The amount of underlying to hypothetically borrow.
	/// Returns (hypothetical account liquidity in excess of collateral requirements,
	/// 		 hypothetical account shortfall below collateral requirements).
	pub fn get_hypothetical_account_liquidity(
		account: &T::AccountId,
		underlying_asset_id: CurrencyId,
		redeem_amount: Balance,
		borrow_amount: Balance,
	) -> LiquidityResult {
		let m_tokens_ids = T::MTokensId::get();

		let mut sum_collateral = Balance::zero();
		let mut sum_borrow_plus_effects = Balance::zero();

		// For each tokens the account is in
		for asset in m_tokens_ids.into_iter() {
			let underlying_asset = Self::get_underlying_asset_id_by_wrapped_id(&asset)?;
			let m_token_balance = T::MultiCurrency::free_balance(asset, account);
			if m_token_balance == Balance::zero() {
				continue;
			}

			// Read the balances and exchange rate from the cToken
			let borrow_balance = Self::borrow_balance_stored(account, underlying_asset)?;
			let exchange_rate = Self::get_exchange_rate(underlying_asset)?;
			let collateral_factor = Self::get_collateral_factor(underlying_asset);

			// Get the normalized price of the asset.
			let oracle_price =
				<Oracle<T>>::get_underlying_price(underlying_asset).map_err(|_| Error::<T>::OraclePriceError)?;

			// Pre-compute a conversion factor from tokens -> dollars (normalized price value)
			let tokens_to_denom = collateral_factor
				.checked_mul(&exchange_rate)
				.ok_or(Error::<T>::NumOverflow)?
				.checked_mul(&oracle_price)
				.ok_or(Error::<T>::NumOverflow)?;

			// sum_collateral += tokens_to_denom * m_token_balance
			sum_collateral =
				Self::mul_price_and_balance_add_to_prev_value(sum_collateral, m_token_balance, tokens_to_denom)?;

			// sum_borrow_plus_effects += oracle_price * borrow_balance
			sum_borrow_plus_effects =
				Self::mul_price_and_balance_add_to_prev_value(sum_borrow_plus_effects, borrow_balance, oracle_price)?;

			// Calculate effects of interacting with Underlying Asset Modify
			if underlying_asset_id == underlying_asset {
				// redeem effect
				// sum_borrow_plus_effects += tokens_to_denom * redeem_tokens
				sum_borrow_plus_effects = Self::mul_price_and_balance_add_to_prev_value(
					sum_borrow_plus_effects,
					redeem_amount,
					tokens_to_denom,
				)?;

				// borrow effect
				// sum_borrow_plus_effects += oracle_price * borrow_amount
				sum_borrow_plus_effects = Self::mul_price_and_balance_add_to_prev_value(
					sum_borrow_plus_effects,
					borrow_amount,
					oracle_price,
				)?;
			}
		}

		return match sum_collateral.cmp(&sum_borrow_plus_effects) {
			Ordering::Greater => Ok((
				sum_collateral
					.checked_sub(sum_borrow_plus_effects)
					.ok_or(Error::<T>::InsufficientLiquidity)?,
				0,
			)),
			_ => Ok((
				0,
				sum_borrow_plus_effects
					.checked_sub(sum_collateral)
					.ok_or(Error::<T>::InsufficientLiquidity)?,
			)),
		};
	}
}

// Private methods
impl<T: Trait> Module<T> {
	// Used in: convert_to_wrapped
	fn get_exchange_rate(underlying_asset_id: CurrencyId) -> RateResult {
		let wrapped_asset_id = Self::get_wrapped_id_by_underlying_asset_id(&underlying_asset_id)?;
		// The total amount of cash the market has
		let total_cash = <LiquidityPools<T>>::get_pool_available_liquidity(underlying_asset_id);

		// Total number of tokens in circulation
		let total_supply = T::MultiCurrency::total_issuance(wrapped_asset_id);

		let current_exchange_rate = Self::calculate_exchange_rate(total_cash, total_supply)?;

		<LiquidityPools<T>>::set_current_exchange_rate(underlying_asset_id, current_exchange_rate)?;

		Ok(current_exchange_rate)
	}

	//Used in: get_exchange_rate
	fn calculate_exchange_rate(total_cash: Balance, total_supply: Balance) -> RateResult {
		let rate = match total_supply.cmp(&Balance::zero()) {
			Ordering::Equal => T::InitialExchangeRate::get(),
			_ => Rate::saturating_from_rational(total_cash, total_supply),
		};

		Ok(rate)
	}

	fn calculate_borrow_interest_rate(
		_current_total_balance: Balance,
		_current_total_borrowed_balance: Balance,
		_current_total_insurance: Balance,
	) -> RateResult {
		// FIXME
		Ok(Rate::saturating_from_rational(1, 1))
	}

	fn calculate_block_delta(
		current_block_number: T::BlockNumber,
		accrual_block_number_previous: T::BlockNumber,
	) -> result::Result<T::BlockNumber, DispatchError> {
		ensure!(
			current_block_number >= accrual_block_number_previous,
			Error::<T>::NumOverflow
		);

		Ok(current_block_number - accrual_block_number_previous)
	}

	// simpleInterestFactor = borrowRate * blockDelta
	fn calculate_interest_factor(
		current_borrow_interest_rate: Rate,
		block_delta: &<T as system::Trait>::BlockNumber,
	) -> RateResult {
		let block_delta_as_usize = TryInto::try_into(*block_delta)
			.ok()
			.expect("blockchain will not exceed 2^32 blocks; qed");

		// FIXME: unusual behavior, we still need interest_factor = 0. To Fix: delete conditional operator
		let interest_factor = match block_delta_as_usize.cmp(&0) {
			Ordering::Greater => Rate::from_inner(0),
			_ => Rate::saturating_from_rational(block_delta_as_usize as u128, 1)
				.checked_mul(&current_borrow_interest_rate)
				.ok_or(Error::<T>::NumOverflow)?,
		};

		Ok(interest_factor)
	}

	// interestAccumulated = simpleInterestFactor * totalBorrows
	fn calculate_interest_accumulated(
		simple_interest_factor: Rate,
		current_total_borrowed_balance: Balance,
	) -> BalanceResult {
		let interest_accumulated = Rate::from_inner(current_total_borrowed_balance)
			.checked_mul(&simple_interest_factor)
			.map(|x| x.into_inner())
			.ok_or(Error::<T>::NumOverflow)?;

		Ok(interest_accumulated)
	}

	// totalBorrowsNew = interestAccumulated + totalBorrows
	fn calculate_new_total_borrow(
		interest_accumulated: Balance,
		current_total_borrowed_balance: Balance,
	) -> BalanceResult {
		let new_total_borrows = interest_accumulated
			.checked_add(current_total_borrowed_balance)
			.ok_or(Error::<T>::NumOverflow)?;

		Ok(new_total_borrows)
	}

	// totalInsuranceNew = interestAccumulated * insuranceFactor + totalInsurance
	fn calculate_new_total_insurance(
		interest_accumulated: Balance,
		insurance_factor: Rate,
		current_total_insurance: Balance,
	) -> BalanceResult {
		let insurance_accumulated = Rate::from_inner(interest_accumulated)
			.checked_mul(&insurance_factor)
			.map(|x| x.into_inner())
			.ok_or(Error::<T>::NumOverflow)?;

		let total_insurance_new = insurance_accumulated
			.checked_add(current_total_insurance)
			.ok_or(Error::<T>::NumOverflow)?;

		Ok(total_insurance_new)
	}

	/// Returning: value += balance_scalar * rate_scalar
	fn mul_price_and_balance_add_to_prev_value(
		value: Balance,
		balance_scalar: Balance,
		rate_scalar: Rate,
	) -> BalanceResult {
		let result = value
			.checked_add(
				Rate::from_inner(balance_scalar)
					.checked_mul(&rate_scalar)
					.map(|x| x.into_inner())
					.ok_or(Error::<T>::NumOverflow)?,
			)
			.ok_or(Error::<T>::NumOverflow)?;
		Ok(result)
	}

	fn get_wrapped_id_by_underlying_asset_id(asset_id: &CurrencyId) -> CurrencyIdResult {
		match asset_id {
			CurrencyId::DOT => Ok(CurrencyId::MDOT),
			CurrencyId::KSM => Ok(CurrencyId::MKSM),
			CurrencyId::BTC => Ok(CurrencyId::MBTC),
			CurrencyId::ETH => Ok(CurrencyId::METH),
			_ => Err(Error::<T>::NotValidUnderlyingAssetId.into()),
		}
	}

	fn get_underlying_asset_id_by_wrapped_id(wrapped_id: &CurrencyId) -> CurrencyIdResult {
		match wrapped_id {
			CurrencyId::MDOT => Ok(CurrencyId::DOT),
			CurrencyId::MKSM => Ok(CurrencyId::KSM),
			CurrencyId::MBTC => Ok(CurrencyId::BTC),
			CurrencyId::METH => Ok(CurrencyId::ETH),
			_ => Err(Error::<T>::NotValidWrappedTokenId.into()),
		}
	}
}

// Getters for LiquidityPools
impl<T: Trait> Module<T> {
	/// Determines how much a user can borrow.
	fn get_collateral_factor(pool_id: CurrencyId) -> Rate {
		Self::controller_dates(pool_id).collateral_factor
	}
}
