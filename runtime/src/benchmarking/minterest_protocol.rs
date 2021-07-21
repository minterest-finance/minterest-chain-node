use super::utils::{
	enable_is_collateral_mock, enable_whitelist_mode_and_add_member, prepare_for_mnt_distribution, set_balance, SEED,
};
use crate::{
	AccountId, Balance, Currencies, EnabledUnderlyingAssetsIds, EnabledWrappedTokensId, LiquidityPools,
	LiquidityPoolsPalletId, MinterestProtocol, MntTokenPalletId, Origin, Rate, RiskManager, Runtime, System, Whitelist,
	BTC, DOLLARS, DOT, ETH, KSM, MBTC, MDOT, MNT,
};
use frame_benchmarking::account;
use frame_system::RawOrigin;
use liquidity_pools::Pool;
use minterest_primitives::Operation;
use minterest_protocol::PoolInitData;
use orml_benchmarking::runtime_benchmarks;
use orml_traits::MultiCurrency;
use pallet_traits::{
	LiquidityPoolStorageProvider, RiskManagerStorageProvider, UserLiquidationAttemptsManager, UserStorageProvider,
};
use sp_runtime::{
	traits::{AccountIdConversion, One, Zero},
	FixedPointNumber,
};
use sp_std::prelude::*;

fn hypothetical_liquidity_setup(borrower: &AccountId, lender: &AccountId) -> Result<(), &'static str> {
	// set balance for users
	EnabledWrappedTokensId::get()
		.into_iter()
		.try_for_each(|token_id| -> Result<(), &'static str> {
			if token_id == MBTC {
				set_balance(token_id, borrower, 30_000 * DOLLARS)?;
			} else {
				set_balance(token_id, borrower, 10_000 * DOLLARS)?;
			}
			Ok(())
		})?;
	set_balance(MDOT, lender, 20_000 * DOLLARS)?;

	// set balance for Pools
	set_balance(DOT, &LiquidityPoolsPalletId::get().into_account(), 20_000 * DOLLARS)?;
	set_balance(BTC, &LiquidityPoolsPalletId::get().into_account(), 20_000 * DOLLARS)?;

	// enable pools as collateral
	EnabledUnderlyingAssetsIds::get()
		.into_iter()
		.try_for_each(|asset_id| -> Result<(), &'static str> {
			enable_is_collateral_mock::<Runtime>(Origin::signed(borrower.clone()), asset_id)?;
			// set borrow params
			LiquidityPools::set_pool_borrow_underlying(asset_id, 10_000 * DOLLARS);
			LiquidityPools::set_user_borrow_and_interest_index(borrower, asset_id, 10_000 * DOLLARS, Rate::one());
			Ok(())
		})?;
	Ok(())
}

