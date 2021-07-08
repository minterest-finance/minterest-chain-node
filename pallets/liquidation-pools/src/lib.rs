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
use liquidity_pools::Pool;
use minterest_primitives::{arithmetic::sum_with_mult_result, Balance, CurrencyId, OffchainErr, Rate};
use orml_traits::MultiCurrency;
use pallet_traits::{
	CurrencyConverter, DEXManager, LiquidationPoolsManager, LiquidityPoolStorageProvider, PoolsManager, PricesManager,
};
use sp_runtime::{
	offchain::storage_lock::{StorageLock, Time},
	traits::{AccountIdConversion, CheckedMul, One, Zero},
	transaction_validity::TransactionPriority,
	DispatchResult, FixedPointNumber, RuntimeDebug,
};

use minterest_primitives::currency::CurrencyType::UnderlyingAsset;
pub use module::*;
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
	pub max_ideal_balance: Option<Balance>,
}

type BalanceResult = sp_std::result::Result<Balance, DispatchError>;

#[frame_support::pallet]
pub mod module {
	use super::*;
	use pallet_traits::CurrencyConverter;

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
		type PriceSource: PricesManager<CurrencyId>;

		/// The basic liquidity pools manager.
		type LiquidityPoolsManager: LiquidityPoolStorageProvider<Self::AccountId, Pool>
			+ CurrencyConverter
			+ PoolsManager<Self::AccountId>;

		/// The origin which may update liquidation pools parameters. Root or
		/// Half Minterest Council can always do this.
		type UpdateOrigin: EnsureOrigin<Self::Origin>;

		/// The DEX participating in balancing
		type Dex: DEXManager<Self::AccountId, CurrencyId, Balance>;

