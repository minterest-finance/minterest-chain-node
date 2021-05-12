//! Tests for the risk-manager pallet.
/// Unit tests for liquidation functions see in unit-tests for runtime.
use super::*;
use mock::{Event, *};

use frame_support::{assert_noop, assert_ok};
use sp_runtime::{traits::BadOrigin, FixedPointNumber};

use sp_core::offchain::{
	testing::{TestOffchainExt, TestTransactionPoolExt},
	OffchainExt, TransactionPoolExt,
};

#[test]
fn test_offchain_worker() {
	let mut ext = ExtBuilder::default()
		.pool_init(DOT)
		.pool_init(KSM)
		.user_balance(ALICE, KSM, 100_000 * DOLLARS)
		.user_balance(ALICE, DOT, 100_000 * DOLLARS)
		.user_balance(BOB, KSM, 100_000 * DOLLARS)
		.liquidity_pool_balance(DOT, 10_000 * DOLLARS)
		.liquidity_pool_balance(KSM, 10_000 * DOLLARS)
		// .liquidation_pool_balance(KSM, 15_000 * DOLLARS)
		// .liquidation_pool_balance(DOT, 15_000 * DOLLARS)
		.build();

	let (offchain, _state) = TestOffchainExt::new();
	let (pool, pool_state) = TestTransactionPoolExt::new();
	ext.register_extension(OffchainExt::new(offchain));
	ext.register_extension(TransactionPoolExt::new(pool));

	ext.execute_with(|| {
		// Prices::unlock_price(admin(), DOT).unwrap();
		System::set_block_number(2);
		assert_ok!(TestMinterestProtocol::deposit_underlying(
			alice(),
			DOT,
			11_000 * DOLLARS
		));

		System::set_block_number(3);
		assert_ok!(TestMinterestProtocol::deposit_underlying(bob(), KSM, 12_000 * DOLLARS));
		assert_ok!(TestMinterestProtocol::enable_is_collateral(alice(), DOT));
		assert_ok!(TestMinterestProtocol::borrow(alice(), KSM, 10_500 * DOLLARS));

		System::set_block_number(4);
		// Decrease DOT price. Now alice collateral isn't enough
		// and loan shoud be liquidated
		Prices::unlock_price(admin(), DOT).unwrap();

		TestRiskManager::offchain_worker(4);
	});
}

#[test]
fn set_max_attempts_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		// Can be set to 0.0
		assert_ok!(TestRiskManager::set_max_attempts(admin(), DOT, 0));
		assert_eq!(TestRiskManager::risk_manager_dates(DOT).max_attempts, 0);
		let expected_event = Event::risk_manager(crate::Event::MaxValueOFLiquidationAttempsHasChanged(0));
		assert!(System::events().iter().any(|record| record.event == expected_event));

		// ALICE set max_attempts equal 2.0
		assert_ok!(TestRiskManager::set_max_attempts(admin(), DOT, 2));
		assert_eq!(TestRiskManager::risk_manager_dates(DOT).max_attempts, 2);
		let expected_event = Event::risk_manager(crate::Event::MaxValueOFLiquidationAttempsHasChanged(2));
		assert!(System::events().iter().any(|record| record.event == expected_event));

		// The dispatch origin of this call must be Administrator.
		assert_noop!(TestRiskManager::set_max_attempts(alice(), DOT, 10), BadOrigin);

		// MDOT is wrong CurrencyId for underlying assets.
		assert_noop!(
			TestRiskManager::set_max_attempts(admin(), MDOT, 10),
			Error::<Test>::NotValidUnderlyingAssetId
		);
	});
}

#[test]
fn set_min_partial_liquidation_sum_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		// Can be set to 0.0
		assert_ok!(TestRiskManager::set_min_partial_liquidation_sum(
			admin(),
			DOT,
			Balance::zero()
		));
		assert_eq!(
			TestRiskManager::risk_manager_dates(DOT).min_partial_liquidation_sum,
			Balance::zero()
		);
		let expected_event = Event::risk_manager(crate::Event::MinSumForPartialLiquidationHasChanged(Balance::zero()));
		assert!(System::events().iter().any(|record| record.event == expected_event));

		// ALICE set min_partial_liquidation_sum equal to one hundred.
		assert_ok!(TestRiskManager::set_min_partial_liquidation_sum(
			admin(),
			DOT,
			ONE_HUNDRED * DOLLARS
		));
		assert_eq!(
			TestRiskManager::risk_manager_dates(DOT).min_partial_liquidation_sum,
			ONE_HUNDRED * DOLLARS
		);
		let expected_event = Event::risk_manager(crate::Event::MinSumForPartialLiquidationHasChanged(
			ONE_HUNDRED * DOLLARS,
		));
		assert!(System::events().iter().any(|record| record.event == expected_event));

		// The dispatch origin of this call must be Administrator.
		assert_noop!(
			TestRiskManager::set_min_partial_liquidation_sum(alice(), DOT, 10),
			BadOrigin
		);

		// MDOT is wrong CurrencyId for underlying assets.
		assert_noop!(
			TestRiskManager::set_min_partial_liquidation_sum(admin(), MDOT, 10),
			Error::<Test>::NotValidUnderlyingAssetId
		);
	});
}

