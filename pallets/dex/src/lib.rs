//! # DEX Module
//!
//! ## Overview
//!
//! This is a pallet for trading tokens with DeXes. May be used when balancing Liquidation pools or
//! buying back MNT tokens for re-distribution.

#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::unused_unit)]

use frame_support::{pallet_prelude::*, transactional, PalletId};
use minterest_primitives::{Balance, OriginalAsset, CurrencyId};
pub use module::*;
use orml_traits::MultiCurrency;
use pallet_traits::DEXManager;
use sp_runtime::traits::{AccountIdConversion, Zero};

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

#[frame_support::pallet]
pub mod module {
	use super::*;

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

		/// The `MultiCurrency` implementation.
		type MultiCurrency: MultiCurrency<Self::AccountId, Balance = Balance, CurrencyId = CurrencyId>;

		#[pallet::constant]
		/// The Dex module id.
		type DexPalletId: Get<PalletId>;

		#[pallet::constant]
		/// The Dex account id.
		type DexAccountId: Get<Self::AccountId>;
	}

	#[pallet::error]
	pub enum Error<T> {
		/// Insufficient available dex balance.
		InsufficientDexBalance,
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(crate) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Use supply currency to swap target currency. \[trader, supply_asset,
		/// target_asset supply_currency_amount, target_currency_amount\]
		Swap(T::AccountId, OriginalAsset, OriginalAsset, Balance, Balance),
	}

	#[pallet::pallet]
	pub struct Pallet<T>(PhantomData<T>);

	#[pallet::hooks]
	impl<T: Config> Hooks<T::BlockNumber> for Pallet<T> {}

	#[pallet::call]
	impl<T: Config> Pallet<T> {}
}

impl<T: Config> Pallet<T> {
	/// Ensured atomic.
	#[transactional]
	fn do_swap_with_exact_supply(
		who: &T::AccountId,
		supply_asset: OriginalAsset,
		target_asset: OriginalAsset,
		_supply_amount: Balance,
		_min_target_amount: Balance,
	) -> sp_std::result::Result<Balance, DispatchError> {
		let actual_supply_amount = Balance::zero();
		let actual_target_amount = Balance::zero();

		Self::deposit_event(Event::Swap(
			who.clone(),
			supply_asset,
			target_asset,
			actual_supply_amount,
			actual_target_amount,
		));

		Ok(actual_supply_amount)
	}

	/// Ensured atomic.
	///
	/// TODO Temporary implementation. Makes an exchange at the rate of 1:1
	/// (for example: 1 ETH = 1 BTC)
	#[transactional]
	pub fn do_swap_with_exact_target(
		who: &T::AccountId,
		supply_asset: OriginalAsset,
		target_asset: OriginalAsset,
		max_supply_amount: Balance,
		target_amount: Balance,
	) -> sp_std::result::Result<Balance, DispatchError> {
		let target_dex_balance = Self::get_dex_available_liquidity(target_asset);
		let module_account_id = Self::dex_account_id();

		ensure!(target_dex_balance >= target_amount, Error::<T>::InsufficientDexBalance);

		T::MultiCurrency::transfer(supply_asset.into(), &who, &module_account_id, max_supply_amount)?;
		T::MultiCurrency::transfer(target_asset.into(), &module_account_id, &who, target_amount)?;

		Self::deposit_event(Event::Swap(
			who.clone(),
			supply_asset,
			target_asset,
			max_supply_amount,
			target_amount,
		));

		Ok(target_amount)
	}
}

impl<T: Config> Pallet<T> {
	/// Gets module account id.
	pub fn dex_account_id() -> T::AccountId {
		T::DexPalletId::get().into_account()
	}

	/// Gets current the total amount of cash the dex has.
	fn get_dex_available_liquidity(asset: OriginalAsset) -> Balance {
		let module_account_id = Self::dex_account_id();
		T::MultiCurrency::free_balance(asset.into(), &module_account_id)
	}
}

impl<T: Config> DEXManager<T::AccountId, Balance> for Pallet<T> {
	fn swap_with_exact_supply(
		who: &T::AccountId,
		supply_asset: OriginalAsset,
		target_asset: OriginalAsset,
		supply_amount: Balance,
		min_target_amount: Balance,
	) -> sp_std::result::Result<Balance, DispatchError> {
		Self::do_swap_with_exact_supply(
			who,
			supply_asset,
			target_asset,
			supply_amount,
			min_target_amount,
		)
	}

	fn swap_with_exact_target(
		who: &T::AccountId,
		supply_asset: OriginalAsset,
		target_asset: OriginalAsset,
		max_supply_amount: Balance,
		target_amount: Balance,
	) -> sp_std::result::Result<Balance, DispatchError> {
		Self::do_swap_with_exact_target(
			who,
			supply_asset,
			target_asset,
			max_supply_amount,
			target_amount,
		)
	}
}
