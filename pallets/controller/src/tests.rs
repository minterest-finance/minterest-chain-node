//! Tests for the controller pallet.

use super::*;
use mock::*;

use frame_support::{assert_err, assert_noop, assert_ok};

fn dollars<T: Into<u128>>(d: T) -> Balance {
	1_000_000_000_000_000_000_u128.saturating_mul(d.into())
}

fn multiplier_per_block_equal_max_value() -> ControllerData<BlockNumber> {
	ControllerData {
		timestamp: 0,
		supply_rate: Rate::from_inner(0),
		borrow_rate: Rate::from_inner(0),
		insurance_factor: Rate::saturating_from_rational(101, 100),
		max_borrow_rate: Rate::saturating_from_rational(5, 1000),
		kink: Rate::saturating_from_rational(12, 10),
		base_rate_per_block: Rate::from_inner(0),
		multiplier_per_block: Rate::from_inner(u128::max_value()),
		jump_multiplier_per_block: Rate::saturating_from_rational(207, 1_000_000_000), // 1.09 PerYear
		collateral_factor: Rate::saturating_from_rational(9, 10),                      // 90%
	}
}

fn base_rate_per_block_equal_max_value() -> ControllerData<BlockNumber> {
	ControllerData {
		timestamp: 0,
		supply_rate: Rate::from_inner(0),
		borrow_rate: Rate::from_inner(0),
		insurance_factor: Rate::saturating_from_rational(101, 100),
		max_borrow_rate: Rate::saturating_from_rational(5, 1000),
		kink: Rate::saturating_from_rational(12, 10),
		base_rate_per_block: Rate::from_inner(u128::max_value()),
		multiplier_per_block: Rate::saturating_from_rational(9, 1_000_000_000), // 0.047304 PerYear
		jump_multiplier_per_block: Rate::saturating_from_rational(207, 1_000_000_000), // 1.09 PerYear
		collateral_factor: Rate::saturating_from_rational(9, 10),               // 90%
	}
}

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
				Controller::controller_dates(CurrencyId::DOT).borrow_rate,
				Rate::saturating_from_rational(72u128, 10_000_000_000u128)
			);
			assert_eq!(
				Controller::controller_dates(CurrencyId::DOT).supply_rate,
				Rate::from_inner(5_184_000_000)
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
fn convert_to_wrapped_should_work() {
	ExtBuilder::default()
		.user_balance(ALICE, CurrencyId::DOT, ONE_HUNDRED)
		.user_balance(ALICE, CurrencyId::MDOT, ONE_HUNDRED)
		.pool_total_borrowed(CurrencyId::DOT, 40)
		.build()
		.execute_with(|| {
			// exchange_rate = 40 / 100 = 0.4
			assert_eq!(Controller::convert_to_wrapped(CurrencyId::DOT, 10), Ok(25));

			// Overflow in calculation: wrapped_amount = max_value() / exchange_rate,
			// when exchange_rate < 1
			assert_err!(
				Controller::convert_to_wrapped(CurrencyId::DOT, Balance::max_value()),
				Error::<Runtime>::NumOverflow
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
			assert_eq!(Controller::convert_from_wrapped(CurrencyId::MDOT, 10), Ok(4));

			// Overflow in calculation: underlying_amount = max_value() * exchange_rate
			assert_err!(
				Controller::convert_from_wrapped(CurrencyId::MBTC, Balance::max_value()),
				Error::<Runtime>::NumOverflow
			);
		});
}

#[test]
fn calculate_exchange_rate_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		// exchange_rate = (102 - 2 + 20) / 100 = 1.2
		assert_eq!(
			Controller::calculate_exchange_rate(102, 100, 2, 20),
			Ok(Rate::saturating_from_rational(12, 10))
		);
		// If there are no tokens minted: exchangeRate = InitialExchangeRate = 1.0
		assert_eq!(
			Controller::calculate_exchange_rate(102, 0, 2, 0),
			Ok(Rate::saturating_from_rational(1, 1))
		);

		// Overflow in calculation: total_cash + total_borrowed
		assert_noop!(
			Controller::calculate_exchange_rate(Balance::max_value(), 100, 100, 100),
			Error::<Runtime>::NumOverflow
		);

		// Overflow in calculation: cash_plus_borrows - total_insurance
		assert_noop!(
			Controller::calculate_exchange_rate(100, 100, Balance::max_value(), 100),
			Error::<Runtime>::NumOverflow
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
				Controller::get_exchange_rate(CurrencyId::DOT),
				Ok(Rate::saturating_from_rational(32, 10))
			);
		});
}

