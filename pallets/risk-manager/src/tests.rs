//! Tests for the risk-manager pallet.
use super::*;
use crate::mock::*;

#[test]
fn user_liquidation_attempts_should_work() {
	ExternalityBuilder::default().build().execute_with(|| {
		TestRiskManager::increase_by_one(&ALICE);
		assert_eq!(TestRiskManager::get_user_liquidation_attempts(&ALICE), u8::one());
		TestRiskManager::increase_by_one(&ALICE);
		assert_eq!(TestRiskManager::get_user_liquidation_attempts(&ALICE), 2_u8);
		TestRiskManager::reset_to_zero(&ALICE);
		assert_eq!(TestRiskManager::get_user_liquidation_attempts(&ALICE), u8::zero());
	})
}

#[test]
fn mutate_upon_deposit_should_work() {
	ExternalityBuilder::default().build().execute_with(|| {
		assert_eq!(TestRiskManager::get_user_liquidation_attempts(&ALICE), u8::zero());
		TestRiskManager::increase_by_one(&ALICE);
		TestRiskManager::mutate_upon_deposit(DOT, &ALICE);
	})
}
