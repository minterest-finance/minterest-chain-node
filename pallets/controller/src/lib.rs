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
use minterest_primitives::{
	arithmetic::sum_with_mult_result,
	currency::CurrencyType::{UnderlyingAsset, WrappedToken},
};
use minterest_primitives::{Balance, CurrencyId, Operation, Rate};
pub use module::*;
use orml_traits::MultiCurrency;
use pallet_traits::{ControllerManager, LiquidityPoolsManager, MinterestModelManager, PoolsManager, PricesManager};
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};
use sp_runtime::traits::CheckedSub;
use sp_runtime::{
	traits::{CheckedAdd, CheckedDiv, CheckedMul, Zero},
	DispatchError, DispatchResult, FixedPointNumber, FixedU128, RuntimeDebug,
};
use sp_std::{cmp::Ordering, convert::TryInto, prelude::Vec, result};
pub use weights::WeightInfo;
#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;
pub mod weights;

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
	/// loans. For instance, 0.9 allows 90% of total pool value to be used as a collateral. Must be
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
type RateResult = result::Result<Rate, DispatchError>;
type BalanceResult = result::Result<Balance, DispatchError>;
type LiquidityResult = result::Result<(Balance, Balance), DispatchError>;

#[frame_support::pallet]
pub mod module {
	use super::*;

	#[pallet::config]
	pub trait Config: frame_system::Config + liquidity_pools::Config {
		/// The overarching event type.
		type Event: From<Event> + IsType<<Self as frame_system::Config>::Event>;

		/// Provides the basic liquidity pools manager and liquidity pool functionality.
		type LiquidityPoolsManager: LiquidityPoolsManager<Self::AccountId>;

		/// Provides the basic minterest model functionality.
		type MinterestModelManager: MinterestModelManager;

		#[pallet::constant]
		/// Maximum total borrow amount per pool in usd.
		type MaxBorrowCap: Get<Balance>;

		/// The origin which may update controller parameters. Root or
		/// Half Minterest Council can always do this.
		type UpdateOrigin: EnsureOrigin<Self::Origin>;

		/// Weight information for the extrinsics.
		type ControllerWeightInfo: WeightInfo;
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

	#[pallet::genesis_config]
	pub struct GenesisConfig<T: Config> {
		#[allow(clippy::type_complexity)]
		pub controller_dates: Vec<(CurrencyId, ControllerData<T::BlockNumber>)>,
		pub pause_keepers: Vec<(CurrencyId, PauseKeeper)>,
	}

	#[cfg(feature = "std")]
	impl<T: Config> Default for GenesisConfig<T> {
		fn default() -> Self {
			GenesisConfig {
				controller_dates: vec![],
				pause_keepers: vec![],
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
	}
}

// RPC methods
impl<T: Config> Pallet<T> {
	/// Gets exchange, borrow interest rate and supply interest rate. The rates is calculated
	/// for the current block.
	pub fn get_pool_exchange_borrow_and_supply_rates(pool_id: CurrencyId) -> Option<(Rate, Rate, Rate)> {
		if !<LiquidityPools<T>>::pool_exists(&pool_id) {
			return None;
		}
		let pool_data: Pool = Self::calculate_current_pool_data(pool_id).ok()?;
		let protocol_interest_factor: Rate = Self::controller_dates(pool_id).protocol_interest_factor;
		let utilization_rate: Rate = Self::get_utilization_rate(pool_id)?;
		let exchange_rate: Rate = <LiquidityPools<T>>::get_exchange_rate_by_interest_params(
			pool_id,
			pool_data.total_protocol_interest,
			pool_data.total_borrowed,
		)
		.ok()?;
		let borrow_rate: Rate =
			T::MinterestModelManager::calculate_borrow_interest_rate(pool_id, utilization_rate).ok()?;
		// supply_interest_rate = utilization_rate * borrow_rate * (1 - protocol_interest_factor)
		let supply_rate: Rate = Rate::one()
			.checked_sub(&protocol_interest_factor)
			.and_then(|v| v.checked_mul(&borrow_rate))
			.and_then(|v| v.checked_mul(&utilization_rate))
			.ok_or(Error::<T>::NumOverflow)
			.ok()?;

		Some((exchange_rate, borrow_rate, supply_rate))
	}

	/// Gets current utilization rate of the pool. The rate is calculated for the current block.
	pub fn get_utilization_rate(pool_id: CurrencyId) -> Option<Rate> {
		let pool_data = Self::calculate_current_pool_data(pool_id).ok()?;
		let pool_underlying_balance = T::LiquidityPoolsManager::get_pool_available_liquidity(pool_id);
		Self::calculate_utilization_rate(
			pool_underlying_balance,
			pool_data.total_borrowed,
			pool_data.total_protocol_interest,
		)
		.ok()
	}

