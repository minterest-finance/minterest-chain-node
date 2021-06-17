use super::*;

// Description of the test:
// Two liquidation pools have oversupply and two liquidation pools have shortfall.
// Two "sales" are required for balancing.
#[test]
fn collects_sales_list_should_work_2_2() {
	ExtBuilder::default()
		.pool_initial(DOT)
		.pool_initial(KSM)
		.pool_initial(ETH)
		.pool_initial(BTC)
		.liquidity_pool_balance(DOT, 2_700_000 * DOLLARS)
		.liquidity_pool_balance(KSM, 1_000_000 * DOLLARS)
		.liquidity_pool_balance(ETH, 2_500_000_000 * DOLLARS)
		.liquidity_pool_balance(BTC, 1_200_000 * DOLLARS)
		.liquidation_pool_balance(DOT, 400_000 * DOLLARS)
		.liquidation_pool_balance(KSM, 300_000 * DOLLARS)
		.liquidation_pool_balance(ETH, 800_000_000 * DOLLARS)
		.liquidation_pool_balance(BTC, 100_000 * DOLLARS)
		.build()
		.execute_with(|| {
			let prices: Vec<(CurrencyId, Price)> = vec![
				(DOT, Price::saturating_from_integer(30)),
				(KSM, Price::saturating_from_integer(5)),
				(ETH, Price::saturating_from_integer(1_500)),
				(BTC, Price::saturating_from_integer(50_000)),
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
					supply_pool_id: ETH,
					target_pool_id: BTC,
					amount: 7_000_000_000 * DOLLARS, // USD equivalent
				},
				Sales {
					supply_pool_id: ETH,
					target_pool_id: DOT,
					amount: 4_200_000 * DOLLARS, // USD equivalent
				},
			];

			assert_eq!(LiquidationPools::collects_sales_list(), Ok(expected_sales_list));
		});
}

#[test]
fn balance_liquidation_pools_should_work() {
	ExtBuilder::default()
		.pool_initial(DOT)
		.pool_initial(KSM)
		.pool_initial(ETH)
		.pool_initial(BTC)
		.liquidity_pool_balance(DOT, 500_000 * DOLLARS)
		.liquidity_pool_balance(KSM, 1_000_000 * DOLLARS)
		.liquidity_pool_balance(ETH, 1_500_000 * DOLLARS)
		.liquidity_pool_balance(BTC, 2_000_000 * DOLLARS)
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
			let prices: Vec<(CurrencyId, Price)> = vec![
				(DOT, Price::saturating_from_integer(1)),
				(KSM, Price::saturating_from_integer(2)),
				(ETH, Price::saturating_from_integer(5)),
				(BTC, Price::saturating_from_integer(10)),
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
					supply_pool_id: DOT,
					target_pool_id: BTC,
					amount: 300_000 * DOLLARS, // USD equivalent
				},
				Sales {
					supply_pool_id: KSM,
					target_pool_id: BTC,
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
					DOT,
					BTC,
					300_000 * DOLLARS, // max_supply_amount = 300_000 DOT
					30_000 * DOLLARS,  // target_amount = 30_000 BTC
				),
				dex::Event::Swap(
					LiquidationPools::pools_account_id(),
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
	ExtBuilder::default()
		.pool_initial(DOT)
		.pool_initial(ETH)
		.liquidity_pool_balance(DOT, 500_000 * DOLLARS)
		.liquidity_pool_balance(ETH, 300_000 * DOLLARS)
		.liquidation_pool_balance(DOT, 170_000 * DOLLARS) // + 140_000$
		.liquidation_pool_balance(ETH, 30_000 * DOLLARS) //- 120_000$
		.dex_balance(DOT, 500_000 * DOLLARS)
		.dex_balance(ETH, 500_000 * DOLLARS)
		.build()
		.execute_with(|| {
			let prices: Vec<(CurrencyId, Price)> = vec![
				(DOT, Price::saturating_from_integer(2)),
				(ETH, Price::saturating_from_integer(4)),
				(BTC, Price::saturating_from_integer(0)), // unused
				(KSM, Price::saturating_from_integer(0)), // unused
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
				supply_pool_id: DOT,
				target_pool_id: ETH,
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
