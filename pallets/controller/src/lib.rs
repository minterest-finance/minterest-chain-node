//! # Controller Module
//!
//! ## Overview
//!
//! Contains protocol settings and helper functions related to interest calculations.
//! Also it is managing paused operations and whitelist mode. These are related to protocol
//! security. In case of emergency some of protocol operations can be paused by authorized users.
//! When Whitelist mode is enabled, protocol interaction is restricted to whitelist members only.

#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::unused_unit)]
#![allow(clippy::upper_case_acronyms)]

use codec::{Decode, Encode};
use frame_support::{ensure, pallet_prelude::*, transactional};
use frame_system::pallet_prelude::*;
use liquidity_pools::Pool;
use minterest_primitives::{Amount, Balance, CurrencyId, Interest, Operation, Rate};
use orml_traits::MultiCurrency;
use pallet_traits::{ControllerManager, LiquidityPoolsManager, MntManager, PoolsManager, PricesManager};
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};
use sp_runtime::traits::CheckedSub;
use sp_runtime::{
	traits::{CheckedAdd, CheckedDiv, CheckedMul, Zero},
	DispatchError, DispatchResult, FixedPointNumber, FixedU128, RuntimeDebug,
};
use sp_std::{cmp::Ordering, convert::TryInto, prelude::Vec, result};

pub use module::*;

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

pub mod weights;
use minterest_primitives::arithmetic::sum_with_mult_result;
use minterest_primitives::currency::CurrencyType::{UnderlyingAsset, WrappedToken};
pub use weights::WeightInfo;

#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(Encode, Decode, Clone, RuntimeDebug, Eq, PartialEq, Default)]
pub struct ControllerData<BlockNumber> {
	/// Block number that interest was last accrued at.
	pub last_interest_accrued_block: BlockNumber,

	/// Defines the portion of borrower interest that is converted into protocol interest.
	pub protocol_interest_factor: Rate,

	/// Maximum borrow rate.
	pub max_borrow_rate: Rate,

	/// This multiplier represents which share of the supplied value can be used as a collateral for
	/// loans. For instance, 0.9 allows 90% of total pool value to be used as a collaterae. Must be
	/// between 0 and 1.
	pub collateral_factor: Rate,

	/// Maximum total borrow amount per pool in usd. No value means infinite borrow cap.
	pub borrow_cap: Option<Balance>,

	/// Minimum protocol interest needed to transfer it to liquidation pool
	pub protocol_interest_threshold: Balance,
}

/// The Root or half MinterestCouncil can pause certain actions as a safety mechanism.
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(Encode, Decode, RuntimeDebug, Eq, PartialEq, Default)]
pub struct PauseKeeper {
	/// Pause mint operation in the pool.
	pub deposit_paused: bool,

	/// Pause redeem operation in the pool.
	pub redeem_paused: bool,

	/// Pause borrow operation in the pool.
	pub borrow_paused: bool,

	/// Pause repay operation in the pool.
	pub repay_paused: bool,

	/// Pause transfer operation in the pool.
	pub transfer_paused: bool,
}

impl PauseKeeper {
	pub fn all_paused() -> Self {
		PauseKeeper {
			deposit_paused: true,
			redeem_paused: true,
			borrow_paused: true,
			repay_paused: true,
			transfer_paused: true,
		}
	}
	pub fn all_unpaused() -> Self {
		PauseKeeper {
			deposit_paused: false,
			redeem_paused: false,
			borrow_paused: false,
			repay_paused: false,
			transfer_paused: false,
		}
	}
}

pub struct GetAllPaused;
impl frame_support::traits::Get<PauseKeeper> for GetAllPaused {
	fn get() -> PauseKeeper {
		PauseKeeper::all_paused()
	}
}

type LiquidityPools<T> = liquidity_pools::Module<T>;
type MinterestModel<T> = minterest_model::Module<T>;
type RateResult = result::Result<Rate, DispatchError>;
type BalanceResult = result::Result<Balance, DispatchError>;
type LiquidityResult = result::Result<(Balance, Balance), DispatchError>;

#[frame_support::pallet]
pub mod module {
	use super::*;

	#[pallet::config]
	pub trait Config: frame_system::Config + liquidity_pools::Config + minterest_model::Config {
		/// The overarching event type.
		type Event: From<Event> + IsType<<Self as frame_system::Config>::Event>;

		/// Provides the basic liquidity pools manager and liquidity pool functionality.
		type LiquidityPoolsManager: LiquidityPoolsManager + PoolsManager<Self::AccountId>;

		#[pallet::constant]
		/// Maximum total borrow amount per pool in usd.
		type MaxBorrowCap: Get<Balance>;

		/// The origin which may update controller parameters. Root or
		/// Half Minterest Council can always do this.
		type UpdateOrigin: EnsureOrigin<Self::Origin>;

		/// Weight information for the extrinsics.
		type ControllerWeightInfo: WeightInfo;

		/// Provides MNT token distribution functionality.
		type MntManager: MntManager<Self::AccountId>;
	}

