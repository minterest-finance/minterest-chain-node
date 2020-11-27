#![cfg_attr(not(feature = "std"), no_std)]

use minterest_primitives::{Balance, CurrencyId};
use sp_runtime::{DispatchResult};

/// An abstraction of liquidity pools for Minterest Protocol.
pub trait LiquidityPools {

    /// Deposit liquidity from `source` to pool of the given amount.
    fn add_liquidity(currency_id: &CurrencyId, amount: &Balance) -> DispatchResult;

    /// Withdraw liquidity from pool to `dest` of the given amount.
    fn withdraw_liquidity(currency_id: &CurrencyId, amount: &Balance) -> DispatchResult;

}























