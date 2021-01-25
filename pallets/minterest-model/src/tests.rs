//! Tests for the minterest-model pallet.

use super::*;
use mock::*;

use frame_support::{assert_noop, assert_ok};

fn multiplier_per_block_equal_max_value() -> MinterestModelData {
	MinterestModelData {
		kink: Rate::saturating_from_rational(12, 10),
		base_rate_per_block: Rate::from_inner(0),
		multiplier_per_block: Rate::from_inner(u128::max_value()),
		jump_multiplier_per_block: Rate::saturating_from_rational(207, 1_000_000_000), // 1.09 PerYear
	}
}

fn base_rate_per_block_equal_max_value() -> MinterestModelData {
	MinterestModelData {
		kink: Rate::saturating_from_rational(12, 10),
		base_rate_per_block: Rate::from_inner(u128::max_value()),
		multiplier_per_block: Rate::saturating_from_rational(9, 1_000_000_000), // 0.047304 PerYear
		jump_multiplier_per_block: Rate::saturating_from_rational(207, 1_000_000_000), // 1.09 PerYear
	}
}

#[test]
fn set_base_rate_per_block_should_work() {
	new_test_ext().execute_with(|| {
		// Can be set to 0.0
		assert_ok!(TestMinterestModel::set_base_rate_per_block(
			alice(),
			CurrencyId::DOT,
			0,
			1
		));
		assert_eq!(
			TestMinterestModel::minterest_model_dates(CurrencyId::DOT).base_rate_per_block,
			Rate::from_inner(0)
		);
		let expected_event = TestEvent::minterest_model(Event::BaseRatePerBlockHasChanged);
		assert!(System::events().iter().any(|record| record.event == expected_event));

		// ALICE set Baser rate per block equal 2.0
		assert_ok!(TestMinterestModel::set_base_rate_per_block(
			alice(),
			CurrencyId::DOT,
			20,
			10
		));
		assert_eq!(
			TestMinterestModel::minterest_model_dates(CurrencyId::DOT).base_rate_per_block,
			Rate::saturating_from_rational(2_000_000_000_000_000_000u128, BLOCKS_PER_YEAR)
		);
		let expected_event = TestEvent::minterest_model(Event::BaseRatePerBlockHasChanged);
		assert!(System::events().iter().any(|record| record.event == expected_event));

		// Base rate per block cannot be set to 0 at the same time as Multiplier per block.
		assert_ok!(TestMinterestModel::set_multiplier_per_block(
			alice(),
			CurrencyId::DOT,
			0,
			1
		));
		assert_noop!(
			TestMinterestModel::set_base_rate_per_block(alice(), CurrencyId::DOT, 0, 1),
			Error::<Test>::BaseRatePerBlockCannotBeZero
		);

		// Overflow in calculation: 20 / 0
		assert_noop!(
			TestMinterestModel::set_base_rate_per_block(alice(), CurrencyId::DOT, 20, 0),
			Error::<Test>::NumOverflow
		);

		// The dispatch origin of this call must be Administrator.
		assert_noop!(
			TestMinterestModel::set_base_rate_per_block(bob(), CurrencyId::DOT, 20, 10),
			Error::<Test>::RequireAdmin
		);

		// MDOT is wrong CurrencyId for underlying assets.
		assert_noop!(
			TestMinterestModel::set_base_rate_per_block(alice(), CurrencyId::MDOT, 20, 10),
			Error::<Test>::NotValidUnderlyingAssetId
		);
	});
}

#[test]
fn set_multiplier_per_block_should_work() {
	new_test_ext().execute_with(|| {
		// Set Base rate per block equal 2.0
		assert_ok!(TestMinterestModel::set_base_rate_per_block(
			alice(),
			CurrencyId::DOT,
			20,
			10
		));

		// Can be set to 0.0
		assert_ok!(TestMinterestModel::set_multiplier_per_block(
			alice(),
			CurrencyId::DOT,
			0,
			10
		));
		assert_eq!(
			TestMinterestModel::minterest_model_dates(CurrencyId::DOT).multiplier_per_block,
			Rate::from_inner(0)
		);
		let expected_event = TestEvent::minterest_model(Event::MultiplierPerBlockHasChanged);
		assert!(System::events().iter().any(|record| record.event == expected_event));

		// Alice set Multiplier per block equal 2.0 / 5_256_000
		assert_ok!(TestMinterestModel::set_multiplier_per_block(
			alice(),
			CurrencyId::DOT,
			20,
			10
		));
		let expected_event = TestEvent::minterest_model(Event::MultiplierPerBlockHasChanged);
		assert!(System::events().iter().any(|record| record.event == expected_event));
		assert_eq!(
			TestMinterestModel::minterest_model_dates(CurrencyId::DOT).multiplier_per_block,
			Rate::saturating_from_rational(2_000_000_000_000_000_000u128, BLOCKS_PER_YEAR)
		);

		//  Multiplier per block cannot be set to 0 at the same time as Base rate per block.
		assert_ok!(TestMinterestModel::set_base_rate_per_block(
			alice(),
			CurrencyId::DOT,
			0,
			1
		));
		assert_noop!(
			TestMinterestModel::set_multiplier_per_block(alice(), CurrencyId::DOT, 0, 1),
			Error::<Test>::MultiplierPerBlockCannotBeZero
		);

		// Overflow in calculation: 20 / 0
		assert_noop!(
			TestMinterestModel::set_multiplier_per_block(alice(), CurrencyId::DOT, 20, 0),
			Error::<Test>::NumOverflow
		);

		// The dispatch origin of this call must be Administrator.
		assert_noop!(
			TestMinterestModel::set_multiplier_per_block(bob(), CurrencyId::DOT, 20, 10),
			Error::<Test>::RequireAdmin
		);

		// MDOT is wrong CurrencyId for underlying assets.
		assert_noop!(
			TestMinterestModel::set_base_rate_per_block(alice(), CurrencyId::MDOT, 20, 10),
			Error::<Test>::NotValidUnderlyingAssetId
		);
	});
}

