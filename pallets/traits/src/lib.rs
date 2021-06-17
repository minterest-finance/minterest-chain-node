#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::upper_case_acronyms)]

use minterest_primitives::{Balance, CurrencyId, Operation, Price, Rate};
use sp_runtime::{DispatchError, DispatchResult};
use sp_std::result::Result;

/// An abstraction of basic borrowing functions
pub trait Borrowing<AccountId> {
	/// Updates the state of the core as a consequence of a borrow action.
	fn update_state_on_borrow(
		who: &AccountId,
		underlying_asset: CurrencyId,
		amount_borrowed: Balance,
		account_borrows: Balance,
	) -> DispatchResult;

	/// updates the state of the core as a consequence of a repay action.
	fn update_state_on_repay(
		who: &AccountId,
		underlying_asset: CurrencyId,
		repay_amount: Balance,
		account_borrows: Balance,
	) -> DispatchResult;
}

/// An abstraction of pools basic functionalities.
pub trait PoolsManager<AccountId> {
	/// Return module account id.
	fn pools_account_id() -> AccountId;

	/// Return liquidity balance of `pool_id`.
	fn get_pool_available_liquidity(pool_id: CurrencyId) -> Balance;
}

/// Provides liquidity pool functionality
pub trait LiquidityPoolsManager<AccountId>: PoolsManager<AccountId> {
	/// Gets total amount borrowed from the pool.
	fn get_pool_total_borrowed(pool_id: CurrencyId) -> Balance;

	/// Gets pool borrow index
	/// Accumulator of the total earned interest rate since the opening of the pool
	fn get_pool_borrow_index(pool_id: CurrencyId) -> Rate;

	/// Gets current total amount of protocol interest of the underlying held in this pool.
	fn get_pool_total_protocol_interest(pool_id: CurrencyId) -> Balance;

	/// Check if pool exists
	fn pool_exists(underlying_asset: &CurrencyId) -> bool;

	/// This is a part of a pool creation flow
	/// Creates storage records for LiquidityPool
	fn create_pool(currency_id: CurrencyId) -> DispatchResult;
}

/// An abstraction of pools basic functionalities.
pub trait LiquidationPoolsManager<AccountId>: PoolsManager<AccountId> {
	/// This is a part of a pool creation flow
	/// Checks parameters validity and creates storage records for LiquidationPoolsData
	fn create_pool(currency_id: CurrencyId, deviation_threshold: Rate, balance_ratio: Rate) -> DispatchResult;
}

/// An abstraction of prices basic functionalities.
pub trait PricesManager<CurrencyId> {
	/// Get price underlying token in USD.
	fn get_underlying_price(currency_id: CurrencyId) -> Option<Price>;

	/// Locks price when get valid price from source.
	fn lock_price(currency_id: CurrencyId);

	/// Unlocks price when get valid price from source.
	fn unlock_price(currency_id: CurrencyId);
}

/// An abstraction of DEXs basic functionalities.
pub trait DEXManager<AccountId, CurrencyId, Balance> {
	//TODO: Add function description
	fn swap_with_exact_supply(
		who: &AccountId,
		target_currency_id: CurrencyId,
		supply_currency_id: CurrencyId,
		supply_amount: Balance,
		min_target_amount: Balance,
	) -> Result<Balance, DispatchError>;

	//TODO: Add function description
	fn swap_with_exact_target(
		who: &AccountId,
		supply_currency_id: CurrencyId,
		target_currency_id: CurrencyId,
		max_supply_amount: Balance,
		target_amount: Balance,
	) -> Result<Balance, DispatchError>;
}

/// An abstraction of controller basic functionalities.
pub trait ControllerManager<AccountId> {
	/// This is a part of a pool creation flow
	/// Creates storage records for ControllerParams and PauseKeepers
	/// All operations are unpaused after this function call
	fn create_pool(
		currency_id: CurrencyId,
		protocol_interest_factor: Rate,
		max_borrow_rate: Rate,
		collateral_factor: Rate,
		protocol_interest_threshold: Balance,
	) -> DispatchResult;

	/// Return the borrow balance of account based on stored data.
	fn borrow_balance_stored(who: &AccountId, underlying_asset_id: CurrencyId) -> Result<Balance, DispatchError>;

