//! Tests for the risk-manager pallet.

use super::*;
use mock::*;

use frame_support::{assert_noop, assert_ok};
use sp_runtime::FixedPointNumber;

#[test]
fn set_max_attempts_should_work() {
	new_test_ext().execute_with(|| {
		// Can be set to 0.0
		assert_ok!(TestRiskManager::set_max_attempts(alice(), CurrencyId::DOT, 0));
		assert_eq!(TestRiskManager::risk_manager_dates(CurrencyId::DOT).max_attempts, 0);
		let expected_event = TestEvent::risk_manager(Event::MaxValueOFLiquidationAttempsHasChanged);
		assert!(System::events().iter().any(|record| record.event == expected_event));

		// ALICE set max_attempts equal 2.0
		assert_ok!(TestRiskManager::set_max_attempts(alice(), CurrencyId::DOT, 2));
		assert_eq!(TestRiskManager::risk_manager_dates(CurrencyId::DOT).max_attempts, 2);
		let expected_event = TestEvent::risk_manager(Event::MaxValueOFLiquidationAttempsHasChanged);
		assert!(System::events().iter().any(|record| record.event == expected_event));

		// The dispatch origin of this call must be Administrator.
		assert_noop!(
			TestRiskManager::set_max_attempts(bob(), CurrencyId::DOT, 10),
			Error::<Test>::RequireAdmin
		);

		// MDOT is wrong CurrencyId for underlying assets.
		assert_noop!(
			TestRiskManager::set_max_attempts(alice(), CurrencyId::MDOT, 10),
			Error::<Test>::NotValidUnderlyingAssetId
		);
	});
}

#[test]
fn set_min_sum_should_work() {
	new_test_ext().execute_with(|| {
		// Can be set to 0.0
		assert_ok!(TestRiskManager::set_min_sum(alice(), CurrencyId::DOT, 0));
		assert_eq!(TestRiskManager::risk_manager_dates(CurrencyId::DOT).min_sum, 0);
		let expected_event = TestEvent::risk_manager(Event::MinSumForPartialLiquidationHasChanged);
		assert!(System::events().iter().any(|record| record.event == expected_event));

		// ALICE set min_sum equal one hundred.
		assert_ok!(TestRiskManager::set_min_sum(
			alice(),
			CurrencyId::DOT,
			ONE_HUNDRED * DOLLARS
		));
		assert_eq!(
			TestRiskManager::risk_manager_dates(CurrencyId::DOT).min_sum,
			ONE_HUNDRED * DOLLARS
		);
		let expected_event = TestEvent::risk_manager(Event::MinSumForPartialLiquidationHasChanged);
		assert!(System::events().iter().any(|record| record.event == expected_event));

		// The dispatch origin of this call must be Administrator.
		assert_noop!(
			TestRiskManager::set_min_sum(bob(), CurrencyId::DOT, 10),
			Error::<Test>::RequireAdmin
		);

		// MDOT is wrong CurrencyId for underlying assets.
		assert_noop!(
			TestRiskManager::set_min_sum(alice(), CurrencyId::MDOT, 10),
			Error::<Test>::NotValidUnderlyingAssetId
		);
	});
}

#[test]
fn set_threshold_should_work() {
	new_test_ext().execute_with(|| {
		// Can be set to 0.0
		assert_ok!(TestRiskManager::set_threshold(alice(), CurrencyId::DOT, Rate::zero()));
		assert_eq!(
			TestRiskManager::risk_manager_dates(CurrencyId::DOT).threshold,
			Rate::zero()
		);
		let expected_event = TestEvent::risk_manager(Event::ValueOfThresholdHasChanged);
		assert!(System::events().iter().any(|record| record.event == expected_event));

		// ALICE set min_sum equal one hundred.
		assert_ok!(TestRiskManager::set_threshold(alice(), CurrencyId::DOT, Rate::one()));
		assert_eq!(
			TestRiskManager::risk_manager_dates(CurrencyId::DOT).threshold,
			Rate::one()
		);
		let expected_event = TestEvent::risk_manager(Event::ValueOfThresholdHasChanged);
		assert!(System::events().iter().any(|record| record.event == expected_event));

		// The dispatch origin of this call must be Administrator.
		assert_noop!(
			TestRiskManager::set_threshold(bob(), CurrencyId::DOT, Rate::one()),
			Error::<Test>::RequireAdmin
		);

		// MDOT is wrong CurrencyId for underlying assets.
		assert_noop!(
			TestRiskManager::set_threshold(alice(), CurrencyId::MDOT, Rate::one()),
			Error::<Test>::NotValidUnderlyingAssetId
		);
	});
}

#[test]
fn set_liquidation_fee_should_work() {
	new_test_ext().execute_with(|| {
		// Can be set to 0.0
		assert_ok!(TestRiskManager::set_liquidation_fee(
			alice(),
			CurrencyId::DOT,
			Rate::zero()
		));
		assert_eq!(
			TestRiskManager::risk_manager_dates(CurrencyId::DOT).liquidation_fee,
			Rate::zero()
		);
		let expected_event = TestEvent::risk_manager(Event::ValueOfLiquidationFeeHasChanged);
		assert!(System::events().iter().any(|record| record.event == expected_event));

		// ALICE set min_sum equal one hundred.
		assert_ok!(TestRiskManager::set_liquidation_fee(
			alice(),
			CurrencyId::DOT,
			Rate::one()
		));
		assert_eq!(
			TestRiskManager::risk_manager_dates(CurrencyId::DOT).liquidation_fee,
			Rate::one()
		);
		let expected_event = TestEvent::risk_manager(Event::ValueOfLiquidationFeeHasChanged);
		assert!(System::events().iter().any(|record| record.event == expected_event));

		// The dispatch origin of this call must be Administrator.
		assert_noop!(
			TestRiskManager::set_liquidation_fee(bob(), CurrencyId::DOT, Rate::one()),
			Error::<Test>::RequireAdmin
		);

		// MDOT is wrong CurrencyId for underlying assets.
		assert_noop!(
			TestRiskManager::set_liquidation_fee(alice(), CurrencyId::MDOT, Rate::one()),
			Error::<Test>::NotValidUnderlyingAssetId
		);
	});
}
