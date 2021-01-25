//  Integration-tests for minterest model pallet.

#[cfg(test)]

mod tests {
	use crate::tests::*;

	// Function `calculate_borrow_interest_rate + calculate_utilization_rate` scenario #1:
	#[test]
	fn calculate_borrow_interest_rate_deposit_without_insurance() {
		ExtBuilder::default()
			.user_balance(ALICE, CurrencyId::DOT, ONE_HUNDRED)
			.pool_user_data(CurrencyId::DOT, ALICE, BALANCE_ZERO, RATE_ZERO, true)
			.pool_total_insurance(CurrencyId::DOT, BALANCE_ZERO)
			.build()
			.execute_with(|| {
				// Expected borrow interest rate based on params before fn accrue_interest_rate called
				let expected_borrow_rate_mock = Rate::zero();

				// Alice deposit to DOT pool
				let alice_deposited_amount = 40_000 * DOLLARS;
				assert_ok!(MinterestProtocol::deposit_underlying(
					Origin::signed(ALICE),
					CurrencyId::DOT,
					alice_deposited_amount
				));

				// Checking if real borrow interest rate is equal to the expected
				assert_eq!(
					TestController::controller_dates(CurrencyId::DOT).borrow_rate,
					expected_borrow_rate_mock
				);
			});
	}

	// Function `calculate_borrow_interest_rate + calculate_utilization_rate` scenario #2:
	#[test]
	fn calculate_borrow_interest_rate_deposit_with_pool_insurance() {
		ExtBuilder::default()
			.user_balance(ALICE, CurrencyId::DOT, ONE_HUNDRED)
			.pool_user_data(CurrencyId::DOT, ALICE, BALANCE_ZERO, RATE_ZERO, true)
			.pool_total_insurance(CurrencyId::DOT, ONE_HUNDRED)
			.build()
			.execute_with(|| {
				// Expected borrow interest rate based on params before fn accrue_interest_rate called
				let expected_borrow_rate_mock = Rate::zero();

				// Alice deposit to DOT pool
				let alice_deposited_amount = 40_000 * DOLLARS;
				assert_ok!(MinterestProtocol::deposit_underlying(
					Origin::signed(ALICE),
					CurrencyId::DOT,
					alice_deposited_amount
				));

				// Checking if real borrow interest rate is equal to the expected
				assert_eq!(
					TestController::controller_dates(CurrencyId::DOT).borrow_rate,
					expected_borrow_rate_mock
				);
			});
	}

	// Function `calculate_borrow_interest_rate + calculate_utilization_rate` scenario #3:
	#[test]
	fn calculate_borrow_interest_rate_deposit_and_borrow_without_insurance() {
		ExtBuilder::default()
			.user_balance(ALICE, CurrencyId::DOT, ONE_HUNDRED)
			.pool_user_data(CurrencyId::DOT, ALICE, BALANCE_ZERO, RATE_ZERO, true)
			.pool_total_insurance(CurrencyId::DOT, BALANCE_ZERO)
			.build()
			.execute_with(|| {
				// Alice deposit to DOT pool
				let alice_deposited_amount = 40_000 * DOLLARS;
				assert_ok!(MinterestProtocol::deposit_underlying(
					Origin::signed(ALICE),
					CurrencyId::DOT,
					alice_deposited_amount
				));

				System::set_block_number(2);

				// Expected borrow interest rate based on params before fn accrue_interest_rate called
				let expected_borrow_rate_mock = Rate::zero();

				// Alice borrow from DOT pool
				let alice_borrowed_amount_in_dot = 20_000 * DOLLARS;
				assert_ok!(MinterestProtocol::borrow(
					Origin::signed(ALICE),
					CurrencyId::DOT,
					alice_borrowed_amount_in_dot
				));

				assert_eq!(
					TestController::controller_dates(CurrencyId::DOT).borrow_rate,
					expected_borrow_rate_mock
				);
			});
	}

	// Function `calculate_borrow_interest_rate + calculate_utilization_rate` scenario #4:
	#[test]
	fn calculate_borrow_interest_rate_deposit_and_borrow_with_insurance() {
		ExtBuilder::default()
			.user_balance(ALICE, CurrencyId::DOT, ONE_HUNDRED)
			.pool_user_data(CurrencyId::DOT, ALICE, BALANCE_ZERO, RATE_ZERO, true)
			.pool_total_insurance(CurrencyId::DOT, ONE_HUNDRED)
			.build()
			.execute_with(|| {
				// Alice deposit to DOT pool
				let alice_deposited_amount = 40_000 * DOLLARS;
				assert_ok!(MinterestProtocol::deposit_underlying(
					Origin::signed(ALICE),
					CurrencyId::DOT,
					alice_deposited_amount
				));

				System::set_block_number(2);

				// Expected borrow interest rate based on params before fn accrue_interest_rate called
				let expected_borrow_rate_mock = Rate::zero();

				// Alice borrow from DOT pool
				let alice_borrowed_amount_in_dot = 20_000 * DOLLARS;
				assert_ok!(MinterestProtocol::borrow(
					Origin::signed(ALICE),
					CurrencyId::DOT,
					alice_borrowed_amount_in_dot
				));

				assert_eq!(
					TestController::controller_dates(CurrencyId::DOT).borrow_rate,
					expected_borrow_rate_mock
				);
			});
	}

	// Function `calculate_borrow_interest_rate + calculate_utilization_rate` scenario #5:
	#[test]
	fn calculate_borrow_interest_rate_few_deposits_and_borrows_with_insurance() {
		ExtBuilder::default()
			.user_balance(ALICE, CurrencyId::DOT, ONE_HUNDRED)
			.user_balance(BOB, CurrencyId::DOT, ONE_HUNDRED)
			.pool_user_data(CurrencyId::DOT, ALICE, BALANCE_ZERO, RATE_ZERO, true)
			.pool_user_data(CurrencyId::DOT, BOB, BALANCE_ZERO, RATE_ZERO, true)
			.pool_total_insurance(CurrencyId::DOT, ONE_HUNDRED)
			.build()
			.execute_with(|| {
				// Alice deposit to DOT pool
				let alice_deposited_amount = 40_000 * DOLLARS;
				assert_ok!(MinterestProtocol::deposit_underlying(
					Origin::signed(ALICE),
					CurrencyId::DOT,
					alice_deposited_amount
				));

				System::set_block_number(2);

				// Alice borrow from DOT pool
				let alice_borrowed_amount_in_dot = 20_000 * DOLLARS;
				assert_ok!(MinterestProtocol::borrow(
					Origin::signed(ALICE),
					CurrencyId::DOT,
					alice_borrowed_amount_in_dot
				));

				System::set_block_number(3);

				// Bob deposit to DOT pool
				let bob_deposited_amount = 60_000 * DOLLARS;
				assert_ok!(MinterestProtocol::deposit_underlying(
					Origin::signed(BOB),
					CurrencyId::DOT,
					bob_deposited_amount
				));

				System::set_block_number(4);

				// Expected borrow interest rate based on params before fn accrue_interest_rate called
				let expected_borrow_rate_mock = Rate::from_inner(1800000006);

				// Alice try to borrow from DOT pool
				let bob_borrowed_amount_in_dot = 50_000 * DOLLARS;
				assert_ok!(MinterestProtocol::borrow(
					Origin::signed(BOB),
					CurrencyId::DOT,
					bob_borrowed_amount_in_dot
				));

				// Checking if real borrow interest rate is equal to the expected
				assert_eq!(
					TestController::controller_dates(CurrencyId::DOT).borrow_rate,
					expected_borrow_rate_mock
				);
			});
	}
}
