//! Tests for the risk-manager pallet.
use super::*;
use frame_support::{assert_noop, assert_ok};
use minterest_primitives::Operation::Deposit;
use mock::{Event, *};
use sp_runtime::{traits::BadOrigin, FixedPointNumber};

#[test]
fn user_liquidation_attempts_should_work() {
	ExternalityBuilder::default().build().execute_with(|| {
		TestRiskManager::user_liquidation_attempts_increase_by_one(&ALICE);
		assert_eq!(TestRiskManager::get_user_liquidation_attempts(&ALICE), u8::one());
		TestRiskManager::user_liquidation_attempts_increase_by_one(&ALICE);
		assert_eq!(TestRiskManager::get_user_liquidation_attempts(&ALICE), 2_u8);
		TestRiskManager::user_liquidation_attempts_reset_to_zero(&ALICE);
		assert_eq!(TestRiskManager::get_user_liquidation_attempts(&ALICE), u8::zero());
	})
}

#[test]
fn mutate_depending_operation_should_work() {
	ExternalityBuilder::default()
		.set_pool_user_data(DOT, ALICE, Balance::zero(), Rate::zero(), true)
		.build()
		.execute_with(|| {
			assert_eq!(TestRiskManager::get_user_liquidation_attempts(&ALICE), u8::zero());
			TestRiskManager::user_liquidation_attempts_increase_by_one(&ALICE);
			assert_eq!(TestRiskManager::get_user_liquidation_attempts(&ALICE), 1_u8);

			// ETH pool is disabled as collateral. Don't reset liquidation attempts.
			TestRiskManager::mutate_depending_operation(Some(ETH), &ALICE, Deposit);
			assert_eq!(TestRiskManager::get_user_liquidation_attempts(&ALICE), 1_u8);

			// DOT pool is enabled as collateral. Reset liquidation attempts to zero.
			TestRiskManager::mutate_depending_operation(Some(DOT), &ALICE, Deposit);
			assert_eq!(TestRiskManager::get_user_liquidation_attempts(&ALICE), u8::zero());
		})
}

#[test]
fn set_liquidation_fee_should_work() {
	ExternalityBuilder::default().build().execute_with(|| {
		// Can be set to 0..
		assert_ok!(TestRiskManager::set_liquidation_fee(
			admin_origin(),
			DOT,
			Rate::saturating_from_rational(3, 10)
		));
		assert_eq!(
			TestRiskManager::liquidation_fee_storage(DOT),
			Rate::saturating_from_rational(3, 10)
		);
		let expected_event = Event::TestRiskManager(crate::Event::LiquidationFeeUpdated(
			DOT,
			Rate::saturating_from_rational(3, 10),
		));
		assert!(System::events().iter().any(|record| record.event == expected_event));

		// Can not be set to 1.0
		assert_noop!(
			TestRiskManager::set_liquidation_fee(admin_origin(), DOT, Rate::one()),
			Error::<Test>::InvalidLiquidationFeeValue
		);

		// The dispatch origin of this call must be Administrator.
		assert_noop!(
			TestRiskManager::set_liquidation_fee(alice_origin(), DOT, Rate::one()),
			BadOrigin
		);

		// MDOT is wrong CurrencyId for underlying assets.
		assert_noop!(
			TestRiskManager::set_liquidation_fee(admin_origin(), MDOT, Rate::one()),
			Error::<Test>::NotValidUnderlyingAssetId
		);
	});
}

#[test]
fn set_threshold_should_work() {
	ExternalityBuilder::default().build().execute_with(|| {
		// Can be set to 1.0
		assert_ok!(TestRiskManager::set_liquidation_threshold(
			admin_origin(),
			DOT,
			Rate::one()
		));
		assert_eq!(TestRiskManager::liquidation_threshold_storage(), Rate::one());
		let expected_event = Event::TestRiskManager(crate::Event::LiquidationThresholdUpdated(Rate::one()));
		assert!(System::events().iter().any(|record| record.event == expected_event));

		// The dispatch origin of this call must be Administrator.
		assert_noop!(
			TestRiskManager::set_liquidation_threshold(alice_origin(), DOT, Rate::one()),
			BadOrigin
		);

		// MDOT is wrong CurrencyId for underlying assets.
		assert_noop!(
			TestRiskManager::set_liquidation_threshold(admin_origin(), MDOT, Rate::one()),
			Error::<Test>::NotValidUnderlyingAssetId
		);
	});
}

