#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode};
use frame_support::{decl_error, decl_event, decl_module, decl_storage, ensure, traits::Get};
use frame_system::{self as system, ensure_signed};
use minterest_primitives::{Balance, CurrencyId, Operation, Price, Rate};
use orml_traits::MultiCurrency;
use orml_utilities::with_transaction_result;
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};
use sp_runtime::{
	traits::{CheckedAdd, CheckedDiv, CheckedMul, Zero},
	DispatchError, DispatchResult, FixedPointNumber, RuntimeDebug,
};
use sp_std::{cmp::Ordering, convert::TryInto, prelude::Vec, result};

#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(Encode, Decode, RuntimeDebug, Eq, PartialEq, Default)]
pub struct ControllerData<BlockNumber> {
	/// Block number that interest was last accrued at.
	pub timestamp: BlockNumber,
	pub borrow_rate: Rate,
	pub insurance_factor: Rate,
	pub max_borrow_rate: Rate,
	pub kink: Rate,
	pub base_rate_per_block: Rate,
	pub multiplier_per_block: Rate,
	pub jump_multiplier_per_block: Rate,
	/// Determines how much a user can borrow.
	pub collateral_factor: Rate,
}

/// The Administrator can pause certain actions as a safety mechanism.
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(Encode, Decode, RuntimeDebug, Eq, PartialEq, Default)]
pub struct PauseKeeper {
	pub deposit_paused: bool,
	pub redeem_paused: bool,
	pub borrow_paused: bool,
	pub repay_paused: bool,
}

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

type LiquidityPools<T> = liquidity_pools::Module<T>;
type Oracle<T> = oracle::Module<T>;
type Accounts<T> = accounts::Module<T>;

pub trait Trait: liquidity_pools::Trait + system::Trait + oracle::Trait + accounts::Trait {
	/// The overarching event type.
	type Event: From<Event> + Into<<Self as system::Trait>::Event>;

	/// Start exchange rate
	type InitialExchangeRate: Get<Rate>;

	/// The approximate number of blocks per year
	type BlocksPerYear: Get<u128>;

	/// Wrapped currency IDs.
	type UnderlyingAssetId: Get<Vec<CurrencyId>>;

	/// Wrapped currency IDs.
	type MTokensId: Get<Vec<CurrencyId>>;
}

decl_event! {
	pub enum Event {
		/// InsuranceFactor has been successfully changed
		InsuranceFactorChanged,

		/// JumpMultiplierPerBlock has been successfully changed
		JumpMultiplierPerBlockHasChanged,

		/// BaseRatePerBlock has been successfully changed
		BaseRatePerBlockHasChanged,

		/// MultiplierPerBlock has been successfully changed
		MultiplierPerBlockHasChanged,

		/// The operation is paused: \[pool_id, operation\]
		OperationIsPaused(CurrencyId, Operation),

		/// The operation is unpaused: \[pool_id, operation\]
		OperationIsUnPaused(CurrencyId, Operation),

		/// Insurance balance replenished: \[pool_id, amount\]
		DepositedInsurance(CurrencyId, Balance),

		/// Insurance balance redeemed: \[pool_id, amount\]
		RedeemedInsurance(CurrencyId, Balance),
	}
}

decl_storage! {
	trait Store for Module<T: Trait> as ControllerStorage {
		pub ControllerDates get(fn controller_dates) config(): map hasher(blake2_128_concat) CurrencyId => ControllerData<T::BlockNumber>;
		pub PauseKeepers get(fn pause_keepers) config(): map hasher(blake2_128_concat) CurrencyId => PauseKeeper;
	}
}

decl_error! {
	pub enum Error for Module<T: Trait> {
		/// Number overflow in calculation.
		NumOverflow,

		/// Borrow rate is absurdly high.
		BorrowRateIsTooHight,

		/// The currency is not enabled in protocol.
		NotValidUnderlyingAssetId,

		/// The currency is not enabled in wrapped protocol.
		NotValidWrappedTokenId,

		/// Oracle unavailable or price equal 0ю
		OraclePriceError,

		/// Insufficient available liquidity.
		InsufficientLiquidity,

		/// Pool not found.
		PoolNotFound,

		/// The dispatch origin of this call must be Administrator.
		RequireAdmin,

		/// Not enough balance to withdraw or repay.
		NotEnoughBalance,

		/// Balance overflows maximum.
		///
		/// Only happened when the balance went wrong and balance overflows the integer type.
		BalanceOverflowed,

		/// Operation (deposit, redeem, borrow, repay) is paused.
		OperationPaused,
	}
}