	/// Calculates user total supply and user total borrowed balance in usd based on
	/// total_borrowed, total_protocol_interest, borrow_index values calculated for current block.
	pub fn get_user_total_supply_and_borrowed_balance_in_usd(
		who: &T::AccountId,
	) -> result::Result<(Balance, Balance), DispatchError> {
		CurrencyId::get_enabled_tokens_in_protocol(UnderlyingAsset)
			.iter()
			.filter(|&underlying_id| T::LiquidityPoolsManager::pool_exists(underlying_id))
			.try_fold(
				(Balance::zero(), Balance::zero()),
				|(mut acc_user_supply_in_usd, mut acc_user_borrowed_in_usd),
				 &pool_id|
				 -> result::Result<(Balance, Balance), DispatchError> {
					let wrapped_id = pool_id.wrapped_asset().ok_or(Error::<T>::PoolNotFound)?;

					// Check if user has / had borrowed wrapped tokens in the pool
					let user_supply_wrap = T::MultiCurrency::free_balance(wrapped_id, &who);
					let has_user_supply_wrap_balance = !user_supply_wrap.is_zero();
					let has_user_borrow_underlying_balance =
						!<LiquidityPools<T>>::get_user_borrow_balance(&who, pool_id).is_zero();
					// Skip this pool if there is nothing to calculate
					if !has_user_supply_wrap_balance && !has_user_borrow_underlying_balance {
						return Ok((acc_user_supply_in_usd, acc_user_borrowed_in_usd));
					}

					let pool_data = Self::calculate_current_pool_data(pool_id)?;
					let oracle_price =
						T::PriceSource::get_underlying_price(pool_id).ok_or(Error::<T>::InvalidFeedPrice)?;

					if has_user_supply_wrap_balance {
						let current_exchange_rate = <LiquidityPools<T>>::get_exchange_rate_by_interest_params(
							pool_id,
							pool_data.total_protocol_interest,
							pool_data.total_borrowed,
						)?;
						acc_user_supply_in_usd += Rate::from_inner(user_supply_wrap)
							.checked_mul(&current_exchange_rate)
							.and_then(|v| v.checked_mul(&oracle_price))
							.map(|x| x.into_inner())
							.ok_or(Error::<T>::BalanceOverflow)?;
					}
					if has_user_borrow_underlying_balance {
						let user_borrow_balance =
							Self::calculate_user_borrow_balance(&who, pool_id, pool_data.borrow_index)?;
						let user_borrow_balance_in_usd = Rate::from_inner(user_borrow_balance)
							.checked_mul(&oracle_price)
							.map(|x| x.into_inner())
							.ok_or(Error::<T>::BalanceOverflow)?;
						acc_user_borrowed_in_usd += user_borrow_balance_in_usd;
					}
					Ok((acc_user_supply_in_usd, acc_user_borrowed_in_usd))
				},
			)
	}

	/// Calculates total amount of money currently held in the protocol in usd.
	/// Total value is calculated as: sum(total_issuance_n * exchange_rate_n * oracle_price_n),
	/// where:
	///     `total_issuance_n` - total number of wrapped tokens in the n pool;
	///     `exchange_rate_n` - exchange rate in the n pool;
	///     `oracle_price_n` - oracle price for the n pool.
	/// The amounts is calculated for the current block.
	pub fn get_protocol_total_value() -> BalanceResult {
		CurrencyId::get_enabled_tokens_in_protocol(UnderlyingAsset)
			.iter()
			.filter(|&underlying_id| T::LiquidityPoolsManager::pool_exists(underlying_id))
			.try_fold(Balance::zero(), |current_value, &pool_id| -> BalanceResult {
				let pool_data = Self::calculate_current_pool_data(pool_id)?;
				let current_exchange_rate = <LiquidityPools<T>>::get_exchange_rate_by_interest_params(
					pool_id,
					pool_data.total_protocol_interest,
					pool_data.total_borrowed,
				)?;
				let wrapped_id = pool_id.wrapped_asset().ok_or(Error::<T>::NotValidUnderlyingAssetId)?;
				let pool_supply_wrapped_balance = T::MultiCurrency::total_issuance(wrapped_id);
				let oracle_price = T::PriceSource::get_underlying_price(pool_id).ok_or(Error::<T>::InvalidFeedPrice)?;
				let pool_supply_balance_in_usd = Rate::from_inner(pool_supply_wrapped_balance)
					.checked_mul(&current_exchange_rate)
					.and_then(|v| v.checked_mul(&oracle_price))
					.map(|x| x.into_inner())
					.ok_or(Error::<T>::BalanceOverflow)?;
				Ok(current_value
					.checked_add(pool_supply_balance_in_usd)
					.ok_or(Error::<T>::BalanceOverflow)?)
			})
	}

