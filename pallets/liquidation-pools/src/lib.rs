//! # Liquidation Pools Module
//!
//! ## Overview
//!
//! Liquidation Pools are responsible for holding funds for automatic liquidation.
//! This module has offchain worker implemented which is running constantly.
//! Offchain worker keeps pools in balance to avoid lack of funds for liquidation.

#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::unused_unit)]
#![allow(clippy::upper_case_acronyms)]

use codec::{Decode, Encode};
use frame_support::{ensure, log, pallet_prelude::*, traits::Get, transactional, PalletId};
use frame_system::{
	offchain::{SendTransactionTypes, SubmitTransaction},
	pallet_prelude::*,
};
use liquidity_pools::PoolData;
use minterest_primitives::{
	arithmetic::sum_with_mult_result, OriginalAsset, Balance, CurrencyId, OffchainErr, Rate,
};
pub use module::*;
use orml_traits::MultiCurrency;
use pallet_traits::{
	ControllerManager, CurrencyConverter, DEXManager, LiquidationPoolsManager, LiquidityPoolStorageProvider,
	PoolsManager, PricesManager,
};
use sp_runtime::{
	offchain::storage_lock::{StorageLock, Time},
	traits::{AccountIdConversion, CheckedMul, One, Zero},
	transaction_validity::TransactionPriority,
	DispatchResult, FixedPointNumber, RuntimeDebug,
};
use sp_std::{cmp::Ordering, prelude::*};

#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};
#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

pub mod weights;
pub use weights::WeightInfo;

const OFFCHAIN_LIQUIDATION_WORKER_LOCK: &[u8] = b"pallets/liquidation-pools/lock/";

/// Liquidation Pool metadata
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(Encode, Decode, Clone, RuntimeDebug, Eq, PartialEq, Default)]
pub struct LiquidationPoolData {
	/// Balance Deviation Threshold represents how much current value in a pool may differ from
	/// ideal value (defined by balance_ratio).
	pub deviation_threshold: Rate,
	/// Balance Ration represents the percentage of Working pool value to be covered by value in
	/// Liquidation Poll.
	pub balance_ratio: Rate,
	/// Maximum ideal balance during pool balancing
	pub max_ideal_balance_usd: Option<Balance>,
}

type BalanceResult = sp_std::result::Result<Balance, DispatchError>;

#[frame_support::pallet]
pub mod module {
	use super::*;

	#[pallet::config]
	pub trait Config: frame_system::Config + SendTransactionTypes<Call<Self>> {
		/// The overarching event type.
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

		/// The `MultiCurrency` implementation.
		type MultiCurrency: MultiCurrency<Self::AccountId, Balance = Balance, CurrencyId = CurrencyId>;

		/// A configuration for base priority of unsigned transactions.
		///
		/// This is exposed so that it can be tuned for particular runtime, when
		/// multiple pallets send unsigned transactions.
		type UnsignedPriority: Get<TransactionPriority>;

		#[pallet::constant]
		/// The Liquidation Pool's module id, keep all assets in Pools.
		type LiquidationPoolsPalletId: Get<PalletId>;

		#[pallet::constant]
		/// The Liquidation Pool's account id, keep all assets in Pools.
		type LiquidationPoolAccountId: Get<Self::AccountId>;

		/// The price source of currencies
		type PriceSource: PricesManager<OriginalAsset>;

		/// The basic liquidity pools manager.
		type LiquidityPoolsManager: LiquidityPoolStorageProvider<Self::AccountId, PoolData>
			+ CurrencyConverter
			+ PoolsManager<Self::AccountId>;

		/// The origin which may update liquidation pools parameters. Root or
		/// Half Minterest Council can always do this.
		type UpdateOrigin: EnsureOrigin<Self::Origin>;

		/// The DEX participating in balancing
		type Dex: DEXManager<Self::AccountId, Balance>;

		/// Weight information for the extrinsics.
		type LiquidationPoolsWeightInfo: WeightInfo;

