#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::upper_case_acronyms)]

use minterest_primitives::{Balance, CurrencyId, Interest, Operation, Price, Rate};
use sp_runtime::{DispatchError, DispatchResult};
use sp_std::{collections::btree_set::BTreeSet, result::Result, vec::Vec};

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

/// Provides functionality for working with storage of liquidity pools.
pub trait LiquidityPoolStorageProvider<AccountId, PoolData> {
	/// Sets pool data.
	fn set_pool_data(pool_id: CurrencyId, pool_data: PoolData);

	/// Sets the total borrowed value in the pool.
	fn set_pool_borrow_underlying(pool_id: CurrencyId, new_pool_borrows: Balance);

	/// Sets the total interest in the pool.
	fn set_pool_protocol_interest(pool_id: CurrencyId, new_pool_protocol_interest: Balance);

	/// Gets pool associated data.
	fn get_pool_data(pool_id: CurrencyId) -> PoolData;

	/// Get list of users with active loan positions for a particular pool.
	fn get_pool_members_with_loan(underlying_asset: CurrencyId) -> Vec<AccountId>;

	/// Gets total amount borrowed from the pool.
	fn get_pool_borrow_underlying(pool_id: CurrencyId) -> Balance;

	/// Gets pool borrow index
	/// Accumulator of the total earned interest rate since the opening of the pool.
	fn get_pool_borrow_index(pool_id: CurrencyId) -> Rate;

	/// Gets current total amount of protocol interest of the underlying held in this pool.
	fn get_pool_protocol_interest(pool_id: CurrencyId) -> Balance;

	/// Check if pool exists.
	fn pool_exists(underlying_asset: &CurrencyId) -> bool;

	/// This is a part of a pool creation flow.
	/// Creates storage records for LiquidityPool.
	fn create_pool(pool_id: CurrencyId) -> DispatchResult;

	/// Removes pool data.
	fn remove_pool_data(pool_id: CurrencyId);
}

/// Provides functionality for working with a user's storage. Set parameters in storage,
/// get parameters, check parameters.
pub trait UserStorageProvider<AccountId, PoolUserData> {
	/// Sets the total borrowed and interest index for user.
	fn set_user_borrow_and_interest_index(
		who: &AccountId,
		pool_id: CurrencyId,
		new_borrow_underlying: Balance,
		new_interest_index: Rate,
	);

	/// Gets user data.
	fn get_user_data(pool_id: CurrencyId, who: &AccountId) -> PoolUserData;

	/// Global borrow_index as of the most recent balance-changing action.
	fn get_user_borrow_index(who: &AccountId, pool_id: CurrencyId) -> Rate;

	/// Gets total user borrowing.
	fn get_user_borrow_balance(who: &AccountId, pool_id: CurrencyId) -> Balance;
}

/// Provides functionality for working with a user's collateral pools.
pub trait UserCollateral<AccountId> {
	/// Returns an array of collateral pools for the user.
	/// The array is sorted in descending order by the number of wrapped tokens in USD.
	///
	/// - `who`: AccountId for which the pool array is returned.
	fn get_user_collateral_pools(who: &AccountId) -> Result<Vec<CurrencyId>, DispatchError>;

	/// Checks if the user has enabled the pool as collateral.
	fn is_pool_collateral(who: &AccountId, pool_id: CurrencyId) -> bool;

	/// Checks if the user has the collateral.
	fn check_user_has_collateral(who: &AccountId) -> bool;

	/// Sets the parameter `is_collateral` to `true`.
	fn enable_is_collateral(who: &AccountId, pool_id: CurrencyId);

	/// Sets the parameter `is_collateral` to `false`.
	fn disable_is_collateral(who: &AccountId, pool_id: CurrencyId);
}

