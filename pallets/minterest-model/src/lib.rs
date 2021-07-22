//! # Minterest Model Pallet
//!
//! ## Overview
//!
//! Minterest Model pallet is responsible for storing and updating parameters related to economy.
//!
//! In its storage, it contains all the parameters necessary for this:
//! -`kink`: the utilization point at which the jump multiplier is applied;
//! -`base_rate_per_block`: The base interest rate which is the y-intercept
//! when utilization rate is 0;
//! -`multiplier_per_block`: The multiplier of utilization rate that gives the slope
//! of the interest rate;
//! -`jump_multiplier_per_block`: the multiplier of utilization rate after hitting a specified
//! utilization point - kink.
//!
//! ## Interface
//!
//! -`calculate_borrow_interest_rate`: calculates the current borrow rate per block;
//! -`create_pool`: checks parameters validity and creates storage records for
//! MinterestModelDataStorage
//!
//! ### Dispatchable Functions (extrinsics)
//!
//! -`set_jump_multiplier`: set JumpMultiplierPerBlock from JumpMultiplierPerYear;
//! -`set_base_rate`: set BaseRatePerBlock from BaseRatePerYear;
//! -`set_multiplier`: set MultiplierPerBlock from MultiplierPerYear;
//! -`set_kink`: set parameter kink.

#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::unused_unit)]
#![allow(clippy::upper_case_acronyms)]

use codec::{Decode, Encode};
use frame_support::{ensure, pallet_prelude::*, transactional};
use frame_system::pallet_prelude::*;
use minterest_primitives::{CurrencyId, Rate};
use pallet_traits::MinterestModelManager;
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};
use sp_runtime::{
	traits::{CheckedAdd, CheckedDiv, CheckedMul, One, Zero},
	DispatchError, DispatchResult, FixedPointNumber, RuntimeDebug,
};
use sp_std::{cmp::Ordering, result};

pub use module::*;

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

pub mod weights;
pub use weights::WeightInfo;

#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(Encode, Decode, Clone, RuntimeDebug, Eq, PartialEq, Default)]
pub struct MinterestModelData {
	/// The utilization point at which the jump multiplier is applied
	pub kink: Rate,

	/// The base interest rate which is the y-intercept when utilization rate is 0
	pub base_rate_per_block: Rate,

	/// The multiplier of utilization rate that gives the slope of the interest rate
	pub multiplier_per_block: Rate,

	/// The multiplier of utilization rate after hitting a specified utilization point - kink
	pub jump_multiplier_per_block: Rate,
}

type RateResult = result::Result<Rate, DispatchError>;

#[frame_support::pallet]
pub mod module {
	use super::*;

	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// The overarching event type.
		type Event: From<Event> + IsType<<Self as frame_system::Config>::Event>;

		#[pallet::constant]
		/// The approximate number of blocks per year
		type BlocksPerYear: Get<u128>;

		/// The origin which may update minterest model parameters. Root or
		/// Half Minterest Council can always do this.
		type ModelUpdateOrigin: EnsureOrigin<Self::Origin>;