	#[pallet::error]
	pub enum Error<T> {
		/// Number overflow in calculation.
		NumOverflow,
		/// Borrow rate is absurdly high.
		BorrowRateTooHigh,
		/// Feed price is invalid
		InvalidFeedPrice,
		/// Insufficient available liquidity.
		InsufficientLiquidity,
		/// Pool not found.
		PoolNotFound,
		/// Pool is already created
		PoolAlreadyCreated,
		/// Balance exceeds maximum value.
		/// Only happened when the balance went wrong and balance exceeds the integer type.
		BalanceOverflow,
		/// Collateral balance exceeds maximum value.
		CollateralBalanceOverflow,
		/// Borrow balance exceeds maximum value.
		BorrowBalanceOverflow,
		/// Protocol interest exceeds maximum value.
		ProtocolInterestOverflow,
		/// Maximum borrow rate cannot be set to 0.
		MaxBorrowRateCannotBeZero,
		/// Collateral factor must be in range (0..1].
		CollateralFactorIncorrectValue,
		/// Borrow cap is reached
		BorrowCapReached,
		/// Invalid borrow cap. Borrow cap must be in range [0..MAX_BORROW_CAP].
		InvalidBorrowCap,
		/// Utilization rate calculation error.
		UtilizationRateCalculationError,
		/// Hypothetical account liquidity calculation error.
		HypotheticalLiquidityCalculationError,
		/// The currency is not enabled in wrapped protocol.
		NotValidWrappedTokenId,
		/// The currency is not enabled in protocol.
		NotValidUnderlyingAssetId,
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(crate) fn deposit_event)]
	pub enum Event {
		/// InterestFactor has been successfully changed
		InterestFactorChanged,
		/// Max Borrow Rate has been successfully changed
		MaxBorrowRateChanged,
		/// Collateral factor has been successfully changed
		CollateralFactorChanged,
		/// The operation is paused: \[pool_id, operation\]
		OperationIsPaused(CurrencyId, Operation),
		/// The operation is unpaused: \[pool_id, operation\]
		OperationIsUnPaused(CurrencyId, Operation),
		/// Borrow cap changed: \[pool_id, new_cap\]
		BorrowCapChanged(CurrencyId, Option<Balance>),
		/// Protocol operation mode switched: \[is_whitelist_mode\]
		ProtocolOperationModeSwitched(bool),
		/// Protocol interest threshold changed: \[pool_id, new_value\]
		ProtocolInterestThresholdChanged(CurrencyId, Balance),
	}

	/// Controller data information: `(timestamp, protocol_interest_factor, collateral_factor,
	/// max_borrow_rate)`.
	#[pallet::storage]
	#[pallet::getter(fn controller_dates)]
	pub type ControllerParams<T: Config> =
		StorageMap<_, Twox64Concat, CurrencyId, ControllerData<T::BlockNumber>, ValueQuery>;

	/// The Pause Guardian can pause certain actions as a safety mechanism.
	#[pallet::storage]
	#[pallet::getter(fn pause_keepers)]
	pub(crate) type PauseKeepers<T: Config> =
		StorageMap<_, Twox64Concat, CurrencyId, PauseKeeper, ValueQuery, GetAllPaused>;

	/// Boolean variable. Protocol operation mode. In whitelist mode, only members
	/// 'WhitelistCouncil' can work with protocols.
	#[pallet::storage]
	#[pallet::getter(fn whitelist_mode)]
	pub type WhitelistMode<T: Config> = StorageValue<_, bool, ValueQuery>;

	#[pallet::genesis_config]
	pub struct GenesisConfig<T: Config> {
		#[allow(clippy::type_complexity)]
		pub controller_dates: Vec<(CurrencyId, ControllerData<T::BlockNumber>)>,
		pub pause_keepers: Vec<(CurrencyId, PauseKeeper)>,
		pub whitelist_mode: bool,
	}

	#[cfg(feature = "std")]
	impl<T: Config> Default for GenesisConfig<T> {
		fn default() -> Self {
			GenesisConfig {
				controller_dates: vec![],
				pause_keepers: vec![],
				whitelist_mode: false,
			}
		}
	}

	#[pallet::genesis_build]
	impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
		fn build(&self) {
			self.controller_dates.iter().for_each(|(currency_id, controller_data)| {
				ControllerParams::<T>::insert(currency_id, ControllerData { ..*controller_data })
			});
			self.pause_keepers.iter().for_each(|(currency_id, pause_keeper)| {
				PauseKeepers::<T>::insert(currency_id, PauseKeeper { ..*pause_keeper })
			});
			WhitelistMode::<T>::put(self.whitelist_mode);
		}
	}

	#[pallet::pallet]
	pub struct Pallet<T>(PhantomData<T>);

	#[pallet::hooks]
	impl<T: Config> Hooks<T::BlockNumber> for Pallet<T> {}

	// Admin functions
	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Pause specific operation (deposit, redeem, borrow, repay) with the pool.
		///
		/// The dispatch origin of this call must be 'UpdateOrigin'.
		#[pallet::weight(T::ControllerWeightInfo::pause_operation())]
		#[transactional]
		pub fn pause_operation(
			origin: OriginFor<T>,
			pool_id: CurrencyId,
			operation: Operation,
		) -> DispatchResultWithPostInfo {
			T::UpdateOrigin::ensure_origin(origin)?;
			ensure!(pool_id.is_supported_underlying_asset(), Error::<T>::PoolNotFound);
			ensure!(
				T::LiquidityPoolsManager::pool_exists(&pool_id),
				Error::<T>::PoolNotFound
			);

			PauseKeepers::<T>::mutate(pool_id, |pool| match operation {
				Operation::Deposit => pool.deposit_paused = true,
				Operation::Redeem => pool.redeem_paused = true,
				Operation::Borrow => pool.borrow_paused = true,
				Operation::Repay => pool.repay_paused = true,
				Operation::Transfer => pool.transfer_paused = true,
			});

			Self::deposit_event(Event::OperationIsPaused(pool_id, operation));
			Ok(().into())
		}

		/// Unpause specific operation (deposit, redeem, borrow, repay) with the pool.
		///
		/// The dispatch origin of this call must be 'UpdateOrigin'.
		#[pallet::weight(T::ControllerWeightInfo::resume_operation())]
		#[transactional]
		pub fn resume_operation(
			origin: OriginFor<T>,
			pool_id: CurrencyId,
			operation: Operation,
		) -> DispatchResultWithPostInfo {
			T::UpdateOrigin::ensure_origin(origin)?;
			ensure!(pool_id.is_supported_underlying_asset(), Error::<T>::PoolNotFound);
			ensure!(
				T::LiquidityPoolsManager::pool_exists(&pool_id),
				Error::<T>::PoolNotFound
			);

			PauseKeepers::<T>::mutate(pool_id, |pool| match operation {
				Operation::Deposit => pool.deposit_paused = false,
				Operation::Redeem => pool.redeem_paused = false,
				Operation::Borrow => pool.borrow_paused = false,
				Operation::Repay => pool.repay_paused = false,
				Operation::Transfer => pool.transfer_paused = false,
			});

			Self::deposit_event(Event::OperationIsUnPaused(pool_id, operation));
			Ok(().into())
		}

		/// Set interest factor.
		/// - `pool_id`: PoolID for which the parameter value is being set.
		/// - `protocol_interest_factor`: new value for interest factor.
		///
		/// The dispatch origin of this call must be 'UpdateOrigin'.
		#[pallet::weight(T::ControllerWeightInfo::set_protocol_interest_factor())]
		#[transactional]
		pub fn set_protocol_interest_factor(
			origin: OriginFor<T>,
			pool_id: CurrencyId,
			protocol_interest_factor: Rate,
		) -> DispatchResultWithPostInfo {
			T::UpdateOrigin::ensure_origin(origin)?;
			ensure!(pool_id.is_supported_underlying_asset(), Error::<T>::PoolNotFound);
			ensure!(
				T::LiquidityPoolsManager::pool_exists(&pool_id),
				Error::<T>::PoolNotFound
			);

			ControllerParams::<T>::mutate(pool_id, |data| data.protocol_interest_factor = protocol_interest_factor);
			Self::deposit_event(Event::InterestFactorChanged);
			Ok(().into())
		}

		/// Set Maximum borrow rate.
		/// - `pool_id`: PoolID for which the parameter value is being set.
		/// - `max_borrow_rate`: new value for maximum borrow rate.
		///
		/// The dispatch origin of this call must be 'UpdateOrigin'.
		#[pallet::weight(T::ControllerWeightInfo::set_max_borrow_rate())]
		#[transactional]
		pub fn set_max_borrow_rate(
			origin: OriginFor<T>,
			pool_id: CurrencyId,
			max_borrow_rate: Rate,
		) -> DispatchResultWithPostInfo {
			T::UpdateOrigin::ensure_origin(origin)?;
			ensure!(pool_id.is_supported_underlying_asset(), Error::<T>::PoolNotFound);
			ensure!(
				T::LiquidityPoolsManager::pool_exists(&pool_id),
				Error::<T>::PoolNotFound
			);
			ensure!(
				Self::is_valid_max_borrow_rate(max_borrow_rate),
				Error::<T>::MaxBorrowRateCannotBeZero
			);

			ControllerParams::<T>::mutate(pool_id, |data| data.max_borrow_rate = max_borrow_rate);
			Self::deposit_event(Event::MaxBorrowRateChanged);
			Ok(().into())
		}

		/// Set Collateral factor.
		/// - `pool_id`: PoolID for which the parameter value is being set.
		/// - `collateral_factor`: new value for collateral factor.
		///
		/// The dispatch origin of this call must be 'UpdateOrigin'.
		#[pallet::weight(T::ControllerWeightInfo::set_collateral_factor())]
		#[transactional]
		pub fn set_collateral_factor(
			origin: OriginFor<T>,
			pool_id: CurrencyId,
			collateral_factor: Rate,
		) -> DispatchResultWithPostInfo {
			T::UpdateOrigin::ensure_origin(origin)?;
			ensure!(pool_id.is_supported_underlying_asset(), Error::<T>::PoolNotFound);
			ensure!(
				T::LiquidityPoolsManager::pool_exists(&pool_id),
				Error::<T>::PoolNotFound
			);
			ensure!(
				Self::is_valid_collateral_factor(collateral_factor),
				Error::<T>::CollateralFactorIncorrectValue
			);

			ControllerParams::<T>::mutate(pool_id, |data| data.collateral_factor = collateral_factor);
			Self::deposit_event(Event::CollateralFactorChanged);
			Ok(().into())
		}

		/// Set borrow cap.
		///
		/// The dispatch origin of this call must be Administrator.
		/// Borrow cap value must be in range 0..1_000_000_000_000_000_000_000_000
		#[pallet::weight(T::ControllerWeightInfo::set_borrow_cap())]
		#[transactional]
		pub fn set_borrow_cap(
			origin: OriginFor<T>,
			pool_id: CurrencyId,
			borrow_cap: Option<Balance>,
		) -> DispatchResultWithPostInfo {
			T::UpdateOrigin::ensure_origin(origin)?;
			ensure!(pool_id.is_supported_underlying_asset(), Error::<T>::PoolNotFound);
			ensure!(
				T::LiquidityPoolsManager::pool_exists(&pool_id),
				Error::<T>::PoolNotFound
			);

			ensure!(Self::is_valid_borrow_cap(borrow_cap), Error::<T>::InvalidBorrowCap);
			ControllerParams::<T>::mutate(pool_id, |data| data.borrow_cap = borrow_cap);
			Self::deposit_event(Event::BorrowCapChanged(pool_id, borrow_cap));
			Ok(().into())
		}

		/// Set protocol interest threshold.
		///
		/// The dispatch origin of this call must be Administrator.
		#[pallet::weight(T::ControllerWeightInfo::set_protocol_interest_threshold())]
		#[transactional]
		pub fn set_protocol_interest_threshold(
			origin: OriginFor<T>,
			pool_id: CurrencyId,
			protocol_interest_threshold: Balance,
		) -> DispatchResultWithPostInfo {
			T::UpdateOrigin::ensure_origin(origin)?;
			ensure!(pool_id.is_supported_underlying_asset(), Error::<T>::PoolNotFound);
			ensure!(
				T::LiquidityPoolsManager::pool_exists(&pool_id),
				Error::<T>::PoolNotFound
			);

			ControllerParams::<T>::mutate(pool_id, |data| {
				data.protocol_interest_threshold = protocol_interest_threshold
			});
			Self::deposit_event(Event::ProtocolInterestThresholdChanged(
				pool_id,
				protocol_interest_threshold,
			));
			Ok(().into())
		}

		/// Enable / disable whitelist mode.
		///
		/// The dispatch origin of this call must be 'UpdateOrigin'.
		#[pallet::weight(T::ControllerWeightInfo::switch_whitelist_mode())]
		#[transactional]
		pub fn switch_whitelist_mode(origin: OriginFor<T>) -> DispatchResultWithPostInfo {
			T::UpdateOrigin::ensure_origin(origin)?;
			let mode = WhitelistMode::<T>::mutate(|mode| {
				*mode = !*mode;
				*mode
			});
			Self::deposit_event(Event::ProtocolOperationModeSwitched(mode));
			Ok(().into())
		}
	}
}

