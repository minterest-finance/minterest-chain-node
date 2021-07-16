//! Unit tests for dex module.

#![cfg(test)]

use dex::{Error, Event};
use frame_support::assert_err;
use orml_traits::MultiCurrency;
use pallet_traits::{DEXManager, PoolsManager};
use test_engine::*;

#[test]
fn swap_with_exact_target_should_work() {
	ExtBuilderNew::default()
		.set_liquidation_pool_balance(DOT, 300_000 * DOLLARS)
		.set_liquidation_pool_balance(ETH, 400_000 * DOLLARS)
		.set_dex_balance(DOT, 500_000 * DOLLARS)
		.set_dex_balance(ETH, 500_000 * DOLLARS)
		.build()
		.execute_with(|| {
			assert_eq!(
				TestDex::swap_with_exact_target(
					&TestLiquidationPools::pools_account_id(),
					DOT,
					ETH,
					50_000 * DOLLARS,
					50_000 * DOLLARS
				),
				Ok(50_000 * DOLLARS)
			);

			assert_eq!(
				Currencies::free_balance(DOT, &TestLiquidationPools::pools_account_id()),
				250_000 * DOLLARS
			);
			assert_eq!(
				Currencies::free_balance(ETH, &TestLiquidationPools::pools_account_id()),
				450_000 * DOLLARS
			);

			assert_eq!(
				Currencies::free_balance(DOT, &TestDex::dex_account_id()),
				550_000 * DOLLARS
			);
			assert_eq!(
				Currencies::free_balance(ETH, &TestDex::dex_account_id()),
				450_000 * DOLLARS
			);
		});
}

#[test]
fn do_swap_with_exact_target_should_work() {
	ExtBuilderNew::default()
		.set_liquidation_pool_balance(DOT, 300_000 * DOLLARS)
		.set_liquidation_pool_balance(ETH, 400_000 * DOLLARS)
		.set_dex_balance(DOT, 50_000 * DOLLARS)
		.set_dex_balance(ETH, 50_000 * DOLLARS)
		.build()
		.execute_with(|| {
			assert_eq!(
				TestDex::do_swap_with_exact_target(
					&TestLiquidationPools::pools_account_id(),
					DOT,
					ETH,
					10_000 * DOLLARS,
					10_000 * DOLLARS
				),
				Ok(10_000 * DOLLARS)
			);
			let expected_event = test_engine::Event::TestDex(Event::Swap(
				TestLiquidationPools::pools_account_id(),
				DOT,
				ETH,
				10_000 * DOLLARS,
				10_000 * DOLLARS,
			));
			assert!(System::events().iter().any(|record| record.event == expected_event));

			assert_eq!(
				Currencies::free_balance(DOT, &TestLiquidationPools::pools_account_id()),
				290_000 * DOLLARS
			);
			assert_eq!(
				Currencies::free_balance(ETH, &TestLiquidationPools::pools_account_id()),
				410_000 * DOLLARS
			);

			assert_eq!(
				Currencies::free_balance(DOT, &TestDex::dex_account_id()),
				60_000 * DOLLARS
			);
			assert_eq!(
				Currencies::free_balance(ETH, &TestDex::dex_account_id()),
				40_000 * DOLLARS
			);

			assert_err!(
				TestDex::do_swap_with_exact_target(
					&TestLiquidationPools::pools_account_id(),
					DOT,
					ETH,
					100_000 * DOLLARS,
					100_000 * DOLLARS
				),
				Error::<TestRuntime>::InsufficientDexBalance
			);
		});
}