	/// Calculate total collateral in usd based on collateral factor, fresh exchange rate and latest
	/// oracle price. Collateral is calculated for the current block.
	///
	/// - `who`: the AccountId whose collateral should be calculated.
	pub fn get_user_total_collateral(who: T::AccountId) -> BalanceResult {
		CurrencyId::get_enabled_tokens_in_protocol(UnderlyingAsset)
			.iter()
			.filter(|&pool_id| <LiquidityPools<T>>::check_user_available_collateral(&who, *pool_id))
			.try_fold(Balance::zero(), |acc, &pool_id| -> BalanceResult {
				let user_supply_underlying = Self::get_user_underlying_balance_per_asset(&who, pool_id)?;
				let collateral_factor = Self::controller_dates(pool_id).collateral_factor;
				let oracle_price = T::PriceSource::get_underlying_price(pool_id).ok_or(Error::<T>::InvalidFeedPrice)?;

				let user_collateral_in_usd = Rate::from_inner(user_supply_underlying)
					.checked_mul(&oracle_price)
					.and_then(|x| x.checked_mul(&collateral_factor))
					.map(|x| x.into_inner())
					.ok_or(Error::<T>::NumOverflow)?;

				Ok(acc + user_collateral_in_usd)
			})
	}

	/// Calculate actual borrow balance for user per asset based on fresh latest indexes.
	///
	/// - `who`: the AccountId whose balance should be calculated.
	/// - `currency_id`: ID of the currency, the balance of borrowing of which we calculate.
	pub fn get_user_borrow_per_asset(who: &T::AccountId, underlying_asset_id: CurrencyId) -> BalanceResult {
		ensure!(
			<LiquidityPools<T>>::pool_exists(&underlying_asset_id),
			Error::<T>::PoolNotFound
		);
		ensure!(
			underlying_asset_id.is_supported_underlying_asset(),
			Error::<T>::NotValidUnderlyingAssetId
		);
		let pool_data = Self::calculate_current_pool_data(underlying_asset_id)?;
		Self::calculate_user_borrow_balance(&who, underlying_asset_id, pool_data.borrow_index)
	}

	/// Calculates user balance converted to underlying asset using exchange rate calculated for the
	/// current block.
	///
	/// - `who`: the AccountId whose balance should be calculated.
	/// - `pool_id` - ID of the pool to calculate balance for.
	pub fn get_user_underlying_balance_per_asset(who: &T::AccountId, pool_id: CurrencyId) -> BalanceResult {
		ensure!(<LiquidityPools<T>>::pool_exists(&pool_id), Error::<T>::PoolNotFound);
		let wrapped_id = pool_id.wrapped_asset().ok_or(Error::<T>::NotValidUnderlyingAssetId)?;

		let user_balance_wrapped_tokens = T::MultiCurrency::free_balance(wrapped_id, &who);
		if user_balance_wrapped_tokens.is_zero() {
			return Ok(Balance::zero());
		}

		let pool_data = Self::calculate_current_pool_data(pool_id)?;
		let current_exchange_rate = <LiquidityPools<T>>::get_exchange_rate_by_interest_params(
			pool_id,
			pool_data.total_protocol_interest,
			pool_data.total_borrowed,
		)?;
		Ok(Rate::from_inner(user_balance_wrapped_tokens)
			.checked_mul(&current_exchange_rate)
			.map(|x| x.into_inner())
			.ok_or(Error::<T>::NumOverflow)?)
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
			let pool_borrow_underlying = T::LiquidityPoolsManager::get_pool_total_borrowed(pool_id);

			// new_borrow_balance_in_usd = (pool_borrow_underlying + borrow_amount) * oracle_price
			let new_pool_borrows = pool_borrow_underlying
				.checked_add(borrow_amount)
				.ok_or(Error::<T>::BalanceOverflow)?;

			let new_borrow_balance_in_usd = Rate::from_inner(new_pool_borrows)
				.checked_mul(&oracle_price)
				.map(|x| x.into_inner())
				.ok_or(Error::<T>::BalanceOverflow)?;

			Ok(new_borrow_balance_in_usd >= borrow_cap)
		} else {
			Ok(false)
		}
	}

