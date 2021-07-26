//! Tests for the minterest-protocol pallet.

use super::*;
use mock::{Event, *};

use controller::{ControllerData, PauseKeeper};
use frame_support::{assert_err, assert_noop, assert_ok, error::BadOrigin};
use liquidation_pools::LiquidationPoolData;
use minterest_model::MinterestModelData;
use minterest_primitives::Rate;
use pallet_traits::UserCollateral;
use sp_runtime::{traits::One, FixedPointNumber};

#[test]
fn create_pool_should_work() {
	ExtBuilder::default()
		.set_controller_data(vec![])
		.build()
		.execute_with(|| {
			// The dispatch origin of this call must be Administrator.
			assert_noop!(
				TestMinterestProtocol::create_pool(bob_origin(), DOT, PoolInitData { ..Default::default() }),
				BadOrigin,
			);

			assert_ok!(TestMinterestProtocol::create_pool(
				alice_origin(),
				DOT,
				create_dummy_pool_init_data()
			));
			let expected_event = Event::TestMinterestProtocol(crate::Event::PoolCreated(DOT));
			assert!(System::events().iter().any(|record| record.event == expected_event));

			assert_eq!(
				TestPools::get_pool_data(DOT),
				PoolData {
					borrowed: Balance::zero(),
					borrow_index: Rate::one(),
					protocol_interest: Balance::zero(),
				},
			);
			assert_eq!(
				TestMinterestModel::minterest_model_data_storage(DOT),
				MinterestModelData {
					kink: Rate::saturating_from_rational(2, 3),
					base_rate_per_block: Rate::saturating_from_rational(1, 3),
					multiplier_per_block: Rate::saturating_from_rational(2, 4),
					jump_multiplier_per_block: Rate::saturating_from_rational(1, 2),
				},
			);
			assert_eq!(
				Controller::controller_data_storage(DOT),
				ControllerData {
					last_interest_accrued_block: 1,
					protocol_interest_factor: Rate::saturating_from_rational(1, 10),
					max_borrow_rate: Rate::saturating_from_rational(5, 1000),
					collateral_factor: Rate::saturating_from_rational(9, 10),
					borrow_cap: None,
					protocol_interest_threshold: 100000,
				},
			);
			assert_eq!(Controller::pause_keeper_storage(DOT), PauseKeeper::all_unpaused());
			assert_eq!(
				TestLiquidationPools::liquidation_pool_data_storage(DOT),
				LiquidationPoolData {
					deviation_threshold: Rate::saturating_from_rational(5, 100),
					balance_ratio: Rate::saturating_from_rational(2, 10),
					max_ideal_balance_usd: None,
				},
			);

			// Unable to create pool twice
			assert_noop!(
				TestMinterestProtocol::create_pool(alice_origin(), DOT, create_dummy_pool_init_data()),
				Error::<Test>::PoolAlreadyCreated,
			);
		});
}

#[test]
fn create_pool_should_not_work_when_controller_storage_has_data() {
	ExtBuilder::default()
		.set_controller_data(vec![(DOT, ControllerData::default())])
		.build()
		.execute_with(|| {
			// Controller pallet has record in storage, unable to create new pool
			assert_noop!(
				TestMinterestProtocol::create_pool(alice_origin(), DOT, create_dummy_pool_init_data()),
				controller::Error::<Test>::PoolAlreadyCreated,
			);
		});
}

#[test]
fn create_pool_should_not_work_when_minterest_model_storage_has_data() {
	ExtBuilder::default()
		.set_minterest_model_params(vec![(DOT, MinterestModelData::default())])
		.build()
		.execute_with(|| {
			// MinterestModel pallet has record in storage, unable to create new pool
			assert_noop!(
				TestMinterestProtocol::create_pool(
					alice_origin(),
					DOT,
					PoolInitData {
						kink: Rate::saturating_from_rational(2, 3),
						base_rate_per_block: Rate::saturating_from_rational(1, 3),
						multiplier_per_block: Rate::saturating_from_rational(2, 4),
						jump_multiplier_per_block: Rate::saturating_from_rational(1, 2),
						protocol_interest_factor: Rate::saturating_from_rational(1, 10),
						max_borrow_rate: Rate::saturating_from_rational(5, 1000),
						collateral_factor: Rate::saturating_from_rational(9, 10),
						protocol_interest_threshold: 100000,
						deviation_threshold: Rate::saturating_from_rational(5, 100),
						balance_ratio: Rate::saturating_from_rational(2, 10),
						liquidation_threshold: Rate::saturating_from_rational(3, 100),
						liquidation_fee: Rate::saturating_from_rational(5, 100),
					},
				),
				minterest_model::Error::<Test>::PoolAlreadyCreated,
			);
		});
}

#[test]
fn protocol_operations_not_working_for_nonexisting_pool() {
	ExtBuilder::default()
		.pool_with_params(DOT, Balance::zero(), Rate::saturating_from_rational(1, 1), TEN_THOUSAND)
		.build()
		.execute_with(|| {
			assert_noop!(
				TestMinterestProtocol::deposit_underlying(alice_origin(), ETH, dollars(60_u128)),
				crate::Error::<Test>::PoolNotFound
			);

			assert_noop!(
				TestMinterestProtocol::redeem(alice_origin(), ETH),
				crate::Error::<Test>::PoolNotFound
			);

			assert_noop!(
				TestMinterestProtocol::redeem_underlying(alice_origin(), ETH, dollars(60_u128)),
				crate::Error::<Test>::PoolNotFound
			);

			assert_noop!(
				TestMinterestProtocol::redeem_wrapped(alice_origin(), METH, dollars(60_u128)),
				crate::Error::<Test>::PoolNotFound
			);

			assert_noop!(
				TestMinterestProtocol::borrow(alice_origin(), ETH, dollars(60_u128)),
				crate::Error::<Test>::PoolNotFound
			);

			assert_noop!(
				TestMinterestProtocol::repay(alice_origin(), ETH, Balance::zero()),
				crate::Error::<Test>::PoolNotFound
			);

			assert_noop!(
				TestMinterestProtocol::repay_all(alice_origin(), ETH),
				crate::Error::<Test>::PoolNotFound
			);

			assert_noop!(
				TestMinterestProtocol::repay_on_behalf(bob_origin(), ETH, ALICE, dollars(10_u128)),
				crate::Error::<Test>::PoolNotFound
			);

			assert_noop!(
				TestMinterestProtocol::enable_is_collateral(alice_origin(), ETH),
				crate::Error::<Test>::PoolNotFound
			);

			assert_noop!(
				TestMinterestProtocol::disable_is_collateral(alice_origin(), ETH),
				crate::Error::<Test>::PoolNotFound
			);

			assert_noop!(
				TestMinterestProtocol::transfer_wrapped(alice_origin(), BOB, METH, dollars(10_u128)),
				crate::Error::<Test>::PoolNotFound
			);

			assert_noop!(
				TestMinterestProtocol::claim_mnt(alice_origin(), vec![DOT, ETH]),
				crate::Error::<Test>::PoolNotFound
			);

			assert_noop!(
				TestMinterestProtocol::do_seize(&ALICE, ETH, dollars(100_u128)),
				crate::Error::<Test>::PoolNotFound
			);
		});
}

