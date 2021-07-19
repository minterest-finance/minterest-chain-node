//! # Prices Module
//!
//! ## Overview
//!
//! The data from Oracle cannot be used in business, prices module will do some
//! process and feed prices for Minterest. Process include:
//!   - specify a fixed price for stable currency;
//!   - feed price in USD;
//!   - lock/unlock the price data get from oracle.

#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::unused_unit)]
#![allow(clippy::upper_case_acronyms)]
use frame_support::{pallet_prelude::*, transactional};
use minterest_primitives::{currency::CurrencyType::UnderlyingAsset, CurrencyId, Price};
use orml_traits::{DataFeeder, DataProvider};
use pallet_traits::PricesManager;
use sp_std::vec::Vec;

pub use module::*;

#[cfg(test)]
mod tests;

pub mod weights;
pub use weights::WeightInfo;

#[frame_support::pallet]
pub mod module {
	use super::*;
	use frame_system::pallet_prelude::OriginFor;

	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// The overarching event type.
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

		/// The data source, such as Oracle.
		type Source: DataProvider<CurrencyId, Price> + DataFeeder<CurrencyId, Price, Self::AccountId>;

		/// The origin which may lock and unlock prices feed to system.
		type LockOrigin: EnsureOrigin<Self::Origin>;

		/// Weight information for the extrinsics.
		type WeightInfo: WeightInfo;
	}

	#[pallet::error]
	pub enum Error<T> {
		/// The currency is not enabled in protocol.
		NotValidUnderlyingAssetId,
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
	#[pallet::getter(fn locked_price_storage)]
	pub type LockedPriceStorage<T: Config> = StorageMap<_, Twox64Concat, CurrencyId, Price, OptionQuery>;

	#[pallet::genesis_config]
	pub struct GenesisConfig<T: Config> {
		#[allow(clippy::type_complexity)]
		pub locked_price: Vec<(CurrencyId, Price)>,
		pub _phantom: PhantomData<T>,
	}

	#[cfg(feature = "std")]
	impl<T: Config> Default for GenesisConfig<T> {
		fn default() -> Self {
			GenesisConfig {
				locked_price: vec![],
				_phantom: PhantomData,
			}
		}
	}

	#[pallet::genesis_build]
	impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
		fn build(&self) {
			self.locked_price
				.iter()
				.for_each(|(currency_id, price)| LockedPriceStorage::<T>::insert(currency_id, price));
		}
	}

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
		#[pallet::weight((T::WeightInfo::lock_price(), DispatchClass::Operational))]
		#[transactional]
		pub fn lock_price(origin: OriginFor<T>, currency_id: CurrencyId) -> DispatchResultWithPostInfo {
			T::LockOrigin::ensure_origin(origin)?;

			ensure!(
				currency_id.is_supported_underlying_asset(),
				Error::<T>::NotValidUnderlyingAssetId
			);

			<Pallet<T> as PricesManager<CurrencyId>>::lock_price(currency_id);
			Ok(().into())
		}

		/// Unlock the price and get the price from `PriceProvider` again
		///
		/// The dispatch origin of this call must be `LockOrigin`.
		///
		/// - `currency_id`: currency type.
		#[pallet::weight((T::WeightInfo::unlock_price(), DispatchClass::Operational))]
		#[transactional]
		pub fn unlock_price(origin: OriginFor<T>, currency_id: CurrencyId) -> DispatchResultWithPostInfo {
			T::LockOrigin::ensure_origin(origin)?;

			ensure!(
				currency_id.is_supported_underlying_asset(),
				Error::<T>::NotValidUnderlyingAssetId
			);

			<Pallet<T> as PricesManager<CurrencyId>>::unlock_price(currency_id);
			Ok(().into())
		}
	}
}

impl<T: Config> PricesManager<CurrencyId> for Pallet<T> {
	/// Get price underlying token in USD.
	fn get_underlying_price(currency_id: CurrencyId) -> Option<Price> {
		// if locked price exists, return it, otherwise return latest price from oracle:
		Self::locked_price_storage(currency_id).or_else(|| T::Source::get(&currency_id))
	}

	/// Locks price when get valid price from source.
	fn lock_price(currency_id: CurrencyId) {
		// lock price when get valid price from source
		if let Some(val) = T::Source::get(&currency_id) {
			LockedPriceStorage::<T>::insert(currency_id, val);
			<Pallet<T>>::deposit_event(Event::LockPrice(currency_id, val));
		}
	}

	/// Unlocks price when get valid price from source.
	fn unlock_price(currency_id: CurrencyId) {
		LockedPriceStorage::<T>::remove(currency_id);
		<Pallet<T>>::deposit_event(Event::UnlockPrice(currency_id));
	}
}

/// RPC calls
impl<T: Config> Pallet<T> {
	pub fn get_all_freshest_prices() -> Vec<(CurrencyId, Option<Price>)> {
		CurrencyId::get_enabled_tokens_in_protocol(UnderlyingAsset)
			.into_iter()
			.map(|currency_id| (currency_id, T::Source::get(&currency_id)))
			.collect()
	}
}
