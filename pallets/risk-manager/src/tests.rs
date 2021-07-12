//! Tests for the risk-manager pallet.
use super::*;
use frame_support::{assert_err, assert_noop, assert_ok};
use minterest_primitives::Price;
use mock::{Event, *};
use sp_core::offchain::{
	testing::{TestOffchainExt, TestTransactionPoolExt},
	OffchainDbExt, OffchainWorkerExt, TransactionPoolExt,
};
use sp_runtime::{traits::BadOrigin, FixedPointNumber};
use test_helper::offchain_ext::OffChainExtWithHooks;

#[test]
fn test_offchain_worker_lock_expired() {
	let mut ext = ExtBuilder::default()
		.pool_init(ETH)
		.pool_init(BTC)
		.user_balance(ALICE, BTC, 100_000 * DOLLARS)
		.liquidity_pool_balance(BTC, 15_000 * DOLLARS)
		.liquidity_pool_balance(ETH, 15_000 * DOLLARS)
		.build();

	let (offchain, state) = TestOffchainExt::new();
	let offchain_ext = OffChainExtWithHooks::new(offchain);

	let (pool, trans_pool_state) = TestTransactionPoolExt::new();
	ext.register_extension(OffchainDbExt::new(offchain_ext.clone()));
	ext.register_extension(OffchainWorkerExt::new(offchain_ext));
	ext.register_extension(TransactionPoolExt::new(pool));

	ext.execute_with(|| {
		set_price_for_all_assets(Price::saturating_from_integer(10));

		System::set_block_number(2);
		assert_ok!(TestMinterestProtocol::deposit_underlying(
			alice_origin(),
			BTC,
			11_000 * DOLLARS
		));
		assert_ok!(TestMinterestProtocol::enable_is_collateral(alice_origin(), BTC));

		System::set_block_number(3);
		assert_ok!(TestMinterestProtocol::borrow(alice_origin(), ETH, 10_500 * DOLLARS));

		System::set_block_number(4);
		// Decrease DOT price. Now alice collateral isn't enough
		// and loan should be liquidated
		set_prices_for_assets(vec![(BTC, Price::saturating_from_integer(2))]);

		assert_ok!(TestRiskManager::_offchain_worker());

		// Check liquidation loan transaction.
		assert_eq!(trans_pool_state.read().transactions.len(), 1);

		// It check is liquidation extrinsic was called.
		let transaction = trans_pool_state.write().transactions.pop().unwrap();
		let ex: Extrinsic = Decode::decode(&mut &*transaction).unwrap();
		// Called extrinsic input params
		let (who, pool_id) = match ex.call {
			crate::mock::Call::TestRiskManager(crate::Call::liquidate(who, pool_id, ..)) => (who, pool_id),
			e => panic!("Unexpected call: {:?}", e),
		};

		assert_eq!(who, ALICE);
		assert_eq!(pool_id, ETH);

		// Get saved index from database
		let serialized_index_result = state
			.read()
			.persistent_storage
			.get(OFFCHAIN_WORKER_LATEST_POOL_INDEX)
			.unwrap();
		// If sequence that produced by CurrencyId::get_enabled_tokens_in_protocol was changed, this
		// assertion can fail.
		assert_eq!(u32::decode(&mut &*serialized_index_result).unwrap(), 3);

		// Shouldn't fail
		assert_ok!(TestRiskManager::_offchain_worker());
	});
}

