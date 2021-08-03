//! # Minterest Protocol Module
//!
//! ## Overview
//!
//! This pallet provides ways for user to interact with Minterest protocol.
//! User call deposit, redeem, borrow, repay and transfer tokens.
//! Also user is able to enable/disable pool to be used as collateral.
//! Every first in a block successful call of deposit/redeem/borrow/repay causes interest to be
//! recalculated for a pool.
//! In WhitelistMode only users from WhitelistMembers are able to call extrinsics of this module.
//! Every time Minterest protocol interest reaches threshold (configured in Controller),
//! it is transferred from liquidity to liquidation pool.

#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::large_enum_variant)]
#![allow(clippy::unused_unit)]
#![allow(clippy::upper_case_acronyms)]

use frame_support::{pallet_prelude::*, transactional};
use frame_system::{ensure_signed, offchain::SendTransactionTypes, pallet_prelude::*};
use liquidity_pools::{PoolData, PoolUserData};
use minterest_primitives::{
	OriginalAsset, Balance, WrapToken, CurrencyId, Operation, Operation::Deposit, Rate,
};
pub use module::*;
use orml_traits::MultiCurrency;
use pallet_traits::{
	Borrowing, ControllerManager, CurrencyConverter, LiquidationPoolsManager, LiquidityPoolStorageProvider,
	MinterestModelManager, MinterestProtocolManager, MntManager, PoolsManager, RiskManagerStorageProvider,
	UserCollateral, UserLiquidationAttemptsManager, UserStorageProvider, WhitelistManager,
};
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};
use sp_runtime::{
	traits::{BadOrigin, Zero},
	DispatchError, DispatchResult,
};
use sp_std::{result, vec::Vec};

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

pub mod weights;
pub use weights::WeightInfo;

type TokensResult = result::Result<(Balance, WrapToken, Balance), DispatchError>;
type BalanceResult = result::Result<Balance, DispatchError>;

#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(Encode, Decode, Clone, RuntimeDebug, Eq, PartialEq, Default)]
pub struct PoolInitData {
	// Minterest Model storage data
	pub kink: Rate,
	pub base_rate_per_block: Rate,
	pub multiplier_per_block: Rate,
	pub jump_multiplier_per_block: Rate,
	//Controller storage data
	pub protocol_interest_factor: Rate,
	pub max_borrow_rate: Rate,
	pub collateral_factor: Rate,
	pub protocol_interest_threshold: Balance,
	// Liquidation Pools storage data
	pub deviation_threshold: Rate,
	pub balance_ratio: Rate,
	// Risk manager storage data
	pub liquidation_threshold: Rate,
	pub liquidation_fee: Rate,
}

#[frame_support::pallet]
pub mod module {
	use super::*;

	#[pallet::config]
	pub trait Config: frame_system::Config + SendTransactionTypes<Call<Self>> {
		/// The overarching event type.
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

		/// The `MultiCurrency` implementation.
		type MultiCurrency: MultiCurrency<Self::AccountId, Balance = Balance, CurrencyId = CurrencyId>;

		/// The basic liquidity pools.
		type ManagerLiquidationPools: LiquidationPoolsManager<Self::AccountId>;

		/// The basic liquidity pools.
		type ManagerLiquidityPools: LiquidityPoolStorageProvider<Self::AccountId, PoolData>
			+ PoolsManager<Self::AccountId>
			+ CurrencyConverter
			+ Borrowing<Self::AccountId>
			+ UserStorageProvider<Self::AccountId, PoolUserData>
			+ UserCollateral<Self::AccountId>;

		/// Provides MNT token distribution functionality.
		type MntManager: MntManager<Self::AccountId>;

		/// Weight information for the extrinsics.
		type ProtocolWeightInfo: WeightInfo;

		/// Public API of controller pallet.
		type ControllerManager: ControllerManager<Self::AccountId>;

		/// Public API of risk manager pallet.
		type MinterestModelManager: MinterestModelManager;

		/// The origin which may create pools. Root or
		/// Half Minterest Council can always do this.
		type CreatePoolOrigin: EnsureOrigin<Self::Origin>;

		/// Provides functionality to manage the number of attempts to partially
		/// liquidation a user's loan.
		type UserLiquidationAttempts: UserLiquidationAttemptsManager<Self::AccountId>;

		/// Public API of whitelist module.
		type WhitelistManager: WhitelistManager<Self::AccountId>;

		/// Public API of controller pallet.
		type RiskManager: RiskManagerStorageProvider;
	}

