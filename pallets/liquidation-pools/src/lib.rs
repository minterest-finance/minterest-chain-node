//! # Liquidation Pools Module
//!
//! ## Overview
//!
//! Liquidation Pools are responsible for holding funds for automatic liquidation.

#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::unused_unit)]
#![allow(clippy::upper_case_acronyms)]

use codec::{Decode, Encode};
use frame_support::{ensure, pallet_prelude::*, traits::Get, transactional};
use frame_system::offchain::{SendTransactionTypes, SubmitTransaction};
use frame_system::{ensure_signed, pallet_prelude::*};
use minterest_primitives::{Balance, CurrencyId, Rate};
use orml_traits::MultiCurrency;
use orml_utilities::OffchainErr;
use pallet_traits::PoolsManager;
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};
use sp_runtime::traits::{AccountIdConversion, Zero};
use sp_runtime::{
	offchain::{
		storage::StorageValueRef,
		storage_lock::{StorageLock, Time},
		Duration,
	},
	transaction_validity::TransactionPriority,
	FixedPointNumber, ModuleId, RuntimeDebug,
};
use sp_std::{convert::TryInto, prelude::*, result};

pub const OFFCHAIN_WORKER_DATA: &[u8] = b"pallets/liquidation-pools/data/";
pub const OFFCHAIN_WORKER_LOCK: &[u8] = b"pallets/liquidation-pools/lock/";
pub const OFFCHAIN_WORKER_MAX_ITERATIONS: &[u8] = b"pallets/liquidation-pools/max-iterations/";

pub const LOCK_DURATION: u64 = 100;
pub const DEFAULT_MAX_ITERATIONS: u32 = 1000;

pub use module::*;

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

/// Liquidation Pool metadata
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(Encode, Decode, Clone, RuntimeDebug, Eq, PartialEq, Default)]
pub struct LiquidationPoolCommonData<BlockNumber> {
	/// Block number that pool was last balancing attempted at.
	pub timestamp: BlockNumber,
	/// Balancing pool frequency.
	pub balancing_period: u32,
}

/// Liquidation Pool metadata
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(Encode, Decode, Clone, RuntimeDebug, Eq, PartialEq, Default)]
pub struct LiquidationPool {
	/// Balance Deviation Threshold represents how much current value in a pool may differ from
	/// ideal value (defined by balance_ratio).
	pub deviation_threshold: Rate,
}

type LiquidityPools<T> = liquidity_pools::Module<T>;
type Accounts<T> = accounts::Module<T>;

#[frame_support::pallet]
pub mod module {
	use super::*;

	#[pallet::config]
	pub trait Config:
		frame_system::Config + liquidity_pools::Config + accounts::Config + SendTransactionTypes<Call<Self>>
	{
		/// The overarching event type.
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

		/// A configuration for base priority of unsigned transactions.
		///
		/// This is exposed so that it can be tuned for particular runtime, when
		/// multiple pallets send unsigned transactions.
		type UnsignedPriority: Get<TransactionPriority>;

		#[pallet::constant]
		/// The Liquidation Pool's module id, keep all assets in Pools.
		type LiquidationPoolsModuleId: Get<ModuleId>;

		#[pallet::constant]
		/// The Liquidation Pool's account id, keep all assets in Pools.
		type LiquidationPoolAccountId: Get<Self::AccountId>;
	}

