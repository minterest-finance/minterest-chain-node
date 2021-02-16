//! Tests for the controller pallet.

use super::*;
use mock::*;

use frame_support::{assert_err, assert_noop, assert_ok};

#[test]
fn accrue_interest_should_work() {
	ExtBuilder::default()
		.pool_total_borrowed(CurrencyId::DOT, dollars(80_u128))
		.pool_mock(CurrencyId::BTC)
		.pool_balance(CurrencyId::DOT, dollars(20_u128))
		.build()
		.execute_with(|| {
			System::set_block_number(1);

			assert_ok!(Controller::accrue_interest_rate(CurrencyId::DOT));

			assert_eq!(Controller::controller_dates(CurrencyId::DOT).timestamp, 1);
			assert_eq!(TestPools::pools(CurrencyId::DOT).total_insurance, 57_600_000_000);
			assert_eq!(
				Controller::get_liquidity_pool_borrow_and_supply_rates(CurrencyId::DOT),
				Some((Rate::from_inner(139_680_000_267), Rate::from_inner(100_569_600_394)))
			);
			assert_eq!(
				TestPools::pools(CurrencyId::DOT).total_borrowed,
				80_000_000_576_000_000_000
			);
			assert_eq!(
				TestPools::pools(CurrencyId::DOT).borrow_index,
				Rate::from_inner(1_000_000_007_200_000_000)
			);
		});
}

#[test]
fn accrue_interest_should_not_work() {
	ExtBuilder::default()
		.pool_total_borrowed(CurrencyId::DOT, dollars(80_u128))
		.pool_balance(CurrencyId::DOT, dollars(20_u128))
		.build()
		.execute_with(|| {
			System::set_block_number(1);

			assert_ok!(Controller::accrue_interest_rate(CurrencyId::DOT));
			assert_eq!(Controller::controller_dates(CurrencyId::DOT).timestamp, 1);

			assert_ok!(Controller::set_max_borrow_rate(
				alice(),
				CurrencyId::DOT,
				1,
				1_000_000_000
			));

			System::set_block_number(20);

			assert_noop!(
				Controller::accrue_interest_rate(CurrencyId::DOT),
				Error::<Runtime>::BorrowRateIsTooHight
			);

			assert_ok!(Controller::set_max_borrow_rate(alice(), CurrencyId::DOT, 2, 1));

			assert_ok!(Controller::accrue_interest_rate(CurrencyId::DOT));
		});
}

#[test]
fn calculate_block_delta_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		// block_delta = 10 - 5 = 5
		assert_eq!(Controller::calculate_block_delta(10, 5), Ok(5));

		// Overflow in calculation: 5 - 10 = -5 < 0
		assert_noop!(Controller::calculate_block_delta(5, 10), Error::<Runtime>::NumOverflow);
	});
}

#[test]
fn calculate_interest_factor_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		// interest_factor = 0.1 * 25 = 2.5
		assert_eq!(
			Controller::calculate_interest_factor(Rate::saturating_from_rational(1, 10), &25),
			Ok(Rate::saturating_from_rational(25, 10))
		);

		// Overflow in calculation: block_delta * borrow_interest_rate
		assert_noop!(
			Controller::calculate_interest_factor(Rate::from_inner(u128::max_value()), &20),
			Error::<Runtime>::NumOverflow
		);
	});
}

#[test]
fn calculate_interest_accumulated_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		// interest_accumulated = 0 * 100 = 0
		assert_eq!(
			Controller::calculate_interest_accumulated(Rate::from_inner(0), 100),
			Ok(0)
		);

		// interest_accumulated = 0.03 * 200 = 6
		assert_eq!(
			Controller::calculate_interest_accumulated(
				Rate::saturating_from_rational(3, 100), // eq 0.03 == 3%
				200
			),
			Ok(6)
		);

		// Overflow in calculation: 1.1 * max_value()
		assert_noop!(
			Controller::calculate_interest_accumulated(Rate::saturating_from_rational(11, 10), Balance::max_value()),
			Error::<Runtime>::NumOverflow
		);
	});
}

#[test]
fn calculate_new_total_borrow_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		// new_total_borrows = 15 + 100 = 115
		assert_eq!(Controller::calculate_new_total_borrow(15, 100), Ok(115));

		// Overflow in calculation: 1 + max_value()
		assert_noop!(
			Controller::calculate_new_total_borrow(1, Balance::max_value()),
			Error::<Runtime>::NumOverflow
		);
	});
}

