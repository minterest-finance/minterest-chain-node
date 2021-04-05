use super::utils::{enable_is_collateral, lookup_of_account, set_balance, set_oracle_price_for_all_pools};
use crate::{
	AccountId, CurrencyId, LiquidationPoolsModuleId, LiquidityPools, LiquidityPoolsModuleId, Origin, Rate, Runtime,
	System, DOLLARS,
};
use frame_benchmarking::account;
use frame_system::RawOrigin;
use orml_benchmarking::runtime_benchmarks;
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
		CurrencyId::DOT,
		1u8
	)

	set_min_partial_liquidation_sum {
	}: _(
		RawOrigin::Root,
		CurrencyId::DOT,
		100u128
	)

	set_threshold {
	}: _(
		RawOrigin::Root,
		CurrencyId::DOT,
		Rate::one()
	)

	set_liquidation_fee {
	}: _(
		RawOrigin::Root,
		CurrencyId::DOT,
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
		set_balance(CurrencyId::MDOT, &borrower, 10_000 * DOLLARS)?;
		set_balance(CurrencyId::METH, &borrower, 10_000 * DOLLARS)?;
		set_balance(CurrencyId::MKSM, &borrower, 10_000 * DOLLARS)?;
		set_balance(CurrencyId::MBTC, &borrower, 10_000 * DOLLARS)?;
		set_balance(CurrencyId::MDOT, &lender, 30_000 * DOLLARS)?;

		// set balance for LiquidityPools
		set_balance(CurrencyId::DOT, &liquidity_pool_account_id, 5_000 * DOLLARS)?;
		set_balance(CurrencyId::ETH, &liquidity_pool_account_id, 10_000 * DOLLARS)?;
		set_balance(CurrencyId::KSM, &liquidity_pool_account_id, 10_000 * DOLLARS)?;
		set_balance(CurrencyId::BTC, &liquidity_pool_account_id, 10_000 * DOLLARS)?;

		// set balance for LiquidationPools
		set_balance(CurrencyId::DOT, &liquidation_pool_account_id, 40_000 * DOLLARS)?;

		// enable pool as collateral
		enable_is_collateral::<Runtime>(Origin::signed(borrower.clone()), CurrencyId::DOT)?;
		enable_is_collateral::<Runtime>(Origin::signed(borrower.clone()), CurrencyId::ETH)?;
		enable_is_collateral::<Runtime>(Origin::signed(borrower.clone()), CurrencyId::KSM)?;
		enable_is_collateral::<Runtime>(Origin::signed(borrower.clone()), CurrencyId::BTC)?;

		// set borrow params
		LiquidityPools::set_pool_total_borrowed(CurrencyId::DOT, 35_000 * DOLLARS);
		LiquidityPools::set_user_total_borrowed_and_interest_index(&borrower.clone(), CurrencyId::DOT, 35_000 * DOLLARS, Rate::one());

		// check if borrow params have been set.
		assert_eq!(LiquidityPools::pool_user_data(CurrencyId::DOT, borrower.clone()).total_borrowed, 35_000 * DOLLARS);
		assert_eq!(LiquidityPools::pools(CurrencyId::DOT).total_borrowed, 35_000 * DOLLARS);

		// set next block number for accrue_interest works
		System::set_block_number(2);
	}: _(
		RawOrigin::None,
		borrower_lookup,
		CurrencyId::DOT
	)
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