#[test]
fn calculate_borrow_interest_rate_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		// Utilization rate less or equal than kink:
		// utilization_rate = 0.42
		// borrow_interest_rate = 0,42 * multiplier_per_block + base_rate_per_block
		assert_eq!(
			Controller::calculate_borrow_interest_rate(CurrencyId::DOT, Rate::saturating_from_rational(42, 100)),
			Ok(Rate::from_inner(3_780_000_000))
		);

		// Utilization rate larger than kink:
		// utilization_rate = 0.9
		// borrow_interest_rate = 0.9 * 0.8 * jump_multiplier_per_block +
		// + (0.8 * multiplier_per_block) + base_rate_per_block
		assert_eq!(
			Controller::calculate_borrow_interest_rate(CurrencyId::DOT, Rate::saturating_from_rational(9, 10)),
			Ok(Rate::from_inner(156_240_000_000))
		);
	});
}

#[test]
fn calculate_borrow_interest_rate_fails_if_overflow_kink_mul_multiplier() {
	ExtBuilder::default().build().execute_with(|| {
		let controller_data = multiplier_per_block_equal_max_value();
		<ControllerDates<Runtime>>::insert(CurrencyId::KSM, controller_data.clone());
		// utilization_rate > kink.
		// Overflow in calculation: kink * multiplier_per_block = 1.01 * max_value()
		assert_noop!(
			Controller::calculate_borrow_interest_rate(CurrencyId::KSM, Rate::saturating_from_rational(101, 100)),
			Error::<Runtime>::NumOverflow
		);
	});
}

#[test]
fn calculate_borrow_interest_rate_fails_if_overflow_add_base_rate_per_block() {
	ExtBuilder::default().build().execute_with(|| {
		let controller_data = base_rate_per_block_equal_max_value();
		<ControllerDates<Runtime>>::insert(CurrencyId::KSM, controller_data.clone());
		// utilization_rate > kink.
		// Overflow in calculation: kink_mul_multiplier + base_rate_per_block = ... + max_value()
		assert_noop!(
			Controller::calculate_borrow_interest_rate(CurrencyId::KSM, Rate::saturating_from_rational(9, 10)),
			Error::<Runtime>::NumOverflow
		);
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
fn get_wrapped_id_by_underlying_asset_id_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		assert_eq!(
			Controller::get_wrapped_id_by_underlying_asset_id(&CurrencyId::DOT),
			Ok(CurrencyId::MDOT)
		);
		assert_noop!(
			Controller::get_wrapped_id_by_underlying_asset_id(&CurrencyId::MDOT),
			Error::<Runtime>::NotValidUnderlyingAssetId
		);
	});
}

#[test]
fn get_underlying_asset_id_by_wrapped_id_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		assert_eq!(
			Controller::get_underlying_asset_id_by_wrapped_id(&CurrencyId::MDOT),
			Ok(CurrencyId::DOT)
		);
		assert_noop!(
			Controller::get_underlying_asset_id_by_wrapped_id(&CurrencyId::DOT),
			Error::<Runtime>::NotValidWrappedTokenId
		);
	});
}

