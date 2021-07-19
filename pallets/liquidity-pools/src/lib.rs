//! # Liquidity Pools Pallet
//!
//! ## Overview
//!
//! This pallet stores data about user and liquidity pools. The account of this pallet contains all
//! liquidity that is deposited by users in the protocol. Also This pallet is managing
//! information required for interest calculation.
//!
//! ## Interface
//!
//! Implements the public API in the form of the following traits:
//! -`PoolsManager`: an abstraction of pools basic functionalities.
//! -`LiquidityPoolStorageProvider`: provides functionality for working with storage of
//! liquidity pools.
//! -`UserStorageProvider`: provides functionality for working with a user's storage.
//! Set parameters in storage, get parameters, check parameters.
//! -`CurrencyConverter`: used to get the exchange rate between underlying assets and wrapped
//! tokens. This trait also provides functionality for converting between mTokens, underlying
//! assets and USD.

#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::unused_unit)]
#![allow(clippy::upper_case_acronyms)]

use codec::{Decode, Encode};
use frame_support::{pallet_prelude::*, traits::Get, PalletId};
use minterest_primitives::{currency::CurrencyType::UnderlyingAsset, Balance, CurrencyId, Price, Rate};
pub use module::*;
use orml_traits::MultiCurrency;
use pallet_traits::{
	Borrowing, CurrencyConverter, LiquidityPoolStorageProvider, PoolsManager, PricesManager, UserCollateral,
	UserStorageProvider,
};
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};
use sp_runtime::{
	traits::{AccountIdConversion, CheckedDiv, CheckedMul, One, Zero},
	DispatchError, DispatchResult, FixedPointNumber, RuntimeDebug,
};
use sp_std::{result, vec::Vec};

/// Pool metadata
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(Encode, Decode, RuntimeDebug, Eq, PartialEq, Default)]
pub struct Pool {
	/// The amount of underlying currently loaned out by the pool, and the amount upon which
	/// interest is accumulated to suppliers of the pool.
	pub borrowed: Balance,

	/// Accumulator of the total earned interest rate since the opening of the pool.
	pub borrow_index: Rate,

	/// Total amount of interest of the underlying held in this pool.
	pub protocol_interest: Balance,
}

#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(Encode, Decode, RuntimeDebug, Eq, PartialEq, Default)]
pub struct PoolUserData {
	/// Total balance (with accrued interest), after applying the most
	/// recent balance-changing action.
	pub borrowed: Balance,

	/// Global borrow_index as of the most recent balance-changing action.
	pub interest_index: Rate,

	/// Whether or not pool liquidity is used as a collateral.
	pub is_collateral: bool,
}

type RateResult = result::Result<Rate, DispatchError>;
type BalanceResult = result::Result<Balance, DispatchError>;

#[cfg(test)]
mod tests;

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
		type PalletId: Get<PalletId>;

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
	pub(crate) type Pools<T: Config> = StorageMap<_, Twox64Concat, CurrencyId, Pool, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn pool_user_data)]
	pub(crate) type PoolUserParams<T: Config> =
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

// Private functions
impl<T: Config> Pallet<T> {
	/// Calculates the exchange rate from the underlying to the mToken.
	/// - `pool_supply_underlying`: The total amount of underlying tokens the liquidity pool has.
	/// - `pool_supply_wrap`: Total number of wrapped tokens in circulation.
	/// - `pool_protocol_interest`: Total amount of interest of the underlying held in the pool.
	/// - `pool_borrow_underlying`: Total amount of outstanding borrows of the underlying in this
	/// pool.
	///
	/// returns `exchange_rate = (pool_supply_underlying + pool_borrow_underlying -
	/// - pool_protocol_interest) / pool_supply_wrap`.
	pub fn calculate_exchange_rate(
		pool_supply_underlying: Balance,
		pool_supply_wrap: Balance,
		pool_protocol_interest: Balance,
		pool_borrowed: Balance,
	) -> RateResult {
		let exchange_rate = match pool_supply_wrap.is_zero() {
			// If there are no tokens minted: exchange_rate = initial_exchange_rate.
			true => T::InitialExchangeRate::get(),

			// Otherwise: exchange_rate = (pool_supply_underlying + pool_borrow_underlying -
			// - pool_protocol_interest) / pool_supply_wrap
			_ => Rate::saturating_from_rational(
				pool_supply_underlying
					.checked_add(pool_borrowed)
					.and_then(|v| v.checked_sub(pool_protocol_interest))
					.ok_or(Error::<T>::ExchangeRateCalculationError)?,
				pool_supply_wrap,
			),
		};

		Ok(exchange_rate)
	}
}

