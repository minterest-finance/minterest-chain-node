//! Tests for the risk-manager pallet.
use super::*;
use crate::mock::*;

#[test]
fn user_liquidation_attempts_should_work() {
	ExternalityBuilder::default().build().execute_with(|| {
		TestRiskManager::increase_user_liquidation_attempts(&ALICE);
		assert_eq!(TestRiskManager::get_user_liquidation_attempts(&ALICE), u8::one());
		TestRiskManager::increase_user_liquidation_attempts(&ALICE);
		assert_eq!(TestRiskManager::get_user_liquidation_attempts(&ALICE), 2_u8);
		TestRiskManager::reset_user_liquidation_attempts(&ALICE);
		assert_eq!(TestRiskManager::get_user_liquidation_attempts(&ALICE), u8::zero());
	})
}
