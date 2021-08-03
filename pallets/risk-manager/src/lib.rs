//! # Risk Manager Pallet
//!
//! ## Overview
//!
//! This pallet provides the liquidation functionality. In the Minterest protocol, liquidation
//! is based on the principle of “all loans - all collaterals”. Liquidation pools act as
//! a liquidator. The protocol provides two types of liquidation: partial and complete.
//! In case of the partial liquidation, the balance is withdrawn from the user's borrows and
//! collaterals in order to make the user's loan to a safe state. In case of complete liquidation,
//! the user's entire borrow is written off, and all the user's collateral is withdrawn.
//!
//! Each block is run off-chain worker that checks the loans of all users for insolvency.
//! An insolvent loan is a loan where the user's total borrow is greater than the user's
//! total collateral. The working time of this OCW is limited. If the worker discovers an
//! insolvent loan, then he starts the liquidation process.
//!
//! ## Interface
//!
//! -`UserLiquidationAttemptsManager`: provides functionality to manage the number of attempts to
//! partially liquidation a user's loan.
//! -`RiskManagerStorageProvider`: creates storage records for risk-manager pallet. This is a part
//! of a pool creation flow.
//!
//! ### Dispatchable Functions (extrinsics)
//!
//! - `set_liquidation_fee` - setter for parameter `liquidation_fee`. The dispatch origin of this
//! call must be 'RiskManagerUpdateOrigin'.
//! - `set_liquidation_threshold` - setter for parameter `liquidation_threshold`. The dispatch
//! origin of this call must be 'RiskManagerUpdateOrigin'.
//! - `liquidate` - Liquidate insolvent loan.  The dispatch origin of this call must be
//! _None_. Called from the OCW.

#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::unused_unit)]
#![allow(clippy::upper_case_acronyms)]
#![allow(clippy::redundant_clone)]
#![allow(clippy::unnecessary_wraps)] // TODO: remove after implementation math functions

use frame_support::{
	sp_runtime::offchain::{
		storage_lock::{StorageLock, Time},
		Duration,
	},
	{log, pallet_prelude::*, transactional},
};
use frame_system::{
	ensure_none,
	offchain::{SendTransactionTypes, SubmitTransaction},
	pallet_prelude::OriginFor,
};
pub use liquidation::*;
use liquidity_pools::PoolData;
use minterest_primitives::{
	currency::CurrencyType::UnderlyingAsset, Balance, CurrencyId, OffchainErr, Operation, Rate,
};
pub use module::*;
use orml_traits::MultiCurrency;
use pallet_traits::{
	ControllerManager, CurrencyConverter, LiquidityPoolStorageProvider, MinterestProtocolManager, PoolsManager,
	PricesManager, RiskManagerStorageProvider, UserCollateral, UserLiquidationAttemptsManager,
};
use sp_runtime::{
	traits::{CheckedAdd, CheckedMul, One, StaticLookup, Zero},
	FixedPointNumber,
};
use sp_std::{fmt::Debug, prelude::*};

pub const OFFCHAIN_WORKER_LOCK: &[u8] = b"pallets/risk-manager/lock/";

mod liquidation;
#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

#[frame_support::pallet]
pub mod module {
	use super::*;
	use orml_traits::MultiCurrency;

	#[pallet::config]
	pub trait Config: frame_system::Config + SendTransactionTypes<Call<Self>> + Debug {
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

		/// A configuration for base priority of unsigned transactions.
		///
		/// This is exposed so that it can be tuned for particular runtime, when
		/// multiple pallets send unsigned transactions.
		type UnsignedPriority: Get<TransactionPriority>;

		/// The price source of currencies
		type PriceSource: PricesManager<CurrencyId>;

		/// Provides functionality for working with a user's collateral pools.
		type UserCollateral: UserCollateral<Self::AccountId>;

		#[pallet::constant]
		/// Minimal sum for partial liquidation.
		/// Loans with amount below this parameter will be liquidate in full.
		type PartialLiquidationMinSum: Get<Balance>;

		#[pallet::constant]
		/// The maximum number of partial liquidations a user has. After reaching this parameter,
		/// a complete liquidation occurs.
		type PartialLiquidationMaxAttempts: Get<u8>;

