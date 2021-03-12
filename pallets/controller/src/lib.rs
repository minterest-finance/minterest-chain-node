//! # Controller Module
//!
//! ## Overview
//!
//! TODO: add overview.

#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::unused_unit)]
#![allow(clippy::upper_case_acronyms)]

use codec::{Decode, Encode};
use frame_support::{ensure, pallet_prelude::*, transactional};
use frame_system::{ensure_signed, pallet_prelude::*};
use liquidity_pools::Pool;
use minterest_primitives::{Balance, CurrencyId, Operation, Rate};
use orml_traits::MultiCurrency;
use pallet_traits::{PoolsManager, PriceProvider};
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

#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(Encode, Decode, Clone, RuntimeDebug, Eq, PartialEq, Default)]
pub struct ControllerData<BlockNumber> {
	/// Block number that interest was last accrued at.
	pub timestamp: BlockNumber,

	/// Defines the portion of borrower interest that is converted into insurance.
	pub insurance_factor: Rate,

	/// Maximum borrow rate.
	pub max_borrow_rate: Rate,

	/// Determines how much a user can borrow.
	pub collateral_factor: Rate,

	/// Maximum total borrow amount per pool in usd. Zero value means infinite borrow cap
	pub borrow_cap: Option<Balance>,
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

		/// The basic liquidity pools manager.
		type LiquidityPoolsManager: PoolsManager<Self::AccountId>;

		#[pallet::constant]
		/// Maximum total borrow amount per pool in usd
		type MaxBorrowCap: Get<Balance>;