		/// Public API of controller pallet
		type ControllerManager: ControllerManager<Self::AccountId>;
	}

	#[pallet::error]
	pub enum Error<T> {
		/// Number overflow in calculation.
		NumOverflow,
		/// Balance exceeds maximum value.
		BalanceOverflow,
		/// The currency is not enabled in protocol.
		NotValidUnderlyingAssetId,
		/// Value must be in range [0..1]
		NotValidDeviationThresholdValue,
		/// Value must be in range [0..1]
		NotValidBalanceRatioValue,
		/// Feed price is invalid
		InvalidFeedPrice,
		/// Could not find a pool with required parameters
		PoolNotFound,
		/// Pool is already created
		PoolAlreadyCreated,
		/// Not enough liquidation pool balance.
		NotEnoughBalance,
		/// There is not enough liquidity available on user balance.
		NotEnoughLiquidityAvailable,
		/// Transaction with zero balance is not allowed.
		ZeroBalanceTransaction,
		/// Wrong state for balansing switcher. QA only!
		BalacingStateChangeError,
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(crate) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Liquidation pools are balanced
		LiquidationPoolsBalanced,
		///  Deviation Threshold has been successfully changed: \[pool_id, new_threshold_value\]
		DeviationThresholdChanged(OriginalAsset, Rate),
		///  Balance ratio has been successfully changed: \[pool_id, new_threshold_value\]
		BalanceRatioChanged(OriginalAsset, Rate),
		///  Maximum ideal balance has been successfully changed: \[pool_id, new_threshold_value\]
		MaxIdealBalanceChanged(OriginalAsset, Option<Balance>),
		///  Successful transfer to liquidation pull: \[pool_id, underlying_amount,
		/// who\]
		TransferToLiquidationPool(OriginalAsset, Balance, T::AccountId),
		/// Pool balancing state switched: \[new_state\]. QA only!
		PoolBalacingStateChanged(bool),
	}

	/// Return parameters for liquidation pool configuration.
	///
	/// Return:
	/// - `deviation_threshold`: Deviation Threshold represents how much current value in a pool
	/// may differ from ideal value (defined by balance_ratio).
	/// - `balance_ratio`: Balance Ratio represents the percentage of Working pool value to be
	/// covered by value in Liquidation Pool.
	/// - `max_ideal_balance`: Max Ideal Balance represents the ideal balance of Liquidation Pool
	/// and is used to limit ideal balance during pool balancing.
	///
	/// Storage location:
	/// [`MNT Storage`](?search=liquidation_pools::module::Pallet::liquidation_pools_data)
	#[doc(alias = "MNT Storage")]
	#[doc(alias = "MNT liquidation_pools")]
	#[pallet::storage]
	#[pallet::getter(fn liquidation_pool_data_storage)]
	pub type LiquidationPoolDataStorage<T: Config> =
		StorageMap<_, Twox64Concat, OriginalAsset, LiquidationPoolData, ValueQuery>;

	#[pallet::type_value]
	pub fn BalancingStateDefault<T: Config>() -> bool {
		true
	}
	#[pallet::storage]
	#[pallet::getter(fn pool_balancing_enabled_storage)]
	pub type PoolBalancingEnabledStorage<T: Config> = StorageValue<_, bool, ValueQuery, BalancingStateDefault<T>>;

	#[pallet::genesis_config]
	pub struct GenesisConfig<T: Config> {
		#[allow(clippy::type_complexity)]
		pub liquidation_pools: Vec<(OriginalAsset, LiquidationPoolData)>,
		pub phantom: PhantomData<T>,
	}

	#[cfg(feature = "std")]
	impl<T: Config> Default for GenesisConfig<T> {
		fn default() -> Self {
			GenesisConfig {
				liquidation_pools: vec![],
				phantom: PhantomData,
			}
		}
	}