#[test]
fn calculate_new_total_insurance_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		// total_insurance_new = 100 * 1.2 + 250 = 370
		assert_eq!(
			Controller::calculate_new_total_insurance(100, Rate::saturating_from_rational(12, 10), 250),
			Ok(370)
		);
		// Overflow in calculation: max_value() * 1.1
		assert_noop!(
			Controller::calculate_new_total_insurance(Balance::max_value(), Rate::saturating_from_rational(11, 10), 1),
			Error::<Runtime>::NumOverflow
		);
		// Overflow in calculation: 100 * 1.1 + max_value()
		assert_noop!(
			Controller::calculate_new_total_insurance(100, Rate::saturating_from_rational(1, 1), Balance::max_value()),
			Error::<Runtime>::NumOverflow
		);
	});
}

#[test]
fn borrow_balance_stored_with_zero_balance_should_work() {
	ExtBuilder::default()
		.pool_user_data(CurrencyId::DOT, ALICE, Balance::zero(), Rate::from_inner(0), true, 0)
		.build()
		.execute_with(|| {
			// If borrow_balance = 0 then borrow_index is likely also 0, return Ok(0)
			assert_eq!(
				Controller::borrow_balance_stored(&ALICE, CurrencyId::DOT),
				Ok(Balance::zero())
			);
		});
}

#[test]
fn borrow_balance_stored_should_work() {
	ExtBuilder::default()
		.pool_mock(CurrencyId::DOT)
		.pool_user_data(
			CurrencyId::DOT,
			ALICE,
			100,
			Rate::saturating_from_rational(4, 1),
			true,
			0,
		)
		.build()
		.execute_with(|| {
			// recent_borrow_balance = 100 * 2 / 4 = 50
			assert_eq!(Controller::borrow_balance_stored(&ALICE, CurrencyId::DOT), Ok(50));
		});
}

#[test]
fn borrow_balance_stored_fails_if_num_overflow() {
	ExtBuilder::default()
		.pool_mock(CurrencyId::DOT)
		.pool_user_data(
			CurrencyId::DOT,
			ALICE,
			Balance::max_value(),
			Rate::saturating_from_rational(2, 1),
			true,
			0,
		)
		.pool_mock(CurrencyId::BTC)
		.pool_user_data(CurrencyId::DOT, ALICE, 100, Rate::from_inner(0), true, 0)
		.build()
		.execute_with(|| {
			assert_noop!(
				Controller::borrow_balance_stored(&ALICE, CurrencyId::DOT),
				Error::<Runtime>::NumOverflow
			);
		});
}

#[test]
fn calculate_utilization_rate_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		// if current_total_borrowed_balance == 0 then return Ok(0)
		assert_eq!(
			Controller::calculate_utilization_rate(100, 0, 60),
			Ok(Rate::from_inner(0))
		);
		// utilization_rate = 80 / (22 + 80 - 2) = 0.8
		assert_eq!(
			Controller::calculate_utilization_rate(22, 80, 2),
			Ok(Rate::saturating_from_rational(8, 10))
		);

		// Overflow in calculation: total_balance + total_borrowed = max_value() + 80
		assert_noop!(
			Controller::calculate_utilization_rate(Balance::max_value(), 80, 2),
			Error::<Runtime>::NumOverflow
		);

		// Overflow in calculation: total_balance_total_borrowed_sum - total_insurance = ... - max_value()
		assert_noop!(
			Controller::calculate_utilization_rate(22, 80, Balance::max_value()),
			Error::<Runtime>::NumOverflow
		);

		// Overflow in calculation: total_borrows / 0
		assert_noop!(
			Controller::calculate_utilization_rate(100, 70, 170),
			Error::<Runtime>::NumOverflow
		);
	});
}

#[test]
fn calculate_supply_rate_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		// supply_rate = 0.75 * 0.23 * (1 - 0.1) = 0.15525
		assert_eq!(
			Controller::calculate_supply_interest_rate(
				Rate::saturating_from_rational(75, 100),
				Rate::saturating_from_rational(23, 100),
				Rate::saturating_from_rational(1, 10)
			),
			Ok(Rate::saturating_from_rational(15525, 100_000))
		);

		// Overflow in calculation: one_minus_insurance_factor = 1 - 2
		assert_noop!(
			Controller::calculate_supply_interest_rate(
				Rate::saturating_from_rational(75, 100),
				Rate::saturating_from_rational(23, 100),
				Rate::saturating_from_rational(2, 1)
			),
			Error::<Runtime>::NumOverflow
		);

		// Overflow in calculation: max_value() * 2.3 * (1 - 0.1)
		assert_noop!(
			Controller::calculate_supply_interest_rate(
				Rate::from_inner(u128::max_value()),
				Rate::saturating_from_rational(23, 10),
				Rate::saturating_from_rational(1, 10)
			),
			Error::<Runtime>::NumOverflow
		);
	});
}

