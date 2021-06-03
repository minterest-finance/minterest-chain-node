//! # Liquidity Pools Module
//!
//! ## Overview
//!
//! This pallet is managing information required for interest calculation.
//! Besides every pool contains some amount of "dead money" which are stored in
//! `total_protocol_interest`. These tokens don`t take part in protocol economy.
//! Also it contains a set of helper functions for conversion between underlying asset and wrapped
//! token.

#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::unused_unit)]
#![allow(clippy::upper_case_acronyms)]

use codec::{Decode, Encode};
use frame_support::{pallet_prelude::*, traits::Get};
use minterest_primitives::currency::CurrencyType::UnderlyingAsset;
use minterest_primitives::{Balance, CurrencyId, Rate};
pub use module::*;
use orml_traits::MultiCurrency;
use pallet_traits::{Borrowing, LiquidityPoolsManager, PoolsManager, PricesManager};
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};
use sp_runtime::{
	traits::{AccountIdConversion, CheckedDiv, CheckedMul, Zero},
	DispatchError, DispatchResult, FixedPointNumber, ModuleId, RuntimeDebug,
};
use sp_std::{cmp::Ordering, result, vec::Vec};

/// Pool metadata
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(Encode, Decode, RuntimeDebug, Eq, PartialEq, Default)]
pub struct Pool {
	/// The amount of underlying currently loaned out by the pool, and the amount upon which
	/// interest is accumulated to suppliers of the pool.
	pub total_borrowed: Balance,

	/// Accumulator of the total earned interest rate since the opening of the pool.
	pub borrow_index: Rate,

	/// Total amount of interest of the underlying held in this pool.
	pub total_protocol_interest: Balance,
}

#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(Encode, Decode, RuntimeDebug, Eq, PartialEq, Default)]
pub struct PoolUserData {
	/// Total balance (with accrued interest), after applying the most
	/// recent balance-changing action.
	pub total_borrowed: Balance,

	/// Global borrow_index as of the most recent balance-changing action.
	pub interest_index: Rate,

	/// Whether or not pool liquidity is used as a collateral.
	pub is_collateral: bool,

	/// Number of partial liquidations for debt
	pub liquidation_attempts: u8,
}

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

type RateResult = result::Result<Rate, DispatchError>;
type BalanceResult = result::Result<Balance, DispatchError>;

#[frame_support::pallet]
pub mod module {
	use super::*;

	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// The `MultiCurrency` implementation.
		type MultiCurrency: MultiCurrency<Self::AccountId, Balance = Balance, CurrencyId = CurrencyId>;

		/// Start exchange rate.
		type InitialExchangeRate: Get<Rate>;

		/// The price source of currencies
		type PriceSource: PricesManager<CurrencyId>;

		#[pallet::constant]
		/// The Liquidity Pool's module id, keep all assets in Pools.
		type ModuleId: Get<ModuleId>;

		#[pallet::constant]
		/// The Liquidity Pool's account id, keep all assets in Pools.
		type LiquidityPoolAccountId: Get<Self::AccountId>;

		#[pallet::constant]
		/// Enabled underlying asset IDs.
		type EnabledUnderlyingAssetsIds: Get<Vec<CurrencyId>>;

		#[pallet::constant]
		/// Enabled wrapped token IDs.
		type EnabledWrappedTokensId: Get<Vec<CurrencyId>>;
	}

	#[pallet::error]
	pub enum Error<T> {
		/// The currency is not enabled in protocol.
		NotValidUnderlyingAssetId,
		/// The currency is not enabled in wrapped protocol.
		NotValidWrappedTokenId,
		/// User is trying to repay more than he borrowed.
		RepayAmountTooBig,
		/// Borrow balance exceeds maximum value.
		BorrowBalanceOverflow,
		/// Exchange rate calculation error.
		ExchangeRateCalculationError,
		/// Conversion error between underlying asset and wrapped token.
		ConversionError,
		/// Pool not found.
		PoolNotFound,
		/// Pool is already created
		PoolAlreadyCreated,
	}

	#[pallet::storage]
	#[pallet::getter(fn pools)]
	pub type Pools<T: Config> = StorageMap<_, Twox64Concat, CurrencyId, Pool, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn pool_user_data)]
	pub type PoolUserParams<T: Config> =
		StorageDoubleMap<_, Blake2_128Concat, CurrencyId, Twox64Concat, T::AccountId, PoolUserData, ValueQuery>;

	#[pallet::genesis_config]
	pub struct GenesisConfig<T: Config> {
		#[allow(clippy::type_complexity)]
		pub pools: Vec<(CurrencyId, Pool)>,
		pub pool_user_data: Vec<(CurrencyId, T::AccountId, PoolUserData)>,
	}

	#[cfg(feature = "std")]
	impl<T: Config> Default for GenesisConfig<T> {
		fn default() -> Self {
			GenesisConfig {
				pools: vec![],
				pool_user_data: vec![],
			}
		}
	}

	#[pallet::genesis_build]
	impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
		fn build(&self) {
			self.pools
				.iter()
				.for_each(|(currency_id, pool)| Pools::<T>::insert(currency_id, Pool { ..*pool }));
			self.pool_user_data
				.iter()
				.for_each(|(currency_id, account_id, pool_user_data)| {
					PoolUserParams::<T>::insert(currency_id, account_id, PoolUserData { ..*pool_user_data })
				});
		}
	}

	#[pallet::pallet]
	pub struct Pallet<T>(PhantomData<T>);

	#[pallet::hooks]
	impl<T: Config> Hooks<T::BlockNumber> for Pallet<T> {}

	#[pallet::call]
	impl<T: Config> Pallet<T> {}
}

