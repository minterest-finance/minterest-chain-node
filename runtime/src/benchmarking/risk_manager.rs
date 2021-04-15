use super::utils::{enable_is_collateral, lookup_of_account, set_balance, set_oracle_price_for_all_pools};
use crate::{
	AccountId, LiquidationPoolsModuleId, LiquidityPools, LiquidityPoolsModuleId, Origin, Rate, Runtime, System, BTC,
	DOLLARS, DOT, ETH, KSM, MBTC, MDOT, METH, MKSM,
};
use frame_benchmarking::account;
use frame_system::RawOrigin;
use orml_benchmarking::runtime_benchmarks;
use pallet_traits::PoolsManager;
use sp_runtime::traits::AccountIdConversion;
use sp_runtime::FixedPointNumber;
use sp_std::prelude::*;

pub const SEED: u32 = 0;

runtime_benchmarks! {
	{ Runtime, risk_manager }

	_ {}

	set_max_attempts {
	}: _(
		RawOrigin::Root,
		DOT,
		1u8
	)

	set_min_partial_liquidation_sum {
	}: _(
		RawOrigin::Root,
		DOT,
		100u128
	)

	set_threshold {
	}: _(
		RawOrigin::Root,
		DOT,
		Rate::one()
	)

	set_liquidation_fee {
	}: _(
		RawOrigin::Root,
		DOT,
		Rate::one()
	)

	liquidate {
		System::set_block_number(1);

		let borrower: AccountId = account("ownerx", 0, SEED);
		let lender: AccountId = account("ownery", 0, SEED);
		let borrower_lookup = lookup_of_account(borrower.clone());

		let liquidity_pool_account_id = LiquidityPoolsModuleId::get().into_account();
		let liquidation_pool_account_id = LiquidationPoolsModuleId::get().into_account();

		// feed price for each pool
		set_oracle_price_for_all_pools::<Runtime>(2, Origin::root(), 1)?;

		// set balance for users
		set_balance(MDOT, &borrower, 10_000 * DOLLARS)?;
		set_balance(METH, &borrower, 10_000 * DOLLARS)?;
		set_balance(MKSM, &borrower, 10_000 * DOLLARS)?;
		set_balance(MBTC, &borrower, 10_000 * DOLLARS)?;
		set_balance(MDOT, &lender, 30_000 * DOLLARS)?;

		// set balance for Liquidity Pool
		set_balance(DOT, &liquidity_pool_account_id, 5_000 * DOLLARS)?;
		set_balance(ETH, &liquidity_pool_account_id, 10_000 * DOLLARS)?;
		set_balance(KSM, &liquidity_pool_account_id, 10_000 * DOLLARS)?;
		set_balance(BTC, &liquidity_pool_account_id, 10_000 * DOLLARS)?;

		// set balance for Liquidation Pool
		set_balance(DOT, &liquidation_pool_account_id, 40_000 * DOLLARS)?;

		// enable pools as collateral
		enable_is_collateral::<Runtime>(Origin::signed(borrower.clone()), DOT)?;
		enable_is_collateral::<Runtime>(Origin::signed(borrower.clone()), ETH)?;
		enable_is_collateral::<Runtime>(Origin::signed(borrower.clone()), KSM)?;
		enable_is_collateral::<Runtime>(Origin::signed(borrower.clone()), BTC)?;

		// set borrow params
		LiquidityPools::set_pool_total_borrowed(DOT, 35_000 * DOLLARS);
		LiquidityPools::set_user_total_borrowed_and_interest_index(&borrower.clone(), DOT, 35_000 * DOLLARS, Rate::one());

		// check if borrow params have been set.
		assert_eq!(LiquidityPools::pool_user_data(DOT, borrower.clone()).total_borrowed, 35_000 * DOLLARS);
		assert_eq!(LiquidityPools::pools(DOT).total_borrowed, 35_000 * DOLLARS);

		// set next block number for accrue_interest to work
		System::set_block_number(2);
	}: _(
		RawOrigin::None,
		borrower_lookup,
		DOT
	) verify {
		assert_eq!(LiquidityPools::get_pool_available_liquidity(DOT), 30_000_008_251_425_000_000_000);
		assert_eq!(LiquidityPools::get_pool_available_liquidity(ETH), 3_249_991_216_225_000_000_000);
		assert_eq!(LiquidityPools::get_pool_available_liquidity(KSM), 0);
		assert_eq!(LiquidityPools::get_pool_available_liquidity(BTC), 0);
	}
}

#[cfg(test)]
pub mod tests {
	use super::*;
	use crate::benchmarking::utils::tests::new_test_ext;
	use frame_support::assert_ok;

	#[test]
	fn test_set_max_attempts() {
		new_test_ext().execute_with(|| {
			assert_ok!(test_benchmark_set_max_attempts());
		})
	}

	#[test]
	fn test_set_min_partial_liquidation_sum() {
		new_test_ext().execute_with(|| {
			assert_ok!(test_benchmark_set_min_partial_liquidation_sum());
		})
	}

	#[test]
	fn test_set_threshold() {
		new_test_ext().execute_with(|| {
			assert_ok!(test_benchmark_set_threshold());
		})
	}

	#[test]
	fn test_set_liquidation_fee() {
		new_test_ext().execute_with(|| {
			assert_ok!(test_benchmark_set_liquidation_fee());
		})
	}

	#[test]
	fn test_liquidate() {
		new_test_ext().execute_with(|| {
			assert_ok!(test_benchmark_liquidate());
		})
	}
}
