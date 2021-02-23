#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode};
use frame_support::{debug, decl_error, decl_event, decl_module, decl_storage, ensure, traits::Get};
use frame_system::{
	ensure_none, ensure_signed,
	offchain::{SendTransactionTypes, SubmitTransaction},
};
use minterest_primitives::{Balance, CurrencyId, Rate};
use orml_traits::MultiCurrency;
use orml_utilities::with_transaction_result;
use pallet_traits::PoolsManager;
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};
use sp_runtime::traits::{CheckedDiv, CheckedMul};
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

pub const OFFCHAIN_WORKER_DATA: &[u8] = b"modules/risk-manager/data/";
pub const OFFCHAIN_WORKER_LOCK: &[u8] = b"modules/risk-manager/lock/";
pub const OFFCHAIN_WORKER_MAX_ITERATIONS: &[u8] = b"modules/risk-manager/max-iterations/";

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

pub trait Config:
	frame_system::Config + liquidity_pools::Config + controller::Config + SendTransactionTypes<Call<Self>>
{
	type Event: From<Event<Self>> + Into<<Self as frame_system::Config>::Event>;

	/// A configuration for base priority of unsigned transactions.
	///
	/// This is exposed so that it can be tuned for particular runtime, when
	/// multiple modules send unsigned transactions.
	type UnsignedPriority: Get<TransactionPriority>;

	/// The basic liquidity pools.
	type LiquidationPoolsManager: PoolsManager<Self::AccountId>;

	/// Pools are responsible for holding funds for automatic liquidation.
	type LiquidityPoolsManager: PoolsManager<Self::AccountId>;
}

decl_storage! {
	trait Store for Module<T: Config> as RiskManagerStorage {
		/// Liquidation params for pools: `(max_attempts, min_sum, threshold, liquidation_fee)`.
		pub RiskManagerDates get(fn risk_manager_dates) config(): map hasher(blake2_128_concat) CurrencyId => RiskManagerData;
	}
}

decl_event!(
	pub enum Event<T>
	 where
		 <T as frame_system::Config>::AccountId,
	 {
		/// Max value of liquidation attempts has been successfully changed: \[who, attempts_amount\]
		MaxValueOFLiquidationAttempsHasChanged(AccountId, u8),

		/// Min sum for partial liquidation has been successfully changed: \[who, min_sum\]
		MinSumForPartialLiquidationHasChanged(AccountId, Balance),

		/// Threshold has been successfully changed: \[who, threshold\]
		ValueOfThresholdHasChanged(AccountId, Rate),

		/// Liquidation fee has been successfully changed: \[ who, threshold\]
		ValueOfLiquidationFeeHasChanged(AccountId, Rate),

		/// Unsafe loan has been successfully liquidated: \[who, liquidate_amount_in_usd, liquidated_pool_id, collateral_pools, partial_liquidation\]
		LiquidateUnsafeLoan(AccountId, Balance, CurrencyId, Vec<CurrencyId>, bool),
	}
);

decl_error! {
	pub enum Error for Module<T: Config> {
	/// Number overflow in calculation.
	NumOverflow,

	/// The currency is not enabled in protocol.
	NotValidUnderlyingAssetId,

	/// The dispatch origin of this call must be Administrator.
	RequireAdmin,

	/// The liquidation hasn't been completed.
	LiquidationRejection,
	}
}

decl_module! {
	pub struct Module<T: Config> for enum Call where origin: T::Origin {
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

			Self::deposit_event(RawEvent::MaxValueOFLiquidationAttempsHasChanged(sender, new_max_value));

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

			Self::deposit_event(RawEvent::MinSumForPartialLiquidationHasChanged(sender, new_min_sum));

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

			Self::deposit_event(RawEvent::ValueOfThresholdHasChanged(sender, new_threshold));

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

			Self::deposit_event(RawEvent::ValueOfLiquidationFeeHasChanged(sender, new_liquidation_fee));

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
		) {
			with_transaction_result(|| {
				ensure_none(origin)?;
				let who = T::Lookup::lookup(who)?;
				Self::liquidate_unsafe_loan(who, pool_id)?;
				Ok(())
			})?;
		}
	}
}