// Dispatchable calls implementation
impl<T: Config> Pallet<T> {
	/// Converts a specified number of underlying assets into wrapped tokens.
	/// The calculation is based on the exchange rate.
	///
	/// - `underlying_asset`: CurrencyId of underlying assets to be converted to wrapped tokens.
	/// - `underlying_amount`: The amount of underlying assets to be converted to wrapped tokens.
	/// Returns `wrapped_amount = underlying_amount / exchange_rate`
	pub fn convert_to_wrapped(underlying_asset: CurrencyId, underlying_amount: Balance) -> BalanceResult {
		let exchange_rate = Self::get_exchange_rate(underlying_asset)?;

		let wrapped_amount = Rate::from_inner(underlying_amount)
			.checked_div(&exchange_rate)
			.map(|x| x.into_inner())
			.ok_or(Error::<T>::ConversionError)?;

		Ok(wrapped_amount)
	}

	/// Converts a specified number of wrapped tokens into underlying assets.
	/// The calculation is based on the exchange rate.
	///
	/// - `wrapped_id`: CurrencyId of the wrapped tokens to be converted to underlying assets.
	/// - `wrapped_amount`: The amount of wrapped tokens to be converted to underlying assets.
	///
	/// Returns `underlying_amount = wrapped_amount * exchange_rate`
	pub fn convert_from_wrapped(wrapped_id: CurrencyId, wrapped_amount: Balance) -> BalanceResult {
		let underlying_asset = wrapped_id
			.underlying_asset()
			.ok_or(Error::<T>::NotValidWrappedTokenId)?;
		let exchange_rate = Self::get_exchange_rate(underlying_asset)?;

		let underlying_amount = Rate::from_inner(wrapped_amount)
			.checked_mul(&exchange_rate)
			.map(|x| x.into_inner())
			.ok_or(Error::<T>::ConversionError)?;

		Ok(underlying_amount)
	}

	/// Gets the exchange rate between a mToken and the underlying asset.
	/// This function does not accrue interest before calculating the exchange rate.
	/// - `underlying_asset`: CurrencyId of underlying assets for which the exchange rate
	/// is calculated.
	///
	/// returns `exchange_rate` between a mToken and the underlying asset.
	pub fn get_exchange_rate(underlying_asset: CurrencyId) -> RateResult {
		ensure!(Self::pool_exists(&underlying_asset), Error::<T>::PoolNotFound);
		let wrapped_asset_id = underlying_asset
			.wrapped_asset()
			.ok_or(Error::<T>::NotValidUnderlyingAssetId)?;
		// Current the total amount of cash the pool has.
		let total_cash = Self::get_pool_available_liquidity(underlying_asset);

		// Current total number of tokens in circulation.
		let total_supply = T::MultiCurrency::total_issuance(wrapped_asset_id);

		// Current pool data.
		let pool_data = Self::get_pool_data(underlying_asset);

		let current_exchange_rate = Self::calculate_exchange_rate(
			total_cash,
			total_supply,
			pool_data.total_protocol_interest,
			pool_data.total_borrowed,
		)?;

		Ok(current_exchange_rate)
	}

