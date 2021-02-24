//! Tests for the risk-manager pallet.
/// Unit tests for liquidation functions see in unit-tests for runtime.
use super::*;
use mock::{Event, *};

use frame_support::{assert_noop, assert_ok};
use sp_runtime::{traits::BadOrigin, FixedPointNumber};

#[test]
fn set_max_attempts_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		// Can be set to 0.0
		assert_ok!(TestRiskManager::set_max_attempts(admin(), CurrencyId::DOT, 0));
		assert_eq!(TestRiskManager::risk_manager_dates(CurrencyId::DOT).max_attempts, 0);
		let expected_event = Event::risk_manager(crate::RawEvent::MaxValueOFLiquidationAttempsHasChanged(ADMIN, 0));
		assert!(System::events().iter().any(|record| record.event == expected_event));

		// ALICE set max_attempts equal 2.0
		assert_ok!(TestRiskManager::set_max_attempts(admin(), CurrencyId::DOT, 2));
		assert_eq!(TestRiskManager::risk_manager_dates(CurrencyId::DOT).max_attempts, 2);
		let expected_event = Event::risk_manager(crate::RawEvent::MaxValueOFLiquidationAttempsHasChanged(ADMIN, 2));
		assert!(System::events().iter().any(|record| record.event == expected_event));

		// The dispatch origin of this call must be Administrator.
		assert_noop!(
			TestRiskManager::set_max_attempts(alice(), CurrencyId::DOT, 10),
			Error::<Test>::RequireAdmin
		);

		// MDOT is wrong CurrencyId for underlying assets.
		assert_noop!(
			TestRiskManager::set_max_attempts(admin(), CurrencyId::MDOT, 10),
			Error::<Test>::NotValidUnderlyingAssetId
		);
	});
}

#[test]
fn set_min_sum_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		// Can be set to 0.0
		assert_ok!(TestRiskManager::set_min_sum(admin(), CurrencyId::DOT, Balance::zero()));
		assert_eq!(
			TestRiskManager::risk_manager_dates(CurrencyId::DOT).min_sum,
			Balance::zero()
		);
		let expected_event = Event::risk_manager(crate::RawEvent::MinSumForPartialLiquidationHasChanged(
			ADMIN,
			Balance::zero(),
		));
		assert!(System::events().iter().any(|record| record.event == expected_event));

		// ALICE set min_sum equal one hundred.
		assert_ok!(TestRiskManager::set_min_sum(
			admin(),
			CurrencyId::DOT,
			ONE_HUNDRED * DOLLARS
		));
		assert_eq!(
			TestRiskManager::risk_manager_dates(CurrencyId::DOT).min_sum,
			ONE_HUNDRED * DOLLARS
		);
		let expected_event = Event::risk_manager(crate::RawEvent::MinSumForPartialLiquidationHasChanged(
			ADMIN,
			ONE_HUNDRED * DOLLARS,
		));
		assert!(System::events().iter().any(|record| record.event == expected_event));

		// The dispatch origin of this call must be Administrator.
		assert_noop!(
			TestRiskManager::set_min_sum(alice(), CurrencyId::DOT, 10),
			Error::<Test>::RequireAdmin
		);

		// MDOT is wrong CurrencyId for underlying assets.
		assert_noop!(
			TestRiskManager::set_min_sum(admin(), CurrencyId::MDOT, 10),
			Error::<Test>::NotValidUnderlyingAssetId
		);
	});
}

