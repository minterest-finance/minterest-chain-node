//! Tests for the risk-manager pallet.
use super::*;
use crate::mock::*;
use minterest_primitives::Operation::Deposit;

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
fn mutate_depending_operation_should_work() {
	ExternalityBuilder::default()
		.pool_user_data(DOT, ALICE, Balance::zero(), Rate::zero(), true)
		.build()
		.execute_with(|| {
			assert_eq!(TestRiskManager::get_user_liquidation_attempts(&ALICE), u8::zero());
			TestRiskManager::increase_by_one(&ALICE);
			assert_eq!(TestRiskManager::get_user_liquidation_attempts(&ALICE), 1_u8);

			// ETH pool is disabled as collateral. Don't reset liquidation attempts.
			TestRiskManager::mutate_depending_operation(ETH, &ALICE, Deposit);
			assert_eq!(TestRiskManager::get_user_liquidation_attempts(&ALICE), 1_u8);

			// DOT pool is enabled as collateral. Reset liquidation attempts to zero.
			TestRiskManager::mutate_depending_operation(DOT, &ALICE, Deposit);
			assert_eq!(TestRiskManager::get_user_liquidation_attempts(&ALICE), u8::zero());
		})
}
