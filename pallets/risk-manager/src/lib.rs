//! # Risk Manager Pallet
//!
//! ## Overview
//!
//! TODO: add comments

#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::unused_unit)]
#![allow(clippy::upper_case_acronyms)]

use frame_support::{log, pallet_prelude::*, transactional};
use frame_system::{
	ensure_none,
	offchain::{SendTransactionTypes, SubmitTransaction},
	pallet_prelude::OriginFor,
};
use minterest_primitives::{Balance, CurrencyId, OffchainErr, Operation, Rate};
pub use module::*;
use pallet_traits::{ControllerManager, RiskManagerStorageProvider, UserCollateral, UserLiquidationAttemptsManager};
use sp_runtime::traits::{One, StaticLookup, Zero};
#[cfg(feature = "std")]
use sp_std::str;
use sp_std::vec::Vec;

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

/// Types of liquidation of user loans.
enum LiquidationMode {
	Partial,
	Complete,
	ForgivableComplete,
}

/// Contains information about the transferred amounts for liquidation.
#[derive(Encode, Decode, RuntimeDebug, Clone, PartialOrd, PartialEq)]
pub struct LiquidationAmounts {
	/// Contains a vector of pools and a balances that must be paid instead of the borrower from
	/// liquidation pools to liquidity pools.
	borrower_loans_to_repay_usd: Vec<(CurrencyId, Balance)>,
	/// Contains a vector of pools and a balances that must be withdrawn from the user's collateral
	/// and sent to the liquidation pools.
	borrower_supplies_to_seize_usd: Vec<(CurrencyId, Balance)>,
}

/// Contains information about the current state of the borrower's loan.
#[derive(Encode, Decode, RuntimeDebug, Clone, PartialOrd, PartialEq)]
pub struct UserLoanState {
	/// Vector of user currencies and loans
	loans: Vec<(CurrencyId, Balance)>,
	/// Vector of user currencies and supplies
	supplies: Vec<(CurrencyId, Balance)>,
}

impl UserLoanState {
	fn _user_total_borrow_usd(&self) -> Balance {
		self.loans.iter().map(|(_, v)| v).sum()
	}

	fn _user_total_supply_usd(&self) -> Balance {
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
	}

	#[pallet::error]
	pub enum Error<T> {
		/// The currency is not enabled in protocol.
		NotValidUnderlyingAssetId,
		/// Liquidation fee can't be greater than 0.5.
		InvalidLiquidationFeeValue,
		/// Risk manager storage (liquidation_fee, liquidation_threshold) is already created.
		RiskManagerParamsAlreadyCreated,
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(crate) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Liquidation fee has been successfully changed: \[liquidation_fee\]
		LiquidationFeeUpdated(Rate),
		/// Liquidation threshold has been successfully changed: \[threshold\]
		LiquidationThresholdUpdated(Rate),
	}

	/// The additional collateral which is taken from borrowers as a penalty for being liquidated.
	#[pallet::storage]
	#[pallet::getter(fn liquidation_fee)]
	pub(crate) type LiquidationFee<T: Config> = StorageMap<_, Twox64Concat, CurrencyId, Rate, ValueQuery>;

	/// Step used in liquidation to protect the user from micro liquidations.
	#[pallet::storage]
	#[pallet::getter(fn liquidation_threshold)]
	pub(crate) type LiquidationThreshold<T: Config> = StorageValue<_, Rate, ValueQuery>;

	/// Counter of the number of partial liquidations at the user.
	#[pallet::storage]
	#[pallet::getter(fn user_liquidation_attempts)]
	pub(crate) type UserLiquidationAttempts<T: Config> = StorageMap<_, Twox64Concat, T::AccountId, u8, ValueQuery>;

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
				LiquidationFee::<T>::insert(pool_id, liquidation_fee)
			});
			LiquidationThreshold::<T>::put(self.liquidation_threshold);
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
		/// Set Liquidation fee that covers liquidation costs.
		///
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
			LiquidationFee::<T>::insert(pool_id, liquidation_fee);
			Self::deposit_event(Event::LiquidationFeeUpdated(liquidation_fee));
			Ok(().into())
		}

		/// Set threshold which used in liquidation to protect the user from micro liquidations.
		///
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
			LiquidationThreshold::<T>::put(threshold);
			Self::deposit_event(Event::LiquidationThresholdUpdated(threshold));
			Ok(().into())
		}

		/// TODO: implement
		#[pallet::weight(0)]
		#[transactional]
		pub fn liquidate(
			origin: OriginFor<T>,
			borrower: <T::Lookup as StaticLookup>::Source,
			user_loan_state: LiquidationAmounts,
		) -> DispatchResultWithPostInfo {
			ensure_none(origin)?;
			let borrower = T::Lookup::lookup(borrower)?;
			Self::do_liquidate(&borrower, user_loan_state)?;
			Ok(().into())
		}
	}
}

// Private functions
impl<T: Config> Pallet<T> {
	/// Checks if the node is a validator. The worker is launched every block. The worker's working
	/// time is limited in time. Each next worker starts checking user loans from the beginning.
	/// Calls a processing insolvent loans function.
	fn _offchain_worker() -> Result<(), OffchainErr> {
		// Check if we are a potential validator
		if !sp_io::offchain::is_validator() {
			return Err(OffchainErr::NotValidator);
		}

		let mut user_unsafe_loan_iterator = T::ControllerManager::get_all_users_with_unsafe_loan()
			.map_err(|_| OffchainErr::CheckFail)?
			.into_iter();

		//TODO: offchain worker locks

		while let Some(borrower) = user_unsafe_loan_iterator.next() {
			Self::process_insolvent_loan(borrower)?;
			//TODO: offchain worker guard try extend
		}

		Ok(())
	}

