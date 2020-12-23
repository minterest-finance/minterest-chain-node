use super::*;
use mock::*;

use frame_support::traits::schedule::DispatchTime;
use frame_support::{assert_noop, assert_ok, error::BadOrigin};
use sp_runtime::traits::{One, Zero};

#[test]
fn accrue_interest_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		assert_ok!(Controller::accrue_interest_rate(CurrencyId::DOT));
	});
}

#[test]
fn convert_to_wrapped_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		assert_ok!(Controller::convert_to_wrapped(CurrencyId::DOT, 10));
		assert_eq!(Controller::convert_to_wrapped(CurrencyId::DOT, 10), Ok(10));
	});
}

#[test]
fn calculate_interest_rate_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		assert_ok!(Controller::calculate_interest_rate(CurrencyId::DOT));
		assert_eq!(
			Controller::calculate_interest_rate(CurrencyId::DOT),
			Ok(Rate::saturating_from_rational(1, 1))
		);
	});
}

#[test]
fn convert_from_wrapped_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		assert_ok!(Controller::convert_from_wrapped(CurrencyId::MDOT, 10));
		assert_eq!(Controller::convert_from_wrapped(CurrencyId::MDOT, 10), Ok(10));
	});
}

#[test]
fn calculate_exchange_rate_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		assert_ok!(Controller::calculate_exchange_rate(10, 10));
		assert_eq!(
			Controller::calculate_exchange_rate(10, 10),
			Ok(Rate::saturating_from_rational(1, 1))
		);
	});
}

#[test]
fn get_exchange_rate_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		assert_ok!(Controller::get_exchange_rate(CurrencyId::DOT));
		assert_eq!(
			Controller::get_exchange_rate(CurrencyId::DOT),
			Ok(Rate::saturating_from_rational(1, 1))
		);
		assert_eq!(
			TestPools::reserves(&CurrencyId::DOT).current_exchange_rate,
			Rate::saturating_from_rational(1, 1)
		);
	});
}

#[test]
fn calculate_borrow_interest_rate_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		assert_ok!(Controller::calculate_borrow_interest_rate(100, 10, 10));
		assert_eq!(
			Controller::calculate_borrow_interest_rate(100, 10, 10),
			Ok(Rate::saturating_from_rational(1, 1))
		);
	});
}

#[test]
fn calculate_block_delta_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		assert_eq!(Controller::calculate_block_delta(10, 5), 5);
	});
}

#[test]
fn calculate_interest_factor_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		assert_ok!(Controller::calculate_interest_factor(
			Rate::saturating_from_rational(1, 10),
			&10
		));
		assert_eq!(
			Controller::calculate_interest_factor(Rate::saturating_from_rational(1, 10), &10),
			Ok(Rate::from_inner(0))
		);
	});
}

#[test]
fn calculate_interest_accumulated_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		assert_ok!(Controller::calculate_interest_accumulated(
			Rate::saturating_from_rational(1, 1),
			TestPools::reserves(CurrencyId::DOT).total_balance
		));
		assert_eq!(
			Controller::calculate_interest_accumulated(
				Rate::saturating_from_rational(0, 1),
				TestPools::reserves(CurrencyId::DOT).total_balance
			),
			Ok(0)
		);
	});
}

#[test]
fn calculate_new_total_borrow_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		assert_ok!(Controller::calculate_new_total_borrow(100, 100));
		assert_eq!(Controller::calculate_new_total_borrow(0, 100), Ok(100));
	});
}

#[test]
fn calculate_new_total_insurance_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		assert_ok!(Controller::calculate_new_total_insurance(
			100,
			Rate::saturating_from_rational(1, 1),
			100
		));
		assert_eq!(
			Controller::calculate_new_total_insurance(100, Rate::saturating_from_rational(0, 1), 250),
			Ok(250)
		);
	});
}
