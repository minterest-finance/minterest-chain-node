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

use frame_support::{log, pallet_prelude::*, transactional};
use frame_system::{
	ensure_none,
	offchain::{SendTransactionTypes, SubmitTransaction},
	pallet_prelude::OriginFor,
};
pub use liquidation::*;
use liquidity_pools::PoolData;
use minterest_primitives::{
	OriginalAsset, Balance, OffchainErr, Operation, Rate,
};
pub use module::*;
use pallet_traits::{
	ControllerManager, CurrencyConverter, LiquidityPoolStorageProvider, MinterestProtocolManager, PoolsManager,
	PricesManager, RiskManagerStorageProvider, UserCollateral, UserLiquidationAttemptsManager,
};
use sp_runtime::{
	traits::{CheckedAdd, CheckedMul, One, StaticLookup, Zero},
	FixedPointNumber,
};
use sp_std::prelude::*;

mod liquidation;
#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

#[frame_support::pallet]
pub mod module {
	use super::*;

	#[pallet::config]
	pub trait Config: frame_system::Config + SendTransactionTypes<Call<Self>> {
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

		/// A configuration for base priority of unsigned transactions.
		///
		/// This is exposed so that it can be tuned for particular runtime, when
		/// multiple pallets send unsigned transactions.
		type UnsignedPriority: Get<TransactionPriority>;

		/// The price source of currencies
		type PriceSource: PricesManager<OriginalAsset>;

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
			+ UserCollateral<Self::AccountId>;

		/// Provides the basic liquidation pools functionality.
		type LiquidationPoolsManager: PoolsManager<Self::AccountId>;

		/// Provides the basic minterest protocol functionality.
		type MinterestProtocolManager: MinterestProtocolManager<Self::AccountId>;
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
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(crate) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Liquidation fee has been successfully changed: \[pool_id, liquidation_fee\]
		LiquidationFeeUpdated(OriginalAsset, Rate),
		/// Liquidation threshold has been successfully changed: \[threshold\]
		LiquidationThresholdUpdated(Rate),
	}

	/// The additional collateral which is taken from borrowers as a penalty for being liquidated.
	/// Sets for each liquidity pool separately.
	#[pallet::storage]
	#[pallet::getter(fn liquidation_fee_storage)]
	pub(crate) type LiquidationFeeStorage<T: Config> = StorageMap<_, Twox64Concat, OriginalAsset, Rate, ValueQuery>;

	/// Step used in liquidation to protect the user from micro liquidations. One value for
	/// the entire protocol.
	#[pallet::storage]
	#[pallet::getter(fn liquidation_threshold_storage)]
	pub(crate) type LiquidationThresholdStorage<T: Config> = StorageValue<_, Rate, ValueQuery>;

	/// Counter of the number of partial liquidations at the user.
	#[pallet::storage]
	#[pallet::getter(fn user_liquidation_attempts_storage)]
	pub(crate) type UserLiquidationAttemptsStorage<T: Config> =
		StorageMap<_, Twox64Concat, T::AccountId, u8, ValueQuery>;

