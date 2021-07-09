//! # Risk Manager Pallet
//!
//! ## Overview
//!
//! TODO

#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::unused_unit)]
#![allow(clippy::upper_case_acronyms)]

use frame_support::pallet_prelude::*;
use minterest_primitives::CurrencyId;
pub use module::*;
use pallet_traits::{UserCollateral, UserLiquidationAttemptsManager};
use sp_runtime::traits::{One, Zero};
#[cfg(feature = "std")]
use sp_std::str;

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

		/// Provides functionality for working with a user's collateral pools.
		type UserCollateral: UserCollateral<Self::AccountId>;
	}

	#[pallet::error]
	pub enum Error<T> {}

	#[pallet::event]
	#[pallet::generate_deposit(pub(crate) fn deposit_event)]
	pub enum Event<T: Config> {}

	/// Counter of the number of partial liquidations at the user.
	#[pallet::storage]
	#[pallet::getter(fn user_liquidation_attempts)]
	pub(crate) type UserLiquidationAttempts<T: Config> = StorageMap<_, Twox64Concat, T::AccountId, u8, ValueQuery>;

	#[pallet::genesis_config]
	pub struct GenesisConfig<T: Config> {
		pub _phantom: sp_std::marker::PhantomData<T>,
	}

	#[cfg(feature = "std")]
	impl<T: Config> Default for GenesisConfig<T> {
		fn default() -> Self {
			GenesisConfig { _phantom: PhantomData }
		}
	}

	#[pallet::genesis_build]
	impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
		fn build(&self) {}
	}

	#[pallet::pallet]
	pub struct Pallet<T>(PhantomData<T>);

	#[pallet::hooks]
	impl<T: Config> Hooks<T::BlockNumber> for Pallet<T> {}

	#[pallet::call]
	impl<T: Config> Pallet<T> {}
}

impl<T: Config> Pallet<T> {}

impl<T: Config> UserLiquidationAttemptsManager<T::AccountId> for Pallet<T> {
	fn get_user_liquidation_attempts(who: &T::AccountId) -> u8 {
		Self::user_liquidation_attempts(who)
	}

	fn increase_by_one(who: &T::AccountId) {
		UserLiquidationAttempts::<T>::mutate(who, |p| *p += u8::one())
	}

	fn reset_to_zero(who: &T::AccountId) {
		UserLiquidationAttempts::<T>::mutate(&who, |p| *p = u8::zero())
	}

	fn mutate_upon_deposit(pool_id: CurrencyId, who: &T::AccountId) {
		if T::UserCollateral::is_pool_collateral(&who, pool_id) {
			let user_liquidation_attempts = Self::get_user_liquidation_attempts(&who);
			if !user_liquidation_attempts.is_zero() {
				Self::reset_to_zero(&who);
			}
		}
	}
}
