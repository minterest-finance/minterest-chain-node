//! Tests for the liquidation-pools pallet.

use super::*;
use frame_support::{assert_noop, assert_ok};
use mock::{Event, *};
use sp_core::offchain::{
	testing::{TestOffchainExt, TestTransactionPoolExt},
	OffchainExt, TransactionPoolExt,
};
use sp_runtime::traits::{BadOrigin, Zero};

use test_helper::offchain_ext::OffChainExtWithHooks;

#[test]
fn offchain_worker_balancing_test() {
	// balance ratio = 0.2 for two pools. Price the same.
	// The offchain worker must send transaction por balancing.
	// It must change 10_000 ETH to 10_000 DOT
	let mut ext = ExternalityBuilder::default()
		.liquidation_pool_balance(DOT, 10_000 * DOLLARS)
		.liquidation_pool_balance(ETH, 30_000 * DOLLARS)
		.liquidity_pool_balance(DOT, 100_000 * DOLLARS)
		.liquidity_pool_balance(ETH, 100_000 * DOLLARS)
		.build();
	let (offchain, _) = TestOffchainExt::new();
	let offchain_ext = OffChainExtWithHooks::new(offchain, None);

	let (pool, trans_pool_state) = TestTransactionPoolExt::new();
	ext.register_extension(OffchainExt::new(offchain_ext));
	ext.register_extension(TransactionPoolExt::new(pool));

	ext.execute_with(|| {
		assert_ok!(TestLiquidationPools::_offchain_worker(0));

		// 1 balancing transcation in transactions pool
		assert_eq!(trans_pool_state.read().transactions.len(), 1);
		let transaction = trans_pool_state.write().transactions.pop().unwrap();
		let ex: Extrinsic = Decode::decode(&mut &*transaction).unwrap();
		// Called extrinsic input params
		let (supply_pool_id, target_pool_id, max_supply_amount, target_supply_amount) = match ex.call {
			crate::mock::Call::TestLiquidationPools(crate::Call::balance_liquidation_pools(
				supply_pool_id,
				target_pool_id,
				max_supply_amount,
				target_supply_amount,
			)) => (supply_pool_id, target_pool_id, max_supply_amount, target_supply_amount),
			e => panic!("Unexpected call: {:?}", e),
		};
		assert_eq!(supply_pool_id, ETH);
		assert_eq!(target_pool_id, DOT);
		assert_eq!(max_supply_amount, 10_000 * DOLLARS);
		assert_eq!(target_supply_amount, 10_000 * DOLLARS);
	});
}

#[test]
fn protocol_operations_not_working_for_nonexisting_pool() {
	ExternalityBuilder::default().build().execute_with(|| {
		assert_noop!(
			TestLiquidationPools::set_deviation_threshold(admin(), KSM, 123),
			Error::<Test>::PoolNotFound
		);

		assert_noop!(
			TestLiquidationPools::set_balance_ratio(admin(), KSM, 123),
			Error::<Test>::PoolNotFound
		);

		assert_noop!(
			TestLiquidationPools::set_max_ideal_balance(admin(), KSM, Some(123)),
			Error::<Test>::PoolNotFound
		);

		assert_noop!(
			TestLiquidationPools::balance_liquidation_pools(Origin::none(), KSM, DOT, Balance::zero(), Balance::zero()),
			Error::<Test>::PoolNotFound
		);

		assert_noop!(
			TestLiquidationPools::transfer_to_liquidation_pool(admin(), KSM, 123),
			Error::<Test>::PoolNotFound
		);
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
fn transfer_to_liquidation_pool_should_work() {
	ExternalityBuilder::default()
		.liquidation_pool_balance(DOT, 500_000)
		.user_balance(ADMIN, DOT, 20_000)
		.build()
		.execute_with(|| {
			let who = ensure_signed(admin());
			//  Check that transfer to liquidation pool works correctly
			// Liquidity pool value: 500_000
			// Transfer amount: 20_000
			assert_ok!(TestLiquidationPools::transfer_to_liquidation_pool(admin(), DOT, 20_000));

			let expected_event =
				Event::liquidation_pools(crate::Event::TransferToLiquidationPool(DOT, 20_000, who.unwrap()));
			assert!(System::events().iter().any(|record| record.event == expected_event));

			assert_eq!(TestLiquidationPools::get_pool_available_liquidity(DOT), 520_000);

			// Check that transfer with zero amount returns error.
			//  Transfer amount: 0
			//  Expected error: ZeroBalanceTransaction
			assert_noop!(
				TestLiquidationPools::transfer_to_liquidation_pool(admin(), DOT, 0),
				Error::<Test>::ZeroBalanceTransaction
			);

			// Check thet transaction with unsuppurted asset returns error.
			// Asset: MNT - native asset, underline assets are only allowed
			// Expected error: NotValidUnderlyingAssetId
			assert_noop!(
				TestLiquidationPools::transfer_to_liquidation_pool(admin(), MNT, 20_000),
				Error::<Test>::NotValidUnderlyingAssetId
			);

			// Check that attempt to transfer amount bigger that user balance returns error
			// Transfer amount: 40_0000
			// Balance: 0
			assert_noop!(
				TestLiquidationPools::transfer_to_liquidation_pool(admin(), DOT, 40_000),
				Error::<Test>::NotEnoughLiquidityAvailable
			);
		});
}
