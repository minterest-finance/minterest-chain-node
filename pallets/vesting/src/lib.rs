//! # Vesting Module
//!
//! ## Overview
//!
//! Vesting module provides a means of scheduled balance lock on an account. It
//! uses the *graded vesting* way, which unlocks a specific amount of balance
//! every period of time, until all balance unlocked.
//!
//! ### Vesting Schedule
//!
//! The schedule of a vesting is described by data structure `VestingSchedule`:
//! from the block number of `start`, for every `period` amount of blocks,
//! `per_period` amount of balance would unlocked, until number of periods
//! `period_count` reached. Note in vesting schedules, *time* is measured by
//! block number. `bucket` - Vesting bucket type. All `VestingSchedule`s under
//! an account could be queried in chain state.
//!
//! ### Vesting Buckets
//!
//! In the Minterest protocol, all Vesting are divided into `VestingBucket`. Each vesting bucket
//! has its own vesting start, vesting duration and total number of tokens.
//!
//! ## Interface
//!
//! ### Dispatchable Functions
//!
//! - `claim` - Claim unlocked balances.
//! - `vested_transfer` - Add a new vesting schedule for an account.
//! - `remove_vesting_schedules` - Remove a vesting schedule from an account. Unlocks the user's
//! balance, transfers unvested tokens to the vesting bucket.

#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::unused_unit)]

use codec::{Decode, Encode};
use frame_support::sp_runtime::{traits::CheckedMul, FixedPointNumber};
use frame_support::{
	ensure,
	pallet_prelude::*,
	traits::{Currency, EnsureOrigin, ExistenceRequirement, Get, LockIdentifier, LockableCurrency, WithdrawReasons},
	transactional,
};
use frame_system::{ensure_signed, pallet_prelude::*};
use minterest_primitives::constants::time::{BLOCKS_PER_YEAR, DAYS};
use minterest_primitives::{Balance, Rate, VestingBucket};
pub use module::*;

use sp_runtime::{
	traits::{AtLeast32Bit, StaticLookup, Zero},
	DispatchResult, RuntimeDebug,
};
use sp_std::{
	cmp::{Eq, PartialEq},
	vec::Vec,
};
pub mod weights;
pub use weights::WeightInfo;

#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

pub const VESTING_LOCK_ID: LockIdentifier = *b"mod/vest";

/// The vesting schedule.
///
/// Benefits would be granted gradually, `per_period` amount every `period`
/// of blocks after `start`.
#[derive(Clone, Encode, Decode, PartialEq, Eq, RuntimeDebug)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct VestingSchedule<BlockNumber> {
	/// Vesting bucket type
	pub bucket: VestingBucket,
	/// Vesting starting block
	pub start: BlockNumber,
	/// Number of blocks between vest
	pub period: BlockNumber,
	/// Number of vest
	pub period_count: u32,
	/// Amount of tokens to release per vest
	pub per_period: Rate,
}

impl<BlockNumber: AtLeast32Bit + Copy> VestingSchedule<BlockNumber> {
	/// Creates a new schedule with default parameters (start, period, period_count, per_period).
	///
	/// - `amount`: the number of tokens for which need to create a schedule.
	/// Note: this constructor can only be used when configuring a genesis block.
	pub fn new(bucket: VestingBucket, amount: Balance) -> Self {
		let start: BlockNumber = (bucket.unlock_begins_in_days() as u32 * DAYS).into();
		let period: BlockNumber = BlockNumber::one(); // block by block
		let period_count: u32 = bucket.vesting_duration() as u32 * BLOCKS_PER_YEAR as u32;
		let per_period = Rate::checked_from_rational(amount, period_count).expect("ensured non-zero period_count; qed");
		Self {
			bucket,
			start,
			period,
			period_count,
			per_period,
		}
	}

	/// Creates a new schedule with default parameters (period, period_count, per_period).
	///
	/// - `bucket`: vesting bucket type (must be `Team` or `Marketing` or `Strategic Partners`).
	/// - `start`: the vesting schedule starting block.
	/// - `amount`: the number of tokens for which need to create a schedule.
	pub fn new_beginning_from(bucket: VestingBucket, start: BlockNumber, amount: Balance) -> Option<Self> {
		if !bucket.is_manipulated_bucket() {
			return None;
		}
		let period: BlockNumber = BlockNumber::one(); // block by block
		let period_count: u32 = bucket.vesting_duration() as u32 * BLOCKS_PER_YEAR as u32;
		let per_period = Rate::saturating_from_rational(amount, period_count);
		Some(Self {
			bucket,
			start,
			period,
			period_count,
			per_period,
		})
	}

