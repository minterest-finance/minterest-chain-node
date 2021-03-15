//! # DEX Module
//!
//! ## Overview
//!
//! TODO: add overview.

#![cfg_attr(not(feature = "std"), no_std)]
#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::unused_unit)]

use frame_support::{pallet_prelude::*, transactional};
use frame_system::pallet_prelude::*;
use minterest_primitives::{Balance, CurrencyId};
use pallet_traits::DEXManager;

mod mock;
mod tests;

pub use module::*;
use sp_runtime::traits::Zero;

#[frame_support::pallet]
pub mod module {
	use super::*;

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
	}

	#[pallet::error]
	pub enum Error<T> {}

	#[pallet::event]
	#[pallet::generate_deposit(pub(crate) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Use supply currency to swap target currency. \[trader, supply_currency_id,
		/// target_currency_id supply_currency_amount, target_currency_amount\]
		Swap(T::AccountId, CurrencyId, CurrencyId, Balance, Balance),
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
		supply_currency_id: CurrencyId,
		target_currency_id: CurrencyId,
		_supply_amount: Balance,
		_min_target_amount: Balance,
	) -> sp_std::result::Result<Balance, DispatchError> {
		let actual_supply_amount = Balance::zero();
		let actual_target_amount = Balance::zero();

		Self::deposit_event(Event::Swap(
			who.clone(),
			supply_currency_id,
			target_currency_id,
			actual_supply_amount,
			actual_target_amount,
		));

		Ok(actual_supply_amount)
	}

	/// Ensured atomic.
	#[transactional]
	fn do_swap_with_exact_target(
		who: &T::AccountId,
		supply_currency_id: CurrencyId,
		target_currency_id: CurrencyId,
		_target_amount: Balance,
		_max_supply_amount: Balance,
	) -> sp_std::result::Result<Balance, DispatchError> {
		let actual_supply_amount = Balance::zero();
		let actual_target_amount = Balance::zero();

		Self::deposit_event(Event::Swap(
			who.clone(),
			supply_currency_id,
			target_currency_id,
			actual_supply_amount,
			actual_target_amount,
		));

		Ok(actual_supply_amount)
	}
}

impl<T: Config> DEXManager<T::AccountId, CurrencyId, Balance> for Pallet<T> {
	fn swap_with_exact_supply(
		who: &T::AccountId,
		supply_currency_id: CurrencyId,
		target_currency_id: CurrencyId,
		supply_amount: Balance,
		min_target_amount: Balance,
	) -> sp_std::result::Result<Balance, DispatchError> {
		Self::do_swap_with_exact_supply(
			who,
			supply_currency_id,
			target_currency_id,
			supply_amount,
			min_target_amount,
		)
	}

	fn swap_with_exact_target(
		who: &T::AccountId,
		supply_currency_id: CurrencyId,
		target_currency_id: CurrencyId,
		target_amount: Balance,
		max_supply_amount: Balance,
	) -> sp_std::result::Result<Balance, DispatchError> {
		Self::do_swap_with_exact_target(
			who,
			supply_currency_id,
			target_currency_id,
			target_amount,
			max_supply_amount,
		)
	}
}
