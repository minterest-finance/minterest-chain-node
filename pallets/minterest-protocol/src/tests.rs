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

		// Alice has 100 ETH on her account, so she cannot make a deposit 150 ETH.
		assert_noop!(
			TestProtocol::deposit_underlying(alice(), CurrencyId::ETH, dollars(150_u128)),
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
fn redeem_fails_if_low_balance_in_pool() {
	ExtBuilder::default()
		.user_balance(ALICE, CurrencyId::BTC, TEN_THOUSAND_DOLLARS)
		.build()
		.execute_with(|| {
			// Alice deposited 10_000$ to BTC pool.
			assert_ok!(TestProtocol::deposit_underlying(
				alice(),
				CurrencyId::BTC,
				TEN_THOUSAND_DOLLARS
			));

			// Alice borrowed 100$ from BTC pool:
			// pool_total_liquidity = 10_000 - 100 = 9_900$
			assert_ok!(TestProtocol::borrow(alice(), CurrencyId::BTC, ONE_HUNDRED_DOLLARS));

			// Alice has 10_000 MBTC. exchange_rate = 1.0
			// Alice is trying to change all her 10_000 MBTC tokens to BTC. She can't do it because:
			// pool_total_liquidity = 9_900 < 10_000 * 1.0 = 10_000
			assert_noop!(
				TestProtocol::redeem(alice(), CurrencyId::BTC),
				Error::<Test>::NotEnoughLiquidityAvailable
			);
		});
}

#[test]
fn redeem_underlying_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		// Alice deposited 60 DOT to the pool.
		assert_ok!(TestProtocol::deposit_underlying(
			alice(),
			CurrencyId::DOT,
			dollars(60_u128)
		));

		// Alice can't redeem 100 DOT, because 100 DOT equal 100 * 1.0 = 100 MDOT
		// And she has 60 MDOT on her balance.
		assert_noop!(
			TestProtocol::redeem_underlying(alice(), CurrencyId::DOT, dollars(100_u128)),
			Error::<Test>::NotEnoughWrappedTokens
		);

		// MDOT is wrong CurrencyId for underlying assets.
		assert_noop!(
			TestProtocol::redeem_underlying(alice(), CurrencyId::MDOT, dollars(20_u128)),
			Error::<Test>::NotValidUnderlyingAssetId
		);

		// Transaction with zero balance is not allowed.
		assert_noop!(
			TestProtocol::redeem_underlying(alice(), CurrencyId::DOT, Balance::zero()),
			Error::<Test>::ZeroBalanceTransaction
		);

		assert_ok!(TestProtocol::redeem_underlying(
			alice(),
			CurrencyId::DOT,
			dollars(30_u128)
		));
		let expected_event = TestEvent::minterest_protocol(RawEvent::Redeemed(
			ALICE,
			CurrencyId::DOT,
			dollars(30_u128),
			CurrencyId::MDOT,
			dollars(30_u128),
		));
		assert!(System::events().iter().any(|record| record.event == expected_event));
	});
}

#[test]
fn redeem_underlying_fails_if_low_balance_in_pool() {
	ExtBuilder::default()
		.user_balance(ALICE, CurrencyId::BTC, TEN_THOUSAND_DOLLARS)
		.build()
		.execute_with(|| {
			// Alice deposited 10_000$ to BTC pool.
			assert_ok!(TestProtocol::deposit_underlying(
				alice(),
				CurrencyId::BTC,
				TEN_THOUSAND_DOLLARS
			));

			// Alice borrowed 100$ from BTC pool:
			// pool_total_liquidity = 10_000 - 100 = 9_900$
			assert_ok!(TestProtocol::borrow(alice(), CurrencyId::BTC, ONE_HUNDRED_DOLLARS));

			// Alice has 10_000 MBTC. exchange_rate = 1.0
			// Alice is trying to change all her 10_000 MBTC tokens to BTC. She can't do it because:
			// pool_total_liquidity = 9_900 BTC < 10_000 BTC
			assert_noop!(
				TestProtocol::redeem_underlying(alice(), CurrencyId::BTC, TEN_THOUSAND_DOLLARS),
				Error::<Test>::NotEnoughLiquidityAvailable
			);
		});
}