#[test]
fn borrow_balance_stored_with_zero_balance_should_work() {
	ExtBuilder::default()
		.pool_user_data(ALICE, CurrencyId::DOT, Balance::zero(), Rate::from_inner(0), true)
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
		.pool_user_data(ALICE, CurrencyId::DOT, 100, Rate::saturating_from_rational(4, 1), true)
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
			ALICE,
			CurrencyId::DOT,
			Balance::max_value(),
			Rate::saturating_from_rational(2, 1),
			true,
		)
		.pool_mock(CurrencyId::BTC)
		.pool_user_data(ALICE, CurrencyId::DOT, 100, Rate::from_inner(0), true)
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
		.pool_user_data(ALICE, CurrencyId::DOT, Balance::zero(), Rate::from_inner(0), true)
		.pool_user_data(BOB, CurrencyId::BTC, Balance::zero(), Rate::from_inner(0), false)
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
		assert_eq!(
			Controller::get_hypothetical_account_liquidity(&ALICE, CurrencyId::DOT, 5, 0),
			Ok((99, 0))
		);
		assert_eq!(
			Controller::get_hypothetical_account_liquidity(&ALICE, CurrencyId::DOT, 60, 0),
			Ok((0, 0))
		);
		assert_eq!(
			Controller::get_hypothetical_account_liquidity(&ALICE, CurrencyId::DOT, 200, 0),
			Ok((0, 252))
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
			assert_eq!(
				Controller::get_hypothetical_account_liquidity(&ALICE, CurrencyId::ETH, 15, 0),
				Ok((117, 0))
			);
			assert_eq!(
				Controller::get_hypothetical_account_liquidity(&ALICE, CurrencyId::ETH, 80, 0),
				Ok((0, 0))
			);
			assert_eq!(
				Controller::get_hypothetical_account_liquidity(&ALICE, CurrencyId::ETH, 100, 0),
				Ok((0, 36))
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
		.pool_user_data(ALICE, CurrencyId::DOT, 30, Rate::saturating_from_rational(1, 1), true)
		.build()
		.execute_with(|| {
			// Checking the function when called from borrow.
			assert_eq!(
				Controller::get_hypothetical_account_liquidity(&ALICE, CurrencyId::DOT, 0, 30),
				Ok((78, 0))
			);
			assert_eq!(
				Controller::get_hypothetical_account_liquidity(&ALICE, CurrencyId::DOT, 0, 50),
				Ok((38, 0))
			);
			assert_eq!(
				Controller::get_hypothetical_account_liquidity(&ALICE, CurrencyId::DOT, 0, 100),
				Ok((0, 62))
			);
		});
}

#[test]
fn deposit_allowed_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		assert_ok!(Controller::deposit_allowed(CurrencyId::DOT, &BOB, 10));
		assert_noop!(
			Controller::deposit_allowed(CurrencyId::KSM, &BOB, 10),
			Error::<Runtime>::OperationPaused
		);
	});
}

#[test]
fn redeem_allowed_should_work() {
	ExtBuilder::default().alice_deposit_60_dot().build().execute_with(|| {
		assert_ok!(Controller::redeem_allowed(CurrencyId::DOT, &ALICE, 40));

		assert_noop!(
			Controller::redeem_allowed(CurrencyId::KSM, &ALICE, 10),
			Error::<Runtime>::OperationPaused
		);

		assert_noop!(
			Controller::redeem_allowed(CurrencyId::DOT, &ALICE, 999),
			Error::<Runtime>::InsufficientLiquidity
		);
	});
}

#[test]
fn borrow_allowed_should_work() {
	ExtBuilder::default().alice_deposit_60_dot().build().execute_with(|| {
		assert_ok!(Controller::borrow_allowed(CurrencyId::DOT, &ALICE, 10));

		assert_noop!(
			Controller::borrow_allowed(CurrencyId::KSM, &ALICE, 10),
			Error::<Runtime>::OperationPaused
		);

		assert_noop!(
			Controller::borrow_allowed(CurrencyId::DOT, &ALICE, 999),
			Error::<Runtime>::InsufficientLiquidity
		);
	});
}

