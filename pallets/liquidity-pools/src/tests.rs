#![cfg(test)]

use super::*;
use mock::*;

use frame_support::{assert_noop, assert_ok};
use sp_arithmetic::FixedPointNumber;

#[test]
fn set_current_exchange_rate_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		// Set exchange_rate eq 1.2
		assert_ok!(LiquidityPools::set_current_exchange_rate(
			CurrencyId::DOT,
			Rate::saturating_from_rational(13, 10)
		));
		assert_eq!(
			<Pools>::get(CurrencyId::DOT).current_exchange_rate,
			Rate::saturating_from_rational(13, 10)
		);
	});
}

#[test]
fn set_pool_total_borrowed_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		// Set pool_total_borrowed eq 100 DOT
		assert_ok!(LiquidityPools::set_pool_total_borrowed(
			CurrencyId::DOT,
			ONE_HUNDRED_DOLLARS
		));
		assert_eq!(<Pools>::get(CurrencyId::DOT).total_borrowed, ONE_HUNDRED_DOLLARS);
	});
}

#[test]
fn set_pool_borrow_index_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		// Set pool_borrow_index eq 0.25
		assert_ok!(LiquidityPools::set_pool_borrow_index(
			CurrencyId::DOT,
			Rate::saturating_from_rational(25, 100)
		));
		assert_eq!(
			<Pools>::get(CurrencyId::DOT).borrow_index,
			Rate::saturating_from_rational(25, 100)
		);
	});
}

#[test]
fn set_pool_total_insurance_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		// Set pool_total_insurance eq 100 DOT.
		assert_ok!(LiquidityPools::set_pool_total_insurance(
			CurrencyId::DOT,
			ONE_HUNDRED_DOLLARS
		));
		assert_eq!(<Pools>::get(CurrencyId::DOT).total_insurance, ONE_HUNDRED_DOLLARS);
	});
}

#[test]
fn set_user_total_borrowed_and_interest_index_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		// Set user_total_borrowed eq 100 DOT and user_interest_index eq 0.33.
		assert_ok!(LiquidityPools::set_user_total_borrowed_and_interest_index(
			&ALICE,
			CurrencyId::DOT,
			ONE_HUNDRED_DOLLARS,
			Rate::saturating_from_rational(33, 100)
		));
		assert_eq!(
			<PoolUserDates<Test>>::get(ALICE, CurrencyId::DOT).total_borrowed,
			ONE_HUNDRED_DOLLARS
		);
		assert_eq!(
			<PoolUserDates<Test>>::get(ALICE, CurrencyId::DOT).interest_index,
			Rate::saturating_from_rational(33, 100)
		);
	});
}

#[test]
fn set_accrual_interest_params_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		// Set pool total_borrowed eq 100 DOT and pool total_insurance eq 10_000 DOT.
		assert_ok!(LiquidityPools::set_accrual_interest_params(
			CurrencyId::DOT,
			ONE_HUNDRED_DOLLARS,
			TEN_THOUSAND
		));
		assert_eq!(<Pools>::get(CurrencyId::DOT).total_borrowed, ONE_HUNDRED_DOLLARS);
		assert_eq!(<Pools>::get(CurrencyId::DOT).total_insurance, TEN_THOUSAND);
	});
}

#[test]
fn enable_as_collateral_internal_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		// Alice enable as collateral DOT pool.
		assert_ok!(LiquidityPools::enable_as_collateral_internal(&ALICE, CurrencyId::DOT));

		assert!(<PoolUserDates<Test>>::get(ALICE, CurrencyId::DOT).collateral);
	});
}

#[test]
fn disable_collateral_internal_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		// Alice disable collateral DOT pool.
		assert_ok!(LiquidityPools::disable_collateral_internal(&ALICE, CurrencyId::DOT));

		assert!(!<PoolUserDates<Test>>::get(ALICE, CurrencyId::DOT).collateral);
	});
}

#[test]
fn get_pool_available_liquidity_should_work() {
	ExtBuilder::default()
		.pool_balance(CurrencyId::DOT, TEN_THOUSAND)
		.build()
		.execute_with(|| {
			assert_eq!(
				LiquidityPools::get_pool_available_liquidity(CurrencyId::DOT),
				TEN_THOUSAND
			);
		});
}

#[test]
fn get_pool_total_borrowed_should_work() {
	ExtBuilder::default()
		.pool_with_params(
			CurrencyId::DOT,
			TEN_THOUSAND,
			Rate::default(),
			Rate::default(),
			Balance::default(),
		)
		.build()
		.execute_with(|| {
			assert_eq!(LiquidityPools::get_pool_total_borrowed(CurrencyId::DOT), TEN_THOUSAND);
		});
}

#[test]
fn get_pool_total_insurance_should_work() {
	ExtBuilder::default()
		.pool_with_params(
			CurrencyId::DOT,
			Balance::default(),
			Rate::default(),
			Rate::default(),
			TEN_THOUSAND,
		)
		.build()
		.execute_with(|| {
			assert_eq!(LiquidityPools::get_pool_total_insurance(CurrencyId::DOT), TEN_THOUSAND);
		});
}

