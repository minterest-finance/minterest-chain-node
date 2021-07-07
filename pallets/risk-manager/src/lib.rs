//! # Risk Manager Pallet
//!
//! ## Overview
//!
//! Risk Manager pallet is responsible for automatic liquidation which is done by offchain worker.
//! Liquidation occurs in the situations when user`s loan oversupply drops below the value defined
//! by Collateral factor.
//! In cases when there is enough borrowed assets and liquidation attempts hadn`t been exceeded
//! partial liquidation is executed in order to minimize user`s losses.
//! Except collateral assets confiscation, there is an additional amount defined by
//! `liquidation_fee`, which is transferred from user`s collateral liquidity_pool to liquidation
//! pool.

#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::unused_unit)]
#![allow(clippy::upper_case_acronyms)]

use codec::{Decode, Encode};
use frame_support::{ensure, log, pallet_prelude::*, traits::Get, transactional};
use frame_system::{
	ensure_none,
	offchain::{SendTransactionTypes, SubmitTransaction},
	pallet_prelude::*,
};
use liquidity_pools::{Pool, PoolUserData};
use minterest_primitives::currency::CurrencyType::UnderlyingAsset;
use minterest_primitives::{Balance, CurrencyId, OffchainErr, Rate};
use orml_traits::MultiCurrency;
use pallet_traits::{
	ControllerManager, CurrencyConverter, LiquidationPoolsManager, LiquidityPoolStorageProvider, MntManager,
	PoolsManager, PricesManager, RiskManager, UserStorageProvider,
};
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};
use sp_runtime::{
	offchain::{
		storage::StorageValueRef,
		storage_lock::{StorageLock, Time},
		Duration,
	},
	traits::{CheckedDiv, CheckedMul, One, StaticLookup, ValidateUnsigned, Zero},
	transaction_validity::{
		InvalidTransaction, TransactionPriority, TransactionSource, TransactionValidity, ValidTransaction,
	},
	DispatchError, DispatchResult, FixedPointNumber, RuntimeDebug,
};
use sp_std::{cmp::Ordering, prelude::*, result, str};

pub const OFFCHAIN_WORKER_LOCK: &[u8] = b"pallets/risk-manager/lock/";
pub const OFFCHAIN_WORKER_LATEST_POOL_INDEX: &[u8] = b"pallets/risk-manager/counter";

pub use module::*;

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

pub mod weights;
pub use weights::WeightInfo;

/// RiskManager metadata
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(Encode, Decode, RuntimeDebug, Eq, PartialEq, Default)]
pub struct RiskManagerData {
	/// The maximum amount of partial liquidation attempts.
	pub max_attempts: u8,

	/// Minimal sum for partial liquidation.
	/// Loans with amount below this parameter will be liquidate in full.
	pub min_partial_liquidation_sum: Balance,

	/// Step used in liquidation to protect the user from micro liquidations.
	pub threshold: Rate,

	/// The additional collateral which is taken from borrowers as a penalty for being liquidated.
	pub liquidation_fee: Rate,
}

type MinterestProtocol<T> = minterest_protocol::Pallet<T>;

#[frame_support::pallet]
pub mod module {
	use super::*;

	#[pallet::config]
	pub trait Config: frame_system::Config + minterest_protocol::Config + SendTransactionTypes<Call<Self>> {
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

		/// The price source of currencies.
		type PriceSource: PricesManager<CurrencyId>;

		/// A configuration for base priority of unsigned transactions.
		///
		/// This is exposed so that it can be tuned for particular runtime, when
		/// multiple pallets send unsigned transactions.
		type UnsignedPriority: Get<TransactionPriority>;

		/// The basic liquidity pools.
		type LiquidationPoolsManager: LiquidationPoolsManager<Self::AccountId>;

		/// Pools are responsible for holding funds for automatic liquidation.
		type LiquidityPoolsManager: LiquidityPoolStorageProvider<Self::AccountId, Pool>
			+ PoolsManager<Self::AccountId>
			+ CurrencyConverter
			+ UserStorageProvider<Self::AccountId, PoolUserData>;

		/// Public API of controller pallet
		type ControllerManager: ControllerManager<Self::AccountId>;

		/// Provides MNT token distribution functionality.
		type MntManager: MntManager<Self::AccountId>;

