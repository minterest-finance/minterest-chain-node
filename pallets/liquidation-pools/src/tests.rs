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
		assert_ok!(TestLiquidationPools::set_deviation_threshold(admin(), DOT, 0));
		assert_eq!(
			TestLiquidationPools::liquidation_pools_data(DOT).deviation_threshold,
			Rate::zero()
		);
		let expected_event = Event::liquidation_pools(crate::Event::DeviationThresholdChanged(DOT, Rate::zero()));
		assert!(System::events().iter().any(|record| record.event == expected_event));

		// Can be set to 1.0
		assert_ok!(TestLiquidationPools::set_deviation_threshold(
			admin(),
			DOT,
			1_000_000_000_000_000_000u128
		));
		assert_eq!(
			TestLiquidationPools::liquidation_pools_data(DOT).deviation_threshold,
			Rate::one()
		);
		let expected_event = Event::liquidation_pools(crate::Event::DeviationThresholdChanged(DOT, Rate::one()));
		assert!(System::events().iter().any(|record| record.event == expected_event));

		// Can not be set grater than 1.0
		assert_noop!(
			TestLiquidationPools::set_deviation_threshold(admin(), DOT, 2_000_000_000_000_000_000u128),
			Error::<Test>::NotValidDeviationThresholdValue
		);

		// The dispatch origin of this call must be Root or half MinterestCouncil.
		assert_noop!(
			TestLiquidationPools::set_deviation_threshold(alice(), DOT, 10),
			BadOrigin
		);

		// MDOT is wrong CurrencyId for underlying assets.
		assert_noop!(
			TestLiquidationPools::set_deviation_threshold(admin(), MDOT, 10),
			Error::<Test>::NotValidUnderlyingAssetId
		);
	});
}

#[test]
fn set_balance_ratio_should_work() {
	ExternalityBuilder::default().build().execute_with(|| {
		// Can be set to 0.0
		assert_ok!(TestLiquidationPools::set_balance_ratio(admin(), DOT, 0));
		assert_eq!(
			TestLiquidationPools::liquidation_pools_data(DOT).balance_ratio,
			Rate::zero()
		);
		let expected_event = Event::liquidation_pools(crate::Event::BalanceRatioChanged(DOT, Rate::zero()));
		assert!(System::events().iter().any(|record| record.event == expected_event));

		// Can be set to 1.0
		assert_ok!(TestLiquidationPools::set_balance_ratio(
			admin(),
			DOT,
			1_000_000_000_000_000_000u128
		));
		assert_eq!(
			TestLiquidationPools::liquidation_pools_data(DOT).balance_ratio,
			Rate::one()
		);
		let expected_event = Event::liquidation_pools(crate::Event::BalanceRatioChanged(DOT, Rate::one()));
		assert!(System::events().iter().any(|record| record.event == expected_event));

		// Can not be set grater than 1.0
		assert_noop!(
			TestLiquidationPools::set_balance_ratio(admin(), DOT, 2_000_000_000_000_000_000u128),
			Error::<Test>::NotValidBalanceRatioValue
		);

		// The dispatch origin of this call must be Root or half MinterestCouncil.
		assert_noop!(TestLiquidationPools::set_balance_ratio(alice(), DOT, 10), BadOrigin);

		// MDOT is wrong CurrencyId for underlying assets.
		assert_noop!(
			TestLiquidationPools::set_balance_ratio(admin(), MDOT, 10),
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
			DOT,
			Some(Balance::zero())
		));
		assert_eq!(
			TestLiquidationPools::liquidation_pools_data(DOT).max_ideal_balance,
			Some(Balance::zero())
		);
		let expected_event = Event::liquidation_pools(crate::Event::MaxIdealBalanceChanged(DOT, Some(Balance::zero())));
		assert!(System::events().iter().any(|record| record.event == expected_event));

		// Can be set to None
		assert_ok!(TestLiquidationPools::set_max_ideal_balance(admin(), DOT, None));
		assert_eq!(
			TestLiquidationPools::liquidation_pools_data(DOT).max_ideal_balance,
			None
		);
		let expected_event = Event::liquidation_pools(crate::Event::MaxIdealBalanceChanged(DOT, None));
		assert!(System::events().iter().any(|record| record.event == expected_event));

		// The dispatch origin of this call must be Root or half MinterestCouncil.
		assert_noop!(
			TestLiquidationPools::set_max_ideal_balance(alice(), DOT, Some(10u128)),
			BadOrigin
		);

		// MDOT is wrong CurrencyId for underlying assets.
		assert_noop!(
			TestLiquidationPools::set_max_ideal_balance(admin(), MDOT, Some(10u128)),
			Error::<Test>::NotValidUnderlyingAssetId
		);
	});
}