#[test]
fn deposit_underlying_should_work() {
	ExtBuilder::default()
		.pool_with_params(DOT, Balance::zero(), Rate::saturating_from_rational(1, 1), TEN_THOUSAND)
		.pool_with_params(ETH, Balance::zero(), Rate::saturating_from_rational(1, 1), TEN_THOUSAND)
		.pool_with_params(KSM, Balance::zero(), Rate::saturating_from_rational(1, 1), TEN_THOUSAND)
		.build()
		.execute_with(|| {
			// Alice deposit 60 DOT; exchange_rate = 1.0
			// wrapped_amount = 60.0 DOT / 1.0 = 60.0
			assert_ok!(TestMinterestProtocol::deposit_underlying(
				alice_origin(),
				DOT,
				dollars(60_u128)
			));
			let expected_event = Event::TestMinterestProtocol(crate::Event::Deposited(
				ALICE,
				DOT,
				dollars(60_u128),
				MDOT,
				dollars(60_u128),
			));
			assert!(System::events().iter().any(|record| record.event == expected_event));

			// Check liquidation_attempts has been reset.
			assert_eq!(TestRiskManager::get_user_liquidation_attempts(&ALICE), u8::zero());

			// MDOT pool does not exist.
			assert_noop!(
				TestMinterestProtocol::deposit_underlying(alice_origin(), MDOT, 10),
				Error::<Test>::NotValidUnderlyingAssetId
			);

			// Alice has 100 ETH on her account, so she cannot make a deposit 150 ETH.
			assert_noop!(
				TestMinterestProtocol::deposit_underlying(alice_origin(), ETH, dollars(150_u128)),
				Error::<Test>::NotEnoughLiquidityAvailable
			);

			// Transaction with zero balance is not allowed.
			assert_noop!(
				TestMinterestProtocol::deposit_underlying(alice_origin(), DOT, Balance::zero()),
				Error::<Test>::ZeroBalanceTransaction
			);

			// All operations in the KSM pool are paused.
			assert_noop!(
				TestMinterestProtocol::deposit_underlying(alice_origin(), KSM, dollars(10_u128)),
				Error::<Test>::OperationPaused
			);

			// Whitelist Mode is enabled. In whitelist mode, only members
			// from whitelist can work with protocol.
			assert_ok!(TestWhitelist::switch_whitelist_mode(alice_origin(), true));
			assert_ok!(TestWhitelist::add_member(alice_origin(), BOB));
			assert_noop!(
				TestMinterestProtocol::deposit_underlying(alice_origin(), KSM, dollars(10_u128)),
				BadOrigin
			);
			// Bob is a whitelist member.
			assert_ok!(TestMinterestProtocol::deposit_underlying(
				bob_origin(),
				DOT,
				dollars(10_u128)
			));
			assert_ok!(TestWhitelist::switch_whitelist_mode(alice_origin(), false));
			assert_ok!(TestMinterestProtocol::deposit_underlying(
				alice_origin(),
				DOT,
				dollars(10_u128)
			));
		});
}

#[test]
fn redeem_should_work() {
	ExtBuilder::default()
		.pool_with_params(DOT, Balance::zero(), Rate::saturating_from_rational(1, 1), TEN_THOUSAND)
		.build()
		.execute_with(|| {
			// Alice deposit 60 DOT; exchange_rate = 1.0
			assert_ok!(TestMinterestProtocol::deposit_underlying(
				alice_origin(),
				DOT,
				dollars(60_u128)
			));

			// Alice redeem all 60 MDOT; exchange_rate = 1.0
			assert_ok!(TestMinterestProtocol::redeem(alice_origin(), DOT));
			let expected_event = Event::TestMinterestProtocol(crate::Event::Redeemed(
				ALICE,
				DOT,
				dollars(60_u128),
				MDOT,
				dollars(60_u128),
			));
			assert!(System::events().iter().any(|record| record.event == expected_event));
		});
}

#[test]
fn redeem_should_not_work() {
	ExtBuilder::default()
		.user_balance(ALICE, MKSM, ONE_HUNDRED)
		.pool_with_params(DOT, Balance::zero(), Rate::saturating_from_rational(1, 1), TEN_THOUSAND)
		.pool_with_params(KSM, Balance::zero(), Rate::saturating_from_rational(1, 1), TEN_THOUSAND)
		.build()
		.execute_with(|| {
			// Bob has 0 MDOT on her account, so she cannot make a redeem.
			assert_noop!(
				TestMinterestProtocol::redeem(bob_origin(), DOT),
				Error::<Test>::NotEnoughWrappedTokens
			);

			// MDOT is wrong CurrencyId for underlying assets.
			assert_noop!(
				TestMinterestProtocol::redeem(alice_origin(), MDOT),
				Error::<Test>::NotValidUnderlyingAssetId
			);

			// All operations in the KSM pool are paused.
			assert_noop!(
				TestMinterestProtocol::redeem(alice_origin(), KSM),
				Error::<Test>::OperationPaused
			);

			// Alice deposit 60 DOT; exchange_rate = 1.0
			assert_ok!(TestMinterestProtocol::deposit_underlying(
				alice_origin(),
				DOT,
				dollars(60_u128)
			));

			// Whitelist Mode is enabled. In whitelist mode, only members
			// from whitelist can work with protocol.
			assert_ok!(TestWhitelist::switch_whitelist_mode(alice_origin(), true));
			assert_noop!(TestMinterestProtocol::redeem(alice_origin(), DOT), BadOrigin);

			assert_ok!(TestWhitelist::switch_whitelist_mode(alice_origin(), false));
			assert_ok!(TestMinterestProtocol::redeem(alice_origin(), DOT,));
		});
}

