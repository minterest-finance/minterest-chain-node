#![cfg(test)]

use super::*;
use mock::*;

use frame_support::{assert_noop, assert_ok};
use sp_arithmetic::FixedPointNumber;

#[test]
fn set_pool_data_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		TestPools::set_pool_data(DOT, ONE_HUNDRED, Rate::saturating_from_rational(125, 100), ONE_HUNDRED);
		assert_eq!(<Pools<Test>>::get(DOT).borrowed, ONE_HUNDRED);
		assert_eq!(
			<Pools<Test>>::get(DOT).borrow_index,
			Rate::saturating_from_rational(125, 100)
		);
		assert_eq!(<Pools<Test>>::get(DOT).protocol_interest, ONE_HUNDRED);
	});
}

#[test]
fn set_pool_borrow_underlying_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		// Set pool_borrowed eq 100 DOT
		TestPools::set_pool_borrow_underlying(DOT, ONE_HUNDRED);
		assert_eq!(<Pools<Test>>::get(DOT).borrowed, ONE_HUNDRED);
	});
}

#[test]
fn set_pool_protocol_interest_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		// Set pool_protocol_interest eq 100 DOT.
		TestPools::set_pool_protocol_interest(DOT, ONE_HUNDRED);
		assert_eq!(<Pools<Test>>::get(DOT).protocol_interest, ONE_HUNDRED);
	});
}

#[test]
fn set_user_borrow_and_interest_index_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		// Set user_borrowed eq 100 DOT and user_interest_index eq 0.33.
		TestPools::set_user_borrow_and_interest_index(
			&ALICE,
			DOT,
			ONE_HUNDRED,
			Rate::saturating_from_rational(33, 100),
		);
		assert_eq!(<PoolUserParams<Test>>::get(DOT, ALICE).borrowed, ONE_HUNDRED);
		assert_eq!(
			<PoolUserParams<Test>>::get(DOT, ALICE).interest_index,
			Rate::saturating_from_rational(33, 100)
		);
	});
}

#[test]
fn enable_is_collateral_internal_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		// Alice enable as collateral DOT pool.
		TestPools::enable_is_collateral_internal(&ALICE, DOT);

		assert!(<PoolUserParams<Test>>::get(DOT, ALICE).is_collateral);
	});
}

#[test]
fn disable_is_collateral_internal_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		// Alice disable collateral DOT pool.
		TestPools::disable_is_collateral_internal(&ALICE, DOT);

		assert!(!<PoolUserParams<Test>>::get(DOT, ALICE).is_collateral);
	});
}

#[test]
fn get_pool_available_liquidity_should_work() {
	ExtBuilder::default()
		.pool_balance(DOT, TEN_THOUSAND)
		.build()
		.execute_with(|| {
			assert_eq!(TestPools::get_pool_available_liquidity(DOT), TEN_THOUSAND);
		});
}

#[test]
fn get_pool_data_should_work() {
	ExtBuilder::default()
		.pool_with_params(
			DOT,
			TEN_THOUSAND,
			Rate::saturating_from_rational(125, 100),
			TEN_THOUSAND,
		)
		.build()
		.execute_with(|| {
			assert_eq!(
				TestPools::get_pool_data(DOT),
				Pool {
					borrowed: TEN_THOUSAND,
					borrow_index: Rate::saturating_from_rational(125, 100),
					protocol_interest: TEN_THOUSAND,
				}
			);
		});
}

#[test]
fn get_pool_borrow_underlying_should_work() {
	ExtBuilder::default()
		.pool_with_params(DOT, TEN_THOUSAND, Rate::default(), Balance::default())
		.build()
		.execute_with(|| {
			assert_eq!(TestPools::get_pool_borrow_underlying(DOT), TEN_THOUSAND);
		});
}

#[test]
fn get_pool_protocol_interest_should_work() {
	ExtBuilder::default()
		.pool_with_params(DOT, Balance::default(), Rate::default(), TEN_THOUSAND)
		.build()
		.execute_with(|| {
			assert_eq!(TestPools::get_pool_protocol_interest(DOT), TEN_THOUSAND);
		});
}

#[test]
fn get_pool_borrow_index_should_work() {
	ExtBuilder::default()
		.pool_with_params(
			DOT,
			Balance::default(),
			Rate::saturating_from_rational(125, 100),
			Balance::default(),
		)
		.build()
		.execute_with(|| {
			assert_eq!(
				TestPools::get_pool_borrow_index(DOT),
				Rate::saturating_from_rational(125, 100)
			);
		});
}

#[test]
fn get_user_borrow_balance_should_work() {
	ExtBuilder::default()
		.pool_user_data_with_params(DOT, ALICE, ONE_HUNDRED, Rate::default(), true, 0)
		.build()
		.execute_with(|| {
			assert_eq!(TestPools::get_user_borrow_balance(&ALICE, DOT), ONE_HUNDRED);
		});
}

#[test]
fn check_user_available_is_collateral_should_work() {
	ExtBuilder::default()
		.pool_user_data_with_params(DOT, ALICE, Balance::default(), Rate::default(), false, 0)
		.build()
		.execute_with(|| {
			// collateral parameter is set to false
			assert!(!TestPools::check_user_available_collateral(&ALICE, DOT));

			// set collateral parameter to true
			TestPools::enable_is_collateral_internal(&ALICE, DOT);

			assert!(TestPools::check_user_available_collateral(&ALICE, DOT));
		});
}

#[test]
fn pool_should_exists() {
	ExtBuilder::default().pool_mock(DOT).build().execute_with(|| {
		assert_eq!(TestPools::pool_exists(&DOT), true);
		assert_eq!(TestPools::pool_exists(&MDOT), false);
	});
}

