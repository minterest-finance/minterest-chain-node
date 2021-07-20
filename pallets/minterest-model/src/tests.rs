//! Tests for the minterest-model pallet.

use super::*;
use mock::{Event, *};

use frame_support::{assert_noop, assert_ok};
use sp_runtime::DispatchError::BadOrigin;

fn multiplier_per_block_equal_max_value() -> MinterestModelData {
	MinterestModelData {
		kink: Rate::saturating_from_rational(12, 10),
		base_rate_per_block: Rate::from_inner(0),
		multiplier_per_block: Rate::from_inner(u128::MAX),
		jump_multiplier_per_block: Rate::saturating_from_rational(207, 1_000_000_000), // 1.09 PerYear
	}
}

fn base_rate_per_block_equal_max_value() -> MinterestModelData {
	MinterestModelData {
		kink: Rate::saturating_from_rational(12, 10),
		base_rate_per_block: Rate::from_inner(u128::MAX),
		multiplier_per_block: Rate::saturating_from_rational(9, 1_000_000_000), // 0.047304 PerYear
		jump_multiplier_per_block: Rate::saturating_from_rational(207, 1_000_000_000), // 1.09 PerYear
	}
}

#[test]
fn set_pool_base_rate_should_work() {
	test_externalities().execute_with(|| {
		// Set Base rate per block equal to 2.0: (10_512_000 / 1) / 5_256_000
		assert_ok!(TestMinterestModel::set_pool_base_rate(
			alice_origin(),
			DOT,
			Rate::saturating_from_rational(10_512_000, 1)
		));
		assert_eq!(
			TestMinterestModel::minterest_model_data_storage(DOT).base_rate_per_block,
			Rate::saturating_from_rational(2, 1)
		);
		let expected_event = Event::TestMinterestModel(crate::Event::BaseRatePerBlockChanged);
		assert!(System::events().iter().any(|record| record.event == expected_event));

		// Can be set to 0.0: (0 / 10) / 5_256_000
		assert_ok!(TestMinterestModel::set_pool_base_rate(
			alice_origin(),
			DOT,
			Rate::zero()
		));
		assert_eq!(
			TestMinterestModel::minterest_model_data_storage(DOT).base_rate_per_block,
			Rate::zero()
		);

		// ALICE set Base rate per block equal to 0,000000009: (47_304 / 1_000_000) / 5_256_000
		assert_ok!(TestMinterestModel::set_pool_base_rate(
			alice_origin(),
			DOT,
			Rate::saturating_from_rational(47304, 1_000_000)
		));
		assert_eq!(
			TestMinterestModel::minterest_model_data_storage(DOT).base_rate_per_block,
			Rate::from_inner(9_000_000_000)
		);

		// Base rate per block cannot be set to 0 at the same time as Multiplier per block.
		assert_ok!(TestMinterestModel::set_pool_multiplier(
			alice_origin(),
			DOT,
			Rate::zero()
		));
		assert_noop!(
			TestMinterestModel::set_pool_base_rate(alice_origin(), DOT, Rate::zero()),
			Error::<Test>::BaseRatePerBlockCannotBeZero
		);

		// The dispatch origin of this call must be Root or half MinterestCouncil.
		assert_noop!(
			TestMinterestModel::set_pool_base_rate(bob_origin(), DOT, Rate::from_inner(2)),
			BadOrigin
		);

		// MDOT is wrong CurrencyId for underlying assets.
		assert_noop!(
			TestMinterestModel::set_pool_base_rate(alice_origin(), MDOT, Rate::from_inner(2)),
			Error::<Test>::NotValidUnderlyingAssetId
		);
	});
}

