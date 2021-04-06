//! # Risk Manager Module
//!
//! ## Overview
//!
//! TODO: add overview.

#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::unused_unit)]
#![allow(clippy::upper_case_acronyms)]

use codec::{Decode, Encode};
use frame_support::{debug, ensure, traits::Get};
use frame_support::{pallet_prelude::*, transactional};
use frame_system::pallet_prelude::*;
use frame_system::{
	ensure_none,
	offchain::{SendTransactionTypes, SubmitTransaction},
};
use minterest_primitives::{Balance, CurrencyId, OffchainErr, Rate};
use orml_traits::MultiCurrency;
use pallet_traits::{PoolsManager, PriceProvider};
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};
use sp_runtime::traits::{CheckedDiv, CheckedMul, One};
use sp_runtime::{
	offchain::{
		storage::StorageValueRef,
		storage_lock::{StorageLock, Time},
		Duration,
	},
	traits::{BlakeTwo256, Hash, StaticLookup, ValidateUnsigned, Zero},
	transaction_validity::{
		InvalidTransaction, TransactionPriority, TransactionSource, TransactionValidity, ValidTransaction,
	},
	DispatchError, DispatchResult, FixedPointNumber, RandomNumberGenerator, RuntimeDebug,
};
use sp_std::{cmp::Ordering, prelude::*, result, str};

pub const OFFCHAIN_WORKER_DATA: &[u8] = b"pallets/risk-manager/data/";
pub const OFFCHAIN_WORKER_LOCK: &[u8] = b"pallets/risk-manager/lock/";
pub const OFFCHAIN_WORKER_MAX_ITERATIONS: &[u8] = b"pallets/risk-manager/max-iterations/";

pub const LOCK_DURATION: u64 = 100;
pub const DEFAULT_MAX_ITERATIONS: u32 = 1000;

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

type LiquidityPools<T> = liquidity_pools::Module<T>;
type Controller<T> = controller::Module<T>;
type MinterestProtocol<T> = minterest_protocol::Module<T>;

#[frame_support::pallet]
pub mod module {
	use super::*;

	#[pallet::config]
	pub trait Config: frame_system::Config + minterest_protocol::Config + SendTransactionTypes<Call<Self>> {
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

		/// A configuration for base priority of unsigned transactions.
		///
		/// This is exposed so that it can be tuned for particular runtime, when
		/// multiple pallets send unsigned transactions.
		type UnsignedPriority: Get<TransactionPriority>;

		/// The basic liquidity pools.
		type LiquidationPoolsManager: PoolsManager<Self::AccountId>;

		/// Pools are responsible for holding funds for automatic liquidation.
		type LiquidityPoolsManager: PoolsManager<Self::AccountId>;

		/// The origin which may update risk manager parameters. Root can
		/// always do this.
		type RiskManagerUpdateOrigin: EnsureOrigin<Self::Origin>;

		type RiskManagerWeightInfo: WeightInfo;
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
	}

	/// Liquidation params for pools: `(max_attempts, min_partial_liquidation_sum, threshold,
	/// liquidation_fee)`.
	#[pallet::storage]
	#[pallet::getter(fn risk_manager_dates)]
	pub(crate) type RiskManagerParams<T: Config> = StorageMap<_, Twox64Concat, CurrencyId, RiskManagerData, ValueQuery>;

	#[pallet::genesis_config]
	pub struct GenesisConfig {
		pub risk_manager_dates: Vec<(CurrencyId, RiskManagerData)>,
	}

	#[cfg(feature = "std")]
	impl Default for GenesisConfig {
		fn default() -> Self {
			GenesisConfig {
				risk_manager_dates: vec![],
			}
		}
	}

	#[pallet::genesis_build]
	impl<T: Config> GenesisBuild<T> for GenesisConfig {
		fn build(&self) {
			self.risk_manager_dates
				.iter()
				.for_each(|(currency_id, risk_manager_data)| {
					RiskManagerParams::<T>::insert(currency_id, RiskManagerData { ..*risk_manager_data })
				});
		}
	}

	#[cfg(feature = "std")]
	impl GenesisConfig {
		/// Direct implementation of `GenesisBuild::build_storage`.
		///
		/// Kept in order not to break dependency.
		pub fn build_storage<T: Config>(&self) -> Result<sp_runtime::Storage, String> {
			<Self as frame_support::traits::GenesisBuild<T>>::build_storage(self)
		}

		/// Direct implementation of `GenesisBuild::assimilate_storage`.
		///
		/// Kept in order not to break dependency.
		pub fn assimilate_storage<T: Config>(&self, storage: &mut sp_runtime::Storage) -> Result<(), String> {
			<Self as frame_support::traits::GenesisBuild<T>>::assimilate_storage(self, storage)
		}
	}