#[test]
fn redeem_wrapped_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		// Alice deposited 60 DOT to the pool.
		assert_ok!(TestProtocol::deposit_underlying(
			alice(),
			CurrencyId::DOT,
			dollars(60_u128)
		));

		// Alice has 60 MDOT. She can't redeem 100 MDOT.
		assert_noop!(
			TestProtocol::redeem_wrapped(alice(), CurrencyId::MDOT, dollars(100_u128)),
			Error::<Test>::NotEnoughWrappedTokens
		);

		// MDOT is wrong CurrencyId for underlying assets.
		assert_noop!(
			TestProtocol::redeem_wrapped(alice(), CurrencyId::DOT, dollars(20_u128)),
			Error::<Test>::NotValidWrappedTokenId
		);

		// Transaction with zero balance is not allowed.
		assert_noop!(
			TestProtocol::redeem_wrapped(alice(), CurrencyId::MDOT, Balance::zero()),
			Error::<Test>::ZeroBalanceTransaction
		);

		assert_ok!(TestProtocol::redeem_wrapped(
			alice(),
			CurrencyId::MDOT,
			dollars(35_u128)
		));
		let expected_event = TestEvent::minterest_protocol(RawEvent::Redeemed(
			ALICE,
			CurrencyId::DOT,
			dollars(35_u128),
			CurrencyId::MDOT,
			dollars(35_u128),
		));
		assert!(System::events().iter().any(|record| record.event == expected_event));
	});
}

#[test]
fn redeem_wrapped_fails_if_low_balance_in_pool() {
	ExtBuilder::default()
		.user_balance(ALICE, CurrencyId::BTC, TEN_THOUSAND_DOLLARS)
		.build()
		.execute_with(|| {
			// Alice deposited 10_000$ to BTC pool.
			assert_ok!(TestProtocol::deposit_underlying(
				alice(),
				CurrencyId::BTC,
				TEN_THOUSAND_DOLLARS
			));

			// Alice borrowed 100$ from BTC pool:
			// pool_total_liquidity = 10_000 - 100 = 9_900$
			assert_ok!(TestProtocol::borrow(alice(), CurrencyId::BTC, ONE_HUNDRED_DOLLARS));

			// Alice has 10_000 MBTC. exchange_rate = 1.0
			// Alice is trying to change all her 10_000 MBTC tokens to BTC. She can't do it because:
			// pool_total_liquidity = 9_900 BTC < 10_000 MBTC * 1.0 = 10_000 BTC
			assert_noop!(
				TestProtocol::redeem_wrapped(alice(), CurrencyId::MBTC, TEN_THOUSAND_DOLLARS),
				Error::<Test>::NotEnoughLiquidityAvailable
			);
		});
}

#[test]
fn borrow_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		// Alice deposited 60 DOT to the pool.
		assert_ok!(TestProtocol::deposit_underlying(
			alice(),
			CurrencyId::DOT,
			dollars(60_u128)
		));
		// total_pool_liquidity = 10_000 (insurance) + 60 = 10_060
		assert_eq!(
			TestPools::get_pool_available_liquidity(CurrencyId::DOT),
			dollars(10_060_u128)
		);

		// Alice cannot borrow 100 DOT because she deposited 60 DOT.
		assert_noop!(
			TestProtocol::borrow(alice(), CurrencyId::DOT, dollars(100_u128)),
			Error::<Test>::BorrowControllerRejection
		);

		// MDOT is wrong CurrencyId for underlying assets.
		assert_noop!(
			TestProtocol::borrow(alice(), CurrencyId::MDOT, dollars(60_u128)),
			Error::<Test>::NotValidUnderlyingAssetId
		);

		// Transaction with zero balance is not allowed.
		assert_noop!(
			TestProtocol::borrow(alice(), CurrencyId::DOT, Balance::zero()),
			Error::<Test>::ZeroBalanceTransaction
		);

		// Alice borrowed 30 DOT
		assert_ok!(TestProtocol::borrow(alice(), CurrencyId::DOT, dollars(30_u128)));
		let expected_event =
			TestEvent::minterest_protocol(RawEvent::Borrowed(ALICE, CurrencyId::DOT, dollars(30_u128)));
		assert!(System::events().iter().any(|record| record.event == expected_event));
	});
}

