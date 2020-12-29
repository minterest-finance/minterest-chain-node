//! Tests for the minterest-protocol module.

use super::*;
use mock::*;

use frame_support::{assert_noop, assert_ok};

#[test]
fn deposit_underlying_should_work() {
	new_test_ext().execute_with(|| {
		assert_ok!(LiquidityPools::<Test>::unlock_pool_transactions(
			Origin::root(),
			CurrencyId::DOT
		));
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
		assert_eq!(TestPools::get_pool_available_liquidity(CurrencyId::DOT), 60);
		assert_eq!(Currencies::free_balance(CurrencyId::DOT, &ALICE), 40);
		assert_eq!(Currencies::free_balance(CurrencyId::MDOT, &ALICE), 60);

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
		assert_eq!(TestPools::get_pool_available_liquidity(CurrencyId::DOT), 90);
		assert_eq!(Currencies::free_balance(CurrencyId::DOT, &ALICE), 10);
		assert_eq!(Currencies::free_balance(CurrencyId::MDOT, &ALICE), 90);
	});
}

#[test]
fn redeem_underlying_should_work() {
	new_test_ext().execute_with(|| {
		assert_ok!(LiquidityPools::<Test>::unlock_pool_transactions(
			Origin::root(),
			CurrencyId::DOT
		));
		assert_ok!(MinterestProtocol::deposit_underlying(
			Origin::signed(ALICE),
			CurrencyId::DOT,
			60
		));
		assert_eq!(TestPools::get_pool_available_liquidity(CurrencyId::DOT), 60);
		assert_eq!(Currencies::free_balance(CurrencyId::DOT, &ALICE), 40);
		assert_eq!(Currencies::free_balance(CurrencyId::MDOT, &ALICE), 60);

		assert_noop!(
			MinterestProtocol::redeem_underlying(Origin::signed(ALICE), CurrencyId::DOT, 100),
			Error::<Test>::NotEnoughLiquidityAvailable
		);
		assert_noop!(
			MinterestProtocol::redeem_underlying(Origin::signed(ALICE), CurrencyId::MDOT, 20),
			Error::<Test>::NotValidUnderlyingAssetId
		);

		assert_ok!(MinterestProtocol::redeem_underlying(
			Origin::signed(ALICE),
			CurrencyId::DOT,
			30
		));
		assert_eq!(TestPools::get_pool_available_liquidity(CurrencyId::DOT), 30);
		assert_eq!(Currencies::free_balance(CurrencyId::DOT, &ALICE), 70);
		assert_eq!(Currencies::free_balance(CurrencyId::MDOT, &ALICE), 30);
	});
}

#[test]
fn redeem_should_work() {
	new_test_ext().execute_with(|| {
		assert_ok!(LiquidityPools::<Test>::unlock_pool_transactions(
			Origin::root(),
			CurrencyId::DOT
		));
		assert_ok!(MinterestProtocol::deposit_underlying(
			Origin::signed(ALICE),
			CurrencyId::DOT,
			60
		));
		assert_eq!(TestPools::get_pool_available_liquidity(CurrencyId::DOT), 60);
		assert_eq!(Currencies::free_balance(CurrencyId::DOT, &ALICE), 40);
		assert_eq!(Currencies::free_balance(CurrencyId::MDOT, &ALICE), 60);

		assert_ok!(MinterestProtocol::redeem(Origin::signed(ALICE), CurrencyId::DOT));

		assert_ok!(MinterestProtocol::deposit_underlying(
			Origin::signed(ALICE),
			CurrencyId::DOT,
			60
		));
		assert_noop!(
			MinterestProtocol::redeem_underlying(Origin::signed(BOB), CurrencyId::DOT, 30),
			Error::<Test>::NotEnoughWrappedTokens
		);

		assert_noop!(
			MinterestProtocol::redeem_underlying(Origin::signed(ALICE), CurrencyId::MDOT, 20),
			Error::<Test>::NotValidUnderlyingAssetId
		);
	});
}