	/// Calculates the exchange rate from the underlying to the mToken.
	/// - `total_cash`: The total amount of cash the pool has.
	/// - `total_supply`: Total number of tokens in circulation.
	/// - `total_protocol_interest`: Total amount of interest of the underlying held in the pool.
	/// - `total_borrowed`: Total amount of outstanding borrows of the underlying in this pool.
	///
	/// returns `exchange_rate = (total_cash + total_borrowed - total_protocol_interest) /
	/// total_supply`.
	pub fn calculate_exchange_rate(
		total_cash: Balance,
		total_supply: Balance,
		total_protocol_interest: Balance,
		total_borrowed: Balance,
	) -> RateResult {
		let rate = match total_supply.cmp(&Balance::zero()) {
			// If there are no tokens minted: exchange_rate = initial_exchange_rate.
			Ordering::Equal => T::InitialExchangeRate::get(),

			// Otherwise: exchange_rate = (total_cash + total_borrowed - total_protocol_interest) / total_supply
			_ => Rate::saturating_from_rational(
				total_cash
					.checked_add(total_borrowed)
					.and_then(|v| v.checked_sub(total_protocol_interest))
					.ok_or(Error::<T>::ExchangeRateCalculationError)?,
				total_supply,
			),
		};

		Ok(rate)
	}
}

// RPC methods
impl<T: Config> Pallet<T> {
	/// Gets the exchange rate between a mToken and the underlying asset.
	/// This function uses total_protocol_interest and total_borrowed values calculated beforehand
	///
	/// - `underlying_asset`: CurrencyId of underlying assets for which the exchange rate
	/// is calculated.
	pub fn get_exchange_rate_by_interest_params(
		underlying_asset: CurrencyId,
		total_protocol_interest: Balance,
		total_borrowed: Balance,
	) -> RateResult {
		ensure!(Self::pool_exists(&underlying_asset), Error::<T>::PoolNotFound);
		let wrapped_asset_id = underlying_asset
			.wrapped_asset()
			.ok_or(Error::<T>::NotValidUnderlyingAssetId)?;
		// Current the total amount of cash the pool has.
		let total_cash = Self::get_pool_available_liquidity(underlying_asset);

		// Current total number of tokens in circulation.
		let total_supply = T::MultiCurrency::total_issuance(wrapped_asset_id);

		let current_exchange_rate =
			Self::calculate_exchange_rate(total_cash, total_supply, total_protocol_interest, total_borrowed)?;

		Ok(current_exchange_rate)
	}
}

// Storage setters for LiquidityPools
impl<T: Config> Pallet<T> {
	/// Sets pool data
	pub fn set_pool_data(
		pool_id: CurrencyId,
		total_borrowed: Balance,
		borrow_index: Rate,
		total_protocol_interest: Balance,
	) -> DispatchResult {
		Pools::<T>::mutate(pool_id, |pool| {
			pool.total_borrowed = total_borrowed;
			pool.borrow_index = borrow_index;
			pool.total_protocol_interest = total_protocol_interest;
		});
		Ok(())
	}

	/// Sets the total borrowed value in the pool.
	pub fn set_pool_total_borrowed(pool_id: CurrencyId, new_total_borrows: Balance) {
		Pools::<T>::mutate(pool_id, |pool| pool.total_borrowed = new_total_borrows);
	}

	/// Sets the total interest in the pool.
	pub fn set_pool_total_protocol_interest(pool_id: CurrencyId, new_total_protocol_interest: Balance) {
		Pools::<T>::mutate(pool_id, |r| r.total_protocol_interest = new_total_protocol_interest)
	}

	/// Sets the total borrowed and interest index for user.
	pub fn set_user_total_borrowed_and_interest_index(
		who: &T::AccountId,
		pool_id: CurrencyId,
		new_total_borrows: Balance,
		new_interest_index: Rate,
	) {
		PoolUserParams::<T>::mutate(pool_id, who, |p| {
			p.total_borrowed = new_total_borrows;
			p.interest_index = new_interest_index;
		})
	}

	/// Sets the parameter `is_collateral` to `true`.
	pub fn enable_is_collateral_internal(who: &T::AccountId, pool_id: CurrencyId) {
		PoolUserParams::<T>::mutate(pool_id, who, |p| p.is_collateral = true)
	}

	/// Sets the parameter `is_collateral` to `false`.
	pub fn disable_is_collateral_internal(who: &T::AccountId, pool_id: CurrencyId) {
		PoolUserParams::<T>::mutate(pool_id, who, |p| p.is_collateral = false);
	}

