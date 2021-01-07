use super::*;
use mock::*;

use frame_support::{assert_err, assert_noop, assert_ok, error::BadOrigin};

#[test]
fn accrue_interest_should_work() {
	ExtBuilder::default()
		.set_btc_and_dot_pool_mock()
		.borrow_interest_rate_equal_7_200_000_000()
		.build()
		.execute_with(|| {
			System::set_block_number(1);
			assert_ok!(Controller::accrue_interest_rate(CurrencyId::DOT));
			assert_eq!(Controller::controller_dates(CurrencyId::DOT).timestamp, 1);

			assert_noop!(
				Controller::accrue_interest_rate(CurrencyId::BTC),
				Error::<Runtime>::OperationsLocked
			);

			assert_eq!(TestPools::pools(CurrencyId::DOT).total_insurance, 57_600_000_000);
			assert_eq!(
				Controller::controller_dates(CurrencyId::DOT).borrow_rate,
				Rate::saturating_from_rational(72u128, 10_000_000_000u128)
			);

			assert_eq!(
				TestPools::pools(CurrencyId::DOT).total_borrowed,
				80_000_000_576_000_000_000
			);
			assert_eq!(
				TestPools::pools(CurrencyId::DOT).borrow_index,
				Rate::from_inner(1_000_000_007_200_000_000)
			);
		});
}

#[test]
fn accrue_interest_should_not_work() {
	ExtBuilder::default()
		.set_btc_and_dot_pool_mock()
		.borrow_interest_rate_equal_7_200_000_000()
		.build()
		.execute_with(|| {
			System::set_block_number(1);
			assert_ok!(Controller::accrue_interest_rate(CurrencyId::DOT));
			assert_eq!(Controller::controller_dates(CurrencyId::DOT).timestamp, 1);

			System::set_block_number(20);
			assert_noop!(
				Controller::accrue_interest_rate(CurrencyId::DOT),
				Error::<Runtime>::BorrowRateIsTooHight
			);

			assert_ok!(Controller::set_max_borrow_rate(Origin::root(), CurrencyId::DOT, 2, 1));
			assert_ok!(Controller::accrue_interest_rate(CurrencyId::DOT));
		});
}

#[test]
fn convert_to_wrapped_should_work() {
	ExtBuilder::default()
		.exchange_rate_less_than_one()
		.build()
		.execute_with(|| {
			assert_ok!(Currencies::transfer(
				Origin::signed(ALICE),
				TestPools::pools_account_id(),
				CurrencyId::DOT,
				100
			));
			assert_ok!(Controller::convert_to_wrapped(CurrencyId::DOT, 10));
			assert_eq!(Controller::convert_to_wrapped(CurrencyId::DOT, 10), Ok(10));
			assert_err!(
				Controller::convert_to_wrapped(CurrencyId::BTC, Balance::max_value()),
				Error::<Runtime>::NumOverflow
			);
		});
}

#[test]
fn calculate_interest_rate_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		assert_ok!(Controller::calculate_interest_rate(CurrencyId::DOT));
		assert_eq!(
			Controller::calculate_interest_rate(CurrencyId::DOT),
			Ok(Rate::saturating_from_rational(1, 1))
		);
	});
}

#[test]
fn convert_from_wrapped_should_work() {
	ExtBuilder::default()
		.exchange_rate_greater_than_one()
		.build()
		.execute_with(|| {
			assert_ok!(Currencies::transfer(
				Origin::signed(ALICE),
				TestPools::pools_account_id(),
				CurrencyId::DOT,
				100
			));
			assert_ok!(Currencies::transfer(
				Origin::signed(ALICE),
				TestPools::pools_account_id(),
				CurrencyId::BTC,
				100
			));
			assert_ok!(Controller::convert_from_wrapped(CurrencyId::MDOT, 10));
			assert_eq!(Controller::convert_from_wrapped(CurrencyId::MDOT, 10), Ok(10));
			assert_err!(
				Controller::convert_from_wrapped(CurrencyId::MBTC, Balance::max_value()),
				Error::<Runtime>::NumOverflow
			);
		});
}