	#[pallet::genesis_build]
	impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
		fn build(&self) {
			self.liquidation_pools.iter().for_each(|(asset, pool_data)| {
				LiquidationPoolDataStorage::<T>::insert(asset, LiquidationPoolData { ..*pool_data })
			});
		}
	}

	#[pallet::pallet]
	pub struct Pallet<T>(PhantomData<T>);

	#[pallet::hooks]
	impl<T: Config> Hooks<T::BlockNumber> for Pallet<T> {
		fn offchain_worker(now: T::BlockNumber) {
			if let Err(error) = Self::_offchain_worker(now) {
				log::info!(
					target: "LiquidationPool offchain worker",
					"cannot run offchain worker at {:?}: {:?}",
					now,
					error,
				);
			} else {
				log::debug!(
					target: "LiquidationPool offchain worker",
					" LiquidationPool offchain worker start at block: {:?} already done!",
					now,
				);
			}
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// QA only functional!
		/// Switch on/off pools balancing
		/// - `new_state`: true - balancing on, false - off
		///
		/// origin should be root.
		#[pallet::weight(10_000)]
		#[transactional]
		pub fn switch_balancing_state(origin: OriginFor<T>, new_state: bool) -> DispatchResultWithPostInfo {
			ensure_root(origin)?;

			PoolBalancingEnabledStorage::<T>::try_mutate(|mode| -> DispatchResultWithPostInfo {
				ensure!(*mode != new_state, Error::<T>::BalacingStateChangeError);
				*mode = new_state;
				Self::deposit_event(Event::PoolBalacingStateChanged(new_state));
				Ok(().into())
			})
		}

		/// Set new value of deviation threshold.
		///
		/// Parameters:
		/// - `pool_id`: the OriginalAsset of the pool for which the parameter value is being set.
		/// - `threshold`: New value of deviation threshold.
		///
		/// The dispatch origin of this call must be 'UpdateOrigin'.
		#[doc(alias = "MNT Extrinsic")]
		#[doc(alias = "MNT liquidation_pools")]
		#[pallet::weight(T::LiquidationPoolsWeightInfo::set_deviation_threshold())]
		#[transactional]
		pub fn set_deviation_threshold(
			origin: OriginFor<T>,
			pool_id: OriginalAsset,
			threshold: u128,
		) -> DispatchResultWithPostInfo {
			T::UpdateOrigin::ensure_origin(origin)?;

			ensure!(
				T::LiquidityPoolsManager::pool_exists(pool_id),
				Error::<T>::PoolNotFound
			);

			let new_deviation_threshold = Rate::from_inner(threshold);
			ensure!(
				Self::is_valid_deviation_threshold(new_deviation_threshold),
				Error::<T>::NotValidDeviationThresholdValue
			);

			// Write new value into storage.
			LiquidationPoolDataStorage::<T>::mutate(pool_id, |x| x.deviation_threshold = new_deviation_threshold);

			Self::deposit_event(Event::DeviationThresholdChanged(pool_id, new_deviation_threshold));

			Ok(().into())
		}

		/// Set new value of balance ratio.
		///
		/// Parameters:
		/// - `pool_id`: the OriginalAsset of the pool for which the parameter value is being set.
		/// - `balance_ratio`: New value of balance ratio.
		///
		/// The dispatch origin of this call must be 'UpdateOrigin'.
		#[doc(alias = "MNT Extrinsic")]
		#[doc(alias = "MNT liquidation_pools")]
		#[pallet::weight(T::LiquidationPoolsWeightInfo::set_balance_ratio())]
		#[transactional]
		pub fn set_balance_ratio(
			origin: OriginFor<T>,
			pool_id: OriginalAsset,
			balance_ratio: u128,
		) -> DispatchResultWithPostInfo {
			T::UpdateOrigin::ensure_origin(origin)?;

			ensure!(
				T::LiquidityPoolsManager::pool_exists(pool_id),
				Error::<T>::PoolNotFound
			);

			let new_balance_ratio = Rate::from_inner(balance_ratio);
			ensure!(
				Self::is_valid_balance_ratio(new_balance_ratio),
				Error::<T>::NotValidBalanceRatioValue
			);

			// Write new value into storage.
			LiquidationPoolDataStorage::<T>::mutate(pool_id, |x| x.balance_ratio = new_balance_ratio);

			Self::deposit_event(Event::BalanceRatioChanged(pool_id, new_balance_ratio));

			Ok(().into())
		}

		/// Set new value of maximum ideal balance.
		///
		/// Parameters:
		/// - `pool_id`: the OriginalAsset of the pool for which the parameter value is being set.
		/// - `max_ideal_balance`: New value of maximum ideal balance.
		///
		/// The dispatch origin of this call must be 'UpdateOrigin'.
		#[doc(alias = "MNT Extrinsic")]
		#[doc(alias = "MNT liquidation_pools")]
		#[pallet::weight(T::LiquidationPoolsWeightInfo::set_max_ideal_balance())]
		#[transactional]
		pub fn set_max_ideal_balance(
			origin: OriginFor<T>,
			pool_id: OriginalAsset,
			max_ideal_balance_usd: Option<Balance>,
		) -> DispatchResultWithPostInfo {
			T::UpdateOrigin::ensure_origin(origin)?;

			ensure!(
				T::LiquidityPoolsManager::pool_exists(pool_id),
				Error::<T>::PoolNotFound
			);

			// Write new value into storage.
			LiquidationPoolDataStorage::<T>::mutate(pool_id, |x| x.max_ideal_balance_usd = max_ideal_balance_usd);

			Self::deposit_event(Event::MaxIdealBalanceChanged(pool_id, max_ideal_balance_usd));

			Ok(().into())
		}

		/// Make balance the liquidation pools.
		///
		/// The dispatch origin of this call must be _None_.
		///
		/// Parameters:
		/// - `supply_pool_id`: the pool from which tokens are sent for sale on DEX
		/// - `target_pool_id`: pool for which tokens are bought on DEX
		/// - `max_supply_amount`: the maximum number of tokens for sale from the `supply_pool_id`
		/// pool on DEX to buy `target_amount` of tokens
		/// - `target_amount`: number of tokens to buy in `target_pool_id` on DEX
		#[doc(alias = "MNT Extrinsic")]
		#[doc(alias = "MNT liquidation_pools")]
		#[pallet::weight(T::LiquidationPoolsWeightInfo::balance_liquidation_pools())]
		#[transactional]
		pub fn balance_liquidation_pools(
			origin: OriginFor<T>,
			supply_pool_id: OriginalAsset,
			target_pool_id: OriginalAsset,
			max_supply_amount_underlying: Balance,
			target_amount_underlying: Balance,
		) -> DispatchResultWithPostInfo {
			let _ = ensure_none(origin)?;
			ensure!(
				T::LiquidityPoolsManager::pool_exists(supply_pool_id)
					&& T::LiquidityPoolsManager::pool_exists(target_pool_id),
				Error::<T>::PoolNotFound
			);

			let module_id = Self::pools_account_id();
			T::Dex::swap_with_exact_target(
				&module_id,
				supply_pool_id.into(),
				target_pool_id.into(),
				max_supply_amount_underlying,
				target_amount_underlying,
			)?;
			Self::deposit_event(Event::LiquidationPoolsBalanced);
			Ok(().into())
		}

		/// Seed the liquidation pool
		///
		/// Parameters:
		/// - `pool_id`: currency of transfer
		/// - `underlying_amount`: amount to transfer to liquidation pool
		#[doc(alias = "MNT Extrinsic")]
		#[doc(alias = "MNT liquidation_pools")]
		#[pallet::weight(T::LiquidationPoolsWeightInfo::transfer_to_liquidation_pool())]
		#[transactional]
		pub fn transfer_to_liquidation_pool(
			origin: OriginFor<T>,
			pool_id: OriginalAsset,
			underlying_amount: Balance,
		) -> DispatchResultWithPostInfo {
			let who = ensure_signed(origin)?;

			ensure!(
				T::LiquidityPoolsManager::pool_exists(pool_id),
				Error::<T>::PoolNotFound
			);
			ensure!(underlying_amount > Balance::zero(), Error::<T>::ZeroBalanceTransaction);
			ensure!(
				underlying_amount <= T::MultiCurrency::free_balance(pool_id.into(), &who),
				Error::<T>::NotEnoughLiquidityAvailable
			);

			T::MultiCurrency::transfer(pool_id.into(), &who, &Self::pools_account_id(), underlying_amount)?;

			Self::deposit_event(Event::TransferToLiquidationPool(
				pool_id,
				underlying_amount,
				who,
			));
			Ok(().into())
		}
	}

	#[pallet::validate_unsigned]
	impl<T: Config> ValidateUnsigned for Pallet<T> {
		type Call = Call<T>;

		fn validate_unsigned(_source: TransactionSource, call: &Self::Call) -> TransactionValidity {
			match call {
				Call::balance_liquidation_pools(
					_supply_pool_id,
					_target_pool_id,
					_max_supply_amount,
					_target_amount,
				) => ValidTransaction::with_tag_prefix("LiquidationPoolsOffchainWorker")
					.priority(T::UnsignedPriority::get())
					.and_provides(<frame_system::Pallet<T>>::block_number())
					.longevity(64_u64)
					.propagate(true)
					.build(),
				_ => InvalidTransaction::Call.into(),
			}
		}
	}
}