		/// The origin which may update controller parameters. Root can
		/// always do this.
		type UpdateOrigin: EnsureOrigin<Self::Origin>;
	}

	#[pallet::error]
	pub enum Error<T> {
		/// Number overflow in calculation.
		NumOverflow,
		/// Borrow rate is absurdly high.
		BorrowRateIsTooHight,
		/// Oracle unavailable or price equal 0.
		OraclePriceError,
		/// Insufficient available liquidity.
		InsufficientLiquidity,
		/// Pool not found.
		PoolNotFound,
		/// Not enough balance to deposit or withdraw or repay.
		NotEnoughBalance,
		/// Balance overflows maximum.
		/// Only happened when the balance went wrong and balance overflows the integer type.
		BalanceOverflowed,
		/// Maximum borrow rate cannot be set to 0.
		MaxBorrowRateCannotBeZero,
		/// An error occurred in the parameters that were passed to the function.
		ParametersError,
		/// Collateral factor cannot be greater than one.
		CollateralFactorCannotBeGreaterThanOne,
		/// Collateral factor cannot be set to 0.
		CollateralFactorCannotBeZero,
		/// Borrow cap is reached
		BorrowCapReached,
		/// Invalid borrow cap
		InvalidBorrowCap,
		/// Borrow cap is zero
		ZeroBorrowCap,
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(crate) fn deposit_event)]
	pub enum Event {
		/// InsuranceFactor has been successfully changed
		InsuranceFactorChanged,
		/// Max Borrow Rate has been successfully changed
		MaxBorrowRateChanged,
		/// Collateral factor has been successfully changed
		CollateralFactorChanged,
		/// The operation is paused: \[pool_id, operation\]
		OperationIsPaused(CurrencyId, Operation),
		/// The operation is unpaused: \[pool_id, operation\]
		OperationIsUnPaused(CurrencyId, Operation),
		/// Insurance balance replenished: \[pool_id, amount\]
		DepositedInsurance(CurrencyId, Balance),
		/// Insurance balance redeemed: \[pool_id, amount\]
		RedeemedInsurance(CurrencyId, Balance),
		/// Borrow cap changed: \[pool_id, new_cap\]
		BorrowCapChanged(CurrencyId, Option<Balance>),
		/// Protocol operation mode switched: \[is_whitelist_mode\]
		ProtocolOperationModeSwitched(bool),
	}

	/// Controller data information: `(timestamp, insurance_factor, collateral_factor,
	/// max_borrow_rate)`.
	#[pallet::storage]
	#[pallet::getter(fn controller_dates)]
	pub(crate) type ControllerDates<T: Config> =
		StorageMap<_, Twox64Concat, CurrencyId, ControllerData<T::BlockNumber>, ValueQuery>;

	/// The Pause Guardian can pause certain actions as a safety mechanism.
	#[pallet::storage]
	#[pallet::getter(fn pause_keepers)]
	pub(crate) type PauseKeepers<T: Config> = StorageMap<_, Twox64Concat, CurrencyId, PauseKeeper, ValueQuery>;

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
				ControllerDates::<T>::insert(currency_id, ControllerData { ..*controller_data })
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
		#[pallet::weight(0)]
		#[transactional]
		pub fn pause_specific_operation(
			origin: OriginFor<T>,
			pool_id: CurrencyId,
			operation: Operation,
		) -> DispatchResultWithPostInfo {
			T::UpdateOrigin::ensure_origin(origin)?;
			ensure!(
				T::LiquidityPoolsManager::pool_exists(&pool_id),
				Error::<T>::PoolNotFound
			);
			match operation {
				Operation::Deposit => PauseKeepers::<T>::mutate(pool_id, |pool| pool.deposit_paused = true),
				Operation::Redeem => PauseKeepers::<T>::mutate(pool_id, |pool| pool.redeem_paused = true),
				Operation::Borrow => PauseKeepers::<T>::mutate(pool_id, |pool| pool.borrow_paused = true),
				Operation::Repay => PauseKeepers::<T>::mutate(pool_id, |pool| pool.repay_paused = true),
				Operation::Transfer => PauseKeepers::<T>::mutate(pool_id, |pool| pool.transfer_paused = true),
			};
			Self::deposit_event(Event::OperationIsPaused(pool_id, operation));
			Ok(().into())
		}

		/// Unpause specific operation (deposit, redeem, borrow, repay) with the pool.
		///
		/// The dispatch origin of this call must be 'UpdateOrigin'.
		#[pallet::weight(0)]
		#[transactional]
		pub fn unpause_specific_operation(
			origin: OriginFor<T>,
			pool_id: CurrencyId,
			operation: Operation,
		) -> DispatchResultWithPostInfo {
			T::UpdateOrigin::ensure_origin(origin)?;
			ensure!(
				T::LiquidityPoolsManager::pool_exists(&pool_id),
				Error::<T>::PoolNotFound
			);
			match operation {
				Operation::Deposit => PauseKeepers::<T>::mutate(pool_id, |pool| pool.deposit_paused = false),
				Operation::Redeem => PauseKeepers::<T>::mutate(pool_id, |pool| pool.redeem_paused = false),
				Operation::Borrow => PauseKeepers::<T>::mutate(pool_id, |pool| pool.borrow_paused = false),
				Operation::Repay => PauseKeepers::<T>::mutate(pool_id, |pool| pool.repay_paused = false),
				Operation::Transfer => PauseKeepers::<T>::mutate(pool_id, |pool| pool.transfer_paused = false),
			};
			Self::deposit_event(Event::OperationIsUnPaused(pool_id, operation));
			Ok(().into())
		}

		// FIXME: unused functionality
		/// Replenishes the insurance balance.
		#[pallet::weight(0)]
		#[transactional]
		pub fn deposit_insurance(
			origin: OriginFor<T>,
			pool_id: CurrencyId,
			amount: Balance,
		) -> DispatchResultWithPostInfo {
			let sender = ensure_signed(origin)?;
			Self::do_deposit_insurance(&sender, pool_id, amount)?;
			Self::deposit_event(Event::DepositedInsurance(pool_id, amount));
			Ok(().into())
		}

		/// Redeem the insurance balance.
		#[pallet::weight(0)]
		#[transactional]
		pub fn redeem_insurance(
			origin: OriginFor<T>,
			pool_id: CurrencyId,
			amount: Balance,
		) -> DispatchResultWithPostInfo {
			let sender = ensure_signed(origin)?;
			Self::do_redeem_insurance(&sender, pool_id, amount)?;
			Self::deposit_event(Event::RedeemedInsurance(pool_id, amount));
			Ok(().into())
		}

		/// Set insurance factor.
		///
		/// The dispatch origin of this call must be 'UpdateOrigin'.
		#[pallet::weight(0)]
		#[transactional]
		pub fn set_insurance_factor(
			origin: OriginFor<T>,
			pool_id: CurrencyId,
			new_amount_n: u128,
			new_amount_d: u128,
		) -> DispatchResultWithPostInfo {
			T::UpdateOrigin::ensure_origin(origin)?;
			ensure!(
				T::LiquidityPoolsManager::pool_exists(&pool_id),
				Error::<T>::PoolNotFound
			);

			let new_insurance_factor =
				Rate::checked_from_rational(new_amount_n, new_amount_d).ok_or(Error::<T>::NumOverflow)?;

			ControllerDates::<T>::mutate(pool_id, |r| r.insurance_factor = new_insurance_factor);
			Self::deposit_event(Event::InsuranceFactorChanged);
			Ok(().into())
		}

		/// Set Maximum borrow rate.
		///
		/// The dispatch origin of this call must be 'UpdateOrigin'.
		#[pallet::weight(0)]
		#[transactional]
		pub fn set_max_borrow_rate(
			origin: OriginFor<T>,
			pool_id: CurrencyId,
			new_amount_n: u128,
			new_amount_d: u128,
		) -> DispatchResultWithPostInfo {
			T::UpdateOrigin::ensure_origin(origin)?;
			ensure!(
				T::LiquidityPoolsManager::pool_exists(&pool_id),
				Error::<T>::PoolNotFound
			);

			let new_max_borow_rate =
				Rate::checked_from_rational(new_amount_n, new_amount_d).ok_or(Error::<T>::NumOverflow)?;

			ensure!(!new_max_borow_rate.is_zero(), Error::<T>::MaxBorrowRateCannotBeZero);

			ControllerDates::<T>::mutate(pool_id, |r| r.max_borrow_rate = new_max_borow_rate);
			Self::deposit_event(Event::MaxBorrowRateChanged);
			Ok(().into())
		}

		/// Set Collateral factor.
		///
		/// The dispatch origin of this call must be 'UpdateOrigin'.
		#[pallet::weight(0)]
		#[transactional]
		pub fn set_collateral_factor(
			origin: OriginFor<T>,
			pool_id: CurrencyId,
			new_amount_n: u128,
			new_amount_d: u128,
		) -> DispatchResultWithPostInfo {
			T::UpdateOrigin::ensure_origin(origin)?;
			ensure!(
				T::LiquidityPoolsManager::pool_exists(&pool_id),
				Error::<T>::PoolNotFound
			);

			let new_collateral_factor =
				Rate::checked_from_rational(new_amount_n, new_amount_d).ok_or(Error::<T>::NumOverflow)?;

			ensure!(
				new_collateral_factor <= Rate::one(),
				Error::<T>::CollateralFactorCannotBeGreaterThanOne
			);
			ensure!(
				!new_collateral_factor.is_zero(),
				Error::<T>::CollateralFactorCannotBeZero
			);

			ControllerDates::<T>::mutate(pool_id, |r| r.collateral_factor = new_collateral_factor);
			Self::deposit_event(Event::CollateralFactorChanged);
			Ok(().into())
		}

		/// Set borrow cap.
		///
		/// The dispatch origin of this call must be Administrator.
		/// Borrow cap value must be in range 0..1_000_000_000_000_000_000_000_000
		#[pallet::weight(0)]
		#[transactional]
		pub fn set_borrow_cap(
			origin: OriginFor<T>,
			pool_id: CurrencyId,
			borrow_cap: Option<Balance>,
		) -> DispatchResultWithPostInfo {
			T::UpdateOrigin::ensure_origin(origin)?;
			ensure!(
				T::EnabledUnderlyingAssetId::get()
					.into_iter()
					.any(|asset_id| asset_id == pool_id),
				Error::<T>::PoolNotFound
			);

			if let Some(cap) = borrow_cap {
				ensure!(
					cap >= Balance::zero() && cap <= T::MaxBorrowCap::get(),
					Error::<T>::InvalidBorrowCap
				);
			}
			ControllerDates::<T>::mutate(pool_id, |r| r.borrow_cap = borrow_cap);
			Self::deposit_event(Event::BorrowCapChanged(pool_id, borrow_cap));
			Ok(().into())
		}

		/// Enable / disable whitelist mode.
		///
		/// The dispatch origin of this call must be 'UpdateOrigin'.
		#[pallet::weight(0)]
		#[transactional]
		pub fn switch_mode(origin: OriginFor<T>) -> DispatchResultWithPostInfo {
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

impl<T: Config> Pallet<T> {
	/// Applies accrued interest to total borrows and insurances.
	/// This calculates interest accrued from the last checkpointed block
	/// up to the current block and writes new checkpoint to storage.
	pub fn accrue_interest_rate(underlying_asset_id: CurrencyId) -> DispatchResult {
		//Remember the initial block number.
		let current_block_number = <frame_system::Module<T>>::block_number();
		let accrual_block_number_previous = Self::controller_dates(underlying_asset_id).timestamp;
		// Calculate the number of blocks elapsed since the last accrual
		let block_delta = Self::calculate_block_delta(current_block_number, accrual_block_number_previous)?;
		//Short-circuit accumulating 0 interest.
		if block_delta == T::BlockNumber::zero() {
			return Ok(());
		}

		let pool_data = Self::calculate_interest_params(underlying_asset_id, block_delta)?;
		// Save new params
		ControllerDates::<T>::mutate(underlying_asset_id, |x| x.timestamp = current_block_number);
		<LiquidityPools<T>>::set_pool_data(
			underlying_asset_id,
			pool_data.total_borrowed,
			pool_data.borrow_index,
			pool_data.total_insurance,
		)?;

		Ok(())
	}

	/// Return the borrow balance of account based on stored data.
	///
	/// - `who`: The address whose balance should be calculated.
	/// - `currency_id`: ID of the currency, the balance of borrowing of which we calculate.
	pub fn borrow_balance_stored(who: &T::AccountId, underlying_asset_id: CurrencyId) -> BalanceResult {
		let user_borrow_balance = <LiquidityPools<T>>::get_user_total_borrowed(&who, underlying_asset_id);

		// If borrow_balance = 0 then borrow_index is likely also 0.
		// Rather than failing the calculation with a division by 0, we immediately return 0 in this case.
		if user_borrow_balance.is_zero() {
			return Ok(Balance::zero());
		};

		let pool_borrow_index = <LiquidityPools<T>>::get_pool_borrow_index(underlying_asset_id);
		let user_borrow_index = <LiquidityPools<T>>::get_user_borrow_index(&who, underlying_asset_id);

		// Calculate new borrow balance using the borrow index:
		// recent_borrow_balance = user_borrow_balance * pool_borrow_index / user_borrow_index
		let principal_times_index = Rate::from_inner(user_borrow_balance)
			.checked_mul(&pool_borrow_index)
			.map(|x| x.into_inner())
			.ok_or(Error::<T>::NumOverflow)?;

		let result = Rate::from_inner(principal_times_index)
			.checked_div(&user_borrow_index)
			.map(|x| x.into_inner())
			.ok_or(Error::<T>::NumOverflow)?;

		Ok(result)
	}

	/// Determine what the account liquidity would be if the given amounts were redeemed/borrowed.
	///
	/// - `account`: The account to determine liquidity.
	/// - `underlying_asset_id`: The pool to hypothetically redeem/borrow.
	/// - `redeem_amount`: The number of tokens to hypothetically redeem.
	/// - `borrow_amount`: The amount of underlying to hypothetically borrow.
	/// Returns (hypothetical account liquidity in excess of collateral requirements,
	///          hypothetical account shortfall below collateral requirements).
	pub fn get_hypothetical_account_liquidity(
		account: &T::AccountId,
		underlying_to_borrow: CurrencyId,
		redeem_amount: Balance,
		borrow_amount: Balance,
	) -> LiquidityResult {
		let m_tokens_ids: Vec<CurrencyId> = <T as liquidity_pools::Config>::EnabledCurrencyPair::get()
			.iter()
			.map(|currency_pair| currency_pair.wrapped_id)
			.collect();

		let mut sum_collateral = Balance::zero();
		let mut sum_borrow_plus_effects = Balance::zero();

		// For each tokens the account is in
		for asset in m_tokens_ids.into_iter() {
			let underlying_asset = <LiquidityPools<T>>::get_underlying_asset_id_by_wrapped_id(&asset)?;

			// Read the balances and exchange rate from the cToken
			let borrow_balance = Self::borrow_balance_stored(account, underlying_asset)?;
			let exchange_rate = <LiquidityPools<T>>::get_exchange_rate(underlying_asset)?;
			let collateral_factor = Self::get_collateral_factor(underlying_asset);

			// Get the normalized price of the asset.
			let oracle_price =
				T::PriceSource::get_underlying_price(underlying_asset).ok_or(Error::<T>::OraclePriceError)?;

			if oracle_price.is_zero() {
				return Ok((Balance::zero(), Balance::zero()));
			}

			// Pre-compute a conversion factor from tokens -> dollars (normalized price value)
			// tokens_to_denom = collateral_factor * exchange_rate * oracle_price
			let tokens_to_denom = collateral_factor
				.checked_mul(&exchange_rate)
				.and_then(|v| v.checked_mul(&oracle_price))
				.ok_or(Error::<T>::NumOverflow)?;

			if <LiquidityPools<T>>::check_user_available_collateral(&account, underlying_asset) {
				let m_token_balance = T::MultiCurrency::free_balance(asset, account);

				// sum_collateral += tokens_to_denom * m_token_balance
				sum_collateral =
					Self::mul_price_and_balance_add_to_prev_value(sum_collateral, m_token_balance, tokens_to_denom)?;
			}

			// sum_borrow_plus_effects += oracle_price * borrow_balance
			sum_borrow_plus_effects =
				Self::mul_price_and_balance_add_to_prev_value(sum_borrow_plus_effects, borrow_balance, oracle_price)?;

			// Calculate effects of interacting with Underlying Asset Modify.
			if underlying_to_borrow == underlying_asset {
				// redeem effect
				if redeem_amount > 0 {
					// sum_borrow_plus_effects += tokens_to_denom * redeem_tokens
					sum_borrow_plus_effects = Self::mul_price_and_balance_add_to_prev_value(
						sum_borrow_plus_effects,
						redeem_amount,
						tokens_to_denom,
					)?;
				};
				// borrow effect
				if borrow_amount > 0 {
					// sum_borrow_plus_effects += oracle_price * borrow_amount
					sum_borrow_plus_effects = Self::mul_price_and_balance_add_to_prev_value(
						sum_borrow_plus_effects,
						borrow_amount,
						oracle_price,
					)?;
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

	/// Checks if the account should be allowed to redeem tokens in the given pool.
	///
	/// - `underlying_asset_id` - The CurrencyId to verify the redeem against.
	/// - `redeemer` -  The account which would redeem the tokens.
	/// - `redeem_amount` - The number of mTokens to exchange for the underlying asset in the
	/// pool.
	///
	/// Return Ok if the redeem is allowed.
	pub fn redeem_allowed(
		underlying_asset_id: CurrencyId,
		redeemer: &T::AccountId,
		redeem_amount: Balance,
	) -> DispatchResult {
		if LiquidityPools::<T>::check_user_available_collateral(&redeemer, underlying_asset_id) {
			let (_, shortfall) =
				Self::get_hypothetical_account_liquidity(&redeemer, underlying_asset_id, redeem_amount, 0)?;

			ensure!(!(shortfall > 0), Error::<T>::InsufficientLiquidity);
		}

		Ok(())
	}

	/// Checks if the account should be allowed to borrow the underlying asset of the given pool.
	///
	/// - `underlying_asset_id` - The CurrencyId to verify the borrow against.
	/// - `who` -  The account which would borrow the asset.
	/// - `borrow_amount` - The amount of underlying assets the account would borrow.
	///
	/// Return Ok if the borrow is allowed.
	pub fn borrow_allowed(
		underlying_asset_id: CurrencyId,
		who: &T::AccountId,
		borrow_amount: Balance,
	) -> DispatchResult {
		let borrow_cap_reached = Self::is_borrow_cap_reached(underlying_asset_id, borrow_amount)?;
		ensure!(!borrow_cap_reached, Error::<T>::BorrowCapReached);

		let (_, shortfall) = Self::get_hypothetical_account_liquidity(&who, underlying_asset_id, 0, borrow_amount)?;

		ensure!(!(shortfall > 0), Error::<T>::InsufficientLiquidity);

		Ok(())
	}

	/// Checks if a specific operation is allowed on a pool.
	///
	/// Return true - if operation is allowed, false - if operation is unallowed.
	pub fn is_operation_allowed(pool_id: CurrencyId, operation: Operation) -> bool {
		match operation {
			Operation::Deposit => !Self::pause_keepers(pool_id).deposit_paused,
			Operation::Redeem => !Self::pause_keepers(pool_id).redeem_paused,
			Operation::Borrow => !Self::pause_keepers(pool_id).borrow_paused,
			Operation::Repay => !Self::pause_keepers(pool_id).repay_paused,
			Operation::Transfer => !Self::pause_keepers(pool_id).transfer_paused,
		}
	}

	/// Checks if borrow cap is reached
	///
	/// Return true if total borrow per pool will will exceed borrow cap, otherwise false
	pub fn is_borrow_cap_reached(pool_id: CurrencyId, borrow_amount: Balance) -> Result<bool, DispatchError> {
		let borrow_cap = Self::get_borrow_cap(pool_id);
		if borrow_cap == None {
			return Ok(false);
		}

		let oracle_price = T::PriceSource::get_underlying_price(pool_id).ok_or(Error::<T>::OraclePriceError)?;
		let pool_total_borrowed = <LiquidityPools<T>>::get_pool_total_borrowed(pool_id);
		let new_total_borrows = pool_total_borrowed
			.checked_add(borrow_amount)
			.ok_or(Error::<T>::NumOverflow)?;
		let new_total_borrows_in_usd = Rate::from_inner(new_total_borrows)
			.checked_mul(&oracle_price)
			.map(|x| x.into_inner())
			.ok_or(Error::<T>::NumOverflow)?;

		Ok(new_total_borrows_in_usd >= borrow_cap.unwrap_or_else(Balance::zero))
	}
}

// RPC methods
impl<T: Config> Pallet<T> {
	/// Gets the exchange rate between a mToken and the underlying asset.
	pub fn get_liquidity_pool_exchange_rate(pool_id: CurrencyId) -> Option<Rate> {
		let exchange_rate = <LiquidityPools<T>>::get_exchange_rate(pool_id).ok()?;
		Some(exchange_rate)
	}

	/// Gets borrow interest rate and supply interest rate.
	pub fn get_liquidity_pool_borrow_and_supply_rates(pool_id: CurrencyId) -> Option<(Rate, Rate)> {
		let current_total_balance = T::LiquidityPoolsManager::get_pool_available_liquidity(pool_id);
		let pool_data = <LiquidityPools<T>>::get_pool_data(pool_id);
		let insurance_factor = Self::get_insurance_factor(pool_id);

		let utilization_rate = Self::calculate_utilization_rate(
			current_total_balance,
			pool_data.total_borrowed,
			pool_data.total_insurance,
		)
		.ok()?;

		let borrow_rate = <MinterestModel<T>>::calculate_borrow_interest_rate(pool_id, utilization_rate).ok()?;

		let supply_rate = Self::calculate_supply_interest_rate(utilization_rate, borrow_rate, insurance_factor).ok()?;

		Some((borrow_rate, supply_rate))
	}

	/// Calculates total supply and total borrowed balance in usd based on
	/// total_borrowed, total_insurance, borrow_index values calculated for current block
	pub fn get_total_supply_and_borrowed_usd_balance(
		who: &T::AccountId,
	) -> result::Result<(Balance, Balance), DispatchError> {
		let (total_supply_balance, total_borrowed_balance) = T::EnabledCurrencyPair::get().iter().try_fold(
			(Balance::zero(), Balance::zero()),
			|current_value, currency_pair| -> result::Result<(Balance, Balance), DispatchError> {
				let pool_id = currency_pair.underlying_id;
				let wrapped_id = currency_pair.wrapped_id;

				// Check if user has / had borrowed wrapped tokens in the pool
				let wrapped_balance = T::MultiCurrency::free_balance(wrapped_id, &who);
				let has_balance = wrapped_balance > Balance::zero();
				let has_borrow_balance = <LiquidityPools<T>>::get_user_total_borrowed(&who, pool_id) > Balance::zero();
				// Skip this pool if there is nothing to calculate
				if !has_balance && !has_borrow_balance {
					return Ok(current_value);
				}

				let (current_supply_in_usd, current_borrowed_in_usd) = current_value;
				let current_block_number = <frame_system::Module<T>>::block_number();
				let accrual_block_number_previous = Self::controller_dates(pool_id).timestamp;
				// Calculate the number of blocks elapsed since the last accrual
				let block_delta = Self::calculate_block_delta(current_block_number, accrual_block_number_previous)?;
				let pool_data = Self::calculate_interest_params(pool_id, block_delta)?;
				let oracle_price = T::PriceSource::get_underlying_price(pool_id).ok_or(Error::<T>::OraclePriceError)?;

				let mut supply_in_usd = Balance::zero();
				let mut borrowed_in_usd = Balance::zero();
				if has_balance {
					let current_exchange_rate = <LiquidityPools<T>>::get_exchange_rate_by_interest_params(
						pool_id,
						pool_data.total_insurance,
						pool_data.total_borrowed,
					)?;
					supply_in_usd += Rate::from_inner(wrapped_balance)
						.checked_mul(&current_exchange_rate)
						.and_then(|v| v.checked_mul(&oracle_price))
						.map(|x| x.into_inner())
						.ok_or(Error::<T>::NumOverflow)?;
				}
				if has_borrow_balance {
					let borrow_balance =
						Self::borrow_balance_by_interest_params(&who, pool_id, pool_data.borrow_index)?;
					let borrow_balance_in_usd = Rate::from_inner(borrow_balance)
						.checked_mul(&oracle_price)
						.map(|x| x.into_inner())
						.ok_or(Error::<T>::NumOverflow)?;
					borrowed_in_usd += borrow_balance_in_usd;
				}
				Ok((
					current_supply_in_usd + supply_in_usd,
					current_borrowed_in_usd + borrowed_in_usd,
				))
			},
		)?;
		Ok((total_supply_balance, total_borrowed_balance))
	}

	/// Return the borrow balance of account based on pool_borrow_index calculated beforehand.
	///
	/// - `who`: The address whose balance should be calculated.
	/// - `underlying_asset_id`: ID of the currency, the balance of borrowing of which we calculate.
	/// - `pool_borrow_index`: borrow index for the pool
	pub fn borrow_balance_by_interest_params(
		who: &T::AccountId,
		underlying_asset_id: CurrencyId,
		pool_borrow_index: Rate,
	) -> BalanceResult {
		let user_borrow_balance = <LiquidityPools<T>>::get_user_total_borrowed(&who, underlying_asset_id);

		// If borrow_balance = 0 then borrow_index is likely also 0.
		// Rather than failing the calculation with a division by 0, we immediately return 0 in this case.
		if user_borrow_balance.is_zero() {
			return Ok(Balance::zero());
		};

		let user_borrow_index = <LiquidityPools<T>>::get_user_borrow_index(&who, underlying_asset_id);

		// Calculate new borrow balance using the borrow index:
		// recent_borrow_balance = user_borrow_balance * pool_borrow_index / user_borrow_index
		let principal_times_index = Rate::from_inner(user_borrow_balance)
			.checked_mul(&pool_borrow_index)
			.map(|x| x.into_inner())
			.ok_or(Error::<T>::NumOverflow)?;

		let result = Rate::from_inner(principal_times_index)
			.checked_div(&user_borrow_index)
			.map(|x| x.into_inner())
			.ok_or(Error::<T>::NumOverflow)?;

		Ok(result)
	}

	/// Calculates total borrows, total insurance and borrow index for given pool.
	/// Applies accrued interest to total borrows and insurances and calculates interest accrued
	/// from the last checkpointed block up to the current block and writes new checkpoint to
	/// storage.
	///
	/// - `underlying_asset_id`: ID of the currency to make calculations for.
	/// - `block_delta`: number of blocks passed since last accrue interest
	pub fn calculate_interest_params(
		underlying_asset_id: CurrencyId,
		block_delta: T::BlockNumber,
	) -> result::Result<Pool, DispatchError> {
		let current_total_balance = T::LiquidityPoolsManager::get_pool_available_liquidity(underlying_asset_id);
		let pool_data = <LiquidityPools<T>>::get_pool_data(underlying_asset_id);

		let utilization_rate = Self::calculate_utilization_rate(
			current_total_balance,
			pool_data.total_borrowed,
			pool_data.total_insurance,
		)?;

		// Calculate the current borrow interest rate
		let current_borrow_interest_rate =
			<MinterestModel<T>>::calculate_borrow_interest_rate(underlying_asset_id, utilization_rate)?;

		let max_borrow_rate = Self::get_max_borrow_rate(underlying_asset_id);
		let insurance_factor = Self::get_insurance_factor(underlying_asset_id);

		ensure!(
			current_borrow_interest_rate <= max_borrow_rate,
			Error::<T>::BorrowRateIsTooHight
		);

		/*
		Calculate the interest accumulated into borrows and insurance and the new index:
			*  simpleInterestFactor = borrowRate * blockDelta
			*  interestAccumulated = simpleInterestFactor * totalBorrows
			*  totalBorrowsNew = interestAccumulated + totalBorrows
			*  totalInsuranceNew = interestAccumulated * insuranceFactor + totalInsurance
			*  borrowIndexNew = simpleInterestFactor * borrowIndex + borrowIndex
		*/

		let simple_interest_factor = Self::calculate_interest_factor(current_borrow_interest_rate, block_delta)?;
		let interest_accumulated =
			Self::calculate_interest_accumulated(simple_interest_factor, pool_data.total_borrowed)?;
		let new_total_borrow_balance =
			Self::calculate_new_total_borrow(interest_accumulated, pool_data.total_borrowed)?;
		let new_total_insurance =
			Self::calculate_new_total_insurance(interest_accumulated, insurance_factor, pool_data.total_insurance)?;
		let new_borrow_index = Self::calculate_new_borrow_index(simple_interest_factor, pool_data.borrow_index)?;

		Ok(Pool {
			total_borrowed: new_total_borrow_balance,
			total_insurance: new_total_insurance,
			borrow_index: new_borrow_index,
		})
	}
}

// Private methods
impl<T: Config> Pallet<T> {
	/// Calculates the utilization rate of the pool.
	/// - `current_total_balance`: The amount of cash in the pool.
	/// - `current_total_borrowed_balance`: The amount of borrows in the pool.
	/// - `current_total_insurance`: The amount of insurance in the pool (currently unused).
	///
	/// returns `utilization_rate = total_borrows / (total_cash + total_borrows - total_insurance)`
	fn calculate_utilization_rate(
		current_total_balance: Balance,
		current_total_borrowed_balance: Balance,
		current_total_insurance: Balance,
	) -> RateResult {
		// Utilization rate is 0 when there are no borrows
		if current_total_borrowed_balance.is_zero() {
			return Ok(Rate::zero());
		}

		// utilization_rate = current_total_borrowed_balance / (current_total_balance +
		// + current_total_borrowed_balance - current_total_insurance)
		let utilization_rate = Rate::checked_from_rational(
			current_total_borrowed_balance,
			current_total_balance
				.checked_add(current_total_borrowed_balance)
				.and_then(|v| v.checked_sub(current_total_insurance))
				.ok_or(Error::<T>::NumOverflow)?,
		)
		.ok_or(Error::<T>::NumOverflow)?;

		Ok(utilization_rate)
	}

	/// Calculates the current supply interest rate of the pool.
	/// - `utilization_rate`: Current utilization rate.
	/// - `borrow_rate`: Current interest rate that users pay for lending assets.
	/// - `insurance_factor`: Current insurance factor.
	///
	/// returns `supply_interest_rate = utilization_rate * (borrow_rate * (1 - insurance_factor))`
	fn calculate_supply_interest_rate(utilization_rate: Rate, borrow_rate: Rate, insurance_factor: Rate) -> RateResult {
		let supply_interest_rate = Rate::one()
			.checked_sub(&insurance_factor)
			.and_then(|v| v.checked_mul(&borrow_rate))
			.and_then(|v| v.checked_mul(&utilization_rate))
			.ok_or(Error::<T>::NumOverflow)?;

		Ok(supply_interest_rate)
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

		let interest_factor: FixedU128 = Rate::saturating_from_rational(block_delta_as_usize as u128, 1)
			.checked_mul(&current_borrow_interest_rate)
			.ok_or(Error::<T>::NumOverflow)?;

		Ok(interest_factor)
	}

	/// Calculate the interest accumulated into borrows.
	///
	/// returns `interest_accumulated = simple_interest_factor * current_total_borrowed_balance`
	fn calculate_interest_accumulated(
		simple_interest_factor: Rate,
		current_total_borrowed_balance: Balance,
	) -> BalanceResult {
		let interest_accumulated = Rate::from_inner(current_total_borrowed_balance)
			.checked_mul(&simple_interest_factor)
			.map(|x| x.into_inner())
			.ok_or(Error::<T>::NumOverflow)?;

		Ok(interest_accumulated)
	}

	/// Calculates the new total borrow.
	/// - `interest_accumulated`: Accrued interest on the borrower's loan.
	/// - `current_total_borrowed_balance`: The amount of borrows in the pool.
	///
	/// returns `new_total_borrows = interest_accumulated + total_borrows`
	fn calculate_new_total_borrow(
		interest_accumulated: Balance,
		current_total_borrowed_balance: Balance,
	) -> BalanceResult {
		let new_total_borrows = interest_accumulated
			.checked_add(current_total_borrowed_balance)
			.ok_or(Error::<T>::NumOverflow)?;

		Ok(new_total_borrows)
	}

	/// Calculates new total insurance.
	/// - `interest_accumulated`: Accrued interest on the borrower's loan.
	/// - `insurance_factor`: The portion of borrower interest that is converted into insurance
	/// - `current_total_insurance`: The amount of insurance in the pool (currently unused).
	///
	/// returns `total_insurance_new = interest_accumulated * insurance_factor + total_insurance`
	fn calculate_new_total_insurance(
		interest_accumulated: Balance,
		insurance_factor: Rate,
		current_total_insurance: Balance,
	) -> BalanceResult {
		// insurance_accumulated = interest_accumulated * insurance_factor
		let insurance_accumulated = Rate::from_inner(interest_accumulated)
			.checked_mul(&insurance_factor)
			.map(|x| x.into_inner())
			.ok_or(Error::<T>::NumOverflow)?;

		// total_insurance_new = insurance_accumulated + current_total_insurance
		let total_insurance_new = insurance_accumulated
			.checked_add(current_total_insurance)
			.ok_or(Error::<T>::NumOverflow)?;

		Ok(total_insurance_new)
	}

	/// Calculates new borrow index.
	///
	/// returns `new_borrow_index = simple_interest_factor * borrow_index + borrow_index`
	fn calculate_new_borrow_index(simple_interest_factor: Rate, current_borrow_index: Rate) -> RateResult {
		let new_borrow_index = simple_interest_factor
			.checked_mul(&current_borrow_index)
			.and_then(|v| v.checked_add(&current_borrow_index))
			.ok_or(Error::<T>::NumOverflow)?;

		Ok(new_borrow_index)
	}

	/// Performs mathematical calculations.
	///
	/// returns `value = value + balance_scalar * rate_scalar`
	fn mul_price_and_balance_add_to_prev_value(
		value: Balance,
		balance_scalar: Balance,
		rate_scalar: Rate,
	) -> BalanceResult {
		let result = value
			.checked_add(
				Rate::from_inner(balance_scalar)
					.checked_mul(&rate_scalar)
					.map(|x| x.into_inner())
					.ok_or(Error::<T>::NumOverflow)?,
			)
			.ok_or(Error::<T>::NumOverflow)?;

		Ok(result)
	}
}

// Storage getters for Controller Data
impl<T: Config> Pallet<T> {
	/// Determines how much a user can borrow.
	fn get_collateral_factor(pool_id: CurrencyId) -> Rate {
		Self::controller_dates(pool_id).collateral_factor
	}

	/// Gets the maximum borrow rate.
	fn get_max_borrow_rate(pool_id: CurrencyId) -> Rate {
		Self::controller_dates(pool_id).max_borrow_rate
	}

	/// Get the insurance factor.
	fn get_insurance_factor(pool_id: CurrencyId) -> Rate {
		Self::controller_dates(pool_id).insurance_factor
	}

	/// Gets the borrow cap amount
	fn get_borrow_cap(pool_id: CurrencyId) -> Option<Balance> {
		Self::controller_dates(pool_id).borrow_cap
	}
}

// Admin functions
impl<T: Config> Pallet<T> {
	// FIXME It is possible to remove this function
	/// Replenishes the insurance balance.
	/// - `who`: Account ID of the administrator who replenishes the insurance.
	/// - `pool_id`: Pool ID of the replenishing pool.
	/// - `amount`: Amount to replenish insurance in the pool.
	fn do_deposit_insurance(who: &T::AccountId, pool_id: CurrencyId, amount: Balance) -> DispatchResult {
		ensure!(
			T::LiquidityPoolsManager::pool_exists(&pool_id),
			Error::<T>::PoolNotFound
		);

		ensure!(
			amount <= T::MultiCurrency::free_balance(pool_id, &who),
			Error::<T>::NotEnoughBalance
		);

		// transfer amount to this pool
		T::MultiCurrency::transfer(pool_id, &who, &T::LiquidityPoolsManager::pools_account_id(), amount)?;

		// calculate new insurance balance
		let current_insurance_balance = <LiquidityPools<T>>::get_pool_total_insurance(pool_id);

		let new_insurance_balance = current_insurance_balance
			.checked_add(amount)
			.ok_or(Error::<T>::BalanceOverflowed)?;

		<LiquidityPools<T>>::set_pool_total_insurance(pool_id, new_insurance_balance)?;

		Ok(())
	}

	/// Burns the insurance balance.
	/// - `who`: Account ID of the administrator who burns the insurance.
	/// - `pool_id`: Pool ID in which the insurance is decreasing.
	/// - `amount`: Amount to redeem insurance in the pool.
	fn do_redeem_insurance(who: &T::AccountId, pool_id: CurrencyId, amount: Balance) -> DispatchResult {
		ensure!(
			T::LiquidityPoolsManager::pool_exists(&pool_id),
			Error::<T>::PoolNotFound
		);

		ensure!(
			amount <= T::MultiCurrency::free_balance(pool_id, &T::LiquidityPoolsManager::pools_account_id()),
			Error::<T>::NotEnoughBalance
		);

		// calculate new insurance balance
		let current_total_insurance = <LiquidityPools<T>>::get_pool_total_insurance(pool_id);
		ensure!(amount <= current_total_insurance, Error::<T>::NotEnoughBalance);

		let new_insurance_balance = current_total_insurance
			.checked_sub(amount)
			.ok_or(Error::<T>::NotEnoughBalance)?;

		<LiquidityPools<T>>::set_pool_total_insurance(pool_id, new_insurance_balance)?;

		// transfer amount from this pool
		T::MultiCurrency::transfer(pool_id, &T::LiquidityPoolsManager::pools_account_id(), &who, amount)?;

		Ok(())
	}
}