#[test]
fn calculate_exchange_rate_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		assert_ok!(Controller::calculate_exchange_rate(102, 100, 2, 20));
		assert_eq!(
			Controller::calculate_exchange_rate(102, 100, 2, 20),
			Ok(Rate::saturating_from_rational(12, 10))
		);
		assert_eq!(
			Controller::calculate_exchange_rate(102, 0, 2, 0),
			Ok(Rate::saturating_from_rational(1, 1))
		)
	});
}

#[test]
fn get_exchange_rate_should_work() {
	ExtBuilder::default()
		.one_hundred_dots_for_alice()
		.build()
		.execute_with(|| {
			assert_ok!(Currencies::transfer(
				Origin::signed(ALICE),
				TestPools::pools_account_id(),
				CurrencyId::DOT,
				100
			));
			assert_ok!(Controller::get_exchange_rate(CurrencyId::DOT));
			assert_eq!(
				Controller::get_exchange_rate(CurrencyId::DOT),
				Ok(Rate::saturating_from_rational(1, 1))
			);
			assert_eq!(
				TestPools::pools(&CurrencyId::DOT).current_exchange_rate,
				Rate::saturating_from_rational(1, 1)
			);
		});
}

#[test]
fn calculate_borrow_interest_rate_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		assert_ok!(Controller::calculate_borrow_interest_rate(CurrencyId::DOT, 102, 20, 2));

		// utilization rate less than kink
		assert_eq!(
			Controller::calculate_borrow_interest_rate(CurrencyId::DOT, 37, 70, 7),
			Ok(Rate::saturating_from_rational(63u128, 10_000_000_000u128))
		);

		// utilization rate larger or equal than kink
		assert_eq!(
			Controller::calculate_borrow_interest_rate(CurrencyId::DOT, 18, 90, 8),
			Ok(Rate::saturating_from_rational(14400000072u128, 10_000_000_000u128))
		);
	});
}

#[test]
fn calculate_block_delta_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		assert_eq!(Controller::calculate_block_delta(10, 5), Ok(5));
		assert_noop!(Controller::calculate_block_delta(5, 10), Error::<Runtime>::NumOverflow);
	});
}

#[test]
fn calculate_interest_factor_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		assert_ok!(Controller::calculate_interest_factor(
			Rate::saturating_from_rational(1, 10),
			&10
		));
		assert_eq!(
			Controller::calculate_interest_factor(Rate::saturating_from_rational(1, 10), &10),
			Ok(Rate::from_inner(1_000_000_000_000_000_000))
		);
	});
}

#[test]
fn calculate_interest_accumulated_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		assert_ok!(Controller::calculate_interest_accumulated(
			Rate::saturating_from_rational(1, 1),
			TestPools::get_pool_available_liquidity(CurrencyId::DOT)
		));
		assert_eq!(
			Controller::calculate_interest_accumulated(
				Rate::saturating_from_rational(0, 1),
				TestPools::get_pool_available_liquidity(CurrencyId::DOT)
			),
			Ok(0)
		);
		assert_eq!(
			Controller::calculate_interest_accumulated(
				Rate::saturating_from_rational(3, 100), // eq 0.03 == 3%
				200
			),
			Ok(6)
		);
		assert_noop!(
			Controller::calculate_interest_accumulated(Rate::saturating_from_rational(11, 10), Balance::max_value()),
			Error::<Runtime>::NumOverflow
		);
	});
}

#[test]
fn calculate_new_total_borrow_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		assert_ok!(Controller::calculate_new_total_borrow(100, 100));
		assert_eq!(Controller::calculate_new_total_borrow(0, 100), Ok(100));
		assert_noop!(
			Controller::calculate_new_total_borrow(1, Balance::max_value()),
			Error::<Runtime>::NumOverflow
		);
	});
}