/// Used in the liquidation pools balancing algorithm.
#[derive(Debug, Clone)]
struct LiquidationInformation {
	/// OriginalAsset
	pool_id: OriginalAsset,
	/// Pool current balance in USD.
	balance_usd: Balance,
	/// Pool balance above ideal value (USD).
	oversupply_usd: Balance,
	/// Pool balance below ideal value (USD).
	shortfall_usd: Balance,
}

/// Information about the operations required for balancing Liquidation Pools.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Sales {
	/// Liquidation pool OriginalAsset with oversupply.
	pub supply_pool_id: OriginalAsset,
	/// Liquidation pool OriginalAsset with shortfall.
	pub target_pool_id: OriginalAsset,
	/// The amount of underlying asset in usd to transfer from the oversupply pool to the shortfall
	/// pool.
	pub amount_usd: Balance,
}

impl<T: Config> Pallet<T> {
	fn _offchain_worker(_now: T::BlockNumber) -> Result<(), OffchainErr> {
		// Check if we are a potential validator and balancing is enabled.
		ensure!(sp_io::offchain::is_validator(), OffchainErr::NotValidator);
		// Check if pool balansing is switched ON.
		ensure!(Self::pool_balancing_enabled_storage(), OffchainErr::PoolsBalancingIsOff);

		let mut lock = StorageLock::<Time>::new(&OFFCHAIN_LIQUIDATION_WORKER_LOCK);
		// If pools balancing procedure already started should be returned OffchainLock error.
		// To prevent any race condition sutiations.
		let _guard = lock.try_lock().map_err(|_| OffchainErr::OffchainLock)?;
		Self::pools_balancing().map_err(|_| OffchainErr::PoolsBalancingError)?;
		Ok(())
	}