#[test]
fn calculate_ideal_balance_should_work() {
	ExternalityBuilder::default()
		.liquidity_pool_balance(DOT, 500_000 * DOLLARS)
		.build()
		.execute_with(|| {
			// Check that ideal balance is calculated correctly when max_ideal_balance is set to None
			// Liquidity pool value: 500_000
			// Oracle price: 1.0
			// Balance ratio: 0.2
			// Expected ideal balance: 100_000
			assert_eq!(
				TestLiquidationPools::calculate_ideal_balance(DOT),
				Ok(100_000 * DOLLARS)
			);

			assert_ok!(TestLiquidationPools::set_max_ideal_balance(
				admin(),
				DOT,
				Some(1_000 * DOLLARS)
			));
			// Check that ideal balance is calculated correctly when max_ideal_balance is set to 1_000
			// Liquidity pool value: 500_000
			// Oracle price: 1.0
			// Balance ratio: 0.2
			// Expected ideal balance: min(100_000, 1_000) = 1_000
			assert_eq!(TestLiquidationPools::calculate_ideal_balance(DOT), Ok(1_000 * DOLLARS));

			assert_ok!(TestLiquidationPools::set_max_ideal_balance(
				admin(),
				DOT,
				Some(1_000_000 * DOLLARS)
			));
			// Check that ideal balance is calculated correctly when max_ideal_balance is set to 1_000_000
			// Liquidity pool value: 500_000
			// Oracle price: 1.0
			// Balance ratio: 0.2
			// Expected ideal balance: min(100_000, 1_000_000) = 100_000
			assert_eq!(
				TestLiquidationPools::calculate_ideal_balance(DOT),
				Ok(100_000 * DOLLARS)
			);
		});
}

#[test]
fn update_balance_should_work() {
	ExternalityBuilder::default()
		.liquidity_pool_balance(CurrencyId::DOT, 500_000)
		.build()
		.execute_with(|| {
			// Check that update balance work correctly with addition
			// Liquidity pool value: 500000
			// by_amount 10000
			// Expected balance 510000
			let by_amount = 10000;
			assert_eq!(TestLiquidationPools::update_balance(
					admin(),
					CurrencyId::DOT,
					by_amount),
				Ok(().into()));

			let expected_event = Event::liquidation_pools(crate::Event::BalanceUpdated(CurrencyId::DOT, by_amount));
			assert!(System::events().iter().any(|record| record.event == expected_event));

			assert_eq!(TestLiquidationPools::get_pool_available_liquidity(CurrencyId::DOT), 510000);
			// Check that update balance work correctly with substraction
			// Liquidity pool value: 510000
			// by_amount -10000
			// Expected balance 500000
			let by_amount = -10000;
			assert_eq!(TestLiquidationPools::update_balance(
				admin(),
				CurrencyId::DOT,
				by_amount),
			Ok(().into()));

			let expected_event = Event::liquidation_pools(crate::Event::BalanceUpdated(CurrencyId::DOT, by_amount));
			assert!(System::events().iter().any(|record| record.event == expected_event));

			assert_eq!(TestLiquidationPools::get_pool_available_liquidity(CurrencyId::DOT), 500000);
			// Check that update balance work correctly with zero by_amount vavue
			// Liquidity pool value: 500000
			// by_amount 0
			// Expected balance 500000
			let by_amount = 0;
			assert_eq!(TestLiquidationPools::update_balance(
				admin(),
				CurrencyId::DOT,
				by_amount),
			Ok(().into()));

			let expected_event = Event::liquidation_pools(crate::Event::BalanceUpdated(CurrencyId::DOT, by_amount));
			assert!(System::events().iter().any(|record| record.event == expected_event));

			assert_eq!(TestLiquidationPools::get_pool_available_liquidity(CurrencyId::DOT), 500000);
	
	});
}