#[test]
fn calculate_new_total_insurance_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		assert_ok!(Controller::calculate_new_total_insurance(
			100,
			Rate::saturating_from_rational(1, 1),
			100
		));
		assert_eq!(
			Controller::calculate_new_total_insurance(100, Rate::saturating_from_rational(0, 1), 250),
			Ok(250)
		);
		assert_noop!(
			Controller::calculate_new_total_insurance(Balance::max_value(), Rate::saturating_from_rational(11, 10), 1),
			Error::<Runtime>::NumOverflow
		);
		assert_noop!(
			Controller::calculate_new_total_insurance(Balance::max_value(), Rate::saturating_from_rational(1, 1), 1),
			Error::<Runtime>::NumOverflow
		);
	});
}

#[test]
fn get_wrapped_id_by_underlying_asset_id_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		assert_ok!(Controller::get_wrapped_id_by_underlying_asset_id(&CurrencyId::DOT));
		assert_eq!(
			Controller::get_wrapped_id_by_underlying_asset_id(&CurrencyId::DOT),
			Ok(CurrencyId::MDOT)
		);
		assert_noop!(
			Controller::get_wrapped_id_by_underlying_asset_id(&CurrencyId::MDOT),
			Error::<Runtime>::NotValidUnderlyingAssetId
		);
	});
}

#[test]
fn get_underlying_asset_id_by_wrapped_id_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		assert_ok!(Controller::get_underlying_asset_id_by_wrapped_id(&CurrencyId::MDOT));
		assert_eq!(
			Controller::get_underlying_asset_id_by_wrapped_id(&CurrencyId::MDOT),
			Ok(CurrencyId::DOT)
		);
		assert_noop!(
			Controller::get_underlying_asset_id_by_wrapped_id(&CurrencyId::DOT),
			Error::<Runtime>::NotValidWrappedTokenId
		);
	});
}

#[test]
fn borrow_balance_stored_with_zero_balance_should_work() {
	ExtBuilder::default()
		.set_alice_interest_index()
		.build()
		.execute_with(|| {
			assert_eq!(
				Controller::borrow_balance_stored(&ALICE, CurrencyId::DOT),
				Ok(Balance::zero())
			);
		});
}

#[test]
fn borrow_balance_stored_should_work() {
	ExtBuilder::default()
		.set_btc_and_dot_pool_mock()
		.set_alice_total_borrowed_and_interest_index()
		.build()
		.execute_with(|| {
			assert_eq!(Controller::borrow_balance_stored(&ALICE, CurrencyId::DOT), Ok(50));
		});
}

#[test]
fn calculate_utilization_rate_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		assert_ok!(Controller::calculate_utilization_rate(100, 0, 2));
		assert_eq!(Controller::calculate_utilization_rate(0, 0, 0), Ok(Rate::from_inner(0)));
		assert_eq!(
			Controller::calculate_utilization_rate(22, 80, 2),
			Ok(Rate::saturating_from_rational(8, 10))
		);

		assert_noop!(
			Controller::calculate_utilization_rate(Balance::max_value(), 80, 2),
			Error::<Runtime>::NumOverflow
		);
	});
}

#[test]
fn calculate_new_borrow_index_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		assert_ok!(Controller::calculate_new_borrow_index(
			Rate::saturating_from_rational(63u128, 10_000_000_000u128),
			Rate::saturating_from_rational(1, 1)
		));
		assert_eq!(
			Controller::calculate_new_borrow_index(
				Rate::saturating_from_rational(63u128, 10_000_000_000u128),
				Rate::saturating_from_rational(1, 1)
			),
			Ok(Rate::from_inner(1_000_000_006_300_000_000))
		);
	});
}

#[test]
fn mul_price_and_balance_add_to_prev_value_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		assert_eq!(
			Controller::mul_price_and_balance_add_to_prev_value(20, 20, Rate::saturating_from_rational(9, 10)),
			Ok(38)
		);
		assert_eq!(
			Controller::mul_price_and_balance_add_to_prev_value(
				120_000,
				85_000,
				Rate::saturating_from_rational(87, 100)
			),
			Ok(193950)
		);
	});
}