#[test]
fn redeem_fails_if_low_balance_in_pool() {
	ExtBuilder::default()
		.pool_with_params(BTC, Balance::zero(), Rate::one(), Balance::zero())
		.user_balance(ALICE, BTC, TEN_THOUSAND)
		.build()
		.execute_with(|| {
			// Alice deposited 10_000$ to BTC pool.
			assert_ok!(TestMinterestProtocol::deposit_underlying(
				alice_origin(),
				BTC,
				TEN_THOUSAND
			));

			// Alice borrowed 100$ from BTC pool:
			// pool_total_liquidity = 10_000 - 100 = 9_900$
			assert_ok!(TestMinterestProtocol::borrow(alice_origin(), BTC, ONE_HUNDRED));

			// Alice has 10_000 MBTC. exchange_rate = 1.0
			// Alice is trying to change all her 10_000 MBTC tokens to BTC. She can't do it because:
			// pool_total_liquidity = 9_900 < 10_000 * 1.0 = 10_000
			assert_noop!(
				TestMinterestProtocol::redeem(alice_origin(), BTC),
				Error::<Test>::NotEnoughLiquidityAvailable
			);
		});
}

#[test]
fn redeem_underlying_should_work() {
	ExtBuilder::default()
		.user_balance(ALICE, MKSM, ONE_HUNDRED)
		.pool_with_params(DOT, Balance::zero(), Rate::saturating_from_rational(1, 1), TEN_THOUSAND)
		.pool_with_params(KSM, Balance::zero(), Rate::saturating_from_rational(1, 1), TEN_THOUSAND)
		.build()
		.execute_with(|| {
			// Alice deposited 60 DOT to the pool.
			assert_ok!(TestMinterestProtocol::deposit_underlying(
				alice_origin(),
				DOT,
				dollars(60_u128)
			));

			// Alice can't redeem 100 DOT, because 100 DOT equal 100 * 1.0 = 100 MDOT
			// And she has 60 MDOT on her balance.
			assert_noop!(
				TestMinterestProtocol::redeem_underlying(alice_origin(), DOT, dollars(100_u128)),
				Error::<Test>::NotEnoughWrappedTokens
			);

			// MDOT is wrong CurrencyId for underlying assets.
			assert_noop!(
				TestMinterestProtocol::redeem_underlying(alice_origin(), MDOT, dollars(20_u128)),
				Error::<Test>::NotValidUnderlyingAssetId
			);

			// Transaction with zero balance is not allowed.
			assert_noop!(
				TestMinterestProtocol::redeem_underlying(alice_origin(), DOT, Balance::zero()),
				Error::<Test>::ZeroBalanceTransaction
			);

			// All operations in the KSM pool are paused.
			assert_noop!(
				TestMinterestProtocol::redeem_underlying(alice_origin(), KSM, dollars(10_u128)),
				Error::<Test>::OperationPaused
			);

			// Whitelist Mode is enabled. In whitelist mode, only members
			// from whitelist can work with protocol.
			assert_ok!(TestWhitelist::switch_whitelist_mode(alice_origin(), true));
			assert_noop!(
				TestMinterestProtocol::redeem_underlying(alice_origin(), DOT, dollars(30_u128)),
				BadOrigin
			);

			assert_ok!(TestWhitelist::switch_whitelist_mode(alice_origin(), false));

			assert_ok!(TestMinterestProtocol::redeem_underlying(
				alice_origin(),
				DOT,
				dollars(30_u128)
			));
			let expected_event = Event::TestMinterestProtocol(crate::Event::Redeemed(
				ALICE,
				DOT,
				dollars(30_u128),
				MDOT,
				dollars(30_u128),
			));
			assert!(System::events().iter().any(|record| record.event == expected_event));
		});
}

#[test]
fn redeem_underlying_fails_if_low_balance_in_pool() {
	ExtBuilder::default()
		.pool_with_params(BTC, Balance::zero(), Rate::one(), Balance::zero())
		.user_balance(ALICE, BTC, TEN_THOUSAND)
		.build()
		.execute_with(|| {
			// Alice deposited 10_000$ to BTC pool.
			assert_ok!(TestMinterestProtocol::deposit_underlying(
				alice_origin(),
				BTC,
				TEN_THOUSAND
			));

			// Alice borrowed 100$ from BTC pool:
			// pool_total_liquidity = 10_000 - 100 = 9_900$
			assert_ok!(TestMinterestProtocol::borrow(alice_origin(), BTC, ONE_HUNDRED));

			// Alice has 10_000 MBTC. exchange_rate = 1.0
			// Alice is trying to change all her 10_000 MBTC tokens to BTC. She can't do it because:
			// pool_total_liquidity = 9_900 BTC < 10_000 BTC
			assert_noop!(
				TestMinterestProtocol::redeem_underlying(alice_origin(), BTC, TEN_THOUSAND),
				Error::<Test>::NotEnoughLiquidityAvailable
			);
		});
}

