#![cfg(test)]

use super::*;
use mock::*;

use frame_support::{assert_err, assert_noop, assert_ok};
use sp_arithmetic::FixedPointNumber;

fn dollars<T: Into<u128>>(d: T) -> Balance {
	1_000_000_000_000_000_000_u128.saturating_mul(d.into())
}

#[test]
fn set_current_exchange_rate_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		// Set exchange_rate eq 1.2
		assert_ok!(TestPools::set_current_exchange_rate(
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
		assert_ok!(TestPools::set_pool_total_borrowed(CurrencyId::DOT, ONE_HUNDRED_DOLLARS));
		assert_eq!(<Pools>::get(CurrencyId::DOT).total_borrowed, ONE_HUNDRED_DOLLARS);
	});
}

#[test]
fn set_pool_borrow_index_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		// Set pool_borrow_index eq 0.25
		assert_ok!(TestPools::set_pool_borrow_index(
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
		assert_ok!(TestPools::set_pool_total_insurance(
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
		assert_ok!(TestPools::set_user_total_borrowed_and_interest_index(
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
		assert_ok!(TestPools::set_accrual_interest_params(
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
		assert_ok!(TestPools::enable_as_collateral_internal(&ALICE, CurrencyId::DOT));

		assert!(<PoolUserDates<Test>>::get(ALICE, CurrencyId::DOT).collateral);
	});
}

#[test]
fn disable_collateral_internal_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		// Alice disable collateral DOT pool.
		assert_ok!(TestPools::disable_collateral_internal(&ALICE, CurrencyId::DOT));

		assert!(!<PoolUserDates<Test>>::get(ALICE, CurrencyId::DOT).collateral);
	});
}

#[test]
fn get_pool_available_liquidity_should_work() {
	ExtBuilder::default()
		.pool_balance(CurrencyId::DOT, TEN_THOUSAND)
		.build()
		.execute_with(|| {
			assert_eq!(TestPools::get_pool_available_liquidity(CurrencyId::DOT), TEN_THOUSAND);
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
			assert_eq!(TestPools::get_pool_total_borrowed(CurrencyId::DOT), TEN_THOUSAND);
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
			assert_eq!(TestPools::get_pool_total_insurance(CurrencyId::DOT), TEN_THOUSAND);
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
				TestPools::get_pool_borrow_index(CurrencyId::DOT),
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
				TestPools::get_user_total_borrowed(&ALICE, CurrencyId::DOT),
				ONE_HUNDRED_DOLLARS
			);
		});
}

#[test]
fn check_user_available_collateral_should_work() {
	ExtBuilder::default()
		.pool_user_data_with_params(ALICE, CurrencyId::DOT, Balance::default(), Rate::default(), false)
		.build()
		.execute_with(|| {
			// collateral parameter is set to false
			assert!(!TestPools::check_user_available_collateral(&ALICE, CurrencyId::DOT));

			// set collateral parameter to true
			assert_ok!(TestPools::enable_as_collateral_internal(&ALICE, CurrencyId::DOT));

			assert!(TestPools::check_user_available_collateral(&ALICE, CurrencyId::DOT));
		});
}

#[test]
fn pool_should_exists() {
	ExtBuilder::default()
		.pool_mock(CurrencyId::DOT)
		.build()
		.execute_with(|| {
			assert_eq!(TestPools::pool_exists(&CurrencyId::DOT), true);
			assert_eq!(TestPools::pool_exists(&CurrencyId::MDOT), false);
		});
}

#[test]
fn update_state_on_borrow_should_work() {
	ExtBuilder::default()
		.user_balance(ALICE, CurrencyId::DOT, ONE_HUNDRED_DOLLARS)
		.pool_mock(CurrencyId::DOT)
		.build()
		.execute_with(|| {
			assert_eq!(
				TestPools::get_user_borrow_index(&ALICE, CurrencyId::DOT),
				Rate::from_inner(0)
			);

			// Alice borrow 60 DOT
			assert_ok!(TestPools::update_state_on_borrow(&ALICE, CurrencyId::DOT, 60, 0));
			assert_eq!(TestPools::get_pool_total_borrowed(CurrencyId::DOT), 60);
			assert_eq!(TestPools::get_user_total_borrowed(&ALICE, CurrencyId::DOT), 60);
			assert_eq!(
				TestPools::get_user_borrow_index(&ALICE, CurrencyId::DOT),
				Rate::default()
			);

			assert_ok!(TestPools::set_pool_borrow_index(
				CurrencyId::DOT,
				Rate::saturating_from_rational(1, 5)
			));

			// ALice borrow 30 DOT
			assert_ok!(TestPools::update_state_on_borrow(&ALICE, CurrencyId::DOT, 30, 60));
			assert_eq!(TestPools::get_pool_total_borrowed(CurrencyId::DOT), 90);
			assert_eq!(TestPools::get_user_total_borrowed(&ALICE, CurrencyId::DOT), 90);
			assert_eq!(
				TestPools::get_user_borrow_index(&ALICE, CurrencyId::DOT),
				Rate::saturating_from_rational(1, 5)
			);

			// Overflow in calculation: account_borrow_new = 90 + max_value()
			assert_noop!(
				TestPools::update_state_on_borrow(&ALICE, CurrencyId::DOT, Balance::max_value(), 90),
				Error::<Test>::NumOverflow
			);
		});
}

#[test]
fn update_state_on_repay_should_work() {
	ExtBuilder::default()
		.user_balance(ALICE, CurrencyId::DOT, ONE_HUNDRED_DOLLARS)
		.build()
		.execute_with(|| {
			assert_eq!(
				TestPools::get_user_borrow_index(&ALICE, CurrencyId::DOT),
				Rate::from_inner(0)
			);
			assert_ok!(TestPools::update_state_on_borrow(&ALICE, CurrencyId::DOT, 60, 0));
			assert_eq!(TestPools::get_pool_total_borrowed(CurrencyId::DOT), 60);
			assert_eq!(TestPools::get_user_total_borrowed(&ALICE, CurrencyId::DOT), 60);
			assert_eq!(
				TestPools::get_user_borrow_index(&ALICE, CurrencyId::DOT),
				Rate::default()
			);

			assert_ok!(TestPools::update_state_on_repay(&ALICE, CurrencyId::DOT, 30, 60));
			assert_eq!(TestPools::get_pool_total_borrowed(CurrencyId::DOT), 30);
			assert_eq!(TestPools::get_user_total_borrowed(&ALICE, CurrencyId::DOT), 30);

			assert_ok!(TestPools::update_state_on_repay(&ALICE, CurrencyId::DOT, 10, 30));
			assert_eq!(TestPools::get_pool_total_borrowed(CurrencyId::DOT), 20);
			assert_eq!(TestPools::get_user_total_borrowed(&ALICE, CurrencyId::DOT), 20);

			assert_noop!(
				TestPools::update_state_on_repay(&ALICE, CurrencyId::DOT, 100, 20),
				Error::<Test>::NumOverflow
			);
		});
}

#[test]
fn convert_to_wrapped_should_work() {
	ExtBuilder::default()
		.user_balance(ALICE, CurrencyId::DOT, ONE_HUNDRED)
		.user_balance(ALICE, CurrencyId::MDOT, ONE_HUNDRED)
		.pool_total_borrowed(CurrencyId::DOT, 40)
		.build()
		.execute_with(|| {
			// exchange_rate = 40 / 100 = 0.4
			assert_eq!(TestPools::convert_to_wrapped(CurrencyId::DOT, 10), Ok(25));

			// Overflow in calculation: wrapped_amount = max_value() / exchange_rate,
			// when exchange_rate < 1
			assert_err!(
				TestPools::convert_to_wrapped(CurrencyId::DOT, Balance::max_value()),
				Error::<Test>::NumOverflow
			);
		});
}

#[test]
fn convert_from_wrapped_should_work() {
	ExtBuilder::default()
		.user_balance(ALICE, CurrencyId::DOT, ONE_HUNDRED)
		.user_balance(ALICE, CurrencyId::MDOT, ONE_HUNDRED)
		.user_balance(ALICE, CurrencyId::MBTC, 1)
		.pool_balance(CurrencyId::BTC, 100)
		.pool_total_borrowed(CurrencyId::DOT, 40)
		.build()
		.execute_with(|| {
			// underlying_amount = 10 * 0.4 = 4
			assert_eq!(TestPools::convert_from_wrapped(CurrencyId::MDOT, 10), Ok(4));

			// Overflow in calculation: underlying_amount = max_value() * exchange_rate
			assert_err!(
				TestPools::convert_from_wrapped(CurrencyId::MBTC, Balance::max_value()),
				Error::<Test>::NumOverflow
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
			Error::<Test>::NumOverflow
		);

		// Overflow in calculation: cash_plus_borrows - total_insurance
		assert_noop!(
			TestPools::calculate_exchange_rate(100, 100, Balance::max_value(), 100),
			Error::<Test>::NumOverflow
		);
	});
}

#[test]
fn get_exchange_rate_should_work() {
	ExtBuilder::default()
		.pool_balance(CurrencyId::DOT, dollars(100_u128))
		.user_balance(ALICE, CurrencyId::MDOT, dollars(125_u128))
		.pool_total_borrowed(CurrencyId::DOT, dollars(300_u128))
		.build()
		.execute_with(|| {
			// exchange_rate = (100 - 0 + 300) / 125 = 3.2
			assert_eq!(
				TestPools::get_exchange_rate(CurrencyId::DOT),
				Ok(Rate::saturating_from_rational(32, 10))
			);
		});
}

#[test]
fn get_wrapped_id_by_underlying_asset_id_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		assert_eq!(
			TestPools::get_wrapped_id_by_underlying_asset_id(&CurrencyId::DOT),
			Ok(CurrencyId::MDOT)
		);
		assert_noop!(
			TestPools::get_wrapped_id_by_underlying_asset_id(&CurrencyId::MDOT),
			Error::<Test>::NotValidUnderlyingAssetId
		);
	});
}

#[test]
fn get_underlying_asset_id_by_wrapped_id_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		assert_eq!(
			TestPools::get_underlying_asset_id_by_wrapped_id(&CurrencyId::MDOT),
			Ok(CurrencyId::DOT)
		);
		assert_noop!(
			TestPools::get_underlying_asset_id_by_wrapped_id(&CurrencyId::DOT),
			Error::<Test>::NotValidWrappedTokenId
		);
	});
}