#[test]
fn update_state_on_borrow_should_work() {
	ExtBuilder::default()
		.user_balance(ALICE, DOT, ONE_HUNDRED)
		.pool_mock(DOT)
		.build()
		.execute_with(|| {
			assert_eq!(TestPools::get_user_borrow_index(&ALICE, DOT), Rate::from_inner(0));

			// Alice borrow 60 DOT
			assert_ok!(TestPools::update_state_on_borrow(&ALICE, DOT, 60, 0));
			assert_eq!(TestPools::get_pool_borrow_underlying(DOT), 60);
			assert_eq!(TestPools::get_user_borrow_balance(&ALICE, DOT), 60);
			assert_eq!(TestPools::get_user_borrow_index(&ALICE, DOT), Rate::default());

			Pools::<Test>::mutate(DOT, |pool| pool.borrow_index = Rate::saturating_from_rational(1, 5));

			// ALice borrow 30 DOT
			assert_ok!(TestPools::update_state_on_borrow(&ALICE, DOT, 30, 60));
			assert_eq!(TestPools::get_pool_borrow_underlying(DOT), 90);
			assert_eq!(TestPools::get_user_borrow_balance(&ALICE, DOT), 90);
			assert_eq!(
				TestPools::get_user_borrow_index(&ALICE, DOT),
				Rate::saturating_from_rational(1, 5)
			);

			// Overflow in calculation: account_borrow_new = 90 + max_value()
			assert_noop!(
				TestPools::update_state_on_borrow(&ALICE, DOT, Balance::max_value(), 90),
				Error::<Test>::BorrowBalanceOverflow
			);
		});
}

#[test]
fn update_state_on_repay_should_work() {
	ExtBuilder::default()
		.user_balance(ALICE, DOT, ONE_HUNDRED)
		.build()
		.execute_with(|| {
			assert_eq!(TestPools::get_user_borrow_index(&ALICE, DOT), Rate::from_inner(0));
			assert_ok!(TestPools::update_state_on_borrow(&ALICE, DOT, dollars(60), 0));
			assert_eq!(TestPools::get_pool_borrow_underlying(DOT), dollars(60));
			assert_eq!(TestPools::get_user_borrow_balance(&ALICE, DOT), dollars(60));
			assert_eq!(TestPools::get_user_borrow_index(&ALICE, DOT), Rate::default());

			assert_ok!(TestPools::update_state_on_repay(&ALICE, DOT, dollars(30), dollars(60)));
			assert_eq!(TestPools::get_pool_borrow_underlying(DOT), dollars(30));
			assert_eq!(TestPools::get_user_borrow_balance(&ALICE, DOT), dollars(30));

			assert_ok!(TestPools::update_state_on_repay(&ALICE, DOT, dollars(10), dollars(30)));
			assert_eq!(TestPools::get_pool_borrow_underlying(DOT), dollars(20));
			assert_eq!(TestPools::get_user_borrow_balance(&ALICE, DOT), dollars(20));

			assert_noop!(
				TestPools::update_state_on_repay(&ALICE, DOT, 100, 20),
				Error::<Test>::RepayAmountTooBig
			);
		});
}

#[test]
fn get_user_liquidation_attempts_should_work() {
	ExtBuilder::default()
		.pool_user_data_with_params(DOT, ALICE, ONE_HUNDRED, Rate::default(), true, 12)
		.build()
		.execute_with(|| {
			assert_eq!(TestPools::get_user_liquidation_attempts(&ALICE, DOT), 12);
		});
}

#[test]
fn get_pool_members_with_loans_should_work() {
	ExtBuilder::default()
		.pool_user_data_with_params(DOT, ALICE, ONE_HUNDRED, Rate::default(), true, 0)
		.pool_user_data_with_params(DOT, BOB, 0, Rate::default(), true, 0)
		.pool_user_data_with_params(DOT, CHARLIE, 100, Rate::default(), true, 0)
		.pool_user_data_with_params(BTC, ALICE, 0, Rate::default(), true, 0)
		.pool_user_data_with_params(BTC, BOB, 0, Rate::default(), true, 0)
		.pool_user_data_with_params(BTC, CHARLIE, ONE_HUNDRED, Rate::default(), true, 0)
		.build()
		.execute_with(|| {
			assert_eq!(TestPools::get_pool_members_with_loans(DOT), Ok(vec![3, 1]));
			assert_eq!(TestPools::get_pool_members_with_loans(BTC), Ok(vec![3]));
		});
}

#[test]
fn check_user_has_collateral_should_work() {
	ExtBuilder::default()
		.pool_mock(DOT)
		.pool_mock(BTC)
		.pool_mock(ETH)
		.pool_user_data_with_params(DOT, ALICE, Balance::zero(), Rate::default(), true, 0)
		.pool_user_data_with_params(BTC, ALICE, Balance::zero(), Rate::default(), true, 0)
		.pool_user_data_with_params(ETH, ALICE, Balance::zero(), Rate::default(), true, 0)
		.pool_user_data_with_params(DOT, BOB, Balance::zero(), Rate::default(), true, 0)
		.pool_user_data_with_params(BTC, CHARLIE, Balance::zero(), Rate::default(), false, 0)
		.user_balance(ALICE, MDOT, Balance::zero())
		.user_balance(ALICE, MBTC, Balance::zero())
		.user_balance(ALICE, METH, TEN_THOUSAND)
		.user_balance(BOB, MDOT, Balance::zero())
		.user_balance(CHARLIE, MBTC, TEN_THOUSAND)
		.build()
		.execute_with(|| {
			assert_eq!(TestPools::check_user_has_collateral(&ALICE), true);
			assert_eq!(TestPools::check_user_has_collateral(&BOB), false);
			assert_eq!(TestPools::check_user_has_collateral(&CHARLIE), false);
		});
}