		/// The origin which may update risk manager parameters. Root or
		/// Half Minterest Council can always do this.
		type RiskManagerUpdateOrigin: EnsureOrigin<Self::Origin>;

		type RiskManagerWeightInfo: WeightInfo;

		/// Max duration time for offchain worker.
		type OffchainWorkerMaxDurationMs: Get<u64>;
	}

	#[pallet::error]
	pub enum Error<T> {
		/// Number overflow in calculation.
		NumOverflow,
		/// The currency is not enabled in protocol.
		NotValidUnderlyingAssetId,
		/// The liquidation hasn't been completed.
		LiquidationRejection,
		/// Liquidation incentive can't be less than one && greater than 1.5.
		InvalidLiquidationIncentiveValue,
		/// Feed price is invalid
		InvalidFeedPrice,
		/// Pool is already created
		PoolAlreadyCreated,
		/// Pool not found.
		PoolNotFound,
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(crate) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Max value of liquidation attempts has been successfully changed:
		/// \[attempts_amount\]
		MaxValueOFLiquidationAttempsHasChanged(u8),
		/// Min sum for partial liquidation has been successfully changed:
		/// \[min_partial_liquidation_sum\]
		MinSumForPartialLiquidationHasChanged(Balance),
		/// Threshold has been successfully changed: \[threshold\]
		ValueOfThresholdHasChanged(Rate),
		/// Liquidation fee has been successfully changed: \[threshold\]
		ValueOfLiquidationFeeHasChanged(Rate),
		/// Unsafe loan has been successfully liquidated: \[who, liquidate_amount_in_usd,
		/// liquidated_pool_id, seized_pools, partial_liquidation\]
		LiquidateUnsafeLoan(T::AccountId, Balance, CurrencyId, Vec<CurrencyId>, bool),
		/// New pool had been created: \[pool_id\]
		PoolAdded(CurrencyId),
	}

	/// Liquidation params for pools: `(max_attempts, min_partial_liquidation_sum, threshold,
	/// liquidation_fee)`.
	#[pallet::storage]
	#[pallet::getter(fn risk_manager_params)]
	pub type RiskManagerParams<T: Config> = StorageMap<_, Twox64Concat, CurrencyId, RiskManagerData, ValueQuery>;

	#[pallet::genesis_config]
	pub struct GenesisConfig<T: Config> {
		pub risk_manager_params: Vec<(CurrencyId, RiskManagerData)>,
		pub _phantom: sp_std::marker::PhantomData<T>,
	}

	#[cfg(feature = "std")]
	impl<T: Config> Default for GenesisConfig<T> {
		fn default() -> Self {
			GenesisConfig {
				risk_manager_params: vec![],
				_phantom: PhantomData,
			}
		}
	}

