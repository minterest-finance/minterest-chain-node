use super::utils::{enable_as_collateral, lookup_of_account, set_balance, set_oracle_price_for_all_pools};
use crate::{
	AccountId, Balance, CurrencyId, LiquidationPoolsModuleId, LiquidityPools, LiquidityPoolsModuleId, Origin, Rate,
	Runtime, System, DOLLARS,
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

	set_min_sum {
	}: _(
		RawOrigin::Root,
		CurrencyId::DOT,
		100u128
	)

	set_threshold {
	}: _(
		RawOrigin::Root,
		CurrencyId::DOT,
		10u128,
		10u128
	)

	set_liquidation_incentive {
	}: _(
		RawOrigin::Root,
		CurrencyId::DOT,
		10u128,
		10u128
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

		// set balance for user
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
		enable_as_collateral::<Runtime>(Origin::signed(borrower.clone()), CurrencyId::DOT)?;
		enable_as_collateral::<Runtime>(Origin::signed(borrower.clone()), CurrencyId::ETH)?;
		enable_as_collateral::<Runtime>(Origin::signed(borrower.clone()), CurrencyId::KSM)?;
		enable_as_collateral::<Runtime>(Origin::signed(borrower.clone()), CurrencyId::BTC)?;

		// set borrow params
		LiquidityPools::set_pool_total_borrowed(CurrencyId::DOT, 35_000 * DOLLARS)?;
		LiquidityPools::set_user_total_borrowed_and_interest_index(&borrower.clone(), CurrencyId::DOT, 35_000 * DOLLARS, Rate::one())?;

		// check params after do_borrow
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
mod tests {
	use super::*;
	use controller::{ControllerData, PauseKeeper};
	use frame_support::assert_ok;
	use frame_support::traits::GenesisBuild;
	use liquidity_pools::Pool;
	use minterest_model::MinterestModelData;
	use risk_manager::RiskManagerData;
	use sp_runtime::traits::Zero;

	fn new_test_ext() -> sp_io::TestExternalities {
		let mut t = frame_system::GenesisConfig::default()
			.build_storage::<Runtime>()
			.unwrap();
		liquidity_pools::GenesisConfig::<Runtime> {
			pools: vec![
				(
					CurrencyId::ETH,
					Pool {
						total_borrowed: Balance::zero(),
						borrow_index: Rate::one(),
						total_insurance: Balance::zero(),
					},
				),
				(
					CurrencyId::DOT,
					Pool {
						total_borrowed: Balance::zero(),
						borrow_index: Rate::one(),
						total_insurance: Balance::zero(),
					},
				),
				(
					CurrencyId::KSM,
					Pool {
						total_borrowed: Balance::zero(),
						borrow_index: Rate::one(),
						total_insurance: Balance::zero(),
					},
				),
				(
					CurrencyId::BTC,
					Pool {
						total_borrowed: Balance::zero(),
						borrow_index: Rate::one(),
						total_insurance: Balance::zero(),
					},
				),
			],
			pool_user_data: vec![],
		}
		.assimilate_storage(&mut t)
		.unwrap();

		controller::GenesisConfig::<Runtime> {
			controller_dates: vec![
				(
					CurrencyId::ETH,
					ControllerData {
						timestamp: 0,
						insurance_factor: Rate::saturating_from_rational(1, 10),
						max_borrow_rate: Rate::saturating_from_rational(5, 1000),
						collateral_factor: Rate::saturating_from_rational(9, 10), // 90%
						borrow_cap: None,
					},
				),
				(
					CurrencyId::DOT,
					ControllerData {
						timestamp: 0,
						insurance_factor: Rate::saturating_from_rational(1, 10),
						max_borrow_rate: Rate::saturating_from_rational(5, 1000),
						collateral_factor: Rate::saturating_from_rational(9, 10), // 90%
						borrow_cap: None,
					},
				),
				(
					CurrencyId::KSM,
					ControllerData {
						timestamp: 0,
						insurance_factor: Rate::saturating_from_rational(1, 10),
						max_borrow_rate: Rate::saturating_from_rational(5, 1000),
						collateral_factor: Rate::saturating_from_rational(9, 10), // 90%
						borrow_cap: None,
					},
				),
				(
					CurrencyId::BTC,
					ControllerData {
						timestamp: 0,
						insurance_factor: Rate::saturating_from_rational(1, 10),
						max_borrow_rate: Rate::saturating_from_rational(5, 1000),
						collateral_factor: Rate::saturating_from_rational(9, 10), // 90%
						borrow_cap: None,
					},
				),
			],
			pause_keepers: vec![
				(
					CurrencyId::ETH,
					PauseKeeper {
						deposit_paused: false,
						redeem_paused: false,
						borrow_paused: false,
						repay_paused: false,
						transfer_paused: false,
					},
				),
				(
					CurrencyId::DOT,
					PauseKeeper {
						deposit_paused: false,
						redeem_paused: false,
						borrow_paused: false,
						repay_paused: false,
						transfer_paused: false,
					},
				),
				(
					CurrencyId::KSM,
					PauseKeeper {
						deposit_paused: false,
						redeem_paused: false,
						borrow_paused: false,
						repay_paused: false,
						transfer_paused: false,
					},
				),
				(
					CurrencyId::BTC,
					PauseKeeper {
						deposit_paused: false,
						redeem_paused: false,
						borrow_paused: false,
						repay_paused: false,
						transfer_paused: false,
					},
				),
			],
			whitelist_mode: false,
		}
		.assimilate_storage(&mut t)
		.unwrap();

		minterest_model::GenesisConfig {
			minterest_model_dates: vec![
				(
					CurrencyId::ETH,
					MinterestModelData {
						kink: Rate::saturating_from_rational(8, 10), // 0.8 = 80 %
						base_rate_per_block: Rate::zero(),
						multiplier_per_block: Rate::saturating_from_rational(9, 1_000_000_000), // 0.047304 PerYear
						jump_multiplier_per_block: Rate::saturating_from_rational(207, 1_000_000_000), // 1.09 PerYear
					},
				),
				(
					CurrencyId::DOT,
					MinterestModelData {
						kink: Rate::saturating_from_rational(8, 10), // 0.8 = 80 %
						base_rate_per_block: Rate::zero(),
						multiplier_per_block: Rate::saturating_from_rational(9, 1_000_000_000), // 0.047304 PerYear
						jump_multiplier_per_block: Rate::saturating_from_rational(207, 1_000_000_000), // 1.09 PerYear
					},
				),
				(
					CurrencyId::KSM,
					MinterestModelData {
						kink: Rate::saturating_from_rational(8, 10), // 0.8 = 80 %
						base_rate_per_block: Rate::zero(),
						multiplier_per_block: Rate::saturating_from_rational(9, 1_000_000_000), // 0.047304 PerYear
						jump_multiplier_per_block: Rate::saturating_from_rational(207, 1_000_000_000), // 1.09 PerYear
					},
				),
				(
					CurrencyId::BTC,
					MinterestModelData {
						kink: Rate::saturating_from_rational(8, 10), // 0.8 = 80 %
						base_rate_per_block: Rate::zero(),
						multiplier_per_block: Rate::saturating_from_rational(9, 1_000_000_000), // 0.047304 PerYear
						jump_multiplier_per_block: Rate::saturating_from_rational(207, 1_000_000_000), // 1.09 PerYear
					},
				),
			],
		}
		.assimilate_storage::<Runtime>(&mut t)
		.unwrap();

		risk_manager::GenesisConfig {
			risk_manager_dates: vec![
				(
					CurrencyId::ETH,
					RiskManagerData {
						max_attempts: 2,
						min_sum: 200_000 * DOLLARS,                          // In USD. FIXME: temporary value.
						threshold: Rate::saturating_from_rational(103, 100), // 3%
						liquidation_incentive: Rate::saturating_from_rational(105, 100), // 5%
					},
				),
				(
					CurrencyId::DOT,
					RiskManagerData {
						max_attempts: 2,
						min_sum: 100_000 * DOLLARS,                          // In USD. FIXME: temporary value.
						threshold: Rate::saturating_from_rational(103, 100), // 3%
						liquidation_incentive: Rate::saturating_from_rational(105, 100), // 5%
					},
				),
				(
					CurrencyId::KSM,
					RiskManagerData {
						max_attempts: 2,
						min_sum: 200_000 * DOLLARS,                          // In USD. FIXME: temporary value.
						threshold: Rate::saturating_from_rational(103, 100), // 3%
						liquidation_incentive: Rate::saturating_from_rational(105, 100), // 5%
					},
				),
				(
					CurrencyId::BTC,
					RiskManagerData {
						max_attempts: 2,
						min_sum: 200_000 * DOLLARS,                          // In USD. FIXME: temporary value.
						threshold: Rate::saturating_from_rational(103, 100), // 3%
						liquidation_incentive: Rate::saturating_from_rational(105, 100), // 5%
					},
				),
			],
		}
		.assimilate_storage::<Runtime>(&mut t)
		.unwrap();

		t.into()
	}

	#[test]
	fn test_set_max_attempts() {
		new_test_ext().execute_with(|| {
			assert_ok!(test_benchmark_set_max_attempts());
		})
	}

	#[test]
	fn test_set_min_sum() {
		new_test_ext().execute_with(|| {
			assert_ok!(test_benchmark_set_min_sum());
		})
	}

	#[test]
	fn test_set_threshold() {
		new_test_ext().execute_with(|| {
			assert_ok!(test_benchmark_set_threshold());
		})
	}

	#[test]
	fn test_set_liquidation_incentive() {
		new_test_ext().execute_with(|| {
			assert_ok!(test_benchmark_set_liquidation_incentive());
		})
	}

	#[test]
	fn test_liquidate() {
		new_test_ext().execute_with(|| {
			assert_ok!(test_benchmark_liquidate());
		})
	}
}
