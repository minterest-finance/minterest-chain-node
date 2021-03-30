use super::*;

// Description of the test:
// Two liquidation pools have oversupply and two liquidation pools have shortfall.
// Two "sales" are required for balancing.
#[test]
fn collects_sales_list_should_work_2_2() {
	ExtBuilder::default()
		.liquidity_pool_balance(CurrencyId::DOT, 2_700_000 * DOLLARS)
		.liquidity_pool_balance(CurrencyId::KSM, 1_000_000 * DOLLARS)
		.liquidity_pool_balance(CurrencyId::ETH, 2_500_000_000 * DOLLARS)
		.liquidity_pool_balance(CurrencyId::BTC, 1_200_000 * DOLLARS)
		.liquidation_pool_balance(CurrencyId::DOT, 400_000 * DOLLARS)
		.liquidation_pool_balance(CurrencyId::KSM, 300_000 * DOLLARS)
		.liquidation_pool_balance(CurrencyId::ETH, 800_000_000 * DOLLARS)
		.liquidation_pool_balance(CurrencyId::BTC, 100_000 * DOLLARS)
		.build()
		.execute_with(|| {
			let prices: Vec<(CurrencyId, Price)> = vec![
				(CurrencyId::DOT, Price::saturating_from_integer(30)),
				(CurrencyId::KSM, Price::saturating_from_integer(5)),
				(CurrencyId::ETH, Price::saturating_from_integer(1_500)),
				(CurrencyId::BTC, Price::saturating_from_integer(50_000)),
			];

			MinterestOracle::on_finalize(0);

			assert_ok!(MinterestOracle::feed_values(origin_of(ORACLE1::get().clone()), prices));

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
					supply_pool_id: CurrencyId::ETH,
					target_pool_id: CurrencyId::BTC,
					amount: 7_000_000_000 * DOLLARS, // USD equivalent
				},
				Sales {
					supply_pool_id: CurrencyId::ETH,
					target_pool_id: CurrencyId::DOT,
					amount: 4_200_000 * DOLLARS, // USD equivalent
				},
			];

			assert_eq!(LiquidationPools::collects_sales_list(), Ok(expected_sales_list));
		});
}