#[test]
fn redeem_wrapped_should_work() {
	ExtBuilder::default()
		.user_balance(ALICE, MKSM, ONE_HUNDRED)
		.pool_with_params(DOT, Balance::zero(), Rate::saturating_from_rational(1, 1), TEN_THOUSAND)
		.pool_with_params(KSM, Balance::zero(), Rate::saturating_from_rational(1, 1), TEN_THOUSAND)
		.build()
		.execute_with(|| {
			// Alice deposited 60 DOT to the pool.
			assert_ok!(TestMinterestProtocol::deposit_underlying(
				alice_origin(),
				DOT,
				dollars(60_u128)
			));

			// Alice has 60 MDOT. She can't redeem 100 MDOT.
			assert_noop!(
				TestMinterestProtocol::redeem_wrapped(alice_origin(), MDOT, dollars(100_u128)),
				Error::<Test>::NotEnoughWrappedTokens
			);

			// MDOT is wrong CurrencyId for underlying assets.
			assert_noop!(
				TestMinterestProtocol::redeem_wrapped(alice_origin(), DOT, dollars(20_u128)),
				Error::<Test>::NotValidWrappedTokenId
			);

			// Transaction with zero balance is not allowed.
			assert_noop!(
				TestMinterestProtocol::redeem_wrapped(alice_origin(), MDOT, Balance::zero()),
				Error::<Test>::ZeroBalanceTransaction
			);

			// All operations in the KSM pool are paused.
			assert_noop!(
				TestMinterestProtocol::redeem_wrapped(alice_origin(), MKSM, dollars(10_u128)),
				Error::<Test>::OperationPaused
			);

			// Whitelist Mode is enabled. In whitelist mode, only members
			// from whitelist can work with protocol.
			assert_ok!(TestWhitelist::switch_whitelist_mode(alice_origin(), true));
			assert_noop!(
				TestMinterestProtocol::redeem_wrapped(alice_origin(), MDOT, dollars(35_u128)),
				BadOrigin
			);

			assert_ok!(TestWhitelist::switch_whitelist_mode(alice_origin(), false));

			assert_ok!(TestMinterestProtocol::redeem_wrapped(
				alice_origin(),
				MDOT,
				dollars(35_u128)
			));
			let expected_event = Event::TestMinterestProtocol(crate::Event::Redeemed(
				ALICE,
				DOT,
				dollars(35_u128),
				MDOT,
				dollars(35_u128),
			));
			assert!(System::events().iter().any(|record| record.event == expected_event));
		});
}

#[test]
fn redeem_wrapped_fails_if_low_balance_in_pool() {
	ExtBuilder::default()
		.pool_with_params(BTC, Balance::zero(), Rate::one(), Balance::zero())
		.user_balance(ALICE, BTC, TEN_THOUSAND)
		.build()
		.execute_with(|| {
			// Alice deposited 10_000$ to BTC pool.
			assert_ok!(TestMinterestProtocol::deposit_underlying(
				alice_origin(),
				BTC,
				TEN_THOUSAND
			));

			// Alice borrowed 100$ from BTC pool:
			// pool_total_liquidity = 10_000 - 100 = 9_900$
			assert_ok!(TestMinterestProtocol::borrow(alice_origin(), BTC, ONE_HUNDRED));

			// Alice has 10_000 MBTC. exchange_rate = 1.0
			// Alice is trying to change all her 10_000 MBTC tokens to BTC. She can't do it because:
			// pool_total_liquidity = 9_900 BTC < 10_000 MBTC * 1.0 = 10_000 BTC
			assert_noop!(
				TestMinterestProtocol::redeem_wrapped(alice_origin(), MBTC, TEN_THOUSAND),
				Error::<Test>::NotEnoughLiquidityAvailable
			);
		});
}

#[test]
fn borrow_should_work() {
	ExtBuilder::default()
		.pool_with_params(DOT, Balance::zero(), Rate::saturating_from_rational(1, 1), TEN_THOUSAND)
		.pool_with_params(ETH, Balance::zero(), Rate::saturating_from_rational(1, 1), TEN_THOUSAND)
		.pool_with_params(KSM, Balance::zero(), Rate::saturating_from_rational(1, 1), TEN_THOUSAND)
		.build()
		.execute_with(|| {
			// Alice deposited 60 DOT to the pool.
			assert_ok!(TestMinterestProtocol::deposit_underlying(
				alice_origin(),
				DOT,
				dollars(60_u128)
			));
			// total_pool_liquidity = 10_000 (interest) + 60 = 10_060
			assert_eq!(TestPools::get_pool_available_liquidity(DOT), dollars(10_060_u128));

			// Alice cannot borrow 100 DOT because she deposited 60 DOT.
			assert_noop!(
				TestMinterestProtocol::borrow(alice_origin(), DOT, dollars(100_u128)),
				controller::Error::<Test>::InsufficientLiquidity
			);

			// MDOT is wrong CurrencyId for underlying assets.
			assert_noop!(
				TestMinterestProtocol::borrow(alice_origin(), MDOT, dollars(60_u128)),
				Error::<Test>::NotValidUnderlyingAssetId
			);

			// Transaction with zero balance is not allowed.
			assert_noop!(
				TestMinterestProtocol::borrow(alice_origin(), DOT, Balance::zero()),
				Error::<Test>::ZeroBalanceTransaction
			);

			// All operations in the KSM pool are paused.
			assert_noop!(
				TestMinterestProtocol::borrow(alice_origin(), KSM, dollars(10_u128)),
				Error::<Test>::OperationPaused
			);

			// Whitelist Mode is enabled. In whitelist mode, only members
			// from whitelist can work with protocol.
			assert_ok!(TestWhitelist::switch_whitelist_mode(alice_origin(), true));
			assert_noop!(
				TestMinterestProtocol::borrow(alice_origin(), DOT, dollars(30_u128)),
				BadOrigin
			);

			assert_ok!(TestWhitelist::switch_whitelist_mode(alice_origin(), false));

			// Alice borrowed 30 DOT
			assert_ok!(TestMinterestProtocol::borrow(alice_origin(), DOT, dollars(30_u128)));
			let expected_event = Event::TestMinterestProtocol(crate::Event::Borrowed(ALICE, DOT, dollars(30_u128)));
			assert!(System::events().iter().any(|record| record.event == expected_event));
		});
}

#[test]
fn borrow_fails_if_low_balance_in_pool() {
	ExtBuilder::default()
		.pool_with_params(BTC, Balance::zero(), Rate::one(), Balance::zero())
		.user_balance(ALICE, BTC, TEN_THOUSAND)
		.build()
		.execute_with(|| {
			// Alice deposited 100$ to BTC pool.
			assert_ok!(TestMinterestProtocol::deposit_underlying(
				alice_origin(),
				BTC,
				ONE_HUNDRED
			));

			// set total_pool_liquidity = 50 DOT
			assert_ok!(Currencies::withdraw(
				BTC,
				&TestPools::pools_account_id(),
				dollars(50_u128)
			));

			// Alice cannot borrow 100 BTC because there is 50 BTC in the pool.
			assert_noop!(
				TestMinterestProtocol::borrow(alice_origin(), BTC, ONE_HUNDRED),
				Error::<Test>::NotEnoughLiquidityAvailable
			);

			// set total_pool_liquidity = 0 DOT
			assert_ok!(Currencies::withdraw(
				BTC,
				&TestPools::pools_account_id(),
				dollars(50_u128)
			));

			// Alice cannot borrow 100 BTC because there is 0 BTC in the pool.
			assert_noop!(
				TestMinterestProtocol::borrow(alice_origin(), BTC, ONE_HUNDRED),
				Error::<Test>::NotEnoughLiquidityAvailable
			);
		});
}