#[test]
fn redeem_wrapped_should_work() {
	new_test_ext().execute_with(|| {
		assert_ok!(LiquidityPools::<Test>::unlock_pool_transactions(
			Origin::root(),
			CurrencyId::DOT
		));
		assert_ok!(MinterestProtocol::deposit_underlying(
			Origin::signed(ALICE),
			CurrencyId::DOT,
			60
		));
		assert_eq!(TestPools::get_pool_available_liquidity(CurrencyId::DOT), 60);
		assert_eq!(Currencies::free_balance(CurrencyId::DOT, &ALICE), 40);
		assert_eq!(Currencies::free_balance(CurrencyId::MDOT, &ALICE), 60);

		assert_ok!(MinterestProtocol::redeem_wrapped(
			Origin::signed(ALICE),
			CurrencyId::MDOT,
			35
		));
		assert_eq!(TestPools::get_pool_available_liquidity(CurrencyId::DOT), 25);
		assert_eq!(Currencies::free_balance(CurrencyId::DOT, &ALICE), 75);
		assert_eq!(Currencies::free_balance(CurrencyId::MDOT, &ALICE), 25);

		assert_noop!(
			MinterestProtocol::redeem_wrapped(Origin::signed(ALICE), CurrencyId::MDOT, 60),
			Error::<Test>::NotEnoughWrappedTokens
		);
		assert_noop!(
			MinterestProtocol::redeem_wrapped(Origin::signed(ALICE), CurrencyId::DOT, 20),
			Error::<Test>::NotValidWrappedTokenId
		);
	});
}

#[test]
fn getting_assets_from_pool_by_different_users_should_work() {
	new_test_ext().execute_with(|| {
		assert_ok!(LiquidityPools::<Test>::unlock_pool_transactions(
			Origin::root(),
			CurrencyId::DOT
		));
		assert_ok!(MinterestProtocol::deposit_underlying(
			Origin::signed(ALICE),
			CurrencyId::DOT,
			60
		));
		assert_eq!(TestPools::get_pool_available_liquidity(CurrencyId::DOT), 60);
		assert_eq!(Currencies::free_balance(CurrencyId::DOT, &ALICE), 40);
		assert_eq!(Currencies::free_balance(CurrencyId::MDOT, &ALICE), 60);

		assert_noop!(
			MinterestProtocol::redeem_underlying(Origin::signed(BOB), CurrencyId::DOT, 30),
			Error::<Test>::NotEnoughWrappedTokens
		);

		assert_ok!(MinterestProtocol::deposit_underlying(
			Origin::signed(BOB),
			CurrencyId::DOT,
			7
		));
		assert_eq!(TestPools::get_pool_available_liquidity(CurrencyId::DOT), 67);
		assert_eq!(Currencies::free_balance(CurrencyId::DOT, &BOB), 93);
		assert_eq!(Currencies::free_balance(CurrencyId::MDOT, &BOB), 7);
	});
}

