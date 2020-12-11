#![cfg(test)]

use super::*;
use mock::*;

use frame_support::{assert_noop, assert_ok};

#[test]
fn update_state_on_deposit_should_work() {
    ExtBuilder::default().build().execute_with(|| {
        assert_ok!(LiquidityPools::update_state_on_deposit(100, CurrencyId::ETH));
        assert_ok!(LiquidityPools::update_state_on_deposit(100, CurrencyId::DOT));
        assert_ok!(LiquidityPools::update_state_on_deposit(100, CurrencyId::KSM));
        assert_ok!(LiquidityPools::update_state_on_deposit(100, CurrencyId::BTC));
    });
}

#[test]
fn pool_should_exists() {
    ExtBuilder::default().build().execute_with(|| {
        assert_eq!(LiquidityPools::pool_exists(&CurrencyId::DOT), true);
        assert_eq!(LiquidityPools::pool_exists(&CurrencyId::MDOT), false);
    });
}

#[test]
fn pool_not_found() {
    ExtBuilder::default().build().execute_with(|| {
        assert_noop!(
            LiquidityPools::update_state_on_deposit(100, CurrencyId::MBTC),
            Error::<Runtime>::PoolNotFound
        );
    });
}

#[test]
fn not_enough_balance() {
    ExtBuilder::default().build().execute_with(|| {
        assert_ok!(LiquidityPools::update_state_on_deposit(100, CurrencyId::DOT));
        assert_noop!(
            LiquidityPools::update_state_on_redeem(101, CurrencyId::DOT),
            Error::<Runtime>::NotEnoughBalance
        );
    });
}

#[test]
fn balance_overflowed() {
    ExtBuilder::default().build().execute_with(|| {
        assert_ok!(LiquidityPools::update_state_on_deposit(100, CurrencyId::DOT));
        assert_noop!(
            LiquidityPools::update_state_on_deposit(Balance::max_value(), CurrencyId::DOT),
            Error::<Runtime>::BalanceOverflowed
        );
    });
}

#[test]
fn add_and_without_liquidity() {
    ExtBuilder::default().build().execute_with(|| {
        assert_ok!(LiquidityPools::update_state_on_deposit(100, CurrencyId::ETH));
        assert_ok!(LiquidityPools::update_state_on_deposit(100, CurrencyId::DOT));
        assert_ok!(LiquidityPools::update_state_on_deposit(100, CurrencyId::KSM));
        assert_ok!(LiquidityPools::update_state_on_deposit(100, CurrencyId::BTC));
        assert_ok!(LiquidityPools::update_state_on_redeem(100, CurrencyId::ETH));
        assert_ok!(LiquidityPools::update_state_on_redeem(100, CurrencyId::DOT));
        assert_ok!(LiquidityPools::update_state_on_redeem(100, CurrencyId::KSM));
        assert_ok!(LiquidityPools::update_state_on_redeem(100, CurrencyId::BTC));
    });
}
