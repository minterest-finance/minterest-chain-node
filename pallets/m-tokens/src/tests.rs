/// Unit tests for the m-tokens module.

use super::*;
use mock::*;

use frame_support::{
    assert_ok, assert_noop,
    dispatch::{DispatchError},
};

#[test]
fn approve_work() {
    ExtBuilder::default().build().execute_with(|| {
        assert_ok!(MTokens::approve(Origin::signed(1), 2, CurrencyId::MDOT, 33));
        assert_ok!(MTokens::approve(Origin::signed(2), 1, CurrencyId::MKSM, 45));
        assert_eq!(MTokens::allowance((CurrencyId::MDOT, 1, 2)), 33);
        assert_eq!(MTokens::allowance((CurrencyId::MKSM, 2, 1)), 45);
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
fn transfer_from_not_enough_allowance() {
    ExtBuilder::default().build().execute_with(|| {
        assert_ok!(MTokens::approve(Origin::signed(ALICE), BOB, CurrencyId::MDOT, 100));
        assert_noop!(
            MTokens::transfer_from(Origin::signed(ALICE), ALICE, BOB, CurrencyId::MDOT, 101),
            DispatchError::Other("Not enough allowance.")
        );
    });
}

#[test]
fn transfer_from_work() {
    ExtBuilder::default()
        .balances(vec![
            (ALICE, CurrencyId::MINT, ONE_MILL),
            (ALICE, CurrencyId::MDOT, ONE_MILL)
        ])
        .build()
        .execute_with(|| {
            assert_ok!(MTokens::approve(Origin::signed(ALICE), BOB, CurrencyId::MDOT, 100));
            //TODO положить на баланс алисы монеты для газа
            // assert_ok!(MTokens::transfer_from(Origin::signed(ALICE), ALICE, BOB, CurrencyId::MDOT, 50));
        });
}