impl<T: Config> UserStorageProvider<T::AccountId, PoolUserData> for Pallet<T> {
	fn set_user_data(who: &T::AccountId, pool_id: CurrencyId, pool_user_data: PoolUserData) {
		PoolUserParams::<T>::insert(pool_id, who, pool_user_data)
	}

	fn set_user_borrow_and_interest_index(
		who: &T::AccountId,
		pool_id: CurrencyId,
		new_borrow_underlying: Balance,
		new_interest_index: Rate,
	) {
		PoolUserParams::<T>::mutate(pool_id, who, |p| {
			p.borrowed = new_borrow_underlying;
			p.interest_index = new_interest_index;
		})
	}

	fn get_user_data(pool_id: CurrencyId, who: &T::AccountId) -> PoolUserData {
		Self::pool_user_data(pool_id, who)
	}

	fn get_user_borrow_index(who: &T::AccountId, pool_id: CurrencyId) -> Rate {
		Self::pool_user_data(pool_id, who).interest_index
	}

	fn get_user_borrow_balance(who: &T::AccountId, pool_id: CurrencyId) -> Balance {
		Self::pool_user_data(pool_id, who).borrowed
	}
}

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
			.borrowed
			.checked_add(borrow_amount)
			.ok_or(Error::<T>::BorrowBalanceOverflow)?;

		// Write the previously calculated values into storage.
		Self::set_pool_borrow_underlying(pool_id, new_total_borrows);

		Self::set_user_borrow_and_interest_index(&who, pool_id, account_borrow_new, pool_data.borrow_index);

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
			.borrowed
			.checked_sub(repay_amount)
			.ok_or(Error::<T>::RepayAmountTooBig)?;

		// Write the previously calculated values into storage.
		Self::set_pool_borrow_underlying(pool_id, total_borrows_new);
		Self::set_user_borrow_and_interest_index(&who, pool_id, account_borrow_new, pool_data.borrow_index);

		Ok(())
	}
}

impl<T: Config> PoolsManager<T::AccountId> for Pallet<T> {
	/// Gets module account id.
	fn pools_account_id() -> T::AccountId {
		T::PalletId::get().into_account()
	}

	/// Gets current the total amount of cash the pool has.
	fn get_pool_available_liquidity(pool_id: CurrencyId) -> Balance {
		let module_account_id = Self::pools_account_id();
		T::MultiCurrency::free_balance(pool_id, &module_account_id)
	}
}

impl<T: Config> LiquidityPoolStorageProvider<T::AccountId, Pool> for Pallet<T> {
	fn set_pool_data(pool_id: CurrencyId, pool_data: Pool) {
		Pools::<T>::insert(pool_id, pool_data)
	}

	fn set_pool_borrow_underlying(pool_id: CurrencyId, new_pool_borrows: Balance) {
		Pools::<T>::mutate(pool_id, |pool| pool.borrowed = new_pool_borrows);
	}

	fn set_pool_protocol_interest(pool_id: CurrencyId, new_pool_protocol_interest: Balance) {
		Pools::<T>::mutate(pool_id, |r| r.protocol_interest = new_pool_protocol_interest)
	}

