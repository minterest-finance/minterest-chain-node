#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode};
use frame_support::{
	debug, decl_error, decl_event, decl_module, decl_storage, dispatch::DispatchResultWithPostInfo, ensure, traits::Get,
};
use frame_system::{
	ensure_none, ensure_signed,
	offchain::{SendTransactionTypes, SubmitTransaction},
};
use minterest_primitives::{Balance, CurrencyId, Rate};
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};
use sp_runtime::traits::CheckedMul;
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

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

/// Error which may occur while executing the off-chain code.
#[cfg_attr(test, derive(PartialEq))]
enum OffchainErr {
	OffchainLock,
	NotValidator,
	CheckFail,
}

impl sp_std::fmt::Debug for OffchainErr {
	fn fmt(&self, fmt: &mut sp_std::fmt::Formatter) -> sp_std::fmt::Result {
		match *self {
			OffchainErr::OffchainLock => write!(fmt, "Failed to get or extend lock"),
			OffchainErr::NotValidator => write!(fmt, "Not validator"),
			OffchainErr::CheckFail => write!(fmt, "Check fail"),
		}
	}
}

/// RiskManager metadata
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(Encode, Decode, RuntimeDebug, Eq, PartialEq, Default)]
pub struct RiskManagerData {
	/// The maximum amount of partial liquidation attempts.
	pub max_attempts: u8,

	/// Minimal sum for partial liquidation.
	/// Loan whose amount below this parameter will be liquidate in full.
	pub min_sum: Balance,

	/// Step used in liquidation to protect the user from micro liquidations.
	pub threshold: Rate,

	/// Fee that covers liquidation costs.
	pub liquidation_fee: Rate,
}

type LiquidityPools<T> = liquidity_pools::Module<T>;
type Accounts<T> = accounts::Module<T>;
type Controller<T> = controller::Module<T>;
type Oracle<T> = oracle::Module<T>;

pub trait Trait:
	frame_system::Trait + liquidity_pools::Trait + accounts::Trait + controller::Trait + SendTransactionTypes<Call<Self>>
{
	type Event: From<Event> + Into<<Self as frame_system::Trait>::Event>;

	/// A configuration for base priority of unsigned transactions.
	///
	/// This is exposed so that it can be tuned for particular runtime, when
	/// multiple modules send unsigned transactions.
	type UnsignedPriority: Get<TransactionPriority>;
}

decl_storage! {
	trait Store for Module<T: Trait> as RiskManagerStorage {
		/// Liquidation params for pools: `(max_attempts, min_sum, threshold, liquidation_fee)`.
		pub RiskManagerDates get(fn risk_manager_dates) config(): map hasher(blake2_128_concat) CurrencyId => RiskManagerData;
	}
}

decl_event!(
	pub enum Event {
		/// Max value of liquidation attempts has been successfully changed.
		MaxValueOFLiquidationAttempsHasChanged,

		/// Min sum for partial liquidation has been successfully changed.
		MinSumForPartialLiquidationHasChanged,

		/// Threshold has been successfully changed.
		ValueOfThresholdHasChanged,

		/// Liquidation fee has been successfully changed.
		ValueOfLiquidationFeeHasChanged,
	}
);

decl_error! {
	pub enum Error for Module<T: Trait> {
	/// Number overflow in calculation.
	NumOverflow,

	/// The currency is not enabled in protocol.
	NotValidUnderlyingAssetId,

	/// The dispatch origin of this call must be Administrator.
	RequireAdmin,
	}
}

type BalanceResult = result::Result<Balance, DispatchError>;