	#[pallet::error]
	pub enum Error<T> {
		/// The currency is not enabled in protocol.
		NotValidUnderlyingAssetId,
		/// There is not enough liquidity available in the liquidity pool.
		NotEnoughLiquidityAvailable,
		/// Insufficient wrapped tokens in the user account.
		NotEnoughWrappedTokens,
		/// Insufficient underlying assets in the user account.
		NotEnoughUnderlyingAsset,
		/// An internal failure occurred in the execution of the Accrue Interest function.
		AccrueInterestFailed,
		/// Transaction with zero balance is not allowed.
		ZeroBalanceTransaction,
		/// User is trying repay more than he borrowed.
		RepayAmountToBig,
		/// This pool is already collateral.
		AlreadyIsCollateral,
		/// This pool has already been disabled as a collateral.
		IsCollateralAlreadyDisabled,
		/// The user has an outstanding borrow. Cannot be disabled as collateral.
		IsCollateralCannotBeDisabled,
		/// The user has not deposited funds into the pool.
		IsCollateralCannotBeEnabled,
		/// Operation (deposit, redeem, borrow, repay) is paused.
		OperationPaused,
		/// The user is trying to transfer tokens to self
		CannotTransferToSelf,
		/// Hypothetical account liquidity calculation error.
		HypotheticalLiquidityCalculationError,
		/// The currency is not enabled in wrapped protocol.
		NotValidWrappedTokenId,
		/// Pool is already created
		PoolAlreadyCreated,
		/// Pool not found.
		PoolNotFound,
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(crate) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Underlying assets added to pool and wrapped tokens minted: \[who, pool_id,
		/// underlying_amount, wrap_token, wrapped_amount\]
		Deposited(T::AccountId, OriginalAsset, Balance, WrapToken, Balance),
		/// Underlying assets and wrapped tokens redeemed: \[who, pool_id,
		/// underlying_amount, wrap_token, wrapped_amount\]
		Redeemed(T::AccountId, OriginalAsset, Balance, WrapToken, Balance),
		/// Borrowed a specific amount of the pool currency: \[who, pool_id,
		/// the_amount_to_be_borrowed\]
		Borrowed(T::AccountId, OriginalAsset, Balance),
		/// Repaid a borrow on the specific pool, for the specified amount: \[who,
		/// pool_id, the_amount_repaid\]
		Repaid(T::AccountId, OriginalAsset, Balance),
		/// Claimed the MNT accrued by holder: \[holder\]
		Claimed(T::AccountId),
		/// Transferred specified amount on a specified pool from one account to another:
		/// \[who, receiver, wrap_token, wrapped_amount\]
		Transferred(T::AccountId, T::AccountId, WrapToken, Balance),
		/// The user allowed the assets in the pool to be used as collateral: \[who, pool_id\]
		PoolEnabledIsCollateral(T::AccountId, OriginalAsset),
		/// The user forbids the assets in the pool to be used as collateral: \[who, pool_id\]
		PoolDisabledIsCollateral(T::AccountId, OriginalAsset),
		/// Unable to transfer protocol interest from liquidity to liquidation pool: \[pool_id\]
		ProtocolInterestTransferFailed(OriginalAsset),
		/// New pool had been created: \[pool_id\]
		PoolCreated(OriginalAsset),
	}

	#[pallet::pallet]
	pub struct Pallet<T>(PhantomData<T>);