		/// Weight information for the extrinsics.
		type WeightInfo: WeightInfo;
	}

	#[pallet::error]
	pub enum Error<T> {
		/// The currency is not enabled in protocol.
		NotValidUnderlyingAssetId,
		/// Number overflow in calculation.
		NumOverflow,
		/// Base rate per block cannot be set to 0 at the same time as Multiplier per block.
		BaseRatePerBlockCannotBeZero,
		/// Multiplier per block cannot be set to 0 at the same time as Base rate per block.
		MultiplierPerBlockCannotBeZero,
		/// Parameter `kink` cannot be more than one.
		KinkCannotBeMoreThanOne,
		/// Borrow interest rate calculation error.
		BorrowRateCalculationError,
		/// Pool is already created
		PoolAlreadyCreated,
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(crate) fn deposit_event)]
	pub enum Event {
		/// JumpMultiplierPerBlock has been successfully changed.
		JumpMultiplierPerBlockChanged,
		/// BaseRatePerBlock has been successfully changed.
		BaseRatePerBlockChanged,
		/// MultiplierPerBlock has been successfully changed.
		MultiplierPerBlockChanged,
		/// Parameter `kink` has been successfully changed.
		KinkChanged,
	}

	/// The Minterest Model data information: `(kink, base_rate_per_block, multiplier_per_block,
	/// jump_multiplier_per_block)`.
	///
	/// Return:
	/// - `kink`: If Utilization Rate exceeds Kink, the protocol applies correction to Borrow
	/// Interest Rate
	/// - `base_rate_per_block`: Base Interest Rate, which is the y-intercept when Utilization Rate
	/// is 0
	/// - `multiplier_per_block`: Multiplier Per Block is a multiplier against Utilization Rate that
	/// gives the slope of the interest rate
	/// - `jump_multiplier_per_block`: Jump Multiplier Per Block is used to correct Borrow Interest
	/// Rate after Utilization Rate hits Kink
	#[pallet::storage]
	#[pallet::getter(fn minterest_model_data_storage)]
	pub type MinterestModelDataStorage<T: Config> =
		StorageMap<_, Twox64Concat, CurrencyId, MinterestModelData, ValueQuery>;

	#[pallet::genesis_config]
	pub struct GenesisConfig<T: Config> {
		pub minterest_model_params: Vec<(CurrencyId, MinterestModelData)>,
		pub _phantom: sp_std::marker::PhantomData<T>,
	}

	#[cfg(feature = "std")]
	impl<T: Config> Default for GenesisConfig<T> {
		fn default() -> Self {
			GenesisConfig {
				minterest_model_params: vec![],
				_phantom: PhantomData,
			}
		}
	}

	#[pallet::genesis_build]
	impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
		fn build(&self) {
			self.minterest_model_params
				.iter()
				.for_each(|(currency_id, minterest_model_data)| {
					MinterestModelDataStorage::<T>::insert(
						currency_id,
						MinterestModelData {
							..*minterest_model_data
						},
					)
				});
		}
	}

	#[pallet::pallet]
	pub struct Pallet<T>(PhantomData<T>);

	#[pallet::hooks]
	impl<T: Config> Hooks<T::BlockNumber> for Pallet<T> {}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Set JumpMultiplierPerBlock from JumpMultiplierPerYear.
		///
		/// Parameters:
		/// - `pool_id`: the CurrencyId of the pool for which the parameter value is being set.
		/// - `jump_multiplier_rate_per_year`:  new jump multiplier rate per year value. Used to calculate and set up multiplier per block.
		///
		/// `jump_multiplier_per_block = jump_multiplier_rate_per_year / blocks_per_year`
		/// The dispatch origin of this call must be 'ModelUpdateOrigin'.
		#[pallet::weight(0)]
		#[transactional]
		pub fn set_pool_jump_multiplier(
			origin: OriginFor<T>,
			pool_id: CurrencyId,
			jump_multiplier_rate_per_year: Rate,
		) -> DispatchResultWithPostInfo {
			T::ModelUpdateOrigin::ensure_origin(origin)?;

			ensure!(
				pool_id.is_supported_underlying_asset(),
				Error::<T>::NotValidUnderlyingAssetId
			);

			// jump_multiplier_per_block = jump_multiplier_rate_per_year / blocks_per_year
			let new_jump_multiplier_per_block = jump_multiplier_rate_per_year
				.checked_div(&Rate::saturating_from_integer(T::BlocksPerYear::get()))
				.ok_or(Error::<T>::NumOverflow)?;

			// Write the previously calculated values into storage.
			MinterestModelDataStorage::<T>::mutate(pool_id, |r| {
				r.jump_multiplier_per_block = new_jump_multiplier_per_block
			});

			Self::deposit_event(Event::JumpMultiplierPerBlockChanged);

			Ok(().into())
		}

		/// Set BaseRatePerBlock from BaseRatePerYear.
		///
		/// Parameters:
		/// - `pool_id`: the CurrencyId of the pool for which the parameter value is being set.
		/// - `base_rate_per_year`: new base rate per year value. Used to calculate and set up base rate per block.
		///
		/// `base_rate_per_block = base_rate_per_year / blocks_per_year`
		/// The dispatch origin of this call must be 'ModelUpdateOrigin'.
		#[pallet::weight(0)]
		#[transactional]
		pub fn set_pool_base_rate(
			origin: OriginFor<T>,
			pool_id: CurrencyId,
			base_rate_per_year: Rate,
		) -> DispatchResultWithPostInfo {
			T::ModelUpdateOrigin::ensure_origin(origin)?;

			ensure!(
				pool_id.is_supported_underlying_asset(),
				Error::<T>::NotValidUnderlyingAssetId
			);

			let new_base_rate_per_block = base_rate_per_year
				.checked_div(&Rate::saturating_from_integer(T::BlocksPerYear::get()))
				.ok_or(Error::<T>::NumOverflow)?;

			// Base rate per block cannot be set to 0 at the same time as Multiplier per block.
			if new_base_rate_per_block.is_zero() {
				ensure!(
					!Self::minterest_model_data_storage(pool_id)
						.multiplier_per_block
						.is_zero(),
					Error::<T>::BaseRatePerBlockCannotBeZero
				);
			}

			// Write the previously calculated values into storage.
			MinterestModelDataStorage::<T>::mutate(pool_id, |r| r.base_rate_per_block = new_base_rate_per_block);

			Self::deposit_event(Event::BaseRatePerBlockChanged);

			Ok(().into())
		}

		/// Set MultiplierPerBlock from MultiplierPerYear.
		///
		/// Parameters:
		/// - `pool_id`: the CurrencyId of the pool for which the parameter value is being set.
		/// - `multiplier_per_year`: new multiplier per year value.
		///
		/// `multiplier_per_block = multiplier_per_year / blocks_per_year`
		/// The dispatch origin of this call must be 'ModelUpdateOrigin'.
		#[pallet::weight(0)]
		#[transactional]
		pub fn set_pool_multiplier(
			origin: OriginFor<T>,
			pool_id: CurrencyId,
			multiplier_per_year: Rate,
		) -> DispatchResultWithPostInfo {
			T::ModelUpdateOrigin::ensure_origin(origin)?;

			ensure!(
				pool_id.is_supported_underlying_asset(),
				Error::<T>::NotValidUnderlyingAssetId
			);

			let new_multiplier_per_block = multiplier_per_year
				.checked_div(&Rate::saturating_from_integer(T::BlocksPerYear::get()))
				.ok_or(Error::<T>::NumOverflow)?;

			// Multiplier per block cannot be set to 0 at the same time as Base rate per block .
			ensure!(
				Self::is_valid_base_rate_and_multiplier(
					new_multiplier_per_block,
					Self::minterest_model_data_storage(pool_id).base_rate_per_block
				),
				Error::<T>::MultiplierPerBlockCannotBeZero
			);

			// Write the previously calculated values into storage.
			MinterestModelDataStorage::<T>::mutate(pool_id, |r| r.multiplier_per_block = new_multiplier_per_block);
			Self::deposit_event(Event::MultiplierPerBlockChanged);
			Ok(().into())
		}

		/// Set parameter `kink`.
		///
		/// Parameters:
		/// - `pool_id`: the CurrencyId of the pool for which the parameter value is being set.
		/// - `kink`: new kink value, must be less or equal to 1.
		///
		/// The dispatch origin of this call must be 'ModelUpdateOrigin'.
		#[pallet::weight(0)]
		#[transactional]
		pub fn set_pool_kink(origin: OriginFor<T>, pool_id: CurrencyId, kink: Rate) -> DispatchResultWithPostInfo {
			T::ModelUpdateOrigin::ensure_origin(origin)?;

			ensure!(
				pool_id.is_supported_underlying_asset(),
				Error::<T>::NotValidUnderlyingAssetId
			);

			ensure!(Self::is_valid_kink(kink), Error::<T>::KinkCannotBeMoreThanOne);

			// Write the previously calculated values into storage.
			MinterestModelDataStorage::<T>::mutate(pool_id, |r| r.kink = kink);
			Self::deposit_event(Event::KinkChanged);

			Ok(().into())
		}
	}
}

