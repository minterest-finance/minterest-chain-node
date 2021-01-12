#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode};
use frame_support::{decl_error, decl_event, decl_module, decl_storage, traits::Get};
use minterest_primitives::{Balance, CurrencyId, Rate};
use orml_traits::MultiCurrency;
use pallet_traits::Borrowing;
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};
use sp_runtime::{traits::AccountIdConversion, DispatchResult, ModuleId, RuntimeDebug};

#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(Encode, Decode, RuntimeDebug, Eq, PartialEq, Default)]
pub struct Pool {
	pub current_interest_rate: Rate, // FIXME: how can i use it?
	pub total_borrowed: Balance,
	/// Accumulator of the total earned interest rate since the opening of the pool
	pub borrow_index: Rate,
	pub current_exchange_rate: Rate, // FIXME: can be removed.
	pub total_insurance: Balance,
}

#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(Encode, Decode, RuntimeDebug, Eq, PartialEq, Default)]
pub struct PoolUserData {
	/// Total balance (with accrued interest), after applying the most
	/// recent balance-changing action
	pub total_borrowed: Balance,
	/// Global borrow_index as of the most recent balance-changing action
	pub interest_index: Rate,
	pub collateral: bool,
}

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

pub trait Trait: frame_system::Trait {
	type Event: From<Event> + Into<<Self as frame_system::Trait>::Event>;

	/// The Liquidity Pool's module id, keep all assets in Pools.
	type ModuleId: Get<ModuleId>;

	/// The `MultiCurrency` implementation.
	type MultiCurrency: MultiCurrency<Self::AccountId, Balance = Balance, CurrencyId = CurrencyId>;
}

decl_event!(
	pub enum Event {
		/// Pool total balance: \[pool_id, amount\]
		PoolTotalBalance(CurrencyId, Balance),
	}
);

decl_error! {
	pub enum Error for Module<T: Trait> {
	/// Number overflow in calculation.
	NumOverflow,
	}
}

decl_storage! {
	 trait Store for Module<T: Trait> as LiquidityPoolsStorage {
		pub Pools get(fn pools) config(): map hasher(blake2_128_concat) CurrencyId => Pool;
		pub PoolUserDates get(fn pool_user_data) config(): double_map
			hasher(blake2_128_concat) T::AccountId,
			hasher(blake2_128_concat) CurrencyId => PoolUserData;
	}
}

decl_module! {
		pub struct Module<T: Trait> for enum Call where origin: T::Origin {
			type Error = Error<T>;
			fn deposit_event() = default;

			/// The Liquidity Pool's module id, keep all assets in Pools.
			const ModuleId: ModuleId = T::ModuleId::get();

			/// The Liquidity Pool's account id, keep all assets in Pools.
			const PoolAccountId: T::AccountId = T::ModuleId::get().into_account();
	}
}

// Setters for LiquidityPools
impl<T: Trait> Module<T> {
	pub fn set_current_interest_rate(underlying_asset_id: CurrencyId, _rate: Rate) -> DispatchResult {
		Pools::mutate(underlying_asset_id, |r| r.current_interest_rate = Rate::from_inner(1));
		Ok(())
	}

	pub fn set_current_exchange_rate(underlying_asset_id: CurrencyId, rate: Rate) -> DispatchResult {
		Pools::mutate(underlying_asset_id, |r| r.current_exchange_rate = rate);
		Ok(())
	}

	pub fn set_pool_total_borrowed(pool_id: CurrencyId, new_total_borrows: Balance) -> DispatchResult {
		Pools::mutate(pool_id, |pool| pool.total_borrowed = new_total_borrows);
		Ok(())
	}

	pub fn set_pool_borrow_index(pool_id: CurrencyId, new_borrow_index: Rate) -> DispatchResult {
		Pools::mutate(pool_id, |pool| pool.borrow_index = new_borrow_index);
		Ok(())
	}

	pub fn set_pool_total_insurance(pool_id: CurrencyId, new_total_insurance: Balance) -> DispatchResult {
		Pools::mutate(pool_id, |r| r.total_insurance = new_total_insurance);
		Ok(())
	}

	pub fn set_user_total_borrowed_and_interest_index(
		who: &T::AccountId,
		pool_id: CurrencyId,
		new_total_borrows: Balance,
		new_interest_index: Rate,
	) -> DispatchResult {
		PoolUserDates::<T>::mutate(who, pool_id, |p| {
			p.total_borrowed = new_total_borrows;
			p.interest_index = new_interest_index;
		});
		Ok(())
	}