#[test]
fn calculate_new_borrow_index_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		// new_borrow_index = 0.0000063 * 1.28 + 1.28 = 1.280000008064
		assert_eq!(
			Controller::calculate_new_borrow_index(
				Rate::saturating_from_rational(63u128, 10_000_000_000u128),
				Rate::saturating_from_rational(128, 100)
			),
			Ok(Rate::from_inner(1_280_000_008_064_000_000))
		);

		// Overflow in calculation: simple_interest_factor * max_value()
		assert_noop!(
			Controller::calculate_new_borrow_index(
				Rate::saturating_from_rational(12, 10),
				Rate::from_inner(u128::max_value())
			),
			Error::<Runtime>::NumOverflow
		);

		// Overflow in calculation: simple_interest_factor_mul_borrow_index + max_value()
		assert_noop!(
			Controller::calculate_new_borrow_index(
				Rate::saturating_from_rational(1, 1),
				Rate::from_inner(u128::max_value())
			),
			Error::<Runtime>::NumOverflow
		);
	});
}

#[test]
fn mul_price_and_balance_add_to_prev_value_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		// 20 + 20 * 0.9 = 38
		assert_eq!(
			Controller::mul_price_and_balance_add_to_prev_value(20, 20, Rate::saturating_from_rational(9, 10)),
			Ok(38)
		);
		// 120_000 + 85_000 * 0.87 = 193_950
		assert_eq!(
			Controller::mul_price_and_balance_add_to_prev_value(
				120_000,
				85_000,
				Rate::saturating_from_rational(87, 100)
			),
			Ok(193950)
		);

		// Overflow in calculation: max_value() * 1.9
		assert_noop!(
			Controller::mul_price_and_balance_add_to_prev_value(
				100,
				Balance::max_value(),
				Rate::saturating_from_rational(19, 10)
			),
			Error::<Runtime>::NumOverflow
		);

		// Overflow in calculation: max_value() + 100 * 1.9
		assert_noop!(
			Controller::mul_price_and_balance_add_to_prev_value(
				Balance::max_value(),
				100,
				Rate::saturating_from_rational(19, 10)
			),
			Error::<Runtime>::NumOverflow
		);
	});
}

#[test]
fn get_hypothetical_account_liquidity_when_m_tokens_balance_is_zero_should_work() {
	ExtBuilder::default()
		.pool_user_data(CurrencyId::DOT, ALICE, Balance::zero(), Rate::from_inner(0), true, 0)
		.pool_user_data(CurrencyId::BTC, BOB, Balance::zero(), Rate::from_inner(0), false, 0)
		.build()
		.execute_with(|| {
			// Checking the function when called from redeem.
			// The function should return the shortfall to a large zero.
			assert_eq!(
				Controller::get_hypothetical_account_liquidity(&ALICE, CurrencyId::DOT, 5, 0),
				Ok((0, 9))
			);
			// Checking the function when called from borrow.
			// The function should return the shortfall to a large zero.
			assert_eq!(
				Controller::get_hypothetical_account_liquidity(&ALICE, CurrencyId::DOT, 0, 10),
				Ok((0, 20))
			);
			// Checking scenario: the user tries to take a borrow in a currency which is not
			// pool as available for collateral, and he fails.
			assert_eq!(
				Controller::get_hypothetical_account_liquidity(&BOB, CurrencyId::BTC, 0, 10),
				Ok((0, 20))
			);
		});
}

#[test]
fn get_hypothetical_account_liquidity_one_currency_from_redeem_should_work() {
	ExtBuilder::default().alice_deposit_60_dot().build().execute_with(|| {
		// Checking the function when called from redeem.
		// collateral parameter is set to false, user can't redeem.
		assert_eq!(
			Controller::get_hypothetical_account_liquidity(&ALICE, CurrencyId::DOT, 5, 0),
			Ok((0, 9))
		);
		assert_eq!(
			Controller::get_hypothetical_account_liquidity(&ALICE, CurrencyId::DOT, 60, 0),
			Ok((0, 108))
		);
		assert_eq!(
			Controller::get_hypothetical_account_liquidity(&ALICE, CurrencyId::DOT, 200, 0),
			Ok((0, 360))
		);
	});
}

#[test]
fn get_hypothetical_account_liquidity_two_currencies_from_redeem_should_work() {
	ExtBuilder::default()
		.alice_deposit_60_dot()
		.alice_deposit_20_eth()
		.build()
		.execute_with(|| {
			// Checking the function when called from redeem.
			// collateral parameter is set to false, user can't redeem.
			assert_eq!(
				Controller::get_hypothetical_account_liquidity(&ALICE, CurrencyId::ETH, 15, 0),
				Ok((0, 27))
			);
			assert_eq!(
				Controller::get_hypothetical_account_liquidity(&ALICE, CurrencyId::ETH, 80, 0),
				Ok((0, 144))
			);
			assert_eq!(
				Controller::get_hypothetical_account_liquidity(&ALICE, CurrencyId::ETH, 100, 0),
				Ok((0, 180))
			);
		});
}

