///  Integration-tests for luquidity-pools pallet.

#[cfg(test)]

mod tests {
	use crate::tests::*;

	// Function `get_exchange_rate + calculate_exchange_rate` scenario #1:
	#[test]
	fn get_exchange_rate_deposit_without_insurance() {
		ExtBuilder::default()
			.user_balance(ALICE, CurrencyId::DOT, ONE_HUNDRED)
			.pool_user_data(CurrencyId::DOT, ALICE, BALANCE_ZERO, RATE_ZERO, false, 0)
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

				// Expected exchange rate && wrapped amount based on params after fn accrue_interest_rate called
				let expected_amount_wrapped_tokens = 40_000 * DOLLARS;
				let expected_exchange_rate_mock = Rate::one();

				// Checking if real exchange rate && wrapped amount is equal to the expected
				assert_eq!(
					Currencies::free_balance(CurrencyId::MDOT, &ALICE),
					expected_amount_wrapped_tokens
				);
				assert_eq!(
					TestPools::get_exchange_rate(CurrencyId::DOT),
					Ok(expected_exchange_rate_mock)
				);
			});
	}

	// Function `get_exchange_rate + calculate_exchange_rate` scenario #2:
	#[test]
	fn get_exchange_rate_deposit_with_pool_insurance() {
		ExtBuilder::default()
			.user_balance(ALICE, CurrencyId::DOT, ONE_HUNDRED)
			.pool_user_data(CurrencyId::DOT, ALICE, BALANCE_ZERO, RATE_ZERO, false, 0)
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

				// Expected exchange rate && wrapped amount based on params after fn accrue_interest_rate called
				let expected_amount_wrapped_tokens = 40_000 * DOLLARS;
				let expected_exchange_rate_mock = Rate::one();

				// Checking if real exchange rate && wrapped amount is equal to the expected
				assert_eq!(
					Currencies::free_balance(CurrencyId::MDOT, &ALICE),
					expected_amount_wrapped_tokens
				);
				assert_eq!(
					TestPools::get_exchange_rate(CurrencyId::DOT),
					Ok(expected_exchange_rate_mock)
				);
			});
	}

	// Function `get_exchange_rate + calculate_exchange_rate` scenario #3:
	#[test]
	fn get_exchange_rate_deposit_and_borrow_without_insurance() {
		ExtBuilder::default()
			.user_balance(ALICE, CurrencyId::DOT, ONE_HUNDRED)
			.pool_user_data(CurrencyId::DOT, ALICE, BALANCE_ZERO, RATE_ZERO, true, 0)
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

				// Alice borrow from DOT pool
				let alice_borrowed_amount_in_dot = 20_000 * DOLLARS;
				assert_ok!(MinterestProtocol::borrow(
					Origin::signed(ALICE),
					CurrencyId::DOT,
					alice_borrowed_amount_in_dot
				));

				// Expected exchange rate && wrapped amount based on params after fn accrue_interest_rate called
				let expected_amount_wrapped_tokens = 40_000 * DOLLARS;
				let expected_exchange_rate_mock = Rate::one();

				// Checking if real borrow interest rate && wrapped amount is equal to the expected
				assert_eq!(
					Currencies::free_balance(CurrencyId::MDOT, &ALICE),
					expected_amount_wrapped_tokens
				);
				assert_eq!(
					TestPools::get_exchange_rate(CurrencyId::DOT),
					Ok(expected_exchange_rate_mock)
				);
			});
	}

	// Function `get_exchange_rate + calculate_exchange_rate` scenario #4:
	#[test]
	fn get_exchange_rate_deposit_and_borrow_with_insurance() {
		ExtBuilder::default()
			.user_balance(ALICE, CurrencyId::DOT, ONE_HUNDRED)
			.pool_user_data(CurrencyId::DOT, ALICE, BALANCE_ZERO, RATE_ZERO, true, 0)
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

				// Expected exchange rate && wrapped amount based on params after fn accrue_interest_rate called
				let expected_amount_wrapped_tokens = 40_000 * DOLLARS;
				let expected_exchange_rate_mock = Rate::one();

				// Checking if real exchange rate && wrapped amount is equal to the expected
				assert_eq!(
					Currencies::free_balance(CurrencyId::MDOT, &ALICE),
					expected_amount_wrapped_tokens
				);
				assert_eq!(
					TestPools::get_exchange_rate(CurrencyId::DOT),
					Ok(expected_exchange_rate_mock)
				);
			});
	}

	// Function `get_exchange_rate + calculate_exchange_rate` scenario #5:
	#[test]
	fn get_exchange_rate_few_deposits_and_borrows_with_insurance() {
		ExtBuilder::default()
			.user_balance(ALICE, CurrencyId::DOT, ONE_HUNDRED)
			.user_balance(BOB, CurrencyId::DOT, ONE_HUNDRED)
			.pool_user_data(CurrencyId::DOT, ALICE, BALANCE_ZERO, RATE_ZERO, true, 0)
			.pool_user_data(CurrencyId::DOT, BOB, BALANCE_ZERO, RATE_ZERO, true, 0)
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

				// Expected exchange rate based on params before fn accrue_interest_rate in block 4 called
				let expected_exchange_rate_mock_block_number_3 = Rate::from_inner(1000000002025000000);

				assert_eq!(
					TestPools::get_exchange_rate(CurrencyId::DOT),
					Ok(expected_exchange_rate_mock_block_number_3)
				);

				// Alice try to borrow from DOT pool
				let bob_borrowed_amount_in_dot = 50_000 * DOLLARS;
				assert_ok!(MinterestProtocol::borrow(
					Origin::signed(BOB),
					CurrencyId::DOT,
					bob_borrowed_amount_in_dot
				));

				// Expected exchange rate && wrapped amount based on params after
				// fn accrue_interest_rate in block 4 called
				let expected_amount_wrapped_tokens_alice = 40_000 * DOLLARS;
				// bob_deposited_amount/expected_exchange_rate_mock_block_number_3 = 59_999_999_878_500_000_246_037
				let expected_amount_wrapped_tokens_bob = 59_999_999_878_500_000_246_037;
				let expected_exchange_rate_mock_block_number_4 = Rate::from_inner(1000000002349000003);

				// Checking if real exchange rate && wrapped amount is equal to the expected
				assert_eq!(
					Currencies::free_balance(CurrencyId::MDOT, &ALICE),
					expected_amount_wrapped_tokens_alice
				);
				assert_eq!(
					Currencies::free_balance(CurrencyId::MDOT, &BOB),
					expected_amount_wrapped_tokens_bob
				);
				assert_eq!(
					TestPools::get_exchange_rate(CurrencyId::DOT),
					Ok(expected_exchange_rate_mock_block_number_4)
				);
			});
	}
}