// RPC methods
impl<T: Config> Pallet<T> {
	/// Gets the exchange rate between a mToken and the underlying asset.
	pub fn get_liquidity_pool_exchange_rate(pool_id: CurrencyId) -> Option<Rate> {
		<LiquidityPools<T>>::get_exchange_rate(pool_id).ok()
	}

	/// Gets borrow interest rate and supply interest rate.
	pub fn get_liquidity_pool_borrow_and_supply_rates(pool_id: CurrencyId) -> Option<(Rate, Rate)> {
		if !<LiquidityPools<T>>::pool_exists(&pool_id) {
			return None;
		}
		let current_total_balance = T::LiquidityPoolsManager::get_pool_available_liquidity(pool_id);
		let block_delta = Self::get_block_delta(pool_id).ok()?;
		let pool_data = Self::calculate_interest_params(pool_id, block_delta).ok()?;
		let protocol_interest_factor = Self::controller_dates(pool_id).protocol_interest_factor;

		let utilization_rate = Self::calculate_utilization_rate(
			current_total_balance,
			pool_data.total_borrowed,
			pool_data.total_protocol_interest,
		)
		.ok()?;

		let borrow_rate = <MinterestModel<T>>::calculate_borrow_interest_rate(pool_id, utilization_rate).ok()?;

		// supply_interest_rate = utilization_rate * borrow_rate * (1 - protocol_interest_factor)
		let supply_rate = Rate::one()
			.checked_sub(&protocol_interest_factor)
			.and_then(|v| v.checked_mul(&borrow_rate))
			.and_then(|v| v.checked_mul(&utilization_rate))
			.ok_or(Error::<T>::NumOverflow)
			.ok()?;

		Some((borrow_rate, supply_rate))
	}