	#[pallet::genesis_build]
	impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
		fn build(&self) {
			self.risk_manager_params
				.iter()
				.for_each(|(currency_id, risk_manager_data)| {
					RiskManagerParams::<T>::insert(currency_id, RiskManagerData { ..*risk_manager_data })
				});
		}
	}

	#[pallet::pallet]
	pub struct Pallet<T>(PhantomData<T>);

	#[pallet::hooks]
	impl<T: Config> Hooks<T::BlockNumber> for Pallet<T> {
		/// Runs after every block. Start offchain worker to check unsafe loan and
		/// submit unsigned tx to trigger liquidation.
		fn offchain_worker(now: T::BlockNumber) {
			log::info!("Entering in RiskManager off-chain worker");

			if let Err(e) = Self::_offchain_worker() {
				log::info!(
					target: "RiskManager offchain worker",
					"cannot run offchain worker at {:?}: {:?}",
					now,
					e,
				);
			} else {
				log::debug!(
					target: "RiskManager offchain worker",
					" RiskManager offchain worker start at block: {:?} already done!",
					now,
				);
			}
			log::info!("Exited from RiskManager off-chain worker");
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Set maximum amount of partial liquidation attempts.
		/// - `pool_id`: PoolID for which the parameter value is being set.
		/// - `max_attempts`: New max value of liquidation attempts.
		///
		/// The dispatch origin of this call must be 'UpdateOrigin'.
		#[pallet::weight(T::RiskManagerWeightInfo::set_max_attempts())]
		#[transactional]
		pub fn set_max_attempts(
			origin: OriginFor<T>,
			pool_id: CurrencyId,
			max_attempts: u8,
		) -> DispatchResultWithPostInfo {
			T::RiskManagerUpdateOrigin::ensure_origin(origin)?;

			ensure!(
				pool_id.is_supported_underlying_asset(),
				Error::<T>::NotValidUnderlyingAssetId
			);

			// Write new value into storage.
			RiskManagerParams::<T>::mutate(pool_id, |r| r.max_attempts = max_attempts);

			Self::deposit_event(Event::MaxValueOFLiquidationAttempsHasChanged(max_attempts));

			Ok(().into())
		}

		/// Set minimal sum for partial liquidation.
		/// - `pool_id`: PoolID for which the parameter value is being set.
		/// - `min_partial_liquidation_sum`: New min sum for partial liquidation.
		///
		/// The dispatch origin of this call must be 'UpdateOrigin'.
		#[pallet::weight(T::RiskManagerWeightInfo::set_min_partial_liquidation_sum())]
		#[transactional]
		pub fn set_min_partial_liquidation_sum(
			origin: OriginFor<T>,
			pool_id: CurrencyId,
			min_partial_liquidation_sum: Balance,
		) -> DispatchResultWithPostInfo {
			T::RiskManagerUpdateOrigin::ensure_origin(origin)?;

			ensure!(
				pool_id.is_supported_underlying_asset(),
				Error::<T>::NotValidUnderlyingAssetId
			);

			// Write new value into storage.
			RiskManagerParams::<T>::mutate(pool_id, |r| r.min_partial_liquidation_sum = min_partial_liquidation_sum);

			Self::deposit_event(Event::MinSumForPartialLiquidationHasChanged(
				min_partial_liquidation_sum,
			));

			Ok(().into())
		}

		/// Set threshold which used in liquidation to protect the user from micro liquidations..
		/// - `pool_id`: PoolID for which the parameter value is being set.
		/// - `threshold`: new threshold.
		///
		/// The dispatch origin of this call must be 'UpdateOrigin'.
		#[pallet::weight(T::RiskManagerWeightInfo::set_threshold())]
		#[transactional]
		pub fn set_threshold(origin: OriginFor<T>, pool_id: CurrencyId, threshold: Rate) -> DispatchResultWithPostInfo {
			T::RiskManagerUpdateOrigin::ensure_origin(origin)?;

			ensure!(
				pool_id.is_supported_underlying_asset(),
				Error::<T>::NotValidUnderlyingAssetId
			);

			// Write new value into storage.
			RiskManagerParams::<T>::mutate(pool_id, |r| r.threshold = threshold);

			Self::deposit_event(Event::ValueOfThresholdHasChanged(threshold));

			Ok(().into())
		}

		/// Set Liquidation fee that covers liquidation costs.
		/// - `pool_id`: PoolID for which the parameter value is being set.
		/// - `liquidation_fee`: new liquidation incentive.
		///
		/// The dispatch origin of this call must be 'UpdateOrigin'.
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

			// Check if 1 <= liquidation_fee <= 1.5
			ensure!(
				Self::is_valid_liquidation_fee(liquidation_fee),
				Error::<T>::InvalidLiquidationIncentiveValue
			);

			// Write new value into storage.
			RiskManagerParams::<T>::mutate(pool_id, |r| r.liquidation_fee = liquidation_fee);

			Self::deposit_event(Event::ValueOfLiquidationFeeHasChanged(liquidation_fee));

			Ok(().into())
		}

		/// Liquidate unsafe loans
		///
		/// The dispatch origin of this call must be _None_.
		///
		/// - `currency_id`: PoolID for which the loan is being liquidate
		/// - `who`: loan's owner.
		#[pallet::weight(T::RiskManagerWeightInfo::liquidate())]
		#[transactional]
		pub fn liquidate(
			origin: OriginFor<T>,
			who: <T::Lookup as StaticLookup>::Source,
			pool_id: CurrencyId,
		) -> DispatchResultWithPostInfo {
			ensure_none(origin)?;
			ensure!(
				T::ManagerLiquidityPools::pool_exists(&pool_id),
				Error::<T>::PoolNotFound
			);

			let who = T::Lookup::lookup(who)?;
			Self::liquidate_unsafe_loan(who, pool_id)?;
			Ok(().into())
		}
	}
}