decl_module! {
	pub struct Module<T: Trait> for enum Call where origin: T::Origin {
		type Error = Error<T>;

		fn deposit_event() = default;

		/// Set maximum amount of partial liquidation attempts.
		/// - `pool_id`: PoolID for which the parameter value is being set.
		/// - `new_max_value`: New max value of liquidation attempts.
		///
		/// The dispatch origin of this call must be Administrator.
		#[weight = 0]
		pub fn set_max_attempts(origin, pool_id: CurrencyId, new_max_value: u8) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			ensure!(<Accounts<T>>::is_admin_internal(&sender), Error::<T>::RequireAdmin);

			ensure!(
				<LiquidityPools<T>>::is_enabled_underlying_asset_id(pool_id),
				Error::<T>::NotValidUnderlyingAssetId
			);

			// Write new value into storage.
			RiskManagerDates::mutate(pool_id, |r| r.max_attempts = new_max_value);

			Self::deposit_event(Event::MaxValueOFLiquidationAttempsHasChanged);

			Ok(())
		}

		/// Set minimal sum for partial liquidation.
		/// - `pool_id`: PoolID for which the parameter value is being set.
		/// - `new_min_sum`: New min sum for partial liquidation.
		///
		/// The dispatch origin of this call must be Administrator.
		#[weight = 0]
		pub fn set_min_sum(origin, pool_id: CurrencyId, new_min_sum: Balance) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			ensure!(<Accounts<T>>::is_admin_internal(&sender), Error::<T>::RequireAdmin);

			ensure!(
				<LiquidityPools<T>>::is_enabled_underlying_asset_id(pool_id),
				Error::<T>::NotValidUnderlyingAssetId
			);

			// Write new value into storage.
			RiskManagerDates::mutate(pool_id, |r| r.min_sum = new_min_sum);

			Self::deposit_event(Event::MinSumForPartialLiquidationHasChanged);

			Ok(())
		}

		/// Set threshold which used in liquidation to protect the user from micro liquidations..
		/// - `pool_id`: PoolID for which the parameter value is being set.
		/// - `new_threshold_n`: numerator.
		/// - `new_threshold_d`: divider.
		///
		/// `new_threshold = (new_threshold_n / new_threshold_d)`
		/// The dispatch origin of this call must be Administrator.
		#[weight = 0]
		pub fn set_threshold(origin, pool_id: CurrencyId, new_threshold_n: u128, new_threshold_d: u128) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			ensure!(<Accounts<T>>::is_admin_internal(&sender), Error::<T>::RequireAdmin);

			ensure!(
				<LiquidityPools<T>>::is_enabled_underlying_asset_id(pool_id),
				Error::<T>::NotValidUnderlyingAssetId
			);

			let new_threshold = Rate::checked_from_rational(new_threshold_n, new_threshold_d)
				.ok_or(Error::<T>::NumOverflow)?;

			// Write new value into storage.
			RiskManagerDates::mutate(pool_id, |r| r.threshold = new_threshold);

			Self::deposit_event(Event::ValueOfThresholdHasChanged);

			Ok(())
		}

		/// Set Liquidation fee that covers liquidation costs.
		/// - `pool_id`: PoolID for which the parameter value is being set.
		/// - `new_liquidation_fee_n`: numerator.
		/// - `new_liquidation_fee_d`: divider.
		///
		/// `new_liquidation_fee = (new_liquidation_fee_n / new_liquidation_fee_d)`
		/// The dispatch origin of this call must be Administrator.
		#[weight = 0]
		pub fn set_liquidation_fee(origin, pool_id: CurrencyId, new_liquidation_fee_n: u128, new_liquidation_fee_d: u128) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			ensure!(<Accounts<T>>::is_admin_internal(&sender), Error::<T>::RequireAdmin);

			ensure!(
				<LiquidityPools<T>>::is_enabled_underlying_asset_id(pool_id),
				Error::<T>::NotValidUnderlyingAssetId
			);

			let new_liquidation_fee = Rate::checked_from_rational(new_liquidation_fee_n, new_liquidation_fee_d)
				.ok_or(Error::<T>::NumOverflow)?;

			// Write new value into storage.
			RiskManagerDates::mutate(pool_id, |r| r.liquidation_fee = new_liquidation_fee);

			Self::deposit_event(Event::ValueOfLiquidationFeeHasChanged);

			Ok(())
		}

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

		/// Liquidate unsafe loans
		///
		/// The dispatch origin of this call must be _None_.
		///
		/// - `currency_id`: PoolID for which the loan is being liquidate
		/// - `who`: loan's owner.
		#[weight = 0]
		pub fn liquidate(
			origin,
			who: <T::Lookup as StaticLookup>::Source,
			pool_id: CurrencyId
		) -> DispatchResultWithPostInfo {
			ensure_none(origin)?;
			let who = T::Lookup::lookup(who)?;
			Self::liquidate_unsafe_loan(who, pool_id)?;
			Ok(().into())
		}
	}
}

impl<T: Trait> Module<T> {
	fn _offchain_worker() -> Result<(), OffchainErr> {
		// Get available assets list
		let underlying_asset_ids: Vec<CurrencyId> = <T as liquidity_pools::Trait>::EnabledCurrencyPair::get()
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

		// Get the max iterationns config
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
		debug::debug!(
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

	fn submit_unsigned_liquidation(who: T::AccountId, pool_id: CurrencyId) {
		let who = T::Lookup::unlookup(who);
		let call = Call::<T>::liquidate(who.clone(), pool_id);
		if SubmitTransaction::<T, Call<T>>::submit_unsigned_transaction(call.into()).is_err() {
			debug::info!(
				target: "RiskManager offchain worker",
				"submit unsigned liquidation for \n AccountId {:?} CurrencyId {:?} \nfailed!",
				who, pool_id,
			);
		}
	}

	fn liquidate_unsafe_loan(who: T::AccountId, pool_id: CurrencyId) -> DispatchResult {
		let total_borrow_in_pool = Self::get_user_total_borrow_in_usd(&who, pool_id)?;

		match total_borrow_in_pool.cmp(&RiskManagerDates::get(pool_id).min_sum) {
			Ordering::Less => Self::complete_liquidation(&who, pool_id, total_borrow_in_pool)?,
			_ => {}
		}

		Ok(())
	}

	fn complete_liquidation(who: &T::AccountId, pool_id: CurrencyId, total_borrow_in_pool: Balance) -> DispatchResult {
		Ok(())
	}

	fn get_user_total_borrow_in_usd(who: &T::AccountId, pool_id: CurrencyId) -> BalanceResult {
		let total_borrow_in_pool = <LiquidityPools<T>>::get_user_total_borrowed(&who, pool_id);
		let oracle_price = <Oracle<T>>::get_underlying_price(pool_id)?;
		let result = Rate::from_inner(total_borrow_in_pool)
			.checked_mul(&oracle_price)
			.map(|x| x.into_inner())
			.ok_or(Error::<T>::NumOverflow)?;
		Ok(result)
	}
}

impl<T: Trait> ValidateUnsigned for Module<T> {
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
