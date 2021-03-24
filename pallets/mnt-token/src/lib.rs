//! # MNT token Module
//!
//! Provides functionality for minting MNT tokens.

#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::unused_unit)]

use frame_support::{pallet_prelude::*, transactional};
use frame_system::pallet_prelude::*;
use minterest_primitives::{Balance, CurrencyId, Price, Rate};
pub use module::*;
use pallet_traits::{LiquidityPoolsManager, PriceProvider};
use sp_runtime::{
	traits::{CheckedDiv, CheckedMul, Zero},
	DispatchResult, FixedPointNumber,
};
use sp_std::{result, vec::Vec};

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

#[frame_support::pallet]
pub mod module {
	use super::*;

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

		/// Provides Liquidity Pool functionality
		type LiquidityPoolsManager: LiquidityPoolsManager;

		/// The origin which may update MNT token parameters. Root can
		/// always do this.
		type UpdateOrigin: EnsureOrigin<Self::Origin>;

		/// The price source of currencies
		type PriceSource: PriceProvider<CurrencyId>;

		/// Enabled underlying asset IDs.
		type EnabledUnderlyingAssetId: Get<Vec<CurrencyId>>;
	}

	#[pallet::error]
	pub enum Error<T> {
		/// Trying to enable already enabled minting for pool
		MntMintingAlreadyEnabled,

		/// Trying to disable MNT minting that wasn't enabled
		MntMintingNotEnabled,

		/// Arithmetic calculation overflow
		NumOverflow,

		/// Get underlying currency price is failed
		GetUnderlyingPriceFail,

		/// The currency is not enabled in protocol.
		NotValidUnderlyingAssetId,
	}

	#[pallet::event]
	#[pallet::generate_deposit(fn deposit_event)]
	pub enum Event<T: Config> {
		/// Change rate event (old_rate, new_rate)
		NewMntRate(Rate, Rate),

		/// MNT minting enabled for pool
		MntMintingEnabled(CurrencyId),

		/// MNT minting disabled for pool
		MntMintingDisabled(CurrencyId),
	}

	/// MNT minting rate per block
	#[pallet::storage]
	#[pallet::getter(fn mnt_rate)]
	type MntRate<T: Config> = StorageValue<_, Rate, ValueQuery>;

	/// Mnt minting speed for each pool
	#[pallet::storage]
	#[pallet::getter(fn mnt_speeds)]
	pub(crate) type MntSpeeds<T: Config> = StorageMap<_, Twox64Concat, CurrencyId, Rate, OptionQuery>;

	#[pallet::genesis_config]
	pub struct GenesisConfig<T: Config> {
		pub mnt_rate: Rate,
		pub minted_pools: Vec<CurrencyId>,
		pub _marker: PhantomData<T>,
	}

	#[cfg(feature = "std")]
	impl<T: Config> Default for GenesisConfig<T> {
		fn default() -> Self {
			GenesisConfig {
				mnt_rate: Rate::zero(),
				minted_pools: vec![],
				_marker: PhantomData,
			}
		}
	}

	#[pallet::genesis_build]
	impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
		fn build(&self) {
			MntRate::<T>::put(&self.mnt_rate);
			for currency_id in &self.minted_pools {
				MntSpeeds::<T>::insert(currency_id, Rate::zero());
			}
			if !self.minted_pools.is_empty() {
				Pallet::<T>::refresh_mnt_speeds().expect("Calculate MntSpeeds is failed");
			}
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
		/// Enable MNT minting for pool and recalculate MntSpeeds
		pub fn enable_mnt_minting(origin: OriginFor<T>, currency_id: CurrencyId) -> DispatchResultWithPostInfo {
			T::UpdateOrigin::ensure_origin(origin)?;
			ensure!(
				T::EnabledUnderlyingAssetId::get()
					.into_iter()
					.any(|asset_id| asset_id == currency_id),
				Error::<T>::NotValidUnderlyingAssetId
			);
			ensure!(
				!MntSpeeds::<T>::contains_key(currency_id),
				Error::<T>::MntMintingAlreadyEnabled
			);
			MntSpeeds::<T>::insert(currency_id, Rate::zero());
			Self::refresh_mnt_speeds()?;
			Self::deposit_event(Event::MntMintingEnabled(currency_id));
			Ok(().into())
		}

		#[pallet::weight(10_000)]
		#[transactional]
		/// Disable MNT minting for pool and recalculate MntSpeeds
		pub fn disable_mnt_minting(origin: OriginFor<T>, currency_id: CurrencyId) -> DispatchResultWithPostInfo {
			T::UpdateOrigin::ensure_origin(origin)?;
			ensure!(
				T::EnabledUnderlyingAssetId::get()
					.into_iter()
					.any(|asset_id| asset_id == currency_id),
				Error::<T>::NotValidUnderlyingAssetId
			);
			ensure!(
				MntSpeeds::<T>::contains_key(currency_id),
				Error::<T>::MntMintingNotEnabled
			);
			MntSpeeds::<T>::remove(currency_id);
			Self::refresh_mnt_speeds()?;
			Self::deposit_event(Event::MntMintingDisabled(currency_id));
			Ok(().into())
		}

		#[pallet::weight(10_000)]
		#[transactional]
		/// Set MNT rate and recalculate MntSpeeds distribution
		pub fn set_mnt_rate(origin: OriginFor<T>, new_rate: Rate) -> DispatchResultWithPostInfo {
			T::UpdateOrigin::ensure_origin(origin)?;
			let old_rate = MntRate::<T>::get();
			MntRate::<T>::put(new_rate);
			Self::refresh_mnt_speeds()?;
			Self::deposit_event(Event::NewMntRate(old_rate, new_rate));
			Ok(().into())
		}
	}
}