/// An abstraction of pools basic functionalities.
pub trait LiquidationPoolsManager<AccountId>: PoolsManager<AccountId> {
	/// This is a part of a pool creation flow
	/// Checks parameters validity and creates storage records for LiquidationPoolsData
	fn create_pool(pool_id: CurrencyId, deviation_threshold: Rate, balance_ratio: Rate) -> DispatchResult;
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
		pool_id: CurrencyId,
		protocol_interest_factor: Rate,
		max_borrow_rate: Rate,
		collateral_factor: Rate,
		protocol_interest_threshold: Balance,
	) -> DispatchResult;

	/// Return the borrow balance of account based on stored data.
	fn user_borrow_balance_stored(who: &AccountId, underlying_asset_id: CurrencyId) -> Result<Balance, DispatchError>;

	/// Determine what the account liquidity would be if the given amounts were redeemed/borrowed.
	fn get_hypothetical_account_liquidity(
		account: &AccountId,
		underlying_to_borrow: Option<CurrencyId>,
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

	/// Calculates the amount of collateral based on the parameters pool_id and the supply amount.
	/// Reads the collateral factor value from storage.
	///
	/// Returns: `collateral_amount = supply_amount * collateral_factor`.
	fn calculate_collateral(pool_id: CurrencyId, supply_amount: Balance) -> Balance;

	/// For all active pools in the protocol, it checks all users: calls `accrue_interest_rate`,
	/// and then get_hypothetical_account_liquidity. If the user has a shortfall, then writes it
	/// to the vector.
	/// Returns: the vector of all users with an insolvent loan.
	fn get_all_users_with_insolvent_loan() -> Result<BTreeSet<AccountId>, DispatchError>;

	/// Gets exchange, borrow interest rate and supply interest rate. The rates is calculated
	/// for the current block.
	fn get_pool_exchange_borrow_and_supply_rates(pool_id: CurrencyId) -> Option<(Rate, Rate, Rate)>;

	/// Gets current utilization rate of the pool. The rate is calculated for the current block.
	fn get_pool_utilization_rate(pool_id: CurrencyId) -> Option<Rate>;

	/// Calculates user total supply and user total borrow balance in usd based on
	/// pool_borrow, pool_protocol_interest, borrow_index values calculated for current block.
	fn get_user_total_supply_and_borrow_balance_in_usd(who: &AccountId) -> Result<(Balance, Balance), DispatchError>;

	/// Calculates pool_total_supply, pool_total_borrow including interest, tvl (Total Value
	/// Locked), pool_protocol_interest. All values are converted to USD.
	/// pool_total_supply is calculated as: sum(pool_supply_usd)
	/// where:
	///     `pool_supply_usd` - current available liquidity in the n pool;
	/// pool_total_borrow is calculated as: sum(fresh_pool_borrow_usd)
	/// where:
	///     `fresh_pool_borrow_usd` - freshest value of pool borrow in the n pool;
	/// tvl is calculated as: sum(pool_supply_wrap * exchange_rate),
	/// where:
	///     `pool_supply_wrap` - total number of wrapped tokens in the n pool;
	///     `exchange_rate` - exchange rate in the n pool;
	/// pool_total_interest is calculated as: sum(fresh_pool_protocol_interest_usd)
	/// where:
	///     `fresh_pool_protocol_interest_usd` - freshest value of protocol interest in the n pool;
	fn get_protocol_total_values() -> Result<(Balance, Balance, Balance, Balance), DispatchError>;

	/// Calculate user total collateral in usd based on collateral factor, fresh exchange rate and
	/// latest oracle price. Collateral is calculated for the current block.
	///
	/// - `who`: the AccountId whose collateral should be calculated.
	fn get_user_total_collateral(who: AccountId) -> Result<Balance, DispatchError>;

	/// Calculate actual borrow balance for user per asset based on fresh latest indexes.
	///
	/// - `who`: the AccountId whose balance should be calculated.
	/// - `currency_id`: ID of the currency, the balance of borrowing of which we calculate.
	fn get_user_borrow_underlying_balance(
		who: &AccountId,
		underlying_asset_id: CurrencyId,
	) -> Result<Balance, DispatchError>;

	/// Calculates user balance converted to underlying asset using exchange rate calculated for the
	/// current block.
	///
	/// - `who`: the AccountId whose balance should be calculated.
	/// - `pool_id` - ID of the pool to calculate balance for.
	fn get_user_supply_underlying_balance(who: &AccountId, pool_id: CurrencyId) -> Result<Balance, DispatchError>;

	/// Calculate total user's supply APY, borrow APY and Net APY.
	///
	/// - `who`: the AccountId whose APY should be calculated.
	fn get_user_total_supply_borrow_and_net_apy(
		who: AccountId,
	) -> Result<(Interest, Interest, Interest), DispatchError>;

	/// Calculate user total borrow in usd based on fresh exchange rate and
	/// latest oracle price
	///
	/// - `who`: the AccountId whose borrow should be calculated.
	fn get_user_total_borrow_usd(who: &AccountId) -> Result<Balance, DispatchError>;
}