#[test]
fn set_pool_multiplier_should_work() {
	test_externalities().execute_with(|| {
		// Set Multiplier per block equal to 2.0: (10_512_000 / 1) / 5_256_000
		assert_ok!(TestMinterestModel::set_pool_multiplier(
			alice_origin(),
			DOT,
			Rate::saturating_from_rational(10_512_000, 1)
		));
		assert_eq!(
			TestMinterestModel::minterest_model_data_storage(DOT).multiplier_per_block,
			Rate::saturating_from_rational(2, 1)
		);
		let expected_event = Event::TestMinterestModel(crate::Event::MultiplierPerBlockChanged);
		assert!(System::events().iter().any(|record| record.event == expected_event));

		// Can be set to 0.0 if Base rate per block grater than zero: (0 / 10) / 5_256_000
		assert_ok!(TestMinterestModel::set_pool_base_rate(alice_origin(), DOT, Rate::one()));
		assert_ok!(TestMinterestModel::set_pool_multiplier(
			alice_origin(),
			DOT,
			Rate::zero()
		));
		assert_eq!(
			TestMinterestModel::minterest_model_data_storage(DOT).multiplier_per_block,
			Rate::zero()
		);

		// Alice set Multiplier per block equal to 0,000_000_009: (47_304 / 1_000_000) / 5_256_000
		assert_ok!(TestMinterestModel::set_pool_multiplier(
			alice_origin(),
			DOT,
			Rate::saturating_from_rational(47304, 1_000_000)
		));
		assert_eq!(
			TestMinterestModel::minterest_model_data_storage(DOT).multiplier_per_block,
			Rate::from_inner(9_000_000_000)
		);

		//  Multiplier per block cannot be set to 0 at the same time as Base rate per block.
		assert_ok!(TestMinterestModel::set_pool_base_rate(
			alice_origin(),
			DOT,
			Rate::zero()
		));
		assert_noop!(
			TestMinterestModel::set_pool_multiplier(alice_origin(), DOT, Rate::zero()),
			Error::<Test>::MultiplierPerBlockCannotBeZero
		);

		// The dispatch origin of this call must be Root or half MinterestCouncil.
		assert_noop!(
			TestMinterestModel::set_pool_multiplier(bob_origin(), DOT, Rate::from_inner(2)),
			BadOrigin
		);

		// MDOT is wrong CurrencyId for underlying assets.
		assert_noop!(
			TestMinterestModel::set_pool_base_rate(alice_origin(), MDOT, Rate::from_inner(2)),
			Error::<Test>::NotValidUnderlyingAssetId
		);
	});
}

#[test]
fn set_jump_multiplier_should_work() {
	test_externalities().execute_with(|| {
		// Set Jump multiplier per block equal to 2.0: (10_512_000 / 1) / 5_256_000
		assert_ok!(TestMinterestModel::set_jump_multiplier(
			alice_origin(),
			DOT,
			Rate::saturating_from_rational(10_512_000, 1)
		));
		assert_eq!(
			TestMinterestModel::minterest_model_data_storage(DOT).jump_multiplier_per_block,
			Rate::saturating_from_rational(2, 1)
		);
		let expected_event = Event::TestMinterestModel(crate::Event::JumpMultiplierPerBlockChanged);
		assert!(System::events().iter().any(|record| record.event == expected_event));

		// Can be set to 0.0: (0 / 10) / 5_256_000
		assert_ok!(TestMinterestModel::set_jump_multiplier(
			alice_origin(),
			DOT,
			Rate::zero()
		));
		assert_eq!(
			TestMinterestModel::minterest_model_data_storage(DOT).jump_multiplier_per_block,
			Rate::zero()
		);

		// Alice set Jump multiplier per block equal to 0,000_000_009: (47_304 / 1_000_000) / 5_256_000
		assert_ok!(TestMinterestModel::set_jump_multiplier(
			alice_origin(),
			DOT,
			Rate::saturating_from_rational(47_304, 1_000_000)
		));
		assert_eq!(
			TestMinterestModel::minterest_model_data_storage(DOT).jump_multiplier_per_block,
			Rate::from_inner(9_000_000_000)
		);

		// The dispatch origin of this call must be Root or half MinterestCouncil.
		assert_noop!(
			TestMinterestModel::set_jump_multiplier(bob_origin(), DOT, Rate::from_inner(2)),
			BadOrigin
		);

		// MDOT is wrong CurrencyId for underlying assets.
		assert_noop!(
			TestMinterestModel::set_pool_base_rate(alice_origin(), MDOT, Rate::from_inner(2)),
			Error::<Test>::NotValidUnderlyingAssetId
		);
	});
}

