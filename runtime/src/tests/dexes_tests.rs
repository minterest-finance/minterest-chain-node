use super::*;

//------------ TEMPORARY Dex module. Tests ---------------------------
#[test]
fn swap_with_exact_target_should_work() {
	ExtBuilder::default()
		.liquidation_pool_balance(DOT, 300_000 * DOLLARS)
		.liquidation_pool_balance(ETH, 400_000 * DOLLARS)
		.dex_balance(DOT, 500_000 * DOLLARS)
		.dex_balance(ETH, 500_000 * DOLLARS)
		.build()
		.execute_with(|| {
			assert_eq!(
				Dex::swap_with_exact_target(
					&LiquidationPools::pools_account_id(),
					DOT,
					ETH,
					50_000 * DOLLARS,
					50_000 * DOLLARS
				),
				Ok(50_000 * DOLLARS)
			);

			assert_eq!(liquidation_pool_balance(DOT), 250_000 * DOLLARS);
			assert_eq!(liquidation_pool_balance(ETH), 450_000 * DOLLARS);

			assert_eq!(dex_balance(DOT), 550_000 * DOLLARS);
			assert_eq!(dex_balance(ETH), 450_000 * DOLLARS);
		});
}

#[test]
fn do_swap_with_exact_target_should_work() {
	ExtBuilder::default()
		.liquidation_pool_balance(DOT, 300_000 * DOLLARS)
		.liquidation_pool_balance(ETH, 400_000 * DOLLARS)
		.dex_balance(DOT, 50_000 * DOLLARS)
		.dex_balance(ETH, 50_000 * DOLLARS)
		.build()
		.execute_with(|| {
			assert_eq!(
				Dex::do_swap_with_exact_target(
					&LiquidationPools::pools_account_id(),
					DOT,
					ETH,
					10_000 * DOLLARS,
					10_000 * DOLLARS
				),
				Ok(10_000 * DOLLARS)
			);
			let expected_event = Event::dex(dex::Event::Swap(
				LiquidationPools::pools_account_id(),
				DOT,
				ETH,
				10_000 * DOLLARS,
				10_000 * DOLLARS,
			));
			assert!(System::events().iter().any(|record| record.event == expected_event));

			assert_eq!(liquidation_pool_balance(DOT), 290_000 * DOLLARS);
			assert_eq!(liquidation_pool_balance(ETH), 410_000 * DOLLARS);

			assert_eq!(dex_balance(DOT), 60_000 * DOLLARS);
			assert_eq!(dex_balance(ETH), 40_000 * DOLLARS);

			assert_err!(
				Dex::do_swap_with_exact_target(
					&LiquidationPools::pools_account_id(),
					DOT,
					ETH,
					100_000 * DOLLARS,
					100_000 * DOLLARS
				),
				dex::Error::<Runtime>::InsufficientDexBalance
			);
		});
}