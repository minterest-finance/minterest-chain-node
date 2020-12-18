//! Tests for the minterest-protocol module.

use super::*;
use mock::*;

use frame_support::{assert_noop, assert_ok};

#[test]
fn deposit_underlying_should_work() {
	new_test_ext().execute_with(|| {
		assert_noop!(
			MinterestProtocol::deposit_underlying(Origin::signed(ALICE), CurrencyId::ETH, 10),
			Error::<Test>::NotEnoughLiquidityAvailable
		);
		assert_noop!(
			MinterestProtocol::deposit_underlying(Origin::signed(ALICE), CurrencyId::MDOT, 10),
			Error::<Test>::NotValidUnderlyingAssetId
		);

		assert_ok!(MinterestProtocol::deposit_underlying(
			Origin::signed(ALICE),
			CurrencyId::DOT,
			60
		));
		assert_eq!(TestPools::get_reserve_available_liquidity(CurrencyId::DOT), 60);
		assert_eq!(TestMTokens::free_balance(CurrencyId::DOT, &ALICE), 40);
		assert_eq!(TestMTokens::free_balance(CurrencyId::MDOT, &ALICE), 75);

		assert_noop!(
			MinterestProtocol::deposit_underlying(Origin::signed(ALICE), CurrencyId::DOT, 50),
			Error::<Test>::NotEnoughLiquidityAvailable
		);
		assert_noop!(
			MinterestProtocol::deposit_underlying(Origin::signed(ALICE), CurrencyId::MDOT, 100),
			Error::<Test>::NotValidUnderlyingAssetId
		);

		assert_ok!(MinterestProtocol::deposit_underlying(
			Origin::signed(ALICE),
			CurrencyId::DOT,
			30
		));
		assert_eq!(TestPools::get_reserve_available_liquidity(CurrencyId::DOT), 90);
		assert_eq!(TestMTokens::free_balance(CurrencyId::DOT, &ALICE), 10);
		assert_eq!(TestMTokens::free_balance(CurrencyId::MDOT, &ALICE), 112);
	});
}

#[test]
fn redeem_underlying_should_work() {
	new_test_ext().execute_with(|| {
		assert_ok!(MinterestProtocol::deposit_underlying(
			Origin::signed(ALICE),
			CurrencyId::DOT,
			60
		));
		assert_eq!(TestPools::get_reserve_available_liquidity(CurrencyId::DOT), 60);
		assert_eq!(TestMTokens::free_balance(CurrencyId::DOT, &ALICE), 40);
		assert_eq!(TestMTokens::free_balance(CurrencyId::MDOT, &ALICE), 75);

		assert_noop!(
			MinterestProtocol::redeem_underlying(Origin::signed(ALICE), CurrencyId::DOT, 100),
			Error::<Test>::NotEnoughLiquidityAvailable
		);

		assert_ok!(MinterestProtocol::redeem_underlying(
			Origin::signed(ALICE),
			CurrencyId::DOT,
			30
		));
		assert_eq!(TestPools::get_reserve_available_liquidity(CurrencyId::DOT), 30);
		assert_eq!(TestMTokens::free_balance(CurrencyId::DOT, &ALICE), 70);
		assert_eq!(TestMTokens::free_balance(CurrencyId::MDOT, &ALICE), 38);
	});
}

#[test]
fn getting_assets_from_reserve_by_different_users_should_work() {
	new_test_ext().execute_with(|| {
		assert_ok!(MinterestProtocol::deposit_underlying(
			Origin::signed(ALICE),
			CurrencyId::DOT,
			60
		));
		assert_eq!(TestPools::get_reserve_available_liquidity(CurrencyId::DOT), 60);
		assert_eq!(TestMTokens::free_balance(CurrencyId::DOT, &ALICE), 40);
		assert_eq!(TestMTokens::free_balance(CurrencyId::MDOT, &ALICE), 75);

		assert_noop!(
			MinterestProtocol::redeem_underlying(Origin::signed(BOB), CurrencyId::DOT, 30),
			Error::<Test>::NotEnoughWrappedTokens
		);

		assert_ok!(MinterestProtocol::deposit_underlying(
			Origin::signed(BOB),
			CurrencyId::DOT,
			7
		));
		assert_eq!(TestPools::get_reserve_available_liquidity(CurrencyId::DOT), 67);
		assert_eq!(TestMTokens::free_balance(CurrencyId::DOT, &BOB), 93);
		assert_eq!(TestMTokens::free_balance(CurrencyId::MDOT, &BOB), 8);
	});
}

// TODO Implement tests for redeem and redeem_underlying functions
