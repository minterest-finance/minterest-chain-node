//! # MNT token Module
//!
//! TODO: Add overview

#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::unused_unit)]

use frame_support::{pallet_prelude::*, transactional};
use frame_system::pallet_prelude::*;
use minterest_primitives::{CurrencyId, CurrencyPair, Rate};
use pallet_traits::PriceProvider;
use sp_runtime::FixedPointNumber;
use sp_std::prelude::Vec;

mod mock;
mod tests;

pub use module::*;

type Market = CurrencyPair;

#[frame_support::pallet]
pub mod module {
	use super::*;

	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// The price source of currencies
		type PriceSource: PriceProvider<CurrencyId>;

		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

		/// The origin which may update MNT token parameters. Root can
		/// always do this.
		type UpdateOrigin: EnsureOrigin<Self::Origin>;
	}

	#[pallet::error]
	pub enum Error<T> {
		/// Try to add market that already presented in ListedMarkets
		MarketAlreadyExists,

		/// Try to ramove market that is not presented in ListedMarkets
		MarketNotExists,
	}

	#[pallet::event]
	#[pallet::generate_deposit(fn deposit_event)]
	pub enum Event<T: Config> {
		/// Change rate event (old_rate, new_rate)
		NewMntRate(Rate, Rate),

		/// New market was added to listing
		NewMarketListed(Market),

		/// Market was removed from listring
		MarketRemoved(Market),
	}

	#[pallet::storage]
	#[pallet::getter(fn mnt_rate)]
	type MntRate<T: Config> = StorageValue<_, Rate, ValueQuery>;

	/// Markets that allowed to earn MNT token
	#[pallet::storage]
	#[pallet::getter(fn mnt_markets)]
	// TODO Add MAXIMUM value for Vec<Market>
	type ListedMarkets<T: Config> = StorageValue<_, Vec<Market>, ValueQuery>;

	#[pallet::genesis_config]
	pub struct GenesisConfig<T: Config> {
		pub mnt_rate: Rate,
		pub marker: PhantomData<T>,
	}

	#[cfg(feature = "std")]
	impl<T: Config> Default for GenesisConfig<T> {
		fn default() -> Self {
			GenesisConfig {
				mnt_rate: Rate::zero(),
				marker: PhantomData,
			}
		}
	}

	#[pallet::genesis_build]
	impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
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
		#[transactional]
		/// Add market to MNT markets list to allow earn MNT tokens
		pub fn add_market(origin: OriginFor<T>, market: Market) -> DispatchResultWithPostInfo {
			T::UpdateOrigin::ensure_origin(origin)?;
			let mut markets = ListedMarkets::<T>::get();
			ensure!(!markets.contains(&market), Error::<T>::MarketAlreadyExists);
			markets.push(market);
			ListedMarkets::<T>::put(markets);
			Self::deposit_event(Event::NewMarketListed(market));
			Ok(().into())
		}

		/// Stop earning MNT tokens for market
		#[pallet::weight(10_000)]
		#[transactional]
		pub fn remove_market(origin: OriginFor<T>, market: Market) -> DispatchResultWithPostInfo {
			T::UpdateOrigin::ensure_origin(origin)?;
			let mut markets = ListedMarkets::<T>::get();
			ensure!(markets.contains(&market), Error::<T>::MarketNotExists);
			markets.remove(
				markets
					.iter()
					.position(|x| *x == market)
					.expect("Market not found" /* should never be here */),
			);
			ListedMarkets::<T>::put(markets);
			Self::deposit_event(Event::MarketRemoved(market));
			Ok(().into())
		}

		#[pallet::weight(10_000)]
		#[transactional]
		pub fn set_mnt_rate(origin: OriginFor<T>, new_rate: Rate) -> DispatchResultWithPostInfo {
			T::UpdateOrigin::ensure_origin(origin)?;
			let old_rate = MntRate::<T>::get();
			MntRate::<T>::put(new_rate);
			Pallet::<T>::refresh_mnt_speeds();
			Self::deposit_event(Event::NewMntRate(old_rate, new_rate));
			Ok(().into())
		}
	}
}

impl<T: Config> Pallet<T> {
	fn refresh_mnt_speeds() {}
	fn update_mnt_supply_index() {
		// TODO Update only if comp_speed > 0
	}
	fn update_mnt_borrow_index() {
		// TODO Update only if comp_speed > 0
	}
}
