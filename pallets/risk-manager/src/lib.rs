//! # Risk Manager Pallet
//!
//! ## Overview
//!
//! TODO

#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::unused_unit)]
#![allow(clippy::upper_case_acronyms)]

use frame_support::{pallet_prelude::*, transactional};
use minterest_primitives::{CurrencyId, Operation, Rate};
pub use module::*;
use pallet_traits::{RiskManagerStorageProvider, UserCollateral, UserLiquidationAttemptsManager};
use sp_runtime::{
	traits::{One, Zero},
	FixedPointNumber,
};
#[cfg(feature = "std")]
use sp_std::str;

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

#[frame_support::pallet]
pub mod module {
	use super::*;
	use frame_system::pallet_prelude::OriginFor;
	use minterest_primitives::{Balance, Rate};

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

		/// Provides functionality for working with a user's collateral pools.
		type UserCollateral: UserCollateral<Self::AccountId>;

		#[pallet::constant]
		/// Minimal sum for partial liquidation.
		/// Loans with amount below this parameter will be liquidate in full.
		type PartialLiquidationMinSum: Get<Balance>;

		#[pallet::constant]
		/// The maximum number of partial liquidations a user has. After reaching this parameter,
		/// a complete liquidation occurs.
		type PartialLiquidationMaxAttempts: Get<u8>;

		/// The origin which may update risk manager parameters. Root or
		/// Half Minterest Council can always do this.
		type RiskManagerUpdateOrigin: EnsureOrigin<Self::Origin>;
	}

	#[pallet::error]
	pub enum Error<T> {
		/// The currency is not enabled in protocol.
		NotValidUnderlyingAssetId,
		/// Liquidation fee can't be greater than 0.5.
		InvalidLiquidationFeeValue,
		/// Risk manager storage (liquidation_fee, liquidation_threshold) is already created.
		RiskManagerParamsAlreadyCreated,
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(crate) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Liquidation fee has been successfully changed: \[liquidation_fee\]
		LiquidationFeeUpdated(Rate),
		/// Liquidation threshold has been successfully changed: \[threshold\]
		LiquidationThresholdUpdated(Rate),
	}

	/// The additional collateral which is taken from borrowers as a penalty for being liquidated.
	#[pallet::storage]
	#[pallet::getter(fn liquidation_fee)]
	pub(crate) type LiquidationFee<T: Config> = StorageMap<_, Twox64Concat, CurrencyId, Rate, ValueQuery>;

	/// Step used in liquidation to protect the user from micro liquidations.
	#[pallet::storage]
	#[pallet::getter(fn liquidation_threshold)]
	pub(crate) type LiquidationThreshold<T: Config> = StorageValue<_, Rate, ValueQuery>;

	/// Counter of the number of partial liquidations at the user.
	#[pallet::storage]
	#[pallet::getter(fn user_liquidation_attempts)]
	pub(crate) type UserLiquidationAttempts<T: Config> = StorageMap<_, Twox64Concat, T::AccountId, u8, ValueQuery>;

	#[pallet::genesis_config]
	pub struct GenesisConfig<T: Config> {
		pub liquidation_fee: Vec<(CurrencyId, Rate)>,
		pub liquidation_threshold: Rate,
		pub _phantom: sp_std::marker::PhantomData<T>,
	}

	#[cfg(feature = "std")]
	impl<T: Config> Default for GenesisConfig<T> {
		fn default() -> Self {
			GenesisConfig {
				liquidation_fee: vec![],
				liquidation_threshold: Rate::default(),
				_phantom: PhantomData,
			}
		}
	}

	#[pallet::genesis_build]
	impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
		fn build(&self) {
			self.liquidation_fee.iter().for_each(|(pool_id, liquidation_fee)| {
				Pallet::<T>::is_valid_liquidation_fee(*liquidation_fee);
				LiquidationFee::<T>::insert(pool_id, liquidation_fee)
			});
			LiquidationThreshold::<T>::put(self.liquidation_threshold);
		}
	}

	#[pallet::pallet]
	pub struct Pallet<T>(PhantomData<T>);

	#[pallet::hooks]
	impl<T: Config> Hooks<T::BlockNumber> for Pallet<T> {}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Set Liquidation fee that covers liquidation costs.
		///
		/// - `pool_id`: PoolID for which the parameter value is being set.
		/// - `liquidation_fee`: new liquidation fee value.
		///
		/// The dispatch origin of this call must be 'RiskManagerUpdateOrigin'.
		#[pallet::weight(0)]
		#[transactional]
		pub fn set_liquidation_fee(
			origin: OriginFor<T>,
			pool_id: CurrencyId,
			liquidation_fee: Rate,
		) -> DispatchResultWithPostInfo {
			T::RiskManagerUpdateOrigin::ensure_origin(origin)?;
			ensure!(
				pool_id.is_supported_underlying_asset(),
				Error::<T>::NotValidUnderlyingAssetId
			);
			ensure!(
				Self::is_valid_liquidation_fee(liquidation_fee),
				Error::<T>::InvalidLiquidationFeeValue
			);
			LiquidationFee::<T>::insert(pool_id, liquidation_fee);
			Self::deposit_event(Event::LiquidationFeeUpdated(liquidation_fee));
			Ok(().into())
		}

		/// Set threshold which used in liquidation to protect the user from micro liquidations.
		///
		/// - `pool_id`: PoolID for which the parameter value is being set.
		/// - `threshold`: new threshold.
		///
		/// The dispatch origin of this call must be 'RiskManagerUpdateOrigin'.
		#[pallet::weight(0)]
		#[transactional]
		pub fn set_liquidation_threshold(
			origin: OriginFor<T>,
			pool_id: CurrencyId,
			threshold: Rate,
		) -> DispatchResultWithPostInfo {
			T::RiskManagerUpdateOrigin::ensure_origin(origin)?;
			ensure!(
				pool_id.is_supported_underlying_asset(),
				Error::<T>::NotValidUnderlyingAssetId
			);
			LiquidationThreshold::<T>::put(threshold);
			Self::deposit_event(Event::LiquidationThresholdUpdated(threshold));
			Ok(().into())
		}
	}
}

