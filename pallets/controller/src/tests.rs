//! Tests for the controller pallet.

use super::*;
use mock::{Event, *};

use frame_support::{assert_err, assert_noop, assert_ok};
use sp_runtime::DispatchError::BadOrigin;

#[test]
fn operations_are_paused_by_default() {
	ExtBuilder::default().build().execute_with(|| {
		// All operations are paused when nothing is in storage
		assert_eq!(Controller::pause_keepers(KSM), PauseKeeper::all_paused());
	});
}

#[test]
fn protocol_operations_not_working_for_nonexisting_pool() {
	ExtBuilder::default().build().execute_with(|| {
		assert_noop!(
			Controller::pause_operation(alice(), ETH, Operation::Deposit),
			Error::<Runtime>::PoolNotFound
		);

		assert_noop!(
			Controller::resume_operation(alice(), ETH, Operation::Deposit),
			Error::<Runtime>::PoolNotFound
		);

		assert_noop!(
			Controller::set_protocol_interest_factor(alice(), ETH, Rate::one()),
			Error::<Runtime>::PoolNotFound
		);

		assert_noop!(
			Controller::set_max_borrow_rate(alice(), ETH, Rate::one()),
			Error::<Runtime>::PoolNotFound
		);

		assert_noop!(
			Controller::set_collateral_factor(alice(), ETH, Rate::one()),
			Error::<Runtime>::PoolNotFound
		);

		assert_noop!(
			Controller::set_borrow_cap(alice(), ETH, Some(100u128)),
			Error::<Runtime>::PoolNotFound
		);

		assert_noop!(
			Controller::set_protocol_interest_threshold(alice(), ETH, 100u128),
			Error::<Runtime>::PoolNotFound
		);
	});
}

#[test]
fn accrue_interest_should_work() {
	ExtBuilder::default()
		.pool_total_borrowed(DOT, dollars(80_u128))
		.pool_mock(BTC)
		.pool_balance(DOT, dollars(20_u128))
		.build()
		.execute_with(|| {
			System::set_block_number(1);

			assert_ok!(Controller::accrue_interest_rate(DOT));

			assert_eq!(Controller::controller_dates(DOT).last_interest_accrued_block, 1);
			assert_eq!(TestPools::pools(DOT).total_protocol_interest, 57_600_000_000);
			assert_eq!(
				Controller::get_liquidity_pool_borrow_and_supply_rates(DOT),
				Some((Rate::from_inner(139_680_000_267), Rate::from_inner(100_569_600_394)))
			);
			assert_eq!(TestPools::pools(DOT).total_borrowed, 80_000_000_576_000_000_000);
			assert_eq!(
				TestPools::pools(DOT).borrow_index,
				Rate::from_inner(1_000_000_007_200_000_000)
			);
		});
}

#[test]
fn accrue_interest_should_not_work() {
	ExtBuilder::default()
		.pool_total_borrowed(DOT, dollars(80_u128))
		.pool_balance(DOT, dollars(20_u128))
		.build()
		.execute_with(|| {
			System::set_block_number(1);

			assert_ok!(Controller::accrue_interest_rate(DOT));
			assert_eq!(Controller::controller_dates(DOT).last_interest_accrued_block, 1);

			assert_ok!(Controller::set_max_borrow_rate(
				alice(),
				DOT,
				Rate::saturating_from_rational(1, 1_000_000_000)
			));

			System::set_block_number(20);

			assert_noop!(
				Controller::accrue_interest_rate(DOT),
				Error::<Runtime>::BorrowRateTooHigh
			);

			assert_ok!(Controller::set_max_borrow_rate(
				alice(),
				DOT,
				Rate::saturating_from_integer(2)
			));

			assert_ok!(Controller::accrue_interest_rate(DOT));
		});
}

#[test]
fn calculate_block_delta_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		// block_delta = 10 - 5 = 5
		assert_eq!(Controller::calculate_block_delta(10, 5), Ok(5));

		// Overflow in calculation: 5 - 10 = -5 < 0
		assert_noop!(Controller::calculate_block_delta(5, 10), Error::<Runtime>::NumOverflow);
	});
}

#[test]
fn calculate_interest_factor_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		// interest_factor = 0.1 * 25 = 2.5
		assert_eq!(
			Controller::calculate_interest_factor(Rate::saturating_from_rational(1, 10), 25),
			Ok(Rate::saturating_from_rational(25, 10))
		);

		// Overflow in calculation: block_delta * borrow_interest_rate
		assert_noop!(
			Controller::calculate_interest_factor(Rate::from_inner(u128::max_value()), 20),
			Error::<Runtime>::NumOverflow
		);
	});
}

