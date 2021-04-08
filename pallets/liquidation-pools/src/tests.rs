//! Tests for the liquidation-pools pallet.

use super::*;
use frame_support::{assert_noop, assert_ok};
use mock::{Event, *};
use sp_runtime::traits::{BadOrigin, Zero};

#[test]
fn set_balancing_period_should_work() {
	ExternalityBuilder::default().build().execute_with(|| {
		// Can be set to 0.0
		assert_ok!(TestLiquidationPools::set_balancing_period(admin(), u64::zero()));
		assert_eq!(TestLiquidationPools::balancing_period(), u64::zero());
		let expected_event = Event::liquidation_pools(crate::Event::BalancingPeriodChanged(u64::zero()));
		assert!(System::events().iter().any(|record| record.event == expected_event));

		// Admin set period equal amount of blocks per year.
		assert_ok!(TestLiquidationPools::set_balancing_period(admin(), 5256000));
		assert_eq!(TestLiquidationPools::balancing_period(), 5256000);
		let expected_event = Event::liquidation_pools(crate::Event::BalancingPeriodChanged(5256000));
		assert!(System::events().iter().any(|record| record.event == expected_event));

		// The dispatch origin of this call must be Root or half MinterestCouncil.
		assert_noop!(TestLiquidationPools::set_balancing_period(alice(), 10), BadOrigin);
	});
}

#[test]
fn set_deviation_threshold_should_work() {
	ExternalityBuilder::default().build().execute_with(|| {
		// Can be set to 0.0
		assert_ok!(TestLiquidationPools::set_deviation_threshold(
			admin(),
			CurrencyId::DOT,
			0
		));
		assert_eq!(
			TestLiquidationPools::liquidation_pools_data(CurrencyId::DOT).deviation_threshold,
			Rate::zero()
		);
		let expected_event =
			Event::liquidation_pools(crate::Event::DeviationThresholdChanged(CurrencyId::DOT, Rate::zero()));
		assert!(System::events().iter().any(|record| record.event == expected_event));

		// Can be set to 1.0
		assert_ok!(TestLiquidationPools::set_deviation_threshold(
			admin(),
			CurrencyId::DOT,
			1_000_000_000_000_000_000u128
		));
		assert_eq!(
			TestLiquidationPools::liquidation_pools_data(CurrencyId::DOT).deviation_threshold,
			Rate::one()
		);
		let expected_event =
			Event::liquidation_pools(crate::Event::DeviationThresholdChanged(CurrencyId::DOT, Rate::one()));
		assert!(System::events().iter().any(|record| record.event == expected_event));

		// Can not be set grater than 1.0
		assert_noop!(
			TestLiquidationPools::set_deviation_threshold(admin(), CurrencyId::DOT, 2_000_000_000_000_000_000u128),
			Error::<Test>::NotValidDeviationThresholdValue
		);

		// The dispatch origin of this call must be Root or half MinterestCouncil.
		assert_noop!(
			TestLiquidationPools::set_deviation_threshold(alice(), CurrencyId::DOT, 10),
			BadOrigin
		);

		// MDOT is wrong CurrencyId for underlying assets.
		assert_noop!(
			TestLiquidationPools::set_deviation_threshold(admin(), CurrencyId::MDOT, 10),
			Error::<Test>::NotValidUnderlyingAssetId
		);
	});
}

#[test]
fn set_balance_ratio_should_work() {
	ExternalityBuilder::default().build().execute_with(|| {
		// Can be set to 0.0
		assert_ok!(TestLiquidationPools::set_balance_ratio(admin(), CurrencyId::DOT, 0));
		assert_eq!(
			TestLiquidationPools::liquidation_pools_data(CurrencyId::DOT).balance_ratio,
			Rate::zero()
		);
		let expected_event = Event::liquidation_pools(crate::Event::BalanceRatioChanged(CurrencyId::DOT, Rate::zero()));
		assert!(System::events().iter().any(|record| record.event == expected_event));

		// Can be set to 1.0
		assert_ok!(TestLiquidationPools::set_balance_ratio(
			admin(),
			CurrencyId::DOT,
			1_000_000_000_000_000_000u128
		));
		assert_eq!(
			TestLiquidationPools::liquidation_pools_data(CurrencyId::DOT).balance_ratio,
			Rate::one()
		);
		let expected_event = Event::liquidation_pools(crate::Event::BalanceRatioChanged(CurrencyId::DOT, Rate::one()));
		assert!(System::events().iter().any(|record| record.event == expected_event));

		// Can not be set grater than 1.0
		assert_noop!(
			TestLiquidationPools::set_balance_ratio(admin(), CurrencyId::DOT, 2_000_000_000_000_000_000u128),
			Error::<Test>::NotValidBalanceRatioValue
		);

		// The dispatch origin of this call must be Root or half MinterestCouncil.
		assert_noop!(
			TestLiquidationPools::set_balance_ratio(alice(), CurrencyId::DOT, 10),
			BadOrigin
		);

		// MDOT is wrong CurrencyId for underlying assets.
		assert_noop!(
			TestLiquidationPools::set_balance_ratio(admin(), CurrencyId::MDOT, 10),
			Error::<Test>::NotValidUnderlyingAssetId
		);
	});
}

#[test]
fn set_max_ideal_balance_should_work() {
	ExternalityBuilder::default().build().execute_with(|| {
		// Can be set to 0
		assert_ok!(TestLiquidationPools::set_max_ideal_balance(
			admin(),
			CurrencyId::DOT,
			Some(Balance::zero())
		));
		assert_eq!(
			TestLiquidationPools::liquidation_pools_data(CurrencyId::DOT).max_ideal_balance,
			Some(0)
		);
		let expected_event = Event::liquidation_pools(crate::Event::MaxIdealBalanceChanged(
			CurrencyId::DOT,
			Some(Balance::zero()),
		));
		assert!(System::events().iter().any(|record| record.event == expected_event));

		// Can be set to None
		assert_ok!(TestLiquidationPools::set_max_ideal_balance(
			admin(),
			CurrencyId::DOT,
			None
		));
		assert_eq!(
			TestLiquidationPools::liquidation_pools_data(CurrencyId::DOT).max_ideal_balance,
			None
		);
		let expected_event = Event::liquidation_pools(crate::Event::MaxIdealBalanceChanged(CurrencyId::DOT, None));
		assert!(System::events().iter().any(|record| record.event == expected_event));

		// The dispatch origin of this call must be Root or half MinterestCouncil.
		assert_noop!(
			TestLiquidationPools::set_max_ideal_balance(alice(), CurrencyId::DOT, Some(10u128)),
			BadOrigin
		);

		// MDOT is wrong CurrencyId for underlying assets.
		assert_noop!(
			TestLiquidationPools::set_max_ideal_balance(admin(), CurrencyId::MDOT, Some(10u128)),
			Error::<Test>::NotValidUnderlyingAssetId
		);
	});
}