#[test]
fn borrow_fails_if_low_balance_in_pool() {
	ExtBuilder::default()
		.user_balance(ALICE, CurrencyId::BTC, TEN_THOUSAND_DOLLARS)
		.build()
		.execute_with(|| {
			// Alice deposited 100$ to BTC pool.
			assert_ok!(TestProtocol::deposit_underlying(
				alice(),
				CurrencyId::BTC,
				ONE_HUNDRED_DOLLARS
			));

			// set total_pool_liquidity = 50 DOT
			assert_ok!(Currencies::withdraw(
				CurrencyId::BTC,
				&TestPools::pools_account_id(),
				dollars(50_u128)
			));

			// Alice cannot borrow 100 BTC because there is 50 BTC in the pool.
			assert_noop!(
				TestProtocol::borrow(alice(), CurrencyId::BTC, ONE_HUNDRED_DOLLARS),
				Error::<Test>::NotEnoughLiquidityAvailable
			);

			// set total_pool_liquidity = 0 DOT
			assert_ok!(Currencies::withdraw(
				CurrencyId::BTC,
				&TestPools::pools_account_id(),
				dollars(50_u128)
			));

			// Alice cannot borrow 100 BTC because there is 0 BTC in the pool.
			assert_noop!(
				TestProtocol::borrow(alice(), CurrencyId::BTC, ONE_HUNDRED_DOLLARS),
				Error::<Test>::NotEnoughLiquidityAvailable
			);
		});
}

#[test]
fn repay_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		// Alice deposited 60 DOT to the pool.
		assert_ok!(TestProtocol::deposit_underlying(
			alice(),
			CurrencyId::DOT,
			dollars(60_u128)
		));
		// Alice borrowed 30 DOT from the pool.
		assert_ok!(TestProtocol::borrow(alice(), CurrencyId::DOT, dollars(30_u128)));
		// Alice balance = 70 DOT
		assert_eq!(Currencies::free_balance(CurrencyId::DOT, &ALICE), dollars(70_u128));

		// MDOT is wrong CurrencyId for underlying assets.
		assert_noop!(
			TestProtocol::repay(alice(), CurrencyId::MDOT, dollars(10_u128)),
			Error::<Test>::NotValidUnderlyingAssetId
		);

		// Alice cannot repay 100 DOT, because she only have 70 DOT.
		assert_noop!(
			TestProtocol::repay(alice(), CurrencyId::DOT, dollars(100_u128)),
			Error::<Test>::NotEnoughUnderlyingsAssets
		);

		// Alice cannot repay 70 DOT, because she only borrowed 60 DOT.
		assert_noop!(
			TestProtocol::repay(alice(), CurrencyId::DOT, dollars(70_u128)),
			Error::<Test>::RepayAmountToBig
		);

		// Transaction with zero balance is not allowed.
		assert_noop!(
			TestProtocol::repay(alice(), CurrencyId::DOT, Balance::zero()),
			Error::<Test>::ZeroBalanceTransaction
		);

		// Alice repaid 20 DOT. Her borrow_balance = 10 DOT.
		assert_ok!(TestProtocol::repay(alice(), CurrencyId::DOT, dollars(20_u128)));
		let expected_event = TestEvent::minterest_protocol(RawEvent::Repaid(ALICE, CurrencyId::DOT, dollars(20_u128)));
		assert!(System::events().iter().any(|record| record.event == expected_event));
	});
}

#[test]
fn repay_all_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		// Alice deposited 60 DOT to the pool.
		assert_ok!(TestProtocol::deposit_underlying(
			alice(),
			CurrencyId::DOT,
			dollars(60_u128)
		));
		// Alice borrowed 30 DOT from the pool.
		assert_ok!(TestProtocol::borrow(alice(), CurrencyId::DOT, dollars(30_u128)));

		// MDOT is wrong CurrencyId for underlying assets.
		assert_noop!(
			TestProtocol::repay_all(alice(), CurrencyId::MDOT),
			Error::<Test>::NotValidUnderlyingAssetId
		);

		// Alice repaid all 30 DOT.
		assert_ok!(TestProtocol::repay_all(alice(), CurrencyId::DOT));
		let expected_event = TestEvent::minterest_protocol(RawEvent::Repaid(ALICE, CurrencyId::DOT, dollars(30_u128)));
		assert!(System::events().iter().any(|record| record.event == expected_event));
	});
}

