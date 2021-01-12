//! Tests for the minterest-protocol pallet.

use super::*;
use mock::*;

use frame_support::{assert_noop, assert_ok};

fn dollars<T: Into<u128>>(d: T) -> Balance {
	DOLLARS.saturating_mul(d.into())
}

#[test]
fn deposit_underlying_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		// Alice deposit 60 DOT; exchange_rate = 1.0
		// wrapped_amount = 60.0 DOT / 1.0 = 60.0
		assert_ok!(TestProtocol::deposit_underlying(
			alice(),
			CurrencyId::DOT,
			dollars(60_u128)
		));
		let expected_event = TestEvent::minterest_protocol(RawEvent::Deposited(
			ALICE,
			CurrencyId::DOT,
			dollars(60_u128),
			CurrencyId::MDOT,
			dollars(60_u128),
		));
		assert!(System::events().iter().any(|record| record.event == expected_event));

		// MDOT pool does not exist.
		assert_noop!(
			TestProtocol::deposit_underlying(alice(), CurrencyId::MDOT, 10),
			Error::<Test>::NotValidUnderlyingAssetId
		);

		// Alice has 0 ETH on her account, so she cannot make a deposit.
		assert_noop!(
			TestProtocol::deposit_underlying(alice(), CurrencyId::ETH, dollars(10_u128)),
			Error::<Test>::NotEnoughLiquidityAvailable
		);

		// Transaction with zero balance is not allowed.
		assert_noop!(
			TestProtocol::deposit_underlying(alice(), CurrencyId::DOT, Balance::zero()),
			Error::<Test>::ZeroBalanceTransaction
		);
	});
}

#[test]
fn redeem_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		// Alice deposit 60 DOT; exchange_rate = 1.0
		assert_ok!(TestProtocol::deposit_underlying(
			alice(),
			CurrencyId::DOT,
			dollars(60_u128)
		));

		// Alice redeem all 60 MDOT; exchange_rate = 1.0
		assert_ok!(TestProtocol::redeem(alice(), CurrencyId::DOT));
		let expected_event = TestEvent::minterest_protocol(RawEvent::Redeemed(
			ALICE,
			CurrencyId::DOT,
			dollars(60_u128),
			CurrencyId::MDOT,
			dollars(60_u128),
		));
		assert!(System::events().iter().any(|record| record.event == expected_event));
	});
}

#[test]
fn redeem_should_not_work() {
	ExtBuilder::default().build().execute_with(|| {
		// Bob has 0 MDOT on her account, so she cannot make a redeem.
		assert_noop!(
			TestProtocol::redeem(bob(), CurrencyId::DOT),
			Error::<Test>::NumberOfWrappedTokensIsZero
		);

		// MDOT is wrong CurrencyId for underlying assets.
		assert_noop!(
			TestProtocol::redeem(alice(), CurrencyId::MDOT),
			Error::<Test>::NotValidUnderlyingAssetId
		);
	});
}

#[test]
fn redeem_fails_if_no_balance_in_pool() {
	ExtBuilder::default()
		.user_balance(ALICE, CurrencyId::MDOT, ONE_HUNDRED_DOLLARS)
		.build()
		.execute_with(|| {
			// assert_noop!(TestProtocol::redeem());
		});
}

