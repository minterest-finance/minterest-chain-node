use super::utils::{enable_is_collateral_mock, enable_whitelist_mode_and_add_member, set_balance};
use crate::{
	AccountId, Balance, Currencies, EnabledUnderlyingAssetsIds, EnabledWrappedTokensId, LiquidityPools,
	LiquidityPoolsModuleId, MinterestProtocol, MntToken, MntTokenModuleId, Origin, Rate, Runtime, System, BTC, DOLLARS,
	DOT, ETH, KSM, MDOT, MNT,
};
use frame_benchmarking::account;
use frame_system::RawOrigin;
use orml_benchmarking::runtime_benchmarks;
use orml_traits::MultiCurrency;
use sp_runtime::{
	traits::{AccountIdConversion, Zero},
	FixedPointNumber,
};
use sp_std::prelude::*;

pub const SEED: u32 = 0;

fn prepare_for_mnt_distribution() -> Result<(), &'static str> {
	let helper: AccountId = account("helper", 0, SEED);
	enable_whitelist_mode_and_add_member(&helper)?;
	set_balance(DOT, &helper, 50_000 * DOLLARS)?;
	set_balance(MNT, &MntTokenModuleId::get().into_account(), 1_000_000 * DOLLARS)?;
	MinterestProtocol::deposit_underlying(RawOrigin::Signed(helper.clone()).into(), DOT, 50_000 * DOLLARS)?;
	MinterestProtocol::enable_is_collateral(Origin::signed(helper.clone()).into(), DOT)?;
	MinterestProtocol::borrow(RawOrigin::Signed(helper).into(), DOT, 10_000 * DOLLARS)?;
	Ok(())
}

fn hypothetical_liquidity_setup(borrower: &AccountId, lender: &AccountId) -> Result<(), &'static str> {
	// set balance for users
	EnabledWrappedTokensId::get()
		.into_iter()
		.try_for_each(|token_id| -> Result<(), &'static str> {
			set_balance(token_id, borrower, 10_000 * DOLLARS)?;
			Ok(())
		})?;
	set_balance(MDOT, lender, 20_000 * DOLLARS)?;

	// set balance for Pools
	set_balance(DOT, &LiquidityPoolsModuleId::get().into_account(), 20_000 * DOLLARS)?;
	set_balance(BTC, &LiquidityPoolsModuleId::get().into_account(), 20_000 * DOLLARS)?;

	// enable pools as collateral
	EnabledUnderlyingAssetsIds::get()
		.into_iter()
		.try_for_each(|asset_id| -> Result<(), &'static str> {
			enable_is_collateral_mock::<Runtime>(Origin::signed(borrower.clone()), asset_id)?;
			// set borrow params
			LiquidityPools::set_pool_total_borrowed(asset_id, 10_000 * DOLLARS);
			LiquidityPools::set_user_total_borrowed_and_interest_index(
				borrower,
				asset_id,
				10_000 * DOLLARS,
				Rate::one(),
			);
			Ok(())
		})?;
	Ok(())
}