		#[pallet::constant]
		/// The maximum liquidation fee.
		type MaxLiquidationFee: Get<Rate>;

		/// The origin which may update risk manager parameters. Root or
		/// Half Minterest Council can always do this.
		type RiskManagerUpdateOrigin: EnsureOrigin<Self::Origin>;

		/// Public API of controller pallet
		type ControllerManager: ControllerManager<Self::AccountId>;

		/// Provides the basic liquidity pools functionality.
		type LiquidityPoolsManager: LiquidityPoolStorageProvider<Self::AccountId, PoolData>
			+ CurrencyConverter
			+ UserCollateral<Self::AccountId>
			+ PoolsManager<Self::AccountId>;

		/// Provides the basic liquidation pools functionality.
		type LiquidationPoolsManager: PoolsManager<Self::AccountId>;

		/// Provides the basic minterest protocol functionality.
		type MinterestProtocolManager: MinterestProtocolManager<Self::AccountId>;

		/// Max duration time for offchain worker.
		type OffchainWorkerMaxDurationMs: Get<u64>;

		/// The `MultiCurrency` implementation.
		type MultiCurrency: MultiCurrency<Self::AccountId, Balance = Balance, CurrencyId = CurrencyId>;
	}

	#[pallet::error]
	pub enum Error<T> {
		/// The currency is not enabled in protocol.
		NotValidUnderlyingAssetId,
		/// Liquidation fee can't be greater than 0.5.
		InvalidLiquidationFeeValue,
		/// Risk manager storage (liquidation_fee, liquidation_threshold) is already created.
		RiskManagerParamsAlreadyCreated,
		/// Feed price is invalid
		InvalidFeedPrice,
		/// Number overflow in calculation.
		NumOverflow,
		/// User's loan is solvent.
		SolventUserLoan,
		/// An error occurred while changing the number of user liquidation attempts.
		ErrorChangingLiquidationAttempts,
		/// Error in choosing the liquidation mode.
		ErrorLiquidationMode,
		/// Error during Gaussian elimination or math logic error
		LiquidationMathFailed,
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(crate) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Liquidation fee has been successfully changed: \[pool_id, liquidation_fee\]
		LiquidationFeeUpdated(CurrencyId, Rate),
		/// Liquidation threshold has been successfully changed: \[threshold\]
		LiquidationThresholdUpdated(Rate),
		/// Insolvent loan has been successfully liquidated: \[who, repaid_pools,
		/// seized pools, liquidation_mode\]
		LiquidateUnsafeLoan(
			T::AccountId,
			Vec<(CurrencyId, Balance)>,
			Vec<(CurrencyId, Balance)>,
			LiquidationMode,
		),
	}

	/// The additional collateral which is taken from borrowers as a penalty for being liquidated.
	/// Sets for each liquidity pool separately.
	///
	/// Storage location:
	/// [`MNT Storage`](?search=risk_manager::module::Pallet::liquidation_fee_storage)
	#[doc(alias = "MNT Storage")]
	#[doc(alias = "MNT risk_manager")]
	#[pallet::storage]
	#[pallet::getter(fn liquidation_fee_storage)]
	pub(crate) type LiquidationFeeStorage<T: Config> = StorageMap<_, Twox64Concat, CurrencyId, Rate, ValueQuery>;

	/// Step used in liquidation to protect the user from micro liquidations. One value for
	/// the entire protocol.
	///
	/// Storage location:
	/// [`MNT Storage`](?search=risk_manager::module::Pallet::liquidation_threshold_storage)
	#[doc(alias = "MNT Storage")]
	#[doc(alias = "MNT risk_manager")]
	#[pallet::storage]
	#[pallet::getter(fn liquidation_threshold_storage)]
	pub(crate) type LiquidationThresholdStorage<T: Config> = StorageValue<_, Rate, ValueQuery>;