// Private functions
impl<T: Config> Pallet<T> {
	/// Checks if liquidation_fee <= 0.5
	fn is_valid_liquidation_fee(liquidation_fee: Rate) -> bool {
		liquidation_fee <= Rate::saturating_from_rational(5, 10)
	}
}

impl<T: Config> RiskManagerStorageProvider for Pallet<T> {
	fn create_pool(pool_id: CurrencyId, liquidation_threshold: Rate, liquidation_fee: Rate) -> DispatchResult {
		ensure!(
			!LiquidationFee::<T>::contains_key(pool_id),
			Error::<T>::RiskManagerParamsAlreadyCreated
		);
		ensure!(
			Self::is_valid_liquidation_fee(liquidation_fee),
			Error::<T>::InvalidLiquidationFeeValue
		);
		LiquidationFee::<T>::insert(pool_id, liquidation_fee);
		LiquidationThreshold::<T>::put(liquidation_threshold);
		Ok(())
	}

	fn remove_pool(pool_id: CurrencyId) {
		LiquidationFee::<T>::remove(pool_id)
	}
}

impl<T: Config> UserLiquidationAttemptsManager<T::AccountId> for Pallet<T> {
	fn get_user_liquidation_attempts(who: &T::AccountId) -> u8 {
		Self::user_liquidation_attempts(who)
	}

	fn increase_by_one(who: &T::AccountId) {
		UserLiquidationAttempts::<T>::mutate(who, |p| *p += u8::one())
	}

	fn reset_to_zero(who: &T::AccountId) {
		UserLiquidationAttempts::<T>::mutate(&who, |p| *p = u8::zero())
	}

	/// Mutates user liquidation attempts depending on user operation.
	/// If the user makes a deposit to the collateral pool, then attempts are set to zero.
	fn mutate_depending_operation(pool_id: CurrencyId, who: &T::AccountId, operation: Operation) {
		if operation == Operation::Deposit && T::UserCollateral::is_pool_collateral(&who, pool_id) {
			let user_liquidation_attempts = Self::get_user_liquidation_attempts(&who);
			if !user_liquidation_attempts.is_zero() {
				Self::reset_to_zero(&who);
			}
		}
	}
}