#[test]
fn test_offchain_worker_simple_liquidation() {
	let mut ext = ExtBuilder::default()
		.pool_init(DOT)
		.pool_init(KSM)
		.user_balance(ALICE, DOT, 100_000 * DOLLARS)
		.liquidity_pool_balance(DOT, 10_000 * DOLLARS)
		.liquidity_pool_balance(KSM, 15_000 * DOLLARS)
		.build();

	let (offchain, state) = TestOffchainExt::new();
	let (pool, trans_pool_state) = TestTransactionPoolExt::new();
	ext.register_extension(OffchainDbExt::new(offchain.clone()));
	ext.register_extension(OffchainWorkerExt::new(offchain));
	ext.register_extension(TransactionPoolExt::new(pool));

	ext.execute_with(|| {
		set_price_for_all_assets(Price::saturating_from_integer(10));

		System::set_block_number(2);
		assert_ok!(TestMinterestProtocol::deposit_underlying(
			alice_origin(),
			DOT,
			11_000 * DOLLARS
		));
		assert_ok!(TestMinterestProtocol::enable_is_collateral(alice_origin(), DOT));

		System::set_block_number(3);
		assert_ok!(TestMinterestProtocol::borrow(alice_origin(), KSM, 10_500 * DOLLARS));

		System::set_block_number(4);
		// Decrease DOT price. Now alice collateral isn't enough
		// and loan should be liquidated
		set_prices_for_assets(vec![(DOT, Price::saturating_from_integer(5))]);

		assert_ok!(TestRiskManager::_offchain_worker());

		assert_eq!(trans_pool_state.read().transactions.len(), 1);
		let transaction = trans_pool_state.write().transactions.pop().unwrap();
		let ex: Extrinsic = Decode::decode(&mut &*transaction).unwrap();

		// Called extrinsic input params
		let (who, pool_id) = match ex.call {
			crate::mock::Call::TestRiskManager(crate::Call::liquidate(who, pool_id, ..)) => (who, pool_id),
			e => panic!("Unexpected call: {:?}", e),
		};
		assert_eq!(who, ALICE);
		assert_eq!(pool_id, KSM);

		// Make sure that index wasn't set. Because all pools were processed.
		assert_eq!(
			state.read().persistent_storage.get(OFFCHAIN_WORKER_LATEST_POOL_INDEX),
			None
		);
	});
}

#[test]
fn set_max_attempts_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		// Can be set to 0.0
		assert_ok!(TestRiskManager::set_max_attempts(admin_origin(), DOT, 0));
		assert_eq!(TestRiskManager::risk_manager_params(DOT).max_attempts, 0);
		let expected_event = Event::TestRiskManager(crate::Event::MaxValueOFLiquidationAttempsHasChanged(0));
		assert!(System::events().iter().any(|record| record.event == expected_event));

		// ALICE set max_attempts equal 2.0
		assert_ok!(TestRiskManager::set_max_attempts(admin_origin(), DOT, 2));
		assert_eq!(TestRiskManager::risk_manager_params(DOT).max_attempts, 2);
		let expected_event = Event::TestRiskManager(crate::Event::MaxValueOFLiquidationAttempsHasChanged(2));
		assert!(System::events().iter().any(|record| record.event == expected_event));

		// The dispatch origin of this call must be Administrator.
		assert_noop!(TestRiskManager::set_max_attempts(alice_origin(), DOT, 10), BadOrigin);

		// MDOT is wrong CurrencyId for underlying assets.
		assert_noop!(
			TestRiskManager::set_max_attempts(admin_origin(), MDOT, 10),
			Error::<Test>::NotValidUnderlyingAssetId
		);
	});
}

#[test]
fn set_min_partial_liquidation_sum_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		// Can be set to 0.0
		assert_ok!(TestRiskManager::set_min_partial_liquidation_sum(
			admin_origin(),
			DOT,
			Balance::zero()
		));
		assert_eq!(
			TestRiskManager::risk_manager_params(DOT).min_partial_liquidation_sum,
			Balance::zero()
		);
		let expected_event =
			Event::TestRiskManager(crate::Event::MinSumForPartialLiquidationHasChanged(Balance::zero()));
		assert!(System::events().iter().any(|record| record.event == expected_event));

		// ALICE set min_partial_liquidation_sum equal to one hundred.
		assert_ok!(TestRiskManager::set_min_partial_liquidation_sum(
			admin_origin(),
			DOT,
			ONE_HUNDRED
		));
		assert_eq!(
			TestRiskManager::risk_manager_params(DOT).min_partial_liquidation_sum,
			ONE_HUNDRED
		);
		let expected_event = Event::TestRiskManager(crate::Event::MinSumForPartialLiquidationHasChanged(ONE_HUNDRED));
		assert!(System::events().iter().any(|record| record.event == expected_event));

		// The dispatch origin of this call must be Administrator.
		assert_noop!(
			TestRiskManager::set_min_partial_liquidation_sum(alice_origin(), DOT, 10),
			BadOrigin
		);

		// MDOT is wrong CurrencyId for underlying assets.
		assert_noop!(
			TestRiskManager::set_min_partial_liquidation_sum(admin_origin(), MDOT, 10),
			Error::<Test>::NotValidUnderlyingAssetId
		);
	});
}