#[test]
fn borrow_balance_stored_with_zero_balance_should_work() {
	ExtBuilder::default()
		.pool_mock(DOT)
		.pool_user_data(DOT, ALICE, Balance::zero(), Rate::from_inner(0), true, 0)
		.build()
		.execute_with(|| {
			// If borrow_balance = 0 then borrow_index is likely also 0, return Ok(0)
			assert_eq!(Controller::borrow_balance_stored(&ALICE, DOT), Ok(Balance::zero()));
		});
}

#[test]
fn borrow_balance_stored_should_work() {
	ExtBuilder::default()
		.pool_mock(DOT)
		.pool_user_data(DOT, ALICE, 100, Rate::saturating_from_rational(4, 1), true, 0)
		.build()
		.execute_with(|| {
			// recent_borrow_balance = 100 * 2 / 4 = 50
			assert_eq!(Controller::borrow_balance_stored(&ALICE, DOT), Ok(50));
		});
}

#[test]
fn borrow_balance_stored_fails_if_num_overflow() {
	ExtBuilder::default()
		.pool_mock(DOT)
		.pool_user_data(
			DOT,
			ALICE,
			Balance::max_value(),
			Rate::saturating_from_rational(2, 1),
			true,
			0,
		)
		.pool_mock(BTC)
		.pool_user_data(DOT, ALICE, 100, Rate::from_inner(0), true, 0)
		.build()
		.execute_with(|| {
			assert_noop!(
				Controller::borrow_balance_stored(&ALICE, DOT),
				Error::<Runtime>::BorrowBalanceOverflow
			);
		});
}

#[test]
fn calculate_utilization_rate_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		// if current_total_borrowed_balance == 0 then return Ok(0)
		assert_eq!(
			Controller::calculate_utilization_rate(100, 0, 60),
			Ok(Rate::from_inner(0))
		);
		// utilization_rate = 80 / (22 + 80 - 2) = 0.8
		assert_eq!(
			Controller::calculate_utilization_rate(22, 80, 2),
			Ok(Rate::saturating_from_rational(8, 10))
		);

		// Overflow in calculation: total_balance + total_borrowed = max_value() + 80
		assert_noop!(
			Controller::calculate_utilization_rate(Balance::max_value(), 80, 2),
			Error::<Runtime>::UtilizationRateCalculationError
		);

		// Overflow in calculation:
		// total_balance_total_borrowed_sum - total_protocol_interest = ... - max_value()
		assert_noop!(
			Controller::calculate_utilization_rate(22, 80, Balance::max_value()),
			Error::<Runtime>::UtilizationRateCalculationError
		);

		// Overflow in calculation: total_borrows / 0
		assert_noop!(
			Controller::calculate_utilization_rate(100, 70, 170),
			Error::<Runtime>::UtilizationRateCalculationError
		);
	});
}

#[test]
fn get_hypothetical_account_liquidity_when_m_tokens_balance_is_zero_should_work() {
	ExtBuilder::default()
		.pool_mock(DOT)
		.pool_mock(BTC)
		.pool_user_data(DOT, ALICE, Balance::zero(), Rate::from_inner(0), true, 0)
		.pool_user_data(BTC, BOB, Balance::zero(), Rate::from_inner(0), false, 0)
		.build()
		.execute_with(|| {
			// Set price = 2.00 USD for all assets.
			MockPriceSource::set_underlying_price(Some(Price::from_inner(2 * DOLLARS)));

			// Checking the function when called from redeem.
			// The function should return the shortfall to a large zero.
			assert_eq!(
				Controller::get_hypothetical_account_liquidity(&ALICE, DOT, 5, 0),
				Ok((0, 9))
			);
			// Checking the function when called from borrow.
			// The function should return the shortfall to a large zero.
			assert_eq!(
				Controller::get_hypothetical_account_liquidity(&ALICE, DOT, 0, 10),
				Ok((0, 20))
			);
			// Checking scenario: the user tries to take a borrow in a currency which is not
			// pool as available for collateral, and he fails.
			assert_eq!(
				Controller::get_hypothetical_account_liquidity(&BOB, BTC, 0, 10),
				Ok((0, 20))
			);
		});
}

#[test]
fn get_hypothetical_account_liquidity_one_currency_from_redeem_should_work() {
	ExtBuilder::default().alice_deposit_60_dot().build().execute_with(|| {
		// Set price = 2.00 USD for all assets.
		MockPriceSource::set_underlying_price(Some(Price::from_inner(2 * DOLLARS)));

		// Checking the function when called from redeem.
		// collateral parameter is set to false, user can't redeem.
		assert_eq!(
			Controller::get_hypothetical_account_liquidity(&ALICE, DOT, 5, 0),
			Ok((0, 9))
		);
		assert_eq!(
			Controller::get_hypothetical_account_liquidity(&ALICE, DOT, 60, 0),
			Ok((0, 108))
		);
		assert_eq!(
			Controller::get_hypothetical_account_liquidity(&ALICE, DOT, 200, 0),
			Ok((0, 360))
		);
	});
}

