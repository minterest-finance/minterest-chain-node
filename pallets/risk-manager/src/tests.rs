//! Tests for the risk-manager pallet.
use super::*;
use crate::LiquidationMode::{Complete, ForgivableComplete, Partial};
use frame_support::{assert_noop, assert_ok};
use minterest_primitives::Operation::{Deposit, Redeem, Repay};
use mock::{Event, *};
use sp_runtime::{traits::BadOrigin, FixedPointNumber};

fn set_user_liquidation_attempts_to(n: usize) {
	for _ in 0..n {
		assert_ok!(TestRiskManager::try_mutate_attempts(
			&ALICE,
			Operation::Repay,
			None,
			Some(LiquidationMode::Partial)
		));
	}
}

fn check_user_loan_state(
	user_loan_state: &UserLoanState<TestRuntime>,
	liquidation_mode: Option<LiquidationMode>,
	seizes: Vec<(CurrencyId, Balance)>,
	repays: Vec<(CurrencyId, Balance)>,
) {
	assert_eq!(user_loan_state.get_user_liquidation_mode(), liquidation_mode);
	assert_eq!(user_loan_state.get_user_supplies_to_seize_underlying(), seizes);
	assert_eq!(user_loan_state.get_user_borrows_to_repay_underlying(), repays);
}

#[test]
fn user_liquidation_attempts_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		TestRiskManager::user_liquidation_attempts_increase_by_one(&ALICE);
		assert_eq!(TestRiskManager::get_user_liquidation_attempts(&ALICE), u8::one());
		TestRiskManager::user_liquidation_attempts_increase_by_one(&ALICE);
		assert_eq!(TestRiskManager::get_user_liquidation_attempts(&ALICE), 2_u8);
		TestRiskManager::user_liquidation_attempts_reset_to_zero(&ALICE);
		assert_eq!(TestRiskManager::get_user_liquidation_attempts(&ALICE), u8::zero());
	})
}

#[test]
fn try_mutate_attempts_should_work() {
	ExtBuilder::default()
		.set_pool_user_data(DOT, ALICE, Balance::zero(), Rate::zero(), true)
		.build()
		.execute_with(|| {
			assert_eq!(TestRiskManager::get_user_liquidation_attempts(&ALICE), u8::zero());
			assert_ok!(TestRiskManager::try_mutate_attempts(&ALICE, Repay, None, Some(Partial)));
			assert_eq!(TestRiskManager::get_user_liquidation_attempts(&ALICE), 1_u8);

			// ETH pool is disabled as collateral. Don't reset liquidation attempts.
			assert_ok!(TestRiskManager::try_mutate_attempts(&ALICE, Deposit, Some(ETH), None));
			assert_eq!(TestRiskManager::get_user_liquidation_attempts(&ALICE), 1_u8);

			// DOT pool is enabled as collateral. Reset liquidation attempts to zero.
			assert_ok!(TestRiskManager::try_mutate_attempts(&ALICE, Deposit, Some(DOT), None));
			assert_eq!(TestRiskManager::get_user_liquidation_attempts(&ALICE), u8::zero());

			set_user_liquidation_attempts_to(2);
			assert_eq!(TestRiskManager::get_user_liquidation_attempts(&ALICE), 2_u8);

			assert_ok!(TestRiskManager::try_mutate_attempts(
				&ALICE,
				Repay,
				None,
				Some(Complete)
			));
			assert_eq!(TestRiskManager::get_user_liquidation_attempts(&ALICE), u8::zero());

			set_user_liquidation_attempts_to(2);
			assert_eq!(TestRiskManager::get_user_liquidation_attempts(&ALICE), 2_u8);

			assert_ok!(TestRiskManager::try_mutate_attempts(
				&ALICE,
				Repay,
				None,
				Some(ForgivableComplete)
			));
			assert_eq!(TestRiskManager::get_user_liquidation_attempts(&ALICE), u8::zero());

			assert_noop!(
				TestRiskManager::try_mutate_attempts(&ALICE, Deposit, None, None),
				Error::<TestRuntime>::ErrorChangingLiquidationAttempts
			);
			assert_noop!(
				TestRiskManager::try_mutate_attempts(&ALICE, Repay, None, None),
				Error::<TestRuntime>::ErrorChangingLiquidationAttempts
			);
			assert_noop!(
				TestRiskManager::try_mutate_attempts(&ALICE, Redeem, None, None),
				Error::<TestRuntime>::ErrorChangingLiquidationAttempts
			);
		})
}

