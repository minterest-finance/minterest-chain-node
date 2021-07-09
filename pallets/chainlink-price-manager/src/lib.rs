//! # Example Module
//!
//! A simple example of a FRAME pallet demonstrating
//! concepts, APIs and structures common to most FRAME runtimes.

#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::unused_unit)]

use frame_support::{pallet_prelude::*, transactional};
use frame_system::pallet_prelude::*;
use minterest_primitives::{CurrencyId, Price};
use pallet_chainlink_feed::{FeedOracle, RoundId};
use sp_std::vec::Vec;

mod mock;
mod tests;

pub use module::*;

type ChainlinkFeedPallet<T> = pallet_chainlink_feed::Module<T>;

#[frame_support::pallet]
pub mod module {
	use super::*;

	#[pallet::config]
	pub trait Config: frame_system::Config + pallet_chainlink_feed::Config {
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

		type ChainlinkOracle: FeedOracle<Self>;

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
			payment: pallet_chainlink_feed::BalanceOf<T>,
			timeout: T::BlockNumber,
			submission_value_bounds: (T::Value, T::Value),
			min_submissions: u32,
			decimals: u8,
			description: Vec<u8>,
			restart_delay: RoundId,
			oracles: Vec<(T::AccountId, T::AccountId)>,
			pruning_window: Option<RoundId>,
			max_debt: Option<pallet_chainlink_feed::BalanceOf<T>>,
		) -> DispatchResultWithPostInfo {
			T::UpdateOrigin::ensure_origin(origin)?;

			let adapter_origin = frame_system::RawOrigin::Signed(T::PalletAccountId::get()).into();
			<ChainlinkFeedPallet<T>>::create_feed(
				adapter_origin,
				payment,
				timeout,
				submission_value_bounds,
				min_submissions,
				decimals,
				description,
				restart_delay,
				oracles,
				pruning_window,
				max_debt,
			)
		}
	}
}
