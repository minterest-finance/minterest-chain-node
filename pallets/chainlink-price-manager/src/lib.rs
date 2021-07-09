//! # Example Module
//!
//! A simple example of a FRAME pallet demonstrating
//! concepts, APIs and structures common to most FRAME runtimes.

#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::unused_unit)]

use frame_support::{pallet_prelude::*, transactional};
use frame_system::pallet_prelude::*;
use minterest_primitives::{CurrencyId, Price};
use pallet_chainlink_feed::{FeedInterface, FeedOracle, RoundData, RoundId};
use sp_runtime::traits::{Bounded, One, Zero};
use sp_runtime::FixedU128;
use sp_std::vec::Vec;

mod mock;
mod tests;

pub use module::*;

type ChainlinkFeedPallet<T> = pallet_chainlink_feed::Pallet<T>;

#[frame_support::pallet]
pub mod module {
	use super::*;

	#[pallet::config]
	pub trait Config: frame_system::Config + pallet_chainlink_feed::Config {
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

		/// The pallet account id, keep all assets in Pools.
		type PalletAccountId: Get<Self::AccountId>;

		/// The origin which may update controller parameters. Root or
		/// Half Minterest Council can always do this.
		type UpdateOrigin: EnsureOrigin<Self::Origin>;
	}

	#[pallet::error]
	pub enum Error<T> {
		/// Some wrong behavior
		Wrong,
	}

	#[pallet::storage]
	#[pallet::getter(fn main_feed_keeper)]
	pub type MainFeedKeeper<T: Config> = StorageMap<_, Blake2_128Concat, CurrencyId, T::FeedId, OptionQuery>;

	// TODO IMPLEMENT
	// #[pallet::storage]
	// #[pallet::getter(fn reserve_feed_keeper)]
	// pub type ReserveFeedKeeper<T: Config> = StorageMap<_, Blake2_128Concat, CurrencyId, T::FeedId,
	// ValueQuery>;

	#[pallet::event]
	#[pallet::generate_deposit(fn deposit_event)]
	pub enum Event<T: Config> {}

	#[pallet::pallet]
	pub struct Pallet<T>(PhantomData<T>);

	#[pallet::hooks]
	impl<T: Config> Hooks<T::BlockNumber> for Pallet<T> {}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		///
		#[pallet::weight(10_000)]
		#[transactional]
		pub fn create_feed(
			origin: OriginFor<T>,
			currency_id: CurrencyId,
			// payment: pallet_chainlink_feed::BalanceOf<T>,
			timeout: T::BlockNumber,
			// submission_value_bounds: (T::Value, T::Value),
			min_submissions: u32,
			description: Vec<u8>,
			restart_delay: RoundId,
			oracles: Vec<(T::AccountId, T::AccountId)>,
			pruning_window: Option<RoundId>,
			max_debt: Option<pallet_chainlink_feed::BalanceOf<T>>,
		) -> DispatchResultWithPostInfo {
			T::UpdateOrigin::ensure_origin(origin)?;

			let feed_id = pallet_chainlink_feed::FeedCounter::<T>::get();
			let adapter_origin = frame_system::RawOrigin::Signed(T::PalletAccountId::get()).into();

			<ChainlinkFeedPallet<T>>::create_feed(
				adapter_origin,
				pallet_chainlink_feed::BalanceOf::<T>::zero(),
				timeout,
				(T::Value::zero(), T::Value::max_value()), // TODO think about minimal value greater than zero
				min_submissions,
				18, // 18 decimals
				description,
				restart_delay,
				oracles,
				pruning_window,
				max_debt,
			)?;

			MainFeedKeeper::<T>::insert(currency_id, feed_id);
			Ok(().into())
		}
	}
}

impl<T: Config> Pallet<T> {
	pub fn get_underlying_price(currency_id: CurrencyId) -> Option<T::Value> {
		let feed_id = MainFeedKeeper::<T>::get(currency_id);
		if feed_id == None {
			return None;
		}

		let feed_result = <ChainlinkFeedPallet<T>>::feed(feed_id.unwrap().into()).unwrap();
		let RoundData { answer, .. } = feed_result.latest_data();
		Some(answer)
	}
}