	#[pallet::hooks]
	impl<T: Config> Hooks<T::BlockNumber> for Pallet<T> {
		/// This hook performs the transfer of protocol interest from liquidity pools to
		/// liquidation pools. Runs after finalizing each block.
		fn on_finalize(_block_number: T::BlockNumber) {
			OriginalAsset::get_original_assets()
				.iter()
				.filter(|&&pool_id| T::ManagerLiquidityPools::pool_exists(pool_id))
				.for_each(|&pool_id| Self::transfer_protocol_interest(pool_id));
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Creates pool in storage. It is a part of a pool creation process and must be called
		/// after new CurrencyId is added to runtime.
		///
		/// Parameters:
		/// - `pool_id`: the CurrencyId of the underlying asset to create pool for;
		/// - `pool_data`: data to initialize pool storage in all pallets
		/// 	- [`Minterest Model:`](?search=MinterestModelData) kink, base_rate_per_block,
		/// multiplier_per_block, jump_multiplier_per_block
		/// 	- [`Controller:`](?search=controller::ControllerData) protocol_interest_factor,
		/// max_borrow_rate, collateral_factor, protocol_interest_threshold
		/// 	- [`Liquidation Pools:`](?search=liquidation_pools::LiquidationPoolData)
		/// deviation_threshold,
		/// balance_ratio
		/// 	- [`Risk Manager:`](?search=risk_manager::module::Call) liquidation_threshold,
		/// liquidation_fee
		#[pallet::weight(T::ProtocolWeightInfo::create_pool())]
		#[transactional]
		pub fn create_pool(
			origin: OriginFor<T>,
			pool_id: OriginalAsset,
			pool_data: PoolInitData,
		) -> DispatchResultWithPostInfo {
			T::CreatePoolOrigin::ensure_origin(origin)?;

			ensure!(
				!T::ManagerLiquidityPools::pool_exists(pool_id),
				Error::<T>::PoolAlreadyCreated
			);

			Self::do_create_pool(pool_id, pool_data)?;
			Self::deposit_event(Event::PoolCreated(pool_id));
			Ok(().into())
		}

		/// Transfers an asset into the protocol. The user receives a quantity of wrapped Tokens
		/// equal to the underlying tokens supplied, divided by the current Exchange Rate.
		///
		/// Parameters:
		/// - `pool_id`: CurrencyId of underlying assets to be transferred into the
		///   protocol.
		/// - `underlying_amount`: The amount of the asset to be supplied, in units of the
		///   underlying asset.
		#[pallet::weight(T::ProtocolWeightInfo::deposit_underlying())]
		#[transactional]
		pub fn deposit_underlying(
			origin: OriginFor<T>,
			pool_id: OriginalAsset,
			#[pallet::compact] underlying_amount: Balance,
		) -> DispatchResultWithPostInfo {
			let who = ensure_signed(origin)?;

			if T::WhitelistManager::is_whitelist_mode_enabled() {
				ensure!(T::WhitelistManager::is_whitelist_member(&who), BadOrigin);
			}

			let (_, wrapped_id, wrapped_amount) = Self::do_deposit(&who, pool_id, underlying_amount)?;
			Self::deposit_event(Event::Deposited(
				who,
				pool_id,
				underlying_amount,
				wrapped_id,
				wrapped_amount,
			));
			Ok(().into())
		}

		/// Converts ALL mTokens into a specified quantity of the underlying asset, and returns them
		/// to the user. The amount of underlying tokens received is equal to the quantity of
		/// mTokens redeemed, multiplied by the current Exchange Rate.
		///
		/// Parameters:
		/// - `pool_id`: CurrencyId of underlying assets to be redeemed.
		#[pallet::weight(T::ProtocolWeightInfo::redeem())]
		#[transactional]
		pub fn redeem(origin: OriginFor<T>, pool_id: OriginalAsset) -> DispatchResultWithPostInfo {
			let who = ensure_signed(origin)?;

			if T::WhitelistManager::is_whitelist_mode_enabled() {
				ensure!(T::WhitelistManager::is_whitelist_member(&who), BadOrigin);
			}
			let (underlying_amount, wrapped_id, wrapped_amount) =
				Self::do_redeem(&who, pool_id, Balance::zero(), Balance::zero(), true)?;
			Self::deposit_event(Event::Redeemed(
				who,
				pool_id,
				underlying_amount,
				wrapped_id,
				wrapped_amount,
			));
			Ok(().into())
		}

		/// Converts mTokens into a specified quantity of the underlying asset, and returns them to
		/// the user. The amount of mTokens redeemed is equal to the quantity of underlying tokens
		/// received, divided by the current Exchange Rate.
		///
		/// Parameters:
		/// - `pool_id`: CurrencyId of underlying assets to be redeemed.
		/// - `underlying_amount`: The number of underlying assets to be redeemed.
		#[pallet::weight(T::ProtocolWeightInfo::redeem_underlying())]
		#[transactional]
		pub fn redeem_underlying(
			origin: OriginFor<T>,
			pool_id: OriginalAsset,
			#[pallet::compact] underlying_amount: Balance,
		) -> DispatchResultWithPostInfo {
			let who = ensure_signed(origin)?;

			if T::WhitelistManager::is_whitelist_mode_enabled() {
				ensure!(T::WhitelistManager::is_whitelist_member(&who), BadOrigin);
			}
			let (_, wrapped_id, wrapped_amount) =
				Self::do_redeem(&who, pool_id, underlying_amount, Balance::zero(), false)?;
			Self::deposit_event(Event::Redeemed(
				who,
				pool_id,
				underlying_amount,
				wrapped_id,
				wrapped_amount,
			));
			Ok(().into())
		}

		/// Converts a specified quantity of mTokens into the underlying asset, and returns them to
		/// the user. The amount of underlying tokens received is equal to the quantity of mTokens
		/// redeemed, multiplied by the current Exchange Rate.
		///
		/// Parameters:
		/// - `wrapped_id`: CurrencyId of mTokens to be redeemed.
		/// - `wrapped_amount`: The number of mTokens to be redeemed.
		#[pallet::weight(T::ProtocolWeightInfo::redeem_wrapped())]
		#[transactional]
		pub fn redeem_wrapped(
			origin: OriginFor<T>,
			wrapped_id: WrapToken,
			#[pallet::compact] wrapped_amount: Balance,
		) -> DispatchResultWithPostInfo {
			let who = ensure_signed(origin)?;
			ensure!(wrapped_id.is_valid(), Error::<T>::NotValidWrappedTokenId);

			if T::WhitelistManager::is_whitelist_mode_enabled() {
				ensure!(T::WhitelistManager::is_whitelist_member(&who), BadOrigin);
			}

			let pool_id = wrapped_id.as_asset();
			let (underlying_amount, wrapped_id, _) =
				Self::do_redeem(&who, pool_id, Balance::zero(), wrapped_amount, false)?;
			Self::deposit_event(Event::Redeemed(
				who,
				pool_id,
				underlying_amount,
				wrapped_id,
				wrapped_amount,
			));
			Ok(().into())
		}

		/// Borrowing a specific amount of the pool currency, provided that the borrower already
		/// deposited enough collateral.
		///
		/// Parameters:
		/// - `pool_id`: The currency ID of the underlying asset to be borrowed.
		/// - `underlying_amount`: The amount of the underlying asset to be borrowed.
		#[pallet::weight(T::ProtocolWeightInfo::borrow())]
		#[transactional]
		pub fn borrow(
			origin: OriginFor<T>,
			pool_id: OriginalAsset,
			borrow_amount: Balance,
		) -> DispatchResultWithPostInfo {
			let who = ensure_signed(origin)?;

			if T::WhitelistManager::is_whitelist_mode_enabled() {
				ensure!(T::WhitelistManager::is_whitelist_member(&who), BadOrigin);
			}

			Self::do_borrow(&who, pool_id, borrow_amount)?;
			Self::deposit_event(Event::Borrowed(who, pool_id, borrow_amount));
			Ok(().into())
		}

		/// Repays a borrow on the specific pool, for the specified amount.
		///
		/// Parameters:
		/// - `pool_id`: The currency ID of the underlying asset to be repaid.
		/// - `repay_amount`: The amount of the underlying asset to be repaid.
		#[pallet::weight(T::ProtocolWeightInfo::repay())]
		#[transactional]
		pub fn repay(
			origin: OriginFor<T>,
			pool_id: OriginalAsset,
			#[pallet::compact] repay_amount: Balance,
		) -> DispatchResultWithPostInfo {
			let who = ensure_signed(origin)?;

			if T::WhitelistManager::is_whitelist_mode_enabled() {
				ensure!(T::WhitelistManager::is_whitelist_member(&who), BadOrigin);
			}

			Self::do_repay(&who, &who, pool_id, repay_amount, false)?;
			Self::deposit_event(Event::Repaid(who, pool_id, repay_amount));
			Ok(().into())
		}

		/// Repays a borrow on the specific pool, for the all amount.
		///
		/// Parameters:
		/// - `pool_id`: The currency ID of the underlying asset to be repaid.
		#[pallet::weight(T::ProtocolWeightInfo::repay_all())]
		#[transactional]
		pub fn repay_all(origin: OriginFor<T>, pool_id: OriginalAsset) -> DispatchResultWithPostInfo {
			let who = ensure_signed(origin)?;

			if T::WhitelistManager::is_whitelist_mode_enabled() {
				ensure!(T::WhitelistManager::is_whitelist_member(&who), BadOrigin);
			}

			let repay_amount = Self::do_repay(&who, &who, pool_id, Balance::zero(), true)?;
			Self::deposit_event(Event::Repaid(who, pool_id, repay_amount));
			Ok(().into())
		}

		/// Transfers an asset into the protocol, reducing the target user's borrow balance.
		///
		/// Parameters:
		/// - `pool_id`: The currency ID of the underlying asset to be repaid.
		/// - `borrower`: The account which borrowed the asset to be repaid.
		/// - `repay_amount`: The amount of the underlying borrowed asset to be repaid.
		#[pallet::weight(T::ProtocolWeightInfo::repay_on_behalf())]
		#[transactional]
		pub fn repay_on_behalf(
			origin: OriginFor<T>,
			pool_id: OriginalAsset,
			borrower: T::AccountId,
			repay_amount: Balance,
		) -> DispatchResultWithPostInfo {
			let who = ensure_signed(origin)?;

			if T::WhitelistManager::is_whitelist_mode_enabled() {
				ensure!(T::WhitelistManager::is_whitelist_member(&who), BadOrigin);
			}

			let repay_amount = Self::do_repay(&who, &borrower, pool_id, repay_amount, false)?;
			Self::deposit_event(Event::Repaid(who, pool_id, repay_amount));
			Ok(().into())
		}

		/// Transfers an asset within the pool.
		///
		/// Parameters:
		/// - `receiver`: the account that will receive tokens.
		/// - `wrapped_id`: the currency ID of the wrapped asset to transfer.
		/// - `transfer_amount`: the amount of the wrapped asset to transfer.
		#[pallet::weight(T::ProtocolWeightInfo::transfer_wrapped())]
		#[transactional]
		pub fn transfer_wrapped(
			origin: OriginFor<T>,
			receiver: T::AccountId,
			wrapped_id: WrapToken,
			transfer_amount: Balance,
		) -> DispatchResultWithPostInfo {
			let who = ensure_signed(origin)?;
			ensure!(wrapped_id.is_valid(), Error::<T>::NotValidWrappedTokenId);

			if T::WhitelistManager::is_whitelist_mode_enabled() {
				ensure!(T::WhitelistManager::is_whitelist_member(&who), BadOrigin);
			}

			Self::do_transfer(&who, &receiver, wrapped_id, transfer_amount)?;
			Self::deposit_event(Event::Transferred(who, receiver, wrapped_id, transfer_amount));
			Ok(().into())
		}

		/// Sender allowed the assets in the pool to be used as collateral.
		///
		/// Parameters:
		/// `pool_id`: CurrencyId of a pool that should be used a Collateral.
		#[pallet::weight(T::ProtocolWeightInfo::enable_is_collateral())]
		#[transactional]
		pub fn enable_is_collateral(origin: OriginFor<T>, pool_id: OriginalAsset) -> DispatchResultWithPostInfo {
			let sender = ensure_signed(origin)?;

			if T::WhitelistManager::is_whitelist_mode_enabled() {
				ensure!(T::WhitelistManager::is_whitelist_member(&sender), BadOrigin);
			}

			ensure!(
				T::ManagerLiquidityPools::pool_exists(pool_id),
				Error::<T>::PoolNotFound
			);

			ensure!(
				!T::ManagerLiquidityPools::is_pool_collateral(&sender, pool_id),
				Error::<T>::AlreadyIsCollateral
			);

			// If user does not have assets in the pool, then he cannot enable as collateral the pool.
			let wrapped_id = pool_id.as_wrap().ok_or(Error::<T>::NotValidUnderlyingAssetId)?;
			let user_wrapped_balance = T::MultiCurrency::free_balance(wrapped_id.as_currency(), &sender);
			ensure!(!user_wrapped_balance.is_zero(), Error::<T>::IsCollateralCannotBeEnabled);

			T::ManagerLiquidityPools::enable_is_collateral(&sender, pool_id);
			Self::deposit_event(Event::PoolEnabledIsCollateral(sender, pool_id));
			Ok(().into())
		}

		/// Sender has denies use the assets in pool as collateral.
		///
		/// Parameters:
		/// `pool_id`: CurrencyId of a pool that the user wants to disable as collateral.
		#[pallet::weight(T::ProtocolWeightInfo::disable_is_collateral())]
		#[transactional]
		pub fn disable_is_collateral(origin: OriginFor<T>, pool_id: OriginalAsset) -> DispatchResultWithPostInfo {
			let sender = ensure_signed(origin)?;

			if T::WhitelistManager::is_whitelist_mode_enabled() {
				ensure!(T::WhitelistManager::is_whitelist_member(&sender), BadOrigin);
			}

			ensure!(
				T::ManagerLiquidityPools::pool_exists(pool_id),
				Error::<T>::PoolNotFound
			);

			ensure!(
				T::ManagerLiquidityPools::is_pool_collateral(&sender, pool_id),
				Error::<T>::IsCollateralAlreadyDisabled
			);
			T::ControllerManager::accrue_interest_rate(pool_id)?;
			let exchange_rate = T::ManagerLiquidityPools::get_exchange_rate(pool_id)?;
			let wrapped_id = pool_id.as_wrap().ok_or(Error::<T>::NotValidUnderlyingAssetId)?;
			let user_supply_wrap = T::MultiCurrency::free_balance(wrapped_id.as_currency(), &sender);
			let user_supply_underlying =
				T::ManagerLiquidityPools::wrapped_to_underlying(user_supply_wrap, exchange_rate)?;

			// Check if the user will have enough collateral if he removes one of the collaterals.
			let (_, shortfall) = T::ControllerManager::get_hypothetical_account_liquidity(
				&sender,
				Some(pool_id),
				user_supply_underlying,
				Balance::zero(),
			)
			.map_err(|_| Error::<T>::HypotheticalLiquidityCalculationError)?;
			ensure!(shortfall.is_zero(), Error::<T>::IsCollateralCannotBeDisabled);

			T::ManagerLiquidityPools::disable_is_collateral(&sender, pool_id);
			Self::deposit_event(Event::PoolDisabledIsCollateral(sender, pool_id));
			Ok(().into())
		}

		/// Claim all the MNT accrued by holder in the specified markets.
		///
		/// Parameters:
		/// - `pools`: The vector of markets to claim MNT in
		#[pallet::weight(T::ProtocolWeightInfo::claim_mnt())]
		#[transactional]
		pub fn claim_mnt(origin: OriginFor<T>, pools: Vec<OriginalAsset>) -> DispatchResultWithPostInfo {
			let who = ensure_signed(origin)?;
			Self::do_claim(&who, pools)?;
			Self::deposit_event(Event::Claimed(who));
			Ok(().into())
		}
	}
}

// Private functions
impl<T: Config> Pallet<T> {
	/// This is a part of a new currency creation flow.
	/// Calls internal functions `create_pool` in pallets: liquidity-pools, minterest-model,
	/// controller, liquidation pool, risk-manager.
	fn do_create_pool(pool_id: OriginalAsset, pool_data: PoolInitData) -> DispatchResult {
		T::ManagerLiquidityPools::create_pool(pool_id)?;
		T::MinterestModelManager::create_pool(
			pool_id,
			pool_data.kink,
			pool_data.base_rate_per_block,
			pool_data.multiplier_per_block,
			pool_data.jump_multiplier_per_block,
		)?;
		T::ControllerManager::create_pool(
			pool_id,
			pool_data.protocol_interest_factor,
			pool_data.max_borrow_rate,
			pool_data.collateral_factor,
			pool_data.protocol_interest_threshold,
		)?;
		T::ManagerLiquidationPools::create_pool(pool_id, pool_data.deviation_threshold, pool_data.balance_ratio)?;
		T::RiskManager::create_pool(pool_id, pool_data.liquidation_threshold, pool_data.liquidation_fee)?;
		Ok(())
	}