#[test]
fn set_jump_multiplier_per_block_should_work() {
	new_test_ext().execute_with(|| {
		assert_ok!(TestMinterestModel::set_jump_multiplier_per_block(
			alice(),
			CurrencyId::DOT,
			20,
			10
		));
		assert_eq!(
			TestMinterestModel::minterest_model_dates(CurrencyId::DOT).jump_multiplier_per_block,
			Rate::saturating_from_rational(2_000_000_000_000_000_000u128, BLOCKS_PER_YEAR)
		);
		let expected_event = TestEvent::minterest_model(Event::JumpMultiplierPerBlockHasChanged);
		assert!(System::events().iter().any(|record| record.event == expected_event));

		// Overflow in calculation: 20 / 0
		assert_noop!(
			TestMinterestModel::set_jump_multiplier_per_block(alice(), CurrencyId::DOT, 20, 0),
			Error::<Test>::NumOverflow
		);

		// The dispatch origin of this call must be Administrator.
		assert_noop!(
			TestMinterestModel::set_jump_multiplier_per_block(bob(), CurrencyId::DOT, 20, 10),
			Error::<Test>::RequireAdmin
		);

		// MDOT is wrong CurrencyId for underlying assets.
		assert_noop!(
			TestMinterestModel::set_base_rate_per_block(alice(), CurrencyId::MDOT, 20, 10),
			Error::<Test>::NotValidUnderlyingAssetId
		);
	});
}

#[test]
fn calculate_borrow_interest_rate_should_work() {
	new_test_ext().execute_with(|| {
		// Utilization rate less or equal than kink:
		// utilization_rate = 0.42
		// borrow_interest_rate = 0,42 * multiplier_per_block + base_rate_per_block
		assert_eq!(
			TestMinterestModel::calculate_borrow_interest_rate(
				CurrencyId::DOT,
				Rate::saturating_from_rational(42, 100)
			),
			Ok(Rate::from_inner(3_780_000_000))
		);

		// Utilization rate larger than kink:
		// utilization_rate = 0.9
		// borrow_interest_rate = 0.9 * 0.8 * jump_multiplier_per_block +
		// + (0.8 * multiplier_per_block) + base_rate_per_block
		assert_eq!(
			TestMinterestModel::calculate_borrow_interest_rate(CurrencyId::DOT, Rate::saturating_from_rational(9, 10)),
			Ok(Rate::from_inner(156_240_000_000))
		);
	});
}

#[test]
fn calculate_borrow_interest_rate_fails_if_overflow_kink_mul_multiplier() {
	new_test_ext().execute_with(|| {
		let minterest_model_data = multiplier_per_block_equal_max_value();
		<MinterestModelDates>::insert(CurrencyId::KSM, minterest_model_data.clone());
		// utilization_rate > kink.
		// Overflow in calculation: kink * multiplier_per_block = 1.01 * max_value()
		assert_noop!(
			TestMinterestModel::calculate_borrow_interest_rate(
				CurrencyId::KSM,
				Rate::saturating_from_rational(101, 100)
			),
			Error::<Test>::NumOverflow
		);
	});
}

#[test]
fn calculate_borrow_interest_rate_fails_if_overflow_add_base_rate_per_block() {
	new_test_ext().execute_with(|| {
		let minterest_model_data = base_rate_per_block_equal_max_value();
		<MinterestModelDates>::insert(CurrencyId::KSM, minterest_model_data.clone());
		// utilization_rate > kink.
		// Overflow in calculation: kink_mul_multiplier + base_rate_per_block = ... + max_value()
		assert_noop!(
			TestMinterestModel::calculate_borrow_interest_rate(CurrencyId::KSM, Rate::saturating_from_rational(9, 10)),
			Error::<Test>::NumOverflow
		);
	});
}