impl<T: Config> Pallet<T> {
	fn is_valid_kink(kink: Rate) -> bool {
		kink <= Rate::one()
	}

	fn is_valid_base_rate_and_multiplier(base_rate_per_block: Rate, multiplier_per_block: Rate) -> bool {
		!(base_rate_per_block.is_zero() && multiplier_per_block.is_zero())
	}
}

impl<T: Config> MinterestModelManager for Pallet<T> {
	/// This is a part of a pool creation flow
	/// Checks parameters validity and creates storage records for MinterestModelDataStorage
	fn create_pool(
		currency_id: CurrencyId,
		kink: Rate,
		base_rate_per_block: Rate,
		multiplier_per_block: Rate,
		jump_multiplier_per_block: Rate,
	) -> DispatchResult {
		ensure!(
			!MinterestModelDataStorage::<T>::contains_key(currency_id),
			Error::<T>::PoolAlreadyCreated
		);
		ensure!(Self::is_valid_kink(kink), Error::<T>::KinkCannotBeMoreThanOne);
		ensure!(
			Self::is_valid_base_rate_and_multiplier(
				multiplier_per_block,
				Self::minterest_model_data_storage(currency_id).base_rate_per_block
			),
			Error::<T>::MultiplierPerBlockCannotBeZero
		);

		MinterestModelDataStorage::<T>::insert(
			currency_id,
			MinterestModelData {
				kink,
				base_rate_per_block,
				multiplier_per_block,
				jump_multiplier_per_block,
			},
		);
		Ok(())
	}