#[test]
fn get_hypothetical_account_liquidity_two_currencies_from_borrow_should_work() {
	ExtBuilder::default()
		.alice_deposit_20_eth()
		// ALICE deposit 60 DOT and borrow 30 DOT
		.user_balance(ALICE, CurrencyId::DOT, 70)
		.user_balance(ALICE, CurrencyId::MDOT, 60)
		.pool_balance(CurrencyId::DOT, 60)
		.pool_total_borrowed(CurrencyId::DOT, 30)
		.pool_user_data(
			CurrencyId::DOT,
			ALICE,
			30,
			Rate::saturating_from_rational(1, 1),
			false,
			0,
		)
		.build()
		.execute_with(|| {
			// Checking the function when called from borrow.
			// collateral parameter for DOT and ETH pool is set to false. User can't borrow.
			assert_eq!(
				Controller::get_hypothetical_account_liquidity(&ALICE, CurrencyId::DOT, 0, 30),
				Ok((0, 120))
			);

			// Alice set collateral parameter value to true for DOT pool. Alice can borrow.
			assert_ok!(<LiquidityPools<Runtime>>::enable_as_collateral_internal(
				&ALICE,
				CurrencyId::DOT
			));

			assert_eq!(
				Controller::get_hypothetical_account_liquidity(&ALICE, CurrencyId::DOT, 0, 50),
				Ok((2, 0))
			);
			assert_eq!(
				Controller::get_hypothetical_account_liquidity(&ALICE, CurrencyId::DOT, 0, 100),
				Ok((0, 98))
			);
		});
}

#[test]
fn get_liquidity_pool_exchange_rate_should_work() {
	ExtBuilder::default()
		.pool_balance(CurrencyId::DOT, dollars(100_u128))
		.user_balance(ALICE, CurrencyId::MDOT, dollars(125_u128))
		.pool_total_borrowed(CurrencyId::DOT, dollars(300_u128))
		.build()
		.execute_with(|| {
			// exchange_rate = (100 - 0 + 300) / 125 = 3.2
			assert_eq!(
				Controller::get_liquidity_pool_exchange_rate(CurrencyId::DOT),
				Some(Rate::saturating_from_rational(32, 10))
			);
		});
}

#[test]
fn get_liquidity_pool_borrow_and_supply_rates_less_than_kink() {
	ExtBuilder::default()
		.pool_balance(CurrencyId::DOT, dollars(100_u128))
		.pool_total_borrowed(CurrencyId::DOT, dollars(300_u128))
		.build()
		.execute_with(|| {
			// utilization_rate = 300 / (100 - 0 + 300) = 0.75 < kink = 0.8
			// borrow_rate = 0.75 * 0.000_000_009 + 0 = 0.00000000675
			// supply_rate = 0.75 * 0.00_000_000_675 * (1 - 0.1) = 0.00000000455625
			assert_eq!(
				Controller::get_liquidity_pool_borrow_and_supply_rates(CurrencyId::DOT),
				Some((Rate::from_inner(6750000000), Rate::from_inner(4556250000)))
			);
		});
}

#[test]
fn get_liquidity_pool_borrow_and_supply_rates_above_kink() {
	ExtBuilder::default()
		.pool_balance(CurrencyId::DOT, dollars(100_u128))
		.pool_total_borrowed(CurrencyId::DOT, dollars(500_u128))
		.build()
		.execute_with(|| {
			// utilization_rate = 500 / (100 - 0 + 500) = 0.83 > kink = 0.8
			// borrow_rate = 0.83 * 0.8 * 0.000_000_207  + (0.8 * 0.000_000_009) + 0 = 0.0000001452
			// supply_rate = 0.83 * 0.0000001452 * (1 - 0.1) = 0.00000000455625
			assert_eq!(
				Controller::get_liquidity_pool_borrow_and_supply_rates(CurrencyId::DOT),
				Some((Rate::from_inner(145199999999), Rate::from_inner(108899999999)))
			);
		});
}

#[test]
fn redeem_allowed_should_work() {
	ExtBuilder::default().alice_deposit_60_dot().build().execute_with(|| {
		assert_ok!(Controller::redeem_allowed(CurrencyId::DOT, &ALICE, dollars(40_u128)));

		// collateral parameter is set to true.
		assert_ok!(<LiquidityPools<Runtime>>::enable_as_collateral_internal(
			&ALICE,
			CurrencyId::DOT
		));

		assert_err!(
			Controller::redeem_allowed(CurrencyId::DOT, &ALICE, dollars(100_u128)),
			Error::<Runtime>::InsufficientLiquidity
		);
	});
}