	/// Gets current utilization rate of the pool.
	pub fn get_utilization_rate(pool_id: CurrencyId) -> Option<Rate> {
		let current_block_number = <frame_system::Module<T>>::block_number();
		let accrual_block_number_previous = Self::controller_dates(pool_id).last_interest_accrued_block;
		let block_delta = Self::calculate_block_delta(current_block_number, accrual_block_number_previous).ok()?;
		let pool_data = Self::calculate_interest_params(pool_id, block_delta).ok()?;
		let current_total_balance = T::LiquidityPoolsManager::get_pool_available_liquidity(pool_id);
		Self::calculate_utilization_rate(
			current_total_balance,
			pool_data.total_borrowed,
			pool_data.total_protocol_interest,
		)
		.ok()
	}

	/// Calculates total supply and total borrowed balance in usd based on
	/// total_borrowed, total_protocol_interest, borrow_index values calculated for current block
	pub fn get_user_total_supply_and_borrowed_usd_balance(
		who: &T::AccountId,
	) -> result::Result<(Balance, Balance), DispatchError> {
		let total_supply_and_borrow_in_usd = Self::get_vec_of_supply_and_borrow_balance(who)?;

		let (total_supply_balance, total_borrowed_balance) = total_supply_and_borrow_in_usd.into_iter().try_fold(
			(Balance::zero(), Balance::zero()),
			|(total_supply, total_borrow),
			 (_, supply_balance, borrow_balance)|
			 -> result::Result<(Balance, Balance), DispatchError> {
				Ok((
					total_supply
						.checked_add(supply_balance)
						.ok_or(Error::<T>::NumOverflow)?,
					total_borrow
						.checked_add(borrow_balance)
						.ok_or(Error::<T>::NumOverflow)?,
				))
			},
		)?;

		Ok((total_supply_balance, total_borrowed_balance))
	}

	/// Calculate total collateral in usd based on collateral factor, fresh exchange rate and latest
	/// oracle price.
	///
	/// - `who`: the AccountId whose collateral should be calculated.
	pub fn get_user_total_collateral(who: T::AccountId) -> BalanceResult {
		CurrencyId::get_enabled_tokens_in_protocol(UnderlyingAsset)
			.iter()
			.filter(|&underlying_id| T::LiquidityPoolsManager::pool_exists(underlying_id))
			.filter(|&pool_id| <LiquidityPools<T>>::check_user_available_collateral(&who, *pool_id))
			.try_fold(Balance::zero(), |acc, &pool_id| -> BalanceResult {
				let user_supply_balance = Self::get_user_supply_per_asset(&who, pool_id)?;

				if user_supply_balance.is_zero() {
					return Ok(Balance::zero());
				}

				let collateral_factor = Self::controller_dates(pool_id).collateral_factor;

				let oracle_price = T::PriceSource::get_underlying_price(pool_id).ok_or(Error::<T>::InvalidFeedPrice)?;

				let collateral_in_usd = Rate::from_inner(user_supply_balance)
					.checked_mul(&oracle_price)
					.and_then(|x| x.checked_mul(&collateral_factor))
					.map(|x| x.into_inner())
					.ok_or(Error::<T>::NumOverflow)?;

				Ok(acc + collateral_in_usd)
			})
	}

	/// Calculate actual borrow balance for user per asset based on fresh latest indexes.
	///
	/// - `who`: the AccountId whose balance should be calculated.
	/// - `underlying_asset_id`: ID of the currency, the balance of borrowing of which we calculate.
	pub fn get_user_borrow_per_asset(who: &T::AccountId, underlying_asset_id: CurrencyId) -> BalanceResult {
		ensure!(
			<LiquidityPools<T>>::pool_exists(&underlying_asset_id),
			Error::<T>::PoolNotFound
		);
		ensure!(
			underlying_asset_id.is_supported_underlying_asset(),
			Error::<T>::NotValidUnderlyingAssetId
		);

		if <LiquidityPools<T>>::get_user_total_borrowed(&who, underlying_asset_id).is_zero() {
			return Ok(Balance::zero());
		};

		let block_delta = Self::get_block_delta(underlying_asset_id)?;
		let pool_data = Self::calculate_interest_params(underlying_asset_id, block_delta)?;
		let borrow_balance = Self::calculate_borrow_balance(&who, underlying_asset_id, pool_data.borrow_index)?;
		Ok(borrow_balance)
	}

