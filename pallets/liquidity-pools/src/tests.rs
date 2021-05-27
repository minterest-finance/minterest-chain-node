#![cfg(test)]

use super::*;
use mock::*;

use frame_support::{assert_err, assert_noop, assert_ok};
use sp_arithmetic::FixedPointNumber;

fn dollars<T: Into<u128>>(d: T) -> Balance {
	1_000_000_000_000_000_000_u128.saturating_mul(d.into())
}

#[test]
fn set_pool_data_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		assert_ok!(TestPools::set_pool_data(
			DOT,
			ONE_HUNDRED_DOLLARS,
			Rate::saturating_from_rational(125, 100),
			ONE_HUNDRED_DOLLARS,
		));
		assert_eq!(<Pools<Test>>::get(DOT).total_borrowed, ONE_HUNDRED_DOLLARS);
		assert_eq!(
			<Pools<Test>>::get(DOT).borrow_index,
			Rate::saturating_from_rational(125, 100)
		);
		assert_eq!(<Pools<Test>>::get(DOT).total_protocol_interest, ONE_HUNDRED_DOLLARS);
	});
}

#[test]
fn set_pool_total_borrowed_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		// Set pool_total_borrowed eq 100 DOT
		TestPools::set_pool_total_borrowed(DOT, ONE_HUNDRED_DOLLARS);
		assert_eq!(<Pools<Test>>::get(DOT).total_borrowed, ONE_HUNDRED_DOLLARS);
	});
}

#[test]
fn set_pool_total_protocol_interest_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		// Set pool_total_protocol_interest eq 100 DOT.
		TestPools::set_pool_total_protocol_interest(DOT, ONE_HUNDRED_DOLLARS);
		assert_eq!(<Pools<Test>>::get(DOT).total_protocol_interest, ONE_HUNDRED_DOLLARS);
	});
}

#[test]
fn set_user_total_borrowed_and_interest_index_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		// Set user_total_borrowed eq 100 DOT and user_interest_index eq 0.33.
		TestPools::set_user_total_borrowed_and_interest_index(
			&ALICE,
			DOT,
			ONE_HUNDRED_DOLLARS,
			Rate::saturating_from_rational(33, 100),
		);
		assert_eq!(
			<PoolUserParams<Test>>::get(DOT, ALICE).total_borrowed,
			ONE_HUNDRED_DOLLARS
		);
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
					total_borrowed: TEN_THOUSAND,
					borrow_index: Rate::saturating_from_rational(125, 100),
					total_protocol_interest: TEN_THOUSAND,
				}
			);
		});
}

#[test]
fn get_pool_total_borrowed_should_work() {
	ExtBuilder::default()
		.pool_with_params(DOT, TEN_THOUSAND, Rate::default(), Balance::default())
		.build()
		.execute_with(|| {
			assert_eq!(TestPools::get_pool_total_borrowed(DOT), TEN_THOUSAND);
		});
}