#[test]
fn borrow_allowed_should_work() {
	ExtBuilder::default().alice_deposit_60_dot().build().execute_with(|| {
		// collateral parameter is set to false. User can't borrow
		assert_err!(
			Controller::borrow_allowed(CurrencyId::DOT, &ALICE, dollars(10_u128)),
			Error::<Runtime>::InsufficientLiquidity
		);

		// collateral parameter is set to true. User can borrow.
		assert_ok!(<LiquidityPools<Runtime>>::enable_as_collateral_internal(
			&ALICE,
			CurrencyId::DOT
		));

		assert_ok!(Controller::borrow_allowed(CurrencyId::DOT, &ALICE, dollars(10_u128)));

		assert_noop!(
			Controller::borrow_allowed(CurrencyId::DOT, &ALICE, dollars(999_u128)),
			Error::<Runtime>::InsufficientLiquidity
		);
	});
}

#[test]
fn is_operation_allowed_should_work() {
	ExtBuilder::default()
		.pool_mock(CurrencyId::DOT)
		.build()
		.execute_with(|| {
			assert_eq!(
				Controller::is_operation_allowed(CurrencyId::DOT, Operation::Deposit),
				true
			);
			assert_eq!(
				Controller::is_operation_allowed(CurrencyId::DOT, Operation::Redeem),
				true
			);
			assert_eq!(
				Controller::is_operation_allowed(CurrencyId::DOT, Operation::Borrow),
				true
			);
			assert_eq!(
				Controller::is_operation_allowed(CurrencyId::DOT, Operation::Repay),
				true
			);

			assert_ok!(Controller::pause_specific_operation(
				alice(),
				CurrencyId::DOT,
				Operation::Deposit
			));
			assert_ok!(Controller::pause_specific_operation(
				alice(),
				CurrencyId::DOT,
				Operation::Redeem
			));

			assert_eq!(
				Controller::is_operation_allowed(CurrencyId::DOT, Operation::Deposit),
				false
			);
			assert_eq!(
				Controller::is_operation_allowed(CurrencyId::DOT, Operation::Redeem),
				false
			);
			assert_eq!(
				Controller::is_operation_allowed(CurrencyId::DOT, Operation::Borrow),
				true
			);
			assert_eq!(
				Controller::is_operation_allowed(CurrencyId::DOT, Operation::Repay),
				true
			);
		});
}

/* ----------------------------------------------------------------------------------------- */

// Admin functions

#[test]
fn set_insurance_factor_should_work() {
	ExtBuilder::default()
		.pool_mock(CurrencyId::DOT)
		.build()
		.execute_with(|| {
			// ALICE set insurance factor equal 2.0
			assert_ok!(Controller::set_insurance_factor(alice(), CurrencyId::DOT, 20, 10));
			let expected_event = TestEvent::controller(Event::InsuranceFactorChanged);
			assert!(System::events().iter().any(|record| record.event == expected_event));
			assert_eq!(
				Controller::controller_dates(CurrencyId::DOT).insurance_factor,
				Rate::saturating_from_rational(20, 10)
			);

			// ALICE set insurance factor equal 0.0
			assert_ok!(Controller::set_insurance_factor(alice(), CurrencyId::DOT, 0, 1));
			let expected_event = TestEvent::controller(Event::InsuranceFactorChanged);
			assert!(System::events().iter().any(|record| record.event == expected_event));
			assert_eq!(
				Controller::controller_dates(CurrencyId::DOT).insurance_factor,
				Rate::from_inner(0)
			);

			// Overflow in calculation: 20 / 0
			assert_noop!(
				Controller::set_insurance_factor(alice(), CurrencyId::DOT, 20, 0),
				Error::<Runtime>::NumOverflow
			);

			// The dispatch origin of this call must be Administrator.
			assert_noop!(
				Controller::set_insurance_factor(bob(), CurrencyId::DOT, 20, 10),
				Error::<Runtime>::RequireAdmin
			);

			assert_noop!(
				Controller::set_insurance_factor(alice(), CurrencyId::MDOT, 20, 10),
				Error::<Runtime>::PoolNotFound
			);
		});
}

