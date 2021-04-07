//! Tests for the minterest-protocol pallet.

use super::*;
use mock::{Event, *};

use frame_support::{assert_noop, assert_ok, error::BadOrigin};
use minterest_primitives::Rate;
use sp_runtime::FixedPointNumber;

fn dollars<T: Into<u128>>(d: T) -> Balance {
	DOLLARS.saturating_mul(d.into())
}

#[test]
fn deposit_underlying_should_work() {
	ExtBuilder::default()
		.pool_with_params(
			CurrencyId::DOT,
			Balance::zero(),
			Rate::saturating_from_rational(1, 1),
			TEN_THOUSAND_DOLLARS,
		)
		.pool_with_params(
			CurrencyId::ETH,
			Balance::zero(),
			Rate::saturating_from_rational(1, 1),
			TEN_THOUSAND_DOLLARS,
		)
		.pool_with_params(
			CurrencyId::KSM,
			Balance::zero(),
			Rate::saturating_from_rational(1, 1),
			TEN_THOUSAND_DOLLARS,
		)
		.build()
		.execute_with(|| {
			// Alice deposit 60 DOT; exchange_rate = 1.0
			// wrapped_amount = 60.0 DOT / 1.0 = 60.0
			assert_ok!(TestProtocol::deposit_underlying(
				alice(),
				CurrencyId::DOT,
				dollars(60_u128)
			));
			let expected_event = Event::minterest_protocol(crate::Event::Deposited(
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

			// All operations in the KSM pool are paused.
			assert_noop!(
				TestProtocol::deposit_underlying(alice(), CurrencyId::KSM, dollars(10_u128)),
				Error::<Test>::OperationPaused
			);

			// Whitelist Mode is enabled. In whitelist mode, only members
			// 'WhitelistCouncil' can work with protocols.
			controller::WhitelistMode::<Test>::put(true);
			assert_noop!(
				TestProtocol::deposit_underlying(alice(), CurrencyId::KSM, dollars(10_u128)),
				BadOrigin
			);
			// Bob is a whitelist member.
			assert_ok!(TestProtocol::deposit_underlying(
				bob(),
				CurrencyId::DOT,
				dollars(10_u128)
			));
			controller::WhitelistMode::<Test>::put(false);
			assert_ok!(TestProtocol::deposit_underlying(
				alice(),
				CurrencyId::DOT,
				dollars(10_u128)
			));
		});
}

#[test]
fn redeem_should_work() {
	ExtBuilder::default()
		.pool_with_params(
			CurrencyId::DOT,
			Balance::zero(),
			Rate::saturating_from_rational(1, 1),
			TEN_THOUSAND_DOLLARS,
		)
		.build()
		.execute_with(|| {
			// Alice deposit 60 DOT; exchange_rate = 1.0
			assert_ok!(TestProtocol::deposit_underlying(
				alice(),
				CurrencyId::DOT,
				dollars(60_u128)
			));

			// Alice redeem all 60 MDOT; exchange_rate = 1.0
			assert_ok!(TestProtocol::redeem(alice(), CurrencyId::DOT));
			let expected_event = Event::minterest_protocol(crate::Event::Redeemed(
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
	ExtBuilder::default()
		.user_balance(ALICE, CurrencyId::MKSM, ONE_HUNDRED_DOLLARS)
		.pool_with_params(
			CurrencyId::DOT,
			Balance::zero(),
			Rate::saturating_from_rational(1, 1),
			TEN_THOUSAND_DOLLARS,
		)
		.pool_with_params(
			CurrencyId::KSM,
			Balance::zero(),
			Rate::saturating_from_rational(1, 1),
			TEN_THOUSAND_DOLLARS,
		)
		.build()
		.execute_with(|| {
			// Bob has 0 MDOT on her account, so she cannot make a redeem.
			assert_noop!(
				TestProtocol::redeem(bob(), CurrencyId::DOT),
				Error::<Test>::NotEnoughWrappedTokens
			);

			// MDOT is wrong CurrencyId for underlying assets.
			assert_noop!(
				TestProtocol::redeem(alice(), CurrencyId::MDOT),
				Error::<Test>::NotValidUnderlyingAssetId
			);

			// All operations in the KSM pool are paused.
			assert_noop!(
				TestProtocol::redeem(alice(), CurrencyId::KSM),
				Error::<Test>::OperationPaused
			);

			// Alice deposit 60 DOT; exchange_rate = 1.0
			assert_ok!(TestProtocol::deposit_underlying(
				alice(),
				CurrencyId::DOT,
				dollars(60_u128)
			));

			// Whitelist Mode is enabled. In whitelist mode, only members
			// 'WhitelistCouncil' can work with protocols.
			controller::WhitelistMode::<Test>::put(true);
			assert_noop!(TestProtocol::redeem(alice(), CurrencyId::DOT), BadOrigin);

			controller::WhitelistMode::<Test>::put(false);
			assert_ok!(TestProtocol::redeem(alice(), CurrencyId::DOT,));
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
	ExtBuilder::default()
		.user_balance(ALICE, CurrencyId::MKSM, ONE_HUNDRED_DOLLARS)
		.pool_with_params(
			CurrencyId::DOT,
			Balance::zero(),
			Rate::saturating_from_rational(1, 1),
			TEN_THOUSAND_DOLLARS,
		)
		.pool_with_params(
			CurrencyId::KSM,
			Balance::zero(),
			Rate::saturating_from_rational(1, 1),
			TEN_THOUSAND_DOLLARS,
		)
		.build()
		.execute_with(|| {
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

			// All operations in the KSM pool are paused.
			assert_noop!(
				TestProtocol::redeem_underlying(alice(), CurrencyId::KSM, dollars(10_u128)),
				Error::<Test>::OperationPaused
			);

			// Whitelist Mode is enabled. In whitelist mode, only members
			// 'WhitelistCouncil' can work with protocols.
			controller::WhitelistMode::<Test>::put(true);
			assert_noop!(
				TestProtocol::redeem_underlying(alice(), CurrencyId::DOT, dollars(30_u128)),
				BadOrigin
			);

			controller::WhitelistMode::<Test>::put(false);

			assert_ok!(TestProtocol::redeem_underlying(
				alice(),
				CurrencyId::DOT,
				dollars(30_u128)
			));
			let expected_event = Event::minterest_protocol(crate::Event::Redeemed(
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
	ExtBuilder::default()
		.user_balance(ALICE, CurrencyId::MKSM, ONE_HUNDRED_DOLLARS)
		.pool_with_params(
			CurrencyId::DOT,
			Balance::zero(),
			Rate::saturating_from_rational(1, 1),
			TEN_THOUSAND_DOLLARS,
		)
		.pool_with_params(
			CurrencyId::KSM,
			Balance::zero(),
			Rate::saturating_from_rational(1, 1),
			TEN_THOUSAND_DOLLARS,
		)
		.build()
		.execute_with(|| {
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
				liquidity_pools::Error::<Test>::NotValidWrappedTokenId
			);

			// Transaction with zero balance is not allowed.
			assert_noop!(
				TestProtocol::redeem_wrapped(alice(), CurrencyId::MDOT, Balance::zero()),
				Error::<Test>::ZeroBalanceTransaction
			);

			// All operations in the KSM pool are paused.
			assert_noop!(
				TestProtocol::redeem_wrapped(alice(), CurrencyId::MKSM, dollars(10_u128)),
				Error::<Test>::OperationPaused
			);

			// Whitelist Mode is enabled. In whitelist mode, only members
			// 'WhitelistCouncil' can work with protocols.
			controller::WhitelistMode::<Test>::put(true);
			assert_noop!(
				TestProtocol::redeem_wrapped(alice(), CurrencyId::MDOT, dollars(35_u128)),
				BadOrigin
			);

			controller::WhitelistMode::<Test>::put(false);

			assert_ok!(TestProtocol::redeem_wrapped(
				alice(),
				CurrencyId::MDOT,
				dollars(35_u128)
			));
			let expected_event = Event::minterest_protocol(crate::Event::Redeemed(
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
	ExtBuilder::default()
		.pool_with_params(
			CurrencyId::DOT,
			Balance::zero(),
			Rate::saturating_from_rational(1, 1),
			TEN_THOUSAND_DOLLARS,
		)
		.pool_with_params(
			CurrencyId::ETH,
			Balance::zero(),
			Rate::saturating_from_rational(1, 1),
			TEN_THOUSAND_DOLLARS,
		)
		.build()
		.execute_with(|| {
			// Alice deposited 60 DOT to the pool.
			assert_ok!(TestProtocol::deposit_underlying(
				alice(),
				CurrencyId::DOT,
				dollars(60_u128)
			));
			// total_pool_liquidity = 10_000 (interest) + 60 = 10_060
			assert_eq!(
				TestPools::get_pool_available_liquidity(CurrencyId::DOT),
				dollars(10_060_u128)
			);

			// Alice cannot borrow 100 DOT because she deposited 60 DOT.
			assert_noop!(
				TestProtocol::borrow(alice(), CurrencyId::DOT, dollars(100_u128)),
				controller::Error::<Test>::InsufficientLiquidity
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

			// All operations in the KSM pool are paused.
			assert_noop!(
				TestProtocol::borrow(alice(), CurrencyId::KSM, dollars(10_u128)),
				Error::<Test>::OperationPaused
			);

			// Whitelist Mode is enabled. In whitelist mode, only members
			// 'WhitelistCouncil' can work with protocols.
			controller::WhitelistMode::<Test>::put(true);
			assert_noop!(
				TestProtocol::borrow(alice(), CurrencyId::DOT, dollars(30_u128)),
				BadOrigin
			);

			controller::WhitelistMode::<Test>::put(false);

			// Alice borrowed 30 DOT
			assert_ok!(TestProtocol::borrow(alice(), CurrencyId::DOT, dollars(30_u128)));
			let expected_event =
				Event::minterest_protocol(crate::Event::Borrowed(ALICE, CurrencyId::DOT, dollars(30_u128)));
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
	ExtBuilder::default()
		.pool_with_params(
			CurrencyId::DOT,
			Balance::zero(),
			Rate::saturating_from_rational(1, 1),
			TEN_THOUSAND_DOLLARS,
		)
		.pool_with_params(
			CurrencyId::KSM,
			Balance::zero(),
			Rate::saturating_from_rational(1, 1),
			TEN_THOUSAND_DOLLARS,
		)
		.build()
		.execute_with(|| {
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
				Error::<Test>::NotEnoughUnderlyingAsset
			);

			// Alice cannot repay 70 DOT, because she only borrowed 60 DOT.
			assert_noop!(
				TestProtocol::repay(alice(), CurrencyId::DOT, dollars(70_u128)),
				liquidity_pools::Error::<Test>::RepayAmountTooBig
			);

			// Transaction with zero balance is not allowed.
			assert_noop!(
				TestProtocol::repay(alice(), CurrencyId::DOT, Balance::zero()),
				Error::<Test>::ZeroBalanceTransaction
			);

			// All operations in the KSM pool are paused.
			assert_noop!(
				TestProtocol::repay(alice(), CurrencyId::KSM, dollars(10_u128)),
				Error::<Test>::OperationPaused
			);

			// Whitelist Mode is enabled. In whitelist mode, only members
			// 'WhitelistCouncil' can work with protocols.
			controller::WhitelistMode::<Test>::put(true);
			assert_noop!(
				TestProtocol::repay(alice(), CurrencyId::DOT, dollars(20_u128)),
				BadOrigin
			);

			controller::WhitelistMode::<Test>::put(false);

			// Alice repaid 20 DOT. Her borrow_balance = 10 DOT.
			assert_ok!(TestProtocol::repay(alice(), CurrencyId::DOT, dollars(20_u128)));
			let expected_event =
				Event::minterest_protocol(crate::Event::Repaid(ALICE, CurrencyId::DOT, dollars(20_u128)));
			assert!(System::events().iter().any(|record| record.event == expected_event));
		});
}

#[test]
fn repay_all_should_work() {
	ExtBuilder::default()
		.pool_with_params(
			CurrencyId::DOT,
			Balance::zero(),
			Rate::saturating_from_rational(1, 1),
			TEN_THOUSAND_DOLLARS,
		)
		.pool_with_params(
			CurrencyId::KSM,
			Balance::zero(),
			Rate::saturating_from_rational(1, 1),
			TEN_THOUSAND_DOLLARS,
		)
		.build()
		.execute_with(|| {
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

			// All operations in the KSM pool are paused.
			assert_noop!(
				TestProtocol::repay_all(alice(), CurrencyId::KSM),
				Error::<Test>::OperationPaused
			);

			// Whitelist Mode is enabled. In whitelist mode, only members
			// 'WhitelistCouncil' can work with protocols.
			controller::WhitelistMode::<Test>::put(true);
			assert_noop!(TestProtocol::repay_all(alice(), CurrencyId::DOT), BadOrigin);

			controller::WhitelistMode::<Test>::put(false);

			// Alice repaid all 30 DOT.
			assert_ok!(TestProtocol::repay_all(alice(), CurrencyId::DOT));
			let expected_event =
				Event::minterest_protocol(crate::Event::Repaid(ALICE, CurrencyId::DOT, dollars(30_u128)));
			assert!(System::events().iter().any(|record| record.event == expected_event));
		});
}

#[test]
fn repay_all_fails_if_not_enough_underlying_assets() {
	ExtBuilder::default()
		.pool_with_params(
			CurrencyId::DOT,
			Balance::zero(),
			Rate::saturating_from_rational(1, 1),
			TEN_THOUSAND_DOLLARS,
		)
		.build()
		.execute_with(|| {
			// Alice deposited 60 DOT to the pool.
			assert_ok!(TestProtocol::deposit_underlying(
				alice(),
				CurrencyId::DOT,
				dollars(60_u128)
			));
			// Alice borrowed 30 DOT from the pool.
			assert_ok!(TestProtocol::borrow(alice(), CurrencyId::DOT, dollars(30_u128)));

			// Alice deposited 70 DOT to the pool. Now she have 0 DOT in her account.
			assert_ok!(TestProtocol::deposit_underlying(
				alice(),
				CurrencyId::DOT,
				dollars(70_u128)
			));

			// Insufficient DOT in the ALICE account for repay 30 DOT.
			assert_noop!(
				TestProtocol::repay_all(alice(), CurrencyId::DOT),
				Error::<Test>::NotEnoughUnderlyingAsset
			);
		});
}

#[test]
fn repay_on_behalf_should_work() {
	ExtBuilder::default()
		.pool_with_params(
			CurrencyId::DOT,
			Balance::zero(),
			Rate::saturating_from_rational(1, 1),
			TEN_THOUSAND_DOLLARS,
		)
		.pool_with_params(
			CurrencyId::KSM,
			Balance::zero(),
			Rate::saturating_from_rational(1, 1),
			TEN_THOUSAND_DOLLARS,
		)
		.build()
		.execute_with(|| {
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
				Error::<Test>::NotEnoughUnderlyingAsset
			);

			// Bob cannot repay 100 DOT, because Alice only borrowed 60 DOT.
			assert_noop!(
				TestProtocol::repay_on_behalf(bob(), CurrencyId::DOT, ALICE, dollars(100_u128)),
				liquidity_pools::Error::<Test>::RepayAmountTooBig
			);

			// Transaction with zero balance is not allowed.
			assert_noop!(
				TestProtocol::repay_on_behalf(bob(), CurrencyId::DOT, ALICE, Balance::zero()),
				Error::<Test>::ZeroBalanceTransaction
			);

			// All operations in the KSM pool are paused.
			assert_noop!(
				TestProtocol::repay_on_behalf(bob(), CurrencyId::KSM, ALICE, dollars(10_u128)),
				Error::<Test>::OperationPaused
			);

			// Whitelist Mode is enabled. In whitelist mode, only members
			// 'WhitelistCouncil' can work with protocols.
			controller::WhitelistMode::<Test>::put(true);

			// Bob repaid 20 DOT for Alice.
			assert_ok!(TestProtocol::repay_on_behalf(
				bob(),
				CurrencyId::DOT,
				ALICE,
				dollars(20_u128)
			));
			let expected_event =
				Event::minterest_protocol(crate::Event::Repaid(BOB, CurrencyId::DOT, dollars(20_u128)));
			assert!(System::events().iter().any(|record| record.event == expected_event));
		});
}

#[test]
fn enable_is_collateral_should_work() {
	ExtBuilder::default()
		.pool_with_params(
			CurrencyId::DOT,
			Balance::zero(),
			Rate::saturating_from_rational(1, 1),
			TEN_THOUSAND_DOLLARS,
		)
		.pool_with_params(
			CurrencyId::ETH,
			Balance::zero(),
			Rate::saturating_from_rational(1, 1),
			TEN_THOUSAND_DOLLARS,
		)
		.build()
		.execute_with(|| {
			// Alice cannot enable as collateral ETH pool, because she has not deposited funds into the pool.
			assert_noop!(
				TestProtocol::enable_is_collateral(alice(), CurrencyId::ETH),
				Error::<Test>::IsCollateralCannotBeEnabled
			);

			// Alice deposit 60 ETH
			assert_ok!(TestProtocol::deposit_underlying(
				alice(),
				CurrencyId::ETH,
				dollars(60_u128)
			));

			// Whitelist Mode is enabled. In whitelist mode, only members
			// 'WhitelistCouncil' can work with protocols.
			controller::WhitelistMode::<Test>::put(true);
			assert_noop!(TestProtocol::enable_is_collateral(alice(), CurrencyId::ETH), BadOrigin);

			controller::WhitelistMode::<Test>::put(false);

			// Alice enable as collateral her ETH pool.
			assert_ok!(TestProtocol::enable_is_collateral(alice(), CurrencyId::ETH));
			let expected_event =
				Event::minterest_protocol(crate::Event::PoolEnabledIsCollateral(ALICE, CurrencyId::ETH));
			assert!(System::events().iter().any(|record| record.event == expected_event));
			assert!(TestPools::check_user_available_collateral(&ALICE, CurrencyId::ETH));

			// ETH pool is already collateral.
			assert_noop!(
				TestProtocol::enable_is_collateral(alice(), CurrencyId::ETH),
				Error::<Test>::AlreadyIsCollateral
			);

			assert_noop!(
				TestProtocol::enable_is_collateral(alice(), CurrencyId::MDOT),
				Error::<Test>::NotValidUnderlyingAssetId
			);
		});
}

#[test]
fn disable_is_collateral_should_work() {
	ExtBuilder::default()
		.pool_with_params(
			CurrencyId::DOT,
			Balance::zero(),
			Rate::saturating_from_rational(1, 1),
			TEN_THOUSAND_DOLLARS,
		)
		.pool_with_params(
			CurrencyId::ETH,
			Balance::zero(),
			Rate::saturating_from_rational(1, 1),
			TEN_THOUSAND_DOLLARS,
		)
		.build()
		.execute_with(|| {
			assert_noop!(
				TestProtocol::disable_is_collateral(alice(), CurrencyId::ETH),
				Error::<Test>::IsCollateralAlreadyDisabled
			);

			// Alice deposit 60 ETH
			assert_ok!(TestProtocol::deposit_underlying(
				alice(),
				CurrencyId::ETH,
				dollars(60_u128)
			));

			// Alice enable as collateral her ETH pool.
			assert_ok!(TestProtocol::enable_is_collateral(alice(), CurrencyId::ETH));

			// Whitelist Mode is enabled. In whitelist mode, only members
			// 'WhitelistCouncil' can work with protocols.
			controller::WhitelistMode::<Test>::put(true);
			assert_noop!(TestProtocol::disable_is_collateral(alice(), CurrencyId::ETH), BadOrigin);

			controller::WhitelistMode::<Test>::put(false);

			// Alice disable collateral her ETH pool.
			assert_ok!(TestProtocol::disable_is_collateral(alice(), CurrencyId::ETH));
			let expected_event =
				Event::minterest_protocol(crate::Event::PoolDisabledIsCollateral(ALICE, CurrencyId::ETH));
			assert!(System::events().iter().any(|record| record.event == expected_event));
			assert!(!TestPools::check_user_available_collateral(&ALICE, CurrencyId::ETH));

			assert_noop!(
				TestProtocol::disable_is_collateral(alice(), CurrencyId::ETH),
				Error::<Test>::IsCollateralAlreadyDisabled
			);

			assert_noop!(
				TestProtocol::disable_is_collateral(alice(), CurrencyId::MDOT),
				Error::<Test>::NotValidUnderlyingAssetId
			);
		});
}

#[test]
fn transfer_wrapped_should_work() {
	ExtBuilder::default()
		.user_balance(ALICE, CurrencyId::MDOT, ONE_HUNDRED_DOLLARS)
		.user_balance(BOB, CurrencyId::MBTC, ONE_HUNDRED_DOLLARS)
		.pool_with_params(
			CurrencyId::DOT,
			Balance::zero(),
			Rate::saturating_from_rational(1, 1),
			TEN_THOUSAND_DOLLARS,
		)
		.build()
		.execute_with(|| {
			// Alice can transfer all tokens to Bob
			assert_ok!(TestProtocol::transfer_wrapped(
				alice(),
				BOB,
				CurrencyId::MDOT,
				ONE_HUNDRED_DOLLARS,
			));
			let expected_event = Event::minterest_protocol(crate::Event::Transferred(
				ALICE,
				BOB,
				CurrencyId::MDOT,
				ONE_HUNDRED_DOLLARS,
			));
			assert!(System::events().iter().any(|record| record.event == expected_event));
			assert_eq!(Currencies::free_balance(CurrencyId::MDOT, &ALICE), 0);
			assert_eq!(Currencies::free_balance(CurrencyId::MDOT, &BOB), ONE_HUNDRED_DOLLARS);

			// Bob can transfer all tokens to Alice
			assert_ok!(TestProtocol::transfer_wrapped(
				bob(),
				ALICE,
				CurrencyId::MBTC,
				ONE_HUNDRED_DOLLARS,
			));
			let expected_event = Event::minterest_protocol(crate::Event::Transferred(
				BOB,
				ALICE,
				CurrencyId::MBTC,
				ONE_HUNDRED_DOLLARS,
			));
			assert!(System::events().iter().any(|record| record.event == expected_event));
			assert_eq!(Currencies::free_balance(CurrencyId::MBTC, &ALICE), ONE_HUNDRED_DOLLARS);
			assert_eq!(Currencies::free_balance(CurrencyId::MBTC, &BOB), 0);

			// Alice can transfer part of all tokens to Bob
			assert_ok!(TestProtocol::transfer_wrapped(
				alice(),
				BOB,
				CurrencyId::MBTC,
				dollars(40_u128),
			));
			let expected_event = Event::minterest_protocol(crate::Event::Transferred(
				ALICE,
				BOB,
				CurrencyId::MBTC,
				dollars(40_u128),
			));
			assert!(System::events().iter().any(|record| record.event == expected_event));
			assert_eq!(Currencies::free_balance(CurrencyId::MBTC, &ALICE), dollars(60_u128));
			assert_eq!(Currencies::free_balance(CurrencyId::MBTC, &BOB), dollars(40_u128));

			// Bob can transfer part of all tokens to Alice
			assert_ok!(TestProtocol::transfer_wrapped(
				bob(),
				ALICE,
				CurrencyId::MDOT,
				dollars(40_u128),
			));
			let expected_event = Event::minterest_protocol(crate::Event::Transferred(
				BOB,
				ALICE,
				CurrencyId::MDOT,
				dollars(40_u128),
			));
			assert!(System::events().iter().any(|record| record.event == expected_event));
			assert_eq!(Currencies::free_balance(CurrencyId::MDOT, &ALICE), dollars(40_u128));
			assert_eq!(Currencies::free_balance(CurrencyId::MDOT, &BOB), dollars(60_u128));
		});
}

#[test]
fn transfer_wrapped_should_not_work() {
	ExtBuilder::default()
		.user_balance(ALICE, CurrencyId::MDOT, ONE_HUNDRED_DOLLARS)
		.user_balance(ALICE, CurrencyId::MKSM, ONE_HUNDRED_DOLLARS)
		.pool_with_params(
			CurrencyId::DOT,
			Balance::zero(),
			Rate::saturating_from_rational(1, 1),
			TEN_THOUSAND_DOLLARS,
		)
		.pool_with_params(
			CurrencyId::KSM,
			Balance::zero(),
			Rate::saturating_from_rational(1, 1),
			TEN_THOUSAND_DOLLARS,
		)
		.build()
		.execute_with(|| {
			// Alice is unable to transfer more tokens tan she has
			assert_noop!(
				TestProtocol::transfer_wrapped(alice(), BOB, CurrencyId::MNT, ONE_HUNDRED_DOLLARS),
				liquidity_pools::Error::<Test>::NotValidWrappedTokenId
			);

			// Alice is unable to transfer tokens to self
			assert_noop!(
				TestProtocol::transfer_wrapped(alice(), ALICE, CurrencyId::MDOT, ONE_HUNDRED_DOLLARS),
				Error::<Test>::CannotTransferToSelf
			);

			// Whitelist Mode is enabled. In whitelist mode, only members
			// 'WhitelistCouncil' can work with protocols.
			controller::WhitelistMode::<Test>::put(true);
			assert_noop!(
				TestProtocol::transfer_wrapped(alice(), BOB, CurrencyId::MDOT, ONE_HUNDRED_DOLLARS),
				BadOrigin
			);

			controller::WhitelistMode::<Test>::put(false);

			// Alice is unable to transfer more tokens than she has
			assert_noop!(
				TestProtocol::transfer_wrapped(alice(), BOB, CurrencyId::MDOT, dollars(101_u128)),
				Error::<Test>::NotEnoughWrappedTokens
			);

			// Bob is unable to transfer tokens with zero balance
			assert_noop!(
				TestProtocol::transfer_wrapped(bob(), ALICE, CurrencyId::MDOT, 1_u128),
				Error::<Test>::NotEnoughWrappedTokens
			);

			// Bob is unable to send zero tokens
			assert_noop!(
				TestProtocol::transfer_wrapped(bob(), ALICE, CurrencyId::MBTC, Balance::zero()),
				Error::<Test>::ZeroBalanceTransaction
			);

			// All operations in the KSM pool are paused.
			assert_noop!(
				TestProtocol::transfer_wrapped(alice(), BOB, CurrencyId::MKSM, ONE_HUNDRED_DOLLARS),
				Error::<Test>::OperationPaused
			);
		});
}

#[test]
fn partial_protocol_interest_transfer_should_work() {
	ExtBuilder::default()
		.pool_with_params(
			CurrencyId::DOT,
			Balance::zero(),
			Rate::saturating_from_rational(1, 1),
			dollars(11_000u128),
		)
		.build()
		.execute_with(|| {
			assert_eq!(
				TestPools::pools(CurrencyId::DOT).total_protocol_interest,
				dollars(11_000u128)
			);
			assert_eq!(
				TestPools::get_pool_available_liquidity(CurrencyId::DOT),
				dollars(10_000u128)
			);

			TestProtocol::on_finalize(1);

			// Not all protocol interest transferred because of insufficient liquidity
			assert_eq!(
				TestPools::pools(CurrencyId::DOT).total_protocol_interest,
				dollars(1_000u128)
			);
		});
}