	/// Makes balancing of liquidation pools if it necessary.
	fn pools_balancing() -> DispatchResult {
		// If balancing of pools isn't required then collects_sales_list returns empty list
		// and next steps won't be processed.
		Self::collects_sales_list()?
			.iter()
			.try_for_each(|sale: &Sales| -> DispatchResult {
				let (max_supply_amount_underlying, target_amount_underlying) =
					Self::get_amounts(sale.supply_pool_id, sale.target_pool_id, sale.amount_usd)?;
				Self::submit_unsigned_tx(
					sale.supply_pool_id,
					sale.target_pool_id,
					max_supply_amount_underlying,
					target_amount_underlying,
				);
				Ok(())
			})?;
		Ok(())
	}

	fn submit_unsigned_tx(
		supply_pool_id: OriginalAsset,
		target_pool_id: OriginalAsset,
		max_supply_amount_underlying: Balance,
		target_amount_underlying: Balance,
	) {
		let call = Call::<T>::balance_liquidation_pools(
			supply_pool_id,
			target_pool_id,
			max_supply_amount_underlying,
			target_amount_underlying,
		);
		if SubmitTransaction::<T, Call<T>>::submit_unsigned_transaction(call.into()).is_err() {
			log::info!(
				target: "liquidation-pools offchain worker",
				"submit unsigned balancing tx for \n OriginalAsset {:?} and CurrencyId {:?} \nfailed!",
				supply_pool_id, target_pool_id,
			);
		}
	}

