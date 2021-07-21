//! # Example Module
//!
//! A simple example of a FRAME pallet demonstrating
//! concepts, APIs and structures common to most FRAME runtimes.

#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::unused_unit)]

use frame_support::{log, pallet_prelude::*, transactional, IterableStorageMap};
use frame_system::offchain::{SendTransactionTypes, SubmitTransaction};
use frame_system::pallet_prelude::*;
use minterest_primitives::{currency::*, CurrencyId, OffchainErr, Price};
use pallet_chainlink_feed::{FeedInterface, FeedOracle, RoundData, RoundId};
use pallet_traits::PricesManager;
use sp_runtime::traits::Zero;

mod mock;
mod tests;

pub use module::*;

type ChainlinkFeedPallet<T> = pallet_chainlink_feed::Pallet<T>;

// TODO should be implemented
// enum ProviderType {
// 	Chainlink,
// 	Minterest,
// }

#[frame_support::pallet]
pub mod module {
	use super::*;

	#[pallet::config]
	pub trait Config: frame_system::Config + pallet_chainlink_feed::Config + SendTransactionTypes<Call<Self>> {
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

		/// The pallet account id, keep all assets in Pools.
		type PalletAccountId: Get<Self::AccountId>;

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

	#[pallet::event]
	#[pallet::generate_deposit(fn deposit_event)]
	pub enum Event<T: Config> {
		InitiateNewRound(T::FeedId, RoundId),
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
	impl<T: Config> Hooks<T::BlockNumber> for Pallet<T>
	where
		u128: From<<T as pallet_chainlink_feed::Config>::Value>,
	{
		fn offchain_worker(now: T::BlockNumber) {
			if let Err(error) = Self::_offchain_worker(now) {
				log::info!(
					target: "ChainlinkPriceManager offchain worker",
					"cannot run offchain worker at {:?}: {:?}",
					now,
					error,
				);
			} else {
				log::debug!(
					target: "ChainlinkPriceManager offchain worker",
					" Chainlink offchain worker start at block: {:?} already done!",
					now,
				);
			}
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
			ensure_none(origin)?;
			Self::deposit_event(Event::InitiateNewRound(feed_id, new_round));
			Ok(().into())
		}
	}
}

impl<T: Config> Pallet<T>
where
	u128: From<<T as pallet_chainlink_feed::Config>::Value>,
{
	fn _offchain_worker(now: T::BlockNumber) -> Result<(), OffchainErr> {
		// TODO implement extrinsic to set initiate round period instead hardcoded 3
		let bn: T::BlockNumber = (3_u32).into();
		if (now % bn).is_zero() {
			let feed_id = Self::get_feed_id(ETH).ok_or(OffchainErr::CheckFail)?;

			let feed_result = <ChainlinkFeedPallet<T>>::feed(feed_id).ok_or(OffchainErr::CheckFail)?;
			log::info!("Last feed round_id: {:?}", feed_result.latest_round());

			// TODO should we get latest_round for each pool and produce event pair?
			let call = Call::<T>::initiate_new_round(feed_id, feed_result.latest_round().saturating_add(1));
			SubmitTransaction::<T, Call<T>>::submit_unsigned_transaction(call.into()).unwrap();
		}
		log::info!("ETH price is: {:?}", Self::get_underlying_price(ETH));
		log::info!("DOT price is: {:?}", Self::get_underlying_price(DOT));
		log::info!("KSM price is: {:?}", Self::get_underlying_price(KSM));
		log::info!("BTC price is: {:?}", Self::get_underlying_price(BTC));
		Ok(())
	}

	// TODO This is temporary function. We need move this function as method to
	// privitives/src/currency.rs. Also, add distingiush between chainlink provider and minterest
	fn convert_to_description(currency_id: CurrencyId) -> &'static [u8] {
		match currency_id {
			ETH => b"MIN-ETH",
			DOT => b"MIN-DOT",
			KSM => b"MIN-KSM",
			BTC => b"MIN-BTC",
			_ => b"We should never be here",
		}
	}

	pub fn get_feed_id(currency_id: CurrencyId) -> Option<T::FeedId> {
		Some(
			<pallet_chainlink_feed::Feeds<T> as IterableStorageMap<
				T::FeedId,
				pallet_chainlink_feed::FeedConfigOf<T>,
			>>::iter()
			.find(|(_, v)| v.description == Self::convert_to_description(currency_id))?
			.0,
		)
	}
}

impl<T: Config> PricesManager<CurrencyId> for Pallet<T>
where
	u128: From<<T as pallet_chainlink_feed::Config>::Value>,
{
	fn get_underlying_price(currency_id: CurrencyId) -> Option<Price> {
		let feed_id = Self::get_feed_id(currency_id)?;
		let feed_result = <ChainlinkFeedPallet<T>>::feed(feed_id)?;
		let RoundData { answer, .. } = feed_result.latest_data();
		Some(Price::from_inner(answer.into()))
	}

	// TODO These function will be removed from trait
	fn lock_price(_currency_id: CurrencyId) {
		unimplemented!()
	}
	fn unlock_price(_currency_id: CurrencyId) {
		unimplemented!()
	}
}