#[test]
fn set_liquidation_fee_should_work() {
	ExtBuilder::default().build().execute_with(|| {
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
			Error::<TestRuntime>::InvalidLiquidationFeeValue
		);

		// The dispatch origin of this call must be Administrator.
		assert_noop!(
			TestRiskManager::set_liquidation_fee(alice_origin(), DOT, Rate::one()),
			BadOrigin
		);

		// MDOT is wrong CurrencyId for underlying assets.
		assert_noop!(
			TestRiskManager::set_liquidation_fee(admin_origin(), MDOT, Rate::one()),
			Error::<TestRuntime>::NotValidUnderlyingAssetId
		);
	});
}

#[test]
fn set_threshold_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		// Can be set to 1.0
		assert_ok!(TestRiskManager::set_liquidation_threshold(admin_origin(), Rate::one()));
		assert_eq!(TestRiskManager::liquidation_threshold_storage(), Rate::one());
		let expected_event = Event::TestRiskManager(crate::Event::LiquidationThresholdUpdated(Rate::one()));
		assert!(System::events().iter().any(|record| record.event == expected_event));

		// The dispatch origin of this call must be Administrator.
		assert_noop!(
			TestRiskManager::set_liquidation_threshold(alice_origin(), Rate::one()),
			BadOrigin
		);
	});
}

// ---------------------- mod liquidation tests ----------------------------

// Alice supply: 500 DOT; 500 ETH; 800 BTC collateral.
// Alice borrow: 400 DOT; 330 ETH.
// Note: prices for all assets set equal $1.
#[test]
fn build_user_loan_state_with_accrue_should_work() {
	ExtBuilder::default()
		.set_liquidation_fees(vec![
			(DOT, Rate::saturating_from_rational(5, 100)),
			(ETH, Rate::saturating_from_rational(10, 100)),
			(BTC, Rate::saturating_from_rational(15, 100)),
		])
		.deposit_underlying(ALICE, DOT, dollars(500))
		.deposit_underlying(ALICE, ETH, dollars(500))
		.deposit_underlying(ALICE, BTC, dollars(800))
		.enable_as_collateral(ALICE, BTC)
		.borrow_underlying(ALICE, DOT, dollars(400))
		.borrow_underlying(ALICE, ETH, dollars(330))
		.merge_duplicates()
		.build()
		.execute_with(|| {
			let alice_loan_state = UserLoanState::<TestRuntime>::build_user_loan_state(&ALICE).unwrap();

			assert_eq!(alice_loan_state.get_user_account_id(), &ALICE);
			assert_eq!(alice_loan_state.get_user_supplies(), vec![(BTC, dollars(800))]);
			assert_eq!(
				alice_loan_state.get_user_borrows(),
				vec![(DOT, dollars(400)), (ETH, dollars(330))]
			);
			// alice_total_borrow = $400 + $330 = $730.
			assert_eq!(alice_loan_state.total_borrow().unwrap(), dollars(730));
			// alice_total_supply in collateral pools: $800.
			assert_eq!(alice_loan_state.total_supply().unwrap(), dollars(800));
			// alice_total_collateral = $800 * 0.9 = $720.
			assert_eq!(alice_loan_state.total_collateral().unwrap(), dollars(720));
			// alice_total_seize = $400 * 1.05 + $330 * 1.10 = $783.
			assert_eq!(alice_loan_state.total_seize().unwrap(), dollars(783));
			check_user_loan_state(
				&alice_loan_state,
				Some(Complete),
				<Vec<(CurrencyId, Balance)>>::new(), //TODO: fix after implementation math
				<Vec<(CurrencyId, Balance)>>::new(), //TODO: fix after implementation math
			);

			System::set_block_number(100);

			let alice_loan_state_accrued = UserLoanState::<TestRuntime>::build_user_loan_state(&ALICE).unwrap();

			assert_eq!(alice_loan_state_accrued.get_user_supplies(), vec![(BTC, dollars(800))]);
			assert_eq!(
				alice_loan_state_accrued.get_user_borrows(),
				vec![(DOT, 400_000285120000000000), (ETH, 330_000194059800000000)]
			);
			check_user_loan_state(
				&alice_loan_state_accrued,
				Some(Complete),
				<Vec<(CurrencyId, Balance)>>::new(), //TODO: fix after implementation math
				<Vec<(CurrencyId, Balance)>>::new(), //TODO: fix after implementation math
			);
		})
}

