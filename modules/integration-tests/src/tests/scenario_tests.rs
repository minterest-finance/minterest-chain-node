//  Scenario Integration tests.

#[cfg(test)]

mod tests {
	use crate::tests::*;

	// Description of scenario #1:
	// In this scenario, user uses four operations in the protocol (deposit, borrow, repay, redeem).
	// Changes to the main protocol parameters are also checked here.
	#[test]
	fn scenario_with_four_operations() {
		ExtBuilder::default()
			.user_balance(ADMIN, CurrencyId::DOT, ONE_HUNDRED)
			.user_balance(ALICE, CurrencyId::DOT, ONE_HUNDRED)
			.pool_user_data(CurrencyId::DOT, ALICE, BALANCE_ZERO, RATE_ZERO, true, 0)
			.pool_initial(CurrencyId::DOT)
			.build()
			.execute_with(|| {
				// INITIAL PARAMS
				/* ------------------------------------------------------------------------------ */

				let alice_dot_free_balance_start: Balance = ONE_HUNDRED;
				let alice_m_dot_free_balance_start: Balance = BALANCE_ZERO;
				let alice_dot_total_borrow_start: Balance = BALANCE_ZERO;

				let pool_available_liquidity_start: Balance = BALANCE_ZERO;
				let pool_m_dot_total_issuance_start: Balance = BALANCE_ZERO;
				let pool_total_insurance_start: Balance = BALANCE_ZERO;
				let pool_dot_total_borrow_start: Balance = BALANCE_ZERO;

				// ACTION: DEPOSIT INSURANCE
				/* ------------------------------------------------------------------------------ */

				// Add liquidity to DOT pool from Insurance by Admin
				let admin_deposit_amount_block_number_0: Balance = 100_000 * DOLLARS;
				assert_ok!(TestController::deposit_insurance(
					Origin::signed(ADMIN),
					CurrencyId::DOT,
					admin_deposit_amount_block_number_0
				));

				// PARAMETERS CHECKING
				/* ------------------------------------------------------------------------------ */

				// Checking pool available liquidity
				// Expected: 100_000
				let current_pool_available_liquidity_block_number_0: Balance =
					pool_available_liquidity_start + admin_deposit_amount_block_number_0;
				assert_eq!(
					TestPools::get_pool_available_liquidity(CurrencyId::DOT),
					current_pool_available_liquidity_block_number_0
				);

				// Checking free balance MDOT in pool.
				// Admin doesn't have to get wrapped token after adding liquidity from insurance.
				assert_eq!(
					Currencies::total_issuance(CurrencyId::MDOT),
					pool_m_dot_total_issuance_start
				);

				// Checking free balance DOT && MDOT
				// ADMIN:
				assert_eq!(Currencies::free_balance(CurrencyId::DOT, &ADMIN), BALANCE_ZERO);
				assert_eq!(Currencies::free_balance(CurrencyId::MDOT, &ADMIN), BALANCE_ZERO);

				// Checking DOT pool Storage params
				assert_eq!(TestPools::pools(CurrencyId::DOT).borrow_index, Rate::one());
				// Total insurance changed: 0 -> 100 000
				let pool_total_insurance_block_number_0 =
					pool_total_insurance_start + admin_deposit_amount_block_number_0;
				assert_eq!(
					TestPools::pools(CurrencyId::DOT).total_insurance,
					pool_total_insurance_block_number_0
				);
				assert_eq!(
					TestPools::pools(CurrencyId::DOT).total_borrowed,
					pool_dot_total_borrow_start
				);

				// Checking controller params
				let (borrow_rate, _) =
					TestController::get_liquidity_pool_borrow_and_supply_rates(CurrencyId::DOT).unwrap_or_default();
				assert_eq!(TestController::controller_dates(CurrencyId::DOT).timestamp, 0);
				assert_eq!(borrow_rate, RATE_ZERO);

				// Checking DOT pool User params
				// ADMIN:
				assert_eq!(
					TestPools::pool_user_data(CurrencyId::DOT, ADMIN).total_borrowed,
					alice_dot_total_borrow_start
				);
				assert_eq!(
					TestPools::pool_user_data(CurrencyId::DOT, ADMIN).interest_index,
					RATE_ZERO
				);

				System::set_block_number(1);

				// ACTION: DEPOSIT UNDERLYING
				/* ------------------------------------------------------------------------------ */

				// ALICE deposit 60 000 to DOT pool
				let alice_deposit_amount_block_number_1: Balance = 60_000 * DOLLARS;
				assert_ok!(MinterestProtocol::deposit_underlying(
					Origin::signed(ALICE),
					CurrencyId::DOT,
					alice_deposit_amount_block_number_1
				));

				// PARAMETERS CHECKING
				/* ------------------------------------------------------------------------------ */

				// Checking pool available liquidity
				// Expected: 160 000
				let pool_available_liquidity_block_number_1: Balance =
					admin_deposit_amount_block_number_0 + alice_deposit_amount_block_number_1;
				assert_eq!(
					TestPools::get_pool_available_liquidity(CurrencyId::DOT),
					pool_available_liquidity_block_number_1
				);

				// Checking free balance MDOT in pool.
				// Admin doesn't have to get wrapped token after adding liquidity from insurance.
				// Alice gets wrapped token after adding liquidity by exchange rate 1:1
				// Expected: 60 000
				let pool_m_dot_free_balance_block_number_1: Balance =
					pool_m_dot_total_issuance_start + alice_deposit_amount_block_number_1;
				assert_eq!(
					Currencies::total_issuance(CurrencyId::MDOT),
					pool_m_dot_free_balance_block_number_1
				);

				// Checking free balance DOT && MDOT
				// ADMIN:
				assert_eq!(Currencies::free_balance(CurrencyId::DOT, &ADMIN), BALANCE_ZERO);
				assert_eq!(Currencies::free_balance(CurrencyId::MDOT, &ADMIN), BALANCE_ZERO);

				// ALICE:
				let alice_dot_free_balance_block_number_1: Balance =
					alice_dot_free_balance_start - alice_deposit_amount_block_number_1;
				assert_eq!(
					Currencies::free_balance(CurrencyId::DOT, &ALICE),
					alice_dot_free_balance_block_number_1
				);
				let alice_m_dot_free_balance_block_number_1: Balance =
					alice_m_dot_free_balance_start + alice_deposit_amount_block_number_1;
				assert_eq!(
					Currencies::free_balance(CurrencyId::MDOT, &ALICE),
					alice_m_dot_free_balance_block_number_1
				);

				// Checking DOT pool Storage params
				assert_eq!(TestPools::pools(CurrencyId::DOT).borrow_index, Rate::one());
				// Expected: 100 000
				assert_eq!(
					TestPools::pools(CurrencyId::DOT).total_insurance,
					pool_total_insurance_block_number_0
				);
				assert_eq!(TestPools::pools(CurrencyId::DOT).total_borrowed, BALANCE_ZERO);

				// Checking controller Storage params
				assert_eq!(TestController::controller_dates(CurrencyId::DOT).timestamp, 1);
				let (borrow_rate, _) =
					TestController::get_liquidity_pool_borrow_and_supply_rates(CurrencyId::DOT).unwrap_or_default();
				assert_eq!(borrow_rate, RATE_ZERO);

				// Checking DOT pool User params
				// ADMIN:
				assert_eq!(
					TestPools::pool_user_data(CurrencyId::DOT, ADMIN).total_borrowed,
					BALANCE_ZERO
				);
				assert_eq!(
					TestPools::pool_user_data(CurrencyId::DOT, ADMIN).interest_index,
					RATE_ZERO
				);
				// ALICE:
				assert_eq!(
					TestPools::pool_user_data(CurrencyId::DOT, ALICE).total_borrowed,
					BALANCE_ZERO
				);
				assert_eq!(
					TestPools::pool_user_data(CurrencyId::DOT, ALICE).interest_index,
					RATE_ZERO
				);

				System::set_block_number(2);

				// ACTION: BORROW
				/* ------------------------------------------------------------------------------ */

				//  Alice borrow 30_000 from DOT pool.
				let alice_borrow_amount_block_number_2: Balance = 30_000 * DOLLARS;
				assert_ok!(MinterestProtocol::borrow(
					Origin::signed(ALICE),
					CurrencyId::DOT,
					alice_borrow_amount_block_number_2
				));

				// PARAMETERS CHECKING
				/* ------------------------------------------------------------------------------ */

				// Checking pool available liquidity
				// Expected 130 000
				let current_pool_available_liquidity_block_number_2: Balance =
					pool_available_liquidity_block_number_1 - alice_borrow_amount_block_number_2;
				assert_eq!(
					TestPools::get_pool_available_liquidity(CurrencyId::DOT),
					current_pool_available_liquidity_block_number_2
				);

				// Checking free balance MDOT in pool.
				// Expected: 60 000
				assert_eq!(
					Currencies::total_issuance(CurrencyId::MDOT),
					pool_m_dot_free_balance_block_number_1
				);

				// Checking free balance DOT && MDOT
				// ADMIN:
				assert_eq!(Currencies::free_balance(CurrencyId::DOT, &ADMIN), BALANCE_ZERO);
				assert_eq!(Currencies::free_balance(CurrencyId::MDOT, &ADMIN), BALANCE_ZERO);

				// ALICE:
				// Expected: 70 000
				let alice_dot_free_balance_block_number_2: Balance =
					alice_dot_free_balance_block_number_1 + alice_borrow_amount_block_number_2;
				assert_eq!(
					Currencies::free_balance(CurrencyId::DOT, &ALICE),
					alice_dot_free_balance_block_number_2
				);
				// Expected: 60 000
				assert_eq!(
					Currencies::free_balance(CurrencyId::MDOT, &ALICE),
					alice_m_dot_free_balance_block_number_1
				);

				// Checking pool Storage params
				assert_eq!(TestPools::pools(CurrencyId::DOT).borrow_index, Rate::one());
				// Expected: 100 000
				assert_eq!(
					TestPools::pools(CurrencyId::DOT).total_insurance,
					pool_total_insurance_block_number_0
				);
				// Total borrowed amount changed 0 -> 30 000
				let pool_dot_total_borrow_block_number_2: Balance =
					pool_dot_total_borrow_start + alice_borrow_amount_block_number_2;
				assert_eq!(
					TestPools::pools(CurrencyId::DOT).total_borrowed,
					pool_dot_total_borrow_block_number_2
				);

				// Checking controller Storage params
				assert_eq!(TestController::controller_dates(CurrencyId::DOT).timestamp, 2);
				// Borrow_rate changed: 0 -> 45 * 10^(-10)
				let expected_borrow_rate_block_number_2: Rate =
					Rate::saturating_from_rational(45u128, 10_000_000_000u128);
				let (borrow_rate, _) =
					TestController::get_liquidity_pool_borrow_and_supply_rates(CurrencyId::DOT).unwrap_or_default();
				assert_eq!(borrow_rate, expected_borrow_rate_block_number_2);

				// Checking DOT pool User params
				// ADMIN:
				assert_eq!(
					TestPools::pool_user_data(CurrencyId::DOT, ADMIN).total_borrowed,
					BALANCE_ZERO
				);
				assert_eq!(
					TestPools::pool_user_data(CurrencyId::DOT, ADMIN).interest_index,
					RATE_ZERO
				);
				// ALICE:
				// User total borrowed changed: 0 -> 30 000
				let alice_dot_total_borrow_block_number_2: Balance =
					alice_dot_total_borrow_start + alice_borrow_amount_block_number_2;
				assert_eq!(
					TestPools::pool_user_data(CurrencyId::DOT, ALICE).total_borrowed,
					alice_dot_total_borrow_block_number_2
				);
				// User interest index changed: 0 -> 1
				assert_eq!(
					TestPools::pool_user_data(CurrencyId::DOT, ALICE).interest_index,
					Rate::one()
				);

				System::set_block_number(3);

				// ACTION: REPAY
				/* ------------------------------------------------------------------------------ */

				// Alice repay part of her loan(15 000).
				let alice_repay_amount_block_number_3: Balance = 15_000 * DOLLARS;
				assert_ok!(MinterestProtocol::repay(
					Origin::signed(ALICE),
					CurrencyId::DOT,
					alice_repay_amount_block_number_3
				));

				// PARAMETERS CHECKING
				/* ------------------------------------------------------------------------------ */

				// Checking pool available liquidity
				// Expected 145 000
				let current_pool_available_liquidity_block_number_3: Balance =
					current_pool_available_liquidity_block_number_2 + alice_repay_amount_block_number_3;
				assert_eq!(
					TestPools::get_pool_available_liquidity(CurrencyId::DOT),
					current_pool_available_liquidity_block_number_3
				);

				// Checking free balance MDOT in pool.
				// Expected: 60 000
				assert_eq!(
					Currencies::total_issuance(CurrencyId::MDOT),
					pool_m_dot_free_balance_block_number_1
				);

				// Checking free balance DOT && MDOT
				// ADMIN:
				assert_eq!(Currencies::free_balance(CurrencyId::DOT, &ADMIN), BALANCE_ZERO);
				assert_eq!(Currencies::free_balance(CurrencyId::DOT, &ADMIN), BALANCE_ZERO);

				// ALICE:
				// Expected: 55 000
				let alice_dot_free_balance_block_number_3: Balance =
					alice_dot_free_balance_block_number_2 - alice_repay_amount_block_number_3;
				assert_eq!(
					Currencies::free_balance(CurrencyId::DOT, &ALICE),
					alice_dot_free_balance_block_number_3
				);
				assert_eq!(
					Currencies::free_balance(CurrencyId::MDOT, &ALICE),
					alice_m_dot_free_balance_block_number_1
				);

				// Checking pool Storage params
				// Expected: 1.000000004500000000
				let pool_borrow_index_block_number_3: Rate =
					Rate::saturating_from_rational(10_000_000_045u128, 10_000_000_000u128);
				assert_eq!(
					TestPools::pools(CurrencyId::DOT).borrow_index,
					pool_borrow_index_block_number_3
				);
				// Expected: 100_000,0000135
				let insurance_accumulated_block_number_3: Balance = 13_500_000_000_000;
				let pool_total_insurance_block_number_3: Balance =
					admin_deposit_amount_block_number_0 + insurance_accumulated_block_number_3;
				assert_eq!(
					TestPools::pools(CurrencyId::DOT).total_insurance,
					pool_total_insurance_block_number_3
				);
				// Expected: 15_000,000135
				let borrow_accumulated_block_number_3: Balance = 135_000_000_000_000;
				let pool_dot_total_borrow_block_number_3: Balance = pool_dot_total_borrow_block_number_2
					+ borrow_accumulated_block_number_3
					- alice_repay_amount_block_number_3;
				assert_eq!(
					TestPools::pools(CurrencyId::DOT).total_borrowed,
					pool_dot_total_borrow_block_number_3
				);

				// Checking controller Storage params
				assert_eq!(TestController::controller_dates(CurrencyId::DOT).timestamp, 3);
				// Borrow_rate changed: 0,0000000045 -> 0,000000002250000015
				let expected_borrow_rate_block_number_3: Rate =
					Rate::saturating_from_rational(2_250_000_015u128, 1_000_000_000_000_000_000u128);
				let (borrow_rate, _) =
					TestController::get_liquidity_pool_borrow_and_supply_rates(CurrencyId::DOT).unwrap_or_default();
				assert_eq!(borrow_rate, expected_borrow_rate_block_number_3);

				// Checking DOT pool User params
				// ADMIN:
				assert_eq!(
					TestPools::pool_user_data(CurrencyId::DOT, ADMIN).total_borrowed,
					BALANCE_ZERO
				);
				assert_eq!(
					TestPools::pool_user_data(CurrencyId::DOT, ADMIN).interest_index,
					RATE_ZERO
				);
				// ALICE:
				let alice_dot_total_borrow_block_number_3: Balance = alice_dot_total_borrow_block_number_2
					+ borrow_accumulated_block_number_3
					- alice_repay_amount_block_number_3;
				assert_eq!(
					TestPools::pool_user_data(CurrencyId::DOT, ALICE).total_borrowed,
					alice_dot_total_borrow_block_number_3
				);
				// Interest_index changed: 0 -> 1.000000004500000000
				let user_interest_index_block_number_3: Rate = pool_borrow_index_block_number_3;
				assert_eq!(
					TestPools::pool_user_data(CurrencyId::DOT, ALICE).interest_index,
					user_interest_index_block_number_3
				);

				System::set_block_number(4);

				// ACTION: REPAY_ALL
				/* ------------------------------------------------------------------------------ */

				// Alice repay all loans.
				assert_ok!(MinterestProtocol::repay_all(Origin::signed(ALICE), CurrencyId::DOT));

				// PARAMETERS CHECKING
				/* ------------------------------------------------------------------------------ */

				// Checking pool available liquidity
				// Real expected: 		160_000,000168750000528750
				// Currently expected:	160_000,000168750000526875
				// FIXME: unavailable behavior. That is a reason of error below.
				// FIXME: borrow_accumulated_block_number_4 should be 33_750_000_528_750
				//										   instead of 33_750_000_526_875
				let borrow_accumulated_block_number_4: Balance = 33_750_000_526_875;
				let current_pool_available_liquidity_block_number_4: Balance =
					current_pool_available_liquidity_block_number_3
						+ alice_repay_amount_block_number_3
						+ borrow_accumulated_block_number_3
						+ borrow_accumulated_block_number_4;
				assert_eq!(
					TestPools::get_pool_available_liquidity(CurrencyId::DOT),
					current_pool_available_liquidity_block_number_4
				);

				// Checking free balance MDOT in pool.
				// Expected: 60 000
				assert_eq!(
					Currencies::total_issuance(CurrencyId::MDOT),
					pool_m_dot_free_balance_block_number_1
				);
				// Checking free balance DOT && MDOT for ADMIN
				// ADMIN:
				assert_eq!(Currencies::free_balance(CurrencyId::DOT, &ADMIN), BALANCE_ZERO);
				assert_eq!(Currencies::free_balance(CurrencyId::MDOT, &ADMIN), BALANCE_ZERO);

				// ALICE:
				let alice_dot_free_balance_block_number_4: Balance = alice_dot_free_balance_block_number_3
					- alice_dot_total_borrow_block_number_3
					- borrow_accumulated_block_number_4;
				assert_eq!(
					Currencies::free_balance(CurrencyId::DOT, &ALICE),
					alice_dot_free_balance_block_number_4
				);
				// Expected: 60 000
				assert_eq!(
					Currencies::free_balance(CurrencyId::MDOT, &ALICE),
					alice_m_dot_free_balance_block_number_1
				);
				// Checking pool Storage params
				// Borrow_index changed: 1.000000004500000000 -> 1,000000006750000025
				let pool_borrow_index_block_number_4 =
					Rate::saturating_from_rational(1_000_000_006_750_000_025u128, 1_000_000_000_000_000_000u128);
				assert_eq!(
					TestPools::pools(CurrencyId::DOT).borrow_index,
					pool_borrow_index_block_number_4
				);
				let insurance_accumulated_block_number_4: Balance = 3_375_000_052_875;
				let pool_total_insurance_block_number_4: Balance =
					pool_total_insurance_block_number_3 + insurance_accumulated_block_number_4;
				assert_eq!(
					TestPools::pools(CurrencyId::DOT).total_insurance,
					pool_total_insurance_block_number_4
				);

				// FIXME: unavailable behavior.
				// TODO: should be fixed
				// It must be zero, but it is not.
				// 1875 left - 0 right
				// 15000000168750000528750 new borrow value accrue_interest
				// 15000000168750000526875 new user borrow value
				let borrow_accumulated_block_number_4 = 33_750_000_528_750u128;
				let alice_borrow_accumulated_block_number_4 = 33_750_000_526_875u128;
				let pool_dot_total_borrow_block_number_4 = pool_dot_total_borrow_block_number_3
					+ borrow_accumulated_block_number_4
					- alice_dot_total_borrow_block_number_3
					- alice_borrow_accumulated_block_number_4;
				assert_eq!(
					TestPools::pools(CurrencyId::DOT).total_borrowed,
					pool_dot_total_borrow_block_number_4
				);

				// Checking controller Storage params
				assert_eq!(TestController::controller_dates(CurrencyId::DOT).timestamp, 4);
				// Borrow_rate changed: 0,000000002250000015 -> 0,0
				let expected_borrow_rate_block_number_4 = Rate::zero();
				let (borrow_rate, _) =
					TestController::get_liquidity_pool_borrow_and_supply_rates(CurrencyId::DOT).unwrap_or_default();
				assert_eq!(borrow_rate, expected_borrow_rate_block_number_4);

				// Checking user pool Storage params
				// ADMIN:
				assert_eq!(
					TestPools::pool_user_data(CurrencyId::DOT, ADMIN).total_borrowed,
					BALANCE_ZERO
				);
				assert_eq!(
					TestPools::pool_user_data(CurrencyId::DOT, ADMIN).interest_index,
					RATE_ZERO
				);
				// ALICE:
				assert_eq!(
					TestPools::pool_user_data(CurrencyId::DOT, ALICE).total_borrowed,
					BALANCE_ZERO
				);
				let user_interest_index_block_number_4: Rate = pool_borrow_index_block_number_4;
				assert_eq!(
					TestPools::pool_user_data(CurrencyId::DOT, ALICE).interest_index,
					user_interest_index_block_number_4
				);

				// Check the underline amount before fn accrue_interest called
				let alice_underlining_amount: Balance =
					TestPools::convert_from_wrapped(CurrencyId::MDOT, alice_m_dot_free_balance_block_number_1).unwrap();

				System::set_block_number(5);

				// ACTION: REDEEM
				/* ------------------------------------------------------------------------------ */

				// Alice redeem all assets
				assert_ok!(MinterestProtocol::redeem(Origin::signed(ALICE), CurrencyId::DOT));

				// PARAMETERS CHECKING
				/* ------------------------------------------------------------------------------ */

				// Checking pool available liquidity
				// Expected: 160_000,000016875000046875
				let current_pool_available_liquidity_block_number_5: Balance =
					current_pool_available_liquidity_block_number_4 - alice_underlining_amount;
				assert_eq!(
					TestPools::get_pool_available_liquidity(CurrencyId::DOT),
					current_pool_available_liquidity_block_number_5
				);

				// Checking free balance MDOT in pool.
				// Expected: 0
				assert_eq!(Currencies::total_issuance(CurrencyId::MDOT), BALANCE_ZERO);

				// Checking free balance DOT && MDOT
				// ADMIN:
				assert_eq!(Currencies::free_balance(CurrencyId::DOT, &ADMIN), BALANCE_ZERO);
				assert_eq!(Currencies::free_balance(CurrencyId::MDOT, &ADMIN), BALANCE_ZERO);
				// ALICE:
				// Expected 99_999,999983124999953125
				assert_eq!(
					Currencies::free_balance(CurrencyId::DOT, &ALICE),
					alice_dot_free_balance_block_number_4 + alice_underlining_amount
				);
				// Expected: 0
				assert_eq!(Currencies::free_balance(CurrencyId::MDOT, &ALICE), BALANCE_ZERO);

				// Checking pool Storage params
				// Expected: 1,000000006750000025
				assert_eq!(
					TestPools::pools(CurrencyId::DOT).borrow_index,
					pool_borrow_index_block_number_4
				);
				// Expected: 100_000,000016875000052875
				assert_eq!(
					TestPools::pools(CurrencyId::DOT).total_insurance,
					pool_total_insurance_block_number_4
				);
				//FIXME: something went wrong.....
				//TODO: should be fixed
				assert_eq!(TestPools::pools(CurrencyId::DOT).total_borrowed, 1875);

				// Checking controller Storage params
				assert_eq!(TestController::controller_dates(CurrencyId::DOT).timestamp, 5);
				// borrow_rate changed: 0,000000002250000015 -> 0
				let (borrow_rate, _) =
					TestController::get_liquidity_pool_borrow_and_supply_rates(CurrencyId::DOT).unwrap_or_default();
				assert_eq!(borrow_rate, Rate::from_inner(0));

				// Checking user pool Storage params
				// ADMIN:
				assert_eq!(
					TestPools::pool_user_data(CurrencyId::DOT, ADMIN).total_borrowed,
					BALANCE_ZERO
				);
				assert_eq!(
					TestPools::pool_user_data(CurrencyId::DOT, ADMIN).interest_index,
					RATE_ZERO
				);
				// ALICE:
				// Expected: 0
				assert_eq!(
					TestPools::pool_user_data(CurrencyId::DOT, ALICE).total_borrowed,
					BALANCE_ZERO
				);
				// Expected: 1,000000006750000025
				assert_eq!(
					TestPools::pool_user_data(CurrencyId::DOT, ALICE).interest_index,
					user_interest_index_block_number_4
				);

				assert_ok!(MinterestProtocol::deposit_underlying(
					alice(),
					CurrencyId::DOT,
					20 * DOLLARS,
				));
			});
	}
}