#[test]
fn get_hypothetical_account_liquidity_two_currencies_from_redeem_should_work() {
	ExtBuilder::default()
		.alice_deposit_60_dot()
		.alice_deposit_20_eth()
		.build()
		.execute_with(|| {
			// Set price = 2.00 USD for all assets.
			MockPriceSource::set_underlying_price(Some(Price::from_inner(2 * DOLLARS)));

			// Checking the function when called from redeem.
			// collateral parameter is set to false, user can't redeem.
			assert_eq!(
				Controller::get_hypothetical_account_liquidity(&ALICE, ETH, 15, 0),
				Ok((0, 27))
			);
			assert_eq!(
				Controller::get_hypothetical_account_liquidity(&ALICE, ETH, 80, 0),
				Ok((0, 144))
			);
			assert_eq!(
				Controller::get_hypothetical_account_liquidity(&ALICE, ETH, 100, 0),
				Ok((0, 180))
			);
		});
}

#[test]
fn get_hypothetical_account_liquidity_two_currencies_from_borrow_should_work() {
	ExtBuilder::default()
		.alice_deposit_20_eth()
		// ALICE deposit 60 DOT and borrow 30 DOT
		.user_balance(ALICE, DOT, 70)
		.user_balance(ALICE, MDOT, 60)
		.pool_balance(DOT, 60)
		.pool_total_borrowed(DOT, 30)
		.pool_user_data(DOT, ALICE, 30, Rate::saturating_from_rational(1, 1), false, 0)
		.build()
		.execute_with(|| {
			// Set price = 2.00 USD for all assets.
			MockPriceSource::set_underlying_price(Some(Price::from_inner(2 * DOLLARS)));

			// Checking the function when called from borrow.
			// collateral parameter for DOT and ETH pool is set to false. User can't borrow.
			assert_eq!(
				Controller::get_hypothetical_account_liquidity(&ALICE, DOT, 0, 30),
				Ok((0, 120))
			);

			// Alice set collateral parameter value to true for DOT pool. Alice can borrow.
			<LiquidityPools<Runtime>>::enable_is_collateral_internal(&ALICE, DOT);

			assert_eq!(
				Controller::get_hypothetical_account_liquidity(&ALICE, DOT, 0, 50),
				Ok((2, 0))
			);
			assert_eq!(
				Controller::get_hypothetical_account_liquidity(&ALICE, DOT, 0, 100),
				Ok((0, 98))
			);
		});
}

#[test]
fn get_liquidity_pool_exchange_rate_should_work() {
	ExtBuilder::default()
		.pool_balance(DOT, dollars(100_u128))
		.user_balance(ALICE, MDOT, dollars(125_u128))
		.pool_total_borrowed(DOT, dollars(300_u128))
		.build()
		.execute_with(|| {
			// exchange_rate = (100 - 0 + 300) / 125 = 3.2
			assert_eq!(
				Controller::get_liquidity_pool_exchange_rate(DOT),
				Some(Rate::saturating_from_rational(32, 10))
			);

			assert_eq!(Controller::get_liquidity_pool_exchange_rate(ETH), None);
		});
}

#[test]
fn get_liquidity_pool_borrow_and_supply_rates_less_than_kink() {
	ExtBuilder::default()
		.pool_balance(DOT, dollars(100_u128))
		.pool_total_borrowed(DOT, dollars(300_u128))
		.build()
		.execute_with(|| {
			// utilization_rate = 300 / (100 - 0 + 300) = 0.75 < kink = 0.8
			// borrow_rate = 0.75 * 0.000_000_009 + 0 = 0.00000000675
			// supply_rate = 0.75 * 0.00_000_000_675 * (1 - 0.1) = 0.00000000455625
			assert_eq!(
				Controller::get_liquidity_pool_borrow_and_supply_rates(DOT),
				Some((Rate::from_inner(6750000014), Rate::from_inner(4556250018)))
			);

			assert_eq!(Controller::get_liquidity_pool_borrow_and_supply_rates(ETH), None);
		});
}

#[test]
fn get_liquidity_pool_borrow_and_supply_rates_above_kink() {
	ExtBuilder::default()
		.pool_balance(DOT, dollars(100_u128))
		.pool_total_borrowed(DOT, dollars(500_u128))
		.build()
		.execute_with(|| {
			// utilization_rate = 500 / (100 - 0 + 500) = 0.83 > kink = 0.8
			// borrow_rate = 0,83333336 * 0.8 * 0.000_000_207  + (0.8 * 0.000_000_009) + 0 = 0.0000001452
			// supply_rate = 0,83333336 * 0.0000001452 * (1 - 0.1) = 0,0000001089
			assert_eq!(
				Controller::get_liquidity_pool_borrow_and_supply_rates(DOT),
				Some((Rate::from_inner(145200005009), Rate::from_inner(108900007709)))
			);
		});
}