#[test]
fn balance_liquidation_pools_should_work() {
	ExtBuilder::default()
		.liquidity_pool_balance(CurrencyId::DOT, 500_000 * DOLLARS)
		.liquidity_pool_balance(CurrencyId::KSM, 1_000_000 * DOLLARS)
		.liquidity_pool_balance(CurrencyId::ETH, 1_500_000 * DOLLARS)
		.liquidity_pool_balance(CurrencyId::BTC, 2_000_000 * DOLLARS)
		.liquidation_pool_balance(CurrencyId::DOT, 400_000 * DOLLARS)
		.liquidation_pool_balance(CurrencyId::KSM, 300_000 * DOLLARS)
		.liquidation_pool_balance(CurrencyId::ETH, 200_000 * DOLLARS)
		.liquidation_pool_balance(CurrencyId::BTC, 100_000 * DOLLARS)
		.dex_balance(CurrencyId::DOT, 500_000 * DOLLARS)
		.dex_balance(CurrencyId::KSM, 500_000 * DOLLARS)
		.dex_balance(CurrencyId::ETH, 500_000 * DOLLARS)
		.dex_balance(CurrencyId::BTC, 500_000 * DOLLARS)
		.build()
		.execute_with(|| {
			let prices: Vec<(CurrencyId, Price)> = vec![
				(CurrencyId::DOT, Price::saturating_from_integer(1)),
				(CurrencyId::KSM, Price::saturating_from_integer(2)),
				(CurrencyId::ETH, Price::saturating_from_integer(5)),
				(CurrencyId::BTC, Price::saturating_from_integer(10)),
			];

			MinterestOracle::on_finalize(0);

			assert_ok!(MinterestOracle::feed_values(origin_of(ORACLE1::get().clone()), prices));
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
					supply_pool_id: CurrencyId::DOT,
					target_pool_id: CurrencyId::BTC,
					amount: 300_000 * DOLLARS, // USD equivalent
				},
				Sales {
					supply_pool_id: CurrencyId::KSM,
					target_pool_id: CurrencyId::BTC,
					amount: 200_000 * DOLLARS, // USD equivalent
				},
			];

			assert_eq!(LiquidationPools::collects_sales_list(), Ok(expected_sales_list.clone()));

			expected_sales_list.iter().for_each(|sale| {
				if let Some((max_supply_amount, target_amount)) =
					LiquidationPools::get_amounts(sale.supply_pool_id, sale.target_pool_id, sale.amount).ok()
				{
					let _ = LiquidationPools::balance_liquidation_pools(
						origin_none(),
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
				.filter_map(|e| if let Event::dex(inner) = e { Some(inner) } else { None })
				.collect::<Vec<_>>();
			let expected_events = vec![
				dex::Event::Swap(
					LiquidationPools::pools_account_id(),
					CurrencyId::DOT,
					CurrencyId::BTC,
					300_000 * DOLLARS, // max_supply_amount = 300_000 DOT
					30_000 * DOLLARS,  // target_amount = 30_000 BTC
				),
				dex::Event::Swap(
					LiquidationPools::pools_account_id(),
					CurrencyId::KSM,
					CurrencyId::BTC,
					100_000 * DOLLARS, // max_supply_amount = 100_000 DOT
					20_000 * DOLLARS,  // target_amount = 20_000 BTC
				),
			];
			assert_eq!(our_events, expected_events);

			// Liquidation Pool balances in assets
			assert_eq!(liquidation_pool_balance(CurrencyId::DOT), 100_000 * DOLLARS);
			assert_eq!(liquidation_pool_balance(CurrencyId::KSM), 200_000 * DOLLARS);
			assert_eq!(liquidation_pool_balance(CurrencyId::ETH), 200_000 * DOLLARS);
			assert_eq!(liquidation_pool_balance(CurrencyId::BTC), 150_000 * DOLLARS);
		});
}

#[test]
fn balance_liquidation_pools_two_pools_should_work_test() {
	ExtBuilder::default()
		.liquidity_pool_balance(CurrencyId::DOT, 500_000 * DOLLARS)
		.liquidity_pool_balance(CurrencyId::ETH, 300_000 * DOLLARS)
		.liquidation_pool_balance(CurrencyId::DOT, 170_000 * DOLLARS) // + 140 000$
		.liquidation_pool_balance(CurrencyId::ETH, 30_000 * DOLLARS) //
		// - 120 000$
		.dex_balance(CurrencyId::DOT, 500_000 * DOLLARS)
		.dex_balance(CurrencyId::ETH, 500_000 * DOLLARS)
		.build()
		.execute_with(|| {
			let prices: Vec<(CurrencyId, Price)> = vec![
				(CurrencyId::DOT, Price::saturating_from_integer(2)),
				(CurrencyId::ETH, Price::saturating_from_integer(4)),
				(CurrencyId::BTC, Price::saturating_from_integer(0)), // unused
				(CurrencyId::KSM, Price::saturating_from_integer(0)), // unused
			];
			MinterestOracle::on_finalize(0);
			assert_ok!(MinterestOracle::feed_values(origin_of(ORACLE1::get().clone()), prices));
			/*
			Liquidity Pools balances (in assets): [500_000, 300_000]
			Liquidity Pools balances (in USD): [1_000_000, 1_200_000]
			Liquidation Pools balances (in assets): [170_000, 30_000]
			Liquidation Pools balances (in USD):                  [340_000 (+140_000$), 120_000 (-120_000$)]
			Ideal balances 0.2 * liquidity_pool_balance (in USD): [200_000, 240_000]
			Sales list (in assets): [(DOT, ETH, 120_000$)
			*/
			let expected_sales_list = vec![Sales {
				supply_pool_id: CurrencyId::DOT,
				target_pool_id: CurrencyId::ETH,
				amount: 120_000 * DOLLARS, // USD equivalent
			}];

			assert_eq!(LiquidationPools::collects_sales_list(), Ok(expected_sales_list.clone()));

			expected_sales_list.iter().for_each(|sale| {
				if let Some((max_supply_amount, target_amount)) =
					LiquidationPools::get_amounts(sale.supply_pool_id, sale.target_pool_id, sale.amount).ok()
				{
					let _ = LiquidationPools::balance_liquidation_pools(
						origin_none(),
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
				.filter_map(|e| if let Event::dex(inner) = e { Some(inner) } else { None })
				.collect::<Vec<_>>();
			let expected_events = vec![dex::Event::Swap(
				LiquidationPools::pools_account_id(),
				CurrencyId::DOT,
				CurrencyId::ETH,
				60_000 * DOLLARS, // max_supply_amount = 60_000 DOT
				30_000 * DOLLARS, // target_amount = 30_000 ETH
			)];

			assert_eq!(our_events, expected_events);

			// Liquidation Pool balances in assets
			assert_eq!(liquidation_pool_balance(CurrencyId::DOT), 110_000 * DOLLARS);
			assert_eq!(liquidation_pool_balance(CurrencyId::ETH), 60_000 * DOLLARS);
		});
}
