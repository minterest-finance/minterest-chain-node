//! # Prices Module
//!
//! ## Overview
//!
//! The data from Oracle cannot be used in business, prices module will do some
//! process and feed prices for Minterest. Process include:
//!   - specify a fixed price for stable currency;
//!   - feed price in USD or related price between two currencies;
//!   - lock/unlock the price data get from oracle.

#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::unused_unit)]
#![allow(clippy::upper_case_acronyms)]

use frame_support::pallet_prelude::*;
use minterest_primitives::{CurrencyId, Price};
use orml_traits::{DataFeeder, DataProvider};
use sp_runtime::DispatchError;
use sp_std::result;

pub use module::*;

type PriceResult = result::Result<Price, DispatchError>;

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

#[frame_support::pallet]
pub mod module {
	use super::*;

	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// The overarching event type.
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

		/// The data source, such as Oracle.
		type Source: DataProvider<CurrencyId, Price> + DataFeeder<CurrencyId, Price, Self::AccountId>;
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(crate) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Lock price. \[currency_id, locked_price\]
		LockPrice(CurrencyId, Price),
		/// Unlock price. \[currency_id\]
		UnlockPrice(CurrencyId),
	}

	/// Mapping from currency id to it's locked price
	#[pallet::storage]
	#[pallet::getter(fn locked_price)]
	pub type LockedPrice<T: Config> = StorageMap<_, Twox64Concat, CurrencyId, Price, OptionQuery>;

	#[pallet::pallet]
	pub struct Pallet<T>(PhantomData<T>);

	#[pallet::hooks]
	impl<T: Config> Hooks<T::BlockNumber> for Pallet<T> {}

	#[pallet::call]
	impl<T: Config> Pallet<T> {}
}
impl<T: Config> Pallet<T> {
	pub fn get_underlying_price(_underlying_asset_id: CurrencyId) -> PriceResult {
		let price_two_dollars = 2_00u128 * 10_000_000_000_000_000;
		Ok(Price::from_inner(price_two_dollars)) // Price = 2.00 USD
	}
}