	/// This is a part of a pool creation flow
	/// Creates storage records for LiquidationPoolsData
	pub fn create_pool(currency_id: CurrencyId) -> DispatchResult {
		ensure!(!Self::pool_exists(&currency_id), Error::<T>::PoolAlreadyCreated);

		Pools::<T>::insert(
			currency_id,
			Pool {
				total_borrowed: Balance::zero(),
				borrow_index: Rate::one(),
				total_protocol_interest: Balance::zero(),
			},
		);
		Ok(())
	}
}

impl<T: Config> Pallet<T> {
	/// Gets pool associated data
	pub fn get_pool_data(pool_id: CurrencyId) -> Pool {
		Self::pools(pool_id)
	}

	/// Global borrow_index as of the most recent balance-changing action
	pub fn get_user_borrow_index(who: &T::AccountId, pool_id: CurrencyId) -> Rate {
		Self::pool_user_data(pool_id, who).interest_index
	}

	/// Gets total user borrowing.
	pub fn get_user_total_borrowed(who: &T::AccountId, pool_id: CurrencyId) -> Balance {
		Self::pool_user_data(pool_id, who).total_borrowed
	}

	/// Checks if the user has enabled the pool as collateral.
	pub fn check_user_available_collateral(who: &T::AccountId, pool_id: CurrencyId) -> bool {
		Self::pool_user_data(pool_id, who).is_collateral
	}

	// TODO: Maybe implement using a binary tree?
	/// Get list of users with active loan positions for a particular pool.
	pub fn get_pool_members_with_loans(
		underlying_asset: CurrencyId,
	) -> result::Result<Vec<T::AccountId>, DispatchError> {
		let user_vec: Vec<T::AccountId> = PoolUserParams::<T>::iter_prefix(underlying_asset)
			.filter(|(_, pool_user_data)| !pool_user_data.total_borrowed.is_zero())
			.map(|(account, _)| account)
			.collect();
		Ok(user_vec)
	}

	/// Gets user liquidation attempts.
	pub fn get_user_liquidation_attempts(who: &T::AccountId, pool_id: CurrencyId) -> u8 {
		Self::pool_user_data(pool_id, who).liquidation_attempts
	}

	/// Returns an array of collateral pools for the user.
	/// The array is sorted in descending order by the number of wrapped tokens in USD.
	///
	/// - `who`: AccountId for which the pool array is returned.
	pub fn get_is_collateral_pools(who: &T::AccountId) -> result::Result<Vec<CurrencyId>, DispatchError> {
		let mut pools: Vec<(CurrencyId, Balance)> = CurrencyId::get_enabled_tokens_in_protocol(UnderlyingAsset)
			.iter()
			.filter(|&underlying_id| Self::pool_exists(underlying_id))
			.filter_map(|&pool_id| {
				let wrapped_id = pool_id.wrapped_asset()?;

				// only collateral pools.
				if !Self::check_user_available_collateral(&who, pool_id) {
					return None;
				};

				// We calculate the value of the user's wrapped tokens in USD.
				let user_balance_wrapped_tokens = T::MultiCurrency::free_balance(wrapped_id, &who);
				if user_balance_wrapped_tokens.is_zero() {
					return None;
				}
				let user_balance_underlying_asset =
					Self::convert_from_wrapped(wrapped_id, user_balance_wrapped_tokens).ok()?;
				let oracle_price = T::PriceSource::get_underlying_price(pool_id)?;
				let total_deposit_in_usd = Rate::from_inner(user_balance_underlying_asset)
					.checked_mul(&oracle_price)
					.map(|x| x.into_inner())?;

				Some((pool_id, total_deposit_in_usd))
			})
			.collect();

		// Sorted array of pools in descending order.
		pools.sort_by(|x, y| y.1.cmp(&x.1));

		Ok(pools.iter().map(|pool| pool.0).collect::<Vec<CurrencyId>>())
	}

	/// Checks if the user has the collateral.
	pub fn check_user_has_collateral(who: &T::AccountId) -> bool {
		for &pool_id in CurrencyId::get_enabled_tokens_in_protocol(UnderlyingAsset)
			.iter()
			.filter(|&underlying_id| Self::pool_exists(underlying_id))
		{
			if Self::check_user_available_collateral(&who, pool_id) {
				if let Some(wrapped_id) = pool_id.wrapped_asset() {
					if !T::MultiCurrency::free_balance(wrapped_id, &who).is_zero() {
						return true;
					}
				}
			}
		}
		false
	}
}