// ---------------------- mod liquidation tests ----------------------------

// Alice: 500 DOT borrow; 200 DOT supply; 300 ETH borrow; 750 BTC collateral.
// Note: prices for all assets set equal $1.
#[test]
fn build_user_loan_state_should_work() {
	ExternalityBuilder::default()
		.set_liquidation_fees(vec![
			(DOT, Rate::saturating_from_rational(5, 100)),
			(ETH, Rate::saturating_from_rational(10, 100)),
			(BTC, Rate::saturating_from_rational(15, 100)),
		])
		.deposit_underlying(ALICE, DOT, dollars(200_u128))
		.deposit_underlying(ALICE, BTC, dollars(750_u128))
		.enable_as_collateral(ALICE, BTC)
		.borrow_underlying(ALICE, DOT, dollars(500_u128))
		.borrow_underlying(ALICE, ETH, dollars(300_u128))
		.merge_duplicates()
		.build()
		.execute_with(|| {
			let alice_loan_state = UserLoanState::<Test>::build_user_loan_state(&ALICE).unwrap();
			assert_eq!(alice_loan_state.get_user_supplies(), vec![(BTC, dollars(750))]);
			assert_eq!(
				alice_loan_state.get_user_borrows(),
				vec![(DOT, dollars(500)), (ETH, dollars(300))]
			);
			// alice_total_borrow = $500 + $300 = $800.
			assert_eq!(alice_loan_state.total_borrow().unwrap(), dollars(800));
			// alice_total_supply in collateral pools: $750.
			assert_eq!(alice_loan_state.total_supply().unwrap(), dollars(750));
			// alice_total_collateral = $750 * 0.9 = $675.
			assert_eq!(alice_loan_state.total_collateral().unwrap(), dollars(675));
			// alice_total_seize = $500 * 1.05 + $300 * 1.10 = $855.
			assert_eq!(alice_loan_state.total_seize().unwrap(), dollars(855));
		})
}

#[test]
fn calculate_seize_amount_should_work() {
	ExternalityBuilder::default()
		.set_liquidation_fees(vec![
			(DOT, Rate::saturating_from_rational(5, 100)),
			(ETH, Rate::saturating_from_rational(15, 100)),
		])
		.build()
		.execute_with(|| {
			assert_eq!(
				UserLoanState::<Test>::calculate_seize_amount(DOT, dollars(100_u128)).unwrap(),
				dollars(105_u128)
			);
			assert_eq!(
				UserLoanState::<Test>::calculate_seize_amount(ETH, dollars(100_u128)).unwrap(),
				dollars(115_u128)
			);
		})
}

#[test]
fn choose_liquidation_mode_should_work() {
	ExternalityBuilder::default()
		.set_liquidation_fees(vec![
			(DOT, Rate::saturating_from_rational(5, 100)),
			(ETH, Rate::saturating_from_rational(10, 100)),
			(BTC, Rate::saturating_from_rational(15, 100)),
		])
		.set_controller_data_mock(vec![DOT, ETH, BTC])
		.build()
		.execute_with(|| {
			// let make_up_user_loan_state =
			// 	|supplies: &Vec<(CurrencyId, Balance)>, borrows: &Vec<(CurrencyId, Balance)>| -> UserLoanState<Test> {
			// 		let mut user_loan_state = UserLoanState::<Test>::new();
			// 		user_loan_state.supplies.extend_from_slice(&supplies);
			// 		user_loan_state.borrows.extend_from_slice(&borrows);
			// 		user_loan_state
			// 	};
			//
			// let supplies = vec![(DOT, dollars(300)), (ETH, dollars(650)), (BTC, dollars(50))];
			// let solvent_borrows = vec![(DOT, dollars(200)), (ETH, dollars(400))];
			//
			// let solvent_borrow_state = make_up_user_loan_state(&supplies, &solvent_borrows);
			// assert_noop!(
			// 	UserLoanState::<Test>::choose_liquidation_mode(&ALICE, &solvent_borrow_state),
			// 	Error::<Test>::SolventUserLoan
			// );
		});
}