	#[pallet::error]
	pub enum Error<T> {
		/// Number overflow in calculation.
		NumOverflow,
		/// The dispatch origin of this call must be Administrator.
		RequireAdmin,
		/// The currency is not enabled in protocol.
		NotValidUnderlyingAssetId,
		/// Value must be in range [0..1]
		NotValidDeviationThresholdValue,
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(crate) fn deposit_event)]
	pub enum Event<T: Config> {
		///  Balancing period has been successfully changed: \[who, new_period\]
		BalancingPeriodChanged(T::AccountId, u32),
		///  Deviation Threshold has been successfully changed: \[who, new_threshold_value\]
		DeviationThresholdChanged(T::AccountId, Rate),
	}

	#[pallet::storage]
	#[pallet::getter(fn liquidation_pool_params)]
	pub(crate) type LiquidationPoolParams<T: Config> =
		StorageValue<_, LiquidationPoolCommonData<T::BlockNumber>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn liquidation_pools)]
	pub(crate) type LiquidationPools<T: Config> = StorageMap<_, Twox64Concat, CurrencyId, LiquidationPool, ValueQuery>;

	#[pallet::genesis_config]
	pub struct GenesisConfig<T: Config> {
		pub liquidation_pool_params: LiquidationPoolCommonData<T::BlockNumber>,
		#[allow(clippy::type_complexity)]
		pub liquidation_pools: Vec<(CurrencyId, LiquidationPool)>,
	}

	#[cfg(feature = "std")]
	impl<T: Config> Default for GenesisConfig<T> {
		fn default() -> Self {
			GenesisConfig {
				liquidation_pool_params: LiquidationPoolCommonData {
					timestamp: TryInto::<T::BlockNumber>::try_into(1u32)
						.ok()
						.expect(" result convert failed"),
					balancing_period: 600, // Blocks per 10 minutes.
				},
				liquidation_pools: vec![],
			}
		}
	}

	#[pallet::genesis_build]
	impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
		fn build(&self) {
			LiquidationPoolParams::<T>::put(self.liquidation_pool_params.clone());
			self.liquidation_pools.iter().for_each(|(currency_id, pool_data)| {
				LiquidationPools::<T>::insert(
					currency_id,
					LiquidationPool {
						deviation_threshold: pool_data.deviation_threshold,
					},
				)
			});
		}
	}

	#[pallet::pallet]
	pub struct Pallet<T>(PhantomData<T>);

	#[pallet::hooks]
	impl<T: Config> Hooks<T::BlockNumber> for Pallet<T> {
		/// Runs after every block. Start offchain worker to check if balancing needed.
		fn offchain_worker(now: T::BlockNumber) {
			if let Err(e) = Self::_offchain_worker() {
				debug::info!(
					target: "LiquidationPool offchain worker",
					"cannot run offchain worker at {:?}: {:?}",
					now,
					e,
				);
			} else {
				debug::debug!(
					target: "LiquidationPool offchain worker",
					" LiquidationPool offchain worker start at block: {:?} already done!",
					now,
				);
			}
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Set new value of balancing period.
		/// - `pool_id`: PoolID for which the parameter value is being set.
		/// - `new_period`: New value of balancing period.
		///
		/// The dispatch origin of this call must be Administrator.
		#[pallet::weight(0)]
		#[transactional]
		pub fn set_balancing_period(origin: OriginFor<T>, new_period: u32) -> DispatchResultWithPostInfo {
			let sender = ensure_signed(origin)?;
			ensure!(<Accounts<T>>::is_admin_internal(&sender), Error::<T>::RequireAdmin);

			// Write new value into storage.
			LiquidationPoolParams::<T>::mutate(|x| x.balancing_period = new_period);

			Self::deposit_event(Event::BalancingPeriodChanged(sender, new_period));

			Ok(().into())
		}

		/// Set new value of deviation threshold.
		/// - `pool_id`: PoolID for which the parameter value is being set.
		/// - `new_threshold`: New value of deviation threshold.
		///
		/// The dispatch origin of this call must be Administrator.
		#[pallet::weight(0)]
		#[transactional]
		pub fn set_deviation_threshold(
			origin: OriginFor<T>,
			pool_id: CurrencyId,
			new_threshold: u128,
		) -> DispatchResultWithPostInfo {
			let sender = ensure_signed(origin)?;
			ensure!(<Accounts<T>>::is_admin_internal(&sender), Error::<T>::RequireAdmin);

			ensure!(
				<LiquidityPools<T>>::is_enabled_underlying_asset_id(pool_id),
				Error::<T>::NotValidUnderlyingAssetId
			);

			let new_deviation_threshold = Rate::from_inner(new_threshold);

			ensure!(
				(Rate::zero() <= new_deviation_threshold && new_deviation_threshold <= Rate::one()),
				Error::<T>::NotValidDeviationThresholdValue
			);

			// Write new value into storage.
			LiquidationPools::<T>::mutate(pool_id, |x| x.deviation_threshold = new_deviation_threshold);

			Self::deposit_event(Event::DeviationThresholdChanged(sender, new_deviation_threshold));

			Ok(().into())
		}

		/// Make balance the pool.
		///
		/// - `pool_id`: PoolID for which balancing is performed.
		///
		/// The dispatch origin of this call must be _None_.
		#[pallet::weight(0)]
		#[transactional]
		pub fn balancing(origin: OriginFor<T>, pool_id: CurrencyId) -> DispatchResultWithPostInfo {
			ensure_none(origin)?;
			Self::balancing_attempt(pool_id);
			Ok(().into())
		}
	}
}