	#[pallet::pallet]
	pub struct Pallet<T>(PhantomData<T>);

	#[pallet::hooks]
	impl<T: Config> Hooks<T::BlockNumber> for Pallet<T> {
		/// Runs after every block. Start offchain worker to check unsafe loan and
		/// submit unsigned tx to trigger liquidation.
		fn offchain_worker(now: T::BlockNumber) {
			debug::info!("Entering off-chain worker");

			if let Err(e) = Self::_offchain_worker() {
				debug::info!(
					target: "RiskManager offchain worker",
					"cannot run offchain worker at {:?}: {:?}",
					now,
					e,
				);
			} else {
				debug::debug!(
					target: "RiskManager offchain worker",
					" RiskManager offchain worker start at block: {:?} already done!",
					now,
				);
			}
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
			T::UpdateOrigin::ensure_origin(origin)?;

			ensure!(
				<LiquidityPools<T>>::is_enabled_underlying_asset_id(pool_id),
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
			T::UpdateOrigin::ensure_origin(origin)?;

			ensure!(
				<LiquidityPools<T>>::is_enabled_underlying_asset_id(pool_id),
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
			T::UpdateOrigin::ensure_origin(origin)?;

			ensure!(
				<LiquidityPools<T>>::is_enabled_underlying_asset_id(pool_id),
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
			T::UpdateOrigin::ensure_origin(origin)?;

			ensure!(
				<LiquidityPools<T>>::is_enabled_underlying_asset_id(pool_id),
				Error::<T>::NotValidUnderlyingAssetId
			);

			// Check if 1 <= liquidation_fee <= 1.5
			ensure!(
				(liquidation_fee >= Rate::one() && liquidation_fee <= Rate::saturating_from_rational(15, 10)),
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
			let who = T::Lookup::lookup(who)?;
			Self::liquidate_unsafe_loan(who, pool_id)?;
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

		// Check if we are a potential validator
		if !sp_io::offchain::is_validator() {
			return Err(OffchainErr::NotValidator);
		}

		// acquire offchain worker lock
		let lock_expiration = Duration::from_millis(LOCK_DURATION);
		let mut lock = StorageLock::<'_, Time>::with_deadline(&OFFCHAIN_WORKER_LOCK, lock_expiration);
		let mut guard = lock.try_lock().map_err(|_| OffchainErr::OffchainLock)?;

		let to_be_continue = StorageValueRef::persistent(&OFFCHAIN_WORKER_DATA);

		// Get to_be_continue record
		let (collateral_position, start_key) =
			if let Some(Some((last_collateral_position, maybe_last_iterator_previous_key))) =
				to_be_continue.get::<(u32, Option<Vec<u8>>)>()
			{
				(last_collateral_position, maybe_last_iterator_previous_key)
			} else {
				let random_seed = sp_io::offchain::random_seed();
				let mut rng = RandomNumberGenerator::<BlakeTwo256>::new(BlakeTwo256::hash(&random_seed[..]));
				(rng.pick_u32(underlying_asset_ids.len().saturating_sub(1) as u32), None)
			};

		// Get the max iterations config
		let max_iterations = StorageValueRef::persistent(&OFFCHAIN_WORKER_MAX_ITERATIONS)
			.get::<u32>()
			.unwrap_or(Some(DEFAULT_MAX_ITERATIONS));

		let currency_id = underlying_asset_ids[(collateral_position as usize)];

		// Get list of users that have an active loan for current pool
		let pool_members =
			<LiquidityPools<T>>::get_pool_members_with_loans(currency_id).map_err(|_| OffchainErr::CheckFail)?;

		let mut iteration_count = 0;
		let iteration_start_time = sp_io::offchain::timestamp();

		for member in pool_members.into_iter() {
			<Controller<T>>::accrue_interest_rate(currency_id).map_err(|_| OffchainErr::CheckFail)?;
			// Checks if the liquidation should be allowed to occur.
			let (_, shortfall) = <Controller<T>>::get_hypothetical_account_liquidity(&member, currency_id, 0, 0)
				.map_err(|_| OffchainErr::CheckFail)?;

			match shortfall.cmp(&Balance::zero()) {
				Ordering::Equal => continue,
				_ => Self::submit_unsigned_liquidation(member, currency_id),
			}

			iteration_count += 1;

			// extend offchain worker lock
			guard.extend_lock().map_err(|_| OffchainErr::OffchainLock)?;
		}

		let iteration_end_time = sp_io::offchain::timestamp();
		debug::info!(
			target: "RiskManager offchain worker",
			"iteration info:\n max iterations is {:?}\n currency id: {:?}, start key: {:?}, iterate count: {:?}\n iteration start at: {:?}, end at: {:?}, execution time: {:?}\n",
			max_iterations,
			currency_id,
			start_key,
			iteration_count,
			iteration_start_time,
			iteration_end_time,
			iteration_end_time.diff(&iteration_start_time)
		);

		// Consume the guard but **do not** unlock the underlying lock.
		guard.forget();

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
			debug::info!(
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
		<Controller<T>>::accrue_interest_rate(liquidated_pool_id)?;

		// Read prices price for borrowed pool.
		let price_borrowed =
			T::PriceSource::get_underlying_price(liquidated_pool_id).ok_or(Error::<T>::InvalidFeedPrice)?;

		// Get borrower borrow balance and calculate total_repay_amount (in USD):
		// total_repay_amount = borrow_balance * price_borrowed
		let borrow_balance = <Controller<T>>::borrow_balance_stored(&borrower, liquidated_pool_id)?;
		let total_repay_amount = Rate::from_inner(borrow_balance)
			.checked_mul(&price_borrowed)
			.map(|x| x.into_inner())
			.ok_or(Error::<T>::NumOverflow)?;

		let liquidation_attempts = <LiquidityPools<T>>::get_user_liquidation_attempts(&borrower, liquidated_pool_id);

		let is_partial_liquidation = total_repay_amount
			>= RiskManagerParams::<T>::get(liquidated_pool_id).min_partial_liquidation_sum
			&& liquidation_attempts < RiskManagerParams::<T>::get(liquidated_pool_id).max_attempts;

		// Calculate sum required to liquidate.
		let (seize_amount, repay_amount, repay_assets, is_need_mutate_attempts) =
			Self::liquidate_calculate_seize_and_repay(liquidated_pool_id, total_repay_amount, is_partial_liquidation)?;

		let seized_pools = Self::liquidate_borrow_fresh(&borrower, liquidated_pool_id, repay_assets, seize_amount)?;
		if is_need_mutate_attempts {
			Self::mutate_liquidation_attempts(liquidated_pool_id, &borrower, is_partial_liquidation);
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
		repay_assets: Balance,
		mut seize_amount: Balance,
	) -> result::Result<Vec<CurrencyId>, DispatchError> {
		let liquidation_pool_account_id = T::LiquidationPoolsManager::pools_account_id();
		let liquidity_pool_account_id = <T as Config>::LiquidityPoolsManager::pools_account_id();

		<MinterestProtocol<T>>::do_repay_fresh(
			&liquidation_pool_account_id,
			&borrower,
			liquidated_pool_id,
			repay_assets,
			false,
		)?;

		// Get an array of collateral pools for the borrower.
		// The array is sorted in descending order by the number of wrapped tokens in USD.
		let collateral_pools = <LiquidityPools<T>>::get_is_collateral_pools(&borrower)?;

		// Collect seized pools.
		let mut seized_pools: Vec<CurrencyId> = Vec::new();

		for collateral_pool_id in collateral_pools.into_iter() {
			if !seize_amount.is_zero() {
				<Controller<T>>::accrue_interest_rate(collateral_pool_id)?;

				let wrapped_id = <LiquidityPools<T>>::get_wrapped_id_by_underlying_asset_id(&collateral_pool_id)?;
				let balance_wrapped_token = T::MultiCurrency::free_balance(wrapped_id, &borrower);

				// Get the exchange rate, read price for collateral pool and calculate the number
				// of collateral tokens to seize:
				// seize_tokens = seize_amount / (price_collateral * exchange_rate)
				let price_collateral =
					T::PriceSource::get_underlying_price(collateral_pool_id).ok_or(Error::<T>::InvalidFeedPrice)?;
				let exchange_rate = <LiquidityPools<T>>::get_exchange_rate(collateral_pool_id)?;
				let seize_tokens = Rate::from_inner(seize_amount)
					.checked_div(
						&price_collateral
							.checked_mul(&exchange_rate)
							.ok_or(Error::<T>::NumOverflow)?,
					)
					.map(|x| x.into_inner())
					.ok_or(Error::<T>::NumOverflow)?;

				// Check if there are enough collateral wrapped tokens to withdraw seize_tokens.
				match balance_wrapped_token.cmp(&seize_tokens) {
					// Not enough collateral wrapped tokens.
					Ordering::Less => {
						// seize_underlying = balance_wrapped_token * exchange_rate
						let seize_underlying =
							<LiquidityPools<T>>::convert_from_wrapped(wrapped_id, balance_wrapped_token)?;

						T::MultiCurrency::withdraw(wrapped_id, &borrower, balance_wrapped_token)?;

						T::MultiCurrency::transfer(
							collateral_pool_id,
							&liquidity_pool_account_id,
							&liquidation_pool_account_id,
							seize_underlying,
						)?;

						// seize_amount = seize_amount - (seize_underlying * price_collateral)
						seize_amount -= Rate::from_inner(seize_underlying)
							.checked_mul(&price_collateral)
							.map(|x| x.into_inner())
							.ok_or(Error::<T>::NumOverflow)?;
					}
					// Enough collateral wrapped tokens. Transfer all seize_tokens to liquidation_pool.
					_ => {
						// seize_underlying = seize_tokens * exchange_rate
						let seize_underlying = <LiquidityPools<T>>::convert_from_wrapped(wrapped_id, seize_tokens)?;

						T::MultiCurrency::withdraw(wrapped_id, &borrower, seize_tokens)?;

						T::MultiCurrency::transfer(
							collateral_pool_id,
							&liquidity_pool_account_id,
							&liquidation_pool_account_id,
							seize_underlying,
						)?;
						// seize_amount = 0, since all seize_tokens have already been withdrawn
						seize_amount = Balance::zero();
					}
				}
				// Collecting seized pools to display in an Event.
				seized_pools.push(collateral_pool_id);
			}
		}

		ensure!(seize_amount == Balance::zero(), Error::<T>::LiquidationRejection);

		Ok(seized_pools)
	}

	// FIXME: Temporary implementation.
	/// Calculate sum required to liquidate for partial and complete liquidation.
	///
	/// - `liquidated_pool_id`: the CurrencyId of the pool with loan, for which automatic
	/// liquidation is performed.
	/// - `total_repay_amount`: total amount of debt converted into usd.
	/// - `is_partial_liquidation`: partial or complete liquidation.
	///
	/// Returns (`seize_amount`, `repay_amount`, `repay_assets`)
	/// - `seize_amount`: the number of collateral tokens to seize converted
	/// into USD (consider liquidation_fee).
	/// - `repay_amount`: current amount of debt converted into usd.
	/// - `repay_assets`: the amount of the underlying borrowed asset to repay.
	pub fn liquidate_calculate_seize_and_repay(
		liquidated_pool_id: CurrencyId,
		total_repay_amount: Balance,
		is_partial_liquidation: bool,
	) -> result::Result<(Balance, Balance, Balance, bool), DispatchError> {
		let liquidation_fee = Self::risk_manager_dates(liquidated_pool_id).liquidation_fee;
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
		let is_need_mutate_attempts = liquidation_pool_balance_usd >= repay_amount;

		// repay_amount = min(amount_to_liquidate, liquidation_pool_balance_usd)
		repay_amount = repay_amount.min(liquidation_pool_balance_usd);

		// seize_amount = liquidation_fee * repay_amount
		let seize_amount = Rate::from_inner(repay_amount)
			.checked_mul(&liquidation_fee)
			.map(|x| x.into_inner())
			.ok_or(Error::<T>::NumOverflow)?;

		let price_borrowed =
			T::PriceSource::get_underlying_price(liquidated_pool_id).ok_or(Error::<T>::InvalidFeedPrice)?;

		// repay_assets = repay_amount / price_borrowed (Tokens)
		let repay_assets = Rate::from_inner(repay_amount)
			.checked_div(&price_borrowed)
			.map(|x| x.into_inner())
			.ok_or(Error::<T>::NumOverflow)?;

		Ok((seize_amount, repay_amount, repay_assets, is_need_mutate_attempts))
	}

	/// Changes the parameter liquidation_attempts depending on the type of liquidation.
	///
	/// - `liquidated_pool_id`: the CurrencyId of the pool with loan, for which automatic.
	/// - `borrower`: the borrower in automatic liquidation.
	/// - `is_partial_liquidation`: partial or complete liquidation.
	fn mutate_liquidation_attempts(
		liquidated_pool_id: CurrencyId,
		borrower: &T::AccountId,
		is_partial_liquidation: bool,
	) {
		// partial_liquidation -> liquidation_attempts += 1
		// complete_liquidation -> liquidation_attempts = 0
		liquidity_pools::PoolUserParams::<T>::mutate(liquidated_pool_id, &borrower, |p| {
			if is_partial_liquidation {
				p.liquidation_attempts += u8::one();
			} else {
				p.liquidation_attempts = u8::zero();
			}
		})
	}
}

impl<T: Config> ValidateUnsigned for Pallet<T> {
	type Call = Call<T>;

	fn validate_unsigned(_source: TransactionSource, call: &Self::Call) -> TransactionValidity {
		match call {
			Call::liquidate(who, pool_id) => ValidTransaction::with_tag_prefix("RiskManagerOffchainWorker")
				.priority(T::UnsignedPriority::get())
				.and_provides((<frame_system::Module<T>>::block_number(), pool_id, who))
				.longevity(64_u64)
				.propagate(true)
				.build(),
			_ => InvalidTransaction::Call.into(),
		}
	}
}