	/// Performs the necessary checks for the existence of currency, check the user's
	/// balance, calls `accrue_interest_rate`, `update_mnt_supply_index`, `distribute_supplier_mnt`.
	/// Transfers an asset into the protocol. The user receives a quantity of wrapped Tokens equal
	/// to the underlying tokens supplied, divided by the current Exchange Rate.
	/// Also resets `user_liquidation_attempts` if it's greater than zero.
	///
	/// - `pool_id`: CurrencyId of underlying assets to be transferred into the protocol.
	/// - `deposit_underlying_amount`: The amount of the asset to be supplied, in units of the
	///   underlying asset.
	///
	/// Returns (`deposit_underlying_amount`, `wrapped_id`, `deposit_wrapped_amount`).
	fn do_deposit(
		who: &T::AccountId,
		pool_id: OriginalAsset,
		deposit_underlying_amount: Balance,
	) -> TokensResult {
		ensure!(
			T::ManagerLiquidityPools::pool_exists(pool_id),
			Error::<T>::PoolNotFound
		);

		ensure!(!deposit_underlying_amount.is_zero(), Error::<T>::ZeroBalanceTransaction);

		ensure!(
			deposit_underlying_amount <= T::MultiCurrency::free_balance(pool_id.as_currency(), &who),
			Error::<T>::NotEnoughLiquidityAvailable
		);

		T::ControllerManager::accrue_interest_rate(pool_id).map_err(|_| Error::<T>::AccrueInterestFailed)?;

		T::MntManager::update_pool_mnt_supply_index(pool_id)?;
		T::MntManager::distribute_supplier_mnt(pool_id, who, false)?;

		// Fail if deposit not allowed
		ensure!(
			T::ControllerManager::is_operation_allowed(pool_id, Operation::Deposit),
			Error::<T>::OperationPaused
		);

		let wrapped_id = pool_id
			.as_wrap()
			.ok_or(Error::<T>::NotValidUnderlyingAssetId)?;

		let exchange_rate = T::ManagerLiquidityPools::get_exchange_rate(pool_id)?;
		let deposit_wrapped_amount =
			T::ManagerLiquidityPools::underlying_to_wrapped(deposit_underlying_amount, exchange_rate)?;

		T::MultiCurrency::transfer(
			pool_id.as_currency(),
			&who,
			&T::ManagerLiquidityPools::pools_account_id(),
			deposit_underlying_amount,
		)?;

		T::MultiCurrency::deposit(wrapped_id.as_currency(), &who, deposit_wrapped_amount)?;
		T::UserLiquidationAttempts::try_mutate_attempts(&who, Deposit, Some(pool_id), None)?;

		Ok((deposit_underlying_amount, wrapped_id, deposit_wrapped_amount))
	}

