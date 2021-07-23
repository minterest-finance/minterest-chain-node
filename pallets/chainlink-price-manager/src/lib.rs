//! # Chainlink Price Manager
//!
//! Main oracle price manager that provide updated and reliable oracle prices.
//!
//! ## Interface
//!
//! -`PricesManager`: provides get_underlying_price interface.
//!
//! ### Dispatchable Functions (extrinsics)
//!
//! - `enable_feeding` - Enable providing oracle prices.
//!
//! - `disable_feeding` - Disable providing oracle prices.
//! The get_underlying_price will return None.
//!
//!  TODO Pallet in development.
//!  Implement provider types Chainlink and Minterest

#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::unused_unit)]

use frame_support::{log, pallet_prelude::*, transactional, IterableStorageMap};
use frame_system::{
	pallet_prelude::*,
	offchain::{SendTransactionTypes, SubmitTransaction},
};
use minterest_primitives::{currency::CurrencyType::UnderlyingAsset, currency::*, CurrencyId, OffchainErr, Price};
use pallet_chainlink_feed::{FeedInterface, FeedOracle, RoundData, RoundId};
use pallet_traits::PricesManager;
use sp_runtime::traits::Zero;

#[cfg(test)]
mod mock;
#[cfg(test)]
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

		/// A configuration for base priority of unsigned transactions.
		///
		/// This is exposed so that it can be tuned for particular runtime, when
		/// multiple pallets send unsigned transactions.
		type UnsignedPriority: Get<TransactionPriority>;
	}

	#[pallet::error]
	pub enum Error<T> {}

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
		/// Produces events to initiate a new round for oracles.
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

		/// Enables feeding. Start providing prices
		#[pallet::weight(10_000)]
		#[transactional]
		pub fn enable_feeding(_origin: OriginFor<T>) -> DispatchResult {
			// TODO make additional checks
			// Check is all feed description are unique
			// Check is all enabled currencies has feed
			Ok(())
		}

		// TODO. Then get_underlying_price should always return None
		#[pallet::weight(10_000)]
		#[transactional]
		pub fn disable_feeding(_origin: OriginFor<T>) -> DispatchResult {
			Ok(())
		}
	}
}

impl<T: Config> Pallet<T>
where
	u128: From<<T as pallet_chainlink_feed::Config>::Value>,
{
	// TODO rework it.
	// This is temporary function. Shouldn't use in production but helpful in test project stage.
	// If one of pool_id has lower RoundId that others we should syncronize it with others.
	// This can happen if oracle hasn't enough time to submit all prices.
	fn get_min_round_id() -> Result<RoundId, OffchainErr> {
		let mut min_round_id: RoundId = RoundId::MAX;
		for currency in CurrencyId::get_enabled_tokens_in_protocol(UnderlyingAsset) {
			let feed_id = Self::get_feed_id(currency).ok_or(OffchainErr::ChainlinkFeedNotExists)?;
			let feed_result = <ChainlinkFeedPallet<T>>::feed(feed_id)
				.ok_or(OffchainErr::FailReceivingOraclePrice)?
				.latest_round();
			min_round_id = min_round_id.min(feed_result);
		}
		Ok(min_round_id)
	}

	// Print prices to node debug log
	fn print_prices() {
		for currency in CurrencyId::get_enabled_tokens_in_protocol(UnderlyingAsset) {
			if let Some(price) = Self::get_underlying_price(currency) {
				log::debug!("{:?} price is {:?}", currency, price);
			} else {
				log::warn!("Can't receive price for {:?}", currency);
			}
		}
	}

	fn _offchain_worker(now: T::BlockNumber) -> Result<(), OffchainErr> {
		// TODO implement extrinsic to set initiate round period instead of hardcoded 3
		let bn: T::BlockNumber = (3_u32).into();
		if (now % bn).is_zero() {
			// TODO should we get latest_round for each pool and produce event pair, or enough take
			// only min round id?
			let new_round_id = Self::get_min_round_id()?.saturating_add(1);
			// Currently feed id is not play any role. See TODO above
			let feed_id = Self::get_feed_id(ETH).ok_or(OffchainErr::ChainlinkFeedNotExists)?;
			let call = Call::<T>::initiate_new_round(feed_id, new_round_id);
			log::debug!("New round_id: {:?}", new_round_id);

			if SubmitTransaction::<T, Call<T>>::submit_unsigned_transaction(call.into()).is_err() {
				log::info!(
					target: "ChainlinkPriceManager offchain worker",
					"Initiate a new round is faled",
				);
			}
		}
		Self::print_prices();
		Ok(())
	}

	// TODO This is temporary function. We need tom move this function as method to
	// privitives/src/currency.rs. Also, add distingiush between chainlink provider and minterest
	fn convert_to_description(currency_id: CurrencyId) -> &'static [u8] {
		match currency_id {
			ETH => b"MIN-ETH",
			DOT => b"MIN-DOT",
			KSM => b"MIN-KSM",
			BTC => b"MIN-BTC",
			// This must be gone after implementing strict CurrencyId types
			_ => b"Non-existent-feed",
		}
	}

	/// Looks for appropriate feed config with description and returns FeedId
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
		// TODO check is feeding enabled
		let feed_id = Self::get_feed_id(currency_id)?;
		let feed_result = <ChainlinkFeedPallet<T>>::feed(feed_id)?;
		// TODO handle updated_at
		let RoundData { answer, .. } = feed_result.latest_data();

		// There is an issue that pallet-chainlink-feed can return Some(0)
		// if feed was created but submit() extrinsic wasn't called
		if answer.is_zero() {
			return None;
		}

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