	/// Calculate total user's supply APY, borrow APY and Net APY.
	///
	/// - `who`: the AccountId whose APY should be calculated.
	pub fn get_user_supply_borrow_and_net_apy(
		who: T::AccountId,
	) -> Result<(Interest, Interest, Interest), DispatchError> {
		// Get vector of balances for each asset(in USD).
		let supply_and_borrow_per_asset = Self::get_vec_of_supply_and_borrow_balance(&who)?;

		// Calculate total supply and borrow balance for current user for all pools.
		let (total_supply, total_borrow) = supply_and_borrow_per_asset.iter().fold(
			(Balance::zero(), Balance::zero()),
			|(total_supply, total_borrow), &(_, supply_balance, borrow_balance)| -> (Balance, Balance) {
				(total_supply + supply_balance, total_borrow + borrow_balance)
			},
		);

		if total_supply.is_zero() {
			return Ok((Interest::zero(), Interest::zero(), Interest::zero()));
		}

		// Calculate expected interest on all supplies and borrows for particular user on all pools;
		// Calculate indicator for NET APY formula
		let (total_supply_interest, total_borrow_interest, net_apy_indicator) =
			supply_and_borrow_per_asset.into_iter().try_fold(
				(Interest::zero(), Interest::zero(), Interest::zero()),
				|(sum_supply_interest, sum_borrow_interest, sum_net_apy_dividend),
				 (pool_id, current_supply_in_usd, current_borrow_in_usd)|
				 -> result::Result<(Interest, Interest, Interest), DispatchError> {
					if current_supply_in_usd.is_zero() && current_borrow_in_usd.is_zero() {
						return Ok((sum_supply_interest, sum_borrow_interest, sum_net_apy_dividend));
					}

					let (borrow_rate, supply_rate) =
						Self::get_liquidity_pool_borrow_and_supply_rates(pool_id).ok_or(Error::<T>::NumOverflow)?;

					let calculate_interest = |amount: Balance, rate: Balance| {
						Interest::from_inner(amount as Amount)
							.checked_mul(&Interest::from_inner(rate as Amount))
							.ok_or(Error::<T>::NumOverflow)
					};

					let supply_interest = calculate_interest(current_supply_in_usd, supply_rate.into_inner())?;
					let borrow_interest = calculate_interest(current_borrow_in_usd, borrow_rate.into_inner())?;

					// Calculate mnt interest for both borrow and supply.
					let (mnt_borrow_rate, mnt_supply_rate) = T::MntManager::get_mnt_borrow_and_supply_rates(pool_id)?;

					let mnt_supply_inerest = calculate_interest(current_supply_in_usd, mnt_supply_rate.into_inner())?;
					let mnt_borrow_inerest = calculate_interest(current_borrow_in_usd, mnt_borrow_rate.into_inner())?;

					// Calculate NET API indicator:
					// net_apy_indicator = supply_interest - borrow_interest + mnt_supply_interest + mnt_borrow_inerest
					let current_net_apy_indicator = supply_interest
						.checked_sub(&borrow_interest)
						.and_then(|v| v.checked_add(&mnt_supply_inerest))
						.and_then(|v| v.checked_add(&mnt_borrow_inerest))
						.ok_or(Error::<T>::NumOverflow)?;

					Ok((
						sum_supply_interest + supply_interest,
						sum_borrow_interest + borrow_interest,
						sum_net_apy_dividend + current_net_apy_indicator,
					))
				},
			)?;

		// Calculate APY given the amount of blocks per year.
		// supply_apy = total_supply_interest / total_supply * BlocksPerYear
		// borrow_apy = total_borrow_interest / total_borrow * BlocksPerYear
		let calculate_apy = |interest: Interest, amount: Balance| {
			interest
				.checked_div(&Interest::from_inner(amount as Amount))
				.and_then(|v| v.checked_mul(&Interest::from_inner(T::BlocksPerYear::get() as Amount)))
				.ok_or(Error::<T>::NumOverflow)
		};

		let supply_apy = calculate_apy(total_supply_interest, total_supply)?;
		let borrow_apy = calculate_apy(total_borrow_interest, total_borrow)?;

		// Calculate NET APY:
		// if net_apy_indicator > 0
		//		net_apy = net_apy_indicator / total_supply * BlocksPerYear
		// if net_apy_indicator < 0
		//		net_apy = net_apy_indicator / total_borrow * BlocksPerYear

		let net_apy = match net_apy_indicator {
			net_apy_indicator if net_apy_indicator > Interest::zero() => {
				calculate_apy(net_apy_indicator, total_supply)?
			}
			net_apy_indicator if net_apy_indicator < Interest::zero() => {
				calculate_apy(net_apy_indicator, total_borrow)?
			}
			_ => Interest::zero(),
		};

		Ok((supply_apy, borrow_apy, net_apy))
	}
}

// Private methods
impl<T: Config> Pallet<T> {
	/// Checks if borrow cap is reached.
	///
	/// Return true if total borrow per pool will exceed borrow cap, otherwise false.
	fn is_borrow_cap_reached(pool_id: CurrencyId, borrow_amount: Balance) -> Result<bool, DispatchError> {
		if let Some(borrow_cap) = Self::controller_dates(pool_id).borrow_cap {
			let oracle_price = T::PriceSource::get_underlying_price(pool_id).ok_or(Error::<T>::InvalidFeedPrice)?;
			let pool_total_borrowed = T::LiquidityPoolsManager::get_pool_total_borrowed(pool_id);

			// new_total_borrows_in_usd = (pool_total_borrowed + borrow_amount) * oracle_price
			let new_total_borrows = pool_total_borrowed
				.checked_add(borrow_amount)
				.ok_or(Error::<T>::BalanceOverflow)?;

			let new_total_borrows_in_usd = Rate::from_inner(new_total_borrows)
				.checked_mul(&oracle_price)
				.map(|x| x.into_inner())
				.ok_or(Error::<T>::BalanceOverflow)?;

			Ok(new_total_borrows_in_usd >= borrow_cap)
		} else {
			Ok(false)
		}
	}

	/// Return the borrow balance of account based on pool_borrow_index calculated beforehand.
	///
	/// - `who`: The address whose balance should be calculated.
	/// - `underlying_asset`: ID of the currency, the balance of borrowing of which we calculate.
	/// - `pool_borrow_index`: borrow index for the pool
	fn calculate_borrow_balance(
		who: &T::AccountId,
		underlying_asset: CurrencyId,
		pool_borrow_index: Rate,
	) -> BalanceResult {
		let user_borrow_balance = <LiquidityPools<T>>::get_user_total_borrowed(&who, underlying_asset);

		// If borrow_balance = 0 then borrow_index is likely also 0.
		// Rather than failing the calculation with a division by 0, we immediately return 0 in this case.
		if user_borrow_balance.is_zero() {
			return Ok(Balance::zero());
		};

		let user_borrow_index = <LiquidityPools<T>>::get_user_borrow_index(&who, underlying_asset);

		// Calculate new borrow balance using the borrow index:
		// recent_borrow_balance = user_borrow_balance * pool_borrow_index / user_borrow_index
		let recent_borrow_balance = Rate::from_inner(user_borrow_balance)
			.checked_mul(&pool_borrow_index)
			.and_then(|v| v.checked_div(&user_borrow_index))
			.map(|x| x.into_inner())
			.ok_or(Error::<T>::BorrowBalanceOverflow)?;

		Ok(recent_borrow_balance)
	}