#[test]
fn get_user_supply_and_borrow_apy_should_work() {
	ExtBuilder::default()
		.pool_balance(DOT, dollars(100_u128))
		.pool_total_borrowed(DOT, dollars(500_u128))
		.pool_balance(ETH, dollars(100_u128))
		.pool_total_borrowed(ETH, dollars(300_u128))
		.pool_balance(BTC, dollars(100_u128))
		.pool_total_borrowed(BTC, dollars(100_u128))
		.pool_balance(KSM, dollars(100_u128))
		.pool_total_borrowed(KSM, dollars(100_u128))
		.user_balance(ALICE, MDOT, dollars(600_u128))
		.pool_user_data(DOT, ALICE, dollars(500_u128), Rate::one(), true, 0)
		.user_balance(ALICE, METH, dollars(400_u128))
		.pool_user_data(ETH, ALICE, dollars(300_u128), Rate::one(), true, 0)
		.user_balance(ALICE, MBTC, dollars(200_u128))
		.pool_user_data(BTC, ALICE, dollars(100_u128), Rate::one(), true, 0)
		.user_balance(ALICE, MKSM, dollars(100_u128))
		.pool_user_data(KSM, ALICE, Balance::zero(), Rate::one(), false, 0)
		.build()
		.execute_with(|| {
			// Set price = 2.00 USD for all assets.
			MockPriceSource::set_underlying_price(Some(Price::from_inner(2 * DOLLARS)));

			// borrow_rate_per_year = 0.0000001452 * 5256000 = 76,3171226327304 %
			// supply_rate_per_year = 0,0000001089 * 5256000 = 57,2378440518504 %

			assert_eq!(
				Controller::get_liquidity_pool_borrow_and_supply_rates(DOT),
				Some((Rate::from_inner(145200005009), Rate::from_inner(108900007709)))
			);

			// borrow_rate_per_year = 0.000000006750000014 * 5256000 = 3,5478 %
			// supply_rate_per_year = 0,000000004556250018 * 5256000 = 2,3947 %

			assert_eq!(
				Controller::get_liquidity_pool_borrow_and_supply_rates(ETH),
				Some((Rate::from_inner(6750000014), Rate::from_inner(4556250018)))
			);

			// borrow_rate_per_year = 0,000000004500000011 * 5256000 = 2,3652 %
			// supply_rate_per_year = 0,000000002025000009 * 5256000 = 1,0643 %
			assert_eq!(
				Controller::get_liquidity_pool_borrow_and_supply_rates(BTC),
				Some((Rate::from_inner(4500000011), Rate::from_inner(2025000009)))
			);

			// borrow_rate_per_year = 0,000000004500000011 * 5256000 = 2,3652 %
			// supply_rate_per_year = 0,000000002025000009 * 5256000 = 1,0643 %
			assert_eq!(
				Controller::get_liquidity_pool_borrow_and_supply_rates(KSM),
				Some((Rate::from_inner(4500000011), Rate::from_inner(2025000009)))
			);

			// Hypothetical year supply interest(for the pool):
			// supply_interest = user_supply_in_usd * supply_apy_as_decimal
			// DOT: 1200 * 0.5723 = 686.854 $
			// ETH: 800 * 0.0234 = 19.157 $
			// BTC: 400 * 0.0106 = 4.257 $
			// KSM: 400 * 0.0106 = 4.257 $
			// sum = 686.854 + 19.157 + 4.257 + 4.257 = 714.525 $
			// total_supply_in_usd = 1200 + 800 + 400 + 400 = 2800 $
			// sum_supply_apy = 714.525/2800 = 25.51 %

			// Hypothetical year borrow interest(for the pool):
			// borrow_interest = user_borrow_in_usd * borrow_apy_as_decimal
			// DOT: 1000 * 0.7631 = 763.17 $
			// ETH: 600 * 0.0354 = 21.286 $
			// BTC: 200 * 0.0236 = 4.73 $
			// sum = 763.17 + 21.286 + 4.73 = 789.186 $
			// total_borrow_in_usd = 1000 + 600 + 200 = 1800 $
			// sum_borrow_apy = 789.186/1800 = 43.84 %

			assert_eq!(
				Controller::get_user_supply_and_borrow_apy(ALICE),
				Ok((
					Rate::from_inner(255_188_217_480_048_000),
					Rate::from_inner(438_438_039_737_112_000)
				))
			);
		});
}