	/// Calculates the current borrow rate per block. To perform the calculation, this function
	/// takes the main mathematical parameters from the storage. From outside, it only takes
	/// the value of the parameter Utilization Rate.
	/// - `underlying_asset`: asset ID for which the borrow interest rate is calculated.
	/// - `utilization_rate`: current Utilization rate value.
	///
	/// returns `borrow_interest_rate`.
	fn calculate_pool_borrow_interest_rate(underlying_asset: CurrencyId, utilization_rate: Rate) -> RateResult {
		let MinterestModelData {
			kink,
			base_rate_per_block,
			multiplier_per_block,
			jump_multiplier_per_block,
		} = Self::minterest_model_data_storage(underlying_asset);

		// if utilization_rate > kink:
		// normal_rate = kink * multiplier_per_block + base_rate_per_block
		// excess_util = utilization_rate * kink
		// borrow_rate = excess_util * jump_multiplier_per_block + normal_rate
		//
		// if utilization_rate <= kink:
		// borrow_rate = utilization_rate * multiplier_per_block + base_rate_per_block
		let borrow_interest_rate = match utilization_rate.cmp(&kink) {
			Ordering::Greater => {
				let normal_rate = kink
					.checked_mul(&multiplier_per_block)
					.and_then(|v| v.checked_add(&base_rate_per_block))
					.ok_or(Error::<T>::BorrowRateCalculationError)?;
				let excess_util = utilization_rate
					.checked_mul(&kink)
					.ok_or(Error::<T>::BorrowRateCalculationError)?;

				excess_util
					.checked_mul(&jump_multiplier_per_block)
					.and_then(|v| v.checked_add(&normal_rate))
					.ok_or(Error::<T>::BorrowRateCalculationError)?
			}
			_ => utilization_rate
				.checked_mul(&multiplier_per_block)
				.and_then(|v| v.checked_add(&base_rate_per_block))
				.ok_or(Error::<T>::BorrowRateCalculationError)?,
		};

		Ok(borrow_interest_rate)
	}
}