	/// Calculates total borrows, total protocol interest and borrow index for given pool.
	/// Applies accrued interest to total borrows and protocol interest and calculates interest
	/// accrued from the last checkpointed block up to the current block and writes new checkpoint
	/// to storage.
	///
	/// - `underlying_asset`: ID of the currency to make calculations for.
	/// - `block_delta`: number of blocks passed since last accrue interest
	fn calculate_interest_params(
		underlying_asset: CurrencyId,
		block_delta: T::BlockNumber,
	) -> result::Result<Pool, DispatchError> {
		let current_total_balance = T::LiquidityPoolsManager::get_pool_available_liquidity(underlying_asset);
		let pool_data = <LiquidityPools<T>>::get_pool_data(underlying_asset);

		let utilization_rate = Self::calculate_utilization_rate(
			current_total_balance,
			pool_data.total_borrowed,
			pool_data.total_protocol_interest,
		)?;

		// Calculate the current borrow interest rate
		let current_borrow_interest_rate =
			<MinterestModel<T>>::calculate_borrow_interest_rate(underlying_asset, utilization_rate)?;

		let ControllerData {
			max_borrow_rate,
			protocol_interest_factor,
			..
		} = Self::controller_dates(underlying_asset);

		ensure!(
			current_borrow_interest_rate <= max_borrow_rate,
			Error::<T>::BorrowRateTooHigh
		);

		/*
		Calculate the interest accumulated into borrows and protocol interest and the new index:
			*  simple_interest_factor = borrow_rate * block_delta
			*  interest_accumulated = simple_interest_factor * total_borrows
			*  total_borrows_new = interest_accumulated + total_borrows
			*  total_protocol_interest_new = interest_accumulated * protocol_interest_factor + total_protocol_interest
			*  borrow_index_new = simple_interest_factor * borrow_index + borrow_index
		*/

		let simple_interest_factor = Self::calculate_interest_factor(current_borrow_interest_rate, block_delta)?;

		let interest_accumulated = Rate::from_inner(pool_data.total_borrowed)
			.checked_mul(&simple_interest_factor)
			.map(|x| x.into_inner())
			.ok_or(Error::<T>::BalanceOverflow)?;

		let total_borrowed = interest_accumulated
			.checked_add(pool_data.total_borrowed)
			.ok_or(Error::<T>::BorrowBalanceOverflow)?;

		let total_protocol_interest = sum_with_mult_result(
			pool_data.total_protocol_interest,
			interest_accumulated,
			protocol_interest_factor,
		)
		.map_err(|_| Error::<T>::ProtocolInterestOverflow)?;

		let borrow_index = simple_interest_factor
			.checked_mul(&pool_data.borrow_index)
			.and_then(|v| v.checked_add(&pool_data.borrow_index))
			.ok_or(Error::<T>::NumOverflow)?;

		Ok(Pool {
			total_borrowed,
			borrow_index,
			total_protocol_interest,
		})
	}

	/// Calculates the utilization rate of the pool.
	/// - `current_total_balance`: The amount of cash in the pool.
	/// - `current_total_borrowed_balance`: The amount of borrows in the pool.
	/// - `current_total_protocol_interest`: The amount of interest in the pool (currently unused).
	///
	/// returns `utilization_rate =
	///  total_borrows / (total_cash + total_borrows - total_protocol_interest)`
	fn calculate_utilization_rate(
		current_total_balance: Balance,
		current_total_borrowed_balance: Balance,
		current_total_protocol_interest: Balance,
	) -> RateResult {
		// Utilization rate is 0 when there are no borrows
		if current_total_borrowed_balance.is_zero() {
			return Ok(Rate::zero());
		}

		// utilization_rate = current_total_borrowed_balance / (current_total_balance +
		// + current_total_borrowed_balance - current_total_protocol_interest)
		let utilization_rate = Rate::checked_from_rational(
			current_total_borrowed_balance,
			current_total_balance
				.checked_add(current_total_borrowed_balance)
				.and_then(|v| v.checked_sub(current_total_protocol_interest))
				.ok_or(Error::<T>::UtilizationRateCalculationError)?,
		)
		.ok_or(Error::<T>::UtilizationRateCalculationError)?;

		Ok(utilization_rate)
	}

	/// Calculates the number of blocks elapsed since the last accrual.
	/// - `current_block_number`: Current block number.
	/// - `accrual_block_number_previous`: Number of the last block with accruals.
	///
	/// returns `current_block_number - accrual_block_number_previous`
	fn calculate_block_delta(
		current_block_number: T::BlockNumber,
		accrual_block_number_previous: T::BlockNumber,
	) -> result::Result<T::BlockNumber, DispatchError> {
		ensure!(
			current_block_number >= accrual_block_number_previous,
			Error::<T>::NumOverflow
		);

		Ok(current_block_number - accrual_block_number_previous)
	}

	/// Calculates the simple interest factor.
	/// - `current_borrow_interest_rate`: Current interest rate that users pay for lending assets.
	/// - `block_delta`: The number of blocks elapsed since the last accrual.
	///
	/// returns `interest_factor = current_borrow_interest_rate * block_delta`.
	fn calculate_interest_factor(current_borrow_interest_rate: Rate, block_delta: T::BlockNumber) -> RateResult {
		let block_delta_as_usize = TryInto::<usize>::try_into(block_delta)
			.ok()
			.expect("blockchain will not exceed 2^32 blocks; qed");

		let interest_factor: FixedU128 = Rate::saturating_from_integer(block_delta_as_usize as u128)
			.checked_mul(&current_borrow_interest_rate)
			.ok_or(Error::<T>::NumOverflow)?;

		Ok(interest_factor)
	}

	/// Calculate actual supply balance for user per asset based on fresh exchange rate.
	///
	/// - `who`: the AccountId whose balance should be calculated.
	/// - `underlying_asset_id`: ID of the currency, the balance of supplying of which we calculate.
	fn get_user_supply_per_asset(who: &T::AccountId, underlying_asset_id: CurrencyId) -> BalanceResult {
		let wrapped_id = underlying_asset_id.wrapped_asset().ok_or(Error::<T>::PoolNotFound)?;

		let user_balance_wrapped_tokens = T::MultiCurrency::free_balance(wrapped_id, &who);

		if user_balance_wrapped_tokens.is_zero() {
			return Ok(Balance::zero());
		}

		let block_delta = Self::get_block_delta(underlying_asset_id)?;
		let pool_data = Self::calculate_interest_params(underlying_asset_id, block_delta)?;
		let current_exchange_rate = <LiquidityPools<T>>::get_exchange_rate_by_interest_params(
			underlying_asset_id,
			pool_data.total_protocol_interest,
			pool_data.total_borrowed,
		)?;

		let supply_balance = Rate::from_inner(user_balance_wrapped_tokens)
			.checked_mul(&current_exchange_rate)
			.map(|x| x.into_inner())
			.ok_or(Error::<T>::NumOverflow)?;
		Ok(supply_balance)
	}

