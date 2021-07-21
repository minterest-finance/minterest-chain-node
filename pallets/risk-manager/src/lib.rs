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
//! _None_. Called from OCW.

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
use liquidity_pools::Pool;
use minterest_primitives::{
	currency::CurrencyType::UnderlyingAsset, Balance, CurrencyId, OffchainErr, Operation, Rate,
};
pub use module::*;
use pallet_traits::{
	ControllerManager, CurrencyConverter, LiquidityPoolStorageProvider, MinterestProtocolManager, PoolsManager,
	PricesManager, RiskManagerStorageProvider, UserCollateral, UserLiquidationAttemptsManager,
};
use sp_runtime::traits::{One, StaticLookup, Zero};
use sp_std::prelude::*;

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

/// Types of liquidation of user loans.
enum LiquidationMode {
	/// Makes the user's loan solvent. A portion of the user's borrow is paid from the
	/// liquidation pools, and a portion of the user's collateral is withdrawn and transferred to
	/// the liquidation pools.
	Partial,
	/// All user borrow is paid from liquidation pools. The user's collateral required to cover
	/// the borrow is withdrawn and transferred to liquidation pools.
	Complete,
	/// Occurs when the user's borrow exceeds his supply. This type refers to complete liquidation.
	ForgivableComplete,
}

/// Contains information about the transferred amounts for liquidation.
#[derive(Encode, Decode, RuntimeDebug, Clone, PartialOrd, PartialEq)]
pub struct LiquidationAmounts {
	/// Contains a vector of pools and a balances that must be paid instead of the borrower from
	/// liquidation pools to liquidity pools.
	borrower_loans_to_repay_underlying: Vec<(CurrencyId, Balance)>,
	/// Contains a vector of pools and a balances that must be withdrawn from the user's collateral
	/// and sent to the liquidation pools.
	borrower_supplies_to_seize_underlying: Vec<(CurrencyId, Balance)>,
}

/// Contains information about the current state of the borrower's loan.
#[derive(Encode, Decode, RuntimeDebug, Clone, PartialOrd, PartialEq)]
pub struct UserLoanState {
	/// Vector of user loans. Contains information about the CurrencyId and the amount of loan.
	loans: Vec<(CurrencyId, Balance)>,
	/// Vector of user supplies. Contains information about the CurrencyId and the amount of supply.
	supplies: Vec<(CurrencyId, Balance)>,
}

impl UserLoanState {
	fn new() -> Self {
		Self {
			loans: Vec::new(),
			supplies: Vec::new(),
		}
	}

	/// Returns user_total_borrow_usd.
	fn total_borrow_usd(&self) -> Balance {
		self.loans.iter().map(|(_, v)| v).sum()
	}