		/// Weight information for the extrinsics.
		type LiquidationPoolsWeightInfo: WeightInfo;
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
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(crate) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Liquidation pools are balanced
		LiquidationPoolsBalanced,
		///  Deviation Threshold has been successfully changed: \[pool_id, new_threshold_value\]
		DeviationThresholdChanged(CurrencyId, Rate),
		///  Balance ratio has been successfully changed: \[pool_id, new_threshold_value\]
		BalanceRatioChanged(CurrencyId, Rate),
		///  Maximum ideal balance has been successfully changed: \[pool_id, new_threshold_value\]
		MaxIdealBalanceChanged(CurrencyId, Option<Balance>),
		///  Successful transfer to liquidation pull: \[underlying_asset_id, underlying_amount,
		/// who\]
		TransferToLiquidationPool(CurrencyId, Balance, T::AccountId),
	}

	#[pallet::storage]
	#[pallet::getter(fn liquidation_pools_data)]
	pub type LiquidationPoolsData<T: Config> = StorageMap<_, Twox64Concat, CurrencyId, LiquidationPoolData, ValueQuery>;

	#[pallet::genesis_config]
	pub struct GenesisConfig<T: Config> {
		#[allow(clippy::type_complexity)]
		pub liquidation_pools: Vec<(CurrencyId, LiquidationPoolData)>,
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
			self.liquidation_pools.iter().for_each(|(currency_id, pool_data)| {
				LiquidationPoolsData::<T>::insert(currency_id, LiquidationPoolData { ..*pool_data })
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
		/// Set new value of deviation threshold.
		/// - `pool_id`: PoolID for which the parameter value is being set.
		/// - `threshold`: New value of deviation threshold.
		///
		/// The dispatch origin of this call must be 'UpdateOrigin'.
		#[pallet::weight(T::LiquidationPoolsWeightInfo::set_deviation_threshold())]
		#[transactional]
		pub fn set_deviation_threshold(
			origin: OriginFor<T>,
			pool_id: CurrencyId,
			threshold: u128,
		) -> DispatchResultWithPostInfo {
			T::UpdateOrigin::ensure_origin(origin)?;

			ensure!(
				pool_id.is_supported_underlying_asset(),
				Error::<T>::NotValidUnderlyingAssetId
			);
			ensure!(
				T::LiquidityPoolsManager::pool_exists(&pool_id),
				Error::<T>::PoolNotFound
			);

			let new_deviation_threshold = Rate::from_inner(threshold);
			ensure!(
				Self::is_valid_deviation_threshold(new_deviation_threshold),
				Error::<T>::NotValidDeviationThresholdValue
			);

			// Write new value into storage.
			LiquidationPoolsData::<T>::mutate(pool_id, |x| x.deviation_threshold = new_deviation_threshold);

			Self::deposit_event(Event::DeviationThresholdChanged(pool_id, new_deviation_threshold));

			Ok(().into())
		}

		/// Set new value of balance ratio.
		/// - `pool_id`: PoolID for which the parameter value is being set.
		/// - `balance_ratio`: New value of balance ratio.
		///
		/// The dispatch origin of this call must be 'UpdateOrigin'.
		#[pallet::weight(T::LiquidationPoolsWeightInfo::set_balance_ratio())]
		#[transactional]
		pub fn set_balance_ratio(
			origin: OriginFor<T>,
			pool_id: CurrencyId,
			balance_ratio: u128,
		) -> DispatchResultWithPostInfo {
			T::UpdateOrigin::ensure_origin(origin)?;

			ensure!(
				pool_id.is_supported_underlying_asset(),
				Error::<T>::NotValidUnderlyingAssetId
			);
			ensure!(
				T::LiquidityPoolsManager::pool_exists(&pool_id),
				Error::<T>::PoolNotFound
			);

			let new_balance_ratio = Rate::from_inner(balance_ratio);
			ensure!(
				Self::is_valid_balance_ratio(new_balance_ratio),
				Error::<T>::NotValidBalanceRatioValue
			);

			// Write new value into storage.
			LiquidationPoolsData::<T>::mutate(pool_id, |x| x.balance_ratio = new_balance_ratio);

			Self::deposit_event(Event::BalanceRatioChanged(pool_id, new_balance_ratio));

			Ok(().into())
		}

		/// Set new value of maximum ideal balance.
		/// - `pool_id`: PoolID for which the parameter value is being set.
		/// - `max_ideal_balance`: New value of maximum ideal balance.
		///
		/// The dispatch origin of this call must be 'UpdateOrigin'.
		#[pallet::weight(T::LiquidationPoolsWeightInfo::set_max_ideal_balance())]
		#[transactional]
		pub fn set_max_ideal_balance(
			origin: OriginFor<T>,
			pool_id: CurrencyId,
			max_ideal_balance: Option<Balance>,
		) -> DispatchResultWithPostInfo {
			T::UpdateOrigin::ensure_origin(origin)?;

			ensure!(
				pool_id.is_supported_underlying_asset(),
				Error::<T>::NotValidUnderlyingAssetId
			);
			ensure!(
				T::LiquidityPoolsManager::pool_exists(&pool_id),
				Error::<T>::PoolNotFound
			);

			// Write new value into storage.
			LiquidationPoolsData::<T>::mutate(pool_id, |x| x.max_ideal_balance = max_ideal_balance);

			Self::deposit_event(Event::MaxIdealBalanceChanged(pool_id, max_ideal_balance));

			Ok(().into())
		}

		/// Make balance the liquidation pools.
		///
		/// The dispatch origin of this call must be _None_.
		#[pallet::weight(T::LiquidationPoolsWeightInfo::balance_liquidation_pools())]
		#[transactional]
		pub fn balance_liquidation_pools(
			origin: OriginFor<T>,
			supply_pool_id: CurrencyId,
			target_pool_id: CurrencyId,
			max_supply_amount: Balance,
			target_amount: Balance,
		) -> DispatchResultWithPostInfo {
			let _ = ensure_none(origin)?;
			ensure!(
				T::LiquidityPoolsManager::pool_exists(&supply_pool_id)
					&& T::LiquidityPoolsManager::pool_exists(&target_pool_id),
				Error::<T>::PoolNotFound
			);

			let module_id = Self::pools_account_id();
			T::Dex::swap_with_exact_target(
				&module_id,
				supply_pool_id,
				target_pool_id,
				max_supply_amount,
				target_amount,
			)?;
			Self::deposit_event(Event::LiquidationPoolsBalanced);
			Ok(().into())
		}

		/// Seed the liquidation pool
		/// - `underlying_asset_id`: currency of transfer
		/// - `underlying_amount`: amount to transfer to liquidation pool
		#[pallet::weight(T::LiquidationPoolsWeightInfo::transfer_to_liquidation_pool())]
		#[transactional]
		pub fn transfer_to_liquidation_pool(
			origin: OriginFor<T>,
			underlying_asset_id: CurrencyId,
			underlying_amount: Balance,
		) -> DispatchResultWithPostInfo {
			let who = ensure_signed(origin)?;

			ensure!(
				underlying_asset_id.is_supported_underlying_asset(),
				Error::<T>::NotValidUnderlyingAssetId
			);
			ensure!(
				T::LiquidityPoolsManager::pool_exists(&underlying_asset_id),
				Error::<T>::PoolNotFound
			);
			ensure!(underlying_amount > Balance::zero(), Error::<T>::ZeroBalanceTransaction);
			ensure!(
				underlying_amount <= T::MultiCurrency::free_balance(underlying_asset_id, &who),
				Error::<T>::NotEnoughLiquidityAvailable
			);

			T::MultiCurrency::transfer(underlying_asset_id, &who, &Self::pools_account_id(), underlying_amount)?;

			Self::deposit_event(Event::TransferToLiquidationPool(
				underlying_asset_id,
				underlying_amount,
				who,
			));
			Ok(().into())
		}
	}
}

/// Used in the liquidation pools balancing algorithm.
#[derive(Debug, Clone)]
struct LiquidationInformation {
	/// CurrencyId
	pool_id: CurrencyId,
	/// Pool current balance in USD.
	balance: Balance,
	/// Pool balance above ideal value (USD).
	oversupply: Balance,
	/// Pool balance below ideal value (USD).
	shortfall: Balance,
}

/// Information about the operations required for balancing Liquidation Pools.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Sales {
	/// Liquidation pool CurrencyId with oversupply.
	pub supply_pool_id: CurrencyId,
	/// Liquidation pool CurrencyId with shortfall.
	pub target_pool_id: CurrencyId,
	/// The amount of underlying asset to transfer from the oversupply pool to the shortfall pool.
	pub amount: Balance,
}