	/// Create a vector that contains total supply and total borrowed balance in usd for each pool
	/// based on freshest data.
	///
	/// - `who`: the AccountId whose APYs should be calculated.
	/// Returns: Vec<(CurrencyId, supply_balance, borrow_balance)>
	fn get_vec_of_supply_and_borrow_balance(
		who: &T::AccountId,
	) -> result::Result<Vec<(CurrencyId, Balance, Balance)>, DispatchError> {
		CurrencyId::get_enabled_tokens_in_protocol(UnderlyingAsset)
			.into_iter()
			.try_fold(
				Vec::<(CurrencyId, Balance, Balance)>::new(),
				|mut vec, pool_id| -> result::Result<Vec<(CurrencyId, Balance, Balance)>, DispatchError> {
					// Get supply and borrow balance per asset.
					let mut supply_balance = Self::get_user_supply_per_asset(&who, pool_id)?;
					let mut borrow_balance = Self::get_user_borrow_per_asset(&who, pool_id)?;

					// Skip this pool if there is nothing to calculate.
					if supply_balance.is_zero() && borrow_balance.is_zero() {
						vec.push((pool_id, Balance::zero(), Balance::zero()));
						return Ok(vec);
					}

					let oracle_price =
						T::PriceSource::get_underlying_price(pool_id).ok_or(Error::<T>::InvalidFeedPrice)?;

					// TODO: refactor and optimize this is_zero() checks

					// Convert to USD if balance exists.
					if !supply_balance.is_zero() {
						supply_balance = Rate::from_inner(supply_balance)
							.checked_mul(&oracle_price)
							.map(|x| x.into_inner())
							.ok_or(Error::<T>::BalanceOverflow)?;
					}
					if !borrow_balance.is_zero() {
						borrow_balance = Rate::from_inner(borrow_balance)
							.checked_mul(&oracle_price)
							.map(|x| x.into_inner())
							.ok_or(Error::<T>::BalanceOverflow)?;
					}

					vec.push((pool_id, supply_balance, borrow_balance));
					Ok(vec)
				},
			)
	}

	/// Calculate the delta of blocks from the last interest accrued to the current.
	///
	/// - `underlying_asset_id`: ID of the currency, the blocks delta of which we calculate.
	fn get_block_delta(underlying_asset_id: CurrencyId) -> result::Result<T::BlockNumber, DispatchError> {
		let current_block_number = <frame_system::Module<T>>::block_number();
		let accrual_block_number_previous = Self::controller_dates(underlying_asset_id).last_interest_accrued_block;
		Self::calculate_block_delta(current_block_number, accrual_block_number_previous)
	}

	fn is_valid_max_borrow_rate(max_borrow_rate: Rate) -> bool {
		!max_borrow_rate.is_zero()
	}

	fn is_valid_collateral_factor(collateral_factor: Rate) -> bool {
		!collateral_factor.is_zero() && collateral_factor <= Rate::one()
	}

	fn is_valid_borrow_cap(borrow_cap: Option<Balance>) -> bool {
		match borrow_cap {
			Some(cap) => cap >= Balance::zero() && cap <= T::MaxBorrowCap::get(),
			None => true,
		}
	}
}

impl<T: Config> ControllerManager<T::AccountId> for Pallet<T> {
	/// This is a part of a pool creation flow
	/// Creates storage records for ControllerParams and PauseKeepers
	/// All operations are unpaused after this function call
	fn create_pool(
		currency_id: CurrencyId,
		protocol_interest_factor: Rate,
		max_borrow_rate: Rate,
		collateral_factor: Rate,
		protocol_interest_threshold: Balance,
	) -> DispatchResult {
		ensure!(
			!ControllerParams::<T>::contains_key(currency_id),
			Error::<T>::PoolAlreadyCreated
		);
		ensure!(
			Self::is_valid_max_borrow_rate(max_borrow_rate),
			Error::<T>::MaxBorrowRateCannotBeZero
		);
		ensure!(
			Self::is_valid_collateral_factor(collateral_factor),
			Error::<T>::CollateralFactorIncorrectValue
		);

		ControllerParams::<T>::insert(
			currency_id,
			ControllerData {
				last_interest_accrued_block: <frame_system::Module<T>>::block_number(),
				protocol_interest_factor,
				max_borrow_rate,
				collateral_factor,
				borrow_cap: None,
				protocol_interest_threshold,
			},
		);
		PauseKeepers::<T>::insert(
			currency_id,
			PauseKeeper {
				deposit_paused: false,
				redeem_paused: false,
				borrow_paused: false,
				repay_paused: false,
				transfer_paused: false,
			},
		);
		Ok(())
	}

	/// Return the borrow balance of account based on stored data.
	///
	/// - `who`: The address whose balance should be calculated.
	/// - `currency_id`: ID of the currency, the balance of borrowing of which we calculate.
	fn borrow_balance_stored(who: &T::AccountId, underlying_asset_id: CurrencyId) -> BalanceResult {
		let pool_borrow_index = T::LiquidityPoolsManager::get_pool_borrow_index(underlying_asset_id);
		let borrow_balance = Self::calculate_borrow_balance(who, underlying_asset_id, pool_borrow_index)?;
		Ok(borrow_balance)
	}