impl<T: Config> Pallet<T> {
	/// Checks insolvent loans and liquidate them if it required.
	fn process_insolvent_loans() -> Result<(), OffchainErr> {
		// Get available assets list
		let mut underlying_assets: Vec<CurrencyId> = CurrencyId::get_enabled_tokens_in_protocol(UnderlyingAsset)
			.into_iter()
			.filter(|&underlying_id| T::LiquidityPoolsManager::pool_exists(&underlying_id))
			.collect();
		if underlying_assets.is_empty() {
			return Ok(());
		}

		// acquire offchain worker lock
		let lock_expiration = Duration::from_millis(T::OffchainWorkerMaxDurationMs::get());
		let mut lock = StorageLock::<'_, Time>::with_deadline(&OFFCHAIN_WORKER_LOCK, lock_expiration);
		let mut guard = lock.try_lock().map_err(|_| OffchainErr::OffchainLock)?;

		let start_pool_index = match StorageValueRef::persistent(&OFFCHAIN_WORKER_LATEST_POOL_INDEX).get::<u32>() {
			Some(Some(index)) => {
				// Assume that count of enbled tokens can be changed. So make sure that index is not out of
				// bounds
				index as usize % underlying_assets.len()
			}
			_ => usize::zero(),
		};
		StorageValueRef::persistent(&OFFCHAIN_WORKER_LATEST_POOL_INDEX).clear();

		// Start iteration from the pool where we finished. Otherwise, take first pool.
		underlying_assets.rotate_left(start_pool_index);
		let mut loans_checked_count = 0;
		let mut loans_liquidated_count = 0;
		let working_start_time = sp_io::offchain::timestamp();

		for (pos, currency_id) in underlying_assets.iter().enumerate() {
			log::info!("RiskManager starts processing loans for {:?}", currency_id);
			<T as module::Config>::ControllerManager::accrue_interest_rate(*currency_id)
				.map_err(|_| OffchainErr::CheckFail)?;
			let pool_members = T::LiquidityPoolsManager::get_pool_members_with_loans(*currency_id)
				.map_err(|_| OffchainErr::CheckFail)?;
			for member in pool_members.into_iter() {
				// We check if the user has the collateral so as not to start the liquidation process
				// for users who have collateral = 0 and borrow > 0.
				let user_has_collateral = T::LiquidityPoolsManager::check_user_has_collateral(&member);

				// Checks if the liquidation should be allowed to occur.
				if user_has_collateral {
					let (_, shortfall) = <T as module::Config>::ControllerManager::get_hypothetical_account_liquidity(
						&member,
						*currency_id,
						0,
						0,
					)
					.map_err(|_| OffchainErr::CheckFail)?;
					if !shortfall.is_zero() {
						Self::submit_unsigned_liquidation(member, *currency_id);
						loans_liquidated_count += 1;
					}
				} else {
					//TODO It is place for handle the case when collateral = 0, borrow > 0
					continue;
				}

				loans_checked_count += 1;

				if guard.extend_lock().is_err() {
					// The lock's deadline is happened
					log::warn!(
						"Risk Manager offchain worker hasn't(!) processed all pools. \
						MAX duration time is expired. Loans checked count: {:?}, \
						loans liquidated count: {:?}",
						loans_checked_count,
						loans_liquidated_count
					);
					StorageValueRef::persistent(&OFFCHAIN_WORKER_LATEST_POOL_INDEX).set(&(pos as u32));
					return Ok(());
				}
			}
			log::info!("RiskManager finished processing loans for {:?}", currency_id);
		}

		let working_time = sp_io::offchain::timestamp().diff(&working_start_time);
		log::info!(
			"Risk Manager offchain worker has processed all pools. Loans checked count {:?}, \
			loans liquidated count: {:?}, execution time(ms): {:?}",
			loans_checked_count,
			loans_liquidated_count,
			working_time.millis()
		);

		Ok(())
	}

	fn _offchain_worker() -> Result<(), OffchainErr> {
		// Check if we are a potential validator
		if !sp_io::offchain::is_validator() {
			return Err(OffchainErr::NotValidator);
		}

		Self::process_insolvent_loans()?;
		Ok(())
	}