#[test]
fn get_pool_total_protocol_interest_should_work() {
	ExtBuilder::default()
		.pool_with_params(DOT, Balance::default(), Rate::default(), TEN_THOUSAND)
		.build()
		.execute_with(|| {
			assert_eq!(TestPools::get_pool_total_protocol_interest(DOT), TEN_THOUSAND);
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
fn get_user_total_borrowed_should_work() {
	ExtBuilder::default()
		.pool_user_data_with_params(DOT, ALICE, ONE_HUNDRED_DOLLARS, Rate::default(), true, 0)
		.build()
		.execute_with(|| {
			assert_eq!(TestPools::get_user_total_borrowed(&ALICE, DOT), ONE_HUNDRED_DOLLARS);
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
		.user_balance(ALICE, DOT, ONE_HUNDRED_DOLLARS)
		.pool_mock(DOT)
		.build()
		.execute_with(|| {
			assert_eq!(TestPools::get_user_borrow_index(&ALICE, DOT), Rate::from_inner(0));

			// Alice borrow 60 DOT
			assert_ok!(TestPools::update_state_on_borrow(&ALICE, DOT, 60, 0));
			assert_eq!(TestPools::get_pool_total_borrowed(DOT), 60);
			assert_eq!(TestPools::get_user_total_borrowed(&ALICE, DOT), 60);
			assert_eq!(TestPools::get_user_borrow_index(&ALICE, DOT), Rate::default());

			Pools::<Test>::mutate(DOT, |pool| pool.borrow_index = Rate::saturating_from_rational(1, 5));

			// ALice borrow 30 DOT
			assert_ok!(TestPools::update_state_on_borrow(&ALICE, DOT, 30, 60));
			assert_eq!(TestPools::get_pool_total_borrowed(DOT), 90);
			assert_eq!(TestPools::get_user_total_borrowed(&ALICE, DOT), 90);
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
		.user_balance(ALICE, DOT, ONE_HUNDRED_DOLLARS)
		.build()
		.execute_with(|| {
			assert_eq!(TestPools::get_user_borrow_index(&ALICE, DOT), Rate::from_inner(0));
			assert_ok!(TestPools::update_state_on_borrow(&ALICE, DOT, 60, 0));
			assert_eq!(TestPools::get_pool_total_borrowed(DOT), 60);
			assert_eq!(TestPools::get_user_total_borrowed(&ALICE, DOT), 60);
			assert_eq!(TestPools::get_user_borrow_index(&ALICE, DOT), Rate::default());

			assert_ok!(TestPools::update_state_on_repay(&ALICE, DOT, 30, 60));
			assert_eq!(TestPools::get_pool_total_borrowed(DOT), 30);
			assert_eq!(TestPools::get_user_total_borrowed(&ALICE, DOT), 30);

			assert_ok!(TestPools::update_state_on_repay(&ALICE, DOT, 10, 30));
			assert_eq!(TestPools::get_pool_total_borrowed(DOT), 20);
			assert_eq!(TestPools::get_user_total_borrowed(&ALICE, DOT), 20);

			assert_noop!(
				TestPools::update_state_on_repay(&ALICE, DOT, 100, 20),
				Error::<Test>::RepayAmountTooBig
			);
		});
}

#[test]
fn convert_to_wrapped_should_work() {
	ExtBuilder::default()
		.user_balance(ALICE, DOT, ONE_HUNDRED)
		.user_balance(ALICE, MDOT, ONE_HUNDRED)
		.pool_total_borrowed(DOT, 40)
		.build()
		.execute_with(|| {
			// exchange_rate = 40 / 100 = 0.4
			assert_eq!(TestPools::convert_to_wrapped(DOT, 10), Ok(25));

			// Overflow in calculation: wrapped_amount = max_value() / exchange_rate,
			// when exchange_rate < 1
			assert_err!(
				TestPools::convert_to_wrapped(DOT, Balance::max_value()),
				Error::<Test>::ConversionError
			);
		});
}

#[test]
fn convert_from_wrapped_should_work() {
	ExtBuilder::default()
		.pool_with_params(DOT, Balance::zero(), Rate::zero(), Balance::zero())
		.pool_with_params(BTC, Balance::zero(), Rate::zero(), Balance::zero())
		.user_balance(ALICE, DOT, ONE_HUNDRED)
		.user_balance(ALICE, MDOT, ONE_HUNDRED)
		.user_balance(ALICE, MBTC, 1)
		.pool_balance(BTC, 100)
		.pool_total_borrowed(DOT, 40)
		.build()
		.execute_with(|| {
			// underlying_amount = 10 * 0.4 = 4
			assert_eq!(TestPools::convert_from_wrapped(MDOT, 10), Ok(4));

			// Overflow in calculation: underlying_amount = max_value() * exchange_rate
			assert_err!(
				TestPools::convert_from_wrapped(MBTC, Balance::max_value()),
				Error::<Test>::ConversionError
			);
		});
}

#[test]
fn calculate_exchange_rate_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		// exchange_rate = (102 - 2 + 20) / 100 = 1.2
		assert_eq!(
			TestPools::calculate_exchange_rate(102, 100, 2, 20),
			Ok(Rate::saturating_from_rational(12, 10))
		);
		// If there are no tokens minted: exchangeRate = InitialExchangeRate = 1.0
		assert_eq!(
			TestPools::calculate_exchange_rate(102, 0, 2, 0),
			Ok(Rate::saturating_from_rational(1, 1))
		);

		// Overflow in calculation: total_cash + total_borrowed
		assert_noop!(
			TestPools::calculate_exchange_rate(Balance::max_value(), 100, 100, 100),
			Error::<Test>::ExchangeRateCalculationError
		);

		// Overflow in calculation: cash_plus_borrows - total_protocol_interest
		assert_noop!(
			TestPools::calculate_exchange_rate(100, 100, Balance::max_value(), 100),
			Error::<Test>::ExchangeRateCalculationError
		);
	});
}

#[test]
fn get_exchange_rate_should_work() {
	ExtBuilder::default()
		.pool_balance(DOT, dollars(100_u128))
		.user_balance(ALICE, MDOT, dollars(125_u128))
		.pool_total_borrowed(DOT, dollars(300_u128))
		.build()
		.execute_with(|| {
			// Pool needs to be created first
			assert_noop!(TestPools::get_exchange_rate(ETH), Error::<Test>::PoolNotFound);
			// exchange_rate = (100 - 0 + 300) / 125 = 3.2
			assert_eq!(
				TestPools::get_exchange_rate(DOT),
				Ok(Rate::saturating_from_rational(32, 10))
			);
		});
}

#[test]
fn get_exchange_rate_by_interest_params_should_work() {
	ExtBuilder::default()
		.pool_with_params(DOT, Balance::zero(), Rate::zero(), Balance::zero())
		.pool_balance(DOT, dollars(100_u128))
		.user_balance(ALICE, MDOT, dollars(125_u128))
		.build()
		.execute_with(|| {
			// Pool needs to be created first
			assert_noop!(
				TestPools::get_exchange_rate_by_interest_params(ETH, Balance::zero(), dollars(300_u128)),
				Error::<Test>::PoolNotFound
			);
			// Invalid protocol interest (more than pool balance) causes an error
			assert_noop!(
				TestPools::get_exchange_rate_by_interest_params(DOT, dollars(200_u128), Balance::zero()),
				Error::<Test>::ExchangeRateCalculationError
			);
			// exchange_rate = (100 - 0 + 300) / 125 = 3.2
			assert_eq!(
				TestPools::get_exchange_rate_by_interest_params(DOT, Balance::zero(), dollars(300_u128)),
				Ok(Rate::saturating_from_rational(32, 10))
			);
			// exchange_rate = (100 - 0 + 0) / 125 = 0.8
			assert_eq!(
				TestPools::get_exchange_rate_by_interest_params(DOT, Balance::zero(), Balance::zero()),
				Ok(Rate::saturating_from_rational(8, 10))
			);
			// exchange_rate = (100 - 100 + 0) / 125 = 0
			assert_eq!(
				TestPools::get_exchange_rate_by_interest_params(DOT, dollars(100_u128), Balance::zero()),
				Ok(Rate::zero())
			);
		});
}

#[test]
fn get_user_liquidation_attempts_should_work() {
	ExtBuilder::default()
		.pool_user_data_with_params(DOT, ALICE, ONE_HUNDRED_DOLLARS, Rate::default(), true, 12)
		.build()
		.execute_with(|| {
			assert_eq!(TestPools::get_user_liquidation_attempts(&ALICE, DOT), 12);
		});
}

#[test]
fn get_pool_members_with_loans_should_work() {
	ExtBuilder::default()
		.pool_user_data_with_params(DOT, ALICE, ONE_HUNDRED_DOLLARS, Rate::default(), true, 0)
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
fn get_is_collateral_pools_should_work() {
	ExtBuilder::default()
		.pool_balance(KSM, 1 * TEN_THOUSAND)
		.pool_balance(DOT, 3 * TEN_THOUSAND)
		.pool_balance(ETH, 2 * TEN_THOUSAND)
		.pool_balance(BTC, 4 * TEN_THOUSAND)
		.pool_total_borrowed(KSM, ONE_HUNDRED_DOLLARS)
		.pool_total_borrowed(DOT, ONE_HUNDRED_DOLLARS)
		.pool_total_borrowed(ETH, ONE_HUNDRED_DOLLARS)
		.pool_total_borrowed(BTC, ONE_HUNDRED_DOLLARS)
		.pool_user_data_with_params(KSM, ALICE, Balance::zero(), Rate::default(), true, 0)
		.pool_user_data_with_params(DOT, ALICE, Balance::zero(), Rate::default(), true, 0)
		.pool_user_data_with_params(ETH, ALICE, Balance::zero(), Rate::default(), true, 0)
		.pool_user_data_with_params(BTC, ALICE, Balance::zero(), Rate::default(), false, 0)
		.user_balance(ALICE, MKSM, Balance::zero())
		.user_balance(ALICE, MDOT, TEN_THOUSAND)
		.user_balance(ALICE, METH, TEN_THOUSAND)
		.user_balance(ALICE, MBTC, TEN_THOUSAND)
		.build()
		.execute_with(|| {
			assert_eq!(TestPools::get_is_collateral_pools(&ALICE), Ok(vec![DOT, ETH]));
			assert_eq!(TestPools::get_is_collateral_pools(&BOB), Ok(vec![]));
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
