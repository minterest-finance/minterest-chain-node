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

fn hypothetical_liquidity_setup() -> Result<(AccountId, AccountId), &'static str> {
	let borrower: AccountId = account("borrower", 0, SEED);
	let lender: AccountId = account("lender", 0, SEED);

	// set balance for users
	EnabledWrappedTokensId::get()
		.into_iter()
		.try_for_each(|token_id| -> Result<(), &'static str> {
			set_balance(token_id, &borrower, 10_000 * DOLLARS)?;
			Ok(())
		})?;
	set_balance(MDOT, &lender, 20_000 * DOLLARS)?;

	// set balance for Pools
	set_balance(DOT, &LiquidityPoolsModuleId::get().into_account(), 20_000 * DOLLARS)?;
	set_balance(BTC, &LiquidityPoolsModuleId::get().into_account(), 20_000 * DOLLARS)?;

	// enable pool as collateral
	EnabledUnderlyingAssetsIds::get()
		.into_iter()
		.try_for_each(|asset_id| -> Result<(), &'static str> {
			enable_is_collateral_mock::<Runtime>(Origin::signed(borrower.clone()), asset_id)?;
			// set borrow params
			LiquidityPools::set_pool_total_borrowed(asset_id, 10_000 * DOLLARS);
			LiquidityPools::set_user_total_borrowed_and_interest_index(
				&borrower.clone(),
				asset_id,
				10_000 * DOLLARS,
				Rate::one(),
			);
			Ok(())
		})?;
	Ok((borrower, lender))
}