	/// Sends an unsigned liquidation transaction to the blockchain.
	///
	/// - `borrower`: the borrower in automatic liquidation.
	/// - `pool_id`: the CurrencyId of the pool with loan, for which automatic liquidation
	/// is performed.
	fn submit_unsigned_liquidation(borrower: T::AccountId, pool_id: CurrencyId) {
		let who = T::Lookup::unlookup(borrower);
		let call = Call::<T>::liquidate(who.clone(), pool_id);
		if SubmitTransaction::<T, Call<T>>::submit_unsigned_transaction(call.into()).is_err() {
			log::info!(
				target: "RiskManager offchain worker",
				"submit unsigned liquidation for \n AccountId {:?} CurrencyId {:?} \nfailed!",
				who, pool_id,
			);
		}
	}

	/// Defines the type of liquidation (partial or full) and causes liquidation.
	///
	/// - `borrower`: the borrower in automatic liquidation.
	/// - `liquidated_pool_id`: the CurrencyId of the pool with loan, for which automatic
	/// liquidation is performed.
	pub fn liquidate_unsafe_loan(borrower: T::AccountId, liquidated_pool_id: CurrencyId) -> DispatchResult {
		<T as module::Config>::ControllerManager::accrue_interest_rate(liquidated_pool_id)?;

		// Read prices price for borrowed pool.
		let price_borrowed =
			T::PriceSource::get_underlying_price(liquidated_pool_id).ok_or(Error::<T>::InvalidFeedPrice)?;

		// Get borrower borrow balance and calculate total_repay_amount (in USD):
		// total_repay_amount = borrow_balance * price_borrowed
		let borrow_balance =
			<T as module::Config>::ControllerManager::borrow_balance_stored(&borrower, liquidated_pool_id)?;
		let total_repay_amount = T::LiquidityPoolsManager::underlying_to_usd(borrow_balance, price_borrowed)?;

		let liquidation_attempts =
			T::LiquidityPoolsManager::get_user_liquidation_attempts(&borrower, liquidated_pool_id);

		let is_partial_liquidation = total_repay_amount
			>= RiskManagerParams::<T>::get(liquidated_pool_id).min_partial_liquidation_sum
			&& liquidation_attempts < RiskManagerParams::<T>::get(liquidated_pool_id).max_attempts;

		// Calculate sum required to liquidate.
		let (seize_amount, is_attempt_increment_required) =
			Self::calculate_liquidation_info(liquidated_pool_id, total_repay_amount, is_partial_liquidation)?;

		let (seized_pools, repay_amount) = Self::liquidate_borrow_fresh(&borrower, liquidated_pool_id, seize_amount)?;

		if is_attempt_increment_required {
			T::LiquidityPoolsManager::mutate_user_liquidation_attempts(
				liquidated_pool_id,
				&borrower,
				is_partial_liquidation,
			);
		}

		Self::deposit_event(Event::LiquidateUnsafeLoan(
			borrower,
			repay_amount,
			liquidated_pool_id,
			seized_pools,
			is_partial_liquidation,
		));

		Ok(())
	}

