/// Unit tests for the m-tokens module.
use super::*;
use mock::*;

use frame_support::{assert_noop, assert_ok};

#[test]
fn approve_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		assert_ok!(MTokens::approve(Origin::signed(ALICE), BOB, CurrencyId::MDOT, 33));
		assert_ok!(MTokens::approve(Origin::signed(BOB), ALICE, CurrencyId::MKSM, 45));
		assert_eq!(MTokens::allowance((CurrencyId::MDOT, ALICE, BOB)), 33);
		assert_eq!(MTokens::allowance((CurrencyId::MKSM, BOB, ALICE)), 45);
	});
}

#[test]
fn double_approve_work() {
	ExtBuilder::default().build().execute_with(|| {
		assert_ok!(MTokens::approve(Origin::signed(ALICE), BOB, CurrencyId::METH, 33));
		assert_ok!(MTokens::approve(Origin::signed(ALICE), BOB, CurrencyId::METH, 67));
		assert_eq!(MTokens::allowance((CurrencyId::METH, ALICE, BOB)), 100);
	});
}

#[test]
fn approve_fails_if_overflow() {
	ExtBuilder::default().build().execute_with(|| {
		assert_ok!(MTokens::approve(Origin::signed(ALICE), BOB, CurrencyId::MDOT, 100));
		assert_eq!(MTokens::allowance((CurrencyId::MDOT, ALICE, BOB)), 100);
		assert_noop!(
			MTokens::approve(Origin::signed(ALICE), BOB, CurrencyId::MDOT, Balance::max_value()),
			Error::<Runtime>::OverflowAllowance
		);
	});
}

#[test]
fn transfer_from_should_work() {
	ExtBuilder::default()
		.one_million_mnt_and_mdot_for_alice()
		.build()
		.execute_with(|| {
			assert_ok!(MTokens::approve(Origin::signed(ALICE), BOB, CurrencyId::MDOT, 100));
			assert_eq!(MTokens::allowance((CurrencyId::MDOT, ALICE, BOB)), 100);
			assert_ok!(MTokens::transfer_from(
				Origin::signed(ALICE),
				ALICE,
				BOB,
				CurrencyId::MDOT,
				50
			));
		});
}

#[test]
fn transfer_from_not_enough_allowance() {
	ExtBuilder::default().build().execute_with(|| {
		assert_ok!(MTokens::approve(Origin::signed(ALICE), BOB, CurrencyId::MDOT, 100));
		assert_eq!(MTokens::allowance((CurrencyId::MDOT, ALICE, BOB)), 100);
		assert_noop!(
			MTokens::transfer_from(Origin::signed(ALICE), ALICE, BOB, CurrencyId::MDOT, 101),
			Error::<Runtime>::NotEnoughAllowance
		);
	});
}

#[test]
fn transfer_from_fails_if_balance_too_low() {
	ExtBuilder::default().build().execute_with(|| {
		assert_ok!(MTokens::approve(Origin::signed(ALICE), BOB, CurrencyId::MDOT, 100));
		assert_eq!(MTokens::allowance((CurrencyId::MDOT, ALICE, BOB)), 100);
		assert_noop!(
			MTokens::transfer_from(Origin::signed(ALICE), ALICE, BOB, CurrencyId::MDOT, 50),
			orml_tokens::Error::<Runtime>::BalanceTooLow
		);
	});
}

#[test]
fn transfer_from_fails_if_allowance_does_not_exist() {
	ExtBuilder::default()
		.one_million_mnt_and_mdot_for_alice()
		.build()
		.execute_with(|| {
			assert_ok!(MTokens::approve(Origin::signed(ALICE), BOB, CurrencyId::MDOT, 100));
			assert_eq!(MTokens::allowance((CurrencyId::MDOT, ALICE, BOB)), 100);
			assert_noop!(
				MTokens::transfer_from(Origin::signed(ALICE), ALICE, BOB, CurrencyId::DOT, 50),
				Error::<Runtime>::AllowanceDoesNotExist
			);
		});
}