runtime_benchmarks! {
	{ Runtime, minterest_protocol }

	_ {}

	deposit_underlying {
		let lender = account("lender", 0, SEED);
		// set balance for user
		set_balance(DOT, &lender, 10_000 * DOLLARS)?;

		enable_whitelist_mode_and_add_member(lender.clone())?;
	}: _(RawOrigin::Signed(lender), DOT, 10_000 * DOLLARS)
	verify { assert_eq!(Currencies::free_balance(DOT, &LiquidityPoolsModuleId::get().into_account() ), 10_000 * DOLLARS) }

	redeem {
		let (borrower, lender) = hypothetical_liquidity_setup()?;

		enable_whitelist_mode_and_add_member(borrower.clone())?;
	}: _(RawOrigin::Signed(borrower.clone()), DOT)
	verify { assert_eq!(Currencies::free_balance(DOT, &borrower ), 10_000_000_009_000_000_000_000u128) }

	redeem_underlying {
		let (borrower, lender) = hypothetical_liquidity_setup()?;

		enable_whitelist_mode_and_add_member(borrower.clone())?;
	}: _(RawOrigin::Signed(borrower.clone()), DOT, 10_000 * DOLLARS)
	verify { assert_eq!(Currencies::free_balance(DOT, &borrower ), 10_000 * DOLLARS) }

	redeem_wrapped {
		let (borrower, lender) = hypothetical_liquidity_setup()?;

		enable_whitelist_mode_and_add_member(borrower.clone())?;
	}: _(RawOrigin::Signed(borrower.clone()), MDOT, 10_000 * DOLLARS)
	verify { assert_eq!(Currencies::free_balance(DOT, &borrower ), 10_000_000_009_000_000_000_000u128) }

	borrow {
		let (borrower, lender) = hypothetical_liquidity_setup()?;

		enable_whitelist_mode_and_add_member(borrower.clone())?;
	}: _(RawOrigin::Signed(borrower.clone()), DOT, 10_000 * DOLLARS)
	verify { assert_eq!(Currencies::free_balance(DOT, &borrower ), 10_000 * DOLLARS) }

	repay {
		let borrower: AccountId = account("borrower", 0, SEED);
		// set balance for user
		set_balance(DOT, &borrower, 20_000 * DOLLARS)?;
		set_balance(MDOT, &borrower, 12_000 * DOLLARS)?;
		// set borrow params
		LiquidityPools::set_pool_total_borrowed(DOT, 10_000 * DOLLARS);
		LiquidityPools::set_user_total_borrowed_and_interest_index(&borrower.clone(), DOT, 10_000 * DOLLARS, Rate::one());

		enable_whitelist_mode_and_add_member(borrower.clone())?;
	}: _(RawOrigin::Signed(borrower.clone()), DOT, 10_000 * DOLLARS)
	verify { assert_eq!(LiquidityPools::pool_user_data(DOT, borrower).total_borrowed, 1_728_000_000_000_000u128) }

	repay_all {
		let borrower:AccountId = account("borrower", 0, SEED);
		// set balance for user
		set_balance(DOT, &borrower, 20_000 * DOLLARS)?;
		set_balance(MDOT, &borrower, 12_000 * DOLLARS)?;
		// set borrow params
		LiquidityPools::set_pool_total_borrowed(DOT, 10_000 * DOLLARS);
		LiquidityPools::set_user_total_borrowed_and_interest_index(&borrower.clone(), DOT, 10_000 * DOLLARS, Rate::one());

		enable_whitelist_mode_and_add_member(borrower.clone())?;
	}: _(RawOrigin::Signed(borrower.clone()), DOT)
	verify { assert_eq!(LiquidityPools::pool_user_data(DOT, borrower).total_borrowed, Balance::zero()) }

	repay_on_behalf {
		let borrower:AccountId = account("borrower", 0, SEED);
		let lender:AccountId = account("lender", 0, SEED);
		// set balance for users
		set_balance(DOT, &lender, 20_000 * DOLLARS)?;
		set_balance(MDOT, &borrower, 12_000 * DOLLARS)?;
		// set borrow params
		LiquidityPools::set_pool_total_borrowed(DOT, 10_000 * DOLLARS);
		LiquidityPools::set_user_total_borrowed_and_interest_index(&borrower.clone(), DOT, 10_000 * DOLLARS, Rate::one());

		enable_whitelist_mode_and_add_member(lender.clone())?;
	}: _(RawOrigin::Signed(lender.clone()), DOT, borrower.clone(), 10_000 * DOLLARS)
	verify { assert_eq!(LiquidityPools::pool_user_data(DOT, borrower).total_borrowed, 1_728_000_000_000_000u128) }

	transfer_wrapped {
		let (borrower, lender) = hypothetical_liquidity_setup()?;

		enable_whitelist_mode_and_add_member(borrower.clone())?;
	}: _(RawOrigin::Signed(borrower.clone()), lender.clone(), MDOT, 10_000 * DOLLARS)
	verify  {
		assert_eq!(Currencies::free_balance(MDOT, &borrower ), Balance::zero());
		assert_eq!(Currencies::free_balance(MDOT, &lender ), 30_000 * DOLLARS);
	 }

	enable_is_collateral {
		let borrower:AccountId = account("borrower", 0, SEED);
		// set balance for users
		set_balance(MDOT, &borrower, 10_000 * DOLLARS)?;

		enable_whitelist_mode_and_add_member(borrower.clone())?;
	}: _(RawOrigin::Signed(borrower.clone()), DOT)
	verify  { assert_eq!(LiquidityPools::pool_user_data(DOT, borrower).is_collateral, true) }

	disable_is_collateral {
		let (borrower, lender) = hypothetical_liquidity_setup()?;

		enable_whitelist_mode_and_add_member(borrower.clone())?;
	}: _(RawOrigin::Signed(borrower.clone()), DOT)
	verify  { assert_eq!(LiquidityPools::pool_user_data(DOT, borrower).is_collateral, false) }

	claim_mnt {
		let lender: AccountId = account("lender", 0, SEED);
		let borrower: AccountId = account("borrower", 0, SEED);
		enable_whitelist_mode_and_add_member(lender.clone())?;
		enable_whitelist_mode_and_add_member(borrower.clone())?;

		set_balance(
			MNT,
			&MntTokenModuleId::get().into_account(),
			1_000_000 * DOLLARS,
		)?;
		set_balance(MNT, &lender, 1_000_000 * DOLLARS)?;
		set_balance(MNT, &borrower, 1_000_000 * DOLLARS)?;

		// set next block number and refresh speeds
		System::set_block_number(10);
		MntToken::refresh_mnt_speeds()?;

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
		Initial balance 1_000_000 MNT + accrued MNT tokens: 500_000 MNT - gas payment.
		 */
		assert_eq!(Currencies::free_balance(MNT, &borrower), 1_000_499_999_967_375_001_870_120)
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