#[test]
fn set_pool_kink_should_work() {
	test_externalities().execute_with(|| {
		assert_ok!(TestMinterestModel::set_pool_kink(
			alice_origin(),
			DOT,
			Rate::saturating_from_rational(8, 10)
		));
		assert_eq!(
			TestMinterestModel::minterest_model_data_storage(DOT).kink,
			Rate::saturating_from_rational(8, 10)
		);
		let expected_event = Event::TestMinterestModel(crate::Event::KinkChanged);
		assert!(System::events().iter().any(|record| record.event == expected_event));

		// The dispatch origin of this call must be Root or half MinterestCouncil.
		assert_noop!(
			TestMinterestModel::set_pool_kink(bob_origin(), DOT, Rate::saturating_from_rational(8, 10)),
			BadOrigin
		);

		// MDOT is wrong CurrencyId for underlying assets.
		assert_noop!(
			TestMinterestModel::set_pool_kink(alice_origin(), MDOT, Rate::saturating_from_rational(8, 10)),
			Error::<Test>::NotValidUnderlyingAssetId
		);

		// Parameter `kink` cannot be larger than one.
		assert_noop!(
			TestMinterestModel::set_pool_kink(alice_origin(), DOT, Rate::saturating_from_rational(11, 10)),
			Error::<Test>::KinkCannotBeMoreThanOne
		);
	});
}

#[test]
fn calculate_pool_borrow_interest_rate_should_work() {
	test_externalities().execute_with(|| {
		// Utilization rate less or equal to kink:
		// utilization_rate = 0.42
		// borrow_interest_rate = 0,42 * multiplier_per_block + base_rate_per_block
		assert_eq!(
			TestMinterestModel::calculate_pool_borrow_interest_rate(DOT, Rate::saturating_from_rational(42, 100)),
			Ok(Rate::from_inner(3_780_000_000))
		);

		// Utilization rate larger than kink:
		// utilization_rate = 0.9
		// borrow_interest_rate = 0.9 * 0.8 * jump_multiplier_per_block +
		// + (0.8 * multiplier_per_block) + base_rate_per_block
		assert_eq!(
			TestMinterestModel::calculate_pool_borrow_interest_rate(DOT, Rate::saturating_from_rational(9, 10)),
			Ok(Rate::from_inner(156_240_000_000))
		);
	});
}

#[test]
fn calculate_pool_borrow_interest_rate_fails_if_overflow_kink_mul_multiplier() {
	test_externalities().execute_with(|| {
		let minterest_model_data = multiplier_per_block_equal_max_value();
		<MinterestModelDataStorage<Test>>::insert(KSM, minterest_model_data.clone());
		// utilization_rate > kink.
		// Overflow in calculation: kink * multiplier_per_block = 1.01 * max_value()
		assert_noop!(
			TestMinterestModel::calculate_pool_borrow_interest_rate(KSM, Rate::saturating_from_rational(101, 100)),
			Error::<Test>::BorrowRateCalculationError
		);
	});
}

#[test]
fn calculate_pool_borrow_interest_rate_fails_if_overflow_add_base_rate_per_block() {
	test_externalities().execute_with(|| {
		let minterest_model_data = base_rate_per_block_equal_max_value();
		<MinterestModelDataStorage<Test>>::insert(KSM, minterest_model_data.clone());
		// utilization_rate > kink.
		// Overflow in calculation: kink_mul_multiplier + base_rate_per_block = ... + max_value()
		assert_noop!(
			TestMinterestModel::calculate_pool_borrow_interest_rate(KSM, Rate::saturating_from_rational(9, 10)),
			Error::<Test>::BorrowRateCalculationError
		);
	});
}