#[test]
fn repay_should_work() {
	ExtBuilder::default()
		.pool_with_params(DOT, Balance::zero(), Rate::saturating_from_rational(1, 1), TEN_THOUSAND)
		.pool_with_params(KSM, Balance::zero(), Rate::saturating_from_rational(1, 1), TEN_THOUSAND)
		.build()
		.execute_with(|| {
			// Alice deposited 60 DOT to the pool.
			assert_ok!(TestMinterestProtocol::deposit_underlying(
				alice_origin(),
				DOT,
				dollars(60_u128)
			));
			// Alice borrowed 30 DOT from the pool.
			assert_ok!(TestMinterestProtocol::borrow(alice_origin(), DOT, dollars(30_u128)));
			// Alice balance = 70 DOT
			assert_eq!(Currencies::free_balance(DOT, &ALICE), dollars(70_u128));

			// MDOT is wrong CurrencyId for underlying assets.
			assert_noop!(
				TestMinterestProtocol::repay(alice_origin(), MDOT, dollars(10_u128)),
				Error::<Test>::NotValidUnderlyingAssetId
			);

			// Alice cannot repay 100 DOT, because she only have 70 DOT.
			assert_noop!(
				TestMinterestProtocol::repay(alice_origin(), DOT, dollars(100_u128)),
				Error::<Test>::NotEnoughUnderlyingAsset
			);

			// Alice cannot repay 70 DOT, because she only borrowed 60 DOT.
			assert_noop!(
				TestMinterestProtocol::repay(alice_origin(), DOT, dollars(70_u128)),
				liquidity_pools::Error::<Test>::RepayAmountTooBig
			);

			// Transaction with zero balance is not allowed.
			assert_noop!(
				TestMinterestProtocol::repay(alice_origin(), DOT, Balance::zero()),
				Error::<Test>::ZeroBalanceTransaction
			);

			// All operations in the KSM pool are paused.
			assert_noop!(
				TestMinterestProtocol::repay(alice_origin(), KSM, dollars(10_u128)),
				Error::<Test>::OperationPaused
			);

			// Whitelist Mode is enabled. In whitelist mode, only members
			// from whitelist can work with protocol.
			assert_ok!(TestWhitelist::switch_whitelist_mode(alice_origin(), true));
			assert_noop!(
				TestMinterestProtocol::repay(alice_origin(), DOT, dollars(20_u128)),
				BadOrigin
			);

			assert_ok!(TestWhitelist::switch_whitelist_mode(alice_origin(), false));

			// Alice repaid 20 DOT. Her borrow_balance = 10 DOT.
			assert_ok!(TestMinterestProtocol::repay(alice_origin(), DOT, dollars(20_u128)));
			let expected_event = Event::TestMinterestProtocol(crate::Event::Repaid(ALICE, DOT, dollars(20_u128)));
			assert!(System::events().iter().any(|record| record.event == expected_event));
		});
}

#[test]
fn repay_all_should_work() {
	ExtBuilder::default()
		.pool_with_params(DOT, Balance::zero(), Rate::saturating_from_rational(1, 1), TEN_THOUSAND)
		.pool_with_params(KSM, Balance::zero(), Rate::saturating_from_rational(1, 1), TEN_THOUSAND)
		.build()
		.execute_with(|| {
			// Alice deposited 60 DOT to the pool.
			assert_ok!(TestMinterestProtocol::deposit_underlying(
				alice_origin(),
				DOT,
				dollars(60_u128)
			));
			// Alice borrowed 30 DOT from the pool.
			assert_ok!(TestMinterestProtocol::borrow(alice_origin(), DOT, dollars(30_u128)));

			// MDOT is wrong CurrencyId for underlying assets.
			assert_noop!(
				TestMinterestProtocol::repay_all(alice_origin(), MDOT),
				Error::<Test>::NotValidUnderlyingAssetId
			);

			// All operations in the KSM pool are paused.
			assert_noop!(
				TestMinterestProtocol::repay_all(alice_origin(), KSM),
				Error::<Test>::OperationPaused
			);

			// Whitelist Mode is enabled. In whitelist mode, only members
			// from whitelist can work with protocol.
			assert_ok!(TestWhitelist::switch_whitelist_mode(alice_origin(), true));
			assert_noop!(TestMinterestProtocol::repay_all(alice_origin(), DOT), BadOrigin);

			assert_ok!(TestWhitelist::switch_whitelist_mode(alice_origin(), false));

			// Alice repaid all 30 DOT.
			assert_ok!(TestMinterestProtocol::repay_all(alice_origin(), DOT));
			let expected_event = Event::TestMinterestProtocol(crate::Event::Repaid(ALICE, DOT, dollars(30_u128)));
			assert!(System::events().iter().any(|record| record.event == expected_event));
		});
}

#[test]
fn repay_all_fails_if_not_enough_underlying_assets() {
	ExtBuilder::default()
		.pool_with_params(DOT, Balance::zero(), Rate::saturating_from_rational(1, 1), TEN_THOUSAND)
		.build()
		.execute_with(|| {
			// Alice deposited 60 DOT to the pool.
			assert_ok!(TestMinterestProtocol::deposit_underlying(
				alice_origin(),
				DOT,
				dollars(60_u128)
			));
			// Alice borrowed 30 DOT from the pool.
			assert_ok!(TestMinterestProtocol::borrow(alice_origin(), DOT, dollars(30_u128)));

			// Alice deposited 70 DOT to the pool. Now she have 0 DOT in her account.
			assert_ok!(TestMinterestProtocol::deposit_underlying(
				alice_origin(),
				DOT,
				dollars(70_u128)
			));

			// Insufficient DOT in the ALICE account for repay 30 DOT.
			assert_noop!(
				TestMinterestProtocol::repay_all(alice_origin(), DOT),
				Error::<Test>::NotEnoughUnderlyingAsset
			);
		});
}