#[test]
fn set_max_borrow_rate_should_work() {
	ExtBuilder::default()
		.pool_mock(CurrencyId::DOT)
		.build()
		.execute_with(|| {
			// ALICE set max borrow rate equal 2.0
			assert_ok!(Controller::set_max_borrow_rate(alice(), CurrencyId::DOT, 20, 10));
			let expected_event = TestEvent::controller(Event::MaxBorrowRateChanged);
			assert!(System::events().iter().any(|record| record.event == expected_event));
			assert_eq!(
				Controller::controller_dates(CurrencyId::DOT).max_borrow_rate,
				Rate::saturating_from_rational(20, 10)
			);

			// ALICE can't set max borrow rate equal 0.0
			assert_noop!(
				Controller::set_max_borrow_rate(alice(), CurrencyId::DOT, 0, 1),
				Error::<Runtime>::MaxBorrowRateCannotBeZero
			);

			// Overflow in calculation: 20 / 0
			assert_noop!(
				Controller::set_max_borrow_rate(alice(), CurrencyId::DOT, 20, 0),
				Error::<Runtime>::NumOverflow
			);

			// The dispatch origin of this call must be Administrator.
			assert_noop!(
				Controller::set_max_borrow_rate(bob(), CurrencyId::DOT, 20, 10),
				Error::<Runtime>::RequireAdmin
			);

			assert_noop!(
				Controller::set_max_borrow_rate(alice(), CurrencyId::MDOT, 20, 10),
				Error::<Runtime>::PoolNotFound
			);
		});
}

#[test]
fn pause_specific_operation_should_work() {
	ExtBuilder::default()
		.pool_mock(CurrencyId::DOT)
		.build()
		.execute_with(|| {
			assert_eq!(Controller::pause_keepers(&CurrencyId::DOT).deposit_paused, false);
			assert_eq!(Controller::pause_keepers(&CurrencyId::DOT).redeem_paused, false);
			assert_eq!(Controller::pause_keepers(&CurrencyId::DOT).borrow_paused, false);
			assert_eq!(Controller::pause_keepers(&CurrencyId::DOT).repay_paused, false);
			assert_eq!(Controller::pause_keepers(&CurrencyId::DOT).transfer_paused, false);

			assert_ok!(Controller::pause_specific_operation(
				alice(),
				CurrencyId::DOT,
				Operation::Deposit
			));
			let expected_event = TestEvent::controller(Event::OperationIsPaused(CurrencyId::DOT, Operation::Deposit));
			assert!(System::events().iter().any(|record| record.event == expected_event));

			assert_ok!(Controller::pause_specific_operation(
				alice(),
				CurrencyId::DOT,
				Operation::Redeem
			));
			let expected_event = TestEvent::controller(Event::OperationIsPaused(CurrencyId::DOT, Operation::Redeem));
			assert!(System::events().iter().any(|record| record.event == expected_event));

			assert_ok!(Controller::pause_specific_operation(
				alice(),
				CurrencyId::DOT,
				Operation::Borrow
			));
			let expected_event = TestEvent::controller(Event::OperationIsPaused(CurrencyId::DOT, Operation::Borrow));
			assert!(System::events().iter().any(|record| record.event == expected_event));

			assert_ok!(Controller::pause_specific_operation(
				alice(),
				CurrencyId::DOT,
				Operation::Repay
			));
			let expected_event = TestEvent::controller(Event::OperationIsPaused(CurrencyId::DOT, Operation::Repay));
			assert!(System::events().iter().any(|record| record.event == expected_event));

			assert_ok!(Controller::pause_specific_operation(
				alice(),
				CurrencyId::DOT,
				Operation::Transfer
			));
			let expected_event = TestEvent::controller(Event::OperationIsPaused(CurrencyId::DOT, Operation::Transfer));
			assert!(System::events().iter().any(|record| record.event == expected_event));

			assert_eq!(Controller::pause_keepers(&CurrencyId::DOT).deposit_paused, true);
			assert_eq!(Controller::pause_keepers(&CurrencyId::DOT).redeem_paused, true);
			assert_eq!(Controller::pause_keepers(&CurrencyId::DOT).borrow_paused, true);
			assert_eq!(Controller::pause_keepers(&CurrencyId::DOT).repay_paused, true);
			assert_eq!(Controller::pause_keepers(&CurrencyId::DOT).transfer_paused, true);

			assert_noop!(
				Controller::pause_specific_operation(bob(), CurrencyId::DOT, Operation::Deposit),
				Error::<Runtime>::RequireAdmin
			);
			assert_noop!(
				Controller::pause_specific_operation(alice(), CurrencyId::MDOT, Operation::Redeem),
				Error::<Runtime>::PoolNotFound
			);
		});
}

