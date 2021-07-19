//! # Example Module
//!
//! A simple example of a FRAME pallet demonstrating
//! concepts, APIs and structures common to most FRAME runtimes.

#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::unused_unit)]

use frame_support::{log, pallet_prelude::*, transactional};
use frame_system::offchain::{SendTransactionTypes, SubmitTransaction};
use frame_system::pallet_prelude::*;
use minterest_primitives::{currency::*, BlockNumber, CurrencyId, Price};
use pallet_chainlink_feed::{FeedInterface, FeedOracle, RoundData, RoundId};
use sp_runtime::traits::{Bounded, One, Zero};
use sp_runtime::FixedU128;
use sp_std::vec::Vec;
use sp_std::{convert::TryInto, result};

mod mock;
mod tests;

pub use module::*;

type ChainlinkFeedPallet<T> = pallet_chainlink_feed::Pallet<T>;

#[frame_support::pallet]
pub mod module {
	use super::*;

	#[pallet::config]
	pub trait Config: frame_system::Config + pallet_chainlink_feed::Config + SendTransactionTypes<Call<Self>> {
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

		/// The pallet account id, keep all assets in Pools.
		type PalletAccountId: Get<Self::AccountId>;

		/// Root or half Minterest Council can always do this.
		type UpdateOrigin: EnsureOrigin<Self::Origin>;
		/// A configuration for base priority of unsigned transactions.
		///
		/// This is exposed so that it can be tuned for particular runtime, when
		/// multiple pallets send unsigned transactions.
		type UnsignedPriority: Get<TransactionPriority>;
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
	pub enum Event<T: Config> {
		// feed_id, new_round
		InitiateNewRound(T::FeedId, RoundId),
		DummyEvent(u8),
	}

	#[pallet::pallet]
	pub struct Pallet<T>(PhantomData<T>);

	#[pallet::validate_unsigned]
	impl<T: Config> ValidateUnsigned for Pallet<T> {
		type Call = Call<T>;

		fn validate_unsigned(_source: TransactionSource, call: &Self::Call) -> TransactionValidity {
			match call {
				Call::initiate_new_round(feed_id, round_id) => {
					ValidTransaction::with_tag_prefix("ChainlinkPriceManagerWorker")
						.priority(T::UnsignedPriority::get())
						.and_provides((<frame_system::Pallet<T>>::block_number(), feed_id, round_id))
						.longevity(64_u64)
						.propagate(true)
						.build()
				}
				_ => InvalidTransaction::Call.into(),
			}
		}
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<T::BlockNumber> for Pallet<T> {
		fn offchain_worker(now: T::BlockNumber) {
			let bn: T::BlockNumber = (3_u32).into();
			if (now % bn).is_zero() {
				let feed_id = MainFeedKeeper::<T>::get(ETH);
				// TODO produce Event if get_underlying_price isn't possible
				if feed_id == None {
					return;
				}

				let feed_result = <ChainlinkFeedPallet<T>>::feed(feed_id.unwrap()).unwrap();
				log::info!("This mambo number {:?}", now);
				log::info!("Last round_id is: {:?}", feed_result.latest_round());
				let call =
					Call::<T>::initiate_new_round(feed_id.unwrap(), feed_result.latest_round().saturating_add(1));
				SubmitTransaction::<T, Call<T>>::submit_unsigned_transaction(call.into()).unwrap();
			}
			log::info!("ETH price is: {:?}", Self::get_underlying_price(ETH));
			log::info!("DOT price is: {:?}", Self::get_underlying_price(DOT));
			log::info!("KSM price is: {:?}", Self::get_underlying_price(KSM));
			log::info!("BTC price is: {:?}", Self::get_underlying_price(BTC));
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::weight(10_000)]
		#[transactional]
		pub fn initiate_new_round(
			origin: OriginFor<T>,
			feed_id: T::FeedId,
			new_round: RoundId,
		) -> DispatchResultWithPostInfo {
			Self::deposit_event(Event::InitiateNewRound(feed_id, new_round));
			Ok(().into())
		}

		#[pallet::weight(10_000)]
		#[transactional]
		pub fn submit(
			origin: OriginFor<T>,
			currency_id: CurrencyId,
			#[pallet::compact] round_id: RoundId,
			#[pallet::compact] submission: T::Value,
		) -> DispatchResultWithPostInfo {
			let feed_id = MainFeedKeeper::<T>::get(currency_id);
			if feed_id == None {
				return Ok(().into());
			}

			<ChainlinkFeedPallet<T>>::submit(origin, feed_id.unwrap(), round_id, submission)
		}

		#[pallet::weight(10_000)]
		#[transactional]
		pub fn create_minterest_feed(
			origin: OriginFor<T>,
			currency_id: CurrencyId,
			min_submissions: u32,
			oracles: Vec<(T::AccountId, T::AccountId)>,
		) -> DispatchResultWithPostInfo {
			T::UpdateOrigin::ensure_origin(origin)?;

			let feed_id = pallet_chainlink_feed::FeedCounter::<T>::get();
			let adapter_origin = frame_system::RawOrigin::Signed(T::PalletAccountId::get()).into();
			let bn: T::BlockNumber = (10_u32).into();

			<ChainlinkFeedPallet<T>>::create_feed(
				adapter_origin,
				pallet_chainlink_feed::BalanceOf::<T>::zero(),
				bn,
				(T::Value::zero(), T::Value::max_value()),
				min_submissions,
				18, // 18 decimals
				b"".to_vec(),
				0,
				oracles,
				None,
				None,
			)?;

			// Todo REPLACE
			MainFeedKeeper::<T>::insert(currency_id, feed_id);
			Ok(().into())
		}

		#[pallet::weight(10_000)]
		#[transactional]
		pub fn create_chainlink_feed(
			origin: OriginFor<T>,
			currency_id: CurrencyId,
			payment: pallet_chainlink_feed::BalanceOf<T>,
			timeout: T::BlockNumber,
			submission_value_bounds: (T::Value, T::Value),
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
				payment,
				timeout,
				submission_value_bounds,
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
		// TODO produce Event if get_underlying_price isn't possible
		if feed_id == None {
			return None;
		}

		// TODO handle if price is 0

		let feed_result = <ChainlinkFeedPallet<T>>::feed(feed_id.unwrap().into()).unwrap();
		let RoundData { answer, .. } = feed_result.latest_data();
		Some(answer)
	}
}