	fn do_redeem(
		who: &T::AccountId,
		pool_id: OriginalAsset,
		mut underlying_amount: Balance,
		wrapped_amount: Balance,
		all_assets: bool,
	) -> TokensResult {
		ensure!(
			T::ManagerLiquidityPools::pool_exists(pool_id),
			Error::<T>::PoolNotFound
		);

		T::ControllerManager::accrue_interest_rate(pool_id).map_err(|_| Error::<T>::AccrueInterestFailed)?;
		let exchange_rate = T::ManagerLiquidityPools::get_exchange_rate(pool_id)?;
		let wrapped_id = pool_id
			.as_wrap()
			.ok_or(Error::<T>::NotValidUnderlyingAssetId)?;

		let wrapped_amount = match (underlying_amount, wrapped_amount, all_assets) {
			(0, 0, true) => {
				let total_wrapped_amount = T::MultiCurrency::free_balance(wrapped_id.as_currency(), &who);
				ensure!(
					total_wrapped_amount > Balance::zero(),
					Error::<T>::NotEnoughWrappedTokens
				);
				underlying_amount =
					T::ManagerLiquidityPools::wrapped_to_underlying(total_wrapped_amount, exchange_rate)?;
				total_wrapped_amount
			}
			(_, 0, false) => {
				ensure!(underlying_amount > Balance::zero(), Error::<T>::ZeroBalanceTransaction);
				T::ManagerLiquidityPools::underlying_to_wrapped(underlying_amount, exchange_rate)?
			}
			_ => {
				ensure!(wrapped_amount > Balance::zero(), Error::<T>::ZeroBalanceTransaction);
				underlying_amount = T::ManagerLiquidityPools::wrapped_to_underlying(wrapped_amount, exchange_rate)?;
				wrapped_amount
			}
		};

		ensure!(
			underlying_amount <= T::ManagerLiquidityPools::get_pool_available_liquidity(pool_id),
			Error::<T>::NotEnoughLiquidityAvailable
		);

		ensure!(
			wrapped_amount <= T::MultiCurrency::free_balance(wrapped_id.as_currency(), &who),
			Error::<T>::NotEnoughWrappedTokens
		);

		// Fail if redeem not allowed
		ensure!(
			T::ControllerManager::is_operation_allowed(pool_id, Operation::Redeem),
			Error::<T>::OperationPaused
		);
		T::ControllerManager::redeem_allowed(pool_id, &who, wrapped_amount)?;

		T::MntManager::update_pool_mnt_supply_index(pool_id)?;
		T::MntManager::distribute_supplier_mnt(pool_id, who, false)?;

		T::MultiCurrency::withdraw(wrapped_id.as_currency(), &who, wrapped_amount)?;

		T::MultiCurrency::transfer(
			pool_id.as_currency(),
			&T::ManagerLiquidityPools::pools_account_id(),
			&who,
			underlying_amount,
		)?;

		Ok((underlying_amount, wrapped_id, wrapped_amount))
	}

