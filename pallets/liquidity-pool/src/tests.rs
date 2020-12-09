#![cfg(test)]

use super::*;
use mock::*;

use frame_support::{assert_noop, assert_ok};

#[test]
fn add_liquidity_should_work() {
    ExtBuilder::default().build().execute_with(|| {
        assert_ok!(LiquidityPool::add_liquidity(&CurrencyId::ETH, &100));
        assert_ok!(LiquidityPool::add_liquidity(&CurrencyId::DOT, &100));
        assert_ok!(LiquidityPool::add_liquidity(&CurrencyId::KSM, &100));
        assert_ok!(LiquidityPool::add_liquidity(&CurrencyId::BTC, &100));
    });
}

#[test]
fn pool_should_exists() {
    ExtBuilder::default().build().execute_with(|| {
        assert_eq!(LiquidityPool::pool_exists(&CurrencyId::DOT), true);
        assert_eq!(LiquidityPool::pool_exists(&CurrencyId::MDOT), false);
    });
}

#[test]
fn pool_not_found() {
    ExtBuilder::default().build().execute_with(|| {
        assert_noop!(
            LiquidityPool::add_liquidity(&CurrencyId::MBTC, &100),
            liquidity_pool::Error::<Runtime>::PoolNotFound
        );
    });
}

#[test]
fn not_enough_balance() {
    ExtBuilder::default().build().execute_with(|| {
        assert_ok!(LiquidityPool::add_liquidity(&CurrencyId::DOT, &100));
        assert_noop!(
            LiquidityPool::withdraw_liquidity(&CurrencyId::DOT, &101),
            liquidity_pool::Error::<Runtime>::NotEnoughBalance
        );
    });
}

#[test]
fn add_and_without_liquidity() {
    ExtBuilder::default().build().execute_with(|| {
        assert_ok!(LiquidityPool::add_liquidity(&CurrencyId::ETH, &100));
        assert_ok!(LiquidityPool::add_liquidity(&CurrencyId::DOT, &100));
        assert_ok!(LiquidityPool::add_liquidity(&CurrencyId::KSM, &100));
        assert_ok!(LiquidityPool::add_liquidity(&CurrencyId::BTC, &100));
        assert_ok!(LiquidityPool::withdraw_liquidity(&CurrencyId::ETH, &50));
        assert_ok!(LiquidityPool::withdraw_liquidity(&CurrencyId::DOT, &50));
        assert_ok!(LiquidityPool::withdraw_liquidity(&CurrencyId::KSM, &50));
        assert_ok!(LiquidityPool::withdraw_liquidity(&CurrencyId::BTC, &50));
    });
}