#[test]
fn repay_on_behalf_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		// Alice deposited 60 DOT to the pool.
		assert_ok!(TestProtocol::deposit_underlying(
			alice(),
			CurrencyId::DOT,
			dollars(60_u128)
		));

		// Alice borrowed 30 DOT from the pool.
		assert_ok!(TestProtocol::borrow(alice(), CurrencyId::DOT, dollars(30_u128)));

		// MDOT is wrong CurrencyId for underlying assets.
		assert_noop!(
			TestProtocol::repay_on_behalf(bob(), CurrencyId::MDOT, ALICE, dollars(10_u128)),
			Error::<Test>::NotValidUnderlyingAssetId
		);

		// Bob can't pay off the 120 DOT debt for Alice, because he has 100 DOT in his account.
		assert_noop!(
			TestProtocol::repay_on_behalf(bob(), CurrencyId::DOT, ALICE, dollars(120_u128)),
			Error::<Test>::NotEnoughUnderlyingsAssets
		);

		// Bob cannot repay 100 DOT, because Alice only borrowed 60 DOT.
		assert_noop!(
			TestProtocol::repay_on_behalf(bob(), CurrencyId::DOT, ALICE, dollars(100_u128)),
			Error::<Test>::RepayAmountToBig
		);

		// Transaction with zero balance is not allowed.
		assert_noop!(
			TestProtocol::repay_on_behalf(bob(), CurrencyId::DOT, ALICE, Balance::zero()),
			Error::<Test>::ZeroBalanceTransaction
		);

		// Bob repaid 20 DOT for Alice.
		assert_ok!(TestProtocol::repay_on_behalf(
			bob(),
			CurrencyId::DOT,
			ALICE,
			dollars(20_u128)
		));
		let expected_event = TestEvent::minterest_protocol(RawEvent::Repaid(BOB, CurrencyId::DOT, dollars(20_u128)));
		assert!(System::events().iter().any(|record| record.event == expected_event));
	});
}

#[test]
fn enable_as_collateral_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		// Alice cannot enable as collateral ETH pool, because she has not deposited funds into the pool.
		assert_noop!(
			TestProtocol::enable_as_collateral(alice(), CurrencyId::ETH),
			Error::<Test>::CanotBeEnabledAsCollateral
		);

		// Alice deposit 60 ETH
		assert_ok!(TestProtocol::deposit_underlying(
			alice(),
			CurrencyId::ETH,
			dollars(60_u128)
		));

		// Alice enable as collateral her ETH pool.
		assert_ok!(TestProtocol::enable_as_collateral(alice(), CurrencyId::ETH));
		let expected_event = TestEvent::minterest_protocol(RawEvent::PoolEnabledAsCollateral(ALICE, CurrencyId::ETH));
		assert!(System::events().iter().any(|record| record.event == expected_event));
		assert!(TestPools::check_user_available_collateral(&ALICE, CurrencyId::ETH));

		// ETH pool is already collateral.
		assert_noop!(
			TestProtocol::enable_as_collateral(alice(), CurrencyId::ETH),
			Error::<Test>::AlreadyCollateral
		);

		assert_noop!(
			TestProtocol::enable_as_collateral(alice(), CurrencyId::MDOT),
			Error::<Test>::PoolNotFound
		);
	});
}

#[test]
fn disable_collateral_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		assert_noop!(
			TestProtocol::disable_collateral(alice(), CurrencyId::ETH),
			Error::<Test>::AlreadyDisabledCollateral
		);

		// Alice deposit 60 ETH
		assert_ok!(TestProtocol::deposit_underlying(
			alice(),
			CurrencyId::ETH,
			dollars(60_u128)
		));

		// Alice enable as collateral her ETH pool.
		assert_ok!(TestProtocol::enable_as_collateral(alice(), CurrencyId::ETH));

		// Alice disable collateral her ETH pool.
		assert_ok!(TestProtocol::disable_collateral(alice(), CurrencyId::ETH));
		let expected_event = TestEvent::minterest_protocol(RawEvent::PoolDisabledCollateral(ALICE, CurrencyId::ETH));
		assert!(System::events().iter().any(|record| record.event == expected_event));
		assert!(!TestPools::check_user_available_collateral(&ALICE, CurrencyId::ETH));

		assert_noop!(
			TestProtocol::disable_collateral(alice(), CurrencyId::ETH),
			Error::<Test>::AlreadyDisabledCollateral
		);

		assert_noop!(
			TestProtocol::disable_collateral(alice(), CurrencyId::MDOT),
			Error::<Test>::PoolNotFound
		);
	});
}