	/// Returns the end of all periods, `None` if calculation overflows.
	pub fn end(&self) -> Option<BlockNumber> {
		// period * period_count + start
		self.period
			.checked_mul(&self.period_count.into())?
			.checked_add(&self.start)
	}

	/// Returns all locked amount, `None` if calculation overflows.
	pub fn total_amount(&self) -> Option<Balance> {
		Rate::from_inner(self.period_count as u128)
			.checked_mul(&self.per_period)
			.map(|x| x.into_inner())
	}

	/// Returns locked amount for a given `time`.
	///
	/// Note this func assumes schedule is a valid one(non-zero period and
	/// non-overflow total amount), and it should be guaranteed by callers.
	pub fn locked_amount(&self, time: BlockNumber) -> Balance {
		// expired_periods = (time - start) / period
		// unrealized_periods = period_count - expired_periods
		// locked_amount = per_period * unrealized_periods
		let expired_periods = time
			.saturating_sub(self.start)
			.checked_div(&self.period)
			.expect("ensured non-zero period; qed");
		let unrealized_periods = self
			.period_count
			.saturating_sub(expired_periods.unique_saturated_into());
		Rate::from_inner(unrealized_periods as u128)
			.checked_mul(&self.per_period)
			.map(|x| x.into_inner())
			.expect("ensured non-overflow total amount; qed")
	}
}

#[frame_support::pallet]
pub mod module {
	use super::*;

	/// Type alias for VestingSchedule.
	pub(crate) type VestingScheduleOf<T> = VestingSchedule<<T as frame_system::Config>::BlockNumber>;
	/// Tuple struct for GenesisConfig. `(account_id, start, period, period_count, per_period)`
	pub type ScheduledItem<T> = (VestingBucket, <T as frame_system::Config>::AccountId, Balance);

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

		/// A currency whose accounts can have liquidity restrictions.
		type Currency: LockableCurrency<Self::AccountId, Moment = Self::BlockNumber, Balance = Balance>;

		#[pallet::constant]
		/// The minimum amount transferred to call `vested_transfer`.
		type MinVestedTransfer: Get<Balance>;

		/// Required origin for vested transfer.  Root or
		/// Two thirds of Minterest Council can always do this.
		type VestedTransferOrigin: EnsureOrigin<Self::Origin>;

		/// Weight information for extrinsics in this module.
		type WeightInfo: WeightInfo;

		/// The maximum number of vesting schedules an account can have.
		type MaxVestingSchedules: Get<u32>;

