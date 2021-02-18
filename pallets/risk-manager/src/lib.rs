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
	DispatchResult, FixedPointNumber, RandomNumberGenerator, RuntimeDebug,
};
use sp_std::{cmp::Ordering, prelude::*, str};

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
type MinterestProtocol<T> = minterest_protocol::Module<T>;
type Oracle<T> = oracle::Module<T>;

pub trait Trait:
	frame_system::Trait
	+ minterest_protocol::Trait
	+ liquidity_pools::Trait
	+ controller::Trait
	+ SendTransactionTypes<Call<Self>>
{
	type Event: From<Event<Self>> + Into<<Self as frame_system::Trait>::Event>;

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
	trait Store for Module<T: Trait> as RiskManagerStorage {
		/// Liquidation params for pools: `(max_attempts, min_sum, threshold, liquidation_fee)`.
		pub RiskManagerDates get(fn risk_manager_dates) config(): map hasher(blake2_128_concat) CurrencyId => RiskManagerData;
	}
}

decl_event!(
	pub enum Event<T>
	 where
		 <T as frame_system::Trait>::AccountId,
	 {
		/// Max value of liquidation attempts has been successfully changed: \[who, attempts_amount\]
		MaxValueOFLiquidationAttempsHasChanged(AccountId, u8),

		/// Min sum for partial liquidation has been successfully changed: \[who, min_sum\]
		MinSumForPartialLiquidationHasChanged(AccountId, Balance),

		/// Threshold has been successfully changed: \[who, threshold\]
		ValueOfThresholdHasChanged(AccountId, Rate),

		/// Liquidation fee has been successfully changed: \[ who, threshold\]
		ValueOfLiquidationFeeHasChanged(AccountId, Rate),

		/// Unsafe loan has been successfully liquidated: \[who, liquidate_amount_in_usd, liquidated_pool_id, partial_liquidation\]
		LiquidateUnsafeLoan(AccountId, Balance, CurrencyId, bool),
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

	/// The liquidation hasn't been completed.
	LiquidationRejection,
	}
}

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
	/// - `pool_id`: the CurrencyId of the pool with loan, for which automatic liquidation is performed.
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

	/// Defines the type of liquidation (partial or full).
	///
	/// - `borrower`: the borrower in automatic liquidation.
	/// - `pool_id`: the CurrencyId of the pool with loan, for which automatic liquidation is performed.
	pub fn liquidate_unsafe_loan(borrower: T::AccountId, liquidated_pool_id: CurrencyId) -> DispatchResult {
		<Controller<T>>::accrue_interest_rate(liquidated_pool_id)?;

		// Read oracle price for borrowed pool.
		let price_borrowed = <Oracle<T>>::get_underlying_price(liquidated_pool_id)?;

		// Get borrower borrow balance and calculate seize_amount:
		// seize_amount = borrow_balance * price_borrowed
		let borrow_balance = <Controller<T>>::borrow_balance_stored(&borrower, liquidated_pool_id)?;
		let seize_amount = Rate::from_inner(borrow_balance)
			.checked_mul(&price_borrowed)
			.map(|x| x.into_inner())
			.ok_or(Error::<T>::NumOverflow)?;

		let liquidation_attempts = <LiquidityPools<T>>::get_user_liquidation_attempts(&borrower, liquidated_pool_id);
		let mut is_partial_liquidation: bool = false;

		if seize_amount >= RiskManagerDates::get(liquidated_pool_id).min_sum
			&& liquidation_attempts < RiskManagerDates::get(liquidated_pool_id).max_attempts
		{
			// Partial liquidation.
			let repay_amount = <Controller<T>>::get_sum_required_to_liquidate(seize_amount)?;
			let seize_amount = Rate::from_inner(repay_amount)
				.checked_mul(&price_borrowed)
				.map(|x| x.into_inner())
				.ok_or(Error::<T>::NumOverflow)?;

			Self::liquidate_borrow_fresh(&borrower, liquidated_pool_id, repay_amount, seize_amount)?;
			is_partial_liquidation = true;

			// Increase the number of attempts by 1
			liquidity_pools::PoolUserDates::<T>::try_mutate(liquidated_pool_id, &borrower, |p| -> DispatchResult {
				p.liquidation_attempts = p
					.liquidation_attempts
					.checked_add(1_u8)
					.ok_or(Error::<T>::NumOverflow)?;
				Ok(())
			})?;
		} else {
			// Full liquidation
			Self::liquidate_borrow_fresh(&borrower, liquidated_pool_id, borrow_balance, seize_amount)?;

			// Set the number of attempts to 0
			liquidity_pools::PoolUserDates::<T>::try_mutate(liquidated_pool_id, &borrower, |p| -> DispatchResult {
				p.liquidation_attempts = 0_u8;
				Ok(())
			})?;
		}

		Self::deposit_event(RawEvent::LiquidateUnsafeLoan(
			borrower,
			seize_amount,
			liquidated_pool_id,
			is_partial_liquidation,
		));

		Ok(())
	}

	/// The liquidation pool liquidates the borrowers collateral. The collateral seized is
	/// transferred to the liquidation pool.
	pub fn liquidate_borrow_fresh(
		borrower: &T::AccountId,
		liquidated_pool_id: CurrencyId,
		repay_amount: Balance,
		mut seize_amount: Balance,
	) -> DispatchResult {
		let liquidation_pool_account_id = T::LiquidationPoolsManager::pools_account_id();

		<MinterestProtocol<T>>::do_repay_fresh(
			&liquidation_pool_account_id,
			&borrower,
			liquidated_pool_id,
			repay_amount,
			false,
		)?;

		// Get an array of collateral pools for the borrower.
		// The array is sorted in descending order by the number of wrapped tokens in USD.
		let pools = <LiquidityPools<T>>::get_pools_are_collateral(&borrower)?;

		for collateral_pool_id in pools.into_iter() {
			<Controller<T>>::accrue_interest_rate(collateral_pool_id)?;

			if seize_amount.is_zero() {
				break;
			}

			let wrapped_id = <LiquidityPools<T>>::get_wrapped_id_by_underlying_asset_id(&collateral_pool_id)?;
			let balance_wrapped_token = T::MultiCurrency::free_balance(wrapped_id, &borrower);

			// Get the exchange rate, read oracle price for collateral pool and calculate the number
			// of collateral tokens to seize:
			// seize_tokens = seize_amount / (price_collateral * exchange_rate)
			let price_collateral = <Oracle<T>>::get_underlying_price(collateral_pool_id)?;
			let exchange_rate = <LiquidityPools<T>>::get_exchange_rate(collateral_pool_id)?;
			let seize_tokens = Rate::from_inner(seize_amount)
				.checked_div(
					&price_collateral
						.checked_mul(&exchange_rate)
						.ok_or(Error::<T>::NumOverflow)?,
				)
				.map(|x| x.into_inner())
				.ok_or(Error::<T>::NumOverflow)?;

			match balance_wrapped_token.cmp(&seize_tokens) {
				Ordering::Less => {
					T::MultiCurrency::transfer(
						wrapped_id,
						&borrower,
						&liquidation_pool_account_id,
						balance_wrapped_token,
					)?;
					seize_amount -= Rate::from_inner(seize_tokens)
						.checked_mul(
							&price_collateral
								.checked_mul(&exchange_rate)
								.ok_or(Error::<T>::NumOverflow)?,
						)
						.map(|x| x.into_inner())
						.ok_or(Error::<T>::NumOverflow)?;
				}
				_ => {
					T::MultiCurrency::transfer(wrapped_id, &borrower, &liquidation_pool_account_id, seize_tokens)?;
					seize_amount = Balance::zero();
				}
			}
		}

		ensure!(seize_amount == Balance::zero(), Error::<T>::LiquidationRejection);

		Ok(())
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