	/// Counter of the number of partial liquidations at the user.
	///
	/// Storage location:
	/// [`MNT Storage`](?search=risk_manager::module::Pallet::user_liquidation_attempts_storage)
	#[doc(alias = "MNT Storage")]
	#[doc(alias = "MNT risk_manager")]
	#[pallet::storage]
	#[pallet::getter(fn user_liquidation_attempts_storage)]
	pub(crate) type UserLiquidationAttemptsStorage<T: Config> =
		StorageMap<_, Twox64Concat, T::AccountId, u8, ValueQuery>;

	#[pallet::genesis_config]
	pub struct GenesisConfig<T: Config> {
		pub liquidation_fee: Vec<(CurrencyId, Rate)>,
		pub liquidation_threshold: Rate,
		pub _phantom: sp_std::marker::PhantomData<T>,
	}

	#[cfg(feature = "std")]
	impl<T: Config> Default for GenesisConfig<T> {
		fn default() -> Self {
			GenesisConfig {
				liquidation_fee: vec![],
				liquidation_threshold: Rate::default(),
				_phantom: PhantomData,
			}
		}
	}

	#[pallet::genesis_build]
	impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
		fn build(&self) {
			self.liquidation_fee.iter().for_each(|(pool_id, liquidation_fee)| {
				Pallet::<T>::is_valid_liquidation_fee(*liquidation_fee);
				LiquidationFeeStorage::<T>::insert(pool_id, liquidation_fee)
			});
			LiquidationThresholdStorage::<T>::put(self.liquidation_threshold);
		}
	}

	#[pallet::pallet]
	pub struct Pallet<T>(PhantomData<T>);

	#[pallet::hooks]
	impl<T: Config> Hooks<T::BlockNumber> for Pallet<T> {
		/// Runs after every block. Offchain worker checks insolvent loans and
		/// submit unsigned tx to trigger liquidation.
		fn offchain_worker(now: T::BlockNumber) {
			if let Err(e) = Self::_offchain_worker() {
				log::info!(
					target: "RiskManager offchain worker",
					"Error in RiskManager offchain worker at {:?}: {:?}",
					now,
					e,
				);
			} else {
				log::debug!(
					target: "RiskManager offchain worker",
					"RiskManager offchain worker start at block: {:?} already done!",
					now,
				);
			}
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Set liquidation fee that covers liquidation costs.
		///
		/// Parameters:
		/// - `pool_id`: PoolID for which the parameter value is being set.
		/// - `liquidation_fee`: new liquidation fee value.
		///
		/// The dispatch origin of this call must be 'RiskManagerUpdateOrigin'.
		#[doc(alias = "MNT Extrinsic")]
		#[doc(alias = "MNT risk_manager")]
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
			ensure!(
				Self::is_valid_liquidation_fee(liquidation_fee),
				Error::<T>::InvalidLiquidationFeeValue
			);
			LiquidationFeeStorage::<T>::insert(pool_id, liquidation_fee);
			Self::deposit_event(Event::LiquidationFeeUpdated(pool_id, liquidation_fee));
			Ok(().into())
		}

		/// Set threshold which used in liquidation to protect the user from micro liquidations.
		///
		/// Parameters:
		/// - `threshold`: new threshold.
		///
		/// The dispatch origin of this call must be 'RiskManagerUpdateOrigin'.
		#[doc(alias = "MNT Extrinsic")]
		#[doc(alias = "MNT risk_manager")]
		#[pallet::weight(0)]
		#[transactional]
		pub fn set_liquidation_threshold(origin: OriginFor<T>, threshold: Rate) -> DispatchResultWithPostInfo {
			T::RiskManagerUpdateOrigin::ensure_origin(origin)?;
			LiquidationThresholdStorage::<T>::put(threshold);
			Self::deposit_event(Event::LiquidationThresholdUpdated(threshold));
			Ok(().into())
		}

		// TODO: cover with tests
		/// Liquidate insolvent loan. Calls internal functions from minterest-protocol pallet
		/// `do_repay` and `do_seize`, these functions within themselves call
		/// `accrue_interest_rate`. Before calling the extrinsic, it is necessary to perform all
		/// checks and math calculations of the user's borrows and collaterals.
		///
		/// Parameters:
		/// - `borrower`: AccountId of the borrower whose loan is being liquidated.
		/// - `user_loan_state`: contains a vectors with user's borrows to be paid from the
		/// liquidation pools instead of the borrower, and a vector with user's supplies to be
		/// withdrawn from the borrower and sent to the liquidation pools. Balances are calculated
		/// in underlying assets.
		///
		/// The dispatch origin of this call must be _None_.
		#[doc(alias = "MNT Extrinsic")]
		#[doc(alias = "MNT risk_manager")]
		#[pallet::weight(0)]
		#[transactional]
		pub fn liquidate(
			origin: OriginFor<T>,
			borrower: <T::Lookup as StaticLookup>::Source,
			user_loan_state: UserLoanState<T>,
		) -> DispatchResultWithPostInfo {
			ensure_none(origin)?;
			let borrower = T::Lookup::lookup(borrower)?;
			Self::do_liquidate(&borrower, user_loan_state.clone())?;
			Self::deposit_event(Event::LiquidateUnsafeLoan(
				borrower,
				user_loan_state.get_user_borrows_to_repay_underlying(),
				user_loan_state.get_user_supplies_to_seize_underlying(),
				user_loan_state
					.get_user_liquidation_mode()
					.ok_or(Error::<T>::ErrorLiquidationMode)?,
			));
			Ok(().into())
		}
	}

	#[pallet::validate_unsigned]
	impl<T: Config> ValidateUnsigned for Pallet<T> {
		type Call = Call<T>;

		fn validate_unsigned(_source: TransactionSource, call: &Self::Call) -> TransactionValidity {
			match call {
				Call::liquidate(who, _borrower_loan_state) => {
					ValidTransaction::with_tag_prefix("RiskManagerOffchainWorker")
						.priority(T::UnsignedPriority::get())
						.and_provides((<frame_system::Pallet<T>>::block_number(), who))
						.longevity(64_u64)
						.propagate(true)
						.build()
				}
				_ => InvalidTransaction::Call.into(),
			}
		}
	}
}