#[test]
fn set_threshold_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		// Can be set to 0.0
		assert_ok!(TestRiskManager::set_threshold(admin(), CurrencyId::DOT, 0, 1));
		assert_eq!(
			TestRiskManager::risk_manager_dates(CurrencyId::DOT).threshold,
			Rate::zero()
		);
		let expected_event = Event::risk_manager(crate::RawEvent::ValueOfThresholdHasChanged(ADMIN, Rate::zero()));
		assert!(System::events().iter().any(|record| record.event == expected_event));

		// ALICE set min_sum equal one hundred.
		assert_ok!(TestRiskManager::set_threshold(admin(), CurrencyId::DOT, 1, 1));
		assert_eq!(
			TestRiskManager::risk_manager_dates(CurrencyId::DOT).threshold,
			Rate::one()
		);
		let expected_event = Event::risk_manager(crate::RawEvent::ValueOfThresholdHasChanged(ADMIN, Rate::one()));
		assert!(System::events().iter().any(|record| record.event == expected_event));

		// The dispatch origin of this call must be Administrator.
		assert_noop!(
			TestRiskManager::set_threshold(alice(), CurrencyId::DOT, 1, 1),
			Error::<Test>::RequireAdmin
		);

		// MDOT is wrong CurrencyId for underlying assets.
		assert_noop!(
			TestRiskManager::set_threshold(admin(), CurrencyId::MDOT, 1, 1),
			Error::<Test>::NotValidUnderlyingAssetId
		);
	});
}

#[test]
fn set_liquidation_incentive_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		// Can be set to 1.0
		assert_ok!(TestRiskManager::set_liquidation_incentive(
			admin(),
			CurrencyId::DOT,
			1,
			1
		));
		assert_eq!(
			TestRiskManager::risk_manager_dates(CurrencyId::DOT).liquidation_incentive,
			Rate::one()
		);
		let expected_event = Event::risk_manager(crate::RawEvent::ValueOfLiquidationFeeHasChanged(ADMIN, Rate::one()));
		assert!(System::events().iter().any(|record| record.event == expected_event));

		// Can not be set to 0.0
		assert_noop!(
			TestRiskManager::set_liquidation_incentive(admin(), CurrencyId::DOT, 0, 1),
			Error::<Test>::InvalidLiquidationIncentiveValue
		);

		// Can not be set to 2.0
		assert_noop!(
			TestRiskManager::set_liquidation_incentive(admin(), CurrencyId::DOT, 2, 1),
			Error::<Test>::InvalidLiquidationIncentiveValue
		);

		// The dispatch origin of this call must be Administrator.
		assert_noop!(
			TestRiskManager::set_liquidation_incentive(alice(), CurrencyId::DOT, 1, 1),
			Error::<Test>::RequireAdmin
		);

		// MDOT is wrong CurrencyId for underlying assets.
		assert_noop!(
			TestRiskManager::set_liquidation_incentive(admin(), CurrencyId::MDOT, 1, 1),
			Error::<Test>::NotValidUnderlyingAssetId
		);
	});
}

#[test]
fn liquidate_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		// Origin::signed(Alice) is wrong origin for fn liquidate.
		assert_noop!(
			TestRiskManager::liquidate(Origin::signed(ALICE), ALICE, CurrencyId::DOT),
			BadOrigin
		);

		// Origin::none is available origin for fn liquidate.
		assert_noop!(
			TestRiskManager::liquidate(Origin::none(), ALICE, CurrencyId::DOT),
			minterest_protocol::Error::<Test>::ZeroBalanceTransaction
		);
	})
}

#[test]
fn mutate_liquidation_attempts_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		assert_ok!(TestRiskManager::mutate_liquidation_attempts(
			CurrencyId::DOT,
			&ALICE,
			true
		));
		assert_eq!(
			liquidity_pools::PoolUserDates::<Test>::get(CurrencyId::DOT, ALICE).liquidation_attempts,
			u8::one()
		);
		assert_ok!(TestRiskManager::mutate_liquidation_attempts(
			CurrencyId::DOT,
			&ALICE,
			true
		));
		assert_eq!(
			liquidity_pools::PoolUserDates::<Test>::get(CurrencyId::DOT, ALICE).liquidation_attempts,
			2_u8
		);
		assert_ok!(TestRiskManager::mutate_liquidation_attempts(
			CurrencyId::DOT,
			&ALICE,
			false
		));
		assert_eq!(
			liquidity_pools::PoolUserDates::<Test>::get(CurrencyId::DOT, ALICE).liquidation_attempts,
			u8::zero()
		);
	})
}