	/// Determine what the account liquidity would be if the given amounts were redeemed/borrowed.
	fn get_hypothetical_account_liquidity(
		account: &AccountId,
		underlying_to_borrow: CurrencyId,
		redeem_amount: Balance,
		borrow_amount: Balance,
	) -> Result<(Balance, Balance), DispatchError>;

	/// Applies accrued interest to total borrows and protocol interest.
	/// This calculates interest accrued from the last checkpointed block
	/// up to the current block and writes new checkpoint to storage.
	fn accrue_interest_rate(underlying_asset_id: CurrencyId) -> DispatchResult;

	/// Checks if a specific operation is allowed on a pool.
	fn is_operation_allowed(pool_id: CurrencyId, operation: Operation) -> bool;

	/// Checks if the account should be allowed to redeem tokens in the given pool.
	fn redeem_allowed(underlying_asset_id: CurrencyId, redeemer: &AccountId, redeem_amount: Balance) -> DispatchResult;

	/// Checks if the account should be allowed to borrow the underlying asset of the given pool.
	fn borrow_allowed(underlying_asset_id: CurrencyId, who: &AccountId, borrow_amount: Balance) -> DispatchResult;

	/// Return minimum protocol interest needed to transfer it to liquidation pool
	fn get_protocol_interest_threshold(pool_id: CurrencyId) -> Balance;

	/// Protocol operation mode. In whitelist mode, only members 'WhitelistCouncil' can work with
	/// protocols.
	fn is_whitelist_mode_enabled() -> bool;
}

pub trait MntManager<AccountId> {
	/// Update MNT supply index for a pool.
	///
	/// - `underlying_asset`: The pool which supply index to update.
	fn update_mnt_supply_index(underlying_id: CurrencyId) -> DispatchResult;

	/// Update MNT borrow index for a pool.
	///
	/// - `underlying_asset`: The pool which borrow index to update.
	fn update_mnt_borrow_index(underlying_id: CurrencyId) -> DispatchResult;

	/// Distribute MNT token to supplier. It should be called after update_mnt_supply_index.
	///
	/// - `underlying_id`: The pool in which the supplier is acting;
	/// - `supplier`: The AccountId of the supplier to distribute MNT to.
	///
	/// returns `supplier_mnt_accrued`: - The MNT accrued but not yet transferred to each user
	fn distribute_supplier_mnt(
		underlying_id: CurrencyId,
		supplier: &AccountId,
		distribute_all: bool,
	) -> Result<Balance, DispatchError>;

	/// Distribute MNT token to borrower. It should be called after update_mnt_borrow_index.
	/// Borrowers will not begin to accrue tokens till the first interaction with the protocol.
	///
	/// - `underlying_id`: The pool in which the borrower is acting;
	/// - `borrower`: The AccountId of the borrower to distribute MNT to.
	///
	/// returns `borrower_mnt_accrued`: - The MNT accrued but not yet transferred to each user
	fn distribute_borrower_mnt(
		underlying_id: CurrencyId,
		borrower: &AccountId,
		distribute_all: bool,
	) -> Result<Balance, DispatchError>;

	/// Return MNT Borrow Rate and MNT Supply Rate values per block for current pool.
	/// - `pool_id` - the pool to calculate rates
	///
	/// returns (`borrow_apy`, `supply_apy`): - percentage yield per block
	fn get_mnt_borrow_and_supply_rates(pool_id: CurrencyId) -> Result<(Price, Price), DispatchError>;
}

/// An abstraction of risk-manager basic functionalities.
pub trait RiskManagerManager {
	/// This is a part of a pool creation flow
	/// Creates storage records for RiskManagerParams
	fn create_pool(
		currency_id: CurrencyId,
		max_attempts: u8,
		min_partial_liquidation_sum: Balance,
		threshold: Rate,
		liquidation_fee: Rate,
	) -> DispatchResult;
}

/// An abstraction of minterest-model basic functionalities.
pub trait MinterestModelManager {
	/// This is a part of a pool creation flow
	/// Checks parameters validity and creates storage records for MinterestModelParams
	fn create_pool(
		currency_id: CurrencyId,
		kink: Rate,
		base_rate_per_block: Rate,
		multiplier_per_block: Rate,
		jump_multiplier_per_block: Rate,
	) -> DispatchResult;

	fn calculate_borrow_interest_rate(
		underlying_asset: CurrencyId,
		utilization_rate: Rate,
	) -> Result<Rate, DispatchError>;
}