// Private functions
impl<T: Config> Pallet<T> {
	// TODO: cover with tests
	/// Checks if the node is a validator. The worker is launched every block. The worker's working
	/// time is limited in time. Each next worker starts checking user loans from the beginning.
	/// Calls a processing insolvent loan function.
	fn _offchain_worker() -> Result<(), OffchainErr> {
		// Check if we are a potential validator
		ensure!(sp_io::offchain::is_validator(), OffchainErr::NotValidator);

		// acquire offchain worker lock
		let lock_expiration = Duration::from_millis(T::OffchainWorkerMaxDurationMs::get());
		let mut lock = StorageLock::<'_, Time>::with_deadline(&OFFCHAIN_WORKER_LOCK, lock_expiration);
		let mut guard = lock.try_lock().map_err(|_| OffchainErr::OffchainLock)?;

		let users_with_insolvent_loan = T::ControllerManager::get_all_users_with_insolvent_loan()
			.map_err(|_| OffchainErr::GetUsersWithInsolventLoanFailed)?;

		let mut loans_liquidated_count = 0_u32;
		let working_start_time = sp_io::offchain::timestamp();

		for borrower in users_with_insolvent_loan.iter() {
			Self::process_insolvent_loan(borrower)?;
			loans_liquidated_count += 1;
			// extend offchain worker lock
			guard.extend_lock().map_err(|_| {
				log::info!(
					"Risk Manager offchain worker hasn't(!) processed all insolvent loans, \
						MAX duration time is expired. number of insolvent loans: {:?}, number of liquidated loans: {:?}",
					users_with_insolvent_loan.len(),
					loans_liquidated_count
				);
				OffchainErr::OffchainLock
			})?;
		}

		// ensure that all insolvent loans have been liquidated
		ensure!(
			T::ControllerManager::get_all_users_with_insolvent_loan()
				.map_err(|_| OffchainErr::NotAllLoansLiquidated)?
				.is_empty(),
			OffchainErr::NotAllLoansLiquidated
		);

		let working_time = sp_io::offchain::timestamp().diff(&working_start_time);
		log::info!(
			"Risk Manager offchain worker has processed all loans, \
			number of insolvent loans: {:?}, number of liquidated loans: {:?}, execution time(ms): {:?}",
			users_with_insolvent_loan.len(),
			loans_liquidated_count,
			working_time.millis()
		);

		// Consume the guard but **do not** unlock the underlying lock.
		guard.forget();

		Ok(())
	}