#[test]
fn get_hypothetical_account_liquidity_when_m_tokens_balance_is_zero_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		// if m_tokens_balance == 0 then return Ok(0, 0)
		assert_eq!(
			Controller::get_hypothetical_account_liquidity(&ALICE, CurrencyId::DOT, 5, 0),
			Ok((0, 0))
		);
	});
}

#[test]
fn get_hypothetical_account_liquidity_one_currency_from_redeem_should_work() {
	ExtBuilder::default().alice_deposit_60_dots().build().execute_with(|| {
		// Checking the function when called from redeem.
		assert_eq!(
			Controller::get_hypothetical_account_liquidity(&ALICE, CurrencyId::DOT, 5, 0),
			Ok((90, 0))
		);
		assert_eq!(
			Controller::get_hypothetical_account_liquidity(&ALICE, CurrencyId::DOT, 60, 0),
			Ok((0, 0))
		);
		assert_eq!(
			Controller::get_hypothetical_account_liquidity(&ALICE, CurrencyId::DOT, 200, 0),
			Ok((0, 231))
		);
	});
}

#[test]
fn get_hypothetical_account_liquidity_two_currencies_from_redeem_should_work() {
	ExtBuilder::default()
		.alice_deposit_60_dots()
		.alice_deposit_20_eth()
		.build()
		.execute_with(|| {
			// Checking the function when called from redeem.
			assert_eq!(
				Controller::get_hypothetical_account_liquidity(&ALICE, CurrencyId::ETH, 15, 0),
				Ok((105, 0))
			);
			assert_eq!(
				Controller::get_hypothetical_account_liquidity(&ALICE, CurrencyId::ETH, 80, 0),
				Ok((17, 0))
			);
			assert_eq!(
				Controller::get_hypothetical_account_liquidity(&ALICE, CurrencyId::ETH, 100, 0),
				Ok((0, 10))
			);
		});
}

#[test]
fn get_hypothetical_account_liquidity_two_currencies_from_borrow_should_work() {
	ExtBuilder::default()
		.alice_deposit_20_eth()
		.alice_deposit_60_dots()
		.alice_borrow_30_dot()
		.build()
		.execute_with(|| {
			// Checking the function when called from borrow.
			assert_eq!(
				Controller::get_hypothetical_account_liquidity(&ALICE, CurrencyId::DOT, 0, 30),
				Ok((89, 0))
			);
			assert_eq!(
				Controller::get_hypothetical_account_liquidity(&ALICE, CurrencyId::DOT, 0, 50),
				Ok((49, 0))
			);
			assert_eq!(
				Controller::get_hypothetical_account_liquidity(&ALICE, CurrencyId::DOT, 0, 100),
				Ok((0, 51))
			);
		});
}

#[test]
fn redeem_allowed_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		//TODO write tests after the function implementation.
	});
}

#[test]
fn borrow_allowed_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		//TODO write tests after the function implementation.
	});
}

#[test]
fn repay_allowed_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		//TODO write tests after the function implementation.
	});
}

/* ----------------------------------------------------------------------------------------- */

// Admin functions

#[test]
fn set_insurance_factor_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		assert_ok!(Controller::set_insurance_factor(
			Origin::root(),
			CurrencyId::DOT,
			20,
			10
		));
		assert_eq!(
			Controller::controller_dates(CurrencyId::DOT).insurance_factor,
			Rate::saturating_from_rational(20, 10)
		);
		assert_noop!(
			Controller::set_insurance_factor(Origin::root(), CurrencyId::DOT, 20, 0),
			Error::<Runtime>::NumOverflow
		);
		assert_noop!(
			Controller::set_insurance_factor(Origin::signed(ALICE), CurrencyId::DOT, 20, 10),
			BadOrigin
		);
		assert_noop!(
			Controller::set_insurance_factor(Origin::root(), CurrencyId::MDOT, 20, 10),
			Error::<Runtime>::NotValidUnderlyingAssetId
		);
	});
}