runtime_benchmarks! {
	{ Runtime, minterest_protocol }

	_ {}

	deposit_underlying {
		prepare_for_mnt_distribution()?;
		let lender: AccountId = account("lender", 0, SEED);
		enable_whitelist_mode_and_add_member(&lender)?;

		// set balance for lender
		set_balance(DOT, &lender, 50_000 * DOLLARS)?;

		System::set_block_number(10);
		MntToken::refresh_mnt_speeds()?;

		MinterestProtocol::deposit_underlying(RawOrigin::Signed(lender.clone()).into(), DOT, 10_000 * DOLLARS)?;

		System::set_block_number(20);

	}: _(RawOrigin::Signed(lender.clone()), DOT, 10_000 * DOLLARS)
	verify {
		assert_eq!(Currencies::free_balance(DOT, &LiquidityPoolsModuleId::get().into_account() ), 60_000 * DOLLARS);
		// mnt_balance = 10(speed) * 10(delta_blocks) * 10(lender_supply) / 60(total_supply) = 16.66 MNT
		assert_eq!(Currencies::free_balance(MNT, &lender), 16_666_666_621_666_660_145)
	}

	redeem {
		prepare_for_mnt_distribution()?;
		let borrower: AccountId = account("borrower", 0, SEED);
		let lender: AccountId = account("lender", 0, SEED);

		System::set_block_number(10);
		MntToken::refresh_mnt_speeds()?;

		enable_whitelist_mode_and_add_member(&borrower)?;
		hypothetical_liquidity_setup(&borrower, &lender)?;

		System::set_block_number(20);

	}: _(RawOrigin::Signed(borrower.clone()), DOT)
	verify {
		assert_eq!(Currencies::free_balance(DOT, &borrower), 8_750_000_028_928_571_410_000);
		// mnt_balance = 10(speed) * 10(delta_blocks) * 10(borrower_supply) / 80(total_supply) = 12.5 MNT
		assert_eq!(Currencies::free_balance(MNT, &borrower), 12_500_000_000_000_000_000)
	}

	redeem_underlying {
		prepare_for_mnt_distribution()?;
		let borrower: AccountId = account("borrower", 0, SEED);
		let lender: AccountId = account("lender", 0, SEED);

		System::set_block_number(10);
		MntToken::refresh_mnt_speeds()?;

		enable_whitelist_mode_and_add_member(&borrower)?;
		hypothetical_liquidity_setup(&borrower, &lender)?;

		System::set_block_number(20);

	}: _(RawOrigin::Signed(borrower.clone()), DOT, 1_000 * DOLLARS)
	verify {
		assert_eq!(Currencies::free_balance(DOT, &borrower ), 1_000 * DOLLARS);
		// mnt_balance = 10(speed) * 10(delta_blocks) * 10(borrower_supply) / 80(total_supply) = 12.5 MNT
		assert_eq!(Currencies::free_balance(MNT, &borrower), 12_500_000_000_000_000_000)
	}

	redeem_wrapped {
		prepare_for_mnt_distribution()?;
		let borrower: AccountId = account("borrower", 0, SEED);
		let lender: AccountId = account("lender", 0, SEED);

		System::set_block_number(10);
		MntToken::refresh_mnt_speeds()?;

		enable_whitelist_mode_and_add_member(&borrower)?;
		hypothetical_liquidity_setup(&borrower, &lender)?;

		System::set_block_number(20);

	}: _(RawOrigin::Signed(borrower.clone()), MDOT, 10_000 * DOLLARS)
	verify {
		assert_eq!(Currencies::free_balance(DOT, &borrower), 8_750_000_028_928_571_410_000);
		// mnt_balance = 10(speed) * 10(delta_blocks) * 10(borrower_supply) / 80(total_supply) = 12.5 MNT
		assert_eq!(Currencies::free_balance(MNT, &borrower), 12_500_000_000_000_000_000)
	}

	borrow {
		prepare_for_mnt_distribution()?;
		let borrower: AccountId = account("borrower", 0, SEED);
		let lender: AccountId = account("lender", 0, SEED);

		System::set_block_number(10);
		MntToken::refresh_mnt_speeds()?;

		enable_whitelist_mode_and_add_member(&borrower)?;
		hypothetical_liquidity_setup(&borrower, &lender)?;

		MinterestProtocol::borrow(RawOrigin::Signed(borrower.clone()).into(), DOT, 5_000 * DOLLARS)?;

		System::set_block_number(20);

	}: _(RawOrigin::Signed(borrower.clone()), DOT, 5_000 * DOLLARS)
	verify {
		assert_eq!(Currencies::free_balance(DOT, &borrower ), 10_000 * DOLLARS);
		assert_eq!(Currencies::free_balance(MNT, &borrower), 99_999_999_999_999_985_340)
	}

	repay {
		prepare_for_mnt_distribution()?;
		let borrower: AccountId = account("borrower", 0, SEED);
		enable_whitelist_mode_and_add_member(&borrower)?;
		set_balance(DOT, &borrower, 100_000 * DOLLARS)?;
		MinterestProtocol::deposit_underlying(RawOrigin::Signed(borrower.clone()).into(), DOT, 50_000 * DOLLARS)?;

		System::set_block_number(10);
		MntToken::refresh_mnt_speeds()?;

		MinterestProtocol::enable_is_collateral(Origin::signed(borrower.clone()).into(), DOT)?;
		MinterestProtocol::borrow(RawOrigin::Signed(borrower.clone()).into(), DOT, 10_000 * DOLLARS)?;

		System::set_block_number(20);

	}: _(RawOrigin::Signed(borrower.clone()), DOT, 10_000 * DOLLARS)
	verify {
		assert_eq!(LiquidityPools::pool_user_data(DOT, borrower.clone()).total_borrowed, 180_000_000_600_000);
		assert_eq!(Currencies::free_balance(MNT, &borrower), 49_999_999_774_999_992_025)
	}

	repay_all {
		prepare_for_mnt_distribution()?;
		let borrower:AccountId = account("borrower", 0, SEED);
		enable_whitelist_mode_and_add_member(&borrower)?;
		set_balance(DOT, &borrower, 100_000 * DOLLARS)?;
		MinterestProtocol::deposit_underlying(RawOrigin::Signed(borrower.clone()).into(), DOT, 50_000 * DOLLARS)?;

		System::set_block_number(10);
		MntToken::refresh_mnt_speeds()?;

		MinterestProtocol::enable_is_collateral(Origin::signed(borrower.clone()).into(), DOT)?;
		MinterestProtocol::borrow(RawOrigin::Signed(borrower.clone()).into(), DOT, 10_000 * DOLLARS)?;

		System::set_block_number(20);

	}: _(RawOrigin::Signed(borrower.clone()), DOT)
	verify {
		assert_eq!(LiquidityPools::pool_user_data(DOT, borrower.clone()).total_borrowed, Balance::zero());
		assert_eq!(Currencies::free_balance(MNT, &borrower), 49_999_999_774_999_992_025)
	}

	repay_on_behalf {
		prepare_for_mnt_distribution()?;
		let borrower: AccountId = account("borrower", 0, SEED);
		let lender: AccountId = account("lender", 0, SEED);
		enable_whitelist_mode_and_add_member(&lender)?;
		enable_whitelist_mode_and_add_member(&borrower)?;
		set_balance(DOT, &lender, 100_000 * DOLLARS)?;
		set_balance(DOT, &borrower, 100_000 * DOLLARS)?;
		MinterestProtocol::deposit_underlying(RawOrigin::Signed(borrower.clone()).into(), DOT, 50_000 * DOLLARS)?;

		System::set_block_number(10);
		MntToken::refresh_mnt_speeds()?;

		MinterestProtocol::enable_is_collateral(Origin::signed(borrower.clone()).into(), DOT)?;
		MinterestProtocol::borrow(RawOrigin::Signed(borrower.clone()).into(), DOT, 10_000 * DOLLARS)?;

		System::set_block_number(20);

	}: _(RawOrigin::Signed(lender.clone()), DOT, borrower.clone(), 10_000 * DOLLARS)
	verify {
		assert_eq!(LiquidityPools::pool_user_data(DOT, borrower.clone()).total_borrowed, 180_000_000_600_000);
		assert_eq!(Currencies::free_balance(MNT, &borrower), 49_999_999_774_999_992_025);
		assert_eq!(Currencies::free_balance(MNT, &lender), Balance::zero());
	}

	transfer_wrapped {
		prepare_for_mnt_distribution()?;
		let borrower: AccountId = account("borrower", 0, SEED);
		let lender: AccountId = account("lender", 0, SEED);

		System::set_block_number(10);
		MntToken::refresh_mnt_speeds()?;

		enable_whitelist_mode_and_add_member(&borrower)?;
		hypothetical_liquidity_setup(&borrower, &lender)?;

		System::set_block_number(20);

	}: _(RawOrigin::Signed(borrower.clone()), lender.clone(), MDOT, 10_000 * DOLLARS)
	verify  {
		assert_eq!(Currencies::free_balance(MDOT, &borrower.clone()), Balance::zero());
		assert_eq!(Currencies::free_balance(MDOT, &lender.clone()), 30_000 * DOLLARS);
		assert_eq!(Currencies::free_balance(MNT, &borrower), 12_500_000_000_000_000_000);
		assert_eq!(Currencies::free_balance(MNT, &lender), 25_000_000_000_000_000_000);
	 }

	enable_is_collateral {
		let borrower:AccountId = account("borrower", 0, SEED);
		// set balance for users
		set_balance(MDOT, &borrower, 10_000 * DOLLARS)?;

		enable_whitelist_mode_and_add_member(&borrower)?;
	}: _(RawOrigin::Signed(borrower.clone()), DOT)
	verify  { assert_eq!(LiquidityPools::pool_user_data(DOT, borrower).is_collateral, true) }

	disable_is_collateral {
		let borrower: AccountId = account("borrower", 0, SEED);
		let lender: AccountId = account("lender", 0, SEED);
		hypothetical_liquidity_setup(&borrower, &lender)?;

		enable_whitelist_mode_and_add_member(&borrower)?;
	}: _(RawOrigin::Signed(borrower.clone()), DOT)
	verify  { assert_eq!(LiquidityPools::pool_user_data(DOT, borrower).is_collateral, false) }

	claim_mnt {
		let lender: AccountId = account("lender", 0, SEED);
		let borrower: AccountId = account("borrower", 0, SEED);
		enable_whitelist_mode_and_add_member(&lender)?;
		enable_whitelist_mode_and_add_member(&borrower)?;

		set_balance(
			MNT,
			&MntTokenModuleId::get().into_account(),
			1_000_000 * DOLLARS,
		)?;

		EnabledUnderlyingAssetsIds::get().into_iter().try_for_each(|pool_id| -> Result<(), &'static str> {
			set_balance(pool_id, &lender, 100_000 * DOLLARS)?;
			MinterestProtocol::deposit_underlying(RawOrigin::Signed(lender.clone()).into(), pool_id, 100_000 * DOLLARS)?;
			MinterestProtocol::enable_is_collateral(Origin::signed(lender.clone()).into(), pool_id)?;
			MinterestProtocol::borrow(RawOrigin::Signed(lender.clone()).into(), pool_id, 50_000 * DOLLARS)?;
			Ok(())
		})?;

		System::set_block_number(50);
		MntToken::refresh_mnt_speeds()?;

		EnabledUnderlyingAssetsIds::get().into_iter().try_for_each(|pool_id| -> Result<(), &'static str> {
			set_balance(pool_id, &borrower, 100_000 * DOLLARS)?;
			MinterestProtocol::deposit_underlying(RawOrigin::Signed(borrower.clone()).into(), pool_id, 100_000 * DOLLARS)?;
			MinterestProtocol::enable_is_collateral(Origin::signed(borrower.clone()).into(), pool_id)?;
			MinterestProtocol::borrow(RawOrigin::Signed(borrower.clone()).into(), pool_id, 50_000 * DOLLARS)?;
			Ok(())
		})?;

		System::set_block_number(100);

	}: _(RawOrigin::Signed(borrower.clone()), vec![DOT, ETH, BTC, KSM])
	verify {
		/*
		Accrued MNT:
		Supply per pool: prev + speed_pool * block_delta * borrower_supply / total_supply
		supply_balance = 0 + (2.5 * 50 * 0.5) * 4 = 250 MNT
		Borrow: prev + speed_pool * block_delta * borrower_borrow / total_borrow
		borrow_balance = 0 + (2.5 * 50 * 0.5) * 4 = 250 MNT
		accrued MNT tokens: 250 + 250 = ~500_000 MNT
		 */
		assert_eq!(Currencies::free_balance(MNT, &borrower), 499_999_959_218_753_609_568)
	}

}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::benchmarking::utils::tests::test_externalities;
	use frame_support::assert_ok;

	#[test]
	fn test_deposit_underlying() {
		test_externalities().execute_with(|| {
			assert_ok!(test_benchmark_deposit_underlying());
		})
	}

	#[test]
	fn test_redeem() {
		test_externalities().execute_with(|| {
			assert_ok!(test_benchmark_redeem());
		})
	}

	#[test]
	fn test_redeem_underlying() {
		test_externalities().execute_with(|| {
			assert_ok!(test_benchmark_redeem_underlying());
		})
	}

	#[test]
	fn test_redeem_wrapped() {
		test_externalities().execute_with(|| {
			assert_ok!(test_benchmark_redeem_wrapped());
		})
	}

	#[test]
	fn test_borrow() {
		test_externalities().execute_with(|| {
			assert_ok!(test_benchmark_borrow());
		})
	}

	#[test]
	fn test_repay() {
		test_externalities().execute_with(|| {
			assert_ok!(test_benchmark_repay());
		})
	}

	#[test]
	fn test_repay_all() {
		test_externalities().execute_with(|| {
			assert_ok!(test_benchmark_repay_all());
		})
	}

	#[test]
	fn test_repay_on_behalf() {
		test_externalities().execute_with(|| {
			assert_ok!(test_benchmark_repay_on_behalf());
		})
	}

	#[test]
	fn test_transfer_wrapped() {
		test_externalities().execute_with(|| {
			assert_ok!(test_benchmark_transfer_wrapped());
		})
	}

	#[test]
	fn test_enable_is_collateral() {
		test_externalities().execute_with(|| {
			assert_ok!(test_benchmark_enable_is_collateral());
		})
	}

	#[test]
	fn test_disable_is_collateral() {
		test_externalities().execute_with(|| {
			assert_ok!(test_benchmark_disable_is_collateral());
		})
	}

	#[test]
	fn test_claim_mnt() {
		test_externalities().execute_with(|| {
			assert_ok!(test_benchmark_claim_mnt());
		})
	}
}
