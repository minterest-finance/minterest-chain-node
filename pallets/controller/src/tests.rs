//! Tests for the controller pallet.
use frame_support::{assert_err, assert_noop, assert_ok};
pub use minterest_primitives::{Balance, CurrencyId, Interest, Operation, Rate};
use sp_runtime::{traits::Zero, FixedPointNumber};

use pallet_traits::ControllerManager;
use sp_runtime::DispatchError::BadOrigin;

pub use test_engine::*;
pub use test_helper::*;

// Presets for controller testing
#[cfg(test)]
pub trait ControllerPresets {
	fn preset_alice_deposit_60_dot(self) -> Self;
	fn preset_alice_deposit_20_eth(self) -> Self;
}

impl ControllerPresets for ExtBuilderNew {
	fn preset_alice_deposit_60_dot(self) -> Self {
		self.set_user_balance(ALICE, DOT, dollars(40_u128))
			.set_user_balance(ALICE, MDOT, dollars(60_u128))
			.set_pool_balance(DOT, dollars(60_u128))
			.init_pool(
				DOT,                                  // pool_id
				Balance::zero(),                      // borrowed
				Rate::saturating_from_rational(2, 1), // borrow_index
				Balance::zero(),                      // protocol_interest
			)
			.set_pool_user_data(
				DOT,             // pool_id
				ALICE,           // user
				Balance::zero(), // borrowed
				Rate::zero(),    // interest_index
				false,           // is_collateral
				0,               // liquidation_attempts
			)
			.set_controller_data(
				DOT,                                     // currency_id
				0,                                       // last_interest_accrued_block
				Rate::saturating_from_rational(1, 10),   // protocol_interest_factor
				Rate::saturating_from_rational(5, 1000), // max_borrow_rate
				Rate::saturating_from_rational(9, 10),   //collateral_factor
				None,                                    // borrow_cap
				PROTOCOL_INTEREST_TRANSFER_THRESHOLD,    // protocol_interest_threshold
			)
	}

	fn preset_alice_deposit_20_eth(self) -> Self {
		self.set_user_balance(ALICE, ETH, dollars(80_u128))
			.set_user_balance(ALICE, METH, dollars(20_u128))
			.init_pool(
				ETH,                                  // pool_id
				Balance::zero(),                      // borrowed
				Rate::saturating_from_rational(2, 1), // borrow_index
				Balance::zero(),                      // protocol_interest
			)
			.set_pool_user_data(
				ETH,             // pool_id
				ALICE,           // user
				Balance::zero(), // borrowed
				Rate::zero(),    // interest_index
				false,           // is_collateral
				0,               // liquidation_attempts
			)
			.set_pool_balance(ETH, dollars(20_u128))
			.set_controller_data(
				ETH,                                     // currency_id
				0,                                       // last_interest_accrued_block
				Rate::saturating_from_rational(1, 10),   // protocol_interest_factor
				Rate::saturating_from_rational(5, 1000), // max_borrow_rate
				Rate::saturating_from_rational(9, 10),   //collateral_factor
				None,                                    // borrow_cap
				PROTOCOL_INTEREST_TRANSFER_THRESHOLD,    // protocol_interest_threshold
			)
	}
}

#[test]
fn operations_are_paused_by_default() {
	ExtBuilderNew::default().build().execute_with(|| {
		// All operations are paused when nothing is in storage
		assert_eq!(TestController::pause_keepers(KSM), PauseKeeper::all_paused());
	});
}

#[test]
fn protocol_operations_not_working_for_nonexisting_pool() {
	ExtBuilderNew::default().build().execute_with(|| {
		assert_noop!(
			TestController::pause_operation(alice_origin(), ETH, Operation::Deposit),
			Error::<TestRuntime>::PoolNotFound
		);

		assert_noop!(
			TestController::resume_operation(alice_origin(), ETH, Operation::Deposit),
			Error::<TestRuntime>::PoolNotFound
		);

		assert_noop!(
			TestController::set_protocol_interest_factor(alice_origin(), ETH, Rate::one()),
			Error::<TestRuntime>::PoolNotFound
		);

		assert_noop!(
			TestController::set_max_borrow_rate(alice_origin(), ETH, Rate::one()),
			Error::<TestRuntime>::PoolNotFound
		);

		assert_noop!(
			TestController::set_collateral_factor(alice_origin(), ETH, Rate::one()),
			Error::<TestRuntime>::PoolNotFound
		);

		assert_noop!(
			TestController::set_borrow_cap(alice_origin(), ETH, Some(100u128)),
			Error::<TestRuntime>::PoolNotFound
		);

		assert_noop!(
			TestController::set_protocol_interest_threshold(alice_origin(), ETH, 100u128),
			Error::<TestRuntime>::PoolNotFound
		);
	});
}