	#[pallet::genesis_config]
	pub struct GenesisConfig<T: Config> {
		pub liquidation_fee: Vec<(OriginalAsset, Rate)>,
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
		#[pallet::weight(0)]
		#[transactional]
		pub fn set_liquidation_fee(
			origin: OriginFor<T>,
			pool_id: OriginalAsset,
			liquidation_fee: Rate,
		) -> DispatchResultWithPostInfo {
			T::RiskManagerUpdateOrigin::ensure_origin(origin)?;
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
		/// - `pool_id`: PoolID for which the parameter value is being set.
		/// - `threshold`: new threshold.
		///
		/// The dispatch origin of this call must be 'RiskManagerUpdateOrigin'.
		#[pallet::weight(0)]
		#[transactional]
		pub fn set_liquidation_threshold(origin: OriginFor<T>, threshold: Rate) -> DispatchResultWithPostInfo {
			T::RiskManagerUpdateOrigin::ensure_origin(origin)?;
			LiquidationThresholdStorage::<T>::put(threshold);
			Self::deposit_event(Event::LiquidationThresholdUpdated(threshold));
			Ok(().into())
		}

		/// Liquidate insolvent loan. Calls internal functions from minterest-protocol pallet
		/// `do_repay` and `do_seize`, these functions within themselves call
		/// `accrue_interest_rate`. Before calling the extrinsic, it is necessary to perform all
		/// checks and math calculations of the user's borrows and collaterals.
		///
		/// The dispatch origin of this call must be _None_.
		///
		/// Parameters:
		/// - `borrower`: AccountId of the borrower whose loan is being liquidated.
		/// - `liquidation_amounts`: contains a vectors with user's borrows to be paid from the
		/// liquidation pools instead of the borrower, and a vector with user's supplies to be
		/// withdrawn from the borrower and sent to the liquidation pools. Balances are calculated
		/// in underlying assets.
		///TODO: try to use the struct `UserLoanState` in the last parameter (add Debug constraint
		/// to Config).
		///
		/// The dispatch origin of this call must be _None_.
		#[pallet::weight(0)]
		#[transactional]
		pub fn liquidate(
			origin: OriginFor<T>,
			borrower: <T::Lookup as StaticLookup>::Source,
			liquidation_amounts: LiquidationAmounts,
		) -> DispatchResultWithPostInfo {
			ensure_none(origin)?;
			let borrower = T::Lookup::lookup(borrower)?;
			Self::do_liquidate(&borrower, liquidation_amounts)?;
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
	/// Checks if the node is a validator. The worker is launched every block. The worker's working
	/// time is limited in time. Each next worker starts checking user loans from the beginning.
	/// Calls a processing insolvent loan function.
	fn _offchain_worker() -> Result<(), OffchainErr> {
		// Check if we are a potential validator
		if !sp_io::offchain::is_validator() {
			return Err(OffchainErr::NotValidator);
		}

		//TODO: After implementing the architecture of liquidation, implement specific errors in
		// the enum OffchainErr.
		let borrower_iterator = T::ControllerManager::get_all_users_with_insolvent_loan()
			.map_err(|_| OffchainErr::CheckFail)?
			.into_iter();

		// TODO: offchain worker locks

		for borrower in borrower_iterator {
			Self::process_insolvent_loan(borrower)?;
			// TODO: offchain worker guard try extend
		}

		Ok(())
	}

	/// Handles the user's loan. Selects one of the required types of liquidation (Partial,
	/// Complete or Forgivable Complete) and calls extrinsic `liquidate()`. This function within
	/// itself call `accrue_interest_rate`.
	///
	/// -`borrower`: AccountId of the borrower whose loan is being processed.
	fn process_insolvent_loan(borrower: T::AccountId) -> Result<(), OffchainErr> {
		let user_loan_state: UserLoanState<T> =
			UserLoanState::build_user_loan_state(&borrower).map_err(|_| OffchainErr::CheckFail)?;
		ensure!(borrower == *user_loan_state.get_user(), OffchainErr::CheckFail);
		let liquidation_amounts = match user_loan_state
			.choose_liquidation_mode()
			.map_err(|_| OffchainErr::CheckFail)?
		{
			LiquidationMode::Partial => user_loan_state
				.calculate_partial_liquidation()
				.map_err(|_| OffchainErr::CheckFail)?,
			LiquidationMode::Complete => user_loan_state
				.calculate_complete_liquidation()
				.map_err(|_| OffchainErr::CheckFail)?,
			LiquidationMode::ForgivableComplete => user_loan_state
				.calculate_forgivable_complete_liquidation()
				.map_err(|_| OffchainErr::CheckFail)?,
		};

		// call to change the offchain worker local storage
		Self::do_liquidate(&borrower, liquidation_amounts.clone()).map_err(|_| OffchainErr::CheckFail)?;
		Self::submit_unsigned_liquidation(&borrower, liquidation_amounts);
		Ok(())
	}

	/// Submits an unsigned liquidation transaction to the blockchain.
	///
	/// -`borrower`: AccountId of the borrower whose loan is being processed.
	/// - `liquidation_amounts`: contains a vectors with user's borrows to be paid from the
	/// liquidation pools instead of the borrower, and a vector with user's supplies to be
	/// withdrawn from the borrower and sent to the liquidation pools. Balances are calculated
	/// in underlying assets.
	fn submit_unsigned_liquidation(borrower: &T::AccountId, liquidation_amounts: LiquidationAmounts) {
		let who = T::Lookup::unlookup(borrower.clone());
		let call = Call::<T>::liquidate(who.clone(), liquidation_amounts);
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
	fn do_liquidate(borrower: &T::AccountId, liquidation_amounts: LiquidationAmounts) -> DispatchResult {
		let liquidation_pool_account_id = T::LiquidationPoolsManager::pools_account_id();
		liquidation_amounts
			.borrower_loans_to_repay_underlying
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
		liquidation_amounts
			.borrower_supplies_to_seize_underlying
			.into_iter()
			.try_for_each(|(pool_id, seize_underlying)| -> DispatchResult {
				T::MinterestProtocolManager::do_seize(&borrower, pool_id, seize_underlying)?;
				Ok(())
			})?;
		// TODO: need liquidation mode here
		<Self as UserLiquidationAttemptsManager<T::AccountId>>::try_mutate_attempts(
			&borrower,
			Operation::Repay,
			None,
			None,
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
	fn create_pool(pool_id: OriginalAsset, liquidation_threshold: Rate, liquidation_fee: Rate) -> DispatchResult {
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

	fn remove_pool(pool_id: OriginalAsset) {
		LiquidationFeeStorage::<T>::remove(pool_id)
	}
}

impl<T: Config> UserLiquidationAttemptsManager<T::AccountId> for Pallet<T> {
	type LiquidationMode = LiquidationMode;

	/// Gets user liquidation attempts.
	fn get_user_liquidation_attempts(who: &T::AccountId) -> u8 {
		Self::user_liquidation_attempts_storage(who)
	}

	// TODO: Raw implementation, cover with tests. No need to review this function.
	/// Mutates user liquidation attempts depending on user operation.
	/// If the user makes a deposit to the collateral pool, then attempts are set to zero.
	/// -`who`:
	/// -`operation`:
	/// -`pool_id`:
	/// -`liquidation_mode`:
	fn try_mutate_attempts(
		who: &T::AccountId,
		operation: Operation,
		pool_id: Option<OriginalAsset>,
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
						LiquidationMode::ForgivableComplete => Self::user_liquidation_attempts_reset_to_zero(&who),
					}
					Ok(())
				}
			}),
			_ => Err(Error::<T>::ErrorChangingLiquidationAttempts),
		}?;
		Ok(())
	}
}