#[test]
fn set_threshold_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		// Can be set to 0.0
		assert_ok!(TestRiskManager::set_threshold(admin(), DOT, Rate::zero()));
		assert_eq!(TestRiskManager::risk_manager_dates(DOT).threshold, Rate::zero());
		let expected_event = Event::risk_manager(crate::Event::ValueOfThresholdHasChanged(Rate::zero()));
		assert!(System::events().iter().any(|record| record.event == expected_event));

		// ALICE set min_partial_liquidation_sum equal one hundred.
		assert_ok!(TestRiskManager::set_threshold(admin(), DOT, Rate::one()));
		assert_eq!(TestRiskManager::risk_manager_dates(DOT).threshold, Rate::one());
		let expected_event = Event::risk_manager(crate::Event::ValueOfThresholdHasChanged(Rate::one()));
		assert!(System::events().iter().any(|record| record.event == expected_event));

		// The dispatch origin of this call must be Administrator.
		assert_noop!(TestRiskManager::set_threshold(alice(), DOT, Rate::one()), BadOrigin);

		// MDOT is wrong CurrencyId for underlying assets.
		assert_noop!(
			TestRiskManager::set_threshold(admin(), MDOT, Rate::one()),
			Error::<Test>::NotValidUnderlyingAssetId
		);
	});
}

#[test]
fn set_liquidation_fee_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		// Can be set to 1.0
		assert_ok!(TestRiskManager::set_liquidation_fee(admin(), DOT, Rate::one()));
		assert_eq!(TestRiskManager::risk_manager_dates(DOT).liquidation_fee, Rate::one());
		let expected_event = Event::risk_manager(crate::Event::ValueOfLiquidationFeeHasChanged(Rate::one()));
		assert!(System::events().iter().any(|record| record.event == expected_event));

		// Can not be set to 0.0
		assert_noop!(
			TestRiskManager::set_liquidation_fee(admin(), DOT, Rate::zero()),
			Error::<Test>::InvalidLiquidationIncentiveValue
		);

		// Can not be set to 2.0
		assert_noop!(
			TestRiskManager::set_liquidation_fee(admin(), DOT, Rate::saturating_from_integer(2)),
			Error::<Test>::InvalidLiquidationIncentiveValue
		);

		// The dispatch origin of this call must be Administrator.
		assert_noop!(
			TestRiskManager::set_liquidation_fee(alice(), DOT, Rate::one()),
			BadOrigin
		);

		// MDOT is wrong CurrencyId for underlying assets.
		assert_noop!(
			TestRiskManager::set_liquidation_fee(admin(), MDOT, Rate::one()),
			Error::<Test>::NotValidUnderlyingAssetId
		);
	});
}

#[test]
fn liquidate_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		// Origin::signed(Alice) is wrong origin for fn liquidate.
		assert_noop!(TestRiskManager::liquidate(Origin::signed(ALICE), ALICE, DOT), BadOrigin);

		// Origin::none is available origin for fn liquidate.
		assert_noop!(
			TestRiskManager::liquidate(Origin::none(), ALICE, DOT),
			minterest_protocol::Error::<Test>::ZeroBalanceTransaction
		);
	})
}

#[test]
fn mutate_liquidation_attempts_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		TestRiskManager::mutate_liquidation_attempts(DOT, &ALICE, true);
		assert_eq!(
			liquidity_pools::PoolUserParams::<Test>::get(DOT, ALICE).liquidation_attempts,
			u8::one()
		);
		TestRiskManager::mutate_liquidation_attempts(DOT, &ALICE, true);
		assert_eq!(
			liquidity_pools::PoolUserParams::<Test>::get(DOT, ALICE).liquidation_attempts,
			2_u8
		);
		TestRiskManager::mutate_liquidation_attempts(DOT, &ALICE, false);
		assert_eq!(
			liquidity_pools::PoolUserParams::<Test>::get(DOT, ALICE).liquidation_attempts,
			u8::zero()
		);
	})
}