	/// Users borrow assets from the protocol to their own address
	///
	/// - `who`: the address of the user who borrows.
	/// - `pool_id`: the currency ID of the underlying asset to borrow.
	/// - `underlying_amount`: the amount of the underlying asset to borrow.
	fn do_borrow(who: &T::AccountId, pool_id: OriginalAsset, borrow_amount: Balance) -> DispatchResult {
		ensure!(
			T::ManagerLiquidityPools::pool_exists(pool_id),
			Error::<T>::PoolNotFound
		);

		let pool_available_liquidity = T::ManagerLiquidityPools::get_pool_available_liquidity(pool_id);

		// Raise an error if pool has insufficient supply underlying balance.
		ensure!(
			borrow_amount <= pool_available_liquidity,
			Error::<T>::NotEnoughLiquidityAvailable
		);

		ensure!(borrow_amount > Balance::zero(), Error::<T>::ZeroBalanceTransaction);

		T::ControllerManager::accrue_interest_rate(pool_id).map_err(|_| Error::<T>::AccrueInterestFailed)?;

		// Fail if borrow not allowed.
		ensure!(
			T::ControllerManager::is_operation_allowed(pool_id, Operation::Borrow),
			Error::<T>::OperationPaused
		);
		T::ControllerManager::borrow_allowed(pool_id, &who, borrow_amount)?;

		T::MntManager::update_pool_mnt_borrow_index(pool_id)?;
		T::MntManager::distribute_borrower_mnt(pool_id, who, false)?;

		// Fetch the amount the borrower owes, with accumulated interest.
		let account_borrows = T::ControllerManager::user_borrow_balance_stored(&who, pool_id)?;

		T::ManagerLiquidityPools::update_state_on_borrow(&who, pool_id, borrow_amount, account_borrows)?;

		// Transfer the borrow_amount from the protocol account to the borrower's account.
		T::MultiCurrency::transfer(
			pool_id.as_currency(),
			&T::ManagerLiquidityPools::pools_account_id(),
			&who,
			borrow_amount,
		)?;

		Ok(())
	}