	/// Collects information about required transactions on DEX.
	fn collects_sales_list() -> sp_std::result::Result<Vec<Sales>, DispatchError> {
		// Collecting information about the current state of liquidation pools.
		let (mut information_vec, mut sum_oversupply_usd, mut sum_shortfall_usd) =
			OriginalAsset::get_original_assets()
				.into_iter()
				.filter(|&&pool_id| T::LiquidityPoolsManager::pool_exists(pool_id))
				.try_fold(
					(Vec::<LiquidationInformation>::new(), Balance::zero(), Balance::zero()),
					|(mut current_vec, mut current_sum_oversupply_usd, mut current_sum_shortfall_usd),
					 &pool_id|
					 -> sp_std::result::Result<(Vec<LiquidationInformation>, Balance, Balance), DispatchError> {
						T::ControllerManager::accrue_interest_rate(pool_id)?;
						let oracle_price =
							T::PriceSource::get_underlying_price(pool_id).ok_or(Error::<T>::InvalidFeedPrice)?;
						let liquidation_pool_supply_underlying = Self::get_pool_available_liquidity(pool_id);
						let liquidation_pool_supply_usd = T::LiquidityPoolsManager::underlying_to_usd(
							liquidation_pool_supply_underlying,
							oracle_price,
						)?;
						let pool_ideal_balance_usd = Self::calculate_pool_ideal_balance_usd(pool_id)?;

						// If the pool is not balanced:
						// oversupply_usd = liquidation_pool_balance - pool_ideal_balance_usd
						// shortfall_usd = pool_ideal_balance_usd - liquidation_pool_balance
						let (oversupply_usd, shortfall_usd) = match liquidation_pool_supply_usd
							.cmp(&pool_ideal_balance_usd)
						{
							Ordering::Greater => {
								(liquidation_pool_supply_usd - pool_ideal_balance_usd, Balance::zero())
							}
							Ordering::Less => (Balance::zero(), pool_ideal_balance_usd - liquidation_pool_supply_usd),
							Ordering::Equal => (Balance::zero(), Balance::zero()),
						};

						current_vec.push(LiquidationInformation {
							pool_id,
							balance_usd: liquidation_pool_supply_usd,
							oversupply_usd,
							shortfall_usd,
						});

						// Calculate sum_extra and sum_shortfall for all pools.
						let deviation_threshold = Self::liquidation_pool_data_storage(pool_id).deviation_threshold;
						// right_border = pool_ideal_balance_usd + pool_ideal_balance_usd * deviation_threshold
						let right_border =
							sum_with_mult_result(pool_ideal_balance_usd, pool_ideal_balance_usd, deviation_threshold)
								.map_err(|_| Error::<T>::BalanceOverflow)?;

						// left_border = pool_ideal_balance_usd - pool_ideal_balance_usd * deviation_threshold
						let left_border = pool_ideal_balance_usd
							.checked_sub(
								Rate::from_inner(pool_ideal_balance_usd)
									.checked_mul(&deviation_threshold)
									.map(|x| x.into_inner())
									.ok_or(Error::<T>::NumOverflow)?,
							)
							.ok_or(Error::<T>::NumOverflow)?;

						if liquidation_pool_supply_usd > right_border {
							current_sum_oversupply_usd = current_sum_oversupply_usd
								.checked_add(oversupply_usd)
								.ok_or(Error::<T>::BalanceOverflow)?;
						}
						if liquidation_pool_supply_usd < left_border {
							current_sum_shortfall_usd = current_sum_shortfall_usd
								.checked_add(shortfall_usd)
								.ok_or(Error::<T>::BalanceOverflow)?;
						}

						Ok((current_vec, current_sum_oversupply_usd, current_sum_shortfall_usd))
					},
				)?;

		// Contains information about the necessary transactions on the DEX.
		let mut to_sell_list: Vec<Sales> = Vec::new();

		while sum_shortfall_usd > Balance::zero() && sum_oversupply_usd > Balance::zero() {
			// Find the pool with the maximum oversupply and the pool with the maximum shortfall.
			let (max_oversupply_index, max_oversupply_pool_id, max_oversupply_usd) = information_vec
				.iter()
				.enumerate()
				.max_by(|(_, a), (_, b)| a.oversupply_usd.cmp(&b.oversupply_usd))
				.map(|(index, pool)| (index, pool.pool_id, pool.oversupply_usd))
				.ok_or(Error::<T>::PoolNotFound)?;

			let (max_shortfall_index, max_shortfall_pool_id, max_shortfall_usd) = information_vec
				.iter()
				.enumerate()
				.max_by(|(_, a), (_, b)| a.shortfall_usd.cmp(&b.shortfall_usd))
				.map(|(index, pool)| (index, pool.pool_id, pool.shortfall_usd))
				.ok_or(Error::<T>::PoolNotFound)?;

			// The number USD equivalent to be sent to the DEX will be equal to
			// the minimum value between (max_shortfall_usd, max_oversupply_usd).
			let bite_usd = max_oversupply_usd.min(max_shortfall_usd);

			// Add "sale" to the sales list.
			to_sell_list.push(Sales {
				supply_pool_id: max_oversupply_pool_id,
				target_pool_id: max_shortfall_pool_id,
				amount_usd: bite_usd,
			});

			// Updating the information vector.
			let pool_with_max_oversupply = &mut information_vec[max_oversupply_index];
			pool_with_max_oversupply.balance_usd = pool_with_max_oversupply
				.balance_usd
				.checked_sub(bite_usd)
				.ok_or(Error::<T>::NotEnoughBalance)?;
			pool_with_max_oversupply.oversupply_usd = pool_with_max_oversupply
				.oversupply_usd
				.checked_sub(bite_usd)
				.ok_or(Error::<T>::NotEnoughBalance)?;

			let pool_with_max_shortfall = &mut information_vec[max_shortfall_index];
			pool_with_max_shortfall.balance_usd = pool_with_max_shortfall
				.balance_usd
				.checked_add(bite_usd)
				.ok_or(Error::<T>::NotEnoughBalance)?;
			pool_with_max_shortfall.shortfall_usd = pool_with_max_shortfall
				.shortfall_usd
				.checked_sub(bite_usd)
				.ok_or(Error::<T>::NotEnoughBalance)?;

			sum_oversupply_usd = sum_oversupply_usd
				.checked_sub(bite_usd)
				.ok_or(Error::<T>::NumOverflow)?;
			sum_shortfall_usd = sum_shortfall_usd.checked_sub(bite_usd).ok_or(Error::<T>::NumOverflow)?;
		}

		Ok(to_sell_list)
	}