#[test]
fn calculate_seize_amount_should_work() {
	ExtBuilder::default()
		.set_liquidation_fees(vec![
			(DOT, Rate::saturating_from_rational(5, 100)),
			(ETH, Rate::saturating_from_rational(15, 100)),
		])
		.build()
		.execute_with(|| {
			// seize_amount = $100 * 1.05 = $105.
			assert_eq!(
				UserLoanState::<TestRuntime>::calculate_seize_amount(DOT, dollars(100)).unwrap(),
				dollars(105)
			);
			// seize_amount = $100 * 1.15 = $115.
			assert_eq!(
				UserLoanState::<TestRuntime>::calculate_seize_amount(ETH, dollars(100)).unwrap(),
				dollars(115)
			);
		})
}

// Bob   supply: --- DOT; --- ETH; 500 BTC - for liquidity in the BTC pool.
// Alice supply: 300 DOT; 650 ETH; 50 BTC. - all enabled as collateral
// Alice borrow: 200 DOT; 400 ETH; 360 BTC.
// prices for all assets set equal $1.
// partial_liquidation_min_sum = $10_000.
// alice_total_supply = $1000, alice_total_collateral = $900, alice_total_borrow = $960.
// seize=$1008 > supply=$1000 && borrow=$960<min_sum=$10_000 => forgivable.
#[test]
fn forgivable_liquidation_less_min_sum() {
	ExtBuilder::default()
		.set_liquidation_fees(vec![
			(DOT, Rate::saturating_from_rational(5, 100)),
			(ETH, Rate::saturating_from_rational(5, 100)),
			(BTC, Rate::saturating_from_rational(5, 100)),
		])
		.deposit_underlying(BOB, BTC, dollars(500))
		.deposit_underlying(ALICE, DOT, dollars(300))
		.deposit_underlying(ALICE, ETH, dollars(650))
		.deposit_underlying(ALICE, BTC, dollars(50))
		.enable_as_collateral(ALICE, DOT)
		.enable_as_collateral(ALICE, ETH)
		.enable_as_collateral(ALICE, BTC)
		.borrow_underlying(ALICE, DOT, dollars(200))
		.borrow_underlying(ALICE, ETH, dollars(400))
		.borrow_underlying(ALICE, BTC, dollars(360))
		.merge_duplicates()
		.build()
		.execute_with(|| {
			// alice_liquidation_attempts == 0:
			let alice_loan_state = UserLoanState::<TestRuntime>::build_user_loan_state(&ALICE).unwrap();
			assert_eq!(TestRiskManager::get_user_liquidation_attempts(&ALICE), 0_u8);
			check_user_loan_state(
				&alice_loan_state,
				Some(ForgivableComplete),
				<Vec<(CurrencyId, Balance)>>::new(), //TODO: fix after implementation math
				<Vec<(CurrencyId, Balance)>>::new(), //TODO: fix after implementation math
			);

			set_user_liquidation_attempts_to(1);

			// alice_liquidation_attempts == 1:
			assert_eq!(TestRiskManager::get_user_liquidation_attempts(&ALICE), 1_u8);
			let alice_loan_state = UserLoanState::<TestRuntime>::build_user_loan_state(&ALICE).unwrap();
			check_user_loan_state(
				&alice_loan_state,
				Some(ForgivableComplete),
				<Vec<(CurrencyId, Balance)>>::new(), //TODO: fix after implementation math
				<Vec<(CurrencyId, Balance)>>::new(), //TODO: fix after implementation math
			);
		});
}

