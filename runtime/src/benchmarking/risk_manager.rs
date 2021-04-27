use super::utils::{
	enable_whitelist_mode_and_add_member, lookup_of_account, prepare_for_mnt_distribution, set_balance, SEED,
};
use crate::{
	AccountId, Currencies, EnabledUnderlyingAssetsIds, LiquidationPools, LiquidationPoolsModuleId, LiquidityPools,
	MinterestProtocol, MntToken, Origin, Rate, Runtime, System, BTC, DOLLARS, DOT, ETH, KSM, MNT,
};
use frame_benchmarking::account;
use frame_system::RawOrigin;
use orml_benchmarking::runtime_benchmarks;
use orml_traits::MultiCurrency;
use pallet_traits::PoolsManager;
use sp_runtime::traits::AccountIdConversion;
use sp_runtime::FixedPointNumber;
use sp_std::prelude::*;

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
		prepare_for_mnt_distribution(EnabledUnderlyingAssetsIds::get())?;
		let borrower: AccountId = account("ownerx", 0, SEED);
		let borrower_lookup = lookup_of_account(borrower.clone());

		System::set_block_number(10);
		MntToken::refresh_mnt_speeds()?;

		enable_whitelist_mode_and_add_member(&borrower)?;

		EnabledUnderlyingAssetsIds::get().into_iter().try_for_each(|pool_id| -> Result<(), &'static str> {
			set_balance(pool_id, &borrower, 100_000 * DOLLARS)?;
			MinterestProtocol::deposit_underlying(RawOrigin::Signed(borrower.clone()).into(), pool_id, 10_000 * DOLLARS)?;
			MinterestProtocol::enable_is_collateral(Origin::signed(borrower.clone()).into(), pool_id)?;
			Ok(())
		})?;

		MinterestProtocol::borrow(RawOrigin::Signed(borrower.clone()).into(), DOT, 35_000 * DOLLARS)?;

		let liquidation_pool_account_id: AccountId = LiquidationPoolsModuleId::get().into_account();

		// set balance for Liquidation Pool
		set_balance(DOT, &liquidation_pool_account_id, 40_000 * DOLLARS)?;

		System::set_block_number(20);
	}: _(
		RawOrigin::None,
		borrower_lookup,
		DOT
	) verify {
		assert_eq!(LiquidityPools::get_pool_available_liquidity(DOT), 40_000_001_906_875_001_666_225);
		assert_eq!(LiquidityPools::get_pool_available_liquidity(ETH), 43_249_998_019_999_999_547_975);
		assert_eq!(LiquidityPools::get_pool_available_liquidity(KSM), 39_999_999_977_499_999_322_900);
		assert_eq!(LiquidityPools::get_pool_available_liquidity(BTC), 39_999_999_977_499_999_322_900);
		assert_eq!(Currencies::free_balance(MNT, &borrower), 36_111_110_988_333_296_544);
	}
}

#[cfg(test)]
pub mod tests {
	use super::*;
	use crate::benchmarking::utils::tests::test_externalities;
	use frame_support::assert_ok;

	#[test]
	fn test_set_max_attempts() {
		test_externalities().execute_with(|| {
			assert_ok!(test_benchmark_set_max_attempts());
		})
	}

	#[test]
	fn test_set_min_partial_liquidation_sum() {
		test_externalities().execute_with(|| {
			assert_ok!(test_benchmark_set_min_partial_liquidation_sum());
		})
	}

	#[test]
	fn test_set_threshold() {
		test_externalities().execute_with(|| {
			assert_ok!(test_benchmark_set_threshold());
		})
	}

	#[test]
	fn test_set_liquidation_fee() {
		test_externalities().execute_with(|| {
			assert_ok!(test_benchmark_set_liquidation_fee());
		})
	}

	#[test]
	fn test_liquidate() {
		test_externalities().execute_with(|| {
			assert_ok!(test_benchmark_liquidate());
		})
	}
}