impl<T: Config> Pallet<T> {
	/// Calculate utilities for enabled pools and sum of all pools utilities
	///
	/// returns (Vector<CurrencyId, pool_utility>, sum_of_all_pools_utilities)
	fn calculate_enabled_pools_utilities() -> result::Result<(Vec<(CurrencyId, Balance)>, Balance), DispatchError> {
		let minted_pools = MntSpeeds::<T>::iter();
		let mut result: Vec<(CurrencyId, Balance)> = Vec::new();
		let mut total_utility: Balance = Balance::zero();
		for (currency_id, _) in minted_pools {
			let underlying_price =
				T::PriceSource::get_underlying_price(currency_id).ok_or(Error::<T>::GetUnderlyingPriceFail)?;
			let total_borrow = T::LiquidityPoolsManager::get_pool_total_borrowed(currency_id);

			// utility = m_tokens_total_borrows * asset_price
			let utility = Price::from_inner(total_borrow)
				.checked_mul(&underlying_price)
				.map(|x| x.into_inner())
				.ok_or(Error::<T>::NumOverflow)?;

			total_utility = total_utility.checked_add(utility).ok_or(Error::<T>::NumOverflow)?;

			result.push((currency_id, utility));
		}
		Ok((result, total_utility))
	}

	/// Recalculate MNT speeds
	fn refresh_mnt_speeds() -> DispatchResult {
		// TODO Add update indexes here when it will be implemented
		let (pool_utilities, sum_of_all_utilities) = Self::calculate_enabled_pools_utilities()?;
		let sum_of_all_utilities = Rate::from_inner(sum_of_all_utilities);
		if sum_of_all_utilities == Rate::zero() {
			// There is nothing to calculate.
			return Ok(());
		}
		let mnt_rate = Self::mnt_rate();
		for (currency_id, utility) in pool_utilities {
			let utility = Rate::from_inner(utility);
			let utility_fraction = utility
				.checked_div(&sum_of_all_utilities)
				.ok_or(Error::<T>::NumOverflow)?;
			let pool_mnt_speed = mnt_rate.checked_mul(&utility_fraction).ok_or(Error::<T>::NumOverflow)?;
			MntSpeeds::<T>::insert(currency_id, pool_mnt_speed);
		}
		Ok(())
	}
}
