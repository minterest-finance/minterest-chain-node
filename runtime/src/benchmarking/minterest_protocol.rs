use super::utils::{
	enable_is_collateral, enable_whitelist_mode_a_add_member, set_balance, set_oracle_price_for_all_pools,
};
use crate::{
	AccountId, Balance, Currencies, CurrencyId, LiquidityPools, LiquidityPoolsModuleId, Origin, Rate, Runtime, DOLLARS,
};
use frame_benchmarking::account;
use frame_system::RawOrigin;
use orml_benchmarking::runtime_benchmarks;
use orml_traits::MultiCurrency;
use sp_runtime::traits::AccountIdConversion;
use sp_runtime::traits::Zero;
use sp_runtime::FixedPointNumber;
use sp_std::prelude::*;

pub const SEED: u32 = 0;

fn hypothetical_liquidity_setup() -> Result<(AccountId, AccountId), &'static str> {
	let borrower: AccountId = account("borrower", 0, SEED);
	let lender: AccountId = account("lender", 0, SEED);
	// feed price for each pool
	set_oracle_price_for_all_pools::<Runtime>(2, Origin::root(), 0)?;

	// set balance for users
	set_balance(CurrencyId::MDOT, &borrower, 10_000 * DOLLARS)?;
	set_balance(CurrencyId::METH, &borrower, 10_000 * DOLLARS)?;
	set_balance(CurrencyId::MKSM, &borrower, 10_000 * DOLLARS)?;
	set_balance(CurrencyId::MBTC, &borrower, 30_000 * DOLLARS)?;
	set_balance(CurrencyId::MDOT, &lender, 20_000 * DOLLARS)?;

	// set balance for Pools
	set_balance(
		CurrencyId::DOT,
		&LiquidityPoolsModuleId::get().into_account(),
		20_000 * DOLLARS,
	)?;
	set_balance(
		CurrencyId::BTC,
		&LiquidityPoolsModuleId::get().into_account(),
		20_000 * DOLLARS,
	)?;

	// enable pool as collateral
	enable_is_collateral::<Runtime>(Origin::signed(borrower.clone()), CurrencyId::DOT)?;
	enable_is_collateral::<Runtime>(Origin::signed(borrower.clone()), CurrencyId::ETH)?;
	enable_is_collateral::<Runtime>(Origin::signed(borrower.clone()), CurrencyId::KSM)?;
	enable_is_collateral::<Runtime>(Origin::signed(borrower.clone()), CurrencyId::BTC)?;

	// set borrow params
	LiquidityPools::set_pool_total_borrowed(CurrencyId::DOT, 10_000 * DOLLARS);
	LiquidityPools::set_user_total_borrowed_and_interest_index(
		&borrower.clone(),
		CurrencyId::DOT,
		10_000 * DOLLARS,
		Rate::one(),
	);
	LiquidityPools::set_pool_total_borrowed(CurrencyId::ETH, 10_000 * DOLLARS);
	LiquidityPools::set_user_total_borrowed_and_interest_index(
		&borrower.clone(),
		CurrencyId::ETH,
		10_000 * DOLLARS,
		Rate::one(),
	);
	LiquidityPools::set_pool_total_borrowed(CurrencyId::KSM, 10_000 * DOLLARS);
	LiquidityPools::set_user_total_borrowed_and_interest_index(
		&borrower.clone(),
		CurrencyId::KSM,
		10_000 * DOLLARS,
		Rate::one(),
	);
	LiquidityPools::set_pool_total_borrowed(CurrencyId::BTC, 10_000 * DOLLARS);
	LiquidityPools::set_user_total_borrowed_and_interest_index(
		&borrower.clone(),
		CurrencyId::BTC,
		10_000 * DOLLARS,
		Rate::one(),
	);
	Ok((borrower, lender))
}