#[test]
fn set_threshold_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		// Can be set to 0.0
		assert_ok!(TestRiskManager::set_threshold(admin_origin(), DOT, Rate::zero()));
		assert_eq!(TestRiskManager::risk_manager_params(DOT).threshold, Rate::zero());
		let expected_event = Event::TestRiskManager(crate::Event::ValueOfThresholdHasChanged(Rate::zero()));
		assert!(System::events().iter().any(|record| record.event == expected_event));

		// ALICE set min_partial_liquidation_sum equal one hundred.
		assert_ok!(TestRiskManager::set_threshold(admin_origin(), DOT, Rate::one()));
		assert_eq!(TestRiskManager::risk_manager_params(DOT).threshold, Rate::one());
		let expected_event = Event::TestRiskManager(crate::Event::ValueOfThresholdHasChanged(Rate::one()));
		assert!(System::events().iter().any(|record| record.event == expected_event));

		// The dispatch origin of this call must be Administrator.
		assert_noop!(
			TestRiskManager::set_threshold(alice_origin(), DOT, Rate::one()),
			BadOrigin
		);

		// MDOT is wrong CurrencyId for underlying assets.
		assert_noop!(
			TestRiskManager::set_threshold(admin_origin(), MDOT, Rate::one()),
			Error::<Test>::NotValidUnderlyingAssetId
		);
	});
}

