#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::{decl_error, decl_event, decl_module, decl_storage, ensure};
use frame_system::{self as system};
use minterest_primitives::{Balance, CurrencyId, Rate};
use orml_traits::MultiCurrency;
use sp_runtime::{traits::CheckedDiv, DispatchError, DispatchResult, FixedPointNumber};

use sp_runtime::traits::{CheckedMul, One, Zero};
use sp_std::result;

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

type LiquidityPools<T> = liquidity_pools::Module<T>;

// FIXME: move to runtime
pub const MAX_BORROW_RATE: Rate = Rate::from_inner(1);
pub const INSURANCE_FACTOR: Rate = Rate::from_inner(1);

pub trait Trait: liquidity_pools::Trait {
	/// The overarching event type.
	type Event: From<Event> + Into<<Self as system::Trait>::Event>;

	/// The `MultiCurrency` implementation for wrapped.
	type MultiCurrency: MultiCurrency<Self::AccountId, Balance = Balance, CurrencyId = CurrencyId>;
}

decl_event! {
	pub enum Event {}
}

decl_storage! {
	trait Store for Module<T: Trait> as X {

	}
}

decl_error! {
	pub enum Error for Module<T: Trait> {

		InvalidValues,

		/// Number overflow in calculation.
		NumOverflow,

		/// Operations with this underlying assets are locked by the administrator
		OperationsLocked,

		/// Borrow rate is absurdly high.
		BorrowRateIsTooHight,
	}
}

decl_module! {
	pub struct Module<T: Trait> for enum Call where origin: T::Origin {
		type Error = Error<T>;
		fn deposit_event() = default;

	}
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
		// FIXME: add Timestamp pallet
		let current_block_number: u64 = 1;
		let accrual_block_number_previous: u64 = 0;

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
			current_borrow_interest_rate <= MAX_BORROW_RATE,
			Error::<T>::BorrowRateIsTooHight
		);

		// Calculate the number of blocks elapsed since the last accrual
		let block_delta = Self::calculate_block_delta(current_block_number, accrual_block_number_previous);

		/*
		Calculate the interest accumulated into borrows and reserves and the new index:
			*  simpleInterestFactor = borrowRate * blockDelta
			*  interestAccumulated = simpleInterestFactor * totalBorrows
			*  totalBorrowsNew = interestAccumulated + totalBorrows
			*  totalReservesNew = interestAccumulated * reserveFactor + totalReserves
			*  borrowIndexNew = simpleInterestFactor * borrowIndex + borrowIndex
		*/

		let simple_interest_factor = Self::calculate_interest_factor(current_borrow_interest_rate, block_delta)?;
		let interest_accumulated =
			Self::calculate_interest_accumulated(simple_interest_factor, current_total_borrowed_balance);
		let new_total_borrow_balance =
			Self::calculate_new_total_borrow(interest_accumulated, current_total_borrowed_balance);
		let new_total_reserves =
			Self::calculate_new_total_reserves(interest_accumulated, INSURANCE_FACTOR, current_total_insurance);
		let _new_borrow_index: Rate; // FIXME: how can i use it?

		// TODO: save new values into the storage

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
		let rate = total_cash.checked_div(total_supply).ok_or(Error::<T>::InvalidValues)?;
		Ok(Rate::from_inner(rate))
	}

	fn calculate_borrow_interest_rate(
		_current_total_balance: Balance,
		_current_total_borrowed_balance: Balance,
		_current_total_insurance: Balance,
	) -> RateResult {
		Ok(Rate::from_inner(1))
	}

	fn calculate_block_delta(_current_block_number: u64, _accrual_block_number_previous: u64) -> u64 {
		//FIXME
		0
	}

	fn calculate_interest_factor(_current_borrow_interest_rate: Rate, _block_delta: u64) -> RateResult {
		//FIXME

		Ok(Rate::from_inner(1))
	}

	fn calculate_interest_accumulated(
		_simple_interest_factor: Rate,
		_current_total_borrowed_balance: Balance,
	) -> Balance {
		//FIXME
		Balance::one()
	}

	fn calculate_new_total_borrow(_interest_accumulated: Balance, _current_total_borrowed_balance: Balance) -> Balance {
		//FIXME
		Balance::zero()
	}

	fn calculate_new_total_reserves(
		_interest_accumulated: Balance,
		_insurance_factor: Rate,
		_current_total_insurance: Balance,
	) -> Balance {
		//FIXME
		Balance::one()
	}
}
