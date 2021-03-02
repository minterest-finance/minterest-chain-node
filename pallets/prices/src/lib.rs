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
use pallet_traits::PriceProvider;

pub use module::*;
use sp_runtime::traits::CheckedDiv;

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
		/// Stub price. \[currency_id, stubbed_price\]
		StubPrice(CurrencyId, Price),
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

impl<T: Config> PriceProvider<CurrencyId> for Pallet<T> {
	/// Get relative price between two currency types.
	///
	/// - `base_currency_id` - The CurrencyId of the first token.
	/// - `quote_currency_id` -  The CurrencyId of the second token.
	///
	/// Returns base_price / quote_price.
	fn get_relative_price(base_currency_id: CurrencyId, quote_currency_id: CurrencyId) -> Option<Price> {
		if let (Some(base_price), Some(quote_price)) = (
			Self::get_underlying_price(base_currency_id),
			Self::get_underlying_price(quote_currency_id),
		) {
			base_price.checked_div(&quote_price)
		} else {
			None
		}
	}

	/// Get price underlying token in USD.
	fn get_underlying_price(currency_id: CurrencyId) -> Option<Price> {
		// if locked price exists, return it, otherwise return latest price from oracle.
		Self::locked_price(currency_id).or_else(|| T::Source::get(&currency_id))
	}

	/// Locks price when get valid price from source.
	fn lock_price(currency_id: CurrencyId) {
		// lock price when get valid price from source
		if let Some(val) = T::Source::get(&currency_id) {
			LockedPrice::<T>::insert(currency_id, val);
			<Pallet<T>>::deposit_event(Event::LockPrice(currency_id, val));
		}
	}

	/// Unlocks price when get valid price from source.
	fn unlock_price(currency_id: CurrencyId) {
		LockedPrice::<T>::remove(currency_id);
		<Pallet<T>>::deposit_event(Event::UnlockPrice(currency_id));
	}
}
