//! Tests for the liquidation-pools pallet.

use super::*;
use frame_support::{assert_noop, assert_ok};
use mock::{Event, *};
use sp_runtime::traits::Zero;

#[test]
fn set_balancing_period_should_work() {
	ExternalityBuilder::build().execute_with(|| {
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
		let expected_event = Event::liquidation_pools(crate::Event::BalancingPeriodChanged(ADMIN, u32::zero()));
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
		let expected_event = Event::liquidation_pools(crate::Event::BalancingPeriodChanged(ADMIN, 5256000));
		assert!(System::events().iter().any(|record| record.event == expected_event));

		// The dispatch origin of this call must be Administrator.
		assert_noop!(
			TestLiquidationPools::set_balancing_period(alice(), CurrencyId::DOT, 10),
			Error::<Test>::RequireAdmin
		);

		// MDOT is wrong CurrencyId for underlying assets.
		assert_noop!(
			TestLiquidationPools::set_balancing_period(admin(), CurrencyId::MDOT, 10),
			Error::<Test>::NotValidUnderlyingAssetId
		);
	});
}