#[test]
fn get_user_supply_per_asset_should_work() {
	ExtBuilder::default()
		.pool_balance(DOT, dollars(100_u128))
		.pool_total_borrowed(DOT, dollars(500_u128))
		.user_balance(ALICE, MDOT, dollars(600_u128))
		.build()
		.execute_with(|| {
			System::set_block_number(0);

			assert_eq!(
				Controller::get_user_supply_per_asset(&ALICE, DOT),
				Ok(dollars(600_u128))
			);

			System::set_block_number(10);

			// block_delta = 10
			// current_borrow_interest_rate = 145_199_999_999
			// interest_accumulated = total_borrow * borrow_interest_rate * block_delta
			// interest_accumulated = 500*10^18 * 0,000_000_145_199_999_999 * 10 = 0,000_725_999_999_995
			// new_total_borrowed = 500*10^18  + 0,000_725_999_999_995 =  500,000_725_999_999_995
			// total_protocol_interest = 0,000_725_999_999_995 * 0.1 =  0,0_000_725_999_999_995

			// expected_exchange_rate = 1,000001088999999992
			let expected_exchange_rate = <LiquidityPools<Runtime>>::get_exchange_rate_by_interest_params(
				DOT,
				72599999999500,
				500000725999999995000,
			)
			.unwrap();

			// expected_supply_balance = 600 * 1,000001088999999992 = 600,0006533999999952
			let expected_supply_balance = Rate::from_inner(dollars(600_u128))
				.checked_mul(&expected_exchange_rate)
				.map(|v| v.into_inner())
				.unwrap();

			assert_eq!(expected_supply_balance, 600_000_653_399_999_995_200);

			assert_eq!(
				Controller::get_user_supply_per_asset(&ALICE, DOT),
				Ok(expected_supply_balance)
			);
		});
}

#[test]
fn get_vec_of_supply_and_borrow_balance_should_work() {
	ExtBuilder::default()
		.pool_balance(DOT, dollars(100_u128))
		.pool_total_borrowed(DOT, dollars(500_u128))
		.pool_balance(ETH, dollars(100_u128))
		.pool_total_borrowed(ETH, dollars(300_u128))
		.pool_balance(BTC, dollars(100_u128))
		.pool_total_borrowed(BTC, dollars(100_u128))
		.pool_balance(KSM, dollars(100_u128))
		.pool_total_borrowed(KSM, dollars(100_u128))
		.user_balance(ALICE, MDOT, dollars(600_u128))
		.pool_user_data(DOT, ALICE, dollars(500_u128), Rate::one(), true, 0)
		.user_balance(ALICE, METH, dollars(400_u128))
		.pool_user_data(ETH, ALICE, dollars(300_u128), Rate::one(), true, 0)
		.user_balance(ALICE, MBTC, dollars(200_u128))
		.pool_user_data(BTC, ALICE, dollars(100_u128), Rate::one(), true, 0)
		.user_balance(ALICE, MKSM, dollars(100_u128))
		.pool_user_data(KSM, ALICE, Balance::zero(), Rate::one(), false, 0)
		.build()
		.execute_with(|| {
			System::set_block_number(0);

			// Set price = 2.00 USD for all assets.
			MockPriceSource::set_underlying_price(Some(Price::from_inner(2 * DOLLARS)));

			let expected_vec = vec![
				(DOT, dollars(1200_u128), dollars(1000_u128)),
				(KSM, dollars(400_u128), Balance::zero()),
				(BTC, dollars(400_u128), dollars(200_u128)),
				(ETH, dollars(800_u128), dollars(600_u128)),
			];

			assert_eq!(
				Controller::get_vec_of_supply_and_borrow_balance(&ALICE),
				Ok(expected_vec)
			);
		});
}

#[test]
fn get_block_delta_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		System::set_block_number(0);

		assert_eq!(Controller::get_block_delta(DOT), Ok(0));

		System::set_block_number(45);

		assert_eq!(Controller::get_block_delta(DOT), Ok(45));
		assert_ok!(Controller::accrue_interest_rate(DOT));
		assert_eq!(Controller::get_block_delta(DOT), Ok(0));

		System::set_block_number(46);

		assert_eq!(Controller::get_block_delta(DOT), Ok(1));
	});
}

#[test]
fn redeem_allowed_should_work() {
	ExtBuilder::default().alice_deposit_60_dot().build().execute_with(|| {
		assert_ok!(Controller::redeem_allowed(DOT, &ALICE, dollars(40_u128)));

		// collateral parameter is set to true.
		<LiquidityPools<Runtime>>::enable_is_collateral_internal(&ALICE, DOT);

		assert_err!(
			Controller::redeem_allowed(DOT, &ALICE, dollars(100_u128)),
			Error::<Runtime>::InsufficientLiquidity
		);
	});
}

