//! # Example Module
//!
//! A simple example of a FRAME pallet demonstrating
//! concepts, APIs and structures common to most FRAME runtimes.

#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::unused_unit)]

use frame_support::pallet_prelude::*;
use frame_system::pallet_prelude::*;
use minterest_primitives::{CurrencyId, Price};
use orml_traits::DataProvider;

mod mock;
mod tests;

pub use module::*;

#[frame_support::pallet]
pub mod module {
	use super::*;

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type Balance: Parameter + codec::HasCompact + From<u32> + Into<Weight> + Default + MaybeSerializeDeserialize;
		#[pallet::constant]
		type SomeConst: Get<Self::Balance>;
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
	}

	#[pallet::error]
	pub enum Error<T> {
		/// Some wrong behavior
		Wrong,
	}

	#[pallet::event]
	#[pallet::generate_deposit(fn deposit_event)]
	pub enum Event<T: Config> {}

	#[pallet::type_value]
	pub fn OnFooEmpty<T: Config>() -> T::Balance {
		3.into()
	}

	#[pallet::pallet]
	pub struct Pallet<T>(PhantomData<T>);

	#[pallet::hooks]
	impl<T: Config> Hooks<T::BlockNumber> for Pallet<T> {}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::weight(<T::Balance as Into<Weight>>::into(new_value.clone()))]
		pub fn set_dummy(origin: OriginFor<T>, #[pallet::compact] new_value: T::Balance) -> DispatchResultWithPostInfo {
			Ok(().into())
		}
	}
}

impl<T: Config> DataProvider<CurrencyId, Price> for Pallet<T> {
	fn get(key: &CurrencyId) -> Option<Price> {
		unimplemented!();
	}
}