#[test]
fn repay_on_behalf_should_work() {
	ExtBuilder::default()
		.pool_with_params(DOT, Balance::zero(), Rate::saturating_from_rational(1, 1), TEN_THOUSAND)
		.pool_with_params(KSM, Balance::zero(), Rate::saturating_from_rational(1, 1), TEN_THOUSAND)
		.build()
		.execute_with(|| {
			// Alice deposited 60 DOT to the pool.
			assert_ok!(TestMinterestProtocol::deposit_underlying(
				alice_origin(),
				DOT,
				dollars(60_u128)
			));

			// Alice borrowed 30 DOT from the pool.
			assert_ok!(TestMinterestProtocol::borrow(alice_origin(), DOT, dollars(30_u128)));

			// MDOT is wrong CurrencyId for underlying assets.
			assert_noop!(
				TestMinterestProtocol::repay_on_behalf(bob_origin(), MDOT, ALICE, dollars(10_u128)),
				Error::<Test>::NotValidUnderlyingAssetId
			);

			// Bob can't pay off the 120 DOT debt for Alice, because he has 100 DOT in his account.
			assert_noop!(
				TestMinterestProtocol::repay_on_behalf(bob_origin(), DOT, ALICE, dollars(120_u128)),
				Error::<Test>::NotEnoughUnderlyingAsset
			);

			// Bob cannot repay 100 DOT, because Alice only borrowed 60 DOT.
			assert_noop!(
				TestMinterestProtocol::repay_on_behalf(bob_origin(), DOT, ALICE, dollars(100_u128)),
				liquidity_pools::Error::<Test>::RepayAmountTooBig
			);

			// Transaction with zero balance is not allowed.
			assert_noop!(
				TestMinterestProtocol::repay_on_behalf(bob_origin(), DOT, ALICE, Balance::zero()),
				Error::<Test>::ZeroBalanceTransaction
			);

			// All operations in the KSM pool are paused.
			assert_noop!(
				TestMinterestProtocol::repay_on_behalf(bob_origin(), KSM, ALICE, dollars(10_u128)),
				Error::<Test>::OperationPaused
			);

			// Whitelist Mode is enabled. In whitelist mode, only members
			// from whitelist can work with protocol.
			assert_ok!(TestWhitelist::switch_whitelist_mode(alice_origin(), true));
			assert_ok!(TestWhitelist::add_member(alice_origin(), BOB));

			// Bob repaid 20 DOT for Alice.
			assert_ok!(TestMinterestProtocol::repay_on_behalf(
				bob_origin(),
				DOT,
				ALICE,
				dollars(20_u128)
			));
			let expected_event = Event::TestMinterestProtocol(crate::Event::Repaid(BOB, DOT, dollars(20_u128)));
			assert!(System::events().iter().any(|record| record.event == expected_event));
		});
}

// TODO: add comments

#[test]
fn do_seize_should_work() {
	ExtBuilder::default()
		.pool_with_params(DOT, Balance::zero(), Rate::one(), Balance::zero())
		.pool_with_params(BTC, dollars(50), Rate::one(), Balance::zero())
		.user_balance(ALICE, MDOT, TEN_THOUSAND)
		.user_balance(ALICE, MBTC, ONE_HUNDRED)
		.build()
		.execute_with(|| {
			// DOT: exchange_rate = 10_000 / 10_000 = 1;
			// alice_seize_wrap = alice_seize_underlying / exchange_rate;
			// alice_seize_wrap = 10.001 DOT / 1 = 10.001 MDOT;
			// alice_supply_MDOT = 10.000 MDOT < alice_seize_wrap = 10.001 MDOT;
			assert_err!(
				TestMinterestProtocol::do_seize(&ALICE, DOT, dollars(10_001)),
				Error::<Test>::NotEnoughWrappedTokens
			);

			// BTC: exchange_rate = 50 / 50 = 1;
			// alice_seize_MBTC = 50 BTC / 1 = 50 MBTC;
			// pool_supply_btc = 0 < alice_seize_BTC = 50 BTC
			assert_err!(
				TestMinterestProtocol::do_seize(&ALICE, BTC, dollars(50)),
				Error::<Test>::NotEnoughLiquidityAvailable
			);

			assert_eq!(Currencies::free_balance(MDOT, &ALICE), TEN_THOUSAND);
			assert_eq!(TestPools::get_pool_available_liquidity(DOT), TEN_THOUSAND);
			assert_eq!(TestLiquidationPools::get_pool_available_liquidity(DOT), Balance::zero());

			assert_ok!(TestMinterestProtocol::do_seize(&ALICE, DOT, dollars(100)));

			assert_eq!(Currencies::free_balance(MDOT, &ALICE), dollars(9_900));
			assert_eq!(TestPools::get_pool_available_liquidity(DOT), dollars(9_900));
			assert_eq!(TestLiquidationPools::get_pool_available_liquidity(DOT), dollars(100));
		});
}

#[test]
fn enable_is_collateral_should_work() {
	ExtBuilder::default()
		.pool_with_params(DOT, Balance::zero(), Rate::saturating_from_rational(1, 1), TEN_THOUSAND)
		.pool_with_params(ETH, Balance::zero(), Rate::saturating_from_rational(1, 1), TEN_THOUSAND)
		.build()
		.execute_with(|| {
			// Alice cannot enable as collateral ETH pool, because she has not deposited funds into the pool.
			assert_noop!(
				TestMinterestProtocol::enable_is_collateral(alice_origin(), ETH),
				Error::<Test>::IsCollateralCannotBeEnabled
			);

			// Alice deposit 60 ETH
			assert_ok!(TestMinterestProtocol::deposit_underlying(
				alice_origin(),
				ETH,
				dollars(60_u128)
			));

			// Whitelist Mode is enabled. In whitelist mode, only members
			// from whitelist can work with protocol.
			assert_ok!(TestWhitelist::switch_whitelist_mode(alice_origin(), true));
			assert_noop!(
				TestMinterestProtocol::enable_is_collateral(alice_origin(), ETH),
				BadOrigin
			);

			assert_ok!(TestWhitelist::switch_whitelist_mode(alice_origin(), false));

			// Alice enable as collateral her ETH pool.
			assert_ok!(TestMinterestProtocol::enable_is_collateral(alice_origin(), ETH));
			let expected_event = Event::TestMinterestProtocol(crate::Event::PoolEnabledIsCollateral(ALICE, ETH));
			assert!(System::events().iter().any(|record| record.event == expected_event));
			assert!(TestPools::is_pool_collateral(&ALICE, ETH));

			// ETH pool is already collateral.
			assert_noop!(
				TestMinterestProtocol::enable_is_collateral(alice_origin(), ETH),
				Error::<Test>::AlreadyIsCollateral
			);

			assert_noop!(
				TestMinterestProtocol::enable_is_collateral(alice_origin(), MDOT),
				Error::<Test>::NotValidUnderlyingAssetId
			);
		});
}