#[test]
fn set_liquidation_fee_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		// Can be set to 1.0
		assert_ok!(TestRiskManager::set_liquidation_fee(admin_origin(), DOT, Rate::one()));
		assert_eq!(TestRiskManager::risk_manager_params(DOT).liquidation_fee, Rate::one());
		let expected_event = Event::TestRiskManager(crate::Event::ValueOfLiquidationFeeHasChanged(Rate::one()));
		assert!(System::events().iter().any(|record| record.event == expected_event));

		// Can not be set to 0.0
		assert_noop!(
			TestRiskManager::set_liquidation_fee(admin_origin(), DOT, Rate::zero()),
			Error::<Test>::InvalidLiquidationIncentiveValue
		);

		// Can not be set to 2.0
		assert_noop!(
			TestRiskManager::set_liquidation_fee(admin_origin(), DOT, Rate::saturating_from_integer(2)),
			Error::<Test>::InvalidLiquidationIncentiveValue
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
fn complete_liquidation_one_collateral_should_work() {
	ExtBuilder::default()
		.liquidity_pool_balance(DOT, dollars(110_000))
		.liquidation_pool_balance(DOT, dollars(100_000))
		.user_balance(ALICE, MDOT, dollars(100_000))
		.user_balance(BOB, MDOT, dollars(100_000))
		.pool_user_data(DOT, ALICE, dollars(90_000), Rate::one(), true, 3)
		.pool_borrow_underlying(DOT, dollars(90_000))
		.build()
		.execute_with(|| {
			// Set price = 2.00 USD for all pools.
			set_price_for_all_assets(Price::saturating_from_integer(2));

			assert_ok!(TestRiskManager::liquidate_unsafe_loan(ALICE, DOT));

			let expected_event = Event::TestRiskManager(crate::Event::LiquidateUnsafeLoan(
				ALICE,
				180_000 * DOLLARS,
				DOT,
				vec![DOT],
				false,
			));
			assert!(System::events().iter().any(|record| record.event == expected_event));

			assert_eq!(Currencies::free_balance(MDOT, &ALICE), dollars(5_500));

			assert_eq!(TestPools::get_pool_available_liquidity(DOT), dollars(105_500));
			assert_eq!(LiquidationPools::get_pool_available_liquidity(DOT), dollars(104_500));

			assert_eq!(TestPools::pools(DOT).borrowed, Balance::zero());
			assert_eq!(TestPools::pool_user_data(DOT, ALICE).borrowed, Balance::zero());

			assert_eq!(TestPools::pool_user_data(DOT, ALICE).liquidation_attempts, 0);
		})
}

#[test]
fn complete_liquidation_multi_collateral_should_work() {
	ExtBuilder::default()
		.pool_init(DOT)
		.pool_init(ETH)
		.liquidity_pool_balance(DOT, 160_000 * DOLLARS)
		.liquidity_pool_balance(ETH, 50_000 * DOLLARS)
		.liquidation_pool_balance(DOT, 100_000 * DOLLARS)
		.liquidation_pool_balance(ETH, 100_000 * DOLLARS)
		.user_balance(ALICE, MDOT, 50_000 * DOLLARS)
		.user_balance(ALICE, METH, 50_000 * DOLLARS)
		.user_balance(BOB, MDOT, 100_000 * DOLLARS)
		.user_balance(CHARLIE, MDOT, 100_000 * DOLLARS)
		.pool_user_data(DOT, ALICE, 90_000 * DOLLARS, Rate::one(), true, 3)
		.pool_user_data(ETH, ALICE, 0, Rate::one(), true, 0)
		.pool_borrow_underlying(DOT, 90_000 * DOLLARS)
		.build()
		.execute_with(|| {
			// Set price = 2.00 USD for all pools.
			set_price_for_all_assets(Price::saturating_from_integer(2));

			assert_ok!(TestRiskManager::liquidate_unsafe_loan(ALICE, DOT));

			let expected_event = Event::TestRiskManager(crate::Event::LiquidateUnsafeLoan(
				ALICE,
				180_000 * DOLLARS,
				DOT,
				vec![DOT, ETH],
				false,
			));
			assert!(System::events().iter().any(|record| record.event == expected_event));

			assert_eq!(Currencies::free_balance(MDOT, &ALICE), Balance::zero());
			assert_eq!(Currencies::free_balance(METH, &ALICE), 5_500 * DOLLARS);

			assert_eq!(TestPools::get_pool_available_liquidity(DOT), 200_000 * DOLLARS);
			assert_eq!(TestPools::get_pool_available_liquidity(ETH), 5_500 * DOLLARS);

			assert_eq!(LiquidationPools::get_pool_available_liquidity(DOT), 60_000 * DOLLARS);
			assert_eq!(LiquidationPools::get_pool_available_liquidity(ETH), 144_500 * DOLLARS);

			assert_eq!(TestPools::pools(DOT).borrowed, Balance::zero());
			assert_eq!(TestPools::pool_user_data(DOT, ALICE).borrowed, Balance::zero());

			assert_eq!(TestPools::pool_user_data(DOT, ALICE).liquidation_attempts, 0);
		})
}

#[test]
fn partial_liquidation_one_collateral_should_work() {
	ExtBuilder::default()
		.liquidity_pool_balance(DOT, 110_000 * DOLLARS)
		.liquidation_pool_balance(DOT, 100_000 * DOLLARS)
		.user_balance(ALICE, MDOT, 100_000 * DOLLARS)
		.user_balance(BOB, MDOT, 100_000 * DOLLARS)
		.pool_user_data(DOT, ALICE, 90_000 * DOLLARS, Rate::one(), true, 0)
		.pool_borrow_underlying(DOT, 90_000 * DOLLARS)
		.build()
		.execute_with(|| {
			// Set price = 2.00 USD for all pools.
			set_price_for_all_assets(Price::saturating_from_integer(2));

			assert_ok!(TestRiskManager::liquidate_unsafe_loan(ALICE, DOT));

			let expected_event = Event::TestRiskManager(crate::Event::LiquidateUnsafeLoan(
				ALICE,
				54_000 * DOLLARS,
				DOT,
				vec![DOT],
				true,
			));
			assert!(System::events().iter().any(|record| record.event == expected_event));

			assert_eq!(Currencies::free_balance(MDOT, &ALICE), 71_650 * DOLLARS);

			assert_eq!(TestPools::get_pool_available_liquidity(DOT), 108_650 * DOLLARS);
			assert_eq!(LiquidationPools::get_pool_available_liquidity(DOT), 101_350 * DOLLARS);

			assert_eq!(TestPools::pools(DOT).borrowed, 63_000 * DOLLARS);
			assert_eq!(TestPools::pool_user_data(DOT, ALICE).borrowed, 63_000 * DOLLARS);

			assert_eq!(TestPools::pool_user_data(DOT, ALICE).liquidation_attempts, 1);
		})
}

#[test]
fn partial_liquidation_multi_collateral_should_work() {
	ExtBuilder::default()
		.pool_init(DOT)
		.pool_init(ETH)
		.liquidity_pool_balance(DOT, 130_000 * DOLLARS)
		.liquidity_pool_balance(ETH, 80_000 * DOLLARS)
		.liquidation_pool_balance(DOT, 100_000 * DOLLARS)
		.liquidation_pool_balance(ETH, 100_000 * DOLLARS)
		.user_balance(ALICE, MDOT, 20_000 * DOLLARS)
		.user_balance(ALICE, METH, 80_000 * DOLLARS)
		.user_balance(BOB, MDOT, 100_000 * DOLLARS)
		.user_balance(CHARLIE, MDOT, 100_000 * DOLLARS)
		.pool_user_data(DOT, ALICE, 90_000 * DOLLARS, Rate::one(), true, 0)
		.pool_user_data(ETH, ALICE, 0, Rate::one(), true, 0)
		.pool_borrow_underlying(DOT, 90_000 * DOLLARS)
		.build()
		.execute_with(|| {
			// Set price = 2.00 USD for all pools.
			set_price_for_all_assets(Price::saturating_from_integer(2));

			assert_ok!(TestRiskManager::liquidate_unsafe_loan(ALICE, DOT));

			let expected_event = Event::TestRiskManager(crate::Event::LiquidateUnsafeLoan(
				ALICE,
				54_000 * DOLLARS,
				DOT,
				vec![ETH],
				true,
			));
			assert!(System::events().iter().any(|record| record.event == expected_event));

			assert_eq!(Currencies::free_balance(MDOT, &ALICE), 20_000 * DOLLARS);
			assert_eq!(Currencies::free_balance(METH, &ALICE), 51_650 * DOLLARS);

			assert_eq!(TestPools::get_pool_available_liquidity(DOT), 157_000 * DOLLARS);
			assert_eq!(TestPools::get_pool_available_liquidity(ETH), 51_650 * DOLLARS);

			assert_eq!(LiquidationPools::get_pool_available_liquidity(DOT), 73_000 * DOLLARS);
			assert_eq!(LiquidationPools::get_pool_available_liquidity(ETH), 128_350 * DOLLARS);

			assert_eq!(TestPools::pools(DOT).borrowed, 63_000 * DOLLARS);
			assert_eq!(TestPools::pool_user_data(DOT, ALICE).borrowed, 63_000 * DOLLARS);

			assert_eq!(TestPools::pool_user_data(DOT, ALICE).liquidation_attempts, 1);
		})
}

// No liquidity in liquidation pools, therefore we expect a zero transaction error.
#[test]
fn complete_liquidation_should_not_work() {
	ExtBuilder::default()
		.liquidity_pool_balance(DOT, 60_000 * DOLLARS)
		.liquidity_pool_balance(ETH, 50_000 * DOLLARS)
		.user_balance(ALICE, MDOT, 50_000 * DOLLARS)
		.user_balance(ALICE, METH, 50_000 * DOLLARS)
		.user_balance(CHARLIE, MDOT, 100_000 * DOLLARS)
		.pool_user_data(DOT, ALICE, 90_000 * DOLLARS, Rate::one(), true, 3)
		.pool_user_data(ETH, ALICE, 0, Rate::one(), false, 0)
		.pool_borrow_underlying(DOT, 90_000 * DOLLARS)
		.build()
		.execute_with(|| {
			assert_err!(
				TestRiskManager::liquidate_unsafe_loan(ALICE, DOT),
				minterest_protocol::Error::<Test>::ZeroBalanceTransaction
			);
		})
}

// No liquidity in liquidation pools, therefore we expect a zero transaction error.
#[test]
fn partial_liquidation_should_not_work() {
	ExtBuilder::default()
		.liquidity_pool_balance(DOT, 20_000 * DOLLARS)
		.liquidity_pool_balance(ETH, 15_000 * DOLLARS)
		.user_balance(ALICE, MDOT, 10_000 * DOLLARS)
		.user_balance(ALICE, METH, 15_000 * DOLLARS)
		.user_balance(CHARLIE, MDOT, 100_000 * DOLLARS)
		.pool_user_data(DOT, ALICE, 90_000 * DOLLARS, Rate::one(), true, 2)
		.pool_user_data(BTC, ALICE, 0, Rate::one(), true, 0)
		.pool_borrow_underlying(DOT, 90_000 * DOLLARS)
		.build()
		.execute_with(|| {
			assert_err!(
				TestRiskManager::liquidate_unsafe_loan(ALICE, DOT),
				minterest_protocol::Error::<Test>::ZeroBalanceTransaction
			);
		})
}

// If the liquidation pool does not have enough funds to pay off the whole debt, then it repays the
// amount of assets available to it. The number of liquidation attempts stays intact.
#[test]
fn complete_liquidation_one_collateral_not_enough_balance_should_work() {
	ExtBuilder::default()
		.liquidity_pool_balance(DOT, dollars(110_000))
		.liquidation_pool_balance(DOT, dollars(50_000))
		.user_balance(ALICE, MDOT, dollars(100_000))
		.user_balance(BOB, MDOT, dollars(100_000))
		.pool_user_data(DOT, ALICE, dollars(90_000), Rate::one(), true, 3)
		.pool_borrow_underlying(DOT, dollars(90_000))
		.build()
		.execute_with(|| {
			// Set price = 2.00 USD for all pools.
			set_price_for_all_assets(Price::saturating_from_integer(2));

			assert_ok!(TestRiskManager::liquidate_unsafe_loan(ALICE, DOT));

			let expected_event = Event::TestRiskManager(crate::Event::LiquidateUnsafeLoan(
				ALICE,
				100_000 * DOLLARS,
				DOT,
				vec![DOT],
				false,
			));
			assert!(System::events().iter().any(|record| record.event == expected_event));

			assert_eq!(Currencies::free_balance(MDOT, &ALICE), dollars(47_500));

			assert_eq!(TestPools::get_pool_available_liquidity(DOT), dollars(107_500));
			assert_eq!(LiquidationPools::get_pool_available_liquidity(DOT), dollars(52_500));

			assert_eq!(TestPools::pools(DOT).borrowed, dollars(40_000));
			assert_eq!(TestPools::pool_user_data(DOT, ALICE).borrowed, dollars(40_000));

			assert_eq!(TestPools::pool_user_data(DOT, ALICE).liquidation_attempts, 3);
		})
}

#[test]
fn complete_liquidation_multi_collateral_not_enough_balance_should_work() {
	ExtBuilder::default()
		.pool_init(DOT)
		.pool_init(ETH)
		.liquidity_pool_balance(DOT, 160_000 * DOLLARS)
		.liquidity_pool_balance(ETH, 50_000 * DOLLARS)
		.liquidation_pool_balance(DOT, 60_000 * DOLLARS)
		.liquidation_pool_balance(ETH, 100_000 * DOLLARS)
		.user_balance(ALICE, MDOT, 50_000 * DOLLARS)
		.user_balance(ALICE, METH, 50_000 * DOLLARS)
		.user_balance(BOB, MDOT, 100_000 * DOLLARS)
		.user_balance(CHARLIE, MDOT, 100_000 * DOLLARS)
		.pool_user_data(DOT, ALICE, 90_000 * DOLLARS, Rate::one(), true, 3)
		.pool_user_data(ETH, ALICE, 0, Rate::one(), true, 0)
		.pool_borrow_underlying(DOT, 90_000 * DOLLARS)
		.build()
		.execute_with(|| {
			// Set price = 2.00 USD for all pools.
			set_price_for_all_assets(Price::saturating_from_integer(2));

			assert_ok!(TestRiskManager::liquidate_unsafe_loan(ALICE, DOT));

			let expected_event = Event::TestRiskManager(crate::Event::LiquidateUnsafeLoan(
				ALICE,
				120_000 * DOLLARS,
				DOT,
				vec![DOT, ETH],
				false,
			));
			assert!(System::events().iter().any(|record| record.event == expected_event));

			assert_eq!(Currencies::free_balance(MDOT, &ALICE), Balance::zero());
			assert_eq!(Currencies::free_balance(METH, &ALICE), 37_000 * DOLLARS);

			assert_eq!(TestPools::get_pool_available_liquidity(DOT), 170_000 * DOLLARS);
			assert_eq!(TestPools::get_pool_available_liquidity(ETH), 37_000 * DOLLARS);

			assert_eq!(LiquidationPools::get_pool_available_liquidity(DOT), 50_000 * DOLLARS);
			assert_eq!(LiquidationPools::get_pool_available_liquidity(ETH), 113_000 * DOLLARS);

			assert_eq!(TestPools::pools(DOT).borrowed, dollars(30_000));
			assert_eq!(TestPools::pool_user_data(DOT, ALICE).borrowed, dollars(30_000));

			assert_eq!(TestPools::pool_user_data(DOT, ALICE).liquidation_attempts, 3);
		})
}

#[test]
fn partial_liquidation_multi_collateral_not_enough_balance_should_work() {
	ExtBuilder::default()
		.pool_init(DOT)
		.pool_init(ETH)
		.liquidity_pool_balance(DOT, 130_000 * DOLLARS)
		.liquidity_pool_balance(ETH, 80_000 * DOLLARS)
		.liquidation_pool_balance(DOT, 10_000 * DOLLARS)
		.liquidation_pool_balance(ETH, 100_000 * DOLLARS)
		.user_balance(ALICE, MDOT, 20_000 * DOLLARS)
		.user_balance(ALICE, METH, 80_000 * DOLLARS)
		.user_balance(BOB, MDOT, 100_000 * DOLLARS)
		.user_balance(CHARLIE, MDOT, 100_000 * DOLLARS)
		.pool_user_data(DOT, ALICE, 90_000 * DOLLARS, Rate::one(), true, 0)
		.pool_user_data(ETH, ALICE, 0, Rate::one(), true, 0)
		.pool_borrow_underlying(DOT, 90_000 * DOLLARS)
		.build()
		.execute_with(|| {
			// Set price = 2.00 USD for all pools.
			set_price_for_all_assets(Price::saturating_from_integer(2));

			assert_ok!(TestRiskManager::liquidate_unsafe_loan(ALICE, DOT));

			let expected_event = Event::TestRiskManager(crate::Event::LiquidateUnsafeLoan(
				ALICE,
				20_000 * DOLLARS,
				DOT,
				vec![ETH],
				true,
			));
			assert!(System::events().iter().any(|record| record.event == expected_event));

			assert_eq!(Currencies::free_balance(MDOT, &ALICE), dollars(20_000));
			assert_eq!(Currencies::free_balance(METH, &ALICE), dollars(69_500));

			assert_eq!(TestPools::get_pool_available_liquidity(DOT), dollars(140_000));
			assert_eq!(TestPools::get_pool_available_liquidity(ETH), dollars(69_500));

			assert_eq!(LiquidationPools::get_pool_available_liquidity(DOT), Balance::zero());
			assert_eq!(LiquidationPools::get_pool_available_liquidity(ETH), dollars(110_500));

			assert_eq!(TestPools::pools(DOT).borrowed, dollars(80_000));
			assert_eq!(TestPools::pool_user_data(DOT, ALICE).borrowed, dollars(80_000));

			assert_eq!(TestPools::pool_user_data(DOT, ALICE).liquidation_attempts, 0);
		})
}