runtime_benchmarks! {
	{ Runtime, minterest_protocol }

	create_pool {
		RiskManager::remove_pool(DOT);
		LiquidityPools::remove_pool_data(DOT);
		liquidation_pools::LiquidationPoolsData::<Runtime>::remove(DOT);
		controller::ControllerParams::<Runtime>::remove(DOT);
		minterest_model::MinterestModelParams::<Runtime>::remove(DOT);
	}: _(
		RawOrigin::Root,
		DOT,
		PoolInitData {
			kink: Rate::saturating_from_rational(2, 3),
			base_rate_per_block: Rate::saturating_from_rational(1, 3),
			multiplier_per_block: Rate::saturating_from_rational(2, 4),
			jump_multiplier_per_block: Rate::saturating_from_rational(1, 2),
			protocol_interest_factor: Rate::saturating_from_rational(1, 10),
			max_borrow_rate: Rate::saturating_from_rational(5, 1000),
			collateral_factor: Rate::saturating_from_rational(9, 10),
			protocol_interest_threshold: 100_000,
			deviation_threshold: Rate::saturating_from_rational(5, 100),
			balance_ratio: Rate::saturating_from_rational(2, 10),
			liquidation_threshold: Rate::saturating_from_rational(3, 100),
			liquidation_fee: Rate::saturating_from_rational(5, 100),
		}
	)

	deposit_underlying {
		prepare_for_mnt_distribution(vec![DOT])?;
		let lender: AccountId = account("lender", 0, SEED);
		Whitelist::add_member(RawOrigin::Root.into(), lender.clone())?;

		// set balance for lender
		set_balance(DOT, &lender, 50_000 * DOLLARS)?;

		// Set liquidation_attempts grater than zero to reset them.
		RiskManager::mutate_attemps(Some(DOT), &lender, Operation::Repay);

		System::set_block_number(10);

		MinterestProtocol::deposit_underlying(RawOrigin::Signed(lender.clone()).into(), DOT, 10_000 * DOLLARS)?;

		System::set_block_number(20);

	}: _(RawOrigin::Signed(lender.clone()), DOT, 10_000 * DOLLARS)
	verify {
		assert_eq!(Currencies::free_balance(DOT, &LiquidityPoolsPalletId::get().into_account() ), 60_000 * DOLLARS);
		// mnt_balance = 2(speed) * 10(delta_blocks) * 10(lender_supply) / 60(total_supply) = 3.33 MNT
		assert_eq!(Currencies::free_balance(MNT, &lender), 3_333_333_324_333_330_029)
	}

	redeem {
		prepare_for_mnt_distribution(vec![DOT])?;
		let borrower: AccountId = account("borrower", 0, SEED);
		let lender: AccountId = account("lender", 0, SEED);

		Whitelist::add_member(RawOrigin::Root.into(), borrower.clone())?;
		hypothetical_liquidity_setup(&borrower, &lender)?;

		System::set_block_number(10);

	}: _(RawOrigin::Signed(borrower.clone()), DOT)
	verify {
		assert_eq!(Currencies::free_balance(DOT, &borrower), 8_750_000_014_464_285_710_000);
		// mnt_balance = 2(speed) * 10(delta_blocks) * 10(borrower_supply) / 80(total_supply) = 2.5 MNT
		assert_eq!(Currencies::free_balance(MNT, &borrower), 2_500_000_000_000_000_000)
	}

	redeem_underlying {
		prepare_for_mnt_distribution(vec![DOT])?;
		let borrower: AccountId = account("borrower", 0, SEED);
		let lender: AccountId = account("lender", 0, SEED);

		Whitelist::add_member(RawOrigin::Root.into(), borrower.clone())?;
		hypothetical_liquidity_setup(&borrower, &lender)?;

		System::set_block_number(10);

	}: _(RawOrigin::Signed(borrower.clone()), DOT, 1_000 * DOLLARS)
	verify {
		assert_eq!(Currencies::free_balance(DOT, &borrower), 1_000 * DOLLARS);
		// mnt_balance = 2(speed) * 10(delta_blocks) * 10(borrower_supply) / 80(total_supply) = 2.5 MNT
		assert_eq!(Currencies::free_balance(MNT, &borrower), 2_500_000_000_000_000_000)
	}

	redeem_wrapped {
		prepare_for_mnt_distribution(vec![DOT])?;
		let borrower: AccountId = account("borrower", 0, SEED);
		let lender: AccountId = account("lender", 0, SEED);

		Whitelist::add_member(RawOrigin::Root.into(), borrower.clone())?;
		hypothetical_liquidity_setup(&borrower, &lender)?;

		System::set_block_number(10);

	}: _(RawOrigin::Signed(borrower.clone()), MDOT, 10_000 * DOLLARS)
	verify {
		assert_eq!(Currencies::free_balance(DOT, &borrower), 8_750_000_014_464_285_710_000);
		// mnt_balance = 2(speed) * 10(delta_blocks) * 10(borrower_supply) / 80(total_supply) = 2.5 MNT
		assert_eq!(Currencies::free_balance(MNT, &borrower), 2_500_000_000_000_000_000)
	}

	borrow {
		prepare_for_mnt_distribution(vec![DOT])?;
		let borrower: AccountId = account("borrower", 0, SEED);
		let lender: AccountId = account("lender", 0, SEED);

		Whitelist::add_member(RawOrigin::Root.into(), borrower.clone())?;
		hypothetical_liquidity_setup(&borrower, &lender)?;

		MinterestProtocol::borrow(RawOrigin::Signed(borrower.clone()).into(), DOT, 5_000 * DOLLARS)?;

		System::set_block_number(10);

	}: _(RawOrigin::Signed(borrower.clone()), DOT, 5_000 * DOLLARS)
	verify {
		assert_eq!(Currencies::free_balance(DOT, &borrower ), 10_000 * DOLLARS);
		assert_eq!(Currencies::free_balance(MNT, &borrower), 19_999_999_999_999_995_000)
	}

	repay {
		prepare_for_mnt_distribution(vec![DOT])?;
		let borrower: AccountId = account("borrower", 0, SEED);
		Whitelist::add_member(RawOrigin::Root.into(), borrower.clone())?;
		set_balance(DOT, &borrower, 100_000 * DOLLARS)?;
		MinterestProtocol::deposit_underlying(RawOrigin::Signed(borrower.clone()).into(), DOT, 50_000 * DOLLARS)?;

		System::set_block_number(10);

		MinterestProtocol::enable_is_collateral(Origin::signed(borrower.clone()).into(), DOT)?;
		MinterestProtocol::borrow(RawOrigin::Signed(borrower.clone()).into(), DOT, 10_000 * DOLLARS)?;

		System::set_block_number(20);

	}: _(RawOrigin::Signed(borrower.clone()), DOT, 10_000 * DOLLARS)
	verify {
		assert_eq!(LiquidityPools::pool_user_data(DOT, borrower.clone()).borrowed, 180_000_000_600_000);
		assert_eq!(Currencies::free_balance(MNT, &borrower), 9_999_999_954_999_990_405)
	}

	repay_all {
		prepare_for_mnt_distribution(vec![DOT])?;
		let borrower:AccountId = account("borrower", 0, SEED);
		Whitelist::add_member(RawOrigin::Root.into(), borrower.clone())?;
		set_balance(DOT, &borrower, 100_000 * DOLLARS)?;
		MinterestProtocol::deposit_underlying(RawOrigin::Signed(borrower.clone()).into(), DOT, 50_000 * DOLLARS)?;

		System::set_block_number(10);

		MinterestProtocol::enable_is_collateral(Origin::signed(borrower.clone()).into(), DOT)?;
		MinterestProtocol::borrow(RawOrigin::Signed(borrower.clone()).into(), DOT, 10_000 * DOLLARS)?;

		System::set_block_number(20);

	}: _(RawOrigin::Signed(borrower.clone()), DOT)
	verify {
		assert_eq!(LiquidityPools::pool_user_data(DOT, borrower.clone()).borrowed, Balance::zero());
		assert_eq!(Currencies::free_balance(MNT, &borrower), 9_999_999_954_999_990_405)
	}

	repay_on_behalf {
		prepare_for_mnt_distribution(vec![DOT])?;
		let borrower: AccountId = account("borrower", 0, SEED);
		let lender: AccountId = account("lender", 0, SEED);
		Whitelist::add_member(RawOrigin::Root.into(), lender.clone())?;
		Whitelist::add_member(RawOrigin::Root.into(), borrower.clone())?;
		set_balance(DOT, &lender, 100_000 * DOLLARS)?;
		set_balance(DOT, &borrower, 100_000 * DOLLARS)?;
		MinterestProtocol::deposit_underlying(RawOrigin::Signed(borrower.clone()).into(), DOT, 50_000 * DOLLARS)?;

		System::set_block_number(10);

		MinterestProtocol::enable_is_collateral(Origin::signed(borrower.clone()).into(), DOT)?;
		MinterestProtocol::borrow(RawOrigin::Signed(borrower.clone()).into(), DOT, 10_000 * DOLLARS)?;

		System::set_block_number(20);

	}: _(RawOrigin::Signed(lender.clone()), DOT, borrower.clone(), 10_000 * DOLLARS)
	verify {
		assert_eq!(LiquidityPools::pool_user_data(DOT, borrower.clone()).borrowed, 180_000_000_600_000);
		assert_eq!(Currencies::free_balance(MNT, &borrower), 9_999_999_954_999_990_405);
		assert_eq!(Currencies::free_balance(MNT, &lender), Balance::zero());
	}

	transfer_wrapped {
		prepare_for_mnt_distribution(vec![DOT])?;
		let borrower: AccountId = account("borrower", 0, SEED);
		let lender: AccountId = account("lender", 0, SEED);

		System::set_block_number(10);

		Whitelist::add_member(RawOrigin::Root.into(), borrower.clone())?;
		hypothetical_liquidity_setup(&borrower, &lender)?;

		System::set_block_number(20);

	}: _(RawOrigin::Signed(borrower.clone()), lender.clone(), MDOT, 10_000 * DOLLARS)
	verify  {
		assert_eq!(Currencies::free_balance(MDOT, &borrower.clone()), Balance::zero());
		assert_eq!(Currencies::free_balance(MDOT, &lender.clone()), 30_000 * DOLLARS);
		assert_eq!(Currencies::free_balance(MNT, &borrower), 5_000_000_000_000_000_000);
		assert_eq!(Currencies::free_balance(MNT, &lender), 10_000_000_000_000_000_000);
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
		Whitelist::add_member(RawOrigin::Root.into(), borrower.clone())?;

		set_balance(
			MNT,
			&MntTokenPalletId::get().into_account(),
			1_000_000 * DOLLARS,
		)?;

		EnabledUnderlyingAssetsIds::get()
			.into_iter()
			.try_for_each(|pool_id| -> Result<(), &'static str> {
				LiquidityPools::set_pool_data(pool_id, Pool {
					borrowed: Balance::zero(),
					borrow_index: Rate::one(),
					protocol_interest: Balance::zero(),
				});
				set_balance(pool_id, &lender, 100_000 * DOLLARS)?;
				MinterestProtocol::deposit_underlying(RawOrigin::Signed(lender.clone()).into(), pool_id, 100_000 * DOLLARS)?;
				MinterestProtocol::enable_is_collateral(Origin::signed(lender.clone()).into(), pool_id)?;
				MinterestProtocol::borrow(RawOrigin::Signed(lender.clone()).into(), pool_id, 50_000 * DOLLARS)?;
				Ok(())
			})?;

		System::set_block_number(50);

		EnabledUnderlyingAssetsIds::get()
			.into_iter()
			.try_for_each(|pool_id| -> Result<(), &'static str> {
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
		supply_balance = 0 + (2 * 50 * 0.5) * 4 = 200 MNT
		Borrow: prev + speed_pool * block_delta * borrower_borrow / total_borrow
		borrow_balance = 0 + (2 * 50 * 0.5) * 4 = 200 MNT
		accrued MNT tokens: 200 + 200 = ~400_000 MNT
		 */
		assert_eq!(Currencies::free_balance(MNT, &borrower), 399_999_967_375_002_687_652)
	}

}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::benchmarking::utils::tests::test_externalities;
	use frame_support::assert_ok;

	#[test]
	fn test_create_pool() {
		test_externalities().execute_with(|| {
			assert_ok!(test_benchmark_create_pool());
		})
	}

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