#[test]
fn unpause_specific_operation_should_work() {
	ExtBuilder::default()
		.pool_mock(CurrencyId::DOT)
		.pool_mock(CurrencyId::KSM)
		.build()
		.execute_with(|| {
			assert_eq!(Controller::pause_keepers(&CurrencyId::KSM).deposit_paused, true);
			assert_eq!(Controller::pause_keepers(&CurrencyId::KSM).redeem_paused, true);
			assert_eq!(Controller::pause_keepers(&CurrencyId::KSM).borrow_paused, true);
			assert_eq!(Controller::pause_keepers(&CurrencyId::KSM).repay_paused, true);
			assert_eq!(Controller::pause_keepers(&CurrencyId::KSM).transfer_paused, true);

			assert_ok!(Controller::unpause_specific_operation(
				alice(),
				CurrencyId::KSM,
				Operation::Deposit
			));
			let expected_event = TestEvent::controller(Event::OperationIsUnPaused(CurrencyId::KSM, Operation::Deposit));
			assert!(System::events().iter().any(|record| record.event == expected_event));

			assert_ok!(Controller::unpause_specific_operation(
				alice(),
				CurrencyId::KSM,
				Operation::Redeem
			));
			let expected_event = TestEvent::controller(Event::OperationIsUnPaused(CurrencyId::KSM, Operation::Redeem));
			assert!(System::events().iter().any(|record| record.event == expected_event));

			assert_ok!(Controller::unpause_specific_operation(
				alice(),
				CurrencyId::KSM,
				Operation::Borrow
			));
			let expected_event = TestEvent::controller(Event::OperationIsUnPaused(CurrencyId::KSM, Operation::Borrow));
			assert!(System::events().iter().any(|record| record.event == expected_event));

			assert_ok!(Controller::unpause_specific_operation(
				alice(),
				CurrencyId::KSM,
				Operation::Repay
			));
			let expected_event = TestEvent::controller(Event::OperationIsUnPaused(CurrencyId::KSM, Operation::Repay));
			assert!(System::events().iter().any(|record| record.event == expected_event));

			assert_ok!(Controller::unpause_specific_operation(
				alice(),
				CurrencyId::KSM,
				Operation::Transfer
			));
			let expected_event = TestEvent::controller(Event::OperationIsUnPaused(CurrencyId::KSM, Operation::Transfer));
			assert!(System::events().iter().any(|record| record.event == expected_event));

			assert_eq!(Controller::pause_keepers(&CurrencyId::KSM).deposit_paused, false);
			assert_eq!(Controller::pause_keepers(&CurrencyId::KSM).redeem_paused, false);
			assert_eq!(Controller::pause_keepers(&CurrencyId::KSM).borrow_paused, false);
			assert_eq!(Controller::pause_keepers(&CurrencyId::KSM).repay_paused, false);
			assert_eq!(Controller::pause_keepers(&CurrencyId::KSM).transfer_paused, false);

			assert_noop!(
				Controller::unpause_specific_operation(bob(), CurrencyId::DOT, Operation::Deposit),
				Error::<Runtime>::RequireAdmin
			);
			assert_noop!(
				Controller::unpause_specific_operation(alice(), CurrencyId::MDOT, Operation::Redeem),
				Error::<Runtime>::PoolNotFound
			);
		});
}

#[test]
fn deposit_insurance_should_work() {
	ExtBuilder::default()
		.user_balance(ALICE, CurrencyId::DOT, ONE_HUNDRED)
		.pool_mock(CurrencyId::DOT)
		.build()
		.execute_with(|| {
			// ALICE deposit 100 DOT in pool insurance
			assert_ok!(Controller::deposit_insurance(alice(), CurrencyId::DOT, 100));
			let expected_event = TestEvent::controller(Event::DepositedInsurance(CurrencyId::DOT, 100));
			assert!(System::events().iter().any(|record| record.event == expected_event));

			assert_eq!(TestPools::get_pool_total_insurance(CurrencyId::DOT), 100);

			// Bob is not added to the allow-list of admins, so this action is not available for him.
			assert_noop!(
				Controller::deposit_insurance(bob(), CurrencyId::DOT, 101),
				Error::<Runtime>::RequireAdmin
			);
		});
}

