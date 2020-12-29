#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode};
use frame_support::{decl_error, decl_event, decl_module, decl_storage, ensure, traits::Get};
use frame_system::{ensure_root, ensure_signed};
use minterest_primitives::{Balance, CurrencyId, Rate};
use orml_traits::MultiCurrency;
use orml_utilities::with_transaction_result;
use pallet_traits::Borrowing;
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};
use sp_runtime::{
	traits::{AccountIdConversion, Zero},
	DispatchResult, ModuleId, RuntimeDebug,
};
use sp_std::cmp::Ordering;

#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(Encode, Decode, RuntimeDebug, Eq, PartialEq, Default)]
pub struct Pool {
	pub current_interest_rate: Rate, // FIXME: how can i use it?
	pub total_borrowed: Balance,
	pub current_exchange_rate: Rate,
	pub is_lock: bool,
	pub total_insurance: Balance,
}

#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(Encode, Decode, RuntimeDebug, Eq, PartialEq, Default)]
pub struct PoolUserData<BlockNumber> {
	pub total_borrowed: Balance,
	pub collateral: bool,
	pub timestamp: BlockNumber,
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
		/// Pool locked: \[pool_id\]
		PoolLocked(CurrencyId),

		/// Pool unlocked: \[pool_id\]
		PoolUnLocked(CurrencyId),

		/// Insurance balance replenished: \[pool_id, amount\]
		DepositedInsurance(CurrencyId, Balance),

		/// Insurance balance redeemed: \[pool_id, amount\]
		RedeemedInsurance(CurrencyId, Balance),
	}
);

decl_error! {
	pub enum Error for Module<T: Trait> {

	/// Pool not found.
	PoolNotFound,

	/// Not enough balance to withdraw or repay.
	NotEnoughBalance,

	/// Balance overflows maximum.
	///
	/// Only happened when the balance went wrong and balance overflows the integer type.
	BalanceOverflowed,
	}
}

decl_storage! {
	 trait Store for Module<T: Trait> as LiquidityPoolsStorage {
		pub Pools get(fn pools) config(): map hasher(blake2_128_concat) CurrencyId => Pool;
		pub PoolUserDates get(fn pool_user_data) config(): double_map
			hasher(blake2_128_concat) T::AccountId,
			hasher(blake2_128_concat) CurrencyId => PoolUserData<T::BlockNumber>;
	}
}

decl_module! {
		pub struct Module<T: Trait> for enum Call where origin: T::Origin {
			type Error = Error<T>;
			fn deposit_event() = default;

			/// The Liquidity Pool's module id, keep all assets in Pools.
			const ModuleId: ModuleId = T::ModuleId::get();

			/// Locks all operations (deposit, redeem, borrow, repay)  with the pool.
			///
			/// The dispatch origin of this call must be _Root_.
			#[weight = 10_000]
			pub fn lock_pool_transactions(origin, pool_id: CurrencyId) -> DispatchResult {
				ensure_root(origin)?;
				ensure!(Self::pool_exists(&pool_id), Error::<T>::PoolNotFound);
				Pools::mutate(pool_id, |r| r.is_lock = true);
				Self::deposit_event(Event::PoolLocked(pool_id));
				Ok(())
			}

			/// Unlocks all operations (deposit, redeem, borrow, repay)  with the pool.
			///
			/// The dispatch origin of this call must be _Root_.
			#[weight = 10_000]
			pub fn unlock_pool_transactions(origin, pool_id: CurrencyId) -> DispatchResult {
				ensure_root(origin)?;
				ensure!(Self::pool_exists(&pool_id), Error::<T>::PoolNotFound);
				Pools::mutate(pool_id, |r| r.is_lock = false);
				Self::deposit_event(Event::PoolUnLocked(pool_id));
				Ok(())
			}

			/// Replenishes the insurance balance.
			///
			/// The dispatch origin of this call must be _Root_.
			#[weight = 10_000]
			pub fn deposit_insurance(origin, pool_id: CurrencyId, #[compact] amount: Balance) {
				with_transaction_result(|| {
					// FIXME This dispatch should only be called as an _Root_.
					let account_id = ensure_signed(origin)?;
					Self::do_deposit_insurance(&account_id, pool_id, amount)?;
					Self::deposit_event(Event::DepositedInsurance(pool_id, amount));
					Ok(())
				})?;
			}

			/// Removes the insurance balance.
			///
			/// The dispatch origin of this call must be _Root_.
			#[weight = 10_000]
			pub fn redeem_insurance(origin, pool_id: CurrencyId, #[compact] amount: Balance) {
				with_transaction_result(|| {
					// FIXME This dispatch should only be called as an _Root_.
					let account_id = ensure_signed(origin)?;
					Self::do_redeem_insurance(&account_id, pool_id, amount)?;
					Self::deposit_event(Event::RedeemedInsurance(pool_id, amount));
					Ok(())
				})?;

			}

	}
}

