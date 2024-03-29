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
	offchain::{SendTransactionTypes, SubmitTransaction},
	pallet_prelude::*,
};
use minterest_primitives::{currency::CurrencyType::UnderlyingAsset, currency::*, CurrencyId, OffchainErr, Price};
use pallet_chainlink_feed::{FeedInterface, FeedOracle, RoundData, RoundId};
use pallet_traits::PricesManager;
use sp_runtime::traits::{One, Zero};
use sp_std::convert::TryInto;
pub use weights::WeightInfo;

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

pub mod weights;

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

		/// Weight information for the extrinsics.
		type ChainlinkPriceManagerWeightInfo: WeightInfo;
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
	impl<T: Config> Hooks<T::BlockNumber> for Pallet<T> {
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

impl<T: Config> Pallet<T> {
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
		if !(now % bn).is_zero() {
			return Ok(());
		}

		for currency in CurrencyId::get_enabled_tokens_in_protocol(UnderlyingAsset) {
			let feed_id = Self::get_feed_id(currency).ok_or(OffchainErr::ChainlinkFeedNotExists)?;
			let new_round_id = <ChainlinkFeedPallet<T>>::feed(feed_id)
				.ok_or(OffchainErr::FailReceivingOraclePrice)?
				.latest_round()
				.checked_add(One::one())
				.ok_or(OffchainErr::NumOverflow)?;
			log::debug!("New round_id {:?} for currency {:?}", new_round_id, currency);
			let call = Call::<T>::initiate_new_round(feed_id, new_round_id);
			if SubmitTransaction::<T, Call<T>>::submit_unsigned_transaction(call.into()).is_err() {
				log::error!(
					target: "ChainlinkPriceManager offchain worker",
					"Initiate a new round is faled",
				);
			}
		}
		Self::print_prices();
		Ok(())
	}

	// TODO This is temporary function. We need tom move this function as method to
	// primitives/src/currency.rs. Also, add distingiush between chainlink provider and minterest
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
			.find(|(_, v)| v.description.into_ref().as_slice() == Self::convert_to_description(currency_id))?
			.0,
		)
	}
}

impl<T: Config> PricesManager<CurrencyId> for Pallet<T> {
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

		Some(Price::from_inner(answer.try_into().ok()?))
	}

	// TODO These function will be removed from trait
	fn lock_price(_currency_id: CurrencyId) {
		unimplemented!()
	}
	fn unlock_price(_currency_id: CurrencyId) {
		unimplemented!()
	}
}