pub trait MntManager<AccountId> {
	/// Update MNT supply index for a pool.
	///
	/// - `underlying_asset`: The pool which supply index to update.
	fn update_pool_mnt_supply_index(underlying_id: CurrencyId) -> DispatchResult;

	/// Update MNT borrow index for a pool.
	///
	/// - `underlying_asset`: The pool which borrow index to update.
	fn update_pool_mnt_borrow_index(underlying_id: CurrencyId) -> DispatchResult;

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
	fn get_pool_mnt_borrow_and_supply_rates(pool_id: CurrencyId) -> Result<(Price, Price), DispatchError>;
}

/// An abstraction of minterest-model basic functionalities.
pub trait MinterestModelManager {
	/// This is a part of a pool creation flow
	/// Checks parameters validity and creates storage records for MinterestModelParams
	fn create_pool(
		pool_id: CurrencyId,
		kink: Rate,
		base_rate_per_block: Rate,
		multiplier_per_block: Rate,
		jump_multiplier_per_block: Rate,
	) -> DispatchResult;

	/// Calculates the current borrow rate per block.
	/// - `underlying_asset`: Asset ID for which the borrow interest rate is calculated.
	/// - `utilization_rate`: Current Utilization rate value.
	///
	/// returns `borrow_interest_rate`.
	fn calculate_pool_borrow_interest_rate(
		underlying_asset: CurrencyId,
		utilization_rate: Rate,
	) -> Result<Rate, DispatchError>;
}

/// An abstraction of controller basic functionalities.
pub trait WhitelistManager<AccountId> {
	/// Protocol operation mode. In whitelist mode, only members from
	/// whitelist can work with protocol.
	fn is_whitelist_mode_enabled() -> bool;

	/// Checks if the account is a whitelist member.
	fn is_whitelist_member(who: &AccountId) -> bool;

	/// Returns the set of all accounts in the whitelist.
	fn get_whitelist_members() -> BTreeSet<AccountId>;
}

/// This trait is used to get the exchange rate between underlying assets and wrapped tokens.
/// Call `fn accrue_interest_rate` first to get a fresh exchange rate. This trait also provides
/// functionality for converting between mTokens, underlying assets and USD.
pub trait CurrencyConverter {
	/// Gets the exchange rate between the wrapped tokens and the underlying asset.
	/// This function does not accrue interest before calculating the exchange rate.
	///
	/// - `pool_id`: pool ID for which the exchange rate is calculated.
	///
	/// returns `exchange_rate` between a mToken and the underlying asset.
	/// Note: first call `accrue_interest` if you want to get a fresh rate.
	fn get_exchange_rate(pool_id: CurrencyId) -> Result<Rate, DispatchError>;

	/// Converts a specified number of underlying assets into wrapped tokens.
	/// The calculation is based on the exchange rate.
	///
	/// - `underlying_amount`: the amount of underlying assets to be converted to wrapped tokens.
	/// - `exchange_rate`: exchange rate between a wrapped tokens and the underlying assets.
	///
	/// Returns `underlying_amount / exchange_rate`
	fn underlying_to_wrapped(underlying_amount: Balance, exchange_rate: Rate) -> Result<Balance, DispatchError>;

	/// Converts a specified number of underlying assets into USD.
	/// The calculation is based on the current oracle price.
	///
	/// - `underlying_amount`: the amount of underlying assets to be converted into USD.
	/// - `oracle_price`: market value of the underlying asset in USD.
	///
	/// Returns `underlying_amount * oracle_price`
	fn underlying_to_usd(underlying_amount: Balance, oracle_price: Price) -> Result<Balance, DispatchError>;

	/// Converts a specified number of wrapped tokens into underlying assets.
	/// The calculation is based on the exchange rate.
	///
	/// - `wrapped_amount`: the amount of wrapped tokens to be converted to underlying assets.
	/// - `exchange_rate`: exchange rate between a wrapped tokens and the underlying assets.
	///
	/// Returns `wrapped_amount * exchange_rate`.
	fn wrapped_to_underlying(wrapped_amount: Balance, exchange_rate: Rate) -> Result<Balance, DispatchError>;