	// TODO: cover with tests
	/// Handles the user's loan. Selects one of the required types of liquidation (Partial,
	/// Complete) and calls extrinsic `liquidate()`. This function within itself call
	/// `accrue_interest_rate`.
	///
	/// -`borrower`: AccountId of the borrower whose loan is being processed.
	fn process_insolvent_loan(borrower: &T::AccountId) -> Result<(), OffchainErr> {
		let user_loan_state: UserLoanState<T> =
			UserLoanState::build_user_loan_state(borrower).map_err(|_| OffchainErr::BuildUserLoanStateFailed)?;

		// call to change the offchain worker local storage
		Self::do_liquidate(&borrower, user_loan_state.clone()).map_err(|_| OffchainErr::LiquidateTransactionFailed)?;
		Self::submit_unsigned_liquidation(&borrower, user_loan_state);
		Ok(())
	}

	/// Submits an unsigned liquidation transaction to the blockchain.
	///
	/// -`borrower`: AccountId of the borrower whose loan is being processed.
	/// - `liquidation_amounts`: contains a vectors with user's borrows to be paid from the
	/// liquidation pools instead of the borrower, and a vector with user's supplies to be
	/// withdrawn from the borrower and sent to the liquidation pools. Balances are calculated
	/// in underlying assets.
	fn submit_unsigned_liquidation(borrower: &T::AccountId, user_loan_state: UserLoanState<T>) {
		let who = T::Lookup::unlookup(borrower.clone());
		let call = Call::<T>::liquidate(who.clone(), user_loan_state);
		if SubmitTransaction::<T, Call<T>>::submit_unsigned_transaction(call.into()).is_err() {
			log::info!(
				target: "RiskManager offchain worker",
				"submit unsigned liquidation for \n AccountId {:?} \nfailed!",
				borrower,
			);
		}
	}

	/// Calls internal functions from minterest-protocol pallet `do_repay` and `do_seize`, these
	/// functions within themselves call `accrue_interest_rate`. Also calls
	/// `mutate_attempts` for mutate user liquidation attempts.
	///
	/// - `borrower`: AccountId of the borrower whose loan is being liquidated.
	/// - `liquidation_amounts`: contains a vectors with user's borrows to be paid from the
	/// liquidation pools instead of the borrower, and a vector with user's supplies to be
	/// withdrawn from the borrower and sent to the liquidation pools. Balances are calculated
	/// in underlying assets.
	fn do_liquidate(borrower: &T::AccountId, user_loan_state: UserLoanState<T>) -> DispatchResult {
		let liquidation_pool_account_id = T::LiquidationPoolsManager::pools_account_id();
		// perform repay
		user_loan_state
			.get_user_borrows_to_repay_underlying()
			.into_iter()
			.try_for_each(|(pool_id, repay_underlying)| -> DispatchResult {
				T::MinterestProtocolManager::do_repay(
					&liquidation_pool_account_id,
					&borrower,
					pool_id,
					repay_underlying,
					false,
				)?;
				Ok(())
			})?;
		// perform seize
		user_loan_state
			.get_user_supplies_to_seize_underlying()
			.into_iter()
			.try_for_each(|(pool_id, seize_underlying)| -> DispatchResult {
				T::MinterestProtocolManager::do_seize(&borrower, pool_id, seize_underlying)?;
				Ok(())
			})?;

		// pay from liquidation pools
		user_loan_state
			.get_user_supplies_to_pay_underlying()
			.into_iter()
			.try_for_each(|(pool_id, pay_underlying)| -> DispatchResult {
				T::MultiCurrency::transfer(
					pool_id,
					&liquidation_pool_account_id,
					&T::LiquidityPoolsManager::pools_account_id(),
					pay_underlying,
				)
			})?;

		<Self as UserLiquidationAttemptsManager<T::AccountId>>::try_mutate_attempts(
			&borrower,
			Operation::Repay,
			None,
			user_loan_state.get_user_liquidation_mode(),
		)?;
		Ok(())
	}