#[test]
fn borrow_should_work() {
	new_test_ext().execute_with(|| {
		assert_ok!(LiquidityPools::<Test>::unlock_pool_transactions(
			Origin::root(),
			CurrencyId::DOT
		));
		assert_ok!(MinterestProtocol::deposit_underlying(
			Origin::signed(ALICE),
			CurrencyId::DOT,
			60
		));
		assert_eq!(TestPools::get_pool_available_liquidity(CurrencyId::DOT), 60);
		assert_eq!(Currencies::free_balance(CurrencyId::DOT, &ALICE), 40);
		assert_eq!(Currencies::free_balance(CurrencyId::MDOT, &ALICE), 60);

		assert_noop!(
			MinterestProtocol::borrow(Origin::signed(ALICE), CurrencyId::DOT, 100),
			Error::<Test>::NotEnoughLiquidityAvailable
		);
		assert_noop!(
			MinterestProtocol::borrow(Origin::signed(ALICE), CurrencyId::MDOT, 60),
			Error::<Test>::NotValidUnderlyingAssetId
		);

		assert_ok!(MinterestProtocol::borrow(Origin::signed(ALICE), CurrencyId::DOT, 30));
		assert_eq!(TestPools::get_pool_available_liquidity(CurrencyId::DOT), 30);
		assert_eq!(Currencies::free_balance(CurrencyId::DOT, &ALICE), 70);
		assert_eq!(Currencies::free_balance(CurrencyId::MDOT, &ALICE), 60);
		assert_eq!(TestPools::get_pool_total_borrowed(CurrencyId::DOT), 30);
		assert_eq!(TestPools::get_user_total_borrowed(&ALICE, CurrencyId::DOT), 30);

		// pool_available_liquidity (DOT) = 30
		// Admin depositing to the insurance 10 DOT, now pool_available_liquidity = 30 + 10 = 40 DOT
		assert_ok!(TestPools::deposit_insurance(Origin::signed(ADMIN), CurrencyId::DOT, 10));
		assert_eq!(TestPools::get_pool_available_liquidity(CurrencyId::DOT), 40);
		assert_eq!(Currencies::free_balance(CurrencyId::DOT, &ADMIN), 90);
		assert_eq!(Currencies::free_balance(CurrencyId::MDOT, &ADMIN), 0);
		assert_eq!(TestPools::get_pool_total_insurance(CurrencyId::DOT), 10);

		// Bob can borrow 35 DOT.
		assert_ok!(MinterestProtocol::borrow(Origin::signed(BOB), CurrencyId::DOT, 35));
		assert_eq!(TestPools::get_pool_available_liquidity(CurrencyId::DOT), 5);
		// FIXME 5 DOT's must remain on the insurance balance
		assert_eq!(TestPools::get_pool_total_insurance(CurrencyId::DOT), 10);
	});
}

#[test]
fn repay_should_work() {
	new_test_ext().execute_with(|| {
		assert_ok!(LiquidityPools::<Test>::unlock_pool_transactions(
			Origin::root(),
			CurrencyId::DOT
		));
		assert_ok!(MinterestProtocol::deposit_underlying(
			Origin::signed(ALICE),
			CurrencyId::DOT,
			60
		));
		assert_eq!(TestPools::get_pool_available_liquidity(CurrencyId::DOT), 60);
		assert_eq!(Currencies::free_balance(CurrencyId::DOT, &ALICE), 40);
		assert_eq!(Currencies::free_balance(CurrencyId::MDOT, &ALICE), 60);

		assert_ok!(MinterestProtocol::borrow(Origin::signed(ALICE), CurrencyId::DOT, 30));
		assert_eq!(TestPools::get_pool_available_liquidity(CurrencyId::DOT), 30);
		assert_eq!(Currencies::free_balance(CurrencyId::DOT, &ALICE), 70);
		assert_eq!(Currencies::free_balance(CurrencyId::MDOT, &ALICE), 60);
		assert_eq!(TestPools::get_pool_total_borrowed(CurrencyId::DOT), 30);
		assert_eq!(TestPools::get_user_total_borrowed(&ALICE, CurrencyId::DOT), 30);

		assert_noop!(
			MinterestProtocol::repay(Origin::signed(ALICE), CurrencyId::MDOT, 10),
			Error::<Test>::NotValidUnderlyingAssetId
		);
		assert_noop!(
			MinterestProtocol::repay(Origin::signed(ALICE), CurrencyId::DOT, 100),
			Error::<Test>::NotEnoughUnderlyingsAssets
		);

		assert_ok!(MinterestProtocol::repay(Origin::signed(ALICE), CurrencyId::DOT, 20));
		assert_eq!(TestPools::get_pool_available_liquidity(CurrencyId::DOT), 50);
		assert_eq!(Currencies::free_balance(CurrencyId::DOT, &ALICE), 50);
		assert_eq!(Currencies::free_balance(CurrencyId::MDOT, &ALICE), 60);
		assert_eq!(TestPools::get_pool_total_borrowed(CurrencyId::DOT), 10);
		assert_eq!(TestPools::get_user_total_borrowed(&ALICE, CurrencyId::DOT), 10);
	});
}