#[test]
fn borrow_allowed_should_work() {
	ExtBuilder::default().alice_deposit_60_dot().build().execute_with(|| {
		// collateral parameter is set to false. User can't borrow
		assert_err!(
			Controller::borrow_allowed(DOT, &ALICE, dollars(10_u128)),
			Error::<Runtime>::InsufficientLiquidity
		);

		// collateral parameter is set to true. User can borrow.
		<LiquidityPools<Runtime>>::enable_is_collateral_internal(&ALICE, DOT);

		assert_ok!(Controller::borrow_allowed(DOT, &ALICE, dollars(10_u128)));

		assert_noop!(
			Controller::borrow_allowed(DOT, &ALICE, dollars(999_u128)),
			Error::<Runtime>::InsufficientLiquidity
		);
	});
}

#[test]
fn is_operation_allowed_should_work() {
	ExtBuilder::default().pool_mock(DOT).build().execute_with(|| {
		assert!(Controller::is_operation_allowed(DOT, Operation::Deposit));
		assert!(Controller::is_operation_allowed(DOT, Operation::Redeem));
		assert!(Controller::is_operation_allowed(DOT, Operation::Borrow));
		assert!(Controller::is_operation_allowed(DOT, Operation::Repay));

		assert_ok!(Controller::pause_operation(alice(), DOT, Operation::Deposit));
		assert_ok!(Controller::pause_operation(alice(), DOT, Operation::Redeem));

		assert!(!Controller::is_operation_allowed(DOT, Operation::Deposit));
		assert!(!Controller::is_operation_allowed(DOT, Operation::Redeem));
		assert!(Controller::is_operation_allowed(DOT, Operation::Borrow));
		assert!(Controller::is_operation_allowed(DOT, Operation::Repay));
	});
}

/* ----------------------------------------------------------------------------------------- */

// Admin functions

#[test]
fn set_protocol_interest_factor_should_work() {
	ExtBuilder::default().pool_mock(DOT).build().execute_with(|| {
		// ALICE set protocol interest factor equal 2.0
		assert_ok!(Controller::set_protocol_interest_factor(
			alice(),
			DOT,
			Rate::saturating_from_integer(2)
		));
		let expected_event = Event::controller(crate::Event::InterestFactorChanged);
		assert!(System::events().iter().any(|record| record.event == expected_event));
		assert_eq!(
			Controller::controller_dates(DOT).protocol_interest_factor,
			Rate::saturating_from_rational(20, 10)
		);

		// ALICE set protocol interest factor equal 0.0
		assert_ok!(Controller::set_protocol_interest_factor(alice(), DOT, Rate::zero()));
		let expected_event = Event::controller(crate::Event::InterestFactorChanged);
		assert!(System::events().iter().any(|record| record.event == expected_event));
		assert_eq!(
			Controller::controller_dates(DOT).protocol_interest_factor,
			Rate::from_inner(0)
		);

		// The dispatch origin of this call must be Root or half MinterestCouncil.
		assert_noop!(
			Controller::set_protocol_interest_factor(bob(), DOT, Rate::saturating_from_integer(2)),
			BadOrigin
		);

		assert_noop!(
			Controller::set_protocol_interest_factor(alice(), MDOT, Rate::saturating_from_integer(2)),
			Error::<Runtime>::PoolNotFound
		);
	});
}

#[test]
fn set_max_borrow_rate_should_work() {
	ExtBuilder::default().pool_mock(DOT).build().execute_with(|| {
		// ALICE set max borrow rate equal 2.0
		assert_ok!(Controller::set_max_borrow_rate(
			alice(),
			DOT,
			Rate::saturating_from_integer(2)
		));
		let expected_event = Event::controller(crate::Event::MaxBorrowRateChanged);
		assert!(System::events().iter().any(|record| record.event == expected_event));
		assert_eq!(
			Controller::controller_dates(DOT).max_borrow_rate,
			Rate::saturating_from_rational(20, 10)
		);

		// ALICE can't set max borrow rate equal 0.0
		assert_noop!(
			Controller::set_max_borrow_rate(alice(), DOT, Rate::zero()),
			Error::<Runtime>::MaxBorrowRateCannotBeZero
		);

		// The dispatch origin of this call must be Root or half MinterestCouncil.
		assert_noop!(
			Controller::set_max_borrow_rate(bob(), DOT, Rate::saturating_from_integer(2)),
			BadOrigin
		);

		assert_noop!(
			Controller::set_max_borrow_rate(alice(), MDOT, Rate::saturating_from_integer(2)),
			Error::<Runtime>::PoolNotFound
		);
	});
}