		#[pallet::constant]
		/// Information for each vesting bucket:
		/// (vesting bucket type, vesting_duration, unlock_begins_in_days, total_amount).
		type VestingBucketsInfo: Get<Vec<(VestingBucket, u8, u8, Balance)>>;
	}

	#[pallet::error]
	pub enum Error<T> {
		/// Vesting period is zero
		ZeroVestingPeriod,
		/// Number of vests is zero
		ZeroVestingPeriodCount,
		/// Arithmetic calculation overflow
		NumOverflow,
		/// Insufficient amount of balance to lock
		InsufficientBalanceToLock,
		/// This account have too many vesting schedules
		TooManyVestingSchedules,
		/// The vested transfer amount is too low
		AmountLow,
		/// Incorrect vesting bucket type. Only vesting from Marketing, Team and
		/// Strategic Partners buckets can be created or removed.
		IncorrectVestingBucketType,
		/// Incorrect vesting bucket account id.
		IncorrectVestingBucketAccountId,
		/// The user does not have such a schedule
		UserDoesNotHaveSuchSchedule,
	}

	#[pallet::event]
	#[pallet::generate_deposit(fn deposit_event)]
	pub enum Event<T: Config> {
		/// Added new vesting schedule. [to, vesting_schedule]
		VestingScheduleAdded(T::AccountId, VestingScheduleOf<T>),
		/// Claimed vesting. [who, locked_amount]
		Claimed(T::AccountId, Balance),
		/// Removed vesting schedules. [who]
		VestingSchedulesRemoved(T::AccountId),
	}

	/// Vesting schedules of an account.
	#[pallet::storage]
	#[pallet::getter(fn vesting_schedule_storage)]
	pub type VestingScheduleStorage<T: Config> =
		StorageMap<_, Blake2_128Concat, T::AccountId, Vec<VestingScheduleOf<T>>, ValueQuery>;

	#[pallet::genesis_config]
	pub struct GenesisConfig<T: Config> {
		pub vesting: Vec<ScheduledItem<T>>,
	}

	#[cfg(feature = "std")]
	impl<T: Config> Default for GenesisConfig<T> {
		fn default() -> Self {
			GenesisConfig { vesting: vec![] }
		}
	}

	#[pallet::genesis_build]
	impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
		fn build(&self) {
			self.vesting.iter().for_each(|(bucket, who, total)| {
				assert!(
					T::Currency::free_balance(who) >= *total,
					"Account do not have enough balance"
				);

				// We do not set a schedule for Market Making vesting bucket.
				if *bucket != VestingBucket::MarketMaking {
					let schedule = VestingSchedule::new(*bucket, *total);
					Pallet::<T>::ensure_valid_vesting_schedule(&schedule).unwrap();
					T::Currency::set_lock(VESTING_LOCK_ID, who, *total, WithdrawReasons::all());
					VestingScheduleStorage::<T>::insert(who, vec![schedule]);
				}
			});
		}
	}

	#[pallet::pallet]
	pub struct Pallet<T>(PhantomData<T>);

	#[pallet::hooks]
	impl<T: Config> Hooks<T::BlockNumber> for Pallet<T> {}

	#[pallet::call]
	impl<T: Config> Pallet<T>
	where
		<T as frame_system::Config>::AccountId: From<[u8; 32]>,
	{
		/// Claim unlocked balances.
		/// Can not get VestingSchedule count from `who`, so use `MaxVestingSchedules / 2`.
		#[pallet::weight(T::WeightInfo::claim((<T as Config>::MaxVestingSchedules::get() / 2) as u32))]
		pub fn claim(origin: OriginFor<T>) -> DispatchResultWithPostInfo {
			let who = ensure_signed(origin)?;
			let locked_amount = Self::do_claim(&who);

			Self::deposit_event(Event::Claimed(who, locked_amount));
			Ok(().into())
		}

		/// Add a new vesting schedule for an account. Transfer balance
		/// from the vesting bucket account to target account.
		///
		/// The dispatch origin of this call must be `VestedTransferOrigin`.
		///
		/// Parameters:
		/// - `target`: the AccountId on which the vesting schedule is created;
		/// - `bucket`: vesting bucket type (must be `Team` or `Marketing` or `Strategic Partners`);
		/// - `start`: block number in which the vesting schedule starts to work;
		/// - `amount`: the balance for which the vesting schedule is created.
		#[pallet::weight(T::WeightInfo::vested_transfer())]
		pub fn vested_transfer(
			origin: OriginFor<T>,
			target: <T::Lookup as StaticLookup>::Source,
			bucket: VestingBucket,
			start: T::BlockNumber,
			amount: Balance,
		) -> DispatchResultWithPostInfo {
			T::VestedTransferOrigin::ensure_origin(origin)?;
			let target = T::Lookup::lookup(target)?;

			let schedule: VestingSchedule<T::BlockNumber> = VestingSchedule::new_beginning_from(bucket, start, amount)
				.ok_or(Error::<T>::IncorrectVestingBucketType)?;

			let raw_bucket_account_id: [u8; 32] = bucket
				.bucket_account_id()
				.ok_or(Error::<T>::IncorrectVestingBucketType)?
				.into();

			Self::do_vested_transfer(&raw_bucket_account_id.into(), &target, schedule.clone())?;

			Self::deposit_event(Event::VestingScheduleAdded(target, schedule));
			Ok(().into())
		}

		/// Remove a vesting schedule from an account. Transfer unvested tokens to the
		/// vesting bucket.
		///
		/// The dispatch origin of this call must be `VestedTransferOrigin`.
		///
		/// Parameters:
		/// - `target`: the account that receives vesting schedule of the MNT tokens;
		/// - `bucket`: the type of vesting bucket from which we want to delete the schedule;
		#[pallet::weight(T::WeightInfo::remove_vesting_schedules())]
		pub fn remove_vesting_schedules(
			origin: OriginFor<T>,
			target: <T::Lookup as StaticLookup>::Source,
			bucket: VestingBucket,
		) -> DispatchResultWithPostInfo {
			T::VestedTransferOrigin::ensure_origin(origin)?;
			let account = T::Lookup::lookup(target)?;
			ensure!(bucket.is_manipulated_bucket(), Error::<T>::IncorrectVestingBucketType);

			Self::do_remove_vesting_schedule(&account, bucket)?;

			Self::deposit_event(Event::VestingSchedulesRemoved(account));
			Ok(().into())
		}
	}
}

impl<T: Config> Pallet<T> {
	/// Claim unlocked balances.
	fn do_claim(who: &T::AccountId) -> Balance {
		let locked = Self::locked_balance(who);
		if locked.is_zero() {
			T::Currency::remove_lock(VESTING_LOCK_ID, who);
		} else {
			T::Currency::set_lock(VESTING_LOCK_ID, who, locked, WithdrawReasons::all());
		}
		locked
	}

