#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode};
use frame_support::{decl_error, decl_event, decl_module, decl_storage, ensure, traits::Get};
use frame_system::{self as system};
use minterest_primitives::{Balance, CurrencyId, Rate};
use orml_traits::MultiCurrency;
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};
use sp_runtime::traits::{CheckedMul, Zero};
use sp_runtime::{traits::CheckedDiv, DispatchError, DispatchResult, FixedPointNumber, RuntimeDebug};
use sp_std::convert::TryInto;
use sp_std::result;

#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(Encode, Decode, RuntimeDebug, Eq, PartialEq, Default)]
pub struct ControllerData<BlockNumber> {
	pub timestamp: BlockNumber,
	pub borrow_rate: Rate,
}

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

type LiquidityPools<T> = liquidity_pools::Module<T>;

pub trait Trait: liquidity_pools::Trait + system::Trait {
	/// The overarching event type.
	type Event: From<Event> + Into<<Self as system::Trait>::Event>;

	/// The `MultiCurrency` implementation for wrapped.
	type MultiCurrency: MultiCurrency<Self::AccountId, Balance = Balance, CurrencyId = CurrencyId>;

	/// Start exchange rate
	type InitialExchangeRate: Get<Rate>;

	/// Fraction of interest currently set aside for insurace.
	type InsuranceFactor: Get<Rate>;

	/// Maximum borrow rate that can ever be applied
	type MaxBorrowRate: Get<Rate>;
}

decl_event! {
	pub enum Event {}
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
	}
}

decl_module! {
	pub struct Module<T: Trait> for enum Call where origin: T::Origin {}
}

type RateResult = result::Result<Rate, DispatchError>;
type BalanceResult = result::Result<Balance, DispatchError>;

impl<T: Trait> Module<T> {
	// Used in controller: do_deposit, do_redeem
	pub fn accrue_interest_rate(underlying_asset_id: CurrencyId) -> DispatchResult {
		ensure!(
			!<LiquidityPools<T>>::reserves(&underlying_asset_id).is_lock,
			Error::<T>::OperationsLocked
		);

		//Remember the initial block number
		let current_block_number = <frame_system::Module<T>>::block_number();
		let accrual_block_number_previous = Self::controller_dates(underlying_asset_id).timestamp;

		//Short-circuit accumulating 0 interest
		if current_block_number == accrual_block_number_previous {
			return Ok(());
		}

		let current_total_balance = <LiquidityPools<T>>::get_reserve_available_liquidity(underlying_asset_id);
		let current_total_borrowed_balance = <LiquidityPools<T>>::get_reserve_total_borrowed(underlying_asset_id);
		let current_total_insurance = <LiquidityPools<T>>::get_reserve_total_insurance(underlying_asset_id);
		let _current_borrow_index: Rate; // FIXME: how can i use it?

		// Calculate the current borrow interest rate
		let current_borrow_interest_rate = Self::calculate_borrow_interest_rate(
			current_total_balance,
			current_total_borrowed_balance,
			current_total_insurance,
		)?;

		ensure!(
			current_borrow_interest_rate <= T::MaxBorrowRate::get(),
			Error::<T>::BorrowRateIsTooHight
		);

		// Calculate the number of blocks elapsed since the last accrual
		let block_delta = Self::calculate_block_delta(current_block_number, accrual_block_number_previous);

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
		let new_total_insurance = Self::calculate_new_total_insurance(
			interest_accumulated,
			T::InsuranceFactor::get(),
			current_total_insurance,
		)?;
		let _new_borrow_index: Rate; // FIXME: how can i use it?

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
		let exchange_rate = Self::get_exchange_rate(wrapped_id)?;

		let underlying_amount = Rate::from_inner(wrapped_amount)
			.checked_mul(&exchange_rate)
			.map(|x| x.into_inner())
			.ok_or(Error::<T>::NumOverflow)?;

		Ok(underlying_amount)
	}

	// Used in controller: do_borrow, do_repay
	pub fn calculate_user_global_data(_who: T::AccountId) -> DispatchResult {
		//FIXME
		let _price_from_oracle = 1;
		Ok(())
	}

	// Used in controller: do_borrow, do_repay
	pub fn calculate_total_available_collateral(_amount: Balance, _underlying_asset_id: CurrencyId) -> DispatchResult {
		//FIXME
		let _price_from_oracle = 1;
		Ok(())
	}

	// Not used yet
	pub fn calculate_interest_rate(_underlying_asset_id: CurrencyId) -> RateResult {
		//FIXME
		Ok(Rate::from_inner(1))
	}
}

// Private method
impl<T: Trait> Module<T> {
	// Used in: convert_to_wrapped
	fn get_exchange_rate(underlying_asset_id: CurrencyId) -> RateResult {
		// The total amount of cash the market has
		let total_cash = <LiquidityPools<T>>::get_reserve_available_liquidity(underlying_asset_id);

		// Total number of tokens in circulation
		let total_supply = T::MultiCurrency::total_issuance(underlying_asset_id);

		let current_exchange_rate = Self::calculate_exchange_rate(total_cash, total_supply)?;

		<LiquidityPools<T>>::set_current_exchange_rate(underlying_asset_id, current_exchange_rate)?;

		Ok(current_exchange_rate)
	}

	//Used in: get_exchange_rate
	fn calculate_exchange_rate(total_cash: Balance, total_supply: Balance) -> RateResult {
		let rate: u128;
		if total_supply == Balance::zero() {
			rate = T::InitialExchangeRate::get().into_inner();
		} else {
			rate = total_cash.checked_div(total_supply).ok_or(Error::<T>::NumOverflow)?;
		}

		Ok(Rate::from_inner(rate))
	}

	fn calculate_borrow_interest_rate(
		_current_total_balance: Balance,
		_current_total_borrowed_balance: Balance,
		_current_total_insurance: Balance,
	) -> RateResult {
		// FIXME
		Ok(Rate::from_inner(1))
	}

	fn calculate_block_delta(
		current_block_number: T::BlockNumber,
		accrual_block_number_previous: T::BlockNumber,
	) -> T::BlockNumber {
		accrual_block_number_previous - current_block_number
	}

	// simpleInterestFactor = borrowRate * blockDelta
	fn calculate_interest_factor(
		current_borrow_interest_rate: Rate,
		block_delta: &<T as system::Trait>::BlockNumber,
	) -> RateResult {
		let block_delta_as_usize = TryInto::try_into(*block_delta)
			.ok()
			.expect("blockchain will not exceed 2^32 blocks; qed");

		let interest_factor = Rate::from_inner(block_delta_as_usize as u128)
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
}