#[test]
fn set_collateral_factor_should_work() {
	ExtBuilder::default().pool_mock(DOT).build().execute_with(|| {
		// ALICE set collateral factor equal 0.5.
		assert_ok!(Controller::set_collateral_factor(
			alice(),
			DOT,
			Rate::saturating_from_rational(1, 2)
		));
		let expected_event = Event::controller(crate::Event::CollateralFactorChanged);
		assert!(System::events().iter().any(|record| record.event == expected_event));
		assert_eq!(
			Controller::controller_dates(DOT).collateral_factor,
			Rate::saturating_from_rational(1, 2)
		);

		// ALICE can't set collateral factor equal 0.0
		assert_noop!(
			Controller::set_collateral_factor(alice(), DOT, Rate::zero()),
			Error::<Runtime>::CollateralFactorIncorrectValue
		);

		// ALICE can't set collateral factor grater than one.
		assert_noop!(
			Controller::set_collateral_factor(alice(), DOT, Rate::saturating_from_rational(11, 10)),
			Error::<Runtime>::CollateralFactorIncorrectValue
		);

		// The dispatch origin of this call must be Root or half MinterestCouncil.
		assert_noop!(
			Controller::set_collateral_factor(bob(), DOT, Rate::saturating_from_integer(2)),
			BadOrigin
		);

		// Unavailable currency id.
		assert_noop!(
			Controller::set_collateral_factor(alice(), MDOT, Rate::saturating_from_integer(2)),
			Error::<Runtime>::PoolNotFound
		);
	});
}

#[test]
fn pause_operation_should_work() {
	ExtBuilder::default()
		.pool_mock(DOT).build().execute_with(|| {
		assert!(!Controller::pause_keepers(&DOT).deposit_paused);
		assert!(!Controller::pause_keepers(&DOT).redeem_paused);
		assert!(!Controller::pause_keepers(&DOT).borrow_paused);
		assert!(!Controller::pause_keepers(&DOT).repay_paused);
		assert!(!Controller::pause_keepers(&DOT).transfer_paused);

		assert_ok!(Controller::pause_operation(alice(), DOT, Operation::Deposit));
		let expected_event = Event::controller(crate::Event::OperationIsPaused(DOT, Operation::Deposit));
		assert!(System::events().iter().any(|record| record.event == expected_event));

		assert_ok!(Controller::pause_operation(alice(), DOT, Operation::Redeem));
		let expected_event = Event::controller(crate::Event::OperationIsPaused(DOT, Operation::Redeem));
		assert!(System::events().iter().any(|record| record.event == expected_event));

		assert_ok!(Controller::pause_operation(alice(), DOT, Operation::Borrow));
		let expected_event = Event::controller(crate::Event::OperationIsPaused(DOT, Operation::Borrow));
		assert!(System::events().iter().any(|record| record.event == expected_event));

		assert_ok!(Controller::pause_operation(alice(), DOT, Operation::Repay));
		let expected_event = Event::controller(crate::Event::OperationIsPaused(DOT, Operation::Repay));
		assert!(System::events().iter().any(|record| record.event == expected_event));

		assert_ok!(Controller::pause_operation(alice(), DOT, Operation::Transfer));
		let expected_event = Event::controller(crate::Event::OperationIsPaused(DOT, Operation::Transfer));
		assert!(System::events().iter().any(|record| record.event == expected_event));

		assert!(Controller::pause_keepers(&DOT).deposit_paused);
		assert!(Controller::pause_keepers(&DOT).redeem_paused);
		assert!(Controller::pause_keepers(&DOT).borrow_paused);
		assert!(Controller::pause_keepers(&DOT).repay_paused);
		assert!(Controller::pause_keepers(&DOT).transfer_paused);

		assert_noop!(Controller::pause_operation(bob(), DOT, Operation::Deposit), BadOrigin);
		assert_noop!(
			Controller::pause_operation(alice(), MDOT, Operation::Redeem),
			Error::<Runtime>::PoolNotFound
		);
	});
}

