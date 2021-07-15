//! Tests for the risk-manager pallet.
use super::*;
use frame_support::{assert_noop, assert_ok};
use minterest_primitives::Operation::Deposit;
use mock::{Event, *};
use sp_runtime::{traits::BadOrigin, FixedPointNumber};

#[test]
fn user_liquidation_attempts_should_work() {
	ExternalityBuilder::default().build().execute_with(|| {
		TestRiskManager::user_liquidation_attempts_increase_by_one(&ALICE);
		assert_eq!(TestRiskManager::get_user_liquidation_attempts(&ALICE), u8::one());
		TestRiskManager::user_liquidation_attempts_increase_by_one(&ALICE);
		assert_eq!(TestRiskManager::get_user_liquidation_attempts(&ALICE), 2_u8);
		TestRiskManager::user_liquidation_attempts_reset_to_zero(&ALICE);
		assert_eq!(TestRiskManager::get_user_liquidation_attempts(&ALICE), u8::zero());
	})
}

#[test]
fn mutate_depending_operation_should_work() {
	ExternalityBuilder::default()
		.pool_user_data(DOT, ALICE, Balance::zero(), Rate::zero(), true)
		.build()
		.execute_with(|| {
			assert_eq!(TestRiskManager::get_user_liquidation_attempts(&ALICE), u8::zero());
			TestRiskManager::user_liquidation_attempts_increase_by_one(&ALICE);
			assert_eq!(TestRiskManager::get_user_liquidation_attempts(&ALICE), 1_u8);

			// ETH pool is disabled as collateral. Don't reset liquidation attempts.
			TestRiskManager::mutate_depending_operation(Some(ETH), &ALICE, Deposit);
			assert_eq!(TestRiskManager::get_user_liquidation_attempts(&ALICE), 1_u8);

			// DOT pool is enabled as collateral. Reset liquidation attempts to zero.
			TestRiskManager::mutate_depending_operation(Some(DOT), &ALICE, Deposit);
			assert_eq!(TestRiskManager::get_user_liquidation_attempts(&ALICE), u8::zero());
		})
}

#[test]
fn set_liquidation_fee_should_work() {
	ExternalityBuilder::default().build().execute_with(|| {
		// Can be set to 0..
		assert_ok!(TestRiskManager::set_liquidation_fee(
			admin_origin(),
			DOT,
			Rate::saturating_from_rational(3, 10)
		));
		assert_eq!(
			TestRiskManager::liquidation_fee_storage(DOT),
			Rate::saturating_from_rational(3, 10)
		);
		let expected_event = Event::TestRiskManager(crate::Event::LiquidationFeeUpdated(
			DOT,
			Rate::saturating_from_rational(3, 10),
		));
		assert!(System::events().iter().any(|record| record.event == expected_event));

		// Can not be set to 1.0
		assert_noop!(
			TestRiskManager::set_liquidation_fee(admin_origin(), DOT, Rate::one()),
			Error::<Test>::InvalidLiquidationFeeValue
		);

		// The dispatch origin of this call must be Administrator.
		assert_noop!(
			TestRiskManager::set_liquidation_fee(alice_origin(), DOT, Rate::one()),
			BadOrigin
		);

		// MDOT is wrong CurrencyId for underlying assets.
		assert_noop!(
			TestRiskManager::set_liquidation_fee(admin_origin(), MDOT, Rate::one()),
			Error::<Test>::NotValidUnderlyingAssetId
		);
	});
}

#[test]
fn set_threshold_should_work() {
	ExternalityBuilder::default().build().execute_with(|| {
		// Can be set to 1.0
		assert_ok!(TestRiskManager::set_liquidation_threshold(
			admin_origin(),
			DOT,
			Rate::one()
		));
		assert_eq!(TestRiskManager::liquidation_threshold_storage(), Rate::one());
		let expected_event = Event::TestRiskManager(crate::Event::LiquidationThresholdUpdated(Rate::one()));
		assert!(System::events().iter().any(|record| record.event == expected_event));

		// The dispatch origin of this call must be Administrator.
		assert_noop!(
			TestRiskManager::set_liquidation_threshold(alice_origin(), DOT, Rate::one()),
			BadOrigin
		);

		// MDOT is wrong CurrencyId for underlying assets.
		assert_noop!(
			TestRiskManager::set_liquidation_threshold(admin_origin(), MDOT, Rate::one()),
			Error::<Test>::NotValidUnderlyingAssetId
		);
	});
}
