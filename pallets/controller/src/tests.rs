use super::*;
use mock::*;

use frame_support::{assert_err, assert_noop, assert_ok};

#[test]
fn accrue_interest_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		System::set_block_number(1);
		assert_ok!(Controller::accrue_interest_rate(CurrencyId::DOT));
		assert_noop!(
			Controller::accrue_interest_rate(CurrencyId::ETH),
			Error::<Runtime>::OperationsLocked
		);
		//FIXME: add test for: MaxBorrowRate
	});
}

#[test]
fn convert_to_wrapped_should_work() {
	ExtBuilder::default()
		.exchange_rate_less_than_one()
		.build()
		.execute_with(|| {
			assert_ok!(Controller::convert_to_wrapped(CurrencyId::DOT, 10));
			assert_eq!(Controller::convert_to_wrapped(CurrencyId::DOT, 10), Ok(10));
			assert_err!(
				Controller::convert_to_wrapped(CurrencyId::BTC, Balance::max_value()),
				Error::<Runtime>::NumOverflow
			);
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
	ExtBuilder::default()
		.exchange_rate_greater_than_one()
		.build()
		.execute_with(|| {
			assert_ok!(Controller::convert_from_wrapped(CurrencyId::MDOT, 10));
			assert_eq!(Controller::convert_from_wrapped(CurrencyId::MDOT, 10), Ok(10));
			assert_err!(
				Controller::convert_from_wrapped(CurrencyId::MBTC, Balance::max_value()),
				Error::<Runtime>::NumOverflow
			);
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
		assert_eq!(
			Controller::calculate_exchange_rate(10, 0),
			Ok(Rate::saturating_from_rational(1, 1))
		)
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
		assert_eq!(Controller::calculate_block_delta(10, 5), Ok(5));
		assert_noop!(Controller::calculate_block_delta(5, 10), Error::<Runtime>::NumOverflow);
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
		assert_noop!(
			Controller::calculate_interest_accumulated(Rate::saturating_from_rational(11, 10), Balance::max_value()),
			Error::<Runtime>::NumOverflow
		);
	});
}

#[test]
fn calculate_new_total_borrow_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		assert_ok!(Controller::calculate_new_total_borrow(100, 100));
		assert_eq!(Controller::calculate_new_total_borrow(0, 100), Ok(100));
		assert_noop!(
			Controller::calculate_new_total_borrow(1, Balance::max_value()),
			Error::<Runtime>::NumOverflow
		);
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
		assert_noop!(
			Controller::calculate_new_total_insurance(Balance::max_value(), Rate::saturating_from_rational(11, 10), 1),
			Error::<Runtime>::NumOverflow
		);
		assert_noop!(
			Controller::calculate_new_total_insurance(Balance::max_value(), Rate::saturating_from_rational(1, 1), 1),
			Error::<Runtime>::NumOverflow
		);
	});
}

#[test]
fn get_wrapped_id_by_underlying_asset_id_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		assert_ok!(Controller::get_wrapped_id_by_underlying_asset_id(&CurrencyId::DOT));
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
		assert_ok!(Controller::get_underlying_asset_id_by_wrapped_id(&CurrencyId::MDOT));
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