#[test]
fn disable_is_collateral_should_work() {
	ExtBuilder::default()
		.pool_with_params(DOT, Balance::zero(), Rate::saturating_from_rational(1, 1), TEN_THOUSAND)
		.pool_with_params(ETH, Balance::zero(), Rate::saturating_from_rational(1, 1), TEN_THOUSAND)
		.build()
		.execute_with(|| {
			assert_noop!(
				TestMinterestProtocol::disable_is_collateral(alice_origin(), ETH),
				Error::<Test>::IsCollateralAlreadyDisabled
			);

			// Alice deposit 60 ETH
			assert_ok!(TestMinterestProtocol::deposit_underlying(
				alice_origin(),
				ETH,
				dollars(60_u128)
			));

			// Alice enable as collateral her ETH pool.
			assert_ok!(TestMinterestProtocol::enable_is_collateral(alice_origin(), ETH));

			// Whitelist Mode is enabled. In whitelist mode, only members
			// from whitelist can work with protocol.
			assert_ok!(TestWhitelist::switch_whitelist_mode(alice_origin(), true));
			assert_noop!(
				TestMinterestProtocol::disable_is_collateral(alice_origin(), ETH),
				BadOrigin
			);

			assert_ok!(TestWhitelist::switch_whitelist_mode(alice_origin(), false));

			// Alice disable collateral her ETH pool.
			assert_ok!(TestMinterestProtocol::disable_is_collateral(alice_origin(), ETH));
			let expected_event = Event::TestMinterestProtocol(crate::Event::PoolDisabledIsCollateral(ALICE, ETH));
			assert!(System::events().iter().any(|record| record.event == expected_event));
			assert!(!TestPools::is_pool_collateral(&ALICE, ETH));

			assert_noop!(
				TestMinterestProtocol::disable_is_collateral(alice_origin(), ETH),
				Error::<Test>::IsCollateralAlreadyDisabled
			);

			assert_noop!(
				TestMinterestProtocol::disable_is_collateral(alice_origin(), MDOT),
				Error::<Test>::NotValidUnderlyingAssetId
			);
		});
}

#[test]
fn transfer_wrapped_should_work() {
	ExtBuilder::default()
		.pool_with_params(DOT, Balance::zero(), Rate::saturating_from_rational(1, 1), TEN_THOUSAND)
		.pool_with_params(BTC, Balance::zero(), Rate::one(), Balance::zero())
		.user_balance(ALICE, MDOT, ONE_HUNDRED)
		.user_balance(BOB, MBTC, ONE_HUNDRED)
		.build()
		.execute_with(|| {
			// Alice can transfer all tokens to Bob
			assert_ok!(TestMinterestProtocol::transfer_wrapped(
				alice_origin(),
				BOB,
				MDOT,
				ONE_HUNDRED
			));
			let expected_event = Event::TestMinterestProtocol(crate::Event::Transferred(ALICE, BOB, MDOT, ONE_HUNDRED));
			assert!(System::events().iter().any(|record| record.event == expected_event));
			assert_eq!(Currencies::free_balance(MDOT, &ALICE), 0);
			assert_eq!(Currencies::free_balance(MDOT, &BOB), ONE_HUNDRED);

			// Bob can transfer all tokens to Alice
			assert_ok!(TestMinterestProtocol::transfer_wrapped(
				bob_origin(),
				ALICE,
				MBTC,
				ONE_HUNDRED,
			));
			let expected_event = Event::TestMinterestProtocol(crate::Event::Transferred(BOB, ALICE, MBTC, ONE_HUNDRED));
			assert!(System::events().iter().any(|record| record.event == expected_event));
			assert_eq!(Currencies::free_balance(MBTC, &ALICE), ONE_HUNDRED);
			assert_eq!(Currencies::free_balance(MBTC, &BOB), 0);

			// Alice can transfer part of all tokens to Bob
			assert_ok!(TestMinterestProtocol::transfer_wrapped(
				alice_origin(),
				BOB,
				MBTC,
				dollars(40_u128),
			));
			let expected_event =
				Event::TestMinterestProtocol(crate::Event::Transferred(ALICE, BOB, MBTC, dollars(40_u128)));
			assert!(System::events().iter().any(|record| record.event == expected_event));
			assert_eq!(Currencies::free_balance(MBTC, &ALICE), dollars(60_u128));
			assert_eq!(Currencies::free_balance(MBTC, &BOB), dollars(40_u128));

			// Bob can transfer part of all tokens to Alice
			assert_ok!(TestMinterestProtocol::transfer_wrapped(
				bob_origin(),
				ALICE,
				MDOT,
				dollars(40_u128),
			));
			let expected_event =
				Event::TestMinterestProtocol(crate::Event::Transferred(BOB, ALICE, MDOT, dollars(40_u128)));
			assert!(System::events().iter().any(|record| record.event == expected_event));
			assert_eq!(Currencies::free_balance(MDOT, &ALICE), dollars(40_u128));
			assert_eq!(Currencies::free_balance(MDOT, &BOB), dollars(60_u128));
		});
}