	/// Checks if liquidation_fee <= 0.5
	fn is_valid_liquidation_fee(liquidation_fee: Rate) -> bool {
		liquidation_fee <= T::MaxLiquidationFee::get()
	}

	/// Increases the parameter liquidation_attempts by one for user.
	fn user_liquidation_attempts_increase_by_one(who: &T::AccountId) {
		UserLiquidationAttemptsStorage::<T>::mutate(who, |p| *p += u8::one())
	}

	/// Resets the parameter liquidation_attempts equal to zero for user.
	fn user_liquidation_attempts_reset_to_zero(who: &T::AccountId) {
		UserLiquidationAttemptsStorage::<T>::mutate(who, |p| *p = u8::zero())
	}
}

impl<T: Config> RiskManagerStorageProvider for Pallet<T> {
	fn create_pool(pool_id: CurrencyId, liquidation_threshold: Rate, liquidation_fee: Rate) -> DispatchResult {
		ensure!(
			!LiquidationFeeStorage::<T>::contains_key(pool_id),
			Error::<T>::RiskManagerParamsAlreadyCreated
		);
		ensure!(
			Self::is_valid_liquidation_fee(liquidation_fee),
			Error::<T>::InvalidLiquidationFeeValue
		);
		LiquidationFeeStorage::<T>::insert(pool_id, liquidation_fee);
		LiquidationThresholdStorage::<T>::put(liquidation_threshold);
		Ok(())
	}

	fn remove_pool(pool_id: CurrencyId) {
		LiquidationFeeStorage::<T>::remove(pool_id)
	}
}

impl<T: Config> UserLiquidationAttemptsManager<T::AccountId> for Pallet<T> {
	type LiquidationMode = LiquidationMode;

	/// Gets user liquidation attempts.
	fn get_user_liquidation_attempts(who: &T::AccountId) -> u8 {
		Self::user_liquidation_attempts_storage(who)
	}

	/// Mutates user liquidation attempts depending on user operation.
	/// If the user makes a deposit to one of his collateral liquidity pools, then user liquidation
	/// attempts are set to zero.
	/// In liquidation process:
	/// - partial liquidation - increases user liquidation attempts by one;
	/// - complete liquidation - set user liquidation attempts to zero.
	///
	/// Parameters:
	/// -`who`: user whose liquidation attempts change;
	/// -`operation`: operation during which changing user liquidation attempts. (parameter
	/// `operation` should be equal `Deposit` - deposit operation, or `Repay` - liquidation of
	/// user insolvent loan);
	/// -`pool_id`: in the case of an operation `Deposit` should be equal to the CurrencyId of
	/// the liquidity pool in which the user makes a deposit. In case of liquidation - should be
	/// equal `None`;
	/// -`liquidation_mode`: type of liquidation of user insolvent loan. In case of `Deposit`
	/// should be equal `None`;
	fn try_mutate_attempts(
		who: &T::AccountId,
		operation: Operation,
		pool_id: Option<CurrencyId>,
		liquidation_mode: Option<LiquidationMode>,
	) -> DispatchResult {
		match operation {
			Operation::Deposit => pool_id.map_or(Err(Error::<T>::ErrorChangingLiquidationAttempts), {
				|pool_id| {
					if T::UserCollateral::is_pool_collateral(&who, pool_id) {
						let user_liquidation_attempts = Self::get_user_liquidation_attempts(&who);
						if !user_liquidation_attempts.is_zero() {
							Self::user_liquidation_attempts_reset_to_zero(&who);
						}
					}
					Ok(())
				}
			}),
			Operation::Repay => liquidation_mode.map_or(Err(Error::<T>::ErrorChangingLiquidationAttempts), {
				|mode| {
					match mode {
						LiquidationMode::Partial => Self::user_liquidation_attempts_increase_by_one(&who),
						LiquidationMode::Complete => Self::user_liquidation_attempts_reset_to_zero(&who),
					}
					Ok(())
				}
			}),
			_ => Err(Error::<T>::ErrorChangingLiquidationAttempts),
		}?;
		Ok(())
	}
}