	/// The liquidation pool liquidates the borrowers collateral. The collateral seized is
	/// transferred to the liquidation pool.
	///
	/// - `borrower`: the borrower in automatic liquidation.
	/// - `liquidated_pool_id`: the CurrencyId of the pool with loan, for which automatic
	/// liquidation is performed.
	/// - `repay_assets`: the amount of the underlying borrowed asset to repay.
	/// - `seize_amount`: the number of collateral tokens to seize converted into USD.
	fn liquidate_borrow_fresh(
		borrower: &T::AccountId,
		liquidated_pool_id: CurrencyId,
		mut seize_amount: Balance,
	) -> result::Result<(Vec<CurrencyId>, Balance), DispatchError> {
		let liquidation_pool_account_id = T::LiquidationPoolsManager::pools_account_id();
		let liquidity_pool_account_id = <T as Config>::LiquidityPoolsManager::pools_account_id();

		// Get an array of collateral pools for the borrower.
		// The array is sorted in descending order by the number of wrapped tokens in USD.
		let collateral_pools = T::LiquidityPoolsManager::get_user_collateral_pools(&borrower)?;

		// Collect seized pools.
		let mut seized_pools: Vec<CurrencyId> = Vec::new();
		let mut already_seized_amount = Balance::zero();

		for collateral_pool_id in collateral_pools.into_iter() {
			if !seize_amount.is_zero() {
				<T as module::Config>::ControllerManager::accrue_interest_rate(collateral_pool_id)?;

				let wrapped_id = collateral_pool_id
					.wrapped_asset()
					.ok_or(Error::<T>::NotValidUnderlyingAssetId)?;
				let balance_wrapped_token = T::MultiCurrency::free_balance(wrapped_id, &borrower);

				// Get the exchange rate, read price for collateral pool and calculate the number
				// of collateral tokens to seize:
				// seize_tokens = seize_amount / (price_collateral * exchange_rate)
				let price_collateral =
					T::PriceSource::get_underlying_price(collateral_pool_id).ok_or(Error::<T>::InvalidFeedPrice)?;
				let exchange_rate = T::LiquidityPoolsManager::get_exchange_rate(collateral_pool_id)?;
				let seize_tokens =
					T::LiquidityPoolsManager::usd_to_wrapped(seize_amount, exchange_rate, price_collateral)?;

				<T as module::Config>::MntManager::update_mnt_supply_index(collateral_pool_id)?;
				<T as module::Config>::MntManager::distribute_supplier_mnt(collateral_pool_id, &borrower, false)?;

				// Check if there are enough collateral wrapped tokens to withdraw seize_tokens.
				let seize_underlying = match balance_wrapped_token.cmp(&seize_tokens) {
					// Not enough collateral wrapped tokens.
					Ordering::Less => {
						// seize_underlying = balance_wrapped_token * exchange_rate
						let seize_underlying =
							T::LiquidityPoolsManager::wrapped_to_underlying(balance_wrapped_token, exchange_rate)?;
						T::MultiCurrency::withdraw(wrapped_id, &borrower, balance_wrapped_token)?;
						// seize_amount = seize_amount - (seize_underlying * price_collateral)
						seize_amount -= Rate::from_inner(seize_underlying)
							.checked_mul(&price_collateral)
							.map(|x| x.into_inner())
							.ok_or(Error::<T>::NumOverflow)?;
						seize_underlying
					}
					// Enough collateral wrapped tokens. Transfer all seize_tokens to liquidation_pool.
					_ => {
						// seize_underlying = seize_tokens * exchange_rate
						let seize_underlying =
							T::LiquidityPoolsManager::wrapped_to_underlying(seize_tokens, exchange_rate)?;
						T::MultiCurrency::withdraw(wrapped_id, &borrower, seize_tokens)?;
						// seize_amount = 0, since all seize_tokens have already been withdrawn
						seize_amount = Balance::zero();
						seize_underlying
					}
				};
				T::MultiCurrency::transfer(
					collateral_pool_id,
					&liquidity_pool_account_id,
					&liquidation_pool_account_id,
					seize_underlying,
				)?;
				// already_seized_amount = already_seized_amount + (seize_underlying * price_collateral)
				already_seized_amount += Rate::from_inner(seize_underlying)
					.checked_mul(&price_collateral)
					.map(|x| x.into_inner())
					.ok_or(Error::<T>::NumOverflow)?;
				// Collecting seized pools to display in an Event.
				seized_pools.push(collateral_pool_id);
			}
		}

		let liquidation_fee = Self::risk_manager_params(liquidated_pool_id).liquidation_fee;

		let repay_amount = Rate::from_inner(already_seized_amount)
			.checked_div(&liquidation_fee)
			.map(|x| x.into_inner())
			.ok_or(Error::<T>::NumOverflow)?;

		let price_borrowed =
			T::PriceSource::get_underlying_price(liquidated_pool_id).ok_or(Error::<T>::InvalidFeedPrice)?;

		// Calculating the number of assets that must be repaid out of the liquidation pool.
		// repay_assets = already_seized_amount / (liquidation_fee * price_borrowed)
		let repay_assets = T::LiquidityPoolsManager::usd_to_underlying(repay_amount, price_borrowed)?;

		<MinterestProtocol<T>>::do_repay_fresh(
			&liquidation_pool_account_id,
			&borrower,
			liquidated_pool_id,
			repay_assets,
			false,
		)?;

		Ok((seized_pools, repay_amount))
	}