#[test]
fn accrue_interest_should_work() {
	ExtBuilderNew::default()
		.set_minterest_model_params(
			DOT,                                                // currency_id
			Rate::saturating_from_rational(8, 10),              // kink
			Rate::zero(),                                       // base_rate_per_block
			Rate::saturating_from_rational(9, 1_000_000_000),   // multiplier_per_block: 0.047304 PerYear
			Rate::saturating_from_rational(207, 1_000_000_000), // jump_multiplier_per_block: 1.09 PerYear
		)
		.init_pool(
			DOT,              // pool_id
			dollars(80_u128), // total_borrowed
			Rate::one(),      // borrow_index
			Balance::zero(),  // total_protocol_interest
		)
		.set_controller_data(
			DOT,                                     // currency_id
			0,                                       // last_interest_accrued_block
			Rate::saturating_from_rational(1, 10),   // protocol_interest_factor
			Rate::saturating_from_rational(5, 1000), // max_borrow_rate
			Rate::saturating_from_rational(9, 10),   //collateral_factor
			None,                                    // borrow_cap
			PROTOCOL_INTEREST_TRANSFER_THRESHOLD,    // protocol_interest_threshold
		)
		.set_pause_keeper(DOT, false)
		.set_pool_balance(DOT, dollars(20_u128))
		.init_pool(
			BTC,                                  // currency_id
			Balance::zero(),                      // total_borrowed
			Rate::saturating_from_rational(2, 1), // borrow_index
			Balance::zero(),                      // total_protocol_interest
		)
		.build()
		.execute_with(|| {
			System::set_block_number(1);

			assert_ok!(TestController::accrue_interest_rate(DOT));
			assert_eq!(TestController::controller_dates(DOT).last_interest_accrued_block, 1);
			assert_eq!(TestPools::pools(DOT).total_protocol_interest, 57_600_000_000);
			assert_eq!(
				TestController::get_pool_exchange_borrow_and_supply_rates(DOT),
				Some((
					Rate::one(),
					Rate::from_inner(139_680_000_267),
					Rate::from_inner(100_569_600_394)
				))
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
	ExtBuilderNew::default()
		.set_minterest_model_params(
			DOT,                                                // currency_id
			Rate::saturating_from_rational(8, 10),              // kink
			Rate::zero(),                                       // base_rate_per_block
			Rate::saturating_from_rational(9, 1_000_000_000),   // multiplier_per_block: 0.047304 PerYear
			Rate::saturating_from_rational(207, 1_000_000_000), // jump_multiplier_per_block: 1.09 PerYear
		)
		.init_pool(
			DOT,              // pool_id
			dollars(80_u128), // total_borrowed
			Rate::one(),      // borrow_index
			Balance::zero(),  // total_protocol_interest
		)
		.set_controller_data(
			DOT,                                     // currency_id
			0,                                       // last_interest_accrued_block
			Rate::saturating_from_rational(1, 10),   // protocol_interest_factor
			Rate::saturating_from_rational(5, 1000), // max_borrow_rate
			Rate::saturating_from_rational(9, 10),   //collateral_factor
			None,                                    // borrow_cap
			PROTOCOL_INTEREST_TRANSFER_THRESHOLD,    // protocol_interest_threshold
		)
		.set_pool_balance(DOT, dollars(20_u128))
		.build()
		.execute_with(|| {
			System::set_block_number(1);

			assert_ok!(TestController::accrue_interest_rate(DOT));
			assert_eq!(TestController::controller_dates(DOT).last_interest_accrued_block, 1);

			assert_ok!(TestController::set_max_borrow_rate(
				alice_origin(),
				DOT,
				Rate::saturating_from_rational(1, 1_000_000_000)
			));

			System::set_block_number(20);

			assert_noop!(
				TestController::accrue_interest_rate(DOT),
				Error::<TestRuntime>::BorrowRateTooHigh
			);

			assert_ok!(TestController::set_max_borrow_rate(
				alice_origin(),
				DOT,
				Rate::saturating_from_integer(2)
			));

			assert_ok!(TestController::accrue_interest_rate(DOT));
		});
}

#[test]
fn calculate_block_delta_should_work() {
	ExtBuilderNew::default().build().execute_with(|| {
		// block_delta = 10 - 5 = 5
		assert_eq!(TestController::calculate_block_delta(10, 5), Ok(5));

		// Overflow in calculation: 5 - 10 = -5 < 0
		assert_noop!(
			TestController::calculate_block_delta(5, 10),
			Error::<TestRuntime>::NumOverflow
		);
	});
}

#[test]
fn calculate_interest_factor_should_work() {
	ExtBuilderNew::default().build().execute_with(|| {
		// interest_factor = 0.1 * 25 = 2.5
		assert_eq!(
			TestController::calculate_interest_factor(Rate::saturating_from_rational(1, 10), 25),
			Ok(Rate::saturating_from_rational(25, 10))
		);

		// Overflow in calculation: block_delta * borrow_interest_rate
		assert_noop!(
			TestController::calculate_interest_factor(Rate::from_inner(u128::max_value()), 20),
			Error::<TestRuntime>::NumOverflow
		);
	});
}

#[test]
fn borrow_balance_stored_with_zero_balance_should_work() {
	ExtBuilderNew::default()
		.init_pool(
			DOT,                                  // pool_id
			Balance::zero(),                      // borrowed
			Rate::saturating_from_rational(2, 1), // borrow_index
			Balance::zero(),                      // protocol_interest
		)
		.set_pool_user_data(
			DOT,             // pool_id
			ALICE,           // user
			Balance::zero(), // borrowed
			Rate::zero(),    // interest_index
			true,            // is_collateral
			0,               // liquidation_attempts
		)
		.build()
		.execute_with(|| {
			// If borrow_balance = 0 then borrow_index is likely also 0, return Ok(0)
			assert_eq!(TestController::borrow_balance_stored(&ALICE, DOT), Ok(Balance::zero()));
		});
}

#[test]
fn borrow_balance_stored_should_work() {
	ExtBuilderNew::default()
		.init_pool(
			DOT,                                  // pool_id
			Balance::zero(),                      // borrowed
			Rate::saturating_from_rational(2, 1), // borrow_index
			Balance::zero(),                      // protocol_interest
		)
		.set_pool_user_data(
			DOT,                                  // pool_id
			ALICE,                                // user
			100,                                  // borrowed
			Rate::saturating_from_rational(4, 1), // interest_index
			true,                                 // is_collateral
			0,                                    // liquidation_attempts
		)
		.build()
		.execute_with(|| {
			// recent_borrow_balance = 100 * 2 / 4 = 50
			assert_eq!(TestController::borrow_balance_stored(&ALICE, DOT), Ok(50));
		});
}

#[test]
fn borrow_balance_stored_fails_if_num_overflow() {
	ExtBuilderNew::default()
		.init_pool(
			DOT,                                  // pool_id
			Balance::zero(),                      // borrowed
			Rate::saturating_from_rational(2, 1), // borrow_index
			Balance::zero(),                      // protocol_interest
		)
		.set_pool_user_data(
			DOT,                                  // pool_id
			ALICE,                                // user
			Balance::max_value(),                 // borrowed
			Rate::saturating_from_rational(2, 1), // interest_index
			true,                                 // is_collateral
			0,                                    // liquidation_attempts
		)
		.init_pool(
			BTC,                                  // pool_id
			Balance::zero(),                      // borrowed
			Rate::saturating_from_rational(2, 1), // borrow_index
			Balance::zero(),                      // protocol_interest
		)
		.set_pool_user_data(
			DOT,          // pool_id
			ALICE,        // user
			100,          // borrowed
			Rate::zero(), // interest_index
			true,         // is_collateral
			0,            // liquidation_attempts
		)
		.build()
		.execute_with(|| {
			assert_noop!(
				TestController::borrow_balance_stored(&ALICE, DOT),
				Error::<TestRuntime>::BorrowBalanceOverflow
			);
		});
}

#[test]
fn calculate_utilization_rate_should_work() {
	ExtBuilderNew::default().build().execute_with(|| {
		// if current_total_borrowed_balance == 0 then return Ok(0)
		assert_eq!(TestController::calculate_utilization_rate(100, 0, 60), Ok(Rate::zero()));
		// utilization_rate = 80 / (22 + 80 - 2) = 0.8
		assert_eq!(
			TestController::calculate_utilization_rate(22, 80, 2),
			Ok(Rate::saturating_from_rational(8, 10))
		);

		// Overflow in calculation: total_balance + total_borrowed = max_value() + 80
		assert_noop!(
			TestController::calculate_utilization_rate(Balance::max_value(), 80, 2),
			Error::<TestRuntime>::UtilizationRateCalculationError
		);

		// Overflow in calculation:
		// total_balance_total_borrowed_sum - total_protocol_interest = ... - max_value()
		assert_noop!(
			TestController::calculate_utilization_rate(22, 80, Balance::max_value()),
			Error::<TestRuntime>::UtilizationRateCalculationError
		);

		// Overflow in calculation: total_borrows / 0
		assert_noop!(
			TestController::calculate_utilization_rate(100, 70, 170),
			Error::<TestRuntime>::UtilizationRateCalculationError
		);
	});
}

#[test]
fn get_hypothetical_account_liquidity_when_m_tokens_balance_is_zero_should_work() {
	ExtBuilderNew::default()
		.init_pool(
			DOT,                                  // pool_id
			Balance::zero(),                      // borrowed
			Rate::saturating_from_rational(2, 1), // borrow_index
			Balance::zero(),                      // protocol_interest
		)
		.init_pool(
			BTC,                                  // pool_id
			Balance::zero(),                      // borrowed
			Rate::saturating_from_rational(2, 1), // borrow_index
			Balance::zero(),                      // protocol_interest
		)
		.set_pool_user_data(
			DOT,             // pool_id
			ALICE,           // user
			Balance::zero(), // borrowed
			Rate::zero(),    // interest_index
			true,            // is_collateral
			0,               // liquidation_attempts
		)
		.set_pool_user_data(
			BTC,             // pool_id
			BOB,             // user
			Balance::zero(), // borrowed
			Rate::zero(),    // interest_index
			false,           // is_collateral
			0,               // liquidation_attempts
		)
		.set_controller_data(
			DOT,                                     // currency_id
			0,                                       // last_interest_accrued_block
			Rate::saturating_from_rational(1, 10),   // protocol_interest_factor
			Rate::saturating_from_rational(5, 1000), // max_borrow_rate
			Rate::saturating_from_rational(9, 10),   //collateral_factor
			None,                                    // borrow_cap
			PROTOCOL_INTEREST_TRANSFER_THRESHOLD,    // protocol_interest_threshold
		)
		.build()
		.execute_with(|| {
			// Set price = 2.00 USD for all assets.
			MockPriceSource::set_underlying_price(Some(Price::from_inner(2 * DOLLARS)));

			// Checking the function when called from redeem.
			// The function should return the shortfall to a large zero.
			assert_eq!(
				TestController::get_hypothetical_account_liquidity(&ALICE, DOT, 5, 0),
				Ok((0, 9))
			);
			// Checking the function when called from borrow.
			// The function should return the shortfall to a large zero.
			assert_eq!(
				TestController::get_hypothetical_account_liquidity(&ALICE, DOT, 0, 10),
				Ok((0, 20))
			);
			// Checking scenario: the user tries to take a borrow in a currency which is not
			// pool as available for collateral, and he fails.
			assert_eq!(
				TestController::get_hypothetical_account_liquidity(&BOB, BTC, 0, 10),
				Ok((0, 20))
			);
		});
}

#[test]
fn get_hypothetical_account_liquidity_one_currency_from_redeem_should_work() {
	ExtBuilderNew::default()
		.preset_alice_deposit_60_dot()
		.build()
		.execute_with(|| {
			// Set price = 2.00 USD for all assets.
			MockPriceSource::set_underlying_price(Some(Price::from_inner(2 * DOLLARS)));

			// Checking the function when called from redeem.
			// collateral parameter is set to false, user can't redeem.
			assert_eq!(
				TestController::get_hypothetical_account_liquidity(&ALICE, DOT, 5, 0),
				Ok((0, 9))
			);
			assert_eq!(
				TestController::get_hypothetical_account_liquidity(&ALICE, DOT, 60, 0),
				Ok((0, 108))
			);
			assert_eq!(
				TestController::get_hypothetical_account_liquidity(&ALICE, DOT, 200, 0),
				Ok((0, 360))
			);
		});
}

#[test]
fn get_hypothetical_account_liquidity_two_currencies_from_redeem_should_work() {
	ExtBuilderNew::default()
		.preset_alice_deposit_60_dot()
		.preset_alice_deposit_20_eth()
		.build()
		.execute_with(|| {
			// Set price = 2.00 USD for all assets.
			MockPriceSource::set_underlying_price(Some(Price::from_inner(2 * DOLLARS)));

			// Checking the function when called from redeem.
			// collateral parameter is set to false, user can't redeem.
			assert_eq!(
				TestController::get_hypothetical_account_liquidity(&ALICE, ETH, 15, 0),
				Ok((0, 27))
			);
			assert_eq!(
				TestController::get_hypothetical_account_liquidity(&ALICE, ETH, 80, 0),
				Ok((0, 144))
			);
			assert_eq!(
				TestController::get_hypothetical_account_liquidity(&ALICE, ETH, 100, 0),
				Ok((0, 180))
			);
		});
}

#[test]
fn get_hypothetical_account_liquidity_two_currencies_from_borrow_should_work() {
	ExtBuilderNew::default()
		.preset_alice_deposit_20_eth()
		// ALICE deposit 60 DOT and borrow 30 DOT
		.set_user_balance(ALICE, DOT, 70)
		.set_user_balance(ALICE, MDOT, 60)
		.set_pool_balance(DOT, 60)
		.init_pool(
			DOT,             // pool_id
			30,              // borrowed
			Rate::one(),     // borrow_index
			Balance::zero(), // protocol_interest
		)
		.set_pool_user_data(
			DOT,         // pool_id
			ALICE,       // user
			30,          // borrowed
			Rate::one(), // interest_index
			false,       // is_collateral
			0,           // liquidation_attempts
		)
		.set_controller_data(
			DOT,                                     // currency_id
			0,                                       // last_interest_accrued_block
			Rate::saturating_from_rational(1, 10),   // protocol_interest_factor
			Rate::saturating_from_rational(5, 1000), // max_borrow_rate
			Rate::saturating_from_rational(9, 10),   //collateral_factor
			None,                                    // borrow_cap
			PROTOCOL_INTEREST_TRANSFER_THRESHOLD,    // protocol_interest_threshold
		)
		.build()
		.execute_with(|| {
			// Set price = 2.00 USD for all assets.
			MockPriceSource::set_underlying_price(Some(Price::from_inner(2 * DOLLARS)));

			// Checking the function when called from borrow.
			// collateral parameter for DOT and ETH pool is set to false. User can't borrow.
			assert_eq!(
				TestController::get_hypothetical_account_liquidity(&ALICE, DOT, 0, 30),
				Ok((0, 120))
			);

			// Alice set collateral parameter value to true for DOT pool. Alice can borrow.
			TestPools::enable_is_collateral_internal(&ALICE, DOT);

			assert_eq!(
				TestController::get_hypothetical_account_liquidity(&ALICE, DOT, 0, 50),
				Ok((2, 0))
			);
			assert_eq!(
				TestController::get_hypothetical_account_liquidity(&ALICE, DOT, 0, 100),
				Ok((0, 98))
			);
		});
}

#[test]
fn get_liquidity_pool_exchange_rate_should_work() {
	ExtBuilderNew::default()
		.set_minterest_model_params(
			DOT,                                                // currency_id
			Rate::saturating_from_rational(8, 10),              // kink
			Rate::zero(),                                       // base_rate_per_block
			Rate::saturating_from_rational(9, 1_000_000_000),   // multiplier_per_block: 0.047304 PerYear
			Rate::saturating_from_rational(207, 1_000_000_000), // jump_multiplier_per_block: 1.09 PerYear
		)
		.init_pool(
			DOT,               // pool_id
			dollars(300_u128), // borrowed
			Rate::one(),       // borrow_index
			Balance::zero(),   // protocol_interest
		)
		.set_pool_balance(DOT, dollars(100_u128))
		.set_user_balance(ALICE, MDOT, dollars(125_u128))
		.set_controller_data(
			DOT,                                     // currency_id
			0,                                       // last_interest_accrued_block
			Rate::saturating_from_rational(1, 10),   // protocol_interest_factor
			Rate::saturating_from_rational(5, 1000), // max_borrow_rate
			Rate::saturating_from_rational(9, 10),   //collateral_factor
			None,                                    // borrow_cap
			PROTOCOL_INTEREST_TRANSFER_THRESHOLD,    // protocol_interest_threshold
		)
		.build()
		.execute_with(|| {
			// exchange_rate = (100 - 0 + 300) / 125 = 3.2
			assert_eq!(
				TestController::get_pool_exchange_borrow_and_supply_rates(DOT)
					.unwrap()
					.0,
				Rate::from_inner(3200000014580000000)
			);
			assert_eq!(TestController::get_pool_exchange_borrow_and_supply_rates(ETH), None);

			// Check that the exchange rate has changed. That is, the current value is returned.
			System::set_block_number(100);
			assert_eq!(
				TestController::get_pool_exchange_borrow_and_supply_rates(DOT)
					.unwrap()
					.0,
				Rate::from_inner(3200001458000000000)
			);
		});
}

#[test]
fn get_pool_exchange_borrow_and_supply_rates_less_than_kink() {
	ExtBuilderNew::default()
		.set_minterest_model_params(
			DOT,                                                // currency_id
			Rate::saturating_from_rational(8, 10),              // kink
			Rate::zero(),                                       // base_rate_per_block
			Rate::saturating_from_rational(9, 1_000_000_000),   // multiplier_per_block: 0.047304 PerYear
			Rate::saturating_from_rational(207, 1_000_000_000), // jump_multiplier_per_block: 1.09 PerYear
		)
		.init_pool(
			DOT,               // pool_id
			dollars(300_u128), // borrowed
			Rate::one(),       // borrow_index
			Balance::zero(),   // protocol_interest
		)
		.set_pool_balance(DOT, dollars(100_u128))
		.set_controller_data(
			DOT,                                     // currency_id
			0,                                       // last_interest_accrued_block
			Rate::saturating_from_rational(1, 10),   // protocol_interest_factor
			Rate::saturating_from_rational(5, 1000), // max_borrow_rate
			Rate::saturating_from_rational(9, 10),   //collateral_factor
			None,                                    // borrow_cap
			PROTOCOL_INTEREST_TRANSFER_THRESHOLD,    // protocol_interest_threshold
		)
		.build()
		.execute_with(|| {
			// utilization_rate = 300 / (100 - 0 + 300) = 0.75 < kink = 0.8
			// borrow_rate = 0.75 * 0.000_000_009 + 0 = 0.00000000675
			// supply_rate = 0.75 * 0.00_000_000_675 * (1 - 0.1) = 0.00000000455625
			assert_eq!(
				TestController::get_pool_exchange_borrow_and_supply_rates(DOT),
				Some((Rate::one(), Rate::from_inner(6750000014), Rate::from_inner(4556250018)))
			);

			assert_eq!(TestController::get_pool_exchange_borrow_and_supply_rates(ETH), None);
		});
}

#[test]
fn get_pool_exchange_borrow_and_supply_rates_above_kink() {
	ExtBuilderNew::default()
		.set_minterest_model_params(
			DOT,                                                // currency_id
			Rate::saturating_from_rational(8, 10),              // kink
			Rate::zero(),                                       // base_rate_per_block
			Rate::saturating_from_rational(9, 1_000_000_000),   // multiplier_per_block: 0.047304 PerYear
			Rate::saturating_from_rational(207, 1_000_000_000), // jump_multiplier_per_block: 1.09 PerYear
		)
		.init_pool(
			DOT,               // pool_id
			dollars(500_u128), // borrowed
			Rate::one(),       // borrow_index
			Balance::zero(),   // protocol_interest
		)
		.set_pool_balance(DOT, dollars(100_u128))
		.set_controller_data(
			DOT,                                     // currency_id
			0,                                       // last_interest_accrued_block
			Rate::saturating_from_rational(1, 10),   // protocol_interest_factor
			Rate::saturating_from_rational(5, 1000), // max_borrow_rate
			Rate::saturating_from_rational(9, 10),   //collateral_factor
			None,                                    // borrow_cap
			PROTOCOL_INTEREST_TRANSFER_THRESHOLD,    // protocol_interest_threshold
		)
		.build()
		.execute_with(|| {
			// utilization_rate = 500 / (100 - 0 + 500) = 0.833 > kink = 0.8
			// borrow_rate = 0.833 * 0.8 * 0.000_000_207  + (0.8 * 0.000_000_009) + 0 = 0.0000001452
			// supply_rate = 0.833 * 0.0000001452 * (1 - 0.1) = 0,0000001089
			assert_eq!(
				TestController::get_pool_exchange_borrow_and_supply_rates(DOT),
				Some((
					Rate::one(),
					Rate::from_inner(145200005009),
					Rate::from_inner(108900007709)
				))
			);
		});
}

#[test]
fn redeem_allowed_should_work() {
	ExtBuilderNew::default()
		.preset_alice_deposit_60_dot()
		.build()
		.execute_with(|| {
			assert_ok!(TestController::redeem_allowed(DOT, &ALICE, dollars(40_u128)));

			// collateral parameter is set to true.
			TestPools::enable_is_collateral_internal(&ALICE, DOT);

			assert_err!(
				TestController::redeem_allowed(DOT, &ALICE, dollars(100_u128)),
				Error::<TestRuntime>::InsufficientLiquidity
			);
		});
}

#[test]
fn borrow_allowed_should_work() {
	ExtBuilderNew::default()
		.preset_alice_deposit_60_dot()
		.build()
		.execute_with(|| {
			// collateral parameter is set to false. User can't borrow
			assert_err!(
				TestController::borrow_allowed(DOT, &ALICE, dollars(10_u128)),
				Error::<TestRuntime>::InsufficientLiquidity
			);

			// collateral parameter is set to true. User can borrow.
			TestPools::enable_is_collateral_internal(&ALICE, DOT);

			assert_ok!(TestController::borrow_allowed(DOT, &ALICE, dollars(10_u128)));

			assert_noop!(
				TestController::borrow_allowed(DOT, &ALICE, dollars(999_u128)),
				Error::<TestRuntime>::InsufficientLiquidity
			);
		});
}

#[test]
fn is_operation_allowed_should_work() {
	ExtBuilderNew::default()
		.init_pool(
			DOT,                                  // pool_id
			Balance::zero(),                      // borrowed
			Rate::saturating_from_rational(2, 1), // borrow_index
			Balance::zero(),                      // protocol_interest
		)
		.set_pause_keeper(DOT, false)
		.build()
		.execute_with(|| {
			assert!(TestController::is_operation_allowed(DOT, Operation::Deposit));
			assert!(TestController::is_operation_allowed(DOT, Operation::Redeem));
			assert!(TestController::is_operation_allowed(DOT, Operation::Borrow));
			assert!(TestController::is_operation_allowed(DOT, Operation::Repay));

			assert_ok!(TestController::pause_operation(alice_origin(), DOT, Operation::Deposit));
			assert_ok!(TestController::pause_operation(alice_origin(), DOT, Operation::Redeem));

			assert!(!TestController::is_operation_allowed(DOT, Operation::Deposit));
			assert!(!TestController::is_operation_allowed(DOT, Operation::Redeem));
			assert!(TestController::is_operation_allowed(DOT, Operation::Borrow));
			assert!(TestController::is_operation_allowed(DOT, Operation::Repay));
		});
}

/* ----------------------------------------------------------------------------------------- */

// Admin functions

#[test]
fn set_protocol_interest_factor_should_work() {
	ExtBuilderNew::default()
		.init_pool(
			DOT,                                  // pool_id
			Balance::zero(),                      // borrowed
			Rate::saturating_from_rational(2, 1), // borrow_index
			Balance::zero(),                      // protocol_interest
		)
		.build()
		.execute_with(|| {
			// ALICE set protocol interest factor equal 2.0
			assert_ok!(TestController::set_protocol_interest_factor(
				alice_origin(),
				DOT,
				Rate::saturating_from_integer(2)
			));
			let expected_event = Event::controller(test_engine::module::Event::InterestFactorChanged);
			assert!(System::events().iter().any(|record| record.event == expected_event));
			assert_eq!(
				TestController::controller_dates(DOT).protocol_interest_factor,
				Rate::saturating_from_rational(20, 10)
			);

			// ALICE set protocol interest factor equal 0.0
			assert_ok!(TestController::set_protocol_interest_factor(
				alice_origin(),
				DOT,
				Rate::zero()
			));
			let expected_event = Event::controller(test_engine::module::Event::InterestFactorChanged);
			assert!(System::events().iter().any(|record| record.event == expected_event));
			assert_eq!(
				TestController::controller_dates(DOT).protocol_interest_factor,
				Rate::zero()
			);

			// The dispatch origin of this call must be Root or half MinterestCouncil.
			assert_noop!(
				TestController::set_protocol_interest_factor(bob_origin(), DOT, Rate::saturating_from_integer(2)),
				BadOrigin
			);

			assert_noop!(
				TestController::set_protocol_interest_factor(alice_origin(), MDOT, Rate::saturating_from_integer(2)),
				Error::<TestRuntime>::PoolNotFound
			);
		});
}

#[test]
fn set_max_borrow_rate_should_work() {
	ExtBuilderNew::default()
		.init_pool(
			DOT,                                  // pool_id
			Balance::zero(),                      // borrowed
			Rate::saturating_from_rational(2, 1), // borrow_index
			Balance::zero(),                      // protocol_interest
		)
		.build()
		.execute_with(|| {
			// ALICE set max borrow rate equal 2.0
			assert_ok!(TestController::set_max_borrow_rate(
				alice_origin(),
				DOT,
				Rate::saturating_from_integer(2)
			));
			let expected_event = Event::controller(test_engine::module::Event::MaxBorrowRateChanged);
			assert!(System::events().iter().any(|record| record.event == expected_event));
			assert_eq!(
				TestController::controller_dates(DOT).max_borrow_rate,
				Rate::saturating_from_rational(20, 10)
			);

			// ALICE can't set max borrow rate equal 0.0
			assert_noop!(
				TestController::set_max_borrow_rate(alice_origin(), DOT, Rate::zero()),
				Error::<TestRuntime>::MaxBorrowRateCannotBeZero
			);

			// The dispatch origin of this call must be Root or half MinterestCouncil.
			assert_noop!(
				TestController::set_max_borrow_rate(bob_origin(), DOT, Rate::saturating_from_integer(2)),
				BadOrigin
			);

			assert_noop!(
				TestController::set_max_borrow_rate(alice_origin(), MDOT, Rate::saturating_from_integer(2)),
				Error::<TestRuntime>::PoolNotFound
			);
		});
}

#[test]
fn set_collateral_factor_should_work() {
	ExtBuilderNew::default()
		.init_pool(
			DOT,                                  // pool_id
			Balance::zero(),                      // borrowed
			Rate::saturating_from_rational(2, 1), // borrow_index
			Balance::zero(),                      // protocol_interest
		)
		.build()
		.execute_with(|| {
			// ALICE set collateral factor equal 0.5.
			assert_ok!(TestController::set_collateral_factor(
				alice_origin(),
				DOT,
				Rate::saturating_from_rational(1, 2)
			));
			let expected_event = Event::controller(test_engine::module::Event::CollateralFactorChanged);
			assert!(System::events().iter().any(|record| record.event == expected_event));
			assert_eq!(
				TestController::controller_dates(DOT).collateral_factor,
				Rate::saturating_from_rational(1, 2)
			);

			// ALICE can't set collateral factor equal 0.0
			assert_noop!(
				TestController::set_collateral_factor(alice_origin(), DOT, Rate::zero()),
				Error::<TestRuntime>::CollateralFactorIncorrectValue
			);

			// ALICE can't set collateral factor grater than one.
			assert_noop!(
				TestController::set_collateral_factor(alice_origin(), DOT, Rate::saturating_from_rational(11, 10)),
				Error::<TestRuntime>::CollateralFactorIncorrectValue
			);

			// The dispatch origin of this call must be Root or half MinterestCouncil.
			assert_noop!(
				TestController::set_collateral_factor(bob_origin(), DOT, Rate::saturating_from_integer(2)),
				BadOrigin
			);

			// Unavailable currency id.
			assert_noop!(
				TestController::set_collateral_factor(alice_origin(), MDOT, Rate::saturating_from_integer(2)),
				Error::<TestRuntime>::PoolNotFound
			);
		});
}

#[test]
fn pause_operation_should_work() {
	ExtBuilderNew::default()
		.init_pool(
			DOT,                                  // pool_id
			Balance::zero(),                      // borrowed
			Rate::saturating_from_rational(2, 1), // borrow_index
			Balance::zero(),                      // protocol_interest
		)
		.set_pause_keeper(DOT, false)
		.build()
		.execute_with(|| {
			assert!(!TestController::pause_keepers(&DOT).deposit_paused);
			assert!(!TestController::pause_keepers(&DOT).redeem_paused);
			assert!(!TestController::pause_keepers(&DOT).borrow_paused);
			assert!(!TestController::pause_keepers(&DOT).repay_paused);
			assert!(!TestController::pause_keepers(&DOT).transfer_paused);

			assert_ok!(TestController::pause_operation(alice_origin(), DOT, Operation::Deposit));
			let expected_event =
				Event::controller(test_engine::module::Event::OperationIsPaused(DOT, Operation::Deposit));
			assert!(System::events().iter().any(|record| record.event == expected_event));

			assert_ok!(TestController::pause_operation(alice_origin(), DOT, Operation::Redeem));
			let expected_event =
				Event::controller(test_engine::module::Event::OperationIsPaused(DOT, Operation::Redeem));
			assert!(System::events().iter().any(|record| record.event == expected_event));

			assert_ok!(TestController::pause_operation(alice_origin(), DOT, Operation::Borrow));
			let expected_event =
				Event::controller(test_engine::module::Event::OperationIsPaused(DOT, Operation::Borrow));
			assert!(System::events().iter().any(|record| record.event == expected_event));

			assert_ok!(TestController::pause_operation(alice_origin(), DOT, Operation::Repay));
			let expected_event =
				Event::controller(test_engine::module::Event::OperationIsPaused(DOT, Operation::Repay));
			assert!(System::events().iter().any(|record| record.event == expected_event));

			assert_ok!(TestController::pause_operation(
				alice_origin(),
				DOT,
				Operation::Transfer
			));
			let expected_event =
				Event::controller(test_engine::module::Event::OperationIsPaused(DOT, Operation::Transfer));
			assert!(System::events().iter().any(|record| record.event == expected_event));

			assert!(TestController::pause_keepers(&DOT).deposit_paused);
			assert!(TestController::pause_keepers(&DOT).redeem_paused);
			assert!(TestController::pause_keepers(&DOT).borrow_paused);
			assert!(TestController::pause_keepers(&DOT).repay_paused);
			assert!(TestController::pause_keepers(&DOT).transfer_paused);

			assert_noop!(
				TestController::pause_operation(bob_origin(), DOT, Operation::Deposit),
				BadOrigin
			);
			assert_noop!(
				TestController::pause_operation(alice_origin(), MDOT, Operation::Redeem),
				Error::<TestRuntime>::PoolNotFound
			);
		});
}

#[test]
fn resume_operation_should_work() {
	ExtBuilderNew::default()
		.init_pool(
			DOT,                                  // pool_id
			Balance::zero(),                      // borrowed
			Rate::saturating_from_rational(2, 1), // borrow_index
			Balance::zero(),                      // protocol_interest
		)
		.init_pool(
			KSM,                                  // pool_id
			Balance::zero(),                      // borrowed
			Rate::saturating_from_rational(2, 1), // borrow_index
			Balance::zero(),                      // protocol_interest
		)
		.set_pause_keeper(KSM, true)
		.build()
		.execute_with(|| {
			assert!(TestController::pause_keepers(&KSM).deposit_paused);
			assert!(TestController::pause_keepers(&KSM).redeem_paused);
			assert!(TestController::pause_keepers(&KSM).borrow_paused);
			assert!(TestController::pause_keepers(&KSM).repay_paused);
			assert!(TestController::pause_keepers(&KSM).transfer_paused);

			assert_ok!(TestController::resume_operation(
				alice_origin(),
				KSM,
				Operation::Deposit
			));
			let expected_event =
				Event::controller(test_engine::module::Event::OperationIsUnPaused(KSM, Operation::Deposit));
			assert!(System::events().iter().any(|record| record.event == expected_event));

			assert_ok!(TestController::resume_operation(alice_origin(), KSM, Operation::Redeem));
			let expected_event =
				Event::controller(test_engine::module::Event::OperationIsUnPaused(KSM, Operation::Redeem));
			assert!(System::events().iter().any(|record| record.event == expected_event));

			assert_ok!(TestController::resume_operation(alice_origin(), KSM, Operation::Borrow));
			let expected_event =
				Event::controller(test_engine::module::Event::OperationIsUnPaused(KSM, Operation::Borrow));
			assert!(System::events().iter().any(|record| record.event == expected_event));

			assert_ok!(TestController::resume_operation(alice_origin(), KSM, Operation::Repay));
			let expected_event =
				Event::controller(test_engine::module::Event::OperationIsUnPaused(KSM, Operation::Repay));
			assert!(System::events().iter().any(|record| record.event == expected_event));

			assert_ok!(TestController::resume_operation(
				alice_origin(),
				KSM,
				Operation::Transfer
			));
			let expected_event = Event::controller(test_engine::module::Event::OperationIsUnPaused(
				KSM,
				Operation::Transfer,
			));
			assert!(System::events().iter().any(|record| record.event == expected_event));

			assert!(!TestController::pause_keepers(&KSM).deposit_paused);
			assert!(!TestController::pause_keepers(&KSM).redeem_paused);
			assert!(!TestController::pause_keepers(&KSM).borrow_paused);
			assert!(!TestController::pause_keepers(&KSM).repay_paused);
			assert!(!TestController::pause_keepers(&KSM).transfer_paused);

			assert_noop!(
				TestController::resume_operation(bob_origin(), DOT, Operation::Deposit),
				BadOrigin
			);
			assert_noop!(
				TestController::resume_operation(alice_origin(), MDOT, Operation::Redeem),
				Error::<TestRuntime>::PoolNotFound
			);
		});
}

#[test]
fn set_borrow_cap_should_work() {
	ExtBuilderNew::default()
		.init_pool(
			DOT,                                  // pool_id
			Balance::zero(),                      // borrowed
			Rate::saturating_from_rational(2, 1), // borrow_index
			Balance::zero(),                      // protocol_interest
		)
		.set_user_balance(ALICE, DOT, ONE_HUNDRED)
		.build()
		.execute_with(|| {
			// The dispatch origin of this call must be Administrator.
			assert_noop!(
				TestController::set_borrow_cap(bob_origin(), DOT, Some(dollars(10))),
				BadOrigin
			);

			// ALICE set borrow cap to 10.
			assert_ok!(TestController::set_borrow_cap(alice_origin(), DOT, Some(dollars(10))));
			let expected_event =
				Event::controller(test_engine::module::Event::BorrowCapChanged(DOT, Some(dollars(10))));
			assert!(System::events().iter().any(|record| record.event == expected_event));

			// ALICE is able to change borrow cap to 9999
			assert_ok!(TestController::set_borrow_cap(alice_origin(), DOT, Some(9999_u128)));
			let expected_event = Event::controller(test_engine::module::Event::BorrowCapChanged(DOT, Some(9999_u128)));
			assert!(System::events().iter().any(|record| record.event == expected_event));

			// Unable to set borrow cap greater than MAX_BORROW_CAP.
			assert_noop!(
				TestController::set_borrow_cap(alice_origin(), DOT, Some(dollars(1_000_001_u128))),
				Error::<TestRuntime>::InvalidBorrowCap
			);

			// Alice is able to set zero borrow cap.
			assert_ok!(TestController::set_borrow_cap(alice_origin(), DOT, Some(0_u128)));
			let expected_event = Event::controller(test_engine::module::Event::BorrowCapChanged(DOT, Some(0_u128)));
			assert!(System::events().iter().any(|record| record.event == expected_event));
		});
}

#[test]
fn set_protocol_interest_threshold_should_work() {
	ExtBuilderNew::default()
		.init_pool(
			DOT,                                  // pool_id
			Balance::zero(),                      // borrowed
			Rate::saturating_from_rational(2, 1), // borrow_index
			Balance::zero(),                      // protocol_interest
		)
		.set_user_balance(ALICE, DOT, ONE_HUNDRED)
		.build()
		.execute_with(|| {
			// The dispatch origin of this call must be Administrator.
			assert_noop!(
				TestController::set_protocol_interest_threshold(bob_origin(), DOT, 10_u128),
				BadOrigin
			);

			// ALICE set protocol interest threshold to 10.
			assert_ok!(TestController::set_protocol_interest_threshold(
				alice_origin(),
				DOT,
				10_u128
			));
			let expected_event = Event::controller(test_engine::module::Event::ProtocolInterestThresholdChanged(
				DOT, 10_u128,
			));
			assert!(System::events().iter().any(|record| record.event == expected_event));

			// Alice is able to set zero protocol interest threshold.
			assert_ok!(TestController::set_protocol_interest_threshold(
				alice_origin(),
				DOT,
				0_u128
			));
			let expected_event = Event::controller(test_engine::module::Event::ProtocolInterestThresholdChanged(
				DOT, 0_u128,
			));
			assert!(System::events().iter().any(|record| record.event == expected_event));
		});
}