	/// Temporary function
	fn get_amounts(
		supply_pool_id: OriginalAsset,
		target_pool_id: OriginalAsset,
		amount_usd: Balance,
	) -> sp_std::result::Result<(Balance, Balance), DispatchError> {
		let supply_oracle_price =
			T::PriceSource::get_underlying_price(supply_pool_id).ok_or(Error::<T>::InvalidFeedPrice)?;
		let target_oracle_price =
			T::PriceSource::get_underlying_price(target_pool_id).ok_or(Error::<T>::InvalidFeedPrice)?;
		let max_supply_amount_underlying =
			T::LiquidityPoolsManager::usd_to_underlying(amount_usd, supply_oracle_price)?;
		let target_amount_underlying = T::LiquidityPoolsManager::usd_to_underlying(amount_usd, target_oracle_price)?;
		Ok((max_supply_amount_underlying, target_amount_underlying))
	}

	/// Calculates ideal balance for pool balancing
	/// - `pool_id`: PoolID for which the ideal balance is calculated.
	///
	/// Returns minimum of (liquidity_pool_borrow_underlying * balance_ratio * oracle_price) and
	/// max_ideal_balance_usd
	fn calculate_pool_ideal_balance_usd(pool_id: OriginalAsset) -> BalanceResult {
		let oracle_price = T::PriceSource::get_underlying_price(pool_id).ok_or(Error::<T>::InvalidFeedPrice)?;
		let balance_ratio = Self::liquidation_pool_data_storage(pool_id).balance_ratio;
		// Liquidation pool ideal balance in USD: liquidity_pool_total_borrow * balance_ratio *
		// oracle_price
		let ideal_balance_usd = Rate::from_inner(T::LiquidityPoolsManager::get_pool_borrow_underlying(pool_id))
			.checked_mul(&balance_ratio)
			.and_then(|v| v.checked_mul(&oracle_price))
			.map(|x| x.into_inner())
			.ok_or(Error::<T>::BalanceOverflow)?;

		match Self::liquidation_pool_data_storage(pool_id).max_ideal_balance_usd {
			Some(max_ideal_balance_usd) => Ok(ideal_balance_usd.min(max_ideal_balance_usd)),
			None => Ok(ideal_balance_usd),
		}
	}