	// FIXME: Temporary implementation.
	/// Calculate sum required to liquidate for partial and complete liquidation.
	///
	/// - `liquidated_pool_id`: the CurrencyId of the pool with loan, for which automatic
	/// liquidation is performed.
	/// - `total_repay_amount`: total amount of debt converted into usd.
	/// - `is_partial_liquidation`: partial or complete liquidation.
	///
	/// Returns:
	/// `seize_amount`: - the number of collateral tokens to seize converted into USD (consider
	/// liquidation_fee).
	/// `is_attempt_increment_required`: - boolean, whether or not to increment
	/// the counter of liquidation attempts.
	pub fn calculate_liquidation_info(
		liquidated_pool_id: CurrencyId,
		total_repay_amount: Balance,
		is_partial_liquidation: bool,
	) -> result::Result<(Balance, bool), DispatchError> {
		let liquidation_fee = Self::risk_manager_params(liquidated_pool_id).liquidation_fee;
		let price_borrowed =
			T::PriceSource::get_underlying_price(liquidated_pool_id).ok_or(Error::<T>::InvalidFeedPrice)?;

		let temporary_factor = match is_partial_liquidation {
			true => Rate::saturating_from_rational(30, 100),
			false => Rate::one(),
		};

		// repay_amount = temporary_factor * total_repay_amount
		let mut repay_amount = Rate::from_inner(total_repay_amount)
			.checked_mul(&temporary_factor)
			.map(|x| x.into_inner())
			.ok_or(Error::<T>::NumOverflow)?;

		let liquidation_pool_balance = T::LiquidationPoolsManager::get_pool_available_liquidity(liquidated_pool_id);
		let liquidation_pool_balance_usd = Rate::from_inner(liquidation_pool_balance)
			.checked_mul(&price_borrowed)
			.map(|x| x.into_inner())
			.ok_or(Error::<T>::NumOverflow)?;

		// If there is not enough liquidity in the liquidation pool, then we do not change
		// the user's liquidation attempts counter.
		let is_attempt_increment_required = liquidation_pool_balance_usd >= repay_amount;

		// repay_amount = min(amount_to_liquidate, liquidation_pool_balance_usd)
		repay_amount = repay_amount.min(liquidation_pool_balance_usd);

		// seize_amount = liquidation_fee * repay_amount
		let seize_amount = Rate::from_inner(repay_amount)
			.checked_mul(&liquidation_fee)
			.map(|x| x.into_inner())
			.ok_or(Error::<T>::NumOverflow)?;

		Ok((seize_amount, is_attempt_increment_required))
	}

	fn is_valid_liquidation_fee(liquidation_fee: Rate) -> bool {
		liquidation_fee >= Rate::one() && liquidation_fee <= Rate::saturating_from_rational(15, 10)
	}
}

impl<T: Config> RiskManager for Pallet<T> {
	/// This is a part of a pool creation flow
	/// Creates storage records for RiskManagerParams
	fn create_pool(
		currency_id: CurrencyId,
		max_attempts: u8,
		min_partial_liquidation_sum: Balance,
		threshold: Rate,
		liquidation_fee: Rate,
	) -> DispatchResult {
		ensure!(
			!RiskManagerParams::<T>::contains_key(currency_id),
			Error::<T>::PoolAlreadyCreated
		);
		ensure!(
			Self::is_valid_liquidation_fee(liquidation_fee),
			Error::<T>::InvalidLiquidationIncentiveValue
		);

		RiskManagerParams::<T>::insert(
			currency_id,
			RiskManagerData {
				max_attempts,
				min_partial_liquidation_sum,
				threshold,
				liquidation_fee,
			},
		);

		Ok(())
	}
}

impl<T: Config> ValidateUnsigned for Pallet<T> {
	type Call = Call<T>;

	fn validate_unsigned(_source: TransactionSource, call: &Self::Call) -> TransactionValidity {
		match call {
			Call::liquidate(who, pool_id) => ValidTransaction::with_tag_prefix("RiskManagerOffchainWorker")
				.priority(T::UnsignedPriority::get())
				.and_provides((<frame_system::Pallet<T>>::block_number(), pool_id, who))
				.longevity(64_u64)
				.propagate(true)
				.build(),
			_ => InvalidTransaction::Call.into(),
		}
	}
}