	/// Returns locked balance based on current block number.
	fn locked_balance(who: &T::AccountId) -> Balance {
		let now = <frame_system::Pallet<T>>::block_number();
		<VestingScheduleStorage<T>>::mutate_exists(who, |maybe_schedules| {
			let total_locked = if let Some(schedules) = maybe_schedules {
				let mut total: Balance = Zero::zero();
				// leave only schedules with a locked balance
				schedules.retain(|s| {
					// calculate the remaining number of locked tokens in the schedule
					let locked_amount = s.locked_amount(now);
					total = total.saturating_add(locked_amount);
					!locked_amount.is_zero()
				});
				total
			} else {
				Zero::zero()
			};
			// If there is no locked balance left, then clear the schedule vector
			if total_locked.is_zero() {
				*maybe_schedules = None;
			}
			total_locked
		})
	}

	/// Add a new vesting schedule for an account. Transfer balance
	/// from the vesting bucket account to target account.
	#[transactional]
	fn do_vested_transfer(from: &T::AccountId, to: &T::AccountId, schedule: VestingScheduleOf<T>) -> DispatchResult {
		let schedule_amount = Self::ensure_valid_vesting_schedule(&schedule)?;

		ensure!(
			<VestingScheduleStorage<T>>::decode_len(to).unwrap_or(0) < T::MaxVestingSchedules::get() as usize,
			Error::<T>::TooManyVestingSchedules
		);

		let total_locked = Self::locked_balance(to)
			.checked_add(schedule_amount)
			.ok_or(Error::<T>::NumOverflow)?;

		T::Currency::transfer(from, to, schedule_amount, ExistenceRequirement::AllowDeath)?;
		T::Currency::set_lock(VESTING_LOCK_ID, to, total_locked, WithdrawReasons::all());
		<VestingScheduleStorage<T>>::append(to, schedule);
		Ok(())
	}

	/// Remove a vesting schedule from an account. Transfer unvested tokens to the
	/// vesting bucket.
	#[transactional]
	fn do_remove_vesting_schedule(target: &T::AccountId, bucket: VestingBucket) -> DispatchResult
	where
		<T as frame_system::Config>::AccountId: From<[u8; 32]>,
	{
		let now = <frame_system::Pallet<T>>::block_number();
		<VestingScheduleStorage<T>>::try_mutate_exists(target, |maybe_schedules| -> DispatchResult {
			// `total_locked` - the balance that needs to be locked for the user after deleting the schedule
			// `total_removed` - the balance to be sent from the user account to the bucket account
			let (mut total_locked, mut total_removed) = (Balance::zero(), Balance::zero());

			if let Some(schedules) = maybe_schedules {
				// leave only the schedules that are not deleted
				schedules.retain(|schedule| {
					if schedule.bucket == bucket {
						total_removed += schedule.locked_amount(now);
					} else {
						total_locked += schedule.locked_amount(now);
					}
					schedule.bucket != bucket
				});

				ensure!(!total_removed.is_zero(), Error::<T>::UserDoesNotHaveSuchSchedule);

				if total_locked.is_zero() {
					T::Currency::remove_lock(VESTING_LOCK_ID, target);
				} else {
					T::Currency::set_lock(VESTING_LOCK_ID, target, total_locked, WithdrawReasons::all());
				}

				let raw_bucket_account_id: [u8; 32] = bucket
					.bucket_account_id()
					.ok_or(Error::<T>::IncorrectVestingBucketType)?
					.into();
				T::Currency::transfer(
					target,
					&raw_bucket_account_id.into(),
					total_removed,
					ExistenceRequirement::AllowDeath,
				)?;

				// If there is no schedules left, then clear the schedule vector
				if schedules.is_empty() {
					*maybe_schedules = None;
				}
			}
			Ok(())
		})
	}

	/// Returns `Ok(total_amount)` if valid schedule, or error.
	fn ensure_valid_vesting_schedule(schedule: &VestingScheduleOf<T>) -> Result<Balance, Error<T>> {
		ensure!(!schedule.period.is_zero(), Error::<T>::ZeroVestingPeriod);
		ensure!(!schedule.period_count.is_zero(), Error::<T>::ZeroVestingPeriodCount);
		ensure!(schedule.end().is_some(), Error::<T>::NumOverflow);

		let total_amount = schedule.total_amount().ok_or(Error::<T>::NumOverflow)?;

		ensure!(total_amount >= T::MinVestedTransfer::get(), Error::<T>::AmountLow);

		Ok(total_amount)
	}
}
