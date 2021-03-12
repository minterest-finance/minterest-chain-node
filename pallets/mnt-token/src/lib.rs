//! # MNT token Module
//!
//! TODO: Add overview

#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::unused_unit)]

use frame_support::{pallet_prelude::*, transactional};
use frame_system::pallet_prelude::*;
use minterest_primitives::Rate;
use sp_runtime::FixedPointNumber;

mod mock;
mod tests;

pub use module::*;

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
	#[pallet::generate_deposit(fn deposit_event)]
	pub enum Event<T: Config> {
		/// Change rate event (old_rate, new_rate)
		NewMntRate(Rate, Rate),
	}

	#[pallet::storage]
	#[pallet::getter(fn mnt_rate)]
	type MntRate<T: Config> = StorageValue<_, Rate, ValueQuery>;

	#[pallet::genesis_config]
	pub struct GenesisConfig {
		pub mnt_rate: Rate,
	}

	#[cfg(feature = "std")]
	impl Default for GenesisConfig {
		fn default() -> Self {
			GenesisConfig { mnt_rate: Rate::zero() }
		}
	}

	#[pallet::genesis_build]
	impl<T: Config> GenesisBuild<T> for GenesisConfig {
		fn build(&self) {
			MntRate::<T>::put(&self.mnt_rate);
		}
	}

	#[pallet::pallet]
	pub struct Pallet<T>(PhantomData<T>);

	#[pallet::hooks]
	impl<T: Config> Hooks<T::BlockNumber> for Pallet<T> {}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::weight(10_000)]
		#[transactional] // TODO ask about this
		pub fn set_mnt_rate(origin: OriginFor<T>, new_rate: Rate) -> DispatchResultWithPostInfo {
			ensure_root(origin)?;
			let old_rate = MntRate::<T>::get();
			MntRate::<T>::put(new_rate);
			Self::deposit_event(Event::NewMntRate(old_rate, new_rate));
			Ok(().into())
		}
	}
}

impl<T: Config> Pallet<T> {}