runtime_benchmarks! {
	{ Runtime, minterest_protocol }

	_ {}

	deposit_underlying {
		let lender = account("lender", 0, SEED);
		// feed price for each pool
		set_oracle_price_for_all_pools::<Runtime>(2, Origin::root(), 1)?;
		// set balance for user
		set_balance(CurrencyId::DOT, &lender, 10_000 * DOLLARS)?;

		enable_whitelist_mode_a_add_member(lender.clone())?;
	}: _(RawOrigin::Signed(lender), CurrencyId::DOT, 10_000 * DOLLARS)
	verify { assert_eq!(Currencies::free_balance(CurrencyId::DOT, &LiquidityPoolsModuleId::get().into_account() ), 10_000 * DOLLARS) }

	redeem {
		let (borrower, lender) = hypothetical_liquidity_setup()?;

		enable_whitelist_mode_a_add_member(borrower.clone())?;
	}: _(RawOrigin::Signed(borrower.clone()), CurrencyId::DOT)
	verify { assert_eq!(Currencies::free_balance(CurrencyId::DOT, &borrower ), 10_000_000_009_000_000_000_000u128) }

	redeem_underlying {
		let (borrower, lender) = hypothetical_liquidity_setup()?;

		enable_whitelist_mode_a_add_member(borrower.clone())?;
	}: _(RawOrigin::Signed(borrower.clone()), CurrencyId::DOT, 10_000 * DOLLARS)
	verify { assert_eq!(Currencies::free_balance(CurrencyId::DOT, &borrower ), 10_000 * DOLLARS) }

	redeem_wrapped {
		let (borrower, lender) = hypothetical_liquidity_setup()?;

		enable_whitelist_mode_a_add_member(borrower.clone())?;
	}: _(RawOrigin::Signed(borrower.clone()), CurrencyId::MDOT, 10_000 * DOLLARS)
	verify { assert_eq!(Currencies::free_balance(CurrencyId::DOT, &borrower ), 10_000_000_009_000_000_000_000u128) }

	borrow {
		let (borrower, lender) = hypothetical_liquidity_setup()?;

		enable_whitelist_mode_a_add_member(borrower.clone())?;
	}: _(RawOrigin::Signed(borrower.clone()), CurrencyId::DOT, 10_000 * DOLLARS)
	verify { assert_eq!(Currencies::free_balance(CurrencyId::DOT, &borrower ), 10_000 * DOLLARS) }

	repay {
		let borrower: AccountId = account("borrower", 0, SEED);
		// set balance for user
		set_balance(CurrencyId::DOT, &borrower, 20_000 * DOLLARS)?;
		set_balance(CurrencyId::MDOT, &borrower, 12_000 * DOLLARS)?;
		// set borrow params
		LiquidityPools::set_pool_total_borrowed(CurrencyId::DOT, 10_000 * DOLLARS);
		LiquidityPools::set_user_total_borrowed_and_interest_index(&borrower.clone(), CurrencyId::DOT, 10_000 * DOLLARS, Rate::one());

		enable_whitelist_mode_a_add_member(borrower.clone())?;
	}: _(RawOrigin::Signed(borrower.clone()), CurrencyId::DOT, 10_000 * DOLLARS)
	verify { assert_eq!(LiquidityPools::pool_user_data(CurrencyId::DOT, borrower).total_borrowed, 1_728_000_000_000_000u128) }

	repay_all {
		let borrower:AccountId = account("borrower", 0, SEED);
		// set balance for user
		set_balance(CurrencyId::DOT, &borrower, 20_000 * DOLLARS)?;
		set_balance(CurrencyId::MDOT, &borrower, 12_000 * DOLLARS)?;
		// set borrow params
		LiquidityPools::set_pool_total_borrowed(CurrencyId::DOT, 10_000 * DOLLARS);
		LiquidityPools::set_user_total_borrowed_and_interest_index(&borrower.clone(), CurrencyId::DOT, 10_000 * DOLLARS, Rate::one());

		enable_whitelist_mode_a_add_member(borrower.clone())?;
	}: _(RawOrigin::Signed(borrower.clone()), CurrencyId::DOT)
	verify { assert_eq!(LiquidityPools::pool_user_data(CurrencyId::DOT, borrower).total_borrowed, Balance::zero()) }

	repay_on_behalf {
		let borrower:AccountId = account("borrower", 0, SEED);
		let lender:AccountId = account("lender", 0, SEED);
		// set balance for users
		set_balance(CurrencyId::DOT, &lender, 20_000 * DOLLARS)?;
		set_balance(CurrencyId::MDOT, &borrower, 12_000 * DOLLARS)?;
		// set borrow params
		LiquidityPools::set_pool_total_borrowed(CurrencyId::DOT, 10_000 * DOLLARS);
		LiquidityPools::set_user_total_borrowed_and_interest_index(&borrower.clone(), CurrencyId::DOT, 10_000 * DOLLARS, Rate::one());

		enable_whitelist_mode_a_add_member(lender.clone())?;
	}: _(RawOrigin::Signed(lender.clone()), CurrencyId::DOT, borrower.clone(), 10_000 * DOLLARS)
	verify { assert_eq!(LiquidityPools::pool_user_data(CurrencyId::DOT, borrower).total_borrowed, 1_728_000_000_000_000u128) }

	transfer_wrapped {
		let (borrower, lender) = hypothetical_liquidity_setup()?;

		enable_whitelist_mode_a_add_member(borrower.clone())?;
	}: _(RawOrigin::Signed(borrower.clone()), lender.clone(), CurrencyId::MDOT, 10_000 * DOLLARS)
	verify  {
		assert_eq!(Currencies::free_balance(CurrencyId::MDOT, &borrower ), Balance::zero());
		assert_eq!(Currencies::free_balance(CurrencyId::MDOT, &lender ), 30_000 * DOLLARS);
	 }

	enable_as_collateral {
		let borrower:AccountId = account("borrower", 0, SEED);
		// set balance for users
		set_balance(CurrencyId::MDOT, &borrower, 10_000 * DOLLARS)?;

		enable_whitelist_mode_a_add_member(borrower.clone())?;
	}: enable_is_collateral(RawOrigin::Signed(borrower.clone()), CurrencyId::DOT)
	verify  { assert_eq!(LiquidityPools::pool_user_data(CurrencyId::DOT, borrower).is_collateral, true) }

	disable_is_collateral {
		let (borrower, lender) = hypothetical_liquidity_setup()?;

		enable_whitelist_mode_a_add_member(borrower.clone())?;
	}: _(RawOrigin::Signed(borrower.clone()), CurrencyId::DOT)
	verify  { assert_eq!(LiquidityPools::pool_user_data(CurrencyId::DOT, borrower).is_collateral, false) }

}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::benchmarking::utils::tests::new_test_ext;
	use frame_support::assert_ok;

	#[test]
	fn test_deposit_underlying() {
		new_test_ext().execute_with(|| {
			assert_ok!(test_benchmark_deposit_underlying());
		})
	}

	#[test]
	fn test_redeem() {
		new_test_ext().execute_with(|| {
			assert_ok!(test_benchmark_redeem());
		})
	}

	#[test]
	fn test_redeem_underlying() {
		new_test_ext().execute_with(|| {
			assert_ok!(test_benchmark_redeem_underlying());
		})
	}

	#[test]
	fn test_redeem_wrapped() {
		new_test_ext().execute_with(|| {
			assert_ok!(test_benchmark_redeem_wrapped());
		})
	}

	#[test]
	fn test_borrow() {
		new_test_ext().execute_with(|| {
			assert_ok!(test_benchmark_borrow());
		})
	}

	#[test]
	fn test_repay() {
		new_test_ext().execute_with(|| {
			assert_ok!(test_benchmark_repay());
		})
	}

	#[test]
	fn test_repay_all() {
		new_test_ext().execute_with(|| {
			assert_ok!(test_benchmark_repay_all());
		})
	}

	#[test]
	fn test_repay_on_behalf() {
		new_test_ext().execute_with(|| {
			assert_ok!(test_benchmark_repay_on_behalf());
		})
	}

	#[test]
	fn test_transfer_wrapped() {
		new_test_ext().execute_with(|| {
			assert_ok!(test_benchmark_transfer_wrapped());
		})
	}

	#[test]
	fn test_enable_is_collateral() {
		new_test_ext().execute_with(|| {
			assert_ok!(test_benchmark_enable_as_collateral());
		})
	}

	#[test]
	fn test_disable_is_collateral() {
		new_test_ext().execute_with(|| {
			assert_ok!(test_benchmark_disable_is_collateral());
		})
	}
}