	fn get_pool_data(pool_id: CurrencyId) -> Pool {
		Self::pools(pool_id)
	}

	fn get_pool_members_with_loans(underlying_asset: CurrencyId) -> result::Result<Vec<T::AccountId>, DispatchError> {
		let user_vec: Vec<T::AccountId> = PoolUserParams::<T>::iter_prefix(underlying_asset)
			.filter(|(_, pool_user_data)| !pool_user_data.borrowed.is_zero())
			.map(|(account, _)| account)
			.collect();
		Ok(user_vec)
	}

	fn get_pool_borrow_underlying(pool_id: CurrencyId) -> Balance {
		Self::pools(pool_id).borrowed
	}

	fn get_pool_borrow_index(pool_id: CurrencyId) -> Rate {
		Self::pools(pool_id).borrow_index
	}

	fn get_pool_protocol_interest(pool_id: CurrencyId) -> Balance {
		Self::pools(pool_id).protocol_interest
	}

	fn pool_exists(underlying_asset: &CurrencyId) -> bool {
		Pools::<T>::contains_key(underlying_asset)
	}

	fn create_pool(currency_id: CurrencyId) -> DispatchResult {
		ensure!(!Self::pool_exists(&currency_id), Error::<T>::PoolAlreadyCreated);

		Pools::<T>::insert(
			currency_id,
			Pool {
				borrowed: Balance::zero(),
				borrow_index: Rate::one(),
				protocol_interest: Balance::zero(),
			},
		);
		Ok(())
	}

	fn remove_pool_data(pool_id: CurrencyId) {
		Pools::<T>::remove(pool_id)
	}
}

impl<T: Config> CurrencyConverter for Pallet<T> {
	/// Gets the exchange rate between a wrapped token and the underlying asset.
	///
	/// returns `exchange_rate = (pool_supply_underlying + pool_borrow_underlying -
	/// - pool_protocol_interest) / pool_supply_wrap`.
	fn get_exchange_rate(underlying_asset: CurrencyId) -> RateResult {
		ensure!(Self::pool_exists(&underlying_asset), Error::<T>::PoolNotFound);

		let wrapped_asset_id = underlying_asset
			.wrapped_asset()
			.ok_or(Error::<T>::NotValidUnderlyingAssetId)?;

		let pool_supply_underlying = Self::get_pool_available_liquidity(underlying_asset);
		let pool_supply_wrap = T::MultiCurrency::total_issuance(wrapped_asset_id);
		let pool_data = Self::get_pool_data(underlying_asset);

		Self::calculate_exchange_rate(
			pool_supply_underlying,
			pool_supply_wrap,
			pool_data.protocol_interest,
			pool_data.borrowed,
		)
	}

	/// Converts a specified number of underlying assets into wrapped tokens.
	fn underlying_to_wrapped(underlying_amount: Balance, exchange_rate: Rate) -> BalanceResult {
		let wrapped_amount = Rate::from_inner(underlying_amount)
			.checked_div(&exchange_rate)
			.map(|x| x.into_inner())
			.ok_or(Error::<T>::ConversionError)?;
		Ok(wrapped_amount)
	}

	/// Converts a specified number of underlying assets into USD.
	fn underlying_to_usd(underlying_amount: Balance, oracle_price: Price) -> BalanceResult {
		let usd_amount = Rate::from_inner(underlying_amount)
			.checked_mul(&oracle_price)
			.map(|x| x.into_inner())
			.ok_or(Error::<T>::ConversionError)?;
		Ok(usd_amount)
	}

	/// Converts a specified number of wrapped tokens into underlying assets.
	fn wrapped_to_underlying(wrapped_amount: Balance, exchange_rate: Rate) -> BalanceResult {
		let underlying_amount = Rate::from_inner(wrapped_amount)
			.checked_mul(&exchange_rate)
			.map(|x| x.into_inner())
			.ok_or(Error::<T>::ConversionError)?;
		Ok(underlying_amount)
	}