impl<T: Config> Module<T> {
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
		let (total_borrow_in_usd, total_borrow_in_underlying, oracle_price, liquidation_attempts) =
			Self::get_user_borrow_information(&who, pool_id)?;

		if total_borrow_in_usd >= RiskManagerDates::get(pool_id).min_sum
			&& liquidation_attempts < RiskManagerDates::get(pool_id).max_attempts
		{
			Self::partial_liquidation(
				who,
				pool_id,
				total_borrow_in_usd,
				total_borrow_in_underlying,
				oracle_price,
				liquidation_attempts,
			)?
		} else {
			Self::complete_liquidation(
				who,
				pool_id,
				total_borrow_in_usd,
				total_borrow_in_underlying,
				oracle_price,
				liquidation_attempts,
			)?
		}

		Ok(())
	}

	/// Partial liquidation of loan for user in a particular pool.
	pub fn partial_liquidation(
		who: T::AccountId,
		liquidated_pool_id: CurrencyId,
		total_borrow_in_usd: Balance,
		mut user_total_borrow_in_underlying: Balance,
		liquidated_asset_oracle_price: Rate,
		liquidation_attempts: u8,
	) -> DispatchResult {
		let sum_required_to_liquidate_in_usd = <Controller<T>>::get_sum_required_to_liquidate(total_borrow_in_usd)?;

		let mut underlying_amount_required_to_write_off_debt =
			Self::div_balance_by_rate(&sum_required_to_liquidate_in_usd, &liquidated_asset_oracle_price)?;

		let mut sum_required_to_liquidate_in_usd_plus_fee = Self::mul_balance_by_rate(
			&sum_required_to_liquidate_in_usd,
			&RiskManagerDates::get(liquidated_pool_id).liquidation_fee,
		)?;

		// Collect pools used as collateral.
		let mut collateral_pools: Vec<CurrencyId> = Vec::new();

		let pools = <LiquidityPools<T>>::get_pools_are_collateral(&who)?;

		for pool in pools.into_iter() {
			if sum_required_to_liquidate_in_usd_plus_fee.is_zero() {
				break;
			}

			let pool_n_oracle_price = <Oracle<T>>::get_underlying_price(pool)?;

			let underlying_amount_required_to_liquidate =
				Self::div_balance_by_rate(&sum_required_to_liquidate_in_usd_plus_fee, &pool_n_oracle_price)?;

			let wrapped_amount_required_to_liquidate =
				<LiquidityPools<T>>::convert_to_wrapped(pool, underlying_amount_required_to_liquidate)?;

			// User's params
			let wrapped_id = <LiquidityPools<T>>::get_wrapped_id_by_underlying_asset_id(&pool)?;

			let free_balance_wrapped_token = T::MultiCurrency::free_balance(wrapped_id, &who);

			match free_balance_wrapped_token.cmp(&wrapped_amount_required_to_liquidate) {
				Ordering::Less => {
					let free_balance_underlying_asset =
						<LiquidityPools<T>>::convert_from_wrapped(wrapped_id, free_balance_wrapped_token)?;
					let user_free_balance_in_usd =
						Self::mul_balance_by_rate(&free_balance_underlying_asset, &pool_n_oracle_price)?;
					let available_amount_liquidated_asset =
						Self::div_balance_by_rate(&user_free_balance_in_usd, &liquidated_asset_oracle_price)?;
					let new_pool_total_borrowed = Self::sub_a_from_b_u128(
						&<LiquidityPools<T>>::get_pool_total_borrowed(liquidated_pool_id),
						&available_amount_liquidated_asset,
					)?;
					user_total_borrow_in_underlying =
						Self::sub_a_from_b_u128(&user_total_borrow_in_underlying, &available_amount_liquidated_asset)?;
					let user_borrow_index = <LiquidityPools<T>>::get_pool_borrow_index(liquidated_pool_id);

					T::MultiCurrency::withdraw(wrapped_id, &who, free_balance_wrapped_token)?;
					T::MultiCurrency::transfer(
						pool,
						&<T as Config>::LiquidityPoolsManager::pools_account_id(),
						&T::LiquidationPoolsManager::pools_account_id(),
						free_balance_underlying_asset,
					)?;
					T::MultiCurrency::transfer(
						liquidated_pool_id,
						&T::LiquidationPoolsManager::pools_account_id(),
						&<T as Config>::LiquidityPoolsManager::pools_account_id(),
						available_amount_liquidated_asset,
					)?;

					<LiquidityPools<T>>::set_pool_total_borrowed(liquidated_pool_id, new_pool_total_borrowed)?;
					<LiquidityPools<T>>::set_user_total_borrowed_and_interest_index(
						&who,
						liquidated_pool_id,
						user_total_borrow_in_underlying,
						user_borrow_index,
					)?;

					sum_required_to_liquidate_in_usd_plus_fee =
						Self::sub_a_from_b_u128(&sum_required_to_liquidate_in_usd_plus_fee, &user_free_balance_in_usd)?;
					underlying_amount_required_to_write_off_debt = Self::sub_a_from_b_u128(
						&underlying_amount_required_to_write_off_debt,
						&available_amount_liquidated_asset,
					)?;
					collateral_pools.push(pool);
				}
				_ => {
					let new_pool_total_borrowed = Self::sub_a_from_b_u128(
						&<LiquidityPools<T>>::get_pool_total_borrowed(liquidated_pool_id),
						&underlying_amount_required_to_write_off_debt,
					)?;
					user_total_borrow_in_underlying = Self::sub_a_from_b_u128(
						&user_total_borrow_in_underlying,
						&underlying_amount_required_to_write_off_debt,
					)?;
					let borrow_index = <LiquidityPools<T>>::get_pool_borrow_index(liquidated_pool_id);

					T::MultiCurrency::withdraw(wrapped_id, &who, wrapped_amount_required_to_liquidate)?;
					T::MultiCurrency::transfer(
						pool,
						&<T as Config>::LiquidityPoolsManager::pools_account_id(),
						&T::LiquidationPoolsManager::pools_account_id(),
						underlying_amount_required_to_liquidate,
					)?;
					T::MultiCurrency::transfer(
						liquidated_pool_id,
						&T::LiquidationPoolsManager::pools_account_id(),
						&<T as Config>::LiquidityPoolsManager::pools_account_id(),
						underlying_amount_required_to_write_off_debt,
					)?;

					<LiquidityPools<T>>::set_pool_total_borrowed(liquidated_pool_id, new_pool_total_borrowed)?;
					<LiquidityPools<T>>::set_user_total_borrowed_and_interest_index(
						&who,
						liquidated_pool_id,
						user_total_borrow_in_underlying,
						borrow_index,
					)?;

					sum_required_to_liquidate_in_usd_plus_fee = Balance::zero();
					collateral_pools.push(pool);
				}
			}
		}

		ensure!(
			sum_required_to_liquidate_in_usd_plus_fee == Balance::zero(),
			Error::<T>::LiquidationRejection
		);

		let new_liquidation_attempts_value = liquidation_attempts.checked_add(1).ok_or(Error::<T>::NumOverflow)?;
		<LiquidityPools<T>>::set_user_liquidation_attempts(&who, liquidated_pool_id, new_liquidation_attempts_value)?;

		Self::deposit_event(RawEvent::LiquidateUnsafeLoan(
			who,
			sum_required_to_liquidate_in_usd,
			liquidated_pool_id,
			collateral_pools,
			true,
		));

		Ok(())
	}

	/// Complete liquidation of loan for user in a particular pool.
	pub fn complete_liquidation(
		who: T::AccountId,
		liquidated_pool_id: CurrencyId,
		total_borrow_in_usd: Balance,
		mut user_total_borrow_in_underlying: Balance,
		liquidated_asset_oracle_price: Rate,
		liquidation_attempts: u8,
	) -> DispatchResult {
		let mut total_borrow_in_usd_plus_fee = Self::mul_balance_by_rate(
			&total_borrow_in_usd,
			&RiskManagerDates::get(liquidated_pool_id).liquidation_fee,
		)?;

		let pools = <LiquidityPools<T>>::get_pools_are_collateral(&who)?;

		// Collect pools used as collateral.
		let mut collateral_pools: Vec<CurrencyId> = Vec::new();

		for pool in pools.into_iter() {
			if total_borrow_in_usd_plus_fee.is_zero() {
				break;
			}

			let pool_n_oracle_price = <Oracle<T>>::get_underlying_price(pool)?;

			let underlying_amount_required_to_liquidate =
				Self::div_balance_by_rate(&total_borrow_in_usd_plus_fee, &pool_n_oracle_price)?;

			let wrapped_amount_required_to_liquidate =
				<LiquidityPools<T>>::convert_to_wrapped(pool, underlying_amount_required_to_liquidate)?;

			// User's params
			let wrapped_id = <LiquidityPools<T>>::get_wrapped_id_by_underlying_asset_id(&pool)?;

			let free_balance_wrapped_token = T::MultiCurrency::free_balance(wrapped_id, &who);

			match free_balance_wrapped_token.cmp(&wrapped_amount_required_to_liquidate) {
				Ordering::Less => {
					let free_balance_underlying_asset =
						<LiquidityPools<T>>::convert_from_wrapped(wrapped_id, free_balance_wrapped_token)?;
					let user_free_balance_in_usd =
						Self::mul_balance_by_rate(&free_balance_underlying_asset, &pool_n_oracle_price)?;
					let available_amount_liquidated_asset =
						Self::div_balance_by_rate(&user_free_balance_in_usd, &liquidated_asset_oracle_price)?;
					let new_pool_total_borrowed = Self::sub_a_from_b_u128(
						&<LiquidityPools<T>>::get_pool_total_borrowed(liquidated_pool_id),
						&available_amount_liquidated_asset,
					)?;
					user_total_borrow_in_underlying =
						Self::sub_a_from_b_u128(&user_total_borrow_in_underlying, &available_amount_liquidated_asset)?;
					let user_borrow_index = <LiquidityPools<T>>::get_pool_borrow_index(liquidated_pool_id);

					T::MultiCurrency::withdraw(wrapped_id, &who, free_balance_wrapped_token)?;
					T::MultiCurrency::transfer(
						pool,
						&<T as Config>::LiquidityPoolsManager::pools_account_id(),
						&T::LiquidationPoolsManager::pools_account_id(),
						free_balance_underlying_asset,
					)?;
					T::MultiCurrency::transfer(
						liquidated_pool_id,
						&T::LiquidationPoolsManager::pools_account_id(),
						&<T as Config>::LiquidityPoolsManager::pools_account_id(),
						available_amount_liquidated_asset,
					)?;

					<LiquidityPools<T>>::set_pool_total_borrowed(liquidated_pool_id, new_pool_total_borrowed)?;
					<LiquidityPools<T>>::set_user_total_borrowed_and_interest_index(
						&who,
						liquidated_pool_id,
						user_total_borrow_in_underlying,
						user_borrow_index,
					)?;

					total_borrow_in_usd_plus_fee =
						Self::sub_a_from_b_u128(&total_borrow_in_usd_plus_fee, &user_free_balance_in_usd)?;
					collateral_pools.push(pool)
				}
				_ => {
					let new_pool_total_borrowed = Self::sub_a_from_b_u128(
						&<LiquidityPools<T>>::get_pool_total_borrowed(liquidated_pool_id),
						&user_total_borrow_in_underlying,
					)?;
					let borrow_index = <LiquidityPools<T>>::get_pool_borrow_index(liquidated_pool_id);

					T::MultiCurrency::withdraw(wrapped_id, &who, wrapped_amount_required_to_liquidate)?;
					T::MultiCurrency::transfer(
						pool,
						&<T as Config>::LiquidityPoolsManager::pools_account_id(),
						&T::LiquidationPoolsManager::pools_account_id(),
						underlying_amount_required_to_liquidate,
					)?;
					T::MultiCurrency::transfer(
						liquidated_pool_id,
						&T::LiquidationPoolsManager::pools_account_id(),
						&<T as Config>::LiquidityPoolsManager::pools_account_id(),
						user_total_borrow_in_underlying,
					)?;

					<LiquidityPools<T>>::set_pool_total_borrowed(liquidated_pool_id, new_pool_total_borrowed)?;
					<LiquidityPools<T>>::set_user_total_borrowed_and_interest_index(
						&who,
						liquidated_pool_id,
						Balance::zero(),
						borrow_index,
					)?;

					total_borrow_in_usd_plus_fee = Balance::zero();
					collateral_pools.push(pool)
				}
			}
		}

		ensure!(
			total_borrow_in_usd_plus_fee == Balance::zero(),
			Error::<T>::LiquidationRejection
		);

		if liquidation_attempts > 0 {
			<LiquidityPools<T>>::set_user_liquidation_attempts(&who, liquidated_pool_id, 0)?;
		}

		Self::deposit_event(RawEvent::LiquidateUnsafeLoan(
			who,
			total_borrow_in_usd,
			liquidated_pool_id,
			collateral_pools,
			false,
		));

		Ok(())
	}

	/// Get user's loan for particular pool in USD/Underlying assets && oracle price for liquidated
	/// pool.
	fn get_user_borrow_information(
		who: &T::AccountId,
		pool_id: CurrencyId,
	) -> result::Result<(Balance, Balance, Rate, u8), DispatchError> {
		let liquidation_attempts = <LiquidityPools<T>>::get_user_liquidation_attempts(&who, pool_id);
		let total_borrow_in_underlying = <Controller<T>>::borrow_balance_stored(&who, pool_id)?;
		let oracle_price = <Oracle<T>>::get_underlying_price(pool_id)?;
		let total_borrow_in_usd = Rate::from_inner(total_borrow_in_underlying)
			.checked_mul(&oracle_price)
			.map(|x| x.into_inner())
			.ok_or(Error::<T>::NumOverflow)?;
		Ok((
			total_borrow_in_usd,
			total_borrow_in_underlying,
			oracle_price,
			liquidation_attempts,
		))
	}

	/// Performs mathematical calculations.
	///
	/// returns `result = balance_scalar * rate_scalar`
	fn mul_balance_by_rate(balance_scalar: &Balance, rate_scalar: &Rate) -> result::Result<Balance, DispatchError> {
		let result = Rate::from_inner(*balance_scalar)
			.checked_mul(rate_scalar)
			.map(|x| x.into_inner())
			.ok_or(Error::<T>::NumOverflow)?;
		Ok(result)
	}

	/// Performs mathematical calculations.
	///
	/// returns `result = balance_scalar / rate_scalar`
	fn div_balance_by_rate(balance: &Balance, rate: &Rate) -> result::Result<Balance, DispatchError> {
		let result = Rate::from_inner(*balance)
			.checked_div(rate)
			.map(|x| x.into_inner())
			.ok_or(Error::<T>::NumOverflow)?;
		Ok(result)
	}

	/// Performs mathematical calculations.
	///
	/// returns `result = b - a`
	fn sub_a_from_b_u128(b: &Balance, a: &Balance) -> result::Result<Balance, DispatchError> {
		let result = b.checked_sub(*a).ok_or(Error::<T>::NumOverflow)?;
		Ok(result)
	}
}

impl<T: Config> ValidateUnsigned for Module<T> {
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