	/// Sender transfers their tokens to other account
	///
	/// - `who`: the account transferring tokens.
	/// - `receiver`: the account that will receive tokens.
	/// - `wrapped_id`: the currency ID of the wrapped asset to transfer.
	/// - `transfer_amount`: the amount of the wrapped asset to transfer.
	fn do_transfer(
		who: &T::AccountId,
		receiver: &T::AccountId,
		wrapped_id: WrapToken,
		transfer_amount: Balance,
	) -> DispatchResult {
		ensure!(transfer_amount > Balance::zero(), Error::<T>::ZeroBalanceTransaction);
		ensure!(who != receiver, Error::<T>::CannotTransferToSelf);
		ensure!(wrapped_id.is_valid(), Error::<T>::NotValidWrappedTokenId);

		// Fail if invalid token id
		let pool_id = wrapped_id.as_asset();
		ensure!(
			T::ManagerLiquidityPools::pool_exists(pool_id),
			Error::<T>::PoolNotFound
		);

		// Fail if transfer is not allowed
		ensure!(
			T::ControllerManager::is_operation_allowed(pool_id, Operation::Transfer),
			Error::<T>::OperationPaused
		);

		// Fail if transfer_amount is not available for redeem
		T::ControllerManager::redeem_allowed(pool_id, &who, transfer_amount)?;

		T::MntManager::update_pool_mnt_supply_index(pool_id)?;
		T::MntManager::distribute_supplier_mnt(pool_id, who, false)?;
		T::MntManager::distribute_supplier_mnt(pool_id, receiver, false)?;

		// Fail if not enough free balance
		ensure!(
			transfer_amount <= T::MultiCurrency::free_balance(wrapped_id.as_currency(), &who),
			Error::<T>::NotEnoughWrappedTokens
		);

		// Transfer the transfer_amount from one account to another
		T::MultiCurrency::transfer(wrapped_id.as_currency(), &who, &receiver, transfer_amount)?;

		Ok(())
	}