	pub fn set_accrual_interest_params(
		underlying_asset_id: CurrencyId,
		new_total_borrow_balance: Balance,
		new_total_insurance: Balance,
	) -> DispatchResult {
		Pools::mutate(underlying_asset_id, |r| {
			r.total_borrowed = new_total_borrow_balance;
			r.total_insurance = new_total_insurance;
		});
		Ok(())
	}

	pub fn enable_as_collateral_internal(who: &T::AccountId, pool_id: CurrencyId) -> DispatchResult {
		PoolUserDates::<T>::mutate(who, pool_id, |p| p.collateral = true);
		Ok(())
	}

	pub fn disable_collateral_internal(who: &T::AccountId, pool_id: CurrencyId) -> DispatchResult {
		PoolUserDates::<T>::mutate(who, pool_id, |p| p.collateral = false);
		Ok(())
	}
}

// Getters for LiquidityPools
impl<T: Trait> Module<T> {
	/// Module account id
	pub fn pools_account_id() -> T::AccountId {
		T::ModuleId::get().into_account()
	}

	pub fn get_pool_available_liquidity(currency_id: CurrencyId) -> Balance {
		let module_account_id = Self::pools_account_id();
		T::MultiCurrency::free_balance(currency_id, &module_account_id)
	}

	pub fn get_pool_total_borrowed(currency_id: CurrencyId) -> Balance {
		Self::pools(currency_id).total_borrowed
	}

	pub fn get_pool_total_insurance(currency_id: CurrencyId) -> Balance {
		Self::pools(currency_id).total_insurance
	}

	/// Accumulator of the total earned interest rate since the opening of the pool
	pub fn get_pool_borrow_index(pool_id: CurrencyId) -> Rate {
		Self::pools(pool_id).borrow_index
	}

	/// Global borrow_index as of the most recent balance-changing action
	pub fn get_user_borrow_index(who: &T::AccountId, currency_id: CurrencyId) -> Rate {
		Self::pool_user_data(who, currency_id).interest_index
	}

	pub fn get_user_total_borrowed(who: &T::AccountId, currency_id: CurrencyId) -> Balance {
		Self::pool_user_data(who, currency_id).total_borrowed
	}

	pub fn check_user_available_collateral(who: &T::AccountId, currency_id: CurrencyId) -> bool {
		Self::pool_user_data(who, currency_id).collateral
	}

	pub fn pool_exists(underlying_asset_id: &CurrencyId) -> bool {
		Pools::contains_key(underlying_asset_id)
	}
}

// Trait Borrowing for LiquidityPools
impl<T: Trait> Borrowing<T::AccountId> for Module<T> {
	fn update_state_on_borrow(
		who: &T::AccountId,
		underlying_asset_id: CurrencyId,
		borrow_amount: Balance,
		account_borrows: Balance,
	) -> DispatchResult {
		let pool_borrow_index = Self::get_pool_borrow_index(underlying_asset_id);

		// Calculate the new borrower and total borrow balances, failing on overflow:
		// account_borrows_new = account_borrows + borrow_amount
		// total_borrows_new = total_borrows + borrow_amount
		let account_borrow_new = account_borrows
			.checked_add(borrow_amount)
			.ok_or(Error::<T>::NumOverflow)?;
		let total_borrows_new = Self::get_pool_total_borrowed(underlying_asset_id)
			.checked_add(borrow_amount)
			.ok_or(Error::<T>::NumOverflow)?;

		// Write the previously calculated values into storage.
		Self::set_pool_total_borrowed(underlying_asset_id, total_borrows_new)?;
		Self::set_user_total_borrowed_and_interest_index(
			&who,
			underlying_asset_id,
			account_borrow_new,
			pool_borrow_index,
		)?;
		Ok(())
	}

	fn update_state_on_repay(
		who: &T::AccountId,
		underlying_asset_id: CurrencyId,
		repay_amount: Balance,
		account_borrows: Balance,
	) -> DispatchResult {
		let pool_borrow_index = Self::get_pool_borrow_index(underlying_asset_id);

		// Calculate the new borrower and total borrow balances, failing on overflow:
		// account_borrows_new = account_borrows - repay_amount
		// total_borrows_new = total_borrows + repay_amount
		let account_borrow_new = account_borrows
			.checked_sub(repay_amount)
			.ok_or(Error::<T>::NumOverflow)?;
		let total_borrows_new = Self::get_pool_total_borrowed(underlying_asset_id)
			.checked_sub(repay_amount)
			.ok_or(Error::<T>::NumOverflow)?;

		// Write the previously calculated values into storage.
		Self::set_pool_total_borrowed(underlying_asset_id, total_borrows_new)?;
		Self::set_user_total_borrowed_and_interest_index(
			&who,
			underlying_asset_id,
			account_borrow_new,
			pool_borrow_index,
		)?;
		Ok(())
	}
}