impl<T: Config> Pallet<T> {
	fn _offchain_worker(_now: T::BlockNumber) -> Result<(), OffchainErr> {
		// Check if we are a potential validator
		if !sp_io::offchain::is_validator() {
			return Err(OffchainErr::NotValidator);
		}
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
				let (max_supply_amount, target_amount) =
					Self::get_amounts(sale.supply_pool_id, sale.target_pool_id, sale.amount)?;
				Self::submit_unsigned_tx(
					sale.supply_pool_id,
					sale.target_pool_id,
					max_supply_amount,
					target_amount,
				);
				Ok(())
			})?;
		Ok(())
	}

	fn submit_unsigned_tx(
		supply_pool_id: CurrencyId,
		target_pool_id: CurrencyId,
		max_supply_amount: Balance,
		target_amount: Balance,
	) {
		let call =
			Call::<T>::balance_liquidation_pools(supply_pool_id, target_pool_id, max_supply_amount, target_amount);
		if SubmitTransaction::<T, Call<T>>::submit_unsigned_transaction(call.into()).is_err() {
			log::info!(
				target: "liquidation-pools offchain worker",
				"submit unsigned balancing tx for \n CurrencyId {:?} and CurrencyId {:?} \nfailed!",
				supply_pool_id, target_pool_id,
			);
		}
	}

	/// Collects information about required transactions on DEX.
	fn collects_sales_list() -> sp_std::result::Result<Vec<Sales>, DispatchError> {
		// Collecting information about the current state of liquidation pools.
		let (mut information_vec, mut sum_oversupply, mut sum_shortfall) =
			CurrencyId::get_enabled_tokens_in_protocol(UnderlyingAsset)
				.iter()
				.filter(|&underlying_id| T::LiquidityPoolsManager::pool_exists(underlying_id))
				.try_fold(
					(Vec::<LiquidationInformation>::new(), Balance::zero(), Balance::zero()),
					|(mut current_vec, mut current_sum_oversupply, mut current_sum_shortfall),
					 pool_id|
					 -> sp_std::result::Result<(Vec<LiquidationInformation>, Balance, Balance), DispatchError> {
						let oracle_price =
							T::PriceSource::get_underlying_price(*pool_id).ok_or(Error::<T>::InvalidFeedPrice)?;
						let liquidation_pool_supply_underlying = Self::get_pool_available_liquidity(*pool_id);
						let liquidation_pool_supply_usd = T::LiquidityPoolsManager::underlying_to_usd(
							liquidation_pool_supply_underlying,
							oracle_price,
						)?;
						let ideal_balance = Self::calculate_ideal_balance(*pool_id)?;

						// If the pool is not balanced:
						// oversupply = liquidation_pool_balance - ideal_balance
						// shortfall = ideal_balance - liquidation_pool_balance
						let (oversupply, shortfall) = match liquidation_pool_supply_usd.cmp(&ideal_balance) {
							Ordering::Greater => (liquidation_pool_supply_usd - ideal_balance, Balance::zero()),
							Ordering::Less => (Balance::zero(), ideal_balance - liquidation_pool_supply_usd),
							Ordering::Equal => (Balance::zero(), Balance::zero()),
						};

						current_vec.push(LiquidationInformation {
							pool_id: *pool_id,
							balance: liquidation_pool_supply_usd,
							oversupply,
							shortfall,
						});

						// Calculate sum_extra and sum_shortfall for all pools.
						let deviation_threshold = Self::liquidation_pools_data(*pool_id).deviation_threshold;
						// right_border = ideal_balance + ideal_balance * deviation_threshold
						let right_border = sum_with_mult_result(ideal_balance, ideal_balance, deviation_threshold)
							.map_err(|_| Error::<T>::BalanceOverflow)?;

						// left_border = ideal_balance - ideal_balance * deviation_threshold
						let left_border = ideal_balance
							.checked_sub(
								Rate::from_inner(ideal_balance)
									.checked_mul(&deviation_threshold)
									.map(|x| x.into_inner())
									.ok_or(Error::<T>::NumOverflow)?,
							)
							.ok_or(Error::<T>::NumOverflow)?;

						if liquidation_pool_supply_usd > right_border {
							current_sum_oversupply = current_sum_oversupply
								.checked_add(oversupply)
								.ok_or(Error::<T>::BalanceOverflow)?;
						}
						if liquidation_pool_supply_usd < left_border {
							current_sum_shortfall = current_sum_shortfall
								.checked_add(shortfall)
								.ok_or(Error::<T>::BalanceOverflow)?;
						}

						Ok((current_vec, current_sum_oversupply, current_sum_shortfall))
					},
				)?;

		// Contains information about the necessary transactions on the DEX.
		let mut to_sell_list: Vec<Sales> = Vec::new();

		while sum_shortfall > Balance::zero() && sum_oversupply > Balance::zero() {
			// Find the pool with the maximum oversupply and the pool with the maximum shortfall.
			let (max_oversupply_index, max_oversupply_pool_id, max_oversupply) = information_vec
				.iter()
				.enumerate()
				.max_by(|(_, a), (_, b)| a.oversupply.cmp(&b.oversupply))
				.map(|(index, pool)| (index, pool.pool_id, pool.oversupply))
				.ok_or(Error::<T>::PoolNotFound)?;

			let (max_shortfall_index, max_shortfall_pool_id, max_shortfall) = information_vec
				.iter()
				.enumerate()
				.max_by(|(_, a), (_, b)| a.shortfall.cmp(&b.shortfall))
				.map(|(index, pool)| (index, pool.pool_id, pool.shortfall))
				.ok_or(Error::<T>::PoolNotFound)?;

			// The number USD equivalent to be sent to the DEX will be equal to
			// the minimum value between (max_shortfall, max_oversupply).
			let bite_in_usd = max_oversupply.min(max_shortfall);

			// Add "sale" to the sales list.
			to_sell_list.push(Sales {
				supply_pool_id: max_oversupply_pool_id,
				target_pool_id: max_shortfall_pool_id,
				amount: bite_in_usd,
			});

			// Updating the information vector.
			let pool_with_max_oversupply = &mut information_vec[max_oversupply_index];
			pool_with_max_oversupply.balance = pool_with_max_oversupply
				.balance
				.checked_sub(bite_in_usd)
				.ok_or(Error::<T>::NotEnoughBalance)?;
			pool_with_max_oversupply.oversupply = pool_with_max_oversupply
				.oversupply
				.checked_sub(bite_in_usd)
				.ok_or(Error::<T>::NotEnoughBalance)?;

			let pool_with_max_shortfall = &mut information_vec[max_shortfall_index];
			pool_with_max_shortfall.balance = pool_with_max_shortfall
				.balance
				.checked_add(bite_in_usd)
				.ok_or(Error::<T>::NotEnoughBalance)?;
			pool_with_max_shortfall.shortfall = pool_with_max_shortfall
				.shortfall
				.checked_sub(bite_in_usd)
				.ok_or(Error::<T>::NotEnoughBalance)?;

			sum_oversupply = sum_oversupply.checked_sub(bite_in_usd).ok_or(Error::<T>::NumOverflow)?;
			sum_shortfall = sum_shortfall.checked_sub(bite_in_usd).ok_or(Error::<T>::NumOverflow)?;
		}

		Ok(to_sell_list)
	}

	/// Temporary function
	fn get_amounts(
		supply_pool_id: CurrencyId,
		target_pool_id: CurrencyId,
		amount: Balance,
	) -> sp_std::result::Result<(Balance, Balance), DispatchError> {
		let supply_oracle_price =
			T::PriceSource::get_underlying_price(supply_pool_id).ok_or(Error::<T>::InvalidFeedPrice)?;
		let target_oracle_price =
			T::PriceSource::get_underlying_price(target_pool_id).ok_or(Error::<T>::InvalidFeedPrice)?;
		let max_supply_amount = T::LiquidityPoolsManager::usd_to_underlying(amount, supply_oracle_price)?;
		let target_amount = T::LiquidityPoolsManager::usd_to_underlying(amount, target_oracle_price)?;
		Ok((max_supply_amount, target_amount))
	}

	/// Calculates ideal balance for pool balancing
	/// - `pool_id`: PoolID for which the ideal balance is calculated.
	///
	/// Returns minimum of (liquidity_pool_balance * balance_ratio * oracle_price) and
	/// max_ideal_balance
	fn calculate_ideal_balance(pool_id: CurrencyId) -> BalanceResult {
		let oracle_price = T::PriceSource::get_underlying_price(pool_id).ok_or(Error::<T>::InvalidFeedPrice)?;
		let balance_ratio = Self::liquidation_pools_data(pool_id).balance_ratio;
		// Liquidation pool ideal balance in USD: liquidity_pool_balance * balance_ratio * oracle_price
		let ideal_balance_usd = Rate::from_inner(T::LiquidityPoolsManager::get_pool_available_liquidity(pool_id))
			.checked_mul(&balance_ratio)
			.and_then(|v| v.checked_mul(&oracle_price))
			.map(|x| x.into_inner())
			.ok_or(Error::<T>::BalanceOverflow)?;

		match Self::liquidation_pools_data(pool_id).max_ideal_balance {
			Some(max_ideal_balance) => Ok(ideal_balance_usd.min(max_ideal_balance)),
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
	/// Gets current the total amount of cash the liquidation pool has.
	fn get_pool_available_liquidity(pool_id: CurrencyId) -> Balance {
		let module_account_id = Self::pools_account_id();
		T::MultiCurrency::free_balance(pool_id, &module_account_id)
	}
}

impl<T: Config> LiquidationPoolsManager<T::AccountId> for Pallet<T> {
	/// This is a part of a pool creation flow
	/// Checks parameters validity and creates storage records for LiquidationPoolsData
	fn create_pool(currency_id: CurrencyId, deviation_threshold: Rate, balance_ratio: Rate) -> DispatchResult {
		ensure!(
			!LiquidationPoolsData::<T>::contains_key(currency_id),
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

		LiquidationPoolsData::<T>::insert(
			currency_id,
			LiquidationPoolData {
				deviation_threshold,
				balance_ratio,
				max_ideal_balance: None,
			},
		);
		Ok(())
	}
}

impl<T: Config> ValidateUnsigned for Pallet<T> {
	type Call = Call<T>;

	fn validate_unsigned(_source: TransactionSource, call: &Self::Call) -> TransactionValidity {
		match call {
			Call::balance_liquidation_pools(_supply_pool_id, _target_pool_id, _max_supply_amount, _target_amount) => {
				ValidTransaction::with_tag_prefix("LiquidationPoolsOffchainWorker")
					.priority(T::UnsignedPriority::get())
					.and_provides(<frame_system::Pallet<T>>::block_number())
					.longevity(64_u64)
					.propagate(true)
					.build()
			}
			_ => InvalidTransaction::Call.into(),
		}
	}
}
