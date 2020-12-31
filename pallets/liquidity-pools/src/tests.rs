#![cfg(test)]

use super::*;
use mock::*;

use frame_support::{assert_noop, assert_ok, error::BadOrigin};
use sp_arithmetic::FixedPointNumber;

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
			LiquidityPools::lock_pool_transactions(Origin::root(), CurrencyId::MBTC),
			Error::<Runtime>::PoolNotFound
		);
	});
}

#[test]
fn lock_pool_transactions_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		assert_eq!(LiquidityPools::pools(&CurrencyId::DOT).is_lock, false);
		assert_ok!(LiquidityPools::lock_pool_transactions(Origin::root(), CurrencyId::DOT));
		assert_eq!(LiquidityPools::pools(&CurrencyId::DOT).is_lock, true);
		assert_noop!(
			LiquidityPools::lock_pool_transactions(Origin::signed(ALICE), CurrencyId::DOT),
			BadOrigin
		);
		assert_noop!(
			LiquidityPools::lock_pool_transactions(Origin::root(), CurrencyId::MDOT),
			Error::<Runtime>::PoolNotFound
		);
	});
}

#[test]
fn unlock_pool_transactions_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		assert_eq!(LiquidityPools::pools(&CurrencyId::ETH).is_lock, true);
		assert_ok!(LiquidityPools::unlock_pool_transactions(
			Origin::root(),
			CurrencyId::ETH
		));
		assert_eq!(LiquidityPools::pools(&CurrencyId::ETH).is_lock, false);
		assert_noop!(
			LiquidityPools::lock_pool_transactions(Origin::signed(ALICE), CurrencyId::ETH),
			BadOrigin
		);
		assert_noop!(
			LiquidityPools::lock_pool_transactions(Origin::root(), CurrencyId::METH),
			Error::<Runtime>::PoolNotFound
		);
	});
}

#[test]
fn deposit_insurance_should_work() {
	ExtBuilder::default()
		.one_hundred_dots_for_alice()
		.build()
		.execute_with(|| {
			// FIXME This dispatch should only be called as an _Root_.
			assert_noop!(
				LiquidityPools::deposit_insurance(Origin::signed(ALICE), CurrencyId::DOT, 101),
				Error::<Runtime>::NotEnoughBalance
			);
			assert_noop!(
				LiquidityPools::deposit_insurance(Origin::signed(ALICE), CurrencyId::MDOT, 5),
				Error::<Runtime>::PoolNotFound
			);

			assert_ok!(LiquidityPools::deposit_insurance(
				Origin::signed(ALICE),
				CurrencyId::DOT,
				60
			));
			assert_eq!(LiquidityPools::get_pool_total_insurance(CurrencyId::DOT), 60);
			assert_eq!(Currencies::free_balance(CurrencyId::DOT, &ALICE), 40);
			assert_eq!(LiquidityPools::get_pool_available_liquidity(CurrencyId::DOT), 60);

			assert_ok!(LiquidityPools::deposit_insurance(
				Origin::signed(ALICE),
				CurrencyId::DOT,
				5
			));
			assert_eq!(LiquidityPools::get_pool_total_insurance(CurrencyId::DOT), 65);
			assert_eq!(Currencies::free_balance(CurrencyId::DOT, &ALICE), 35);
			assert_eq!(LiquidityPools::get_pool_available_liquidity(CurrencyId::DOT), 65);
		});
}

#[test]
fn redeem_insurance_should_work() {
	ExtBuilder::default()
		.one_hundred_dots_for_alice()
		.build()
		.execute_with(|| {
			// FIXME This dispatch should only be called as an _Root_.
			assert_noop!(
				LiquidityPools::deposit_insurance(Origin::signed(ALICE), CurrencyId::MDOT, 5),
				Error::<Runtime>::PoolNotFound
			);

			assert_ok!(LiquidityPools::deposit_insurance(
				Origin::signed(ALICE),
				CurrencyId::DOT,
				60
			));
			assert_eq!(LiquidityPools::get_pool_total_insurance(CurrencyId::DOT), 60);
			assert_eq!(Currencies::free_balance(CurrencyId::DOT, &ALICE), 40);
			assert_eq!(LiquidityPools::get_pool_available_liquidity(CurrencyId::DOT), 60);

			assert_noop!(
				LiquidityPools::redeem_insurance(Origin::signed(ALICE), CurrencyId::DOT, 61),
				Error::<Runtime>::NotEnoughBalance
			);

			assert_ok!(LiquidityPools::redeem_insurance(
				Origin::signed(ALICE),
				CurrencyId::DOT,
				30
			));
			assert_eq!(LiquidityPools::get_pool_total_insurance(CurrencyId::DOT), 30);
			assert_eq!(Currencies::free_balance(CurrencyId::DOT, &ALICE), 70);
			assert_eq!(LiquidityPools::get_pool_available_liquidity(CurrencyId::DOT), 30);
		});
}