// Admin functions
decl_module! {
	pub struct Module<T: Trait> for enum Call where origin: T::Origin {
		type Error = Error<T>;
		fn deposit_event() = default;

		/// Pause specific operation (deposit, redeem, borrow, repay) with the pool.
		///
		/// The dispatch origin of this call must be Administrator.
		#[weight = 0]
		pub fn pause_specific_operation(origin, pool_id: CurrencyId, operation: Operation) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			ensure!(<Accounts<T>>::is_admin_internal(&sender), Error::<T>::RequireAdmin);
			ensure!(<LiquidityPools<T>>::pool_exists(&pool_id), Error::<T>::PoolNotFound);
			match operation {
				Operation::Deposit => PauseKeepers::mutate(pool_id, |pool| pool.deposit_paused = true),
				Operation::Redeem => PauseKeepers::mutate(pool_id, |pool| pool.redeem_paused = true),
				Operation::Borrow => PauseKeepers::mutate(pool_id, |pool| pool.borrow_paused = true),
				Operation::Repay => PauseKeepers::mutate(pool_id, |pool| pool.repay_paused = true),
			};
			Self::deposit_event(Event::OperationIsPaused(pool_id, operation));
			Ok(())
		}

		/// Unpause specific operation (deposit, redeem, borrow, repay) with the pool.
		///
		/// The dispatch origin of this call must be Administrator.
		#[weight = 0]
		pub fn unpause_specific_operation(origin, pool_id: CurrencyId, operation: Operation) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			ensure!(<Accounts<T>>::is_admin_internal(&sender), Error::<T>::RequireAdmin);
			ensure!(<LiquidityPools<T>>::pool_exists(&pool_id), Error::<T>::PoolNotFound);
			match operation {
				Operation::Deposit => PauseKeepers::mutate(pool_id, |pool| pool.deposit_paused = false),
				Operation::Redeem => PauseKeepers::mutate(pool_id, |pool| pool.redeem_paused = false),
				Operation::Borrow => PauseKeepers::mutate(pool_id, |pool| pool.borrow_paused = false),
				Operation::Repay => PauseKeepers::mutate(pool_id, |pool| pool.repay_paused = false),
			};
			Self::deposit_event(Event::OperationIsUnPaused(pool_id, operation));
			Ok(())
		}

		/// Replenishes the insurance balance.
		///
		/// The dispatch origin of this call must be Administrator.
		#[weight = 0]
		pub fn deposit_insurance(origin, pool_id: CurrencyId, #[compact] amount: Balance) {
			with_transaction_result(|| {
				let sender = ensure_signed(origin)?;
				ensure!(<Accounts<T>>::is_admin_internal(&sender), Error::<T>::RequireAdmin);
				Self::do_deposit_insurance(&sender, pool_id, amount)?;
				Self::deposit_event(Event::DepositedInsurance(pool_id, amount));
				Ok(())
			})?;
		}

		/// Redeem the insurance balance.
		///
		/// The dispatch origin of this call must be Administrator.
		#[weight = 0]
		pub fn redeem_insurance(origin, pool_id: CurrencyId, #[compact] amount: Balance) {
			with_transaction_result(|| {
				let sender = ensure_signed(origin)?;
				ensure!(<Accounts<T>>::is_admin_internal(&sender), Error::<T>::RequireAdmin);
				Self::do_redeem_insurance(&sender, pool_id, amount)?;
				Self::deposit_event(Event::RedeemedInsurance(pool_id, amount));
				Ok(())
			})?;
		}

		/// Set insurance factor.
		///
		/// The dispatch origin of this call must be Administrator.
		#[weight = 0]
		pub fn set_insurance_factor(origin, pool_id: CurrencyId, new_amount_n: u128, new_amount_d: u128) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			ensure!(<Accounts<T>>::is_admin_internal(&sender), Error::<T>::RequireAdmin);
			ensure!(<LiquidityPools<T>>::pool_exists(&pool_id), Error::<T>::PoolNotFound);
			ensure!(new_amount_d > 0, Error::<T>::NumOverflow);

			let new_insurance_factor = Rate::saturating_from_rational(new_amount_n, new_amount_d);

			ControllerDates::<T>::mutate(pool_id, |r| r.insurance_factor = new_insurance_factor);
			Self::deposit_event(Event::InsuranceFactorChanged);
			Ok(())
		}

		/// Set Maximum borrow rate.
		///
		/// The dispatch origin of this call must be Administrator.
		#[weight = 0]
		pub fn set_max_borrow_rate(origin, pool_id: CurrencyId, new_amount_n: u128, new_amount_d: u128) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			ensure!(<Accounts<T>>::is_admin_internal(&sender), Error::<T>::RequireAdmin);
			ensure!(<LiquidityPools<T>>::pool_exists(&pool_id), Error::<T>::PoolNotFound);
			ensure!(new_amount_d > 0, Error::<T>::NumOverflow);

			let new_max_borow_rate = Rate::saturating_from_rational(new_amount_n, new_amount_d);

			ControllerDates::<T>::mutate(pool_id, |r| r.max_borrow_rate = new_max_borow_rate);
			Self::deposit_event(Event::InsuranceFactorChanged);
			Ok(())
		}

		/// Set BaseRatePerBlock from BaseRatePerYear.
		///
		/// The dispatch origin of this call must be Administrator.
		#[weight = 0]
		pub fn set_base_rate_per_block(origin, pool_id: CurrencyId, base_rate_per_year_n: u128, base_rate_per_year_d: u128) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			ensure!(<Accounts<T>>::is_admin_internal(&sender), Error::<T>::RequireAdmin);
			ensure!(<LiquidityPools<T>>::pool_exists(&pool_id), Error::<T>::PoolNotFound);
			ensure!(base_rate_per_year_d > 0, Error::<T>::NumOverflow);

			let new_base_rate_per_year = Rate::saturating_from_rational(base_rate_per_year_n, base_rate_per_year_d);
			let new_base_rate_per_block = new_base_rate_per_year
				.checked_div(&Rate::from_inner(T::BlocksPerYear::get()))
				.ok_or(Error::<T>::NumOverflow)?;

			ControllerDates::<T>::mutate(pool_id, |r| r.base_rate_per_block = new_base_rate_per_block);
			Self::deposit_event(Event::BaseRatePerBlockHasChanged);
			Ok(())
		}

		/// Set MultiplierPerBlock from MultiplierPerYear.
		///
		/// The dispatch origin of this call must be Administrator.
		#[weight = 0]
		pub fn set_multiplier_per_block(origin, pool_id: CurrencyId, multiplier_rate_per_year_n: u128, multiplier_rate_per_year_d: u128) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			ensure!(<Accounts<T>>::is_admin_internal(&sender), Error::<T>::RequireAdmin);
			ensure!(<LiquidityPools<T>>::pool_exists(&pool_id), Error::<T>::PoolNotFound);
			ensure!(multiplier_rate_per_year_d > 0, Error::<T>::NumOverflow);

			let new_multiplier_per_year = Rate::saturating_from_rational(multiplier_rate_per_year_n, multiplier_rate_per_year_d);
			let new_multiplier_per_block = new_multiplier_per_year
				.checked_div(&Rate::from_inner(T::BlocksPerYear::get()))
				.ok_or(Error::<T>::NumOverflow)?;

			ControllerDates::<T>::mutate(pool_id, |r| r.multiplier_per_block = new_multiplier_per_block);
			Self::deposit_event(Event::MultiplierPerBlockHasChanged);
			Ok(())
		}

		/// Set JumpMultiplierPerBlock from JumpMultiplierPerYear.
		///
		/// The dispatch origin of this call must be Administrator.
		#[weight = 0]
		pub fn set_jump_multiplier_per_block(origin, pool_id: CurrencyId, jump_multiplier_rate_per_year_n: u128, jump_multiplier_rate_per_year_d: u128) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			ensure!(<Accounts<T>>::is_admin_internal(&sender), Error::<T>::RequireAdmin);
			ensure!(<LiquidityPools<T>>::pool_exists(&pool_id), Error::<T>::PoolNotFound);
			ensure!(jump_multiplier_rate_per_year_d > 0, Error::<T>::NumOverflow);


			let new_jump_multiplier_per_year = Rate::saturating_from_rational(jump_multiplier_rate_per_year_n, jump_multiplier_rate_per_year_d);
			let new_jump_multiplier_per_block = new_jump_multiplier_per_year
				.checked_div(&Rate::from_inner(T::BlocksPerYear::get()))
				.ok_or(Error::<T>::NumOverflow)?;

			ControllerDates::<T>::mutate(pool_id, |r| r.jump_multiplier_per_block = new_jump_multiplier_per_block);
			Self::deposit_event(Event::JumpMultiplierPerBlockHasChanged);
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
		let current_borrow_index = <LiquidityPools<T>>::get_pool_borrow_index(underlying_asset_id);

		// Calculate the current borrow interest rate
		let current_borrow_interest_rate = Self::calculate_borrow_interest_rate(
			underlying_asset_id,
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

		let simple_interest_factor = Self::calculate_interest_factor(current_borrow_interest_rate, &block_delta)?;
		let interest_accumulated =
			Self::calculate_interest_accumulated(simple_interest_factor, current_total_borrowed_balance)?;
		let new_total_borrow_balance =
			Self::calculate_new_total_borrow(interest_accumulated, current_total_borrowed_balance)?;
		let new_total_insurance =
			Self::calculate_new_total_insurance(interest_accumulated, insurance_factor, current_total_insurance)?;
		let new_borrow_index = Self::calculate_new_borrow_index(simple_interest_factor, current_borrow_index)?;

		// Save new params
		ControllerDates::<T>::mutate(underlying_asset_id, |x| x.timestamp = current_block_number);
		ControllerDates::<T>::mutate(underlying_asset_id, |x| x.borrow_rate = current_borrow_interest_rate);
		<LiquidityPools<T>>::set_accrual_interest_params(
			underlying_asset_id,
			new_total_borrow_balance,
			new_total_insurance,
		)?;
		<LiquidityPools<T>>::set_pool_borrow_index(underlying_asset_id, new_borrow_index)?;

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
	///          hypothetical account shortfall below collateral requirements).
	pub fn get_hypothetical_account_liquidity(
		account: &T::AccountId,
		underlying_to_borrow: CurrencyId,
		redeem_amount: Balance,
		borrow_amount: Balance,
	) -> LiquidityResult {
		let m_tokens_ids = T::MTokensId::get();

		let mut sum_collateral = Balance::zero();
		let mut sum_borrow_plus_effects = Balance::zero();

		// For each tokens the account is in
		for asset in m_tokens_ids.into_iter() {
			let underlying_asset = Self::get_underlying_asset_id_by_wrapped_id(&asset)?;
			if !<LiquidityPools<T>>::check_user_available_collateral(&account, underlying_asset) {
				continue;
			}

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

			if oracle_price == Price::zero() {
				return Ok((Balance::zero(), Balance::zero()));
			}

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
			if underlying_to_borrow == underlying_asset {
				// redeem effect
				if redeem_amount > 0 {
					// sum_borrow_plus_effects += tokens_to_denom * redeem_tokens
					sum_borrow_plus_effects = Self::mul_price_and_balance_add_to_prev_value(
						sum_borrow_plus_effects,
						redeem_amount,
						tokens_to_denom,
					)?;
				};
				// borrow effect
				if borrow_amount > 0 {
					// sum_borrow_plus_effects += oracle_price * borrow_amount
					sum_borrow_plus_effects = Self::mul_price_and_balance_add_to_prev_value(
						sum_borrow_plus_effects,
						borrow_amount,
						oracle_price,
					)?;
				}
			}
		}

		ensure!(
			!((sum_collateral == sum_borrow_plus_effects) && (sum_collateral == 0)),
			Error::<T>::InsufficientLiquidity
		);

		match sum_collateral.cmp(&sum_borrow_plus_effects) {
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
		}
	}

	/// Checks if the account should be allowed to deposit tokens in the given pool.
	///
	/// - `underlying_asset_id` - The CurrencyId to verify the redeem against.
	/// - `depositor` -  The account which would deposit the tokens.
	/// - `deposit_amount` - The amount of underlying being supplied to the pool in exchange for
	/// tokens.
	/// Return Ok if the deposit is allowed, otherwise a semi-opaque error code.
	pub fn deposit_allowed(
		underlying_asset_id: CurrencyId,
		_depositor: &T::AccountId,
		_deposit_amount: Balance,
	) -> DispatchResult {
		ensure!(
			Self::is_operation_allowed(underlying_asset_id, Operation::Deposit),
			Error::<T>::OperationPaused
		);
		Ok(())
	}

	/// Checks if the account should be allowed to redeem tokens in the given pool.
	///
	/// - `underlying_asset_id` - The CurrencyId to verify the redeem against.
	/// - `redeemer` -  The account which would redeem the tokens.
	/// - `redeem_amount` - The number of mTokens to exchange for the underlying asset in the
	/// market.
	/// Return Ok if the borrow is allowed, otherwise a semi-opaque error code.
	pub fn redeem_allowed(
		underlying_asset_id: CurrencyId,
		redeemer: &T::AccountId,
		redeem_amount: Balance,
	) -> DispatchResult {
		ensure!(
			Self::is_operation_allowed(underlying_asset_id, Operation::Redeem),
			Error::<T>::OperationPaused
		);

		let (_, shortfall) =
			Self::get_hypothetical_account_liquidity(&redeemer, underlying_asset_id, redeem_amount, 0)?;

		ensure!(!(shortfall > 0), Error::<T>::InsufficientLiquidity);

		Ok(())
	}

	/// Checks if the account should be allowed to borrow the underlying asset of the given pool.
	///
	/// - `underlying_asset_id` - The CurrencyId to verify the borrow against.
	/// - `who` -  The account which would borrow the asset.
	/// - `borrow_amount` - The amount of underlying assets the account would borrow.
	/// Return Ok if the borrow is allowed, otherwise a semi-opaque error code.
	pub fn borrow_allowed(
		underlying_asset_id: CurrencyId,
		who: &T::AccountId,
		borrow_amount: Balance,
	) -> DispatchResult {
		ensure!(
			Self::is_operation_allowed(underlying_asset_id, Operation::Borrow),
			Error::<T>::OperationPaused
		);

		let oracle_price =
			<Oracle<T>>::get_underlying_price(underlying_asset_id).map_err(|_| Error::<T>::OraclePriceError)?;

		ensure!(oracle_price != Price::from_inner(0), Error::<T>::OraclePriceError);

		//FIXME: add borrowCap checking

		let (_, shortfall) = Self::get_hypothetical_account_liquidity(&who, underlying_asset_id, 0, borrow_amount)?;

		ensure!(!(shortfall > 0), Error::<T>::InsufficientLiquidity);

		Ok(())
	}

	/// Checks if the account should be allowed to repay a borrow in the given pool.
	///
	/// - `underlying_asset_id` - The CurrencyId to verify the repay against.
	/// - `who` -  The account which would repay the asset.
	/// - `borrow_amount` - The amount of underlying assets the account would repay.
	/// Return Ok if the borrow is allowed, otherwise a semi-opaque error code
	pub fn repay_borrow_allowed(
		underlying_asset_id: CurrencyId,
		_who: &T::AccountId,
		_repay_amount: Balance,
	) -> DispatchResult {
		ensure!(
			Self::is_operation_allowed(underlying_asset_id, Operation::Repay),
			Error::<T>::OperationPaused
		);

		let _borrow_index = <LiquidityPools<T>>::get_pool_borrow_index(underlying_asset_id);

		Ok(())
	}

	/// Checks if a specific operation is allowed on a pool.
	///
	/// Return true - if operation is allowed, false - if operation is unallowed.
	pub fn is_operation_allowed(pool_id: CurrencyId, operation: Operation) -> bool {
		match operation {
			Operation::Deposit => !Self::pause_keepers(pool_id).deposit_paused,
			Operation::Redeem => !Self::pause_keepers(pool_id).redeem_paused,
			Operation::Borrow => !Self::pause_keepers(pool_id).borrow_paused,
			Operation::Repay => !Self::pause_keepers(pool_id).repay_paused,
		}
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

		let total_insurance = <LiquidityPools<T>>::get_pool_total_insurance(underlying_asset_id);

		let total_borrowed = <LiquidityPools<T>>::get_pool_total_borrowed(underlying_asset_id);

		let current_exchange_rate =
			Self::calculate_exchange_rate(total_cash, total_supply, total_insurance, total_borrowed)?;

		<LiquidityPools<T>>::set_current_exchange_rate(underlying_asset_id, current_exchange_rate)?;

		Ok(current_exchange_rate)
	}

	//Used in: get_exchange_rate
	fn calculate_exchange_rate(
		total_cash: Balance,
		total_supply: Balance,
		total_insurance: Balance,
		total_borrowed: Balance,
	) -> RateResult {
		let rate = match total_supply.cmp(&Balance::zero()) {
			Ordering::Equal => T::InitialExchangeRate::get(),
			_ => {
				let cash_plus_borrows = total_cash.checked_add(total_borrowed).ok_or(Error::<T>::NumOverflow)?;

				let cash_plus_borrows_minus_insurance = cash_plus_borrows
					.checked_sub(total_insurance)
					.ok_or(Error::<T>::NumOverflow)?;

				Rate::saturating_from_rational(cash_plus_borrows_minus_insurance, total_supply)
			}
		};

		Ok(rate)
	}

	fn calculate_borrow_interest_rate(
		underlying_asset_id: CurrencyId,
		current_total_balance: Balance,
		current_total_borrowed_balance: Balance,
		current_total_insurance: Balance,
	) -> RateResult {
		let utilization_rate = Self::calculate_utilization_rate(
			current_total_balance,
			current_total_borrowed_balance,
			current_total_insurance,
		)?;

		let kink = Self::controller_dates(underlying_asset_id).kink;
		let multiplier_per_block = Self::controller_dates(underlying_asset_id).multiplier_per_block;
		let base_rate_per_block = Self::controller_dates(underlying_asset_id).base_rate_per_block;

		let borrow_interest_rate = match utilization_rate.cmp(&kink) {
			Ordering::Greater => {
				let jump_multiplier_per_block = Self::controller_dates(underlying_asset_id).jump_multiplier_per_block;
				let normal_rate = (kink.checked_mul(&multiplier_per_block).ok_or(Error::<T>::NumOverflow)?)
					.checked_add(&base_rate_per_block)
					.ok_or(Error::<T>::NumOverflow)?;
				let excess_util = utilization_rate.checked_mul(&kink).ok_or(Error::<T>::NumOverflow)?;

				excess_util
					.checked_mul(&jump_multiplier_per_block)
					.ok_or(Error::<T>::NumOverflow)?
					.checked_add(&normal_rate)
					.ok_or(Error::<T>::NumOverflow)?
			}
			_ => utilization_rate
				.checked_mul(&multiplier_per_block)
				.ok_or(Error::<T>::NumOverflow)?
				.checked_add(&base_rate_per_block)
				.ok_or(Error::<T>::NumOverflow)?,
		};

		Ok(borrow_interest_rate)
	}

	// Calculates the utilization rate of the market:
	// borrows / (cash + borrows - reserves)
	fn calculate_utilization_rate(
		current_total_balance: Balance,
		current_total_borrowed_balance: Balance,
		current_total_insurance: Balance,
	) -> RateResult {
		if current_total_borrowed_balance == 0 {
			return Ok(Rate::from_inner(0));
		}

		let total_balance_total_borrowed_sum = current_total_balance
			.checked_add(current_total_borrowed_balance)
			.ok_or(Error::<T>::NumOverflow)?;
		let denominator = total_balance_total_borrowed_sum
			.checked_sub(current_total_insurance)
			.ok_or(Error::<T>::NumOverflow)?;
		let utilization_rate = Rate::saturating_from_rational(current_total_borrowed_balance, denominator);

		Ok(utilization_rate)
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

		let interest_factor = Rate::saturating_from_rational(block_delta_as_usize as u128, 1)
			.checked_mul(&current_borrow_interest_rate)
			.ok_or(Error::<T>::NumOverflow)?;

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

	// borrowIndexNew = simpleInterestFactor * borrowIndex + borrowIndex
	fn calculate_new_borrow_index(simple_interest_factor: Rate, current_borrow_index: Rate) -> RateResult {
		let accumulated = simple_interest_factor
			.checked_mul(&current_borrow_index)
			.ok_or(Error::<T>::NumOverflow)?;
		let new_borrow_index = accumulated
			.checked_add(&current_borrow_index)
			.ok_or(Error::<T>::NumOverflow)?;
		Ok(new_borrow_index)
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

	pub fn get_wrapped_id_by_underlying_asset_id(asset_id: &CurrencyId) -> CurrencyIdResult {
		match asset_id {
			CurrencyId::DOT => Ok(CurrencyId::MDOT),
			CurrencyId::KSM => Ok(CurrencyId::MKSM),
			CurrencyId::BTC => Ok(CurrencyId::MBTC),
			CurrencyId::ETH => Ok(CurrencyId::METH),
			_ => Err(Error::<T>::NotValidUnderlyingAssetId.into()),
		}
	}

	pub fn get_underlying_asset_id_by_wrapped_id(wrapped_id: &CurrencyId) -> CurrencyIdResult {
		match wrapped_id {
			CurrencyId::MDOT => Ok(CurrencyId::DOT),
			CurrencyId::MKSM => Ok(CurrencyId::KSM),
			CurrencyId::MBTC => Ok(CurrencyId::BTC),
			CurrencyId::METH => Ok(CurrencyId::ETH),
			_ => Err(Error::<T>::NotValidWrappedTokenId.into()),
		}
	}
}

// Getters for Controller Data
impl<T: Trait> Module<T> {
	/// Determines how much a user can borrow.
	fn get_collateral_factor(pool_id: CurrencyId) -> Rate {
		Self::controller_dates(pool_id).collateral_factor
	}
}

// Admin functions
impl<T: Trait> Module<T> {
	fn do_deposit_insurance(who: &T::AccountId, pool_id: CurrencyId, amount: Balance) -> DispatchResult {
		ensure!(<LiquidityPools<T>>::pool_exists(&pool_id), Error::<T>::PoolNotFound);
		ensure!(
			amount <= T::MultiCurrency::free_balance(pool_id, &who),
			Error::<T>::NotEnoughBalance
		);

		T::MultiCurrency::transfer(pool_id, &who, &<LiquidityPools<T>>::pools_account_id(), amount)?;

		let new_insurance_balance = <LiquidityPools<T>>::get_pool_total_insurance(pool_id)
			.checked_add(amount)
			.ok_or(Error::<T>::BalanceOverflowed)?;

		<LiquidityPools<T>>::set_pool_total_insurance(pool_id, new_insurance_balance)?;

		Ok(())
	}

	fn do_redeem_insurance(who: &T::AccountId, pool_id: CurrencyId, amount: Balance) -> DispatchResult {
		ensure!(<LiquidityPools<T>>::pool_exists(&pool_id), Error::<T>::PoolNotFound);

		let current_total_insurance = <LiquidityPools<T>>::get_pool_total_insurance(pool_id);
		ensure!(amount <= current_total_insurance, Error::<T>::NotEnoughBalance);

		T::MultiCurrency::transfer(pool_id, &<LiquidityPools<T>>::pools_account_id(), &who, amount)?;

		let new_insurance_balance = current_total_insurance
			.checked_sub(amount)
			.ok_or(Error::<T>::NotEnoughBalance)?;

		<LiquidityPools<T>>::set_pool_total_insurance(pool_id, new_insurance_balance)?;

		Ok(())
	}
}
