//! Tests for the liquidation-pools pallet.

use super::*;
use frame_support::{assert_noop, assert_ok};
use mock::{Event, *};
use sp_core::offchain::{
	testing::{TestOffchainExt, TestTransactionPoolExt},
	OffchainDbExt, OffchainWorkerExt, TransactionPoolExt,
};
use sp_runtime::traits::{BadOrigin, Zero};

use minterest_primitives::Price;

#[test]
fn offchain_worker_balancing_test() {
	// balance ratio = 0.2 for two pools. Price the same.
	// The offchain worker must send transaction for balancing.
	// It must change 10_000 ETH to 10_000 DOT
	let mut ext = ExternalityBuilder::default()
		.liquidation_pool_balance(DOT, 10_000 * DOLLARS)
		.liquidation_pool_balance(ETH, 30_000 * DOLLARS)
		.set_pool_borrow_underlying(DOT, 100_000 * DOLLARS)
		.set_pool_borrow_underlying(ETH, 100_000 * DOLLARS)
		.build();
	let (offchain, _) = TestOffchainExt::new();

	let (pool, trans_pool_state) = TestTransactionPoolExt::new();
	ext.register_extension(OffchainDbExt::new(offchain.clone()));
	ext.register_extension(OffchainWorkerExt::new(offchain));
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
			TestLiquidationPools::liquidation_pool_data_storage(DOT).deviation_threshold,
			Rate::zero()
		);
		let expected_event = Event::TestLiquidationPools(crate::Event::DeviationThresholdChanged(DOT, Rate::zero()));
		assert!(System::events().iter().any(|record| record.event == expected_event));

		// Can be set to 1.0
		assert_ok!(TestLiquidationPools::set_deviation_threshold(
			admin(),
			DOT,
			1_000_000_000_000_000_000u128
		));
		assert_eq!(
			TestLiquidationPools::liquidation_pool_data_storage(DOT).deviation_threshold,
			Rate::one()
		);
		let expected_event = Event::TestLiquidationPools(crate::Event::DeviationThresholdChanged(DOT, Rate::one()));
		assert!(System::events().iter().any(|record| record.event == expected_event));

		// Can not be set grater than 1.0
		assert_noop!(
			TestLiquidationPools::set_deviation_threshold(admin(), DOT, 2_000_000_000_000_000_000u128),
			Error::<Test>::NotValidDeviationThresholdValue
		);

		// The dispatch origin of this call must be Root or half MinterestCouncil.
		assert_noop!(
			TestLiquidationPools::set_deviation_threshold(alice_origin(), DOT, 10),
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
			TestLiquidationPools::liquidation_pool_data_storage(DOT).balance_ratio,
			Rate::zero()
		);
		let expected_event = Event::TestLiquidationPools(crate::Event::BalanceRatioChanged(DOT, Rate::zero()));
		assert!(System::events().iter().any(|record| record.event == expected_event));

		// Can be set to 1.0
		assert_ok!(TestLiquidationPools::set_balance_ratio(
			admin(),
			DOT,
			1_000_000_000_000_000_000u128
		));
		assert_eq!(
			TestLiquidationPools::liquidation_pool_data_storage(DOT).balance_ratio,
			Rate::one()
		);
		let expected_event = Event::TestLiquidationPools(crate::Event::BalanceRatioChanged(DOT, Rate::one()));
		assert!(System::events().iter().any(|record| record.event == expected_event));

		// Can not be set grater than 1.0
		assert_noop!(
			TestLiquidationPools::set_balance_ratio(admin(), DOT, 2_000_000_000_000_000_000u128),
			Error::<Test>::NotValidBalanceRatioValue
		);

		// The dispatch origin of this call must be Root or half MinterestCouncil.
		assert_noop!(
			TestLiquidationPools::set_balance_ratio(alice_origin(), DOT, 10),
			BadOrigin
		);

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
			TestLiquidationPools::liquidation_pool_data_storage(DOT).max_ideal_balance_usd,
			Some(Balance::zero())
		);
		let expected_event =
			Event::TestLiquidationPools(crate::Event::MaxIdealBalanceChanged(DOT, Some(Balance::zero())));
		assert!(System::events().iter().any(|record| record.event == expected_event));

		// Can be set to None
		assert_ok!(TestLiquidationPools::set_max_ideal_balance(admin(), DOT, None));
		assert_eq!(
			TestLiquidationPools::liquidation_pool_data_storage(DOT).max_ideal_balance_usd,
			None
		);
		let expected_event = Event::TestLiquidationPools(crate::Event::MaxIdealBalanceChanged(DOT, None));
		assert!(System::events().iter().any(|record| record.event == expected_event));

		// The dispatch origin of this call must be Root or half MinterestCouncil.
		assert_noop!(
			TestLiquidationPools::set_max_ideal_balance(alice_origin(), DOT, Some(10u128)),
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
fn calculate_pool_ideal_balance_usd_should_work() {
	ExternalityBuilder::default()
		.set_pool_borrow_underlying(DOT, 500_000 * DOLLARS)
		.build()
		.execute_with(|| {
			// Check that ideal balance is calculated correctly when max_ideal_balance_usd is set to None
			// Liquidity pool value: 500_000
			// Oracle price: 1.0
			// Balance ratio: 0.2
			// Expected ideal balance: 100_000
			assert_eq!(
				TestLiquidationPools::calculate_pool_ideal_balance_usd(DOT),
				Ok(100_000 * DOLLARS)
			);

			assert_ok!(TestLiquidationPools::set_max_ideal_balance(
				admin(),
				DOT,
				Some(1_000 * DOLLARS)
			));
			// Check that ideal balance is calculated correctly when max_ideal_balance_usd is set to 1_000
			// Liquidity pool value: 500_000
			// Oracle price: 1.0
			// Balance ratio: 0.2
			// Expected ideal balance: min(100_000, 1_000) = 1_000
			assert_eq!(
				TestLiquidationPools::calculate_pool_ideal_balance_usd(DOT),
				Ok(1_000 * DOLLARS)
			);

			assert_ok!(TestLiquidationPools::set_max_ideal_balance(
				admin(),
				DOT,
				Some(1_000_000 * DOLLARS)
			));
			// Check that ideal balance is calculated correctly when max_ideal_balance_usd is set to 1_000_000
			// Liquidity pool value: 500_000
			// Oracle price: 1.0
			// Balance ratio: 0.2
			// Expected ideal balance: min(100_000, 1_000_000) = 100_000
			assert_eq!(
				TestLiquidationPools::calculate_pool_ideal_balance_usd(DOT),
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
			// Check that transfer to liquidation pool works correctly
			// Liquidation pool value: 500_000
			// Transfer amount: 20_000
			assert_ok!(TestLiquidationPools::transfer_to_liquidation_pool(admin(), DOT, 20_000));

			let expected_event =
				Event::TestLiquidationPools(crate::Event::TransferToLiquidationPool(DOT, 20_000, who.unwrap()));
			assert!(System::events().iter().any(|record| record.event == expected_event));

			assert_eq!(TestLiquidationPools::get_pool_available_liquidity(DOT), 520_000);

			// Check that transfer with zero amount returns error.
			// Transfer amount: 0
			// Expected error: ZeroBalanceTransaction
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

// Description of the test:
// Two liquidation pools have oversupply and two liquidation pools have shortfall.
// Two "sales" are required for balancing.
#[test]
fn collects_sales_list_should_work_2_2() {
	ExternalityBuilder::default()
		.set_pool_borrow_underlying(DOT, 2_700_000 * DOLLARS)
		.set_pool_borrow_underlying(KSM, 1_000_000 * DOLLARS)
		.set_pool_borrow_underlying(ETH, 2_500_000_000 * DOLLARS)
		.set_pool_borrow_underlying(BTC, 1_200_000 * DOLLARS)
		.liquidation_pool_balance(DOT, 400_000 * DOLLARS)
		.liquidation_pool_balance(KSM, 300_000 * DOLLARS)
		.liquidation_pool_balance(ETH, 800_000_000 * DOLLARS)
		.liquidation_pool_balance(BTC, 100_000 * DOLLARS)
		.build()
		.execute_with(|| {
			set_prices_for_assets(vec![
				(DOT, Price::saturating_from_integer(30)),
				(KSM, Price::saturating_from_integer(5)),
				(ETH, Price::saturating_from_integer(1_500)),
				(BTC, Price::saturating_from_integer(50_000)),
			]);

			/*
			Liquidity Pools balances (in assets): [2_700_000, 1_000_000, 2_500_000_000, 1_200_000]
			Liquidity Pools balances (in USD): [81_000_000, 5_000_000, 3_750_000_000_000, 60_000_000_000]
			Ideal balances 0.2 * liquidity_pool_balance (in USD): [16_200_000, 1_000_000,
			750_000_000_000, 12_000_000_000]

			Liquidation Pools balances (in assets): [400_000, 300_000, 800_000_000, 100_000]
			Liquidation Pools balances (in USD): [12_000_000, 1_500_000, 1_200_000_000_000,
			5_000_000_000]
			Sales list (in assets): [(ETH, BTC, 140_000), (ETH, DOT, 140_000)]
			*/
			let expected_sales_list = vec![
				Sales {
					supply_pool_id: ETH,
					target_pool_id: BTC,
					amount_usd: 7_000_000_000 * DOLLARS, // USD equivalent
				},
				Sales {
					supply_pool_id: ETH,
					target_pool_id: DOT,
					amount_usd: 4_200_000 * DOLLARS, // USD equivalent
				},
			];

			assert_eq!(TestLiquidationPools::collects_sales_list(), Ok(expected_sales_list));
		});
}

#[test]
fn balance_liquidation_pools_should_work() {
	ExternalityBuilder::default()
		.set_pool_borrow_underlying(DOT, 500_000 * DOLLARS)
		.set_pool_borrow_underlying(KSM, 1_000_000 * DOLLARS)
		.set_pool_borrow_underlying(ETH, 1_500_000 * DOLLARS)
		.set_pool_borrow_underlying(BTC, 2_000_000 * DOLLARS)
		.liquidation_pool_balance(DOT, 400_000 * DOLLARS)
		.liquidation_pool_balance(KSM, 300_000 * DOLLARS)
		.liquidation_pool_balance(ETH, 200_000 * DOLLARS)
		.liquidation_pool_balance(BTC, 100_000 * DOLLARS)
		.dex_balance(DOT, 500_000 * DOLLARS)
		.dex_balance(KSM, 500_000 * DOLLARS)
		.dex_balance(ETH, 500_000 * DOLLARS)
		.dex_balance(BTC, 500_000 * DOLLARS)
		.build()
		.execute_with(|| {
			set_prices_for_assets(vec![
				(DOT, Price::saturating_from_integer(1)),
				(KSM, Price::saturating_from_integer(2)),
				(ETH, Price::saturating_from_integer(5)),
				(BTC, Price::saturating_from_integer(10)),
			]);

			/*
			Liquidity Pools balances (in assets): [500_000, 1_000_000, 1_500_000, 2_000_000]
			Liquidity Pools balances (in USD): [500_000, 2_000_000, 7_500_000, 20_000_000]
			Ideal balances 0.2 * liquidity_pool_balance (in USD): [100_000, 400_000, 1_500_000, 4_000_000]

			Liquidation Pools balances (in assets): [400_000, 300_000, 200_000, 100_000]
			Liquidation Pools balances (in USD): [400_000, 600_000, 1_000_000, 1_000_000]

			Sales list (in assets): [(DOT, BTC, 300_000$), (KSM, BTC, 200_000$)]

			*/
			let expected_sales_list = vec![
				Sales {
					supply_pool_id: DOT,
					target_pool_id: BTC,
					amount_usd: 300_000 * DOLLARS, // USD equivalent
				},
				Sales {
					supply_pool_id: KSM,
					target_pool_id: BTC,
					amount_usd: 200_000 * DOLLARS, // USD equivalent
				},
			];

			assert_eq!(
				TestLiquidationPools::collects_sales_list(),
				Ok(expected_sales_list.clone())
			);

			expected_sales_list.iter().for_each(|sale| {
				if let Some((max_supply_amount, target_amount)) =
					TestLiquidationPools::get_amounts(sale.supply_pool_id, sale.target_pool_id, sale.amount_usd).ok()
				{
					let _ = TestLiquidationPools::balance_liquidation_pools(
						Origin::none(),
						sale.supply_pool_id,
						sale.target_pool_id,
						max_supply_amount,
						target_amount,
					);
				};
			});

			// Test that the expected events were emitted
			let our_events = System::events()
				.into_iter()
				.map(|r| r.event)
				.filter_map(|e| {
					if let Event::TestDex(inner) = e {
						Some(inner)
					} else {
						None
					}
				})
				.collect::<Vec<_>>();
			let expected_events = vec![
				dex::Event::Swap(
					TestLiquidationPools::pools_account_id(),
					DOT,
					BTC,
					300_000 * DOLLARS, // max_supply_amount = 300_000 DOT
					30_000 * DOLLARS,  // target_amount = 30_000 BTC
				),
				dex::Event::Swap(
					TestLiquidationPools::pools_account_id(),
					KSM,
					BTC,
					100_000 * DOLLARS, // max_supply_amount = 100_000 DOT
					20_000 * DOLLARS,  // target_amount = 20_000 BTC
				),
			];
			assert_eq!(our_events, expected_events);

			// Liquidation Pool balances
			assert_eq!(liquidation_pool_balance(DOT), 100_000 * DOLLARS);
			assert_eq!(liquidation_pool_balance(KSM), 200_000 * DOLLARS);
			assert_eq!(liquidation_pool_balance(ETH), 200_000 * DOLLARS);
			assert_eq!(liquidation_pool_balance(BTC), 150_000 * DOLLARS);
		});
}

#[test]
fn balance_liquidation_pools_two_pools_should_work_test() {
	ExternalityBuilder::default()
		.set_pool_borrow_underlying(DOT, 500_000 * DOLLARS)
		.set_pool_borrow_underlying(ETH, 300_000 * DOLLARS)
		.liquidation_pool_balance(DOT, 170_000 * DOLLARS) // + 140_000$
		.liquidation_pool_balance(ETH, 30_000 * DOLLARS) //- 120_000$
		.dex_balance(DOT, 500_000 * DOLLARS)
		.dex_balance(ETH, 500_000 * DOLLARS)
		.build()
		.execute_with(|| {
			set_prices_for_assets(vec![
				(DOT, Price::saturating_from_integer(2)),
				(ETH, Price::saturating_from_integer(4)),
			]);

			/*
			Liquidity Pools balances (in assets): [500_000, 300_000]
			Liquidity Pools balances (in USD): [1_000_000, 1_200_000]
			Liquidation Pools balances (in assets): [170_000, 30_000]
			Liquidation Pools balances (in USD):                  [340_000 (+140_000$), 120_000 (-120_000$)]
			Ideal balances 0.2 * liquidity_pool_balance (in USD): [200_000, 240_000]
			Sales list (in assets): [(DOT, ETH, 120_000$)
			*/
			let expected_sales_list = vec![Sales {
				supply_pool_id: DOT,
				target_pool_id: ETH,
				amount_usd: 120_000 * DOLLARS, // USD equivalent
			}];

			assert_eq!(
				TestLiquidationPools::collects_sales_list(),
				Ok(expected_sales_list.clone())
			);

			expected_sales_list.iter().for_each(|sale| {
				if let Some((max_supply_amount, target_amount)) =
					TestLiquidationPools::get_amounts(sale.supply_pool_id, sale.target_pool_id, sale.amount_usd).ok()
				{
					let _ = TestLiquidationPools::balance_liquidation_pools(
						Origin::none(),
						sale.supply_pool_id,
						sale.target_pool_id,
						max_supply_amount,
						target_amount,
					);
				};
			});

			// Test that the expected events were emitted
			let our_events = System::events()
				.into_iter()
				.map(|r| r.event)
				.filter_map(|e| {
					if let Event::TestDex(inner) = e {
						Some(inner)
					} else {
						None
					}
				})
				.collect::<Vec<_>>();
			let expected_events = vec![dex::Event::Swap(
				TestLiquidationPools::pools_account_id(),
				DOT,
				ETH,
				60_000 * DOLLARS, // max_supply_amount = 60_000 DOT
				30_000 * DOLLARS, // target_amount = 30_000 ETH
			)];

			assert_eq!(our_events, expected_events);

			// Liquidation Pool balances
			assert_eq!(liquidation_pool_balance(DOT), 110_000 * DOLLARS);
			assert_eq!(liquidation_pool_balance(ETH), 60_000 * DOLLARS);
		});
}