	/// Determine what the account liquidity would be if the given amounts were redeemed/borrowed.
	///
	/// - `account`: The account to determine liquidity.
	/// - `underlying_asset`: The pool to hypothetically redeem/borrow.
	/// - `redeem_amount`: The number of tokens to hypothetically redeem.
	/// - `borrow_amount`: The amount of underlying to hypothetically borrow.
	/// Returns (hypothetical account liquidity in excess of collateral requirements,
	///          hypothetical account shortfall below collateral requirements).
	fn get_hypothetical_account_liquidity(
		account: &T::AccountId,
		underlying_to_borrow: CurrencyId,
		redeem_amount: Balance,
		borrow_amount: Balance,
	) -> LiquidityResult {
		let m_tokens_ids: Vec<CurrencyId> = CurrencyId::get_enabled_tokens_in_protocol(WrappedToken);

		let mut sum_collateral = Balance::zero();
		let mut sum_borrow_plus_effects = Balance::zero();

		// For each tokens the account is in
		for asset in m_tokens_ids.into_iter() {
			let underlying_asset = asset.underlying_asset().ok_or(Error::<T>::NotValidWrappedTokenId)?;
			if !T::LiquidityPoolsManager::pool_exists(&underlying_asset) {
				continue;
			}

			// Read the balances and exchange rate from the cToken
			let borrow_balance = Self::borrow_balance_stored(account, underlying_asset)?;
			let exchange_rate = <LiquidityPools<T>>::get_exchange_rate(underlying_asset)?;
			let collateral_factor = Self::controller_dates(underlying_asset).collateral_factor;

			// Get the normalized price of the asset.
			let oracle_price =
				T::PriceSource::get_underlying_price(underlying_asset).ok_or(Error::<T>::InvalidFeedPrice)?;

			// Pre-compute a conversion factor from tokens -> dollars (normalized price value)
			// tokens_to_denom = collateral_factor * exchange_rate * oracle_price
			let tokens_to_denom = collateral_factor
				.checked_mul(&exchange_rate)
				.and_then(|v| v.checked_mul(&oracle_price))
				.ok_or(Error::<T>::NumOverflow)?;

			if <LiquidityPools<T>>::check_user_available_collateral(&account, underlying_asset) {
				let m_token_balance = T::MultiCurrency::free_balance(asset, account);

				// sum_collateral += tokens_to_denom * m_token_balance
				sum_collateral = sum_with_mult_result(sum_collateral, m_token_balance, tokens_to_denom)
					.map_err(|_| Error::<T>::CollateralBalanceOverflow)?;
			}

			// sum_borrow_plus_effects += oracle_price * borrow_balance
			sum_borrow_plus_effects = sum_with_mult_result(sum_borrow_plus_effects, borrow_balance, oracle_price)
				.map_err(|_| Error::<T>::BalanceOverflow)?;

			// Calculate effects of interacting with Underlying Asset Modify.
			if underlying_to_borrow == underlying_asset {
				// redeem effect
				if redeem_amount > 0 {
					// sum_borrow_plus_effects += tokens_to_denom * redeem_tokens
					sum_borrow_plus_effects =
						sum_with_mult_result(sum_borrow_plus_effects, redeem_amount, tokens_to_denom)
							.map_err(|_| Error::<T>::BalanceOverflow)?;
				};
				// borrow effect
				if borrow_amount > 0 {
					// sum_borrow_plus_effects += oracle_price * borrow_amount
					sum_borrow_plus_effects =
						sum_with_mult_result(sum_borrow_plus_effects, borrow_amount, oracle_price)
							.map_err(|_| Error::<T>::BalanceOverflow)?;
				}
			}
		}

		match sum_collateral.cmp(&sum_borrow_plus_effects) {
			Ordering::Less => Ok((
				0,
				sum_borrow_plus_effects
					.checked_sub(sum_collateral)
					.ok_or(Error::<T>::InsufficientLiquidity)?,
			)),
			_ => Ok((
				sum_collateral
					.checked_sub(sum_borrow_plus_effects)
					.ok_or(Error::<T>::InsufficientLiquidity)?,
				0,
			)),
		}
	}

	/// Applies accrued interest to total borrows and protocol interest.
	/// This calculates interest accrued from the last checkpointed block
	/// up to the current block and writes new checkpoint to storage.
	fn accrue_interest_rate(underlying_asset: CurrencyId) -> DispatchResult {
		//Remember the initial block number.
		let current_block_number = <frame_system::Module<T>>::block_number();
		let accrual_block_number_previous = Self::controller_dates(underlying_asset).last_interest_accrued_block;
		// Calculate the number of blocks elapsed since the last accrual
		let block_delta = Self::calculate_block_delta(current_block_number, accrual_block_number_previous)?;
		//Short-circuit accumulating 0 interest.
		if block_delta == T::BlockNumber::zero() {
			return Ok(());
		}

		let pool_data = Self::calculate_interest_params(underlying_asset, block_delta)?;
		// Save new params
		ControllerParams::<T>::mutate(underlying_asset, |data| {
			data.last_interest_accrued_block = current_block_number
		});
		<LiquidityPools<T>>::set_pool_data(
			underlying_asset,
			pool_data.total_borrowed,
			pool_data.borrow_index,
			pool_data.total_protocol_interest,
		)?;

		Ok(())
	}

	/// Checks if a specific operation is allowed on a pool.
	///
	/// Return true - if operation is allowed, false - if operation is unallowed.
	fn is_operation_allowed(pool_id: CurrencyId, operation: Operation) -> bool {
		match operation {
			Operation::Deposit => !Self::pause_keepers(pool_id).deposit_paused,
			Operation::Redeem => !Self::pause_keepers(pool_id).redeem_paused,
			Operation::Borrow => !Self::pause_keepers(pool_id).borrow_paused,
			Operation::Repay => !Self::pause_keepers(pool_id).repay_paused,
			Operation::Transfer => !Self::pause_keepers(pool_id).transfer_paused,
		}
	}

	/// Checks if the account should be allowed to redeem tokens in the given pool.
	///
	/// - `underlying_asset` - The CurrencyId to verify the redeem against.
	/// - `redeemer` -  The account which would redeem the tokens.
	/// - `redeem_amount` - The number of mTokens to exchange for the underlying asset in the
	/// pool.
	///
	/// Return Ok if the redeem is allowed.
	fn redeem_allowed(underlying_asset: CurrencyId, redeemer: &T::AccountId, redeem_amount: Balance) -> DispatchResult {
		if LiquidityPools::<T>::check_user_available_collateral(&redeemer, underlying_asset) {
			let (_, shortfall) =
				Self::get_hypothetical_account_liquidity(&redeemer, underlying_asset, redeem_amount, 0)
					.map_err(|_| Error::<T>::HypotheticalLiquidityCalculationError)?;

			ensure!(shortfall.is_zero(), Error::<T>::InsufficientLiquidity);
		}
		Ok(())
	}

	/// Checks if the account should be allowed to borrow the underlying asset of the given pool.
	///
	/// - `underlying_asset` - The CurrencyId to verify the borrow against.
	/// - `who` -  The account which would borrow the asset.
	/// - `borrow_amount` - The amount of underlying assets the account would borrow.
	///
	/// Return Ok if the borrow is allowed.
	fn borrow_allowed(underlying_asset: CurrencyId, who: &T::AccountId, borrow_amount: Balance) -> DispatchResult {
		let borrow_cap_reached = Self::is_borrow_cap_reached(underlying_asset, borrow_amount)?;
		ensure!(!borrow_cap_reached, Error::<T>::BorrowCapReached);

		let (_, shortfall) = Self::get_hypothetical_account_liquidity(&who, underlying_asset, 0, borrow_amount)
			.map_err(|_| Error::<T>::HypotheticalLiquidityCalculationError)?;

		ensure!(shortfall.is_zero(), Error::<T>::InsufficientLiquidity);

		Ok(())
	}

	/// Return minimum protocol interest needed to transfer it to liquidation pool
	fn get_protocol_interest_threshold(pool_id: CurrencyId) -> Balance {
		Self::controller_dates(pool_id).protocol_interest_threshold
	}

	/// Protocol operation mode. In whitelist mode, only members 'WhitelistCouncil' can work with
	/// protocols.
	fn is_whitelist_mode_enabled() -> bool {
		WhitelistMode::<T>::get()
	}
}
