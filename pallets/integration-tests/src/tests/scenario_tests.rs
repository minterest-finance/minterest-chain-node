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
			.pool_initial(DOT)
			.pool_initial(ETH)
			.user_balance(ADMIN, DOT, ONE_HUNDRED_THOUSAND)
			.user_balance(ALICE, DOT, ONE_HUNDRED_THOUSAND)
			.pool_user_data(DOT, ALICE, Balance::zero(), Rate::zero(), true)
			.build()
			.execute_with(|| {
				// INITIAL PARAMS
				/* ------------------------------------------------------------------------------ */
				System::set_block_number(0);

				let alice_dot_free_balance_start: Balance = ONE_HUNDRED_THOUSAND;
				let alice_m_dot_free_balance_start: Balance = Balance::zero();
				let alice_dot_total_borrow_start: Balance = Balance::zero();

				let pool_available_liquidity_start: Balance = Balance::zero();
				let pool_m_dot_total_issuance_start: Balance = Balance::zero();
				let pool_protocol_interest_start: Balance = Balance::zero();
				let pool_dot_total_borrow_start: Balance = Balance::zero();

				// ACTION: DEPOSIT UNDERLYING
				/* ------------------------------------------------------------------------------ */

				// Add liquidity to DOT pool by Admin
				let admin_deposit_amount_block_number_0: Balance = 100_000 * DOLLARS;
				assert_ok!(MinterestProtocol::deposit_underlying(
					Origin::signed(ADMIN),
					DOT,
					admin_deposit_amount_block_number_0
				));

				// PARAMETERS CHECKING
				/* ------------------------------------------------------------------------------ */

				// Checking pool available liquidity
				// Expected: 100_000
				let current_pool_available_liquidity_block_number_0: Balance =
					pool_available_liquidity_start + admin_deposit_amount_block_number_0;
				assert_eq!(
					TestPools::get_pool_available_liquidity(DOT),
					current_pool_available_liquidity_block_number_0
				);

				// Checking free balance MDOT in pool.
				assert_eq!(
					Currencies::total_issuance(MDOT),
					pool_m_dot_total_issuance_start + admin_deposit_amount_block_number_0
				);

				// Checking free balance DOT && MDOT
				// Admin gets 100_000 wrapped token after adding liquidity by exchange rate 1:1
				// ADMIN:
				assert_eq!(Currencies::free_balance(DOT, &ADMIN), Balance::zero());
				assert_eq!(
					Currencies::free_balance(MDOT, &ADMIN),
					admin_deposit_amount_block_number_0
				);

				// Checking DOT pool Storage params
				assert_eq!(TestPools::pools(DOT).borrow_index, Rate::one());
				// Total interest didn't changed.
				assert_eq!(TestPools::pools(DOT).protocol_interest, pool_protocol_interest_start);
				assert_eq!(TestPools::pools(DOT).borrowed, pool_dot_total_borrow_start);

				// Checking controller params
				let (_, borrow_rate, _) = TestController::get_pool_exchange_borrow_and_supply_rates(DOT).unwrap();
				assert_eq!(
					TestController::controller_data_storage(DOT).last_interest_accrued_block,
					0
				);
				assert_eq!(borrow_rate, Rate::zero());

				// Checking DOT pool User params
				// ADMIN:
				assert_eq!(TestPools::pool_user_data(DOT, ADMIN).borrowed, Balance::zero());
				assert_eq!(TestPools::pool_user_data(DOT, ADMIN).interest_index, Rate::zero());

				System::set_block_number(1);

				// ACTION: DEPOSIT UNDERLYING
				/* ------------------------------------------------------------------------------ */

				// ALICE deposit 60 000 to DOT pool
				let alice_deposit_amount_block_number_1: Balance = 60_000 * DOLLARS;
				assert_ok!(MinterestProtocol::deposit_underlying(
					Origin::signed(ALICE),
					DOT,
					alice_deposit_amount_block_number_1
				));

				// PARAMETERS CHECKING
				/* ------------------------------------------------------------------------------ */

				// Checking pool available liquidity
				// Expected: 160 000
				let pool_available_liquidity_block_number_1: Balance =
					admin_deposit_amount_block_number_0 + alice_deposit_amount_block_number_1;
				assert_eq!(
					TestPools::get_pool_available_liquidity(DOT),
					pool_available_liquidity_block_number_1
				);

				// Checking free balance MDOT in pool.
				// Alice gets 60 000 wrapped token after adding liquidity by exchange rate 1:1
				// Sum expected: 160 000
				let pool_m_dot_free_balance_block_number_1: Balance = pool_m_dot_total_issuance_start
					+ admin_deposit_amount_block_number_0
					+ alice_deposit_amount_block_number_1;
				assert_eq!(Currencies::total_issuance(MDOT), pool_m_dot_free_balance_block_number_1);

				// Checking free balance DOT && MDOT
				// ADMIN:
				assert_eq!(Currencies::free_balance(DOT, &ADMIN), Balance::zero());
				assert_eq!(
					Currencies::free_balance(MDOT, &ADMIN),
					admin_deposit_amount_block_number_0
				);

				// ALICE:
				let alice_dot_free_balance_block_number_1: Balance =
					alice_dot_free_balance_start - alice_deposit_amount_block_number_1;
				assert_eq!(
					Currencies::free_balance(DOT, &ALICE),
					alice_dot_free_balance_block_number_1
				);
				let alice_m_dot_free_balance_block_number_1: Balance =
					alice_m_dot_free_balance_start + alice_deposit_amount_block_number_1;
				assert_eq!(
					Currencies::free_balance(MDOT, &ALICE),
					alice_m_dot_free_balance_block_number_1
				);

				// Checking DOT pool Storage params
				assert_eq!(TestPools::pools(DOT).borrow_index, Rate::one());
				// Expected start value: 0.0
				assert_eq!(TestPools::pools(DOT).protocol_interest, pool_protocol_interest_start);
				assert_eq!(TestPools::pools(DOT).borrowed, Balance::zero());

				// Checking controller Storage params
				assert_eq!(
					TestController::controller_data_storage(DOT).last_interest_accrued_block,
					1
				);
				let (_, borrow_rate, _) = TestController::get_pool_exchange_borrow_and_supply_rates(DOT).unwrap();
				assert_eq!(borrow_rate, Rate::zero());

				// Checking DOT pool User params
				// ADMIN:
				assert_eq!(TestPools::pool_user_data(DOT, ADMIN).borrowed, Balance::zero());
				assert_eq!(TestPools::pool_user_data(DOT, ADMIN).interest_index, Rate::zero());
				// ALICE:
				assert_eq!(TestPools::pool_user_data(DOT, ALICE).borrowed, Balance::zero());
				assert_eq!(TestPools::pool_user_data(DOT, ALICE).interest_index, Rate::zero());

				System::set_block_number(2);

				// ACTION: BORROW
				/* ------------------------------------------------------------------------------ */

				//  Alice borrow 30_000 from DOT pool.
				let alice_borrow_amount_block_number_2: Balance = 30_000 * DOLLARS;
				assert_ok!(MinterestProtocol::borrow(
					Origin::signed(ALICE),
					DOT,
					alice_borrow_amount_block_number_2
				));

				// PARAMETERS CHECKING
				/* ------------------------------------------------------------------------------ */

				// Checking pool available liquidity
				// Expected 130 000
				let current_pool_available_liquidity_block_number_2: Balance =
					pool_available_liquidity_block_number_1 - alice_borrow_amount_block_number_2;
				assert_eq!(
					TestPools::get_pool_available_liquidity(DOT),
					current_pool_available_liquidity_block_number_2
				);

				// Checking free balance MDOT in pool.
				// Expected: 160 000
				assert_eq!(Currencies::total_issuance(MDOT), pool_m_dot_free_balance_block_number_1);

				// Checking free balance DOT && MDOT
				// ADMIN:
				assert_eq!(Currencies::free_balance(DOT, &ADMIN), Balance::zero());
				assert_eq!(
					Currencies::free_balance(MDOT, &ADMIN),
					admin_deposit_amount_block_number_0
				);

				// ALICE:
				// Expected: 70 000
				let alice_dot_free_balance_block_number_2: Balance =
					alice_dot_free_balance_block_number_1 + alice_borrow_amount_block_number_2;
				assert_eq!(
					Currencies::free_balance(DOT, &ALICE),
					alice_dot_free_balance_block_number_2
				);
				// Expected: 60 000
				assert_eq!(
					Currencies::free_balance(MDOT, &ALICE),
					alice_m_dot_free_balance_block_number_1
				);

				// Checking pool Storage params
				assert_eq!(TestPools::pools(DOT).borrow_index, Rate::one());
				// Expected: 0
				assert_eq!(TestPools::pools(DOT).protocol_interest, pool_protocol_interest_start);
				// Total borrowed amount changed 0 -> 30 000
				let pool_dot_total_borrow_block_number_2: Balance =
					pool_dot_total_borrow_start + alice_borrow_amount_block_number_2;
				assert_eq!(TestPools::pools(DOT).borrowed, pool_dot_total_borrow_block_number_2);

				// Checking controller Storage params
				assert_eq!(
					TestController::controller_data_storage(DOT).last_interest_accrued_block,
					2
				);
				// Borrow_rate changed: 0 -> 16_875 * 10^(-13)
				let expected_borrow_rate_block_number_2: Rate =
					Rate::saturating_from_rational(16_875u128, 10_000_000_000_000u128);
				let (_, borrow_rate, _) = TestController::get_pool_exchange_borrow_and_supply_rates(DOT).unwrap();
				assert_eq!(borrow_rate, expected_borrow_rate_block_number_2);

				// Checking DOT pool User params
				// ADMIN:
				assert_eq!(TestPools::pool_user_data(DOT, ADMIN).borrowed, Balance::zero());
				assert_eq!(TestPools::pool_user_data(DOT, ADMIN).interest_index, Rate::zero());
				// ALICE:
				// User total borrowed changed: 0 -> 30 000
				let alice_dot_total_borrow_block_number_2: Balance =
					alice_dot_total_borrow_start + alice_borrow_amount_block_number_2;
				assert_eq!(
					TestPools::pool_user_data(DOT, ALICE).borrowed,
					alice_dot_total_borrow_block_number_2
				);
				// User interest index changed: 0 -> 1
				assert_eq!(TestPools::pool_user_data(DOT, ALICE).interest_index, Rate::one());

				System::set_block_number(3);

				// ACTION: REPAY
				/* ------------------------------------------------------------------------------ */

				// Alice repay part of her loan(15 000).
				let alice_repay_amount_block_number_3: Balance = 15_000 * DOLLARS;
				assert_ok!(MinterestProtocol::repay(
					Origin::signed(ALICE),
					DOT,
					alice_repay_amount_block_number_3
				));

				// PARAMETERS CHECKING
				/* ------------------------------------------------------------------------------ */

				// Checking pool available liquidity
				// Expected 145 000
				let current_pool_available_liquidity_block_number_3: Balance =
					current_pool_available_liquidity_block_number_2 + alice_repay_amount_block_number_3;
				assert_eq!(
					TestPools::get_pool_available_liquidity(DOT),
					current_pool_available_liquidity_block_number_3
				);

				// Checking free balance MDOT in pool.
				// Expected: 160 000
				assert_eq!(Currencies::total_issuance(MDOT), pool_m_dot_free_balance_block_number_1);

				// Checking free balance DOT && MDOT
				// ADMIN:
				assert_eq!(Currencies::free_balance(DOT, &ADMIN), Balance::zero());
				assert_eq!(
					Currencies::free_balance(MDOT, &ADMIN),
					admin_deposit_amount_block_number_0
				);

				// ALICE:
				// Expected: 55 000
				let alice_dot_free_balance_block_number_3: Balance =
					alice_dot_free_balance_block_number_2 - alice_repay_amount_block_number_3;
				assert_eq!(
					Currencies::free_balance(DOT, &ALICE),
					alice_dot_free_balance_block_number_3
				);
				assert_eq!(
					Currencies::free_balance(MDOT, &ALICE),
					alice_m_dot_free_balance_block_number_1
				);

				// Checking pool Storage params
				// Expected: 1.000000001687500000
				let pool_borrow_index_block_number_3: Rate =
					Rate::saturating_from_rational(10_000_000_016_875u128, 10_000_000_000_000u128);
				assert_eq!(TestPools::pools(DOT).borrow_index, pool_borrow_index_block_number_3);
				// Expected: 0,0000050625
				let interest_accumulated_block_number_3: Balance = 5_062_500_000_000;
				let pool_protocol_interest_block_number_3: Balance =
					pool_protocol_interest_start + interest_accumulated_block_number_3;
				assert_eq!(
					TestPools::pools(DOT).protocol_interest,
					pool_protocol_interest_block_number_3
				);
				// Expected: 15_000,000050625
				let borrow_accumulated_block_number_3: Balance = 50_625_000_000_000;
				let pool_dot_total_borrow_block_number_3: Balance = pool_dot_total_borrow_block_number_2
					+ borrow_accumulated_block_number_3
					- alice_repay_amount_block_number_3;
				assert_eq!(TestPools::pools(DOT).borrowed, pool_dot_total_borrow_block_number_3);

				// Checking controller Storage params
				assert_eq!(
					TestController::controller_data_storage(DOT).last_interest_accrued_block,
					3
				);
				// Borrow_rate changed: 0,0000000016875 -> 0.000000000843750002
				let expected_borrow_rate_block_number_3: Rate =
					Rate::saturating_from_rational(843_750_002u128, 1_000_000_000_000_000_000u128);
				let (_, borrow_rate, _) = TestController::get_pool_exchange_borrow_and_supply_rates(DOT).unwrap();
				assert_eq!(borrow_rate, expected_borrow_rate_block_number_3);

				// Checking DOT pool User params
				// ADMIN:
				assert_eq!(TestPools::pool_user_data(DOT, ADMIN).borrowed, Balance::zero());
				assert_eq!(TestPools::pool_user_data(DOT, ADMIN).interest_index, Rate::zero());
				// ALICE:
				let alice_dot_total_borrow_block_number_3: Balance = alice_dot_total_borrow_block_number_2
					+ borrow_accumulated_block_number_3
					- alice_repay_amount_block_number_3;
				assert_eq!(
					TestPools::pool_user_data(DOT, ALICE).borrowed,
					alice_dot_total_borrow_block_number_3
				);
				// Interest_index changed: 0 -> 1.000000001687500000
				let user_interest_index_block_number_3: Rate = pool_borrow_index_block_number_3;
				assert_eq!(
					TestPools::pool_user_data(DOT, ALICE).interest_index,
					user_interest_index_block_number_3
				);

				System::set_block_number(4);

				// ACTION: REPAY_ALL
				/* ------------------------------------------------------------------------------ */

				// Alice repay all loans.
				assert_ok!(MinterestProtocol::repay_all(Origin::signed(ALICE), DOT));

				// PARAMETERS CHECKING
				/* ------------------------------------------------------------------------------ */

				// Checking pool available liquidity
				// Real expected: 		 160_000,000063281250072714
				// Currently expected:	 160_000,000063281250066358
				// FIXME: unavailable behavior. That is a reason of error below.
				// FIXME: borrow_accumulated_block_number_4 should be  12_656_250_072_714
				//										   instead of  12_656_250_066_358
				let borrow_accumulated_block_number_4: Balance = 12_656_250_066_358;
				let current_pool_available_liquidity_block_number_4: Balance =
					current_pool_available_liquidity_block_number_3
						+ alice_repay_amount_block_number_3
						+ borrow_accumulated_block_number_3
						+ borrow_accumulated_block_number_4;
				assert_eq!(
					TestPools::get_pool_available_liquidity(DOT),
					current_pool_available_liquidity_block_number_4
				);

				// Checking free balance MDOT in pool.
				// Expected: 160 000
				assert_eq!(Currencies::total_issuance(MDOT), pool_m_dot_free_balance_block_number_1);
				// Checking free balance DOT && MDOT for ADMIN
				// ADMIN:
				assert_eq!(Currencies::free_balance(DOT, &ADMIN), Balance::zero());
				assert_eq!(
					Currencies::free_balance(MDOT, &ADMIN),
					admin_deposit_amount_block_number_0
				);

				// ALICE:
				let alice_dot_free_balance_block_number_4: Balance = alice_dot_free_balance_block_number_3
					- alice_dot_total_borrow_block_number_3
					- borrow_accumulated_block_number_4;
				assert_eq!(
					Currencies::free_balance(DOT, &ALICE),
					alice_dot_free_balance_block_number_4
				);
				// Expected: 60 000
				assert_eq!(
					Currencies::free_balance(MDOT, &ALICE),
					alice_m_dot_free_balance_block_number_1
				);
				// Checking pool Storage params
				// Borrow_index changed: 1.000000001687500000 -> 1,000000002531250003
				let pool_borrow_index_block_number_4 =
					Rate::saturating_from_rational(1_000_000_002_531_250_003u128, 1_000_000_000_000_000_000u128);
				assert_eq!(TestPools::pools(DOT).borrow_index, pool_borrow_index_block_number_4);
				let interest_accumulated_block_number_4: Balance = 1_265_625_007_271;
				let pool_protocol_interest_block_number_4: Balance =
					pool_protocol_interest_block_number_3 + interest_accumulated_block_number_4;
				assert_eq!(
					TestPools::pools(DOT).protocol_interest,
					pool_protocol_interest_block_number_4
				);

				// FIXME: unavailable behavior.
				// TODO: should be fixed
				// It must be zero, but it is not.
				// 6356 left - 0 right
				// 15000000063281250072714 new borrow value accrue_interest
				// 15000000063281250066358 new user borrow value
				let borrow_accumulated_block_number_4 = 12_656_250_072_714u128;
				let alice_borrow_accumulated_block_number_4 = 12_656_250_066_358u128;
				let pool_dot_total_borrow_block_number_4 = pool_dot_total_borrow_block_number_3
					+ borrow_accumulated_block_number_4
					- alice_dot_total_borrow_block_number_3
					- alice_borrow_accumulated_block_number_4;
				assert_eq!(TestPools::pools(DOT).borrowed, pool_dot_total_borrow_block_number_4);

				// Checking controller Storage params
				assert_eq!(
					TestController::controller_data_storage(DOT).last_interest_accrued_block,
					4
				);
				// Borrow_rate changed: 0,000000002250000015 -> 0,0
				let expected_borrow_rate_block_number_4 = Rate::zero();
				let (_, borrow_rate, _) = TestController::get_pool_exchange_borrow_and_supply_rates(DOT).unwrap();
				assert_eq!(borrow_rate, expected_borrow_rate_block_number_4);

				// Checking user pool Storage params
				// ADMIN:
				assert_eq!(TestPools::pool_user_data(DOT, ADMIN).borrowed, Balance::zero());
				assert_eq!(TestPools::pool_user_data(DOT, ADMIN).interest_index, Rate::zero());
				// ALICE:
				assert_eq!(TestPools::pool_user_data(DOT, ALICE).borrowed, Balance::zero());
				let user_interest_index_block_number_4: Rate = pool_borrow_index_block_number_4;
				assert_eq!(
					TestPools::pool_user_data(DOT, ALICE).interest_index,
					user_interest_index_block_number_4
				);

				// Check the underline amount before fn accrue_interest called
				let exchange_rate_dot = TestPools::get_exchange_rate(DOT).unwrap();
				let alice_underlining_amount: Balance =
					TestPools::wrapped_to_underlying(alice_m_dot_free_balance_block_number_1, exchange_rate_dot)
						.unwrap();

				System::set_block_number(5);

				// ACTION: REDEEM
				/* ------------------------------------------------------------------------------ */

				// Alice redeem all assets
				assert_ok!(MinterestProtocol::redeem(Origin::signed(ALICE), DOT));

				// PARAMETERS CHECKING
				/* ------------------------------------------------------------------------------ */

				// Checking pool available liquidity
				// Expected: 100_000,000_041_923_828_146_358
				let current_pool_available_liquidity_block_number_5: Balance =
					current_pool_available_liquidity_block_number_4 - alice_underlining_amount;
				assert_eq!(
					TestPools::get_pool_available_liquidity(DOT),
					current_pool_available_liquidity_block_number_5
				);

				// Checking free balance MDOT in pool.
				// Expected: 100_00
				assert_eq!(Currencies::total_issuance(MDOT), ONE_HUNDRED_THOUSAND);

				// Checking free balance DOT && MDOT
				// ADMIN:
				assert_eq!(Currencies::free_balance(DOT, &ADMIN), Balance::zero());
				assert_eq!(
					Currencies::free_balance(MDOT, &ADMIN),
					admin_deposit_amount_block_number_0
				);
				// ALICE:
				// Expected 99_999,999_958_076_171_853_642
				assert_eq!(
					Currencies::free_balance(DOT, &ALICE),
					alice_dot_free_balance_block_number_4 + alice_underlining_amount
				);
				// Expected: 0
				assert_eq!(Currencies::free_balance(MDOT, &ALICE), Balance::zero());

				// Checking pool Storage params
				// Expected: 1,000000002531250003
				assert_eq!(TestPools::pools(DOT).borrow_index, pool_borrow_index_block_number_4);
				// Expected: 0,000006328125007271
				assert_eq!(
					TestPools::pools(DOT).protocol_interest,
					pool_protocol_interest_block_number_4
				);
				//FIXME: something went wrong.....
				//TODO: should be fixed
				assert_eq!(TestPools::pools(DOT).borrowed, 6356);

				// Checking controller Storage params
				assert_eq!(
					TestController::controller_data_storage(DOT).last_interest_accrued_block,
					5
				);
				// borrow_rate changed: 0,000000002250000015 -> 0
				let (_, borrow_rate, _) = TestController::get_pool_exchange_borrow_and_supply_rates(DOT).unwrap();
				assert_eq!(borrow_rate, Rate::from_inner(0));

				// Checking user pool Storage params
				// ADMIN:
				assert_eq!(TestPools::pool_user_data(DOT, ADMIN).borrowed, Balance::zero());
				assert_eq!(TestPools::pool_user_data(DOT, ADMIN).interest_index, Rate::zero());
				// ALICE:
				// Expected: 0
				assert_eq!(TestPools::pool_user_data(DOT, ALICE).borrowed, Balance::zero());
				// Expected: 1,000000002531250003
				assert_eq!(
					TestPools::pool_user_data(DOT, ALICE).interest_index,
					user_interest_index_block_number_4
				);

				assert_ok!(MinterestProtocol::deposit_underlying(alice_origin(), DOT, 20 * DOLLARS,));
			});
	}
}
