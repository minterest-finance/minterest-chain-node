#![cfg(test)]

use super::*;
use mock::*;

use frame_support::{assert_noop, assert_ok};
use sp_arithmetic::FixedPointNumber;

#[test]
fn pool_should_exists() {
	ExtBuilder::default().build().execute_with(|| {
		assert_eq!(LiquidityPools::pool_exists(&CurrencyId::DOT), true);
		assert_eq!(LiquidityPools::pool_exists(&CurrencyId::MDOT), false);
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

#[test]
fn enable_as_collateral_internal_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		// Alice enable as collateral DOT pool
		assert_ok!(LiquidityPools::enable_as_collateral_internal(&ALICE, CurrencyId::DOT));

		assert!(<PoolUserDates<Runtime>>::get(ALICE, CurrencyId::DOT).collateral);
	});
}

#[test]
fn disable_collateral_internal_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		// Alice disable collateral DOT pool
		assert_ok!(LiquidityPools::disable_collateral_internal(&ALICE, CurrencyId::DOT));

		assert!(!<PoolUserDates<Runtime>>::get(ALICE, CurrencyId::DOT).collateral);
	});
}