// Bob   supply: ---- DOT; ---- ETH; 5000 BTC - for liquidity in the BTC pool.
// Alice supply: 3000 DOT; 6500 ETH; 1500 BTC. - all enabled as collateral
// Alice borrow: 2000 DOT; 4000 ETH; 4600 BTC.
// Note: 	prices for all assets set equal $1.
//			partial_liquidation_min_sum = $10_000.
// alice_total_supply = $11_000, alice_total_collateral = $9900, alice_total_borrow = $10_600.
// seize=$11_130 > supply=$11_000 && borrow=$11_130>min_sum=$10_000 => forgivable.
#[test]
fn forgivable_liquidation_greater_min_sum() {
	ExtBuilder::default()
		.set_liquidation_fees(vec![
			(DOT, Rate::saturating_from_rational(5, 100)),
			(ETH, Rate::saturating_from_rational(5, 100)),
			(BTC, Rate::saturating_from_rational(5, 100)),
		])
		.deposit_underlying(BOB, BTC, dollars(5000))
		.deposit_underlying(ALICE, DOT, dollars(3000))
		.deposit_underlying(ALICE, ETH, dollars(6500))
		.deposit_underlying(ALICE, BTC, dollars(1500))
		.enable_as_collateral(ALICE, DOT)
		.enable_as_collateral(ALICE, ETH)
		.enable_as_collateral(ALICE, BTC)
		.borrow_underlying(ALICE, DOT, dollars(2000))
		.borrow_underlying(ALICE, ETH, dollars(4000))
		.borrow_underlying(ALICE, BTC, dollars(4600))
		.merge_duplicates()
		.build()
		.execute_with(|| {
			// alice_liquidation_attempts == 0:
			let alice_loan_state = UserLoanState::<TestRuntime>::build_user_loan_state(&ALICE).unwrap();
			assert_eq!(TestRiskManager::get_user_liquidation_attempts(&ALICE), 0_u8);
			check_user_loan_state(
				&alice_loan_state,
				Some(ForgivableComplete),
				<Vec<(CurrencyId, Balance)>>::new(), //TODO: fix after implementation math
				<Vec<(CurrencyId, Balance)>>::new(), //TODO: fix after implementation math
			);

			set_user_liquidation_attempts_to(1);

			// alice_liquidation_attempts == 1:
			assert_eq!(TestRiskManager::get_user_liquidation_attempts(&ALICE), 1_u8);
			let alice_loan_state = UserLoanState::<TestRuntime>::build_user_loan_state(&ALICE).unwrap();
			check_user_loan_state(
				&alice_loan_state,
				Some(ForgivableComplete),
				<Vec<(CurrencyId, Balance)>>::new(), //TODO: fix after implementation math
				<Vec<(CurrencyId, Balance)>>::new(), //TODO: fix after implementation math
			);
		});
}

// Alice supply: 300 DOT; 650 ETH; 50 BTC. - all enabled as collateral
// Alice borrow: 200 DOT; 400 ETH.
// Note: 	prices for all assets set equal $1.
//			partial liquidation min sum = $10_000.
// alice_total_supply = $1000, alice_total_collateral = $900, alice_total_borrow = $600.
// borrow=$600 < collateral=$900 => solvent loan.
#[test]
fn solvent_loan() {
	ExtBuilder::default()
		.set_liquidation_fees(vec![
			(DOT, Rate::saturating_from_rational(5, 100)),
			(ETH, Rate::saturating_from_rational(5, 100)),
			(BTC, Rate::saturating_from_rational(5, 100)),
		])
		.deposit_underlying(ALICE, DOT, dollars(300))
		.deposit_underlying(ALICE, ETH, dollars(650))
		.deposit_underlying(ALICE, BTC, dollars(50))
		.enable_as_collateral(ALICE, DOT)
		.enable_as_collateral(ALICE, ETH)
		.enable_as_collateral(ALICE, BTC)
		.borrow_underlying(ALICE, DOT, dollars(200))
		.borrow_underlying(ALICE, ETH, dollars(400))
		.merge_duplicates()
		.build()
		.execute_with(|| {
			assert_noop!(
				UserLoanState::<TestRuntime>::build_user_loan_state(&ALICE),
				Error::<TestRuntime>::SolventUserLoan
			);
		});
}