	fn transfer_protocol_interest(pool_id: OriginalAsset) {
		let pool_protocol_interest = T::ManagerLiquidityPools::get_pool_protocol_interest(pool_id);
		if pool_protocol_interest < T::ControllerManager::get_protocol_interest_threshold(pool_id) {
			return;
		}

		let pool_supply_underlying = T::ManagerLiquidityPools::get_pool_available_liquidity(pool_id);
		let to_liquidation_pool = pool_supply_underlying.min(pool_protocol_interest);

		// If no overflow and transfer is successful update pool state
		if let Some(new_protocol_interest) = pool_protocol_interest.checked_sub(to_liquidation_pool) {
			if T::MultiCurrency::transfer(
				pool_id.as_currency(),
				&T::ManagerLiquidityPools::pools_account_id(),
				&T::ManagerLiquidationPools::pools_account_id(),
				to_liquidation_pool,
			)
			.is_ok()
			{
				T::ManagerLiquidityPools::set_pool_protocol_interest(pool_id, new_protocol_interest);
			} else {
				Self::deposit_event(Event::ProtocolInterestTransferFailed(pool_id));
			}
		} else {
			Self::deposit_event(Event::ProtocolInterestTransferFailed(pool_id));
		}
	}

	/// Claim all the MNT accrued by holder in the specified markets.
	/// - `holder`: The AccountId to claim mnt for;
	/// - `pools`: The vector of pools to claim MNT in.
	fn do_claim(holder: &T::AccountId, pools: Vec<OriginalAsset>) -> DispatchResult {
		pools.iter().try_for_each(|&pool_id| -> DispatchResult {
			ensure!(
				T::ManagerLiquidityPools::pool_exists(pool_id),
				Error::<T>::PoolNotFound
			);

			T::MntManager::update_pool_mnt_borrow_index(pool_id)?;
			T::MntManager::distribute_borrower_mnt(pool_id, holder, true)?;
			T::MntManager::update_pool_mnt_supply_index(pool_id)?;
			T::MntManager::distribute_supplier_mnt(pool_id, holder, true)?;
			Ok(())
		})
	}
}

impl<T: Config> MinterestProtocolManager<T::AccountId> for Pallet<T> {
	/// Borrows are repaid by another user (possibly the borrower).
	fn do_repay(
		who: &T::AccountId,
		borrower: &T::AccountId,
		pool_id: OriginalAsset,
		repay_amount: Balance,
		all_assets: bool,
	) -> BalanceResult {
		let mut repay_amount = repay_amount;
		ensure!(
			T::ManagerLiquidityPools::pool_exists(pool_id),
			Error::<T>::PoolNotFound
		);

		T::ControllerManager::accrue_interest_rate(pool_id).map_err(|_| Error::<T>::AccrueInterestFailed)?;

		if !all_assets {
			ensure!(!repay_amount.is_zero(), Error::<T>::ZeroBalanceTransaction);
		}

		// Fail if repay_borrow not allowed
		ensure!(
			T::ControllerManager::is_operation_allowed(pool_id, Operation::Repay),
			Error::<T>::OperationPaused
		);

		T::MntManager::update_pool_mnt_borrow_index(pool_id)?;
		T::MntManager::distribute_borrower_mnt(pool_id, borrower, false)?;

		// Fetch the amount the borrower owes, with accumulated interest
		let account_borrows = T::ControllerManager::user_borrow_balance_stored(&borrower, pool_id)?;

		if repay_amount.is_zero() {
			repay_amount = account_borrows
		}

		ensure!(
			repay_amount <= T::MultiCurrency::free_balance(pool_id.as_currency(), &who),
			Error::<T>::NotEnoughUnderlyingAsset
		);

		T::ManagerLiquidityPools::update_state_on_repay(&borrower, pool_id, repay_amount, account_borrows)?;

		// Transfer the repay_amount from the borrower's account to the protocol account.
		T::MultiCurrency::transfer(
			pool_id.as_currency(),
			&who,
			&T::ManagerLiquidityPools::pools_account_id(),
			repay_amount,
		)?;

		Ok(repay_amount)
	}

	/// Withdraws wrapped tokens from the borrower's account. Transfers the corresponding number
	/// of underlying assets from the liquidity pool to the liquidation pool. Called only during
	/// the liquidation process.
	fn do_seize(_borrower: &T::AccountId, _pool_id: OriginalAsset, _seize_underlying: Balance) -> DispatchResult {
		todo!()
	}
}