#[test]
fn get_pool_borrow_index_should_work() {
	ExtBuilder::default()
		.pool_with_params(
			CurrencyId::DOT,
			Balance::default(),
			Rate::saturating_from_rational(125, 100),
			Rate::default(),
			Balance::default(),
		)
		.build()
		.execute_with(|| {
			assert_eq!(
				LiquidityPools::get_pool_borrow_index(CurrencyId::DOT),
				Rate::saturating_from_rational(125, 100)
			);
		});
}

#[test]
fn get_user_total_borrowed_should_work() {
	ExtBuilder::default()
		.pool_user_data_with_params(ALICE, CurrencyId::DOT, ONE_HUNDRED_DOLLARS, Rate::default(), true)
		.build()
		.execute_with(|| {
			assert_eq!(
				LiquidityPools::get_user_total_borrowed(&ALICE, CurrencyId::DOT),
				ONE_HUNDRED_DOLLARS
			);
		});
}

// #[test]
// fn pool_should_exists() {
// 	ExtBuilder::default().build().execute_with(|| {
// 		assert_eq!(LiquidityPools::pool_exists(&CurrencyId::DOT), true);
// 		assert_eq!(LiquidityPools::pool_exists(&CurrencyId::MDOT), false);
// 	});
// }
//
// #[test]
// fn update_state_on_borrow_should_work() {
// 	ExtBuilder::default()
// 		.one_hundred_dots_for_alice()
// 		.build()
// 		.execute_with(|| {
// 			assert_eq!(
// 				LiquidityPools::get_user_borrow_index(&ALICE, CurrencyId::DOT),
// 				Rate::from_inner(0)
// 			);
// 			assert_ok!(LiquidityPools::update_state_on_borrow(&ALICE, CurrencyId::DOT, 60, 0));
// 			assert_eq!(LiquidityPools::get_pool_total_borrowed(CurrencyId::DOT), 60);
// 			assert_eq!(LiquidityPools::get_user_total_borrowed(&ALICE, CurrencyId::DOT), 60);
// 			assert_eq!(
// 				LiquidityPools::get_user_borrow_index(&ALICE, CurrencyId::DOT),
// 				Rate::saturating_from_rational(1, 1)
// 			);
//
// 			assert_ok!(LiquidityPools::set_pool_borrow_index(
// 				CurrencyId::DOT,
// 				Rate::saturating_from_rational(1, 5)
// 			));
// 			assert_ok!(LiquidityPools::update_state_on_borrow(&ALICE, CurrencyId::DOT, 30, 60));
// 			assert_eq!(LiquidityPools::get_pool_total_borrowed(CurrencyId::DOT), 90);
// 			assert_eq!(LiquidityPools::get_user_total_borrowed(&ALICE, CurrencyId::DOT), 90);
// 			assert_eq!(
// 				LiquidityPools::get_user_borrow_index(&ALICE, CurrencyId::DOT),
// 				Rate::saturating_from_rational(1, 5)
// 			);
//
// 			assert_noop!(
// 				LiquidityPools::update_state_on_borrow(&ALICE, CurrencyId::DOT, Balance::max_value(), 90),
// 				Error::<Test>::NumOverflow
// 			);
// 		});
// }
//
// #[test]
// fn update_state_on_repay_should_work() {
// 	ExtBuilder::default()
// 		.one_hundred_dots_for_alice()
// 		.build()
// 		.execute_with(|| {
// 			assert_eq!(
// 				LiquidityPools::get_user_borrow_index(&ALICE, CurrencyId::DOT),
// 				Rate::from_inner(0)
// 			);
// 			assert_ok!(LiquidityPools::update_state_on_borrow(&ALICE, CurrencyId::DOT, 60, 0));
// 			assert_eq!(LiquidityPools::get_pool_total_borrowed(CurrencyId::DOT), 60);
// 			assert_eq!(LiquidityPools::get_user_total_borrowed(&ALICE, CurrencyId::DOT), 60);
// 			assert_eq!(
// 				LiquidityPools::get_user_borrow_index(&ALICE, CurrencyId::DOT),
// 				Rate::saturating_from_rational(1, 1)
// 			);
//
// 			assert_ok!(LiquidityPools::update_state_on_repay(&ALICE, CurrencyId::DOT, 30, 60));
// 			assert_eq!(LiquidityPools::get_pool_total_borrowed(CurrencyId::DOT), 30);
// 			assert_eq!(LiquidityPools::get_user_total_borrowed(&ALICE, CurrencyId::DOT), 30);
//
// 			assert_ok!(LiquidityPools::update_state_on_repay(&ALICE, CurrencyId::DOT, 10, 30));
// 			assert_eq!(LiquidityPools::get_pool_total_borrowed(CurrencyId::DOT), 20);
// 			assert_eq!(LiquidityPools::get_user_total_borrowed(&ALICE, CurrencyId::DOT), 20);
//
// 			assert_noop!(
// 				LiquidityPools::update_state_on_repay(&ALICE, CurrencyId::DOT, 100, 20),
// 				Error::<Test>::NumOverflow
// 			);
// 		});
// }