#[test]
fn update_state_on_borrow_should_work() {
	ExtBuilder::default()
		.one_hundred_dots_for_alice()
		.build()
		.execute_with(|| {
			assert_eq!(
				LiquidityPools::get_user_borrow_index(&ALICE, CurrencyId::DOT),
				Rate::from_inner(0)
			);
			assert_ok!(LiquidityPools::update_state_on_borrow(&ALICE, CurrencyId::DOT, 60, 0));
			assert_eq!(LiquidityPools::get_pool_total_borrowed(CurrencyId::DOT), 60);
			assert_eq!(LiquidityPools::get_user_total_borrowed(&ALICE, CurrencyId::DOT), 60);
			assert_eq!(
				LiquidityPools::get_user_borrow_index(&ALICE, CurrencyId::DOT),
				Rate::saturating_from_rational(1, 1)
			);

			assert_ok!(LiquidityPools::set_pool_borrow_index(
				CurrencyId::DOT,
				Rate::saturating_from_rational(1, 5)
			));
			assert_ok!(LiquidityPools::update_state_on_borrow(&ALICE, CurrencyId::DOT, 30, 60));
			assert_eq!(LiquidityPools::get_pool_total_borrowed(CurrencyId::DOT), 90);
			assert_eq!(LiquidityPools::get_user_total_borrowed(&ALICE, CurrencyId::DOT), 90);
			assert_eq!(
				LiquidityPools::get_user_borrow_index(&ALICE, CurrencyId::DOT),
				Rate::saturating_from_rational(1, 5)
			);

			assert_noop!(
				LiquidityPools::update_state_on_borrow(&ALICE, CurrencyId::DOT, Balance::max_value(), 90),
				Error::<Runtime>::NumOverflow
			);
		});
}

#[test]
fn update_state_on_repay_should_work() {
	ExtBuilder::default()
		.one_hundred_dots_for_alice()
		.build()
		.execute_with(|| {
			assert_eq!(
				LiquidityPools::get_user_borrow_index(&ALICE, CurrencyId::DOT),
				Rate::from_inner(0)
			);
			assert_ok!(LiquidityPools::update_state_on_borrow(&ALICE, CurrencyId::DOT, 60, 0));
			assert_eq!(LiquidityPools::get_pool_total_borrowed(CurrencyId::DOT), 60);
			assert_eq!(LiquidityPools::get_user_total_borrowed(&ALICE, CurrencyId::DOT), 60);
			assert_eq!(
				LiquidityPools::get_user_borrow_index(&ALICE, CurrencyId::DOT),
				Rate::saturating_from_rational(1, 1)
			);

			assert_ok!(LiquidityPools::update_state_on_repay(&ALICE, CurrencyId::DOT, 30, 60));
			assert_eq!(LiquidityPools::get_pool_total_borrowed(CurrencyId::DOT), 30);
			assert_eq!(LiquidityPools::get_user_total_borrowed(&ALICE, CurrencyId::DOT), 30);

			assert_ok!(LiquidityPools::update_state_on_repay(&ALICE, CurrencyId::DOT, 10, 30));
			assert_eq!(LiquidityPools::get_pool_total_borrowed(CurrencyId::DOT), 20);
			assert_eq!(LiquidityPools::get_user_total_borrowed(&ALICE, CurrencyId::DOT), 20);

			assert_noop!(
				LiquidityPools::update_state_on_repay(&ALICE, CurrencyId::DOT, 100, 20),
				Error::<Runtime>::NumOverflow
			);
		});
}