	fn is_valid_deviation_threshold(deviation_threshold: Rate) -> bool {
		Rate::zero() <= deviation_threshold && deviation_threshold <= Rate::one()
	}

	fn is_valid_balance_ratio(balance_ratio: Rate) -> bool {
		Rate::zero() <= balance_ratio && balance_ratio <= Rate::one()
	}
}

impl<T: Config> PoolsManager<T::AccountId> for Pallet<T> {
	/// Gets module account id.
	fn pools_account_id() -> T::AccountId {
		T::LiquidationPoolsPalletId::get().into_account()
	}
	/// Gets current liquidation pool underlying amount.
	fn get_pool_available_liquidity(pool_id: OriginalAsset) -> Balance {
		let module_account_id = Self::pools_account_id();
		T::MultiCurrency::free_balance(pool_id.into(), &module_account_id)
	}
}

impl<T: Config> LiquidationPoolsManager<T::AccountId> for Pallet<T> {
	/// This is a part of a pool creation flow
	/// Checks parameters validity and creates storage records for LiquidationPoolDataStorage
	fn create_pool(pool_id: OriginalAsset, deviation_threshold: Rate, balance_ratio: Rate) -> DispatchResult {
		ensure!(
			!LiquidationPoolDataStorage::<T>::contains_key(pool_id),
			Error::<T>::PoolAlreadyCreated
		);
		ensure!(
			Self::is_valid_deviation_threshold(deviation_threshold),
			Error::<T>::NotValidDeviationThresholdValue
		);
		ensure!(
			Self::is_valid_balance_ratio(balance_ratio),
			Error::<T>::NotValidBalanceRatioValue
		);

		LiquidationPoolDataStorage::<T>::insert(
			pool_id,
			LiquidationPoolData {
				deviation_threshold,
				balance_ratio,
				max_ideal_balance_usd: None,
			},
		);
		Ok(())
	}
}