impl<T: Config> Pallet<T> {
	fn _offchain_worker() -> Result<(), OffchainErr> {
		// Get available assets list
		let underlying_asset_ids: Vec<CurrencyId> = <T as liquidity_pools::Config>::EnabledCurrencyPair::get()
			.iter()
			.map(|currency_pair| currency_pair.underlying_id)
			.collect();

		if underlying_asset_ids.len().is_zero() {
			return Ok(());
		}

		// acquire offchain worker lock.
		let lock_expiration = Duration::from_millis(LOCK_DURATION);
		let mut lock = StorageLock::<'_, Time>::with_deadline(&OFFCHAIN_WORKER_LOCK, lock_expiration);
		let guard = lock.try_lock().map_err(|_| OffchainErr::OffchainLock)?;

		let to_be_continue = StorageValueRef::persistent(&OFFCHAIN_WORKER_DATA);

		// Get to_be_continue record
		let (pool_to_check, start_key) = if let Some(Some((last_pool_to_check, maybe_last_iterator_previous_key))) =
			to_be_continue.get::<(u32, Option<Vec<u8>>)>()
		{
			(last_pool_to_check, maybe_last_iterator_previous_key)
		} else {
			(0, None)
		};

		let currency_id = underlying_asset_ids[(pool_to_check as usize)];
		let iteration_start_time = sp_io::offchain::timestamp();

		let dead_line = Self::calculate_deadline().map_err(|_| OffchainErr::OffchainLock)?;

		if <frame_system::Module<T>>::block_number() > dead_line {
			Self::submit_unsigned_tx(currency_id);
		}

		// update to_be_continue record
		let nex_pool_id = if pool_to_check < underlying_asset_ids.len().saturating_sub(1) as u32 {
			pool_to_check + 1
		} else {
			0
		};
		to_be_continue.set(&(nex_pool_id, Option::<Vec<u8>>::None));

		let iteration_end_time = sp_io::offchain::timestamp();

		debug::info!(
			target: "LiquidationPools offchain worker",
			"iteration info:\n currency id: {:?}, start key: {:?},\n iteration start at: {:?}, end at: {:?}, execution time: {:?}\n",
			currency_id,
			start_key,
			iteration_start_time,
			iteration_end_time,
			iteration_end_time.diff(&iteration_start_time)
		);

		// Consume the guard but **do not** unlock the underlying lock.
		guard.forget();

		Ok(())
	}

	fn calculate_deadline() -> result::Result<T::BlockNumber, DispatchError> {
		let timestamp = Self::liquidation_pool_params().timestamp;
		let period = Self::liquidation_pool_params().balancing_period;

		let timestamp_as_u32 = TryInto::<u32>::try_into(timestamp)
			.ok()
			.expect("blockchain will not exceed 2^32 blocks; qed");

		Ok(
			TryInto::<T::BlockNumber>::try_into(period.checked_add(timestamp_as_u32).ok_or(Error::<T>::NumOverflow)?)
				.ok()
				.expect(" result convert failed"),
		)
	}

	fn submit_unsigned_tx(pool_id: CurrencyId) {
		let call = Call::<T>::balancing(pool_id);
		if SubmitTransaction::<T, Call<T>>::submit_unsigned_transaction(call.into()).is_err() {
			debug::info!(
				target: "LiquidityPools offchain worker",
				"submit unsigned balancing attempt for \n CurrencyId {:?} \nfailed!",
				pool_id,
			);
		}
	}

	/// Preparing data for pool balancing.
	///
	/// - `pool_id`: the CurrencyId of the pool for which automatic balancing is performed.
	fn balancing_attempt(_pool_id: CurrencyId) -> () {
		()
	}
}

impl<T: Config> PoolsManager<T::AccountId> for Pallet<T> {
	/// Gets module account id.
	fn pools_account_id() -> T::AccountId {
		T::LiquidationPoolsModuleId::get().into_account()
	}

	/// Gets current the total amount of cash the liquidation pool has.
	fn get_pool_available_liquidity(pool_id: CurrencyId) -> Balance {
		let module_account_id = Self::pools_account_id();
		T::MultiCurrency::free_balance(pool_id, &module_account_id)
	}

	/// Check if pool exists
	fn pool_exists(underlying_asset_id: &CurrencyId) -> bool {
		LiquidationPools::<T>::contains_key(underlying_asset_id)
	}
}

impl<T: Config> ValidateUnsigned for Pallet<T> {
	type Call = Call<T>;

	fn validate_unsigned(_source: TransactionSource, call: &Self::Call) -> TransactionValidity {
		match call {
			Call::balancing(pool_id) => ValidTransaction::with_tag_prefix("LiquidationPoolsOffchainWorker")
				.priority(T::UnsignedPriority::get())
				.and_provides((<frame_system::Module<T>>::block_number(), pool_id))
				.longevity(64_u64)
				.propagate(true)
				.build(),
			_ => InvalidTransaction::Call.into(),
		}
	}
}
