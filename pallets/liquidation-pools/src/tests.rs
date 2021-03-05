//! Tests for the liquidation-pools pallet.

use super::*;
use frame_support::{assert_noop, assert_ok};
use mock::{Event, *};
use sp_runtime::traits::{BadOrigin, Zero};

#[test]
fn set_balancing_period_should_work() {
	ExternalityBuilder::default().build().execute_with(|| {
		// Can be set to 0.0
		assert_ok!(TestLiquidationPools::set_balancing_period(
			admin(),
			CurrencyId::DOT,
			u32::zero()
		));
		assert_eq!(
			TestLiquidationPools::liquidation_pools(CurrencyId::DOT).balancing_period,
			u32::zero()
		);
		let expected_event = Event::liquidation_pools(crate::Event::BalancingPeriodChanged(u32::zero()));
		assert!(System::events().iter().any(|record| record.event == expected_event));

		// Admin set period equal amount of blocks per year.
		assert_ok!(TestLiquidationPools::set_balancing_period(
			admin(),
			CurrencyId::DOT,
			5256000
		));
		assert_eq!(
			TestLiquidationPools::liquidation_pools(CurrencyId::DOT).balancing_period,
			5256000
		);
		let expected_event = Event::liquidation_pools(crate::Event::BalancingPeriodChanged(5256000));
		assert!(System::events().iter().any(|record| record.event == expected_event));

		// The dispatch origin of this call must be Administrator.
		assert_noop!(
			TestLiquidationPools::set_balancing_period(alice(), CurrencyId::DOT, 10),
			BadOrigin
		);

		// MDOT is wrong CurrencyId for underlying assets.
		assert_noop!(
			TestLiquidationPools::set_balancing_period(admin(), CurrencyId::MDOT, 10),
			Error::<Test>::NotValidUnderlyingAssetId
		);
	});
}

#[test]
fn balancing_should_work() {
	ExternalityBuilder::default().build().execute_with(|| {
		// Origin::signed(Alice) is wrong origin for fn balancing.
		assert_noop!(
			TestLiquidationPools::balancing(Origin::signed(ALICE), CurrencyId::DOT),
			BadOrigin
		);

		// Origin::none is available origin for fn balancing.
		assert_ok!(TestLiquidationPools::balancing(Origin::none(), CurrencyId::DOT));
	});
}

#[test]
fn calculate_deadline_should_work() {
	ExternalityBuilder::default()
		.pool_timestamp_and_period(CurrencyId::DOT, 1, 600)
		.pool_timestamp_and_period(CurrencyId::ETH, 1, u32::MAX)
		.pool_timestamp_and_period(CurrencyId::KSM, u64::MAX, 1)
		.build()
		.execute_with(|| {
			assert_eq!(TestLiquidationPools::calculate_deadline(CurrencyId::DOT), Ok(601));

			assert_noop!(
				TestLiquidationPools::calculate_deadline(CurrencyId::ETH),
				Error::<Test>::NumOverflow
			);
		});
}