	/// Calculate the borrow balance of account based on pool_borrow_index calculated beforehand.
	///
	/// - `who`: The address whose balance should be calculated.
	/// - `underlying_asset`: ID of the currency, the balance of borrowing of which we calculate.
	/// - `pool_borrow_index`: borrow index for the pool
	///
	/// Returns the borrow balance of account in underlying assets.
	fn calculate_user_borrow_balance(
		who: &T::AccountId,
		underlying_asset: CurrencyId,
		pool_borrow_index: Rate,
	) -> BalanceResult {
		let user_borrow_underlying = <LiquidityPools<T>>::get_user_borrow_balance(&who, underlying_asset);

		// If user_borrow_balance = 0 then borrow_index is likely also 0.
		// Rather than failing the calculation with a division by 0, we immediately return 0 in this case.
		if user_borrow_underlying.is_zero() {
			return Ok(Balance::zero());
		};

		let user_borrow_index = <LiquidityPools<T>>::get_user_borrow_index(&who, underlying_asset);

		// Calculate new user borrow balance using the borrow index:
		// recent_user_borrow_balance = user_borrow_balance * pool_borrow_index / user_borrow_index
		let recent_user_borrow_underlying = Rate::from_inner(user_borrow_underlying)
			.checked_mul(&pool_borrow_index)
			.and_then(|v| v.checked_div(&user_borrow_index))
			.map(|x| x.into_inner())
			.ok_or(Error::<T>::BorrowBalanceOverflow)?;
		Ok(recent_user_borrow_underlying)
	}

	/// Calculates the utilization rate of the pool.
	/// - `pool_supply_underlying_balance`: The amount of cash in the pool.
	/// - `pool_borrowed_balance`: The amount of borrows in the pool.
	/// - `pool_protocol_interest`: The amount of interest in the pool (currently unused).
	///
	/// returns `utilization_rate =
	///  pool_borrows / (pool_cash + pool_borrows - pool_protocol_interest)`
	fn calculate_utilization_rate(
		pool_supply_underlying_balance: Balance,
		pool_borrowed_balance: Balance,
		pool_protocol_interest: Balance,
	) -> RateResult {
		// Utilization rate is 0 when there are no borrows
		if pool_borrowed_balance.is_zero() {
			return Ok(Rate::zero());
		}

		// utilization_rate = pool_borrows / (pool_cash + pool_borrows - pool_protocol_interest)
		let utilization_rate = Rate::checked_from_rational(
			pool_borrowed_balance,
			pool_supply_underlying_balance
				.checked_add(pool_borrowed_balance)
				.and_then(|v| v.checked_sub(pool_protocol_interest))
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

	/// Calculates pool borrows, pool protocol interest and pool borrow index for given pool.
	/// Applies accrued interest to pool borrows and protocol interest and calculates interest
	/// accrued from the last checkpointed block up to the current block.
	///
	/// - `pool_id`: CurrencyId to calculate parameters for.
	///
	/// Returns pool parameters calculated for a current block.
	pub fn calculate_current_pool_data(pool_id: CurrencyId) -> result::Result<Pool, DispatchError> {
		let current_block_number = <frame_system::Module<T>>::block_number();
		let accrual_block_number_previous = Self::controller_dates(pool_id).last_interest_accrued_block;
		if current_block_number == accrual_block_number_previous {
			return Ok(<LiquidityPools<T>>::get_pool_data(pool_id));
		}

		let block_delta = Self::calculate_block_delta(current_block_number, accrual_block_number_previous)?;
		let current_total_balance = T::LiquidityPoolsManager::get_pool_available_liquidity(pool_id);
		let pool_data = <LiquidityPools<T>>::get_pool_data(pool_id);

		let utilization_rate = Self::calculate_utilization_rate(
			current_total_balance,
			pool_data.total_borrowed,
			pool_data.total_protocol_interest,
		)?;

		// Calculate the current borrow interest rate
		let current_borrow_interest_rate =
			T::MinterestModelManager::calculate_borrow_interest_rate(pool_id, utilization_rate)?;

		let ControllerData {
			max_borrow_rate,
			protocol_interest_factor,
			..
		} = Self::controller_dates(pool_id);

		ensure!(
			current_borrow_interest_rate <= max_borrow_rate,
			Error::<T>::BorrowRateTooHigh
		);

		/*
		Calculate the interest accumulated into borrows and protocol interest and the new index:
			*  simpleInterestFactor = borrowRate * blockDelta
			*  interestAccumulated = simpleInterestFactor * totalBorrows
			*  totalBorrowsNew = interestAccumulated + totalBorrows
			*  totalProtocolInterestNew = interestAccumulated * protocolInterestFactor + totalProtocolInterest
			*  borrowIndexNew = simpleInterestFactor * borrowIndex + borrowIndex
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
		let borrow_balance = Self::calculate_user_borrow_balance(who, underlying_asset_id, pool_borrow_index)?;
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
		let current_block_number = <frame_system::Module<T>>::block_number();
		if Self::controller_dates(underlying_asset).last_interest_accrued_block == current_block_number {
			return Ok(());
		}

		let pool_data = Self::calculate_current_pool_data(underlying_asset)?;
		// Save new params
		ControllerParams::<T>::mutate(underlying_asset, |data| {
			data.last_interest_accrued_block = current_block_number
		});
		<LiquidityPools<T>>::set_pool_data(
			underlying_asset,
			pool_data.total_borrowed,
			pool_data.borrow_index,
			pool_data.total_protocol_interest,
		);

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
}