#[test]
fn set_max_borrow_rate_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		assert_ok!(Controller::set_max_borrow_rate(Origin::root(), CurrencyId::DOT, 20, 10));
		assert_eq!(
			Controller::controller_dates(CurrencyId::DOT).max_borrow_rate,
			Rate::saturating_from_rational(20, 10)
		);
		assert_noop!(
			Controller::set_max_borrow_rate(Origin::root(), CurrencyId::DOT, 20, 0),
			Error::<Runtime>::NumOverflow
		);
		assert_noop!(
			Controller::set_max_borrow_rate(Origin::signed(ALICE), CurrencyId::DOT, 20, 10),
			BadOrigin
		);
		assert_noop!(
			Controller::set_max_borrow_rate(Origin::root(), CurrencyId::MDOT, 20, 10),
			Error::<Runtime>::NotValidUnderlyingAssetId
		);
	});
}

#[test]
fn set_base_rate_per_block_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		assert_ok!(Controller::set_base_rate_per_block(
			Origin::root(),
			CurrencyId::DOT,
			20,
			10
		));
		assert_eq!(
			Controller::controller_dates(CurrencyId::DOT).base_rate_per_block,
			Rate::saturating_from_rational(2_000_000_000_000_000_000u128, BLOCKS_PER_YEAR)
		);
		assert_noop!(
			Controller::set_base_rate_per_block(Origin::root(), CurrencyId::DOT, 20, 0),
			Error::<Runtime>::NumOverflow
		);
		assert_noop!(
			Controller::set_base_rate_per_block(Origin::signed(ALICE), CurrencyId::DOT, 20, 10),
			BadOrigin
		);
		assert_noop!(
			Controller::set_base_rate_per_block(Origin::root(), CurrencyId::MDOT, 20, 10),
			Error::<Runtime>::NotValidUnderlyingAssetId
		);
	});
}

#[test]
fn set_multiplier_per_block_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		assert_ok!(Controller::set_multiplier_per_block(
			Origin::root(),
			CurrencyId::DOT,
			20,
			10
		));
		assert_eq!(
			Controller::controller_dates(CurrencyId::DOT).multiplier_per_block,
			Rate::saturating_from_rational(2_000_000_000_000_000_000u128, BLOCKS_PER_YEAR)
		);
		assert_noop!(
			Controller::set_multiplier_per_block(Origin::root(), CurrencyId::DOT, 20, 0),
			Error::<Runtime>::NumOverflow
		);
		assert_noop!(
			Controller::set_multiplier_per_block(Origin::signed(ALICE), CurrencyId::DOT, 20, 10),
			BadOrigin
		);
		assert_noop!(
			Controller::set_multiplier_per_block(Origin::root(), CurrencyId::MDOT, 20, 10),
			Error::<Runtime>::NotValidUnderlyingAssetId
		);
	});
}

#[test]
fn set_jump_multiplier_per_block_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		assert_ok!(Controller::set_jump_multiplier_per_block(
			Origin::root(),
			CurrencyId::DOT,
			20,
			10
		));
		assert_eq!(
			Controller::controller_dates(CurrencyId::DOT).jump_multiplier_per_block,
			Rate::saturating_from_rational(2_000_000_000_000_000_000u128, BLOCKS_PER_YEAR)
		);
		assert_noop!(
			Controller::set_jump_multiplier_per_block(Origin::root(), CurrencyId::DOT, 20, 0),
			Error::<Runtime>::NumOverflow
		);
		assert_noop!(
			Controller::set_jump_multiplier_per_block(Origin::signed(ALICE), CurrencyId::DOT, 20, 10),
			BadOrigin
		);
		assert_noop!(
			Controller::set_jump_multiplier_per_block(Origin::root(), CurrencyId::MDOT, 20, 10),
			Error::<Runtime>::NotValidUnderlyingAssetId
		);
	});
}