#[test]
fn repay_allowed_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		assert_ok!(Controller::repay_borrow_allowed(CurrencyId::DOT, &BOB, 10));

		assert_noop!(
			Controller::repay_borrow_allowed(CurrencyId::KSM, &BOB, 10),
			Error::<Runtime>::OperationPaused
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
fn set_base_rate_per_block_should_work() {
	ExtBuilder::default()
		.pool_mock(CurrencyId::DOT)
		.build()
		.execute_with(|| {
			// Set Multiplier per block equal 2.0
			assert_ok!(Controller::set_base_rate_per_block(alice(), CurrencyId::DOT, 20, 10));

			// Can be set to 0.0
			assert_ok!(Controller::set_base_rate_per_block(alice(), CurrencyId::DOT, 0, 1));
			let expected_event = TestEvent::controller(Event::BaseRatePerBlockHasChanged);
			assert!(System::events().iter().any(|record| record.event == expected_event));
			assert_eq!(
				Controller::controller_dates(CurrencyId::DOT).base_rate_per_block,
				Rate::from_inner(0)
			);

			// ALICE set Baser rate per block equal 2.0
			assert_ok!(Controller::set_base_rate_per_block(alice(), CurrencyId::DOT, 20, 10));
			let expected_event = TestEvent::controller(Event::BaseRatePerBlockHasChanged);
			assert!(System::events().iter().any(|record| record.event == expected_event));
			assert_eq!(
				Controller::controller_dates(CurrencyId::DOT).base_rate_per_block,
				Rate::saturating_from_rational(2_000_000_000_000_000_000u128, BLOCKS_PER_YEAR)
			);

			// Base rate per block cannot be set to 0 at the same time as Multiplier per block.
			assert_ok!(Controller::set_multiplier_per_block(alice(), CurrencyId::DOT, 0, 1));
			assert_noop!(
				Controller::set_base_rate_per_block(alice(), CurrencyId::DOT, 0, 1),
				Error::<Runtime>::BaseRatePerBlockCannotBeZero
			);

			// Overflow in calculation: 20 / 0
			assert_noop!(
				Controller::set_base_rate_per_block(alice(), CurrencyId::DOT, 20, 0),
				Error::<Runtime>::NumOverflow
			);

			// The dispatch origin of this call must be Administrator.
			assert_noop!(
				Controller::set_base_rate_per_block(bob(), CurrencyId::DOT, 20, 10),
				Error::<Runtime>::RequireAdmin
			);

			assert_noop!(
				Controller::set_base_rate_per_block(alice(), CurrencyId::MDOT, 20, 10),
				Error::<Runtime>::PoolNotFound
			);
		});
}

#[test]
fn set_multiplier_per_block_should_work() {
	ExtBuilder::default()
		.pool_mock(CurrencyId::DOT)
		.build()
		.execute_with(|| {
			// Set Base rate per block equal 2.0
			assert_ok!(Controller::set_base_rate_per_block(alice(), CurrencyId::DOT, 20, 10));

			// Can be set to 0.0
			assert_ok!(Controller::set_multiplier_per_block(alice(), CurrencyId::DOT, 0, 10));
			let expected_event = TestEvent::controller(Event::MultiplierPerBlockHasChanged);
			assert!(System::events().iter().any(|record| record.event == expected_event));
			assert_eq!(
				Controller::controller_dates(CurrencyId::DOT).multiplier_per_block,
				Rate::from_inner(0)
			);

			// Alice set Multiplier per block equal 2.0 / 5_256_000
			assert_ok!(Controller::set_multiplier_per_block(alice(), CurrencyId::DOT, 20, 10));
			let expected_event = TestEvent::controller(Event::MultiplierPerBlockHasChanged);
			assert!(System::events().iter().any(|record| record.event == expected_event));
			assert_eq!(
				Controller::controller_dates(CurrencyId::DOT).multiplier_per_block,
				Rate::saturating_from_rational(2_000_000_000_000_000_000u128, BLOCKS_PER_YEAR)
			);

			//  Multiplier per block cannot be set to 0 at the same time as Base rate per block.
			assert_ok!(Controller::set_base_rate_per_block(alice(), CurrencyId::DOT, 0, 1));
			assert_noop!(
				Controller::set_multiplier_per_block(alice(), CurrencyId::DOT, 0, 1),
				Error::<Runtime>::MultiplierPerBlockCannotBeZero
			);

			// Overflow in calculation: 20 / 0
			assert_noop!(
				Controller::set_multiplier_per_block(alice(), CurrencyId::DOT, 20, 0),
				Error::<Runtime>::NumOverflow
			);

			// The dispatch origin of this call must be Administrator.
			assert_noop!(
				Controller::set_multiplier_per_block(bob(), CurrencyId::DOT, 20, 10),
				Error::<Runtime>::RequireAdmin
			);

			assert_noop!(
				Controller::set_multiplier_per_block(alice(), CurrencyId::MDOT, 20, 10),
				Error::<Runtime>::PoolNotFound
			);
		});
}

#[test]
fn set_jump_multiplier_per_block_should_work() {
	ExtBuilder::default()
		.pool_mock(CurrencyId::DOT)
		.build()
		.execute_with(|| {
			assert_ok!(Controller::set_jump_multiplier_per_block(
				alice(),
				CurrencyId::DOT,
				20,
				10
			));
			let expected_event = TestEvent::controller(Event::JumpMultiplierPerBlockHasChanged);
			assert!(System::events().iter().any(|record| record.event == expected_event));
			assert_eq!(
				Controller::controller_dates(CurrencyId::DOT).jump_multiplier_per_block,
				Rate::saturating_from_rational(2_000_000_000_000_000_000u128, BLOCKS_PER_YEAR)
			);

			// Overflow in calculation: 20 / 0
			assert_noop!(
				Controller::set_jump_multiplier_per_block(alice(), CurrencyId::DOT, 20, 0),
				Error::<Runtime>::NumOverflow
			);

			// The dispatch origin of this call must be Administrator.
			assert_noop!(
				Controller::set_jump_multiplier_per_block(bob(), CurrencyId::DOT, 20, 10),
				Error::<Runtime>::RequireAdmin
			);

			assert_noop!(
				Controller::set_jump_multiplier_per_block(alice(), CurrencyId::MDOT, 20, 10),
				Error::<Runtime>::PoolNotFound
			);
		});
}

#[test]
fn pool_not_found() {
	ExtBuilder::default()
		.pool_mock(CurrencyId::DOT)
		.build()
		.execute_with(|| {
			assert_noop!(
				Controller::pause_specific_operation(alice(), CurrencyId::MBTC, Operation::Deposit),
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

			assert_eq!(Controller::pause_keepers(&CurrencyId::DOT).deposit_paused, true);
			assert_eq!(Controller::pause_keepers(&CurrencyId::DOT).redeem_paused, true);
			assert_eq!(Controller::pause_keepers(&CurrencyId::DOT).borrow_paused, true);
			assert_eq!(Controller::pause_keepers(&CurrencyId::DOT).repay_paused, true);

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

			assert_eq!(Controller::pause_keepers(&CurrencyId::KSM).deposit_paused, false);
			assert_eq!(Controller::pause_keepers(&CurrencyId::KSM).redeem_paused, false);
			assert_eq!(Controller::pause_keepers(&CurrencyId::KSM).borrow_paused, false);
			assert_eq!(Controller::pause_keepers(&CurrencyId::KSM).repay_paused, false);

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
		.build()
		.execute_with(|| {
			// ALICE redeem 100 DOT from pool insurance.
			assert_ok!(Controller::redeem_insurance(alice(), CurrencyId::DOT, 125));
			let expected_event = TestEvent::controller(Event::RedeemedInsurance(CurrencyId::DOT, 125));
			assert!(System::events().iter().any(|record| record.event == expected_event));

			assert_eq!(TestPools::get_pool_total_insurance(CurrencyId::DOT), 875);

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