// #[test]
// fn redeem_underlying_should_work() {
// 	new_test_ext().execute_with(|| {
// 		assert_ok!(TestProtocol::deposit_underlying(alice(), CurrencyId::DOT, 60));
// 		assert_eq!(TestPools::get_pool_available_liquidity(CurrencyId::DOT), 60);
// 		assert_eq!(Currencies::free_balance(CurrencyId::DOT, &ALICE), 40);
// 		assert_eq!(Currencies::free_balance(CurrencyId::MDOT, &ALICE), 60);
//
// 		assert_noop!(
// 			TestProtocol::redeem_underlying(alice(), CurrencyId::DOT, 100),
// 			Error::<Test>::NotEnoughLiquidityAvailable
// 		);
// 		assert_noop!(
// 			TestProtocol::redeem_underlying(alice(), CurrencyId::MDOT, 20),
// 			Error::<Test>::NotValidUnderlyingAssetId
// 		);
//
// 		assert_ok!(TestProtocol::redeem_underlying(alice(), CurrencyId::DOT, 30));
// 		assert_eq!(TestPools::get_pool_available_liquidity(CurrencyId::DOT), 30);
// 		assert_eq!(Currencies::free_balance(CurrencyId::DOT, &ALICE), 70);
// 		assert_eq!(Currencies::free_balance(CurrencyId::MDOT, &ALICE), 30);
// 	});
// }
//
// #[test]
// fn redeem_wrapped_should_work() {
// 	new_test_ext().execute_with(|| {
// 		assert_ok!(TestProtocol::deposit_underlying(alice(), CurrencyId::DOT, 60));
// 		assert_eq!(TestPools::get_pool_available_liquidity(CurrencyId::DOT), 60);
// 		assert_eq!(Currencies::free_balance(CurrencyId::DOT, &ALICE), 40);
// 		assert_eq!(Currencies::free_balance(CurrencyId::MDOT, &ALICE), 60);
//
// 		assert_ok!(TestProtocol::redeem_wrapped(alice(), CurrencyId::MDOT, 35));
// 		assert_eq!(TestPools::get_pool_available_liquidity(CurrencyId::DOT), 25);
// 		assert_eq!(Currencies::free_balance(CurrencyId::DOT, &ALICE), 75);
// 		assert_eq!(Currencies::free_balance(CurrencyId::MDOT, &ALICE), 25);
//
// 		assert_noop!(
// 			TestProtocol::redeem_wrapped(alice(), CurrencyId::MDOT, 60),
// 			Error::<Test>::NotEnoughWrappedTokens
// 		);
// 		assert_noop!(
// 			TestProtocol::redeem_wrapped(alice(), CurrencyId::DOT, 20),
// 			Error::<Test>::NotValidWrappedTokenId
// 		);
// 	});
// }
//
// #[test]
// fn getting_assets_from_pool_by_different_users_should_work() {
// 	new_test_ext().execute_with(|| {
// 		assert_ok!(TestProtocol::deposit_underlying(alice(), CurrencyId::DOT, 60));
// 		assert_eq!(TestPools::get_pool_available_liquidity(CurrencyId::DOT), 60);
// 		assert_eq!(Currencies::free_balance(CurrencyId::DOT, &ALICE), 40);
// 		assert_eq!(Currencies::free_balance(CurrencyId::MDOT, &ALICE), 60);
//
// 		assert_noop!(
// 			TestProtocol::redeem_underlying(bob(), CurrencyId::DOT, 30),
// 			Error::<Test>::NotEnoughWrappedTokens
// 		);
//
// 		assert_ok!(TestProtocol::deposit_underlying(bob(), CurrencyId::DOT, 7));
// 		assert_eq!(TestPools::get_pool_available_liquidity(CurrencyId::DOT), 67);
// 		assert_eq!(Currencies::free_balance(CurrencyId::DOT, &BOB), 93);
// 		assert_eq!(Currencies::free_balance(CurrencyId::MDOT, &BOB), 7);
// 	});
// }
//
// #[test]
// fn borrow_should_work() {
// 	new_test_ext().execute_with(|| {
// 		assert_ok!(TestProtocol::deposit_underlying(alice(), CurrencyId::DOT, 60));
// 		assert_eq!(TestPools::get_pool_available_liquidity(CurrencyId::DOT), 60);
// 		assert_eq!(Currencies::free_balance(CurrencyId::DOT, &ALICE), 40);
// 		assert_eq!(Currencies::free_balance(CurrencyId::MDOT, &ALICE), 60);
//
// 		assert_noop!(
// 			TestProtocol::borrow(alice(), CurrencyId::DOT, 100),
// 			Error::<Test>::NotEnoughLiquidityAvailable
// 		);
// 		assert_noop!(
// 			TestProtocol::borrow(alice(), CurrencyId::MDOT, 60),
// 			Error::<Test>::NotValidUnderlyingAssetId
// 		);
//
// 		assert_ok!(TestProtocol::borrow(alice(), CurrencyId::DOT, 30));
// 		assert_eq!(TestPools::get_pool_available_liquidity(CurrencyId::DOT), 30);
// 		assert_eq!(Currencies::free_balance(CurrencyId::DOT, &ALICE), 70);
// 		assert_eq!(Currencies::free_balance(CurrencyId::MDOT, &ALICE), 60);
// 		assert_eq!(TestPools::get_pool_total_borrowed(CurrencyId::DOT), 30);
// 		assert_eq!(TestPools::get_user_total_borrowed(&ALICE, CurrencyId::DOT), 30);
//
// 		// pool_available_liquidity (DOT) = 30
// 		// Admin depositing to the insurance 10 DOT, now pool_available_liquidity = 30 + 10 = 40 DOT
// 		assert_ok!(TestAccounts::add_member(Origin::root(), ADMIN));
// 		assert_ok!(TestController::deposit_insurance(admin(), CurrencyId::DOT, 10));
// 		assert_eq!(TestPools::get_pool_available_liquidity(CurrencyId::DOT), 40);
// 		assert_eq!(Currencies::free_balance(CurrencyId::DOT, &ADMIN), 90);
// 		assert_eq!(Currencies::free_balance(CurrencyId::MDOT, &ADMIN), 0);
// 		assert_eq!(TestPools::get_pool_total_insurance(CurrencyId::DOT), 10);
//
// 		// Bob can't borrow 35 DOT.
// 		assert_noop!(
// 			TestProtocol::borrow(bob(), CurrencyId::DOT, 35),
// 			Error::<Test>::BorrowControllerRejection
// 		);
// 	});
// }
//
// #[test]
// fn repay_should_work() {
// 	new_test_ext().execute_with(|| {
// 		assert_ok!(TestProtocol::deposit_underlying(alice(), CurrencyId::DOT, 60));
// 		assert_eq!(TestPools::get_pool_available_liquidity(CurrencyId::DOT), 60);
// 		assert_eq!(Currencies::free_balance(CurrencyId::DOT, &ALICE), 40);
// 		assert_eq!(Currencies::free_balance(CurrencyId::MDOT, &ALICE), 60);
//
// 		assert_ok!(TestProtocol::borrow(alice(), CurrencyId::DOT, 30));
// 		assert_eq!(TestPools::get_pool_available_liquidity(CurrencyId::DOT), 30);
// 		assert_eq!(Currencies::free_balance(CurrencyId::DOT, &ALICE), 70);
// 		assert_eq!(Currencies::free_balance(CurrencyId::MDOT, &ALICE), 60);
// 		assert_eq!(TestPools::get_pool_total_borrowed(CurrencyId::DOT), 30);
// 		assert_eq!(TestPools::get_user_total_borrowed(&ALICE, CurrencyId::DOT), 30);
//
// 		assert_noop!(
// 			TestProtocol::repay(alice(), CurrencyId::MDOT, 10),
// 			Error::<Test>::NotValidUnderlyingAssetId
// 		);
// 		assert_noop!(
// 			TestProtocol::repay(alice(), CurrencyId::DOT, 100),
// 			Error::<Test>::NotEnoughUnderlyingsAssets
// 		);
//
// 		assert_ok!(TestProtocol::repay(alice(), CurrencyId::DOT, 20));
// 		assert_eq!(TestPools::get_pool_available_liquidity(CurrencyId::DOT), 50);
// 		assert_eq!(Currencies::free_balance(CurrencyId::DOT, &ALICE), 50);
// 		assert_eq!(Currencies::free_balance(CurrencyId::MDOT, &ALICE), 60);
// 		assert_eq!(TestPools::get_pool_total_borrowed(CurrencyId::DOT), 10);
// 		assert_eq!(TestPools::get_user_total_borrowed(&ALICE, CurrencyId::DOT), 10);
// 	});
// }
//
// #[test]
// fn repay_on_behalf_should_work() {
// 	new_test_ext().execute_with(|| {
// 		assert_ok!(TestProtocol::deposit_underlying(alice(), CurrencyId::DOT, 60));
// 		assert_eq!(TestPools::get_pool_available_liquidity(CurrencyId::DOT), 60);
// 		assert_eq!(Currencies::free_balance(CurrencyId::DOT, &ALICE), 40);
// 		assert_eq!(Currencies::free_balance(CurrencyId::MDOT, &ALICE), 60);
// 		assert_eq!(Currencies::free_balance(CurrencyId::DOT, &BOB), 100);
//
// 		assert_ok!(TestProtocol::borrow(alice(), CurrencyId::DOT, 30));
// 		assert_eq!(TestPools::get_pool_available_liquidity(CurrencyId::DOT), 30);
// 		assert_eq!(Currencies::free_balance(CurrencyId::DOT, &ALICE), 70);
// 		assert_eq!(Currencies::free_balance(CurrencyId::MDOT, &ALICE), 60);
// 		assert_eq!(TestPools::get_pool_total_borrowed(CurrencyId::DOT), 30);
// 		assert_eq!(TestPools::get_user_total_borrowed(&ALICE, CurrencyId::DOT), 30);
//
// 		assert_noop!(
// 			TestProtocol::repay_on_behalf(bob(), CurrencyId::MDOT, ALICE, 10),
// 			Error::<Test>::NotValidUnderlyingAssetId
// 		);
// 		assert_noop!(
// 			TestProtocol::repay_on_behalf(bob(), CurrencyId::DOT, ALICE, 120),
// 			Error::<Test>::NotEnoughUnderlyingsAssets
// 		);
// 		assert_noop!(
// 			TestProtocol::repay_on_behalf(bob(), CurrencyId::DOT, BOB, 100),
// 			//FIXME: is it Ok to check internal error?
// 			Error::<Test>::InternalPoolError
// 		);
//
// 		assert_ok!(TestProtocol::repay_on_behalf(bob(), CurrencyId::DOT, ALICE, 20));
// 		assert_eq!(TestPools::get_pool_available_liquidity(CurrencyId::DOT), 50);
// 		assert_eq!(Currencies::free_balance(CurrencyId::DOT, &ALICE), 70);
// 		assert_eq!(Currencies::free_balance(CurrencyId::MDOT, &ALICE), 60);
// 		assert_eq!(Currencies::free_balance(CurrencyId::DOT, &BOB), 80);
// 		assert_eq!(TestPools::get_pool_total_borrowed(CurrencyId::DOT), 10);
// 		assert_eq!(TestPools::get_user_total_borrowed(&ALICE, CurrencyId::DOT), 10);
// 	});
// }
//
// #[test]
// fn enable_as_collateral_should_work() {
// 	new_test_ext().execute_with(|| {
// 		// Alice enable as collateral her DOT pool.
// 		assert_ok!(TestProtocol::enable_as_collateral(alice(), CurrencyId::DOT));
// 		let expected_event = TestEvent::minterest_protocol(RawEvent::PoolEnabledAsCollateral(ALICE, CurrencyId::DOT));
// 		assert!(System::events().iter().any(|record| record.event == expected_event));
// 		assert!(TestPools::check_user_available_collateral(&ALICE, CurrencyId::DOT));
//
// 		assert_noop!(
// 			TestProtocol::enable_as_collateral(alice(), CurrencyId::MDOT),
// 			Error::<Test>::PoolNotFound
// 		);
// 	});
// }
//
// #[test]
// fn disable_collateral_should_work() {
// 	new_test_ext().execute_with(|| {
// 		// Alice disable collateral her DOT pool.
// 		assert_ok!(TestProtocol::disable_collateral(alice(), CurrencyId::DOT));
// 		let expected_event = TestEvent::minterest_protocol(RawEvent::PoolDisabledCollateral(ALICE, CurrencyId::DOT));
// 		assert!(System::events().iter().any(|record| record.event == expected_event));
// 		assert!(!TestPools::check_user_available_collateral(&ALICE, CurrencyId::DOT));
//
// 		assert_noop!(
// 			TestProtocol::disable_collateral(alice(), CurrencyId::MDOT),
// 			Error::<Test>::PoolNotFound
// 		);
// 	});
// }
