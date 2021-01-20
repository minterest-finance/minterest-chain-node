#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode};
use frame_support::{decl_error, decl_event, decl_module, decl_storage, traits::Get};
use minterest_primitives::{Balance, CurrencyId, Rate};
use orml_traits::MultiCurrency;
use pallet_traits::Borrowing;
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};
use sp_runtime::{
	traits::{AccountIdConversion, CheckedDiv, CheckedMul, Zero},
	DispatchError, DispatchResult, FixedPointNumber, ModuleId, RuntimeDebug,
};
use sp_std::{cmp::Ordering, result};

#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(Encode, Decode, RuntimeDebug, Eq, PartialEq, Default)]
pub struct Pool {
	pub total_borrowed: Balance,
	/// Accumulator of the total earned interest rate since the opening of the pool
	pub borrow_index: Rate,
	pub current_exchange_rate: Rate, // FIXME. Delete and implement via RPC
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

	/// Start exchange rate
	type InitialExchangeRate: Get<Rate>;
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

	/// The currency is not enabled in protocol.
	NotValidUnderlyingAssetId,

	/// The currency is not enabled in wrapped protocol.
	NotValidWrappedTokenId,
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

type RateResult = result::Result<Rate, DispatchError>;
type CurrencyIdResult = result::Result<CurrencyId, DispatchError>;
type BalanceResult = result::Result<Balance, DispatchError>;

// Setters for LiquidityPools
impl<T: Trait> Module<T> {
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

	/// Converts a specified number of underlying assets into wrapped tokens.
	/// The calculation is based on the exchange rate.
	///
	/// - `underlying_asset_id`: CurrencyId of underlying assets to be converted to wrapped tokens.
	/// - `underlying_amount`: The amount of underlying assets to be converted to wrapped tokens.
	/// Returns `wrapped_amount = underlying_amount / exchange_rate`
	pub fn convert_to_wrapped(underlying_asset_id: CurrencyId, underlying_amount: Balance) -> BalanceResult {
		let exchange_rate = Self::get_exchange_rate(underlying_asset_id)?;

		let wrapped_amount = Rate::from_inner(underlying_amount)
			.checked_div(&exchange_rate)
			.map(|x| x.into_inner())
			.ok_or(Error::<T>::NumOverflow)?;

		Ok(wrapped_amount)
	}

	/// Converts a specified number of wrapped tokens into underlying assets.
	/// The calculation is based on the exchange rate.
	///
	/// - `wrapped_id`: CurrencyId of the wrapped tokens to be converted to underlying assets.
	/// - `wrapped_amount`: The amount of wrapped tokens to be converted to underlying assets.
	/// Returns `underlying_amount = wrapped_amount * exchange_rate`
	pub fn convert_from_wrapped(wrapped_id: CurrencyId, wrapped_amount: Balance) -> BalanceResult {
		let underlying_asset_id = Self::get_underlying_asset_id_by_wrapped_id(&wrapped_id)?;
		let exchange_rate = Self::get_exchange_rate(underlying_asset_id)?;

		let underlying_amount = Rate::from_inner(wrapped_amount)
			.checked_mul(&exchange_rate)
			.map(|x| x.into_inner())
			.ok_or(Error::<T>::NumOverflow)?;

		Ok(underlying_amount)
	}

	/// Calculates the exchange rate from the underlying to the mToken.
	/// This function does not accrue interest before calculating the exchange rate.
	pub fn get_exchange_rate(underlying_asset_id: CurrencyId) -> RateResult {
		let wrapped_asset_id = Self::get_wrapped_id_by_underlying_asset_id(&underlying_asset_id)?;
		// The total amount of cash the market has
		let total_cash = Self::get_pool_available_liquidity(underlying_asset_id);

		// Total number of tokens in circulation
		let total_supply = T::MultiCurrency::total_issuance(wrapped_asset_id);

		let total_insurance = Self::get_pool_total_insurance(underlying_asset_id);

		let total_borrowed = Self::get_pool_total_borrowed(underlying_asset_id);

		let current_exchange_rate =
			Self::calculate_exchange_rate(total_cash, total_supply, total_insurance, total_borrowed)?;

		Ok(current_exchange_rate)
	}

	/// Calculates the exchange rate from the underlying to the mToken.
	fn calculate_exchange_rate(
		total_cash: Balance,
		total_supply: Balance,
		total_insurance: Balance,
		total_borrowed: Balance,
	) -> RateResult {
		let rate = match total_supply.cmp(&Balance::zero()) {
			// If there are no tokens minted: exchangeRate = InitialExchangeRate.
			Ordering::Equal => T::InitialExchangeRate::get(),
			// Otherwise: exchange_rate = (total_cash - total_insurance + total_borrowed) / total_supply
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

// Trait Borrowing for LiquidityPools
impl<T: Trait> Borrowing<T::AccountId> for Module<T> {
	fn update_state_on_borrow(
		who: &T::AccountId,
		underlying_asset_id: CurrencyId,
		borrow_amount: Balance,
		account_borrows: Balance,
	) -> DispatchResult {
		let pool_borrow_index = Self::get_pool_borrow_index(underlying_asset_id);
		let pool_total_borrowed = Self::get_pool_total_borrowed(underlying_asset_id);

		// Calculate the new borrower and total borrow balances, failing on overflow:
		// account_borrows_new = account_borrows + borrow_amount
		// total_borrows_new = total_borrows + borrow_amount
		let account_borrow_new = account_borrows
			.checked_add(borrow_amount)
			.ok_or(Error::<T>::NumOverflow)?;
		let total_borrows_new = pool_total_borrowed
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
