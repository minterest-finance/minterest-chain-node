//! # MNT token Module
//!
//! TODO: Add overview

#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::unused_unit)]

use frame_support::{pallet_prelude::*, transactional};
use frame_system::pallet_prelude::*;
use minterest_primitives::{Balance, CurrencyId, CurrencyPair, Price, Rate};
pub use module::*;
use pallet_traits::{LiquidityPoolsTotalProvider, PoolsManager, PriceProvider};
use sp_runtime::{
	traits::{CheckedAdd, CheckedMul, Zero},
	FixedPointNumber,
};
use sp_std::{result, vec::Vec};

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

type Market = CurrencyPair;

#[frame_support::pallet]
pub mod module {
	use super::*;

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

		/// The basic liquidity pools.
		type LiquidityPoolsManager: PoolsManager<Self::AccountId>;

		/// Provides total functions
		type LiquidityPoolsTotalProvider: LiquidityPoolsTotalProvider;

		/// The origin which may update MNT token parameters. Root can
		/// always do this.
		type UpdateOrigin: EnsureOrigin<Self::Origin>;

		/// The price source of currencies
		type PriceSource: PriceProvider<CurrencyId>;
	}

	#[pallet::error]
	pub enum Error<T> {
		/// Try to add market that already presented in ListedMarkets
		MarketAlreadyExists,

		/// Try to ramove market that is not presented in ListedMarkets
		MarketNotExists,

		/// Pool not found.
		PoolNotFound,

		/// Arithmetic calculation overflow
		NumOverflow,

		/// Get underlying currency price is failed
		GetUnderlyingPriceFail,
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

	#[pallet::storage]
	#[pallet::getter(fn mnt_speeds)]
	type MntSpeeds<T: Config> = StorageMap<_, Twox64Concat, Market, Rate, OptionQuery>;

	/// Markets that allowed to earn MNT token
	#[pallet::storage]
	#[pallet::getter(fn mnt_markets)]
	// TODO Add MAXIMUM value for Vec<Market>
	pub(crate) type ListedMarkets<T: Config> = StorageValue<_, Vec<Market>, ValueQuery>;

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
			ensure!(
				T::LiquidityPoolsManager::pool_exists(&market.underlying_id),
				Error::<T>::PoolNotFound
			);
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
		/// Set MNT rate and recalculate MNT speed distribution for all markets
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
	/// Calculates utilities for all listed markets and total sum of them
	fn get_listed_markets_utilities() -> result::Result<(Vec<(Market, Balance)>, Balance), DispatchError> {
		// utility = total borrow * underlying price
		let markets = ListedMarkets::<T>::get();
		let mut result: Vec<(Market, Balance)> = Vec::new();
		let mut total_utility: Balance = Balance::zero();
		for market in markets.iter() {
			ensure!(
				T::LiquidityPoolsManager::pool_exists(&market.underlying_id),
				Error::<T>::PoolNotFound
			);
			let underlying_price =
				T::PriceSource::get_underlying_price(market.underlying_id).ok_or(Error::<T>::GetUnderlyingPriceFail)?;
			let total_borrow = T::LiquidityPoolsTotalProvider::get_pool_total_borrowed(market.underlying_id);

			// Should we add wrapper for such cases?
			let utility = Price::from_inner(total_borrow)
				.checked_mul(&underlying_price)
				.map(|x| x.into_inner())
				.ok_or(Error::<T>::NumOverflow)?;

			total_utility = total_utility.checked_add(utility).ok_or(Error::<T>::NumOverflow)?;

			result.push((*market, utility));
		}
		Ok((result, total_utility))
	}

	fn refresh_mnt_speeds() -> result::Result<(), DispatchError> {
		let utilities = Pallet::<T>::get_listed_markets_utilities()?;
		Ok(())
	}

	fn update_mnt_supply_index() {
		// TODO Update only if comp_speed > 0
	}
	fn update_mnt_borrow_index() {
		// TODO Update only if comp_speed > 0
	}
}