	/// Handles the user's loan. Selects one of the required types of liquidation (Partial,
	/// Complete or Forgivable Complete) and calls extrinsic `liquidate()`.
	///
	/// -`borrower`: AccountId of the borrower whose loan is being processed.
	fn process_insolvent_loan(borrower: T::AccountId) -> Result<(), OffchainErr> {
		let _user_loan_state = Self::get_user_loan_state(&borrower).map_err(|_| OffchainErr::CheckFail)?;
		let mode = Self::choose_liquidation_mode(&borrower, Balance::zero(), Balance::zero());
		let borrower_loan_state = match mode {
			LiquidationMode::Partial => {
				Self::calculate_partial_liquidation(&borrower).map_err(|_| OffchainErr::CheckFail)?
			}
			LiquidationMode::Complete => {
				Self::calculate_complete_liquidation(&borrower).map_err(|_| OffchainErr::CheckFail)?
			}
			LiquidationMode::ForgivableComplete => {
				Self::calculate_forgivable_complete_liquidation(&borrower).map_err(|_| OffchainErr::CheckFail)?
			}
		};
		// call to change the offchain worker local storage
		Self::do_liquidate(&borrower, borrower_loan_state.clone()).map_err(|_| OffchainErr::CheckFail)?;
		Self::submit_unsigned_liquidation(&borrower, borrower_loan_state);
		Self::mutate_depending_operation(None, &borrower, Operation::Repay);
		Ok(())
	}

	/// Submits an unsigned liquidation transaction to the blockchain.
	///
	/// -`borrower`: AccountId of the borrower whose loan is being processed.
	/// -`user_loan_state`:
	fn submit_unsigned_liquidation(borrower: &T::AccountId, user_loan_state: LiquidationAmounts) {
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

	/// Selects the liquidation mode for the user's loan.
	fn choose_liquidation_mode(
		borrower: &T::AccountId,
		borrower_total_supply_usd: Balance,
		borrower_total_borrow_usd: Balance,
	) -> LiquidationMode {
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

	///
	fn get_user_loan_state(_who: &T::AccountId) -> Result<UserLoanState, DispatchError> {
		todo!()
	}

	///
	fn do_liquidate(_borrower: &T::AccountId, _user_loan_state: LiquidationAmounts) -> DispatchResult {
		// TODO: call accrue_interest somewhere
		todo!()
	}

	/// TODO: implement
	///
	/// Должна вызываться на свежем храгилище, после вызова accrue_interest.
	fn calculate_partial_liquidation(_borrower: &T::AccountId) -> Result<LiquidationAmounts, DispatchError> {
		todo!()
	}

	/// TODO: implement
	///
	/// Должна вызываться на свежем храгилище, после вызова accrue_interest.
	fn calculate_complete_liquidation(_borrower: &T::AccountId) -> Result<LiquidationAmounts, DispatchError> {
		todo!()
	}

	/// TODO: implement
	///
	/// Должна вызываться на свежем храгилище, после вызова accrue_interest.
	fn calculate_forgivable_complete_liquidation(
		_borrower: &T::AccountId,
	) -> Result<LiquidationAmounts, DispatchError> {
		todo!()
	}

	/// Checks if liquidation_fee <= 0.5
	fn is_valid_liquidation_fee(liquidation_fee: Rate) -> bool {
		liquidation_fee <= T::MaxLiquidationFee::get()
	}
}

impl<T: Config> RiskManagerStorageProvider for Pallet<T> {
	fn create_pool(pool_id: CurrencyId, liquidation_threshold: Rate, liquidation_fee: Rate) -> DispatchResult {
		ensure!(
			!LiquidationFee::<T>::contains_key(pool_id),
			Error::<T>::RiskManagerParamsAlreadyCreated
		);
		ensure!(
			Self::is_valid_liquidation_fee(liquidation_fee),
			Error::<T>::InvalidLiquidationFeeValue
		);
		LiquidationFee::<T>::insert(pool_id, liquidation_fee);
		LiquidationThreshold::<T>::put(liquidation_threshold);
		Ok(())
	}

	fn remove_pool(pool_id: CurrencyId) {
		LiquidationFee::<T>::remove(pool_id)
	}
}

impl<T: Config> UserLiquidationAttemptsManager<T::AccountId> for Pallet<T> {
	fn get_user_liquidation_attempts(who: &T::AccountId) -> u8 {
		Self::user_liquidation_attempts(who)
	}

	fn increase_by_one(who: &T::AccountId) {
		UserLiquidationAttempts::<T>::mutate(who, |p| *p += u8::one())
	}

	fn reset_to_zero(who: &T::AccountId) {
		UserLiquidationAttempts::<T>::mutate(&who, |p| *p = u8::zero())
	}

	/// Mutates user liquidation attempts depending on user operation.
	/// If the user makes a deposit to the collateral pool, then attempts are set to zero.
	/// TODO: implement mutate in case of liquidation
	fn mutate_depending_operation(pool_id: Option<CurrencyId>, who: &T::AccountId, operation: Operation) {
		// pool_id existence in case of a deposit operation
		if let Some(pool_id) = pool_id {
			if operation == Operation::Deposit && T::UserCollateral::is_pool_collateral(&who, pool_id) {
				let user_liquidation_attempts = Self::get_user_liquidation_attempts(&who);
				if !user_liquidation_attempts.is_zero() {
					Self::reset_to_zero(&who);
				}
			}
		} else {
			todo!()
		}
	}
}

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