#[test]
fn transfer_wrapped_should_not_work() {
	ExtBuilder::default()
		.user_balance(ALICE, MDOT, ONE_HUNDRED)
		.user_balance(ALICE, MKSM, ONE_HUNDRED)
		.pool_with_params(DOT, Balance::zero(), Rate::saturating_from_rational(1, 1), TEN_THOUSAND)
		.pool_with_params(KSM, Balance::zero(), Rate::saturating_from_rational(1, 1), TEN_THOUSAND)
		.build()
		.execute_with(|| {
			// Alice is unable to transfer more tokens tan she has
			assert_noop!(
				TestMinterestProtocol::transfer_wrapped(alice_origin(), BOB, MNT, ONE_HUNDRED),
				Error::<Test>::NotValidWrappedTokenId
			);

			// Alice is unable to transfer tokens to self
			assert_noop!(
				TestMinterestProtocol::transfer_wrapped(alice_origin(), ALICE, MDOT, ONE_HUNDRED),
				Error::<Test>::CannotTransferToSelf
			);

			// Whitelist Mode is enabled. In whitelist mode, only members
			// from whitelist can work with protocol.
			assert_ok!(TestWhitelist::switch_whitelist_mode(alice_origin(), true));
			assert_noop!(
				TestMinterestProtocol::transfer_wrapped(alice_origin(), BOB, MDOT, ONE_HUNDRED),
				BadOrigin
			);

			assert_ok!(TestWhitelist::switch_whitelist_mode(alice_origin(), false));

			// Alice is unable to transfer more tokens than she has
			assert_noop!(
				TestMinterestProtocol::transfer_wrapped(alice_origin(), BOB, MDOT, dollars(101_u128)),
				Error::<Test>::NotEnoughWrappedTokens
			);

			// Bob is unable to transfer tokens with zero balance
			assert_noop!(
				TestMinterestProtocol::transfer_wrapped(bob_origin(), ALICE, MDOT, 1_u128),
				Error::<Test>::NotEnoughWrappedTokens
			);

			// Bob is unable to send zero tokens
			assert_noop!(
				TestMinterestProtocol::transfer_wrapped(bob_origin(), ALICE, MBTC, Balance::zero()),
				Error::<Test>::ZeroBalanceTransaction
			);

			// All operations in the KSM pool are paused.
			assert_noop!(
				TestMinterestProtocol::transfer_wrapped(alice_origin(), BOB, MKSM, ONE_HUNDRED),
				Error::<Test>::OperationPaused
			);
		});
}

#[test]
fn claim_mnt_should_work() {
	ExtBuilder::default()
		.pool_with_params(DOT, Balance::zero(), Rate::saturating_from_rational(1, 1), TEN_THOUSAND)
		.build()
		.execute_with(|| {
			System::set_block_number(10);
			// Bob's operations are needed to calculate distribution speeds.
			assert_ok!(TestMinterestProtocol::deposit_underlying(
				bob_origin(),
				DOT,
				dollars(100_u128)
			));
			assert_ok!(TestMinterestProtocol::borrow(bob_origin(), DOT, dollars(50_u128)));

			System::set_block_number(50);

			assert_ok!(TestMinterestProtocol::deposit_underlying(
				alice_origin(),
				DOT,
				dollars(60_u128)
			));

			System::set_block_number(100);

			assert_ok!(TestMinterestProtocol::claim_mnt(alice_origin(), vec![DOT]));
			// Calculation of the balance of Alice in MNT tokens (only supply distribution):
			// balance = previous_balance + speed_DOT * block_delta * alice_supply / total_supply;
			// balance = 0 + 0.1 * 50 * 60 / 160 = 1.875 MNT;
			assert_eq!(Currencies::free_balance(MNT, &ALICE), 1_875_000_000_000_000_000);
			let expected_event = Event::TestMinterestProtocol(crate::Event::Claimed(ALICE));
			assert!(System::events().iter().any(|record| record.event == expected_event));

			System::set_block_number(200);

			assert_ok!(TestMinterestProtocol::borrow(alice_origin(), DOT, dollars(10_u128)));

			System::set_block_number(300);

			assert_ok!(TestMinterestProtocol::claim_mnt(alice_origin(), vec![DOT]));
			/*
			Calculation of the balance of Alice in MNT tokens (borrow and supply distribution):
			Supply:
			supply_balance = previous_balance + speed_DOT * block_delta * alice_supply / total_supply;
			supply_balance = 1.875 MNT + 0.1 * 200 * 60 / 160 = 9.375 MNT;
			Borrow:
			borrow_balance = previous_balance + speed_DOT * block_delta * alice_borrow / total_borrow;
			borrow_balance = 0 + 0.1 * 100 * 10 / 60 = 1.6667 MNT
			total_alice_balance = supply_balance + borrow_balance = 9.375 MNT + 1.6667 MNT = 11.042 MNT
			 */
			assert_eq!(Currencies::free_balance(MNT, &ALICE), 11_041_666_666_666_666_660);

			System::set_block_number(400);

			assert_ok!(TestMinterestProtocol::borrow(alice_origin(), DOT, dollars(30_u128)));

			System::set_block_number(500);

			assert_ok!(TestMinterestProtocol::claim_mnt(alice_origin(), vec![DOT]));
			/*
			Calculation of the balance of Alice in MNT tokens (borrow and supply distribution):
			Supply:
			supply_balance = previous_balance + speed_DOT * block_delta * alice_supply / total_supply;
			supply_balance = 9.375 MNT + 0.1 * 200 * 60 / 160 = 16.875 MNT;
			Borrow:
			borrow_balance = previous_balance + speed_DOT * block_delta * alice_borrow / total_borrow;
			borrow_balance = 1.6667 + 0.1 * 100 * 10 / 60 = 3.333 MNT
			borrow_balance = 3.333 + 0.1 * 100 * 40 / 90 = 7.7774 MNT
			total_alice_balance = supply_balance + borrow_balance = 16.875 MNT + 7.7774 MNT = 24.652 MNT
			 */
			assert_eq!(Currencies::free_balance(MNT, &ALICE), 24_652_777_777_777_777_760);
		})
}

#[test]
fn partial_protocol_interest_transfer_should_work() {
	ExtBuilder::default()
		.pool_with_params(
			DOT,
			Balance::zero(),
			Rate::saturating_from_rational(1, 1),
			dollars(11_000u128),
		)
		.build()
		.execute_with(|| {
			assert_eq!(TestPools::pool_data_storage(DOT).protocol_interest, dollars(11_000u128));
			assert_eq!(TestPools::get_pool_available_liquidity(DOT), dollars(10_000u128));

			TestMinterestProtocol::on_finalize(1);

			// Not all protocol interest transferred because of insufficient liquidity
			assert_eq!(TestPools::pool_data_storage(DOT).protocol_interest, dollars(1_000u128));
		});
}
