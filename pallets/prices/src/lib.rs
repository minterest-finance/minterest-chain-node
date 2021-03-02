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

use frame_support::{pallet_prelude::*, transactional};
use minterest_primitives::{CurrencyId, Price};
use orml_traits::{DataFeeder, DataProvider, GetByKey};
use pallet_traits::PriceProvider;

pub use module::*;
use sp_runtime::traits::CheckedDiv;
use sp_runtime::FixedPointNumber;

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

#[frame_support::pallet]
pub mod module {
	use super::*;
	use frame_system::pallet_prelude::OriginFor;
	use orml_traits::GetByKey;

	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// The overarching event type.
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

		/// The data source, such as Oracle.
		type Source: DataProvider<CurrencyId, Price> + DataFeeder<CurrencyId, Price, Self::AccountId>;

		/// The origin which may lock and unlock prices feed to system.
		type LockOrigin: EnsureOrigin<Self::Origin>;

		/// Almost all oracles feed prices based on the natural `1` of tokens,
		/// it's necessary to handle prices with decimals.
		type TokenDecimals: GetByKey<CurrencyId, u32>;
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
	impl<T: Config> Pallet<T> {
		/// Lock the price and feed it to system.
		///
		/// The dispatch origin of this call must be `LockOrigin`.
		///
		/// - `currency_id`: currency type.
		#[pallet::weight((10_000, DispatchClass::Operational))]
		#[transactional]
		pub fn lock_price(origin: OriginFor<T>, currency_id: CurrencyId) -> DispatchResultWithPostInfo {
			T::LockOrigin::ensure_origin(origin)?;
			<Pallet<T> as PriceProvider<CurrencyId>>::lock_price(currency_id);
			Ok(().into())
		}

		/// Unlock the price and get the price from `PriceProvider` again
		///
		/// The dispatch origin of this call must be `LockOrigin`.
		///
		/// - `currency_id`: currency type.
		#[pallet::weight((10_000, DispatchClass::Operational))]
		#[transactional]
		pub fn unlock_price(origin: OriginFor<T>, currency_id: CurrencyId) -> DispatchResultWithPostInfo {
			T::LockOrigin::ensure_origin(origin)?;
			<Pallet<T> as PriceProvider<CurrencyId>>::unlock_price(currency_id);
			Ok(().into())
		}
	}
}

impl<T: Config> PriceProvider<CurrencyId> for Pallet<T> {
	/// Get relative price between two currency types.
	///
	/// - `base_currency_id` - The CurrencyId of the first token.
	/// - `quote_currency_id` -  The CurrencyId of the second token.
	///
	/// Returns base_price / quote_price.
	fn get_relative_price(base_currency_id: CurrencyId, quote_currency_id: CurrencyId) -> Option<Price> {
		match (
			Self::get_underlying_price(base_currency_id),
			Self::get_underlying_price(quote_currency_id),
		) {
			(Some(base_price), Some(quote_price)) => base_price.checked_div(&quote_price),
			_ => None,
		}
	}

	/// Get price underlying token in USD.
	/// Note: this returns the price for 1 basic unit
	fn get_underlying_price(currency_id: CurrencyId) -> Option<Price> {
		// if locked price exists, return it, otherwise return latest price from oracle:
		// Example (DOT costs 40 USD):
		// oracle_price: Price = 40 * 10^18;
		// feed_price: Price = 40 * 10^18 / 10^10 = 40 * 10^8 - the price for 1 basic unit;
		match (
			Self::locked_price(currency_id).or_else(|| T::Source::get(&currency_id)),
			10_u128.checked_pow(T::TokenDecimals::get(&currency_id)),
		) {
			(Some(feed_price), Some(adjustment_multiplier)) => {
				Price::checked_from_rational(feed_price.into_inner(), adjustment_multiplier)
			}
			_ => None,
		}
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