	/// Converts a specified number of wrapped tokens into USD.
	fn wrapped_to_usd(
		wrapped_amount: Balance,
		exchange_rate: Rate,
		oracle_price: Price,
	) -> Result<Balance, DispatchError> {
		let underlying_amount = Self::wrapped_to_underlying(wrapped_amount, exchange_rate)?;
		let usd_amount = Self::underlying_to_usd(underlying_amount, oracle_price)?;
		Ok(usd_amount)
	}

	/// Converts a specified number of USD into underlying assets.
	fn usd_to_underlying(usd_amount: Balance, oracle_price: Price) -> Result<Balance, DispatchError> {
		let underlying_amount = Rate::from_inner(usd_amount)
			.checked_div(&oracle_price)
			.map(|x| x.into_inner())
			.ok_or(Error::<T>::ConversionError)?;
		Ok(underlying_amount)
	}

	/// Converts a specified amount of USD into wrapped tokens.
	fn usd_to_wrapped(usd_amount: Balance, exchange_rate: Rate, oracle_price: Price) -> Result<Balance, DispatchError> {
		let underlying_amount = Self::usd_to_underlying(usd_amount, oracle_price)?;
		let wrapped_amount = Self::underlying_to_wrapped(underlying_amount, exchange_rate)?;
		Ok(wrapped_amount)
	}
}

impl<T: Config> UserCollateral<T::AccountId> for Pallet<T> {
	fn get_user_collateral_pools(who: &T::AccountId) -> result::Result<Vec<CurrencyId>, DispatchError> {
		let mut pools: Vec<(CurrencyId, Balance)> = CurrencyId::get_enabled_tokens_in_protocol(UnderlyingAsset)
			.iter()
			.filter(|&underlying_id| Self::pool_exists(underlying_id) && Self::is_pool_collateral(&who, *underlying_id))
			.filter_map(|&pool_id| {
				let wrapped_id = pool_id.wrapped_asset()?;

				// We calculate the value of the user's wrapped tokens in USD.
				let user_supply_wrap = T::MultiCurrency::free_balance(wrapped_id, &who);
				if user_supply_wrap.is_zero() {
					return None;
				}
				let exchange_rate = Self::get_exchange_rate(pool_id).ok()?;
				let oracle_price = T::PriceSource::get_underlying_price(pool_id)?;
				let user_supply_in_usd = Self::wrapped_to_usd(user_supply_wrap, exchange_rate, oracle_price).ok()?;

				Some((pool_id, user_supply_in_usd))
			})
			.collect();

		// Sorted array of pools in descending order.
		pools.sort_by(|x, y| y.1.cmp(&x.1));

		Ok(pools.iter().map(|pool| pool.0).collect::<Vec<CurrencyId>>())
	}

	fn is_pool_collateral(who: &T::AccountId, pool_id: CurrencyId) -> bool {
		Self::pool_user_data(pool_id, who).is_collateral
	}

	fn check_user_has_collateral(who: &T::AccountId) -> bool {
		for &pool_id in CurrencyId::get_enabled_tokens_in_protocol(UnderlyingAsset)
			.iter()
			.filter(|&underlying_id| Self::pool_exists(underlying_id) && Self::is_pool_collateral(&who, *underlying_id))
		{
			if let Some(wrapped_id) = pool_id.wrapped_asset() {
				if !T::MultiCurrency::free_balance(wrapped_id, &who).is_zero() {
					return true;
				}
			}
		}
		false
	}

	fn enable_is_collateral(who: &T::AccountId, pool_id: CurrencyId) {
		PoolUserParams::<T>::mutate(pool_id, who, |p| p.is_collateral = true)
	}

	fn disable_is_collateral(who: &T::AccountId, pool_id: CurrencyId) {
		PoolUserParams::<T>::mutate(pool_id, who, |p| p.is_collateral = false);
	}
}