#[test]
fn resume_operation_should_work() {
	ExtBuilder::default()
		.pool_mock(DOT)
		.pool_mock(KSM)
		.build()
		.execute_with(|| {
			assert!(Controller::pause_keepers(&KSM).deposit_paused);
			assert!(Controller::pause_keepers(&KSM).redeem_paused);
			assert!(Controller::pause_keepers(&KSM).borrow_paused);
			assert!(Controller::pause_keepers(&KSM).repay_paused);
			assert!(Controller::pause_keepers(&KSM).transfer_paused);

			assert_ok!(Controller::resume_operation(alice(), KSM, Operation::Deposit));
			let expected_event = Event::controller(crate::Event::OperationIsUnPaused(KSM, Operation::Deposit));
			assert!(System::events().iter().any(|record| record.event == expected_event));

			assert_ok!(Controller::resume_operation(alice(), KSM, Operation::Redeem));
			let expected_event = Event::controller(crate::Event::OperationIsUnPaused(KSM, Operation::Redeem));
			assert!(System::events().iter().any(|record| record.event == expected_event));

			assert_ok!(Controller::resume_operation(alice(), KSM, Operation::Borrow));
			let expected_event = Event::controller(crate::Event::OperationIsUnPaused(KSM, Operation::Borrow));
			assert!(System::events().iter().any(|record| record.event == expected_event));

			assert_ok!(Controller::resume_operation(alice(), KSM, Operation::Repay));
			let expected_event = Event::controller(crate::Event::OperationIsUnPaused(KSM, Operation::Repay));
			assert!(System::events().iter().any(|record| record.event == expected_event));

			assert_ok!(Controller::resume_operation(alice(), KSM, Operation::Transfer));
			let expected_event = Event::controller(crate::Event::OperationIsUnPaused(KSM, Operation::Transfer));
			assert!(System::events().iter().any(|record| record.event == expected_event));

			assert!(!Controller::pause_keepers(&KSM).deposit_paused);
			assert!(!Controller::pause_keepers(&KSM).redeem_paused);
			assert!(!Controller::pause_keepers(&KSM).borrow_paused);
			assert!(!Controller::pause_keepers(&KSM).repay_paused);
			assert!(!Controller::pause_keepers(&KSM).transfer_paused);

			assert_noop!(Controller::resume_operation(bob(), DOT, Operation::Deposit), BadOrigin);
			assert_noop!(
				Controller::resume_operation(alice(), MDOT, Operation::Redeem),
				Error::<Runtime>::PoolNotFound
			);
		});
}

#[test]
fn switch_whitelist_mode_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		assert_ok!(Controller::switch_whitelist_mode(alice()));
		let expected_event = Event::controller(crate::Event::ProtocolOperationModeSwitched(true));
		assert!(System::events().iter().any(|record| record.event == expected_event));
		assert!(Controller::whitelist_mode());

		assert_ok!(Controller::switch_whitelist_mode(alice()));
		assert!(!Controller::whitelist_mode());

		assert_noop!(Controller::switch_whitelist_mode(bob()), BadOrigin);
		assert!(!Controller::whitelist_mode());
	});
}

#[test]
fn set_borrow_cap_should_work() {
	ExtBuilder::default()
		.pool_mock(DOT)
		.user_balance(ALICE, DOT, ONE_HUNDRED)
		.build()
		.execute_with(|| {
			// The dispatch origin of this call must be Administrator.
			assert_noop!(Controller::set_borrow_cap(bob(), DOT, Some(10_u128)), BadOrigin);

			// ALICE set borrow cap to 10.
			assert_ok!(Controller::set_borrow_cap(alice(), DOT, Some(10_u128)));
			let expected_event = Event::controller(crate::Event::BorrowCapChanged(DOT, Some(10_u128)));
			assert!(System::events().iter().any(|record| record.event == expected_event));

			// ALICE is able to change borrow cap to 9999
			assert_ok!(Controller::set_borrow_cap(alice(), DOT, Some(9999_u128)));
			let expected_event = Event::controller(crate::Event::BorrowCapChanged(DOT, Some(9999_u128)));
			assert!(System::events().iter().any(|record| record.event == expected_event));

			// Unable to set borrow cap greater than MAX_BORROW_CAP.
			assert_noop!(
				Controller::set_borrow_cap(alice(), DOT, Some(dollars(1_000_001_u128))),
				Error::<Runtime>::InvalidBorrowCap
			);

			// Alice is able to set zero borrow cap.
			assert_ok!(Controller::set_borrow_cap(alice(), DOT, Some(0_u128)));
			let expected_event = Event::controller(crate::Event::BorrowCapChanged(DOT, Some(0_u128)));
			assert!(System::events().iter().any(|record| record.event == expected_event));
		});
}

#[test]
fn set_protocol_interest_threshold_should_work() {
	ExtBuilder::default()
		.pool_mock(DOT)
		.user_balance(ALICE, DOT, ONE_HUNDRED)
		.build()
		.execute_with(|| {
			// The dispatch origin of this call must be Administrator.
			assert_noop!(
				Controller::set_protocol_interest_threshold(bob(), DOT, 10_u128),
				BadOrigin
			);

			// ALICE set protocol interest threshold to 10.
			assert_ok!(Controller::set_protocol_interest_threshold(alice(), DOT, 10_u128));
			let expected_event = Event::controller(crate::Event::ProtocolInterestThresholdChanged(DOT, 10_u128));
			assert!(System::events().iter().any(|record| record.event == expected_event));

			// Alice is able to set zero protocol interest threshold.
			assert_ok!(Controller::set_protocol_interest_threshold(alice(), DOT, 0_u128));
			let expected_event = Event::controller(crate::Event::ProtocolInterestThresholdChanged(DOT, 0_u128));
			assert!(System::events().iter().any(|record| record.event == expected_event));
		});
}