	/// Converts a specified number of wrapped tokens into USD.
	/// The calculation is based on the exchange rate and the oracle price.
	///
	/// - `wrapped_amount`: the amount of wrapped tokens to be converted to USD.
	/// - `exchange_rate`: exchange rate between a wrapped tokens and the underlying assets.
	/// - `oracle_price`: market value of the underlying asset in USD.
	///
	/// Returns `wrapped_amount * exchange_rate * oracle_price`
	/// Note: first call `accrue_interest` if you want to exchange at a fresh exchange rate.
	fn wrapped_to_usd(
		wrapped_amount: Balance,
		exchange_rate: Rate,
		oracle_price: Price,
	) -> Result<Balance, DispatchError>;

	/// Converts a specified number of USD into underlying assets.
	/// The calculation is based on the current oracle price.
	///
	/// - `usd_amount`: the amount of USD to be converted to underlying assets.
	/// - `oracle_price`: market value of the underlying asset in USD.
	///
	/// Returns `usd_amount / oracle_price`
	fn usd_to_underlying(usd_amount: Balance, oracle_price: Price) -> Result<Balance, DispatchError>;

	/// Converts a specified amount of USD into wrapped tokens.
	/// The calculation is based on the exchange rate and the oracle price.
	///
	/// - `usd_amount`: the amount of USD to be converted into wrapped tokens.
	/// - `exchange_rate`: exchange rate between a wrapped tokens and the underlying assets.
	/// - `oracle_price`: market value of the underlying asset in USD.
	///
	/// Returns `usd_amount / oracle_price / exchange_rate `
	fn usd_to_wrapped(usd_amount: Balance, exchange_rate: Rate, oracle_price: Price) -> Result<Balance, DispatchError>;
}

/// Provides functionality to manage the number of attempts to partially liquidation a user's loan.
pub trait UserLiquidationAttemptsManager<AccountId> {
	type LiquidationMode;

	/// Gets user liquidation attempts.
	fn get_user_liquidation_attempts(who: &AccountId) -> u8;

	/// Mutates user liquidation attempts depending on user operation.
	/// If the user makes a deposit to the collateral pool, then attempts are set to zero.
	fn try_mutate_attempts(
		who: &AccountId,
		operation: Operation,
		pool_id: Option<CurrencyId>,
		liquidation_mode: Option<Self::LiquidationMode>,
	) -> DispatchResult;
}

/// Creates storage records for risk-manager pallet. This is a part of a pool creation flow.
pub trait RiskManagerStorageProvider {
	/// Creates storage records for risk-manager pallet: `liquidation_fee`
	/// and `liquidation_threshold`
	fn create_pool(pool_id: CurrencyId, liquidation_threshold: Rate, liquidation_fee: Rate) -> DispatchResult;

	/// Removes parameter values `liquidation_fee` and `liquidation_threshold` in the
	/// risk-manager pallet.
	fn remove_pool(pool_id: CurrencyId);
}

/// An abstraction of minterest-protocol basic functionalities.
pub trait MinterestProtocolManager<AccountId> {
	/// Borrows are repaid by another user (possibly the borrower).
	///
	/// - `who`: the account paying off the borrow.
	/// - `borrower`: the account with the debt being payed off.
	/// - `underlying_asset`: the currency ID of the underlying asset to repay.
	/// - `repay_amount`: the amount of the underlying asset to repay.
	fn do_repay(
		who: &AccountId,
		borrower: &AccountId,
		underlying_asset: CurrencyId,
		repay_amount: Balance,
		all_assets: bool,
	) -> Result<Balance, DispatchError>;

	/// Withdraws wrapped tokens from the borrower's account. Transfers the corresponding number
	/// of underlying assets from the liquidity pool to the liquidation pool. Called only during
	/// the liquidation process.
	///
	/// - `borrower`: borrower's account being liquidated.
	/// - `underlying_asset`: the currency ID of the underlying asset to seize.
	/// - `seize_underlying`: the amount of the underlying asset to seize.
	///
	/// Note: this function should be used after `accrue_interest_rate`.
	fn do_seize(borrower: &AccountId, underlying_asset: CurrencyId, seize_underlying: Balance) -> DispatchResult;
}