// Admin functions
impl<T: Trait> Module<T> {
	fn do_deposit_insurance(who: &T::AccountId, pool_id: CurrencyId, amount: Balance) -> DispatchResult {
		ensure!(Self::pool_exists(&pool_id), Error::<T>::PoolNotFound);
		ensure!(
			amount <= T::MultiCurrency::free_balance(pool_id, &who),
			Error::<T>::NotEnoughBalance
		);

		T::MultiCurrency::transfer(pool_id, &who, &Self::pools_account_id(), amount)?;

		let new_insurance_balance = Self::pools(pool_id)
			.total_insurance
			.checked_add(amount)
			.ok_or(Error::<T>::BalanceOverflowed)?;

		Pools::mutate(pool_id, |r| r.total_insurance = new_insurance_balance);
		Ok(())
	}

	fn do_redeem_insurance(who: &T::AccountId, pool_id: CurrencyId, amount: Balance) -> DispatchResult {
		ensure!(Self::pool_exists(&pool_id), Error::<T>::PoolNotFound);

		let current_total_insurance = Self::pools(pool_id).total_insurance;
		ensure!(amount <= current_total_insurance, Error::<T>::NotEnoughBalance);

		T::MultiCurrency::transfer(pool_id, &Self::pools_account_id(), &who, amount)?;

		let new_insurance_balance = current_total_insurance
			.checked_sub(amount)
			.ok_or(Error::<T>::NotEnoughBalance)?;

		Pools::mutate(pool_id, |r| r.total_insurance = new_insurance_balance);
		Ok(())
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
}

// Getters for LiquidityPools
impl<T: Trait> Module<T> {
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

	pub fn get_user_total_borrowed(who: &T::AccountId, currency_id: CurrencyId) -> Balance {
		Self::pool_user_data(who, currency_id).total_borrowed
	}

	pub fn check_user_available_collateral(who: &T::AccountId, currency_id: CurrencyId) -> bool {
		Self::pool_user_data(who, currency_id).collateral
	}
}

// Private methods for LiquidityPools
impl<T: Trait> Module<T> {
	fn update_pool_and_user_total_borrowed(
		underlying_asset_id: CurrencyId,
		amount_borrowed_add: Balance,
		amount_borrowed_reduce: Balance,
		who: &T::AccountId,
	) -> DispatchResult {
		let current_user_borrow_balance = Self::pool_user_data(who, underlying_asset_id).total_borrowed;
		let current_total_borrow_balance = Self::pools(underlying_asset_id).total_borrowed;

		let (new_user_borrow_balance, new_total_borrow_balance) = match amount_borrowed_add.cmp(&Balance::zero()) {
			Ordering::Greater => (
				current_user_borrow_balance
					.checked_add(amount_borrowed_add)
					.ok_or(Error::<T>::BalanceOverflowed)?,
				current_total_borrow_balance
					.checked_add(amount_borrowed_add)
					.ok_or(Error::<T>::BalanceOverflowed)?,
			),
			_ => (
				current_user_borrow_balance
					.checked_sub(amount_borrowed_reduce)
					.ok_or(Error::<T>::NotEnoughBalance)?,
				current_total_borrow_balance
					.checked_sub(amount_borrowed_reduce)
					.ok_or(Error::<T>::NotEnoughBalance)?,
			),
		};

		PoolUserDates::<T>::mutate(who, underlying_asset_id, |x| x.total_borrowed = new_user_borrow_balance);
		Pools::mutate(underlying_asset_id, |x| x.total_borrowed = new_total_borrow_balance);

		Ok(())
	}

	fn pool_exists(underlying_asset_id: &CurrencyId) -> bool {
		Pools::contains_key(underlying_asset_id)
	}
}

// Trait Borrowing for LiquidityPools
impl<T: Trait> Borrowing<T::AccountId> for Module<T> {
	fn update_state_on_borrow(
		underlying_asset_id: CurrencyId,
		amount_borrowed: Balance,
		who: &T::AccountId,
	) -> DispatchResult {
		Self::update_pool_and_user_total_borrowed(underlying_asset_id, amount_borrowed, Balance::zero(), who)?;
		Ok(())
	}

	fn update_state_on_repay(
		underlying_asset_id: CurrencyId,
		amount_borrowed: Balance,
		who: &T::AccountId,
	) -> DispatchResult {
		Self::update_pool_and_user_total_borrowed(underlying_asset_id, Balance::zero(), amount_borrowed, who)?;
		Ok(())
	}
}