	/// Returns user_total_supply_usd.
	fn total_supply_usd(&self) -> Balance {
		self.supplies.iter().map(|(_, v)| v).sum()
	}
}

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
		type LiquidityPoolsManager: LiquidityPoolStorageProvider<Self::AccountId, Pool>
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
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(crate) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Liquidation fee has been successfully changed: \[pool_id, liquidation_fee\]
		LiquidationFeeUpdated(CurrencyId, Rate),
		/// Liquidation threshold has been successfully changed: \[threshold\]
		LiquidationThresholdUpdated(Rate),
	}

	/// The additional collateral which is taken from borrowers as a penalty for being liquidated.
	/// Sets for each liquidity pool separately.
	#[pallet::storage]
	#[pallet::getter(fn liquidation_fee_storage)]
	pub(crate) type LiquidationFeeStorage<T: Config> = StorageMap<_, Twox64Concat, CurrencyId, Rate, ValueQuery>;

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
		/// - `pool_id`: PoolID for which the parameter value is being set.
		/// - `threshold`: new threshold.
		///
		/// The dispatch origin of this call must be 'RiskManagerUpdateOrigin'.
		#[pallet::weight(0)]
		#[transactional]
		pub fn set_liquidation_threshold(
			origin: OriginFor<T>,
			pool_id: CurrencyId,
			threshold: Rate,
		) -> DispatchResultWithPostInfo {
			T::RiskManagerUpdateOrigin::ensure_origin(origin)?;
			ensure!(
				pool_id.is_supported_underlying_asset(),
				Error::<T>::NotValidUnderlyingAssetId
			);
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
		/// - `borrower`: AccountId of the borrower whose loan is being liquidated.
		/// - `liquidation_amounts`: contains a vectors with user's borrows to be paid from the
		/// liquidation pools instead of the borrower, and a vector with user's supplies to be
		/// withdrawn from the borrower and sent to the liquidation pools. Balances are calculated
		/// in underlying assets.
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
		let user_loan_state = Self::get_user_loan_state(&borrower).map_err(|_| OffchainErr::CheckFail)?;
		let liquidation_mode = Self::choose_liquidation_mode(&borrower, &user_loan_state);
		let liquidation_amounts =
			match liquidation_mode {
				LiquidationMode::Partial => Self::calculate_partial_liquidation(&borrower, &user_loan_state)
					.map_err(|_| OffchainErr::CheckFail)?,
				LiquidationMode::Complete => Self::calculate_complete_liquidation(&borrower, &user_loan_state)
					.map_err(|_| OffchainErr::CheckFail)?,
				LiquidationMode::ForgivableComplete => {
					Self::calculate_forgivable_complete_liquidation(&borrower, &user_loan_state)
						.map_err(|_| OffchainErr::CheckFail)?
				}
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

	// TODO: Raw implementation. Cover with unit-tests.
	/// Selects the liquidation mode for the user's loan. The choice of the liquidation mode is
	/// made based on the parameters of the current number of user's liquidation attempts and
	/// the current state of the user's loan.
	///
	/// -`borrower`: user for which the liquidation mode is chosen.
	/// -`user_loan_state`: contains the current state of the borrower's loan.
	fn choose_liquidation_mode(borrower: &T::AccountId, user_loan_state: &UserLoanState) -> LiquidationMode {
		let (borrower_total_borrow_usd, borrower_total_supply_usd) =
			(user_loan_state.total_borrow_usd(), user_loan_state.total_supply_usd());
		let user_liquidation_attempts = Self::get_user_liquidation_attempts(&borrower);
		if borrower_total_borrow_usd >= T::PartialLiquidationMinSum::get()
			&& user_liquidation_attempts < T::PartialLiquidationMaxAttempts::get()
		{
			LiquidationMode::Partial
		} else if borrower_total_borrow_usd > borrower_total_supply_usd {
			LiquidationMode::ForgivableComplete
		} else {
			LiquidationMode::Complete
		}
	}

	// TODO: Raw implementation. Cover with unit-tests.
	/// Calculates the state of the user's loan. Considers supply only for those pools that are
	/// enabled as collateral.
	/// Returns user supplies and user borrows for each pool.
	///
	/// -`who`: AccountId of the user whose loan state is being calculated.
	fn get_user_loan_state(who: &T::AccountId) -> Result<UserLoanState, DispatchError> {
		CurrencyId::get_enabled_tokens_in_protocol(UnderlyingAsset)
			.into_iter()
			.filter(|&pool_id| T::LiquidityPoolsManager::pool_exists(&pool_id))
			.try_fold(
				UserLoanState::new(),
				|mut acc_user_state, pool_id| -> Result<UserLoanState, DispatchError> {
					let oracle_price =
						T::PriceSource::get_underlying_price(pool_id).ok_or(Error::<T>::InvalidFeedPrice)?;

					let user_borrow_underlying =
						T::ControllerManager::get_user_borrow_underlying_balance(who, pool_id)?;
					let user_borrow_usd =
						T::LiquidityPoolsManager::underlying_to_usd(user_borrow_underlying, oracle_price)?;
					acc_user_state.loans.push((pool_id, user_borrow_usd));

					if T::LiquidityPoolsManager::is_pool_collateral(&who, pool_id) {
						let user_supply_underlying =
							T::ControllerManager::get_user_supply_underlying_balance(who, pool_id)?;
						let user_supply_usd =
							T::LiquidityPoolsManager::underlying_to_usd(user_supply_underlying, oracle_price)?;
						acc_user_state.supplies.push((pool_id, user_supply_usd));
					}
					Ok(acc_user_state)
				},
			)
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
		<Self as UserLiquidationAttemptsManager<T::AccountId>>::mutate_attempts(None, &borrower, Operation::Repay);
		Ok(())
	}

	/// Based on the current state of the user's insolvent loan, it calculates the amounts required
	/// for partial liquidation.
	///
	/// Returns: vectors with user's borrows to be paid from the liquidation pools instead of
	/// the borrower, and a vector with user's supplies to be withdrawn from the borrower and sent
	/// to the liquidation pools. Balances are calculated in underlying assets.
	///
	/// Note: this function should be used after `accrue_interest_rate`.
	fn calculate_partial_liquidation(
		_borrower: &T::AccountId,
		_borrower_loan_state: &UserLoanState,
	) -> Result<LiquidationAmounts, DispatchError> {
		todo!()
	}

	/// Based on the current state of the user's insolvent loan, it calculates the amounts required
	/// for complete liquidation.
	///
	/// Returns: vectors with user's borrows to be paid from the liquidation pools instead of
	/// the borrower, and a vector with user's supplies to be withdrawn from the borrower and sent
	/// to the liquidation pools. Balances are calculated in underlying assets.
	///
	/// Note: this function should be used after `accrue_interest_rate`.
	fn calculate_complete_liquidation(
		_borrower: &T::AccountId,
		_borrower_loan_state: &UserLoanState,
	) -> Result<LiquidationAmounts, DispatchError> {
		todo!()
	}

	/// Based on the current state of the user's insolvent loan, it calculates the amounts required
	/// for "forgivable" complete liquidation. This function is called when user_total_borrow is
	/// greater than user_total_supply.
	///
	/// Returns: vectors with user's borrows to be paid from the liquidation pools instead of
	/// the borrower, and a vector with user's supplies to be withdrawn from the borrower and sent
	/// to the liquidation pools. Balances are calculated in underlying assets.
	///
	/// Note: this function should be used after `accrue_interest_rate`.
	fn calculate_forgivable_complete_liquidation(
		_borrower: &T::AccountId,
		_borrower_loan_state: &UserLoanState,
	) -> Result<LiquidationAmounts, DispatchError> {
		todo!()
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
	/// Gets user liquidation attempts.
	fn get_user_liquidation_attempts(who: &T::AccountId) -> u8 {
		Self::user_liquidation_attempts_storage(who)
	}

	/// Mutates user liquidation attempts depending on user operation.
	/// If the user makes a deposit to the collateral pool, then attempts are set to zero.
	/// TODO: implement mutate in case of liquidation
	fn mutate_attempts(pool_id: Option<CurrencyId>, who: &T::AccountId, operation: Operation) {
		// pool_id existence in case of a deposit operation
		if let Some(pool_id) = pool_id {
			if operation == Operation::Deposit && T::UserCollateral::is_pool_collateral(&who, pool_id) {
				let user_liquidation_attempts = Self::get_user_liquidation_attempts(&who);
				if !user_liquidation_attempts.is_zero() {
					Self::user_liquidation_attempts_reset_to_zero(&who);
				}
			}
		// Fixme: After implementation of liquidation fix this case and cover with tests
		} else if operation == Operation::Repay {
			Self::user_liquidation_attempts_increase_by_one(&who);
		}
	}
}