#[test]
fn do_deposit_insurance_should_work() {
	ExtBuilder::default()
		.user_balance(ALICE, CurrencyId::DOT, ONE_HUNDRED)
		.user_balance(BOB, CurrencyId::BTC, ONE_HUNDRED)
		.pool_mock(CurrencyId::DOT)
		.pool_mock(CurrencyId::BTC)
		.build()
		.execute_with(|| {
			// ALICE deposit 60 DOT in pool insurance
			assert_ok!(Controller::do_deposit_insurance(&ALICE, CurrencyId::DOT, 60));
			assert_eq!(TestPools::get_pool_total_insurance(CurrencyId::DOT), 60);
			assert_eq!(Currencies::free_balance(CurrencyId::DOT, &ALICE), 40);
			assert_eq!(TestPools::get_pool_available_liquidity(CurrencyId::DOT), 60);

			// ALICE deposit 5 DOT in pool insurance
			assert_ok!(Controller::do_deposit_insurance(&ALICE, CurrencyId::DOT, 5));
			assert_eq!(TestPools::get_pool_total_insurance(CurrencyId::DOT), 65);
			assert_eq!(Currencies::free_balance(CurrencyId::DOT, &ALICE), 35);
			assert_eq!(TestPools::get_pool_available_liquidity(CurrencyId::DOT), 65);

			// Not enough balance to deposit insurance.
			assert_noop!(
				Controller::do_deposit_insurance(&ALICE, CurrencyId::DOT, 101),
				Error::<Runtime>::NotEnoughBalance
			);

			// There is no pool 'MDOT'.
			assert_noop!(
				Controller::do_deposit_insurance(&ALICE, CurrencyId::MDOT, 5),
				Error::<Runtime>::PoolNotFound
			);

			// Set BTC pool total insurance = max_value()
			assert_ok!(TestPools::set_pool_total_insurance(
				CurrencyId::BTC,
				Balance::max_value()
			));

			// Overflow in calculation: total_insurance = max_value() + 50
			assert_err!(
				Controller::do_deposit_insurance(&BOB, CurrencyId::BTC, 50),
				Error::<Runtime>::BalanceOverflowed
			);
		});
}

#[test]
fn redeem_insurance_should_work() {
	ExtBuilder::default()
		.user_balance(ALICE, CurrencyId::DOT, ONE_HUNDRED)
		.pool_total_insurance(CurrencyId::DOT, 1000)
		.pool_balance(CurrencyId::DOT, 1000)
		.build()
		.execute_with(|| {
			// ALICE redeem 100 DOT from pool insurance.
			assert_ok!(Controller::redeem_insurance(alice(), CurrencyId::DOT, 125));
			let expected_event = TestEvent::controller(Event::RedeemedInsurance(CurrencyId::DOT, 125));
			assert!(System::events().iter().any(|record| record.event == expected_event));

			assert_eq!(TestPools::get_pool_total_insurance(CurrencyId::DOT), 875);
			assert_eq!(
				Currencies::free_balance(CurrencyId::DOT, &TestPools::pools_account_id()),
				875,
			);

			// Bob is not added to the allow-list of admins, so this action is not available for him.
			assert_noop!(
				Controller::redeem_insurance(bob(), CurrencyId::DOT, 101),
				Error::<Runtime>::RequireAdmin
			);
		});
}

#[test]
fn do_redeem_insurance_should_work() {
	ExtBuilder::default()
		.user_balance(ALICE, CurrencyId::DOT, ONE_HUNDRED)
		.pool_total_insurance(CurrencyId::DOT, 1000)
		.build()
		.execute_with(|| {
			// ALICE redeem 150 DOT from pool insurance
			assert_ok!(Controller::do_redeem_insurance(&ALICE, CurrencyId::DOT, 150));
			assert_eq!(TestPools::get_pool_total_insurance(CurrencyId::DOT), 850);
			assert_eq!(Currencies::free_balance(CurrencyId::DOT, &ALICE), 250);
			assert_eq!(TestPools::get_pool_available_liquidity(CurrencyId::DOT), 850);

			// ALICE redeem 300 DOT from pool insurance
			assert_ok!(Controller::do_redeem_insurance(&ALICE, CurrencyId::DOT, 300));
			assert_eq!(TestPools::get_pool_total_insurance(CurrencyId::DOT), 550);
			assert_eq!(Currencies::free_balance(CurrencyId::DOT, &ALICE), 550);
			assert_eq!(TestPools::get_pool_available_liquidity(CurrencyId::DOT), 550);

			// Not enough balance to redeem insurance: 550 - 600 < 0
			assert_noop!(
				Controller::do_redeem_insurance(&ALICE, CurrencyId::DOT, 600),
				Error::<Runtime>::NotEnoughBalance
			);

			// There is no pool 'MDOT'.
			assert_noop!(
				Controller::do_redeem_insurance(&ALICE, CurrencyId::MDOT, 5),
				Error::<Runtime>::PoolNotFound
			);

			// Set DOT pool total insurance = max_value()
			assert_ok!(TestPools::set_pool_total_insurance(
				CurrencyId::DOT,
				Balance::max_value()
			));
			// Overflow in calculation: total_insurance = max_value() + 50
			assert_err!(
				Controller::do_deposit_insurance(&ALICE, CurrencyId::DOT, 50),
				Error::<Runtime>::BalanceOverflowed
			);
		});
}