// Bob   supply: --- DOT; --- ETH; 500 BTC - for liquidity in the BTC pool.
// Alice supply: 300 DOT; 650 ETH; 50 BTC. - all enabled as collateral
// Alice borrow: 200 DOT; 400 ETH; 310 BTC.
// Note: 	prices for all assets set equal $1.
// alice_total_supply = $1000, alice_total_collateral = $900, alice_total_borrow = $910.
#[test]
fn partial_and_complete_liquidation() {
	ExtBuilder::default()
		.set_liquidation_fees(vec![
			(DOT, Rate::saturating_from_rational(5, 100)),
			(ETH, Rate::saturating_from_rational(5, 100)),
			(BTC, Rate::saturating_from_rational(5, 100)),
		])
		.deposit_underlying(BOB, BTC, dollars(500))
		.deposit_underlying(ALICE, DOT, dollars(300))
		.deposit_underlying(ALICE, ETH, dollars(650))
		.deposit_underlying(ALICE, BTC, dollars(50))
		.enable_as_collateral(ALICE, DOT)
		.enable_as_collateral(ALICE, ETH)
		.enable_as_collateral(ALICE, BTC)
		.borrow_underlying(ALICE, DOT, dollars(200))
		.borrow_underlying(ALICE, ETH, dollars(400))
		.borrow_underlying(ALICE, BTC, dollars(310))
		.merge_duplicates()
		.build()
		.execute_with(|| {
			// borrow=$910<min_sum=$10_000, liquidation_attempts=0, => complete.
			let alice_complete = UserLoanState::<TestRuntime>::build_user_loan_state(&ALICE).unwrap();
			check_user_loan_state(
				&alice_complete,
				Some(Complete),
				<Vec<(CurrencyId, Balance)>>::new(), //TODO: fix after implementation math
				<Vec<(CurrencyId, Balance)>>::new(), //TODO: fix after implementation math
			);

			// set partial_liquidation_min_sum == $500
			MinSumMock::set_partial_liquidation_min_sum(dollars(500));
			// borrow=$910>min_sum=$500, liquidation_attempts=0, => partial.
			let alice_partial = UserLoanState::<TestRuntime>::build_user_loan_state(&ALICE).unwrap();
			check_user_loan_state(
				&alice_partial,
				Some(Partial),
				<Vec<(CurrencyId, Balance)>>::new(), //TODO: fix after implementation math
				<Vec<(CurrencyId, Balance)>>::new(), //TODO: fix after implementation math
			);

			set_user_liquidation_attempts_to(3);

			let alice_complete = UserLoanState::<TestRuntime>::build_user_loan_state(&ALICE).unwrap();
			// alice_liquidation_attempts == 3:
			assert_eq!(TestRiskManager::get_user_liquidation_attempts(&ALICE), 3_u8);
			// borrow=$910>min_sum=$500, liquidation_attempts=3, => complete.
			check_user_loan_state(
				&alice_complete,
				Some(Complete),
				<Vec<(CurrencyId, Balance)>>::new(), //TODO: fix after implementation math
				<Vec<(CurrencyId, Balance)>>::new(), //TODO: fix after implementation math
			);

			// set partial_liquidation_min_sum == $10_000
			MinSumMock::set_partial_liquidation_min_sum(dollars(10_000));
			let alice_complete = UserLoanState::<TestRuntime>::build_user_loan_state(&ALICE).unwrap();
			// alice_liquidation_attempts == 3:
			assert_eq!(TestRiskManager::get_user_liquidation_attempts(&ALICE), 3_u8);
			// borrow=$910<min_sum=$10_000, liquidation_attempts=3, => complete.
			check_user_loan_state(
				&alice_complete,
				Some(Complete),
				<Vec<(CurrencyId, Balance)>>::new(), //TODO: fix after implementation math
				<Vec<(CurrencyId, Balance)>>::new(), //TODO: fix after implementation math
			);
		});
}