// Trait Borrowing for LiquidityPools
impl<T: Config> Borrowing<T::AccountId> for Pallet<T> {
	/// Updates the new borrower balance and pool total borrow balances during the borrow operation.
	/// Also sets the global borrow_index to user interest index.
	/// - `who`: The AccountId whose borrow balance should be calculated.
	/// - `pool_id`: PoolID whose total borrow balance should be calculated.
	/// - `borrow_amount`: The amount of the underlying asset to borrow.
	/// - `account_borrows`: The borrow balance of account.
	///
	/// calculates: `account_borrows_new = account_borrows + borrow_amount`,
	///             `total_borrows_new = total_borrows + borrow_amount`.
	fn update_state_on_borrow(
		who: &T::AccountId,
		pool_id: CurrencyId,
		borrow_amount: Balance,
		account_borrows: Balance,
	) -> DispatchResult {
		let pool_data = Self::get_pool_data(pool_id);

		// Calculate the new borrower and total borrow balances, failing on overflow:
		// account_borrows_new = account_borrows + borrow_amount
		// total_borrows_new = total_borrows + borrow_amount
		let account_borrow_new = account_borrows
			.checked_add(borrow_amount)
			.ok_or(Error::<T>::BorrowBalanceOverflow)?;
		let new_total_borrows = pool_data
			.total_borrowed
			.checked_add(borrow_amount)
			.ok_or(Error::<T>::BorrowBalanceOverflow)?;

		// Write the previously calculated values into storage.
		Self::set_pool_total_borrowed(pool_id, new_total_borrows);

		Self::set_user_total_borrowed_and_interest_index(&who, pool_id, account_borrow_new, pool_data.borrow_index);

		Ok(())
	}

	/// Updates the new borrower balance and pool total borrow balances during the repay operation.
	/// Also sets the global borrow_index to user interest index.
	/// - `who`: The AccountId whose borrow balance should be calculated.
	/// - `pool_id`: PoolID whose total borrow balance should be calculated.
	/// - `repay_amount`: The amount of the underlying asset to repay.
	/// - `account_borrows`: The borrow balance of account.
	///
	/// calculates: `account_borrows_new = account_borrows - borrow_amount`,
	///             `total_borrows_new = total_borrows - borrow_amount`.
	fn update_state_on_repay(
		who: &T::AccountId,
		pool_id: CurrencyId,
		repay_amount: Balance,
		account_borrows: Balance,
	) -> DispatchResult {
		let pool_data = Self::get_pool_data(pool_id);

		// Calculate the new borrower and total borrow balances, failing on overflow:
		// account_borrows_new = account_borrows - repay_amount
		// total_borrows_new = total_borrows - repay_amount
		let account_borrow_new = account_borrows
			.checked_sub(repay_amount)
			.ok_or(Error::<T>::RepayAmountTooBig)?;
		let total_borrows_new = pool_data
			.total_borrowed
			.checked_sub(repay_amount)
			.ok_or(Error::<T>::RepayAmountTooBig)?;

		// Write the previously calculated values into storage.
		Self::set_pool_total_borrowed(pool_id, total_borrows_new);
		Self::set_user_total_borrowed_and_interest_index(&who, pool_id, account_borrow_new, pool_data.borrow_index);

		Ok(())
	}
}

impl<T: Config> PoolsManager<T::AccountId> for Pallet<T> {
	/// Gets module account id.
	fn pools_account_id() -> T::AccountId {
		T::ModuleId::get().into_account()
	}

	/// Gets current the total amount of cash the pool has.
	fn get_pool_available_liquidity(pool_id: CurrencyId) -> Balance {
		let module_account_id = Self::pools_account_id();
		T::MultiCurrency::free_balance(pool_id, &module_account_id)
	}

	/// Check if pool exists
	fn pool_exists(underlying_asset: &CurrencyId) -> bool {
		Pools::<T>::contains_key(underlying_asset)
	}
}

impl<T: Config> LiquidityPoolsManager<T::AccountId> for Pallet<T> {
	/// Gets total amount borrowed from the pool.
	fn get_pool_total_borrowed(pool_id: CurrencyId) -> Balance {
		Self::pools(pool_id).total_borrowed
	}

	/// Gets current total amount of protocol interest of the underlying held in this pool.
	fn get_pool_total_protocol_interest(pool_id: CurrencyId) -> Balance {
		Self::pools(pool_id).total_protocol_interest
	}

	/// Accumulator of the total earned interest rate since the opening of the pool
	fn get_pool_borrow_index(pool_id: CurrencyId) -> Rate {
		Self::pools(pool_id).borrow_index
	}
}
