///  Integration-tests for minterest protocol pallet.

#[cfg(test)]

mod tests {
	use crate::tests::*;

	#[test]
	fn deposit_underlying_with_supplied_insurance_should_work() {
		ExtBuilder::default()
			.pool_initial(CurrencyId::DOT)
			.user_balance(ADMIN, CurrencyId::DOT, ONE_HUNDRED)
			.user_balance(ALICE, CurrencyId::DOT, ONE_HUNDRED)
			.user_balance(BOB, CurrencyId::DOT, ONE_HUNDRED)
			.pool_user_data(CurrencyId::DOT, ALICE, BALANCE_ZERO, RATE_ZERO, false, 0)
			.build()
			.execute_with(|| {
				// Set initial balance
				assert_ok!(MinterestProtocol::deposit_underlying(
					Origin::signed(ADMIN),
					CurrencyId::DOT,
					ONE_HUNDRED
				));

				// Alice deposit to DOT pool
				let alice_deposited_amount = 60_000 * DOLLARS;
				assert_ok!(MinterestProtocol::deposit_underlying(
					Origin::signed(ALICE),
					CurrencyId::DOT,
					alice_deposited_amount
				));

				// Calculate expected amount of wrapped tokens for Alice
				let alice_expected_amount_wrapped_tokens =
					TestPools::convert_to_wrapped(CurrencyId::DOT, alice_deposited_amount).unwrap();

				// Checking pool available liquidity increased by 60 000
				assert_eq!(
					TestPools::get_pool_available_liquidity(CurrencyId::DOT),
					ONE_HUNDRED + alice_deposited_amount
				);

				// Checking current free balance for DOT && MDOT
				assert_eq!(
					Currencies::free_balance(CurrencyId::DOT, &ALICE),
					ONE_HUNDRED - alice_deposited_amount
				);
				assert_eq!(
					Currencies::free_balance(CurrencyId::MDOT, &ALICE),
					alice_expected_amount_wrapped_tokens
				);

				System::set_block_number(2);

				// Alice deposit to DOT pool
				let bob_deposited_amount = ONE_HUNDRED;
				assert_ok!(MinterestProtocol::deposit_underlying(
					Origin::signed(BOB),
					CurrencyId::DOT,
					bob_deposited_amount
				));

				// Calculate expected amount of wrapped tokens for Bob
				let bob_expected_amount_wrapped_tokens =
					TestPools::convert_to_wrapped(CurrencyId::DOT, bob_deposited_amount).unwrap();

				// Checking pool available liquidity increased by 60 000
				assert_eq!(
					TestPools::get_pool_available_liquidity(CurrencyId::DOT),
					ONE_HUNDRED + alice_deposited_amount + bob_deposited_amount
				);

				// Checking current free balance for DOT && MDOT
				assert_eq!(Currencies::free_balance(CurrencyId::DOT, &ALICE), 40_000 * DOLLARS);
				assert_eq!(
					Currencies::free_balance(CurrencyId::DOT, &BOB),
					ONE_HUNDRED - bob_deposited_amount
				);
				assert_eq!(
					Currencies::free_balance(CurrencyId::MDOT, &ALICE),
					alice_expected_amount_wrapped_tokens
				);
				assert_eq!(
					Currencies::free_balance(CurrencyId::MDOT, &BOB),
					bob_expected_amount_wrapped_tokens
				);
			});
	}

	#[test]
	fn deposit_underlying_overflow_while_convert_underline_to_wrap_should_work() {
		ExtBuilder::default()
			// Set genesis to get exchange rate 0,00000000000000001
			.user_balance(ALICE, CurrencyId::DOT, ONE_HUNDRED)
			.user_balance(ALICE, CurrencyId::MDOT, DOLLARS)
			.pool_initial(CurrencyId::DOT)
			.pool_balance(CurrencyId::DOT, 5)
			.pool_total_borrowed(CurrencyId::DOT, 5)
			.build()
			.execute_with(|| {
				// Alice try to deposit ONE_HUNDRED to DOT pool
				assert_noop!(
					MinterestProtocol::deposit_underlying(Origin::signed(ALICE), CurrencyId::DOT, ONE_HUNDRED),
					MinterestProtocolError::<Test>::NumOverflow
				);

				// Alice deposit to DOT pool.
				assert_ok!(MinterestProtocol::deposit_underlying(
					Origin::signed(ALICE),
					CurrencyId::DOT,
					100
				));
			});
	}

	// Extrinsic `redeem_underlying`, description of scenario #1:
	// The user The user tries to redeem all assets in the first currency. He has loan in the first
	// currency.
	// Initial exchange rate for all assets equal 1.0;
	// Collateral factor for all assets equal 0.9;
	// 1. Alice deposit 60 DOT;
	// 2. Alice deposit 50 ETH;
	// 3. Alice borrow 50 DOT;
	// 4. Alice can't `redeem_underlying` 60 DOT: 50 ETH * 0.9 collateral < 50 DOT borrow;
	// 5. Alice deposit 10 ETH;
	// 6. Alice `redeem_underlying` 60 DOT;
	// 7. Alice can't `redeem_underlying` 60 ETH.
	#[test]
	fn redeem_underlying_with_current_currency_borrowing() {
		ExtBuilder::default()
			.pool_initial(CurrencyId::DOT)
			.user_balance(ADMIN, CurrencyId::DOT, ONE_HUNDRED)
			.user_balance(ALICE, CurrencyId::DOT, ONE_HUNDRED)
			.user_balance(ALICE, CurrencyId::ETH, ONE_HUNDRED)
			.pool_user_data(CurrencyId::DOT, ALICE, BALANCE_ZERO, RATE_ZERO, true, 0)
			.pool_user_data(CurrencyId::ETH, ALICE, BALANCE_ZERO, RATE_ZERO, true, 0)
			.build()
			.execute_with(|| {
				// Set initial balance
				assert_ok!(MinterestProtocol::deposit_underlying(
					Origin::signed(ADMIN),
					CurrencyId::DOT,
					ONE_HUNDRED
				));

				// Alice deposit 60 DOT to pool.
				let alice_deposited_amount_in_dot = 60_000 * DOLLARS;
				assert_ok!(MinterestProtocol::deposit_underlying(
					Origin::signed(ALICE),
					CurrencyId::DOT,
					alice_deposited_amount_in_dot
				));

				System::set_block_number(2);

				// Alice deposit 50 ETH to pool.
				let alice_deposited_amount_in_eth = 50_000 * DOLLARS;
				assert_ok!(MinterestProtocol::deposit_underlying(
					Origin::signed(ALICE),
					CurrencyId::ETH,
					alice_deposited_amount_in_eth
				));

				System::set_block_number(3);

				// Alice borrow from DOT pool
				let alice_borrowed_amount_in_dot = 50_000 * DOLLARS;
				assert_ok!(MinterestProtocol::borrow(
					Origin::signed(ALICE),
					CurrencyId::DOT,
					alice_borrowed_amount_in_dot
				));

				// Checking pool available liquidity
				assert_eq!(
					TestPools::get_pool_available_liquidity(CurrencyId::DOT),
					ONE_HUNDRED + alice_deposited_amount_in_dot - alice_borrowed_amount_in_dot
				);

				// Checking Alice's free balance DOT && MDOT.
				assert_eq!(
					Currencies::free_balance(CurrencyId::DOT, &ALICE),
					ONE_HUNDRED - alice_deposited_amount_in_dot + alice_borrowed_amount_in_dot
				);
				assert_eq!(
					Currencies::free_balance(CurrencyId::ETH, &ALICE),
					ONE_HUNDRED - alice_deposited_amount_in_eth
				);
				let expected_amount_wrapped_tokens_in_dot =
					TestPools::convert_to_wrapped(CurrencyId::DOT, alice_deposited_amount_in_dot).unwrap();
				assert_eq!(
					Currencies::free_balance(CurrencyId::MDOT, &ALICE),
					expected_amount_wrapped_tokens_in_dot
				);
				let expected_amount_wrapped_tokens_in_eth =
					TestPools::convert_to_wrapped(CurrencyId::ETH, alice_deposited_amount_in_eth).unwrap();
				assert_eq!(
					Currencies::free_balance(CurrencyId::METH, &ALICE),
					expected_amount_wrapped_tokens_in_eth
				);

				// Checking total borrow for Alice DOT pool
				assert_eq!(
					TestPools::pool_user_data(CurrencyId::DOT, ALICE).total_borrowed,
					alice_borrowed_amount_in_dot
				);
				// Checking total borrow for DOT pool
				assert_eq!(
					TestPools::pools(CurrencyId::DOT).total_borrowed,
					alice_borrowed_amount_in_dot
				);

				System::set_block_number(4);

				// Alice try to redeem all from DOT pool
				assert_noop!(
					MinterestProtocol::redeem_underlying(
						Origin::signed(ALICE),
						CurrencyId::DOT,
						alice_deposited_amount_in_dot
					),
					MinterestProtocolError::<Test>::RedeemControllerRejection
				);

				System::set_block_number(5);

				// Alice add liquidity to ETH pool
				let alice_deposited_amount_in_eth_secondary = 10_000 * DOLLARS;
				assert_ok!(MinterestProtocol::deposit_underlying(
					Origin::signed(ALICE),
					CurrencyId::ETH,
					alice_deposited_amount_in_eth_secondary
				));

				System::set_block_number(6);

				// Alice redeem all DOTs
				let expected_amount_redeemed_underlying_assets = 60_000_000_142_382_812_500_000;
				assert_ok!(MinterestProtocol::redeem_underlying(
					Origin::signed(ALICE),
					CurrencyId::DOT,
					expected_amount_redeemed_underlying_assets
				));

				// Checking free balance DOT/MDOT && ETH/METH for user.
				assert_eq!(
					Currencies::free_balance(CurrencyId::DOT, &ALICE),
					ONE_HUNDRED - alice_deposited_amount_in_dot
						+ alice_borrowed_amount_in_dot
						+ expected_amount_redeemed_underlying_assets
				);
				assert_eq!(
					Currencies::free_balance(CurrencyId::ETH, &ALICE),
					ONE_HUNDRED - alice_deposited_amount_in_eth - alice_deposited_amount_in_eth_secondary
				);

				assert_eq!(Currencies::free_balance(CurrencyId::MDOT, &ALICE), 0);
				let expected_amount_wrapped_tokens_in_eth_summary = expected_amount_wrapped_tokens_in_eth
					+ TestPools::convert_to_wrapped(CurrencyId::ETH, alice_deposited_amount_in_eth_secondary).unwrap();
				assert_eq!(
					Currencies::free_balance(CurrencyId::METH, &ALICE),
					expected_amount_wrapped_tokens_in_eth_summary
				);
				// Checking total borrow for Alice DOT pool
				assert_eq!(
					TestPools::pool_user_data(CurrencyId::DOT, ALICE).total_borrowed,
					alice_borrowed_amount_in_dot
				);
				// Checking total borrow for DOT pool
				let expected_borrow_interest_accumulated = 421875000000000;
				assert_eq!(
					TestPools::pools(CurrencyId::DOT).total_borrowed,
					alice_borrowed_amount_in_dot + expected_borrow_interest_accumulated
				);

				System::set_block_number(7);

				// Alice try to redeem all from ETH pool
				assert_noop!(
					MinterestProtocol::redeem_underlying(
						Origin::signed(ALICE),
						CurrencyId::ETH,
						alice_deposited_amount_in_eth + alice_deposited_amount_in_eth_secondary
					),
					MinterestProtocolError::<Test>::RedeemControllerRejection
				);
			});
	}

	// Extrinsic `redeem_underlying`, description of scenario #2:
	// The user tries to redeem all assets in the first currency. He has loan in the second currency.
	// Initial exchange rate for all assets equal 1.0;
	// Collateral factor for all assets equal 0.9;
	// 1. Alice deposit 60 DOT;
	// 2. Alice borrow 50 ETH;
	// 3. Alice can't `redeem` 60 DOT: 0 DOT collateral < 50 ETH borrow;
	#[test]
	fn redeem_underlying_with_another_currency_borrowing() {
		ExtBuilder::default()
			.user_balance(ADMIN, CurrencyId::DOT, ONE_HUNDRED)
			.user_balance(ADMIN, CurrencyId::ETH, ONE_HUNDRED)
			.user_balance(ALICE, CurrencyId::DOT, ONE_HUNDRED)
			.pool_user_data(CurrencyId::DOT, ALICE, BALANCE_ZERO, RATE_ZERO, true, 0)
			.pool_balance(CurrencyId::DOT, BALANCE_ZERO)
			.build()
			.execute_with(|| {
				// Set initial balance
				assert_ok!(MinterestProtocol::deposit_underlying(
					Origin::signed(ADMIN),
					CurrencyId::DOT,
					ONE_HUNDRED
				));
				assert_ok!(MinterestProtocol::deposit_underlying(
					Origin::signed(ADMIN),
					CurrencyId::ETH,
					ONE_HUNDRED
				));
				// Alice deposit to DOT pool
				let alice_deposited_amount_in_dot = 60_000 * DOLLARS;
				assert_ok!(MinterestProtocol::deposit_underlying(
					Origin::signed(ALICE),
					CurrencyId::DOT,
					alice_deposited_amount_in_dot
				));

				System::set_block_number(2);

				// Alice borrow from ETH pool
				let alice_borrowed_amount_in_eth = 50_000 * DOLLARS;
				assert_ok!(MinterestProtocol::borrow(
					Origin::signed(ALICE),
					CurrencyId::ETH,
					alice_borrowed_amount_in_eth
				));

				// Checking free balance DOT && ETH for user.
				assert_eq!(
					Currencies::free_balance(CurrencyId::DOT, &ALICE),
					ONE_HUNDRED - alice_deposited_amount_in_dot
				);
				assert_eq!(
					Currencies::free_balance(CurrencyId::ETH, &ALICE),
					alice_borrowed_amount_in_eth
				);
				// Checking total borrow for Alice ETH pool
				assert_eq!(
					TestPools::pool_user_data(CurrencyId::ETH, ALICE).total_borrowed,
					alice_borrowed_amount_in_eth
				);
				// // Checking total borrow for ETH pool
				assert_eq!(
					TestPools::pools(CurrencyId::ETH).total_borrowed,
					alice_borrowed_amount_in_eth
				);

				System::set_block_number(3);

				// Alice redeem all DOTs
				assert_noop!(
					MinterestProtocol::redeem_underlying(
						Origin::signed(ALICE),
						CurrencyId::DOT,
						alice_deposited_amount_in_dot
					),
					MinterestProtocolError::<Test>::RedeemControllerRejection
				);

				// Checking free balance DOT && ETH for user.
				// Expected previously values
				assert_eq!(
					Currencies::free_balance(CurrencyId::DOT, &ALICE),
					ONE_HUNDRED - alice_deposited_amount_in_dot
				);
				assert_eq!(
					Currencies::free_balance(CurrencyId::ETH, &ALICE),
					alice_borrowed_amount_in_eth
				);

				// Checking total borrow for Alice ETH pool
				assert_eq!(
					TestPools::pool_user_data(CurrencyId::ETH, ALICE).total_borrowed,
					alice_borrowed_amount_in_eth
				);
				// Checking total borrow for ETH pool
				assert_eq!(
					TestPools::pools(CurrencyId::ETH).total_borrowed,
					alice_borrowed_amount_in_eth
				);
			});
	}

	// Extrinsic `redeem_underlying`, description of scenario #3:
	// The user tries to redeem all assets in the first currency. He has loan in the second
	// currency and deposit in the third currency.
	// Initial exchange rate for all assets equal 1.0;
	// Collateral factor for all assets equal 0.9;
	// 1. Alice deposit 40 DOT;
	// 2. Alice deposit 40 BTC;
	// 3. Alice borrow 70 ETH;
	// 4. Alice can't `redeem_underlying` 40 DOT;
	// 5. Alice deposit 40 BTC;
	// 6. Alice redeem 40 DOT;
	// 7. Alice can't `redeem_underlying` 40 BTC;
	#[test]
	fn redeem_underlying_with_third_currency_borrowing() {
		ExtBuilder::default()
			.pool_initial(CurrencyId::DOT)
			.pool_initial(CurrencyId::ETH)
			.user_balance(ADMIN, CurrencyId::ETH, ONE_HUNDRED)
			.user_balance(ALICE, CurrencyId::DOT, ONE_HUNDRED)
			.user_balance(ALICE, CurrencyId::BTC, ONE_HUNDRED)
			.pool_user_data(CurrencyId::DOT, ALICE, BALANCE_ZERO, RATE_ZERO, true, 0)
			.pool_user_data(CurrencyId::BTC, ALICE, BALANCE_ZERO, RATE_ZERO, true, 0)
			.build()
			.execute_with(|| {
				// Set initial balance
				assert_ok!(MinterestProtocol::deposit_underlying(
					Origin::signed(ADMIN),
					CurrencyId::ETH,
					ONE_HUNDRED
				));

				// Alice deposit to DOT pool
				let alice_deposited_amount_in_dot = 40_000 * DOLLARS;
				assert_ok!(MinterestProtocol::deposit_underlying(
					Origin::signed(ALICE),
					CurrencyId::DOT,
					alice_deposited_amount_in_dot
				));

				System::set_block_number(2);

				// Alice deposit to BTC pool
				let alice_deposited_amount_in_btc = 40_000 * DOLLARS;
				assert_ok!(MinterestProtocol::deposit_underlying(
					Origin::signed(ALICE),
					CurrencyId::BTC,
					alice_deposited_amount_in_btc
				));

				System::set_block_number(3);

				// Alice borrow from ETH pool
				let alice_borrowed_amount_in_eth = 70_000 * DOLLARS;
				assert_ok!(MinterestProtocol::borrow(
					Origin::signed(ALICE),
					CurrencyId::ETH,
					alice_borrowed_amount_in_eth
				));

				System::set_block_number(4);

				// Checking free balance DOT && ETH && BTC for user.
				assert_eq!(
					Currencies::free_balance(CurrencyId::DOT, &ALICE),
					ONE_HUNDRED - alice_deposited_amount_in_dot
				);
				assert_eq!(
					Currencies::free_balance(CurrencyId::BTC, &ALICE),
					ONE_HUNDRED - alice_deposited_amount_in_btc
				);
				assert_eq!(
					Currencies::free_balance(CurrencyId::ETH, &ALICE),
					alice_borrowed_amount_in_eth
				);
				// Checking total borrow for Alice ETH pool
				assert_eq!(
					TestPools::pool_user_data(CurrencyId::ETH, ALICE).total_borrowed,
					alice_borrowed_amount_in_eth
				);
				// Checking total borrow for ETH pool
				assert_eq!(
					TestPools::pools(CurrencyId::ETH).total_borrowed,
					alice_borrowed_amount_in_eth
				);

				// Alice try to redeem all DOTs
				assert_noop!(
					MinterestProtocol::redeem_underlying(
						Origin::signed(ALICE),
						CurrencyId::DOT,
						alice_deposited_amount_in_dot
					),
					MinterestProtocolError::<Test>::RedeemControllerRejection
				);

				System::set_block_number(5);

				// Alice add liquidity to BTC pool
				let alice_deposited_amount_in_btc_secondary = 40_000 * DOLLARS;
				assert_ok!(MinterestProtocol::deposit_underlying(
					Origin::signed(ALICE),
					CurrencyId::BTC,
					alice_deposited_amount_in_btc_secondary
				));

				System::set_block_number(6);

				// Alice redeem all DOTs
				let alice_current_balance_amount_in_m_dot = Currencies::free_balance(CurrencyId::MDOT, &ALICE);
				let alice_redeemed_amount_in_dot =
					TestPools::convert_from_wrapped(CurrencyId::MDOT, alice_current_balance_amount_in_m_dot).unwrap();
				assert_ok!(MinterestProtocol::redeem_underlying(
					Origin::signed(ALICE),
					CurrencyId::DOT,
					alice_redeemed_amount_in_dot
				));

				// Checking pool available liquidity.
				assert_eq!(
					TestPools::get_pool_available_liquidity(CurrencyId::DOT),
					alice_deposited_amount_in_dot - alice_redeemed_amount_in_dot
				);
				// Checking free balance DOT && ETH && BTC for user.
				assert_eq!(
					Currencies::free_balance(CurrencyId::DOT, &ALICE),
					ONE_HUNDRED - alice_deposited_amount_in_dot + alice_redeemed_amount_in_dot
				);
				assert_eq!(
					Currencies::free_balance(CurrencyId::BTC, &ALICE),
					ONE_HUNDRED - alice_deposited_amount_in_btc - alice_deposited_amount_in_btc_secondary
				);
				assert_eq!(
					Currencies::free_balance(CurrencyId::ETH, &ALICE),
					alice_borrowed_amount_in_eth
				);
				// Checking total borrow for Alice ETH pool
				assert_eq!(
					TestPools::pool_user_data(CurrencyId::ETH, ALICE).total_borrowed,
					alice_borrowed_amount_in_eth
				);
				// Checking total borrow for ETH pool
				assert_eq!(
					TestPools::pools(CurrencyId::ETH).total_borrowed,
					alice_borrowed_amount_in_eth
				);

				System::set_block_number(7);

				// Alice try to redeem all BTC.
				assert_noop!(
					MinterestProtocol::redeem_underlying(
						Origin::signed(ALICE),
						CurrencyId::BTC,
						alice_deposited_amount_in_btc_secondary
					),
					MinterestProtocolError::<Test>::RedeemControllerRejection
				);
			});
	}

	// Extrinsic `redeem_underlying`, description of scenario #4:
	// It is possible to redeem assets from the extra liquidity from Admin.
	// 1. Admin deposit 10 DOT to pool;
	// 2. Alice deposit 20 DOT;
	// 3. Bob deposit 20 BTC;
	// 4. Bob deposit 10 DOT;
	// 5. Bob borrow 15 DOT;
	// 6. Alice redeem 20 DOT;
	// 7. DOT pool extra liquidity equals 5 DOT;
	#[test]
	fn redeem_underlying_over_insurance() {
		ExtBuilder::default()
			.pool_initial(CurrencyId::DOT)
			.user_balance(ADMIN, CurrencyId::DOT, ONE_HUNDRED)
			.user_balance(ALICE, CurrencyId::DOT, ONE_HUNDRED)
			.user_balance(BOB, CurrencyId::BTC, ONE_HUNDRED)
			.user_balance(BOB, CurrencyId::DOT, ONE_HUNDRED)
			.pool_user_data(CurrencyId::DOT, ALICE, BALANCE_ZERO, RATE_ZERO, false, 0)
			.pool_user_data(CurrencyId::BTC, BOB, BALANCE_ZERO, RATE_ZERO, true, 0)
			.build()
			.execute_with(|| {
				// Set initial balance
				assert_ok!(MinterestProtocol::deposit_underlying(
					Origin::signed(ADMIN),
					CurrencyId::DOT,
					10_000 * DOLLARS
				));

				// Alice deposit to DOT pool
				let alice_deposited_amount_in_dot = 20_000 * DOLLARS;
				assert_ok!(MinterestProtocol::deposit_underlying(
					Origin::signed(ALICE),
					CurrencyId::DOT,
					alice_deposited_amount_in_dot
				));

				System::set_block_number(2);

				// Bob deposit to BTC pool
				let bob_deposited_amount_in_btc = 20_000 * DOLLARS;
				assert_ok!(MinterestProtocol::deposit_underlying(
					Origin::signed(BOB),
					CurrencyId::BTC,
					bob_deposited_amount_in_btc
				));

				System::set_block_number(3);

				// Bob borrow from DOT pool
				let bob_borrowed_amount_in_dot = 15_000 * DOLLARS;
				assert_ok!(MinterestProtocol::borrow(
					Origin::signed(BOB),
					CurrencyId::DOT,
					bob_borrowed_amount_in_dot
				));

				System::set_block_number(4);

				// Bob deposit to DOT pool
				let bob_deposited_amount_in_dot = 10_000 * DOLLARS;
				assert_ok!(MinterestProtocol::deposit_underlying(
					Origin::signed(BOB),
					CurrencyId::DOT,
					bob_deposited_amount_in_dot
				));

				System::set_block_number(5);

				// Alice redeem all DOTs.
				let alice_current_balance_amount_in_m_dot = Currencies::free_balance(CurrencyId::MDOT, &ALICE);
				// Expected exchange rate 1000000006581250024
				let alice_redeemed_amount_in_dot =
					TestPools::convert_from_wrapped(CurrencyId::MDOT, alice_current_balance_amount_in_m_dot).unwrap();
				assert_ok!(MinterestProtocol::redeem_underlying(
					Origin::signed(ALICE),
					CurrencyId::DOT,
					alice_redeemed_amount_in_dot
				));

				// Checking pool available liquidity.
				assert_eq!(
					TestPools::get_pool_available_liquidity(CurrencyId::DOT),
					10_000 * DOLLARS + alice_deposited_amount_in_dot - alice_redeemed_amount_in_dot
						+ bob_deposited_amount_in_dot
						- bob_borrowed_amount_in_dot
				);

				// Checking free balance DOT && BTC for user.
				assert_eq!(
					Currencies::free_balance(CurrencyId::DOT, &ALICE),
					ONE_HUNDRED - alice_deposited_amount_in_dot + alice_redeemed_amount_in_dot
				);
				assert_eq!(
					Currencies::free_balance(CurrencyId::DOT, &BOB),
					ONE_HUNDRED + bob_borrowed_amount_in_dot - bob_deposited_amount_in_dot
				);
				assert_eq!(
					Currencies::free_balance(CurrencyId::BTC, &BOB),
					ONE_HUNDRED - bob_deposited_amount_in_btc
				);
			});
	}

	// Extrinsic `redeem`, description of scenario #1:
	// The user tries to redeem all assets in the first currency. He has loan in the first currency.
	// Initial exchange rate for all assets equal 1.0;
	// Collateral factor for all assets equal 0.9;
	// 1. Alice deposit 60 DOT;
	// 2. Alice deposit 50 ETH;
	// 3. Alice borrow 50 DOT;
	// 4. Alice can't `redeem` 60 DOT: 10 DOT * 0.9 + 50 ETH * 0.9 collateral < 60 DOT redeem;
	// 5. Alice deposit 10 ETH;
	// 6. Alice `redeem` 60 DOT;
	// 7. Alice can't `redeem` 60 ETH.
	#[test]
	fn redeem_with_current_currency_borrowing() {
		ExtBuilder::default()
			.pool_initial(CurrencyId::DOT)
			.user_balance(ADMIN, CurrencyId::DOT, ONE_HUNDRED)
			.user_balance(ALICE, CurrencyId::DOT, ONE_HUNDRED)
			.user_balance(ALICE, CurrencyId::ETH, ONE_HUNDRED)
			.user_balance(BOB, CurrencyId::DOT, 100_000_000 * DOLLARS)
			.pool_user_data(CurrencyId::DOT, ALICE, BALANCE_ZERO, RATE_ZERO, true, 0)
			.pool_user_data(CurrencyId::ETH, ALICE, BALANCE_ZERO, RATE_ZERO, true, 0)
			.build()
			.execute_with(|| {
				// Set initial balance
				assert_ok!(MinterestProtocol::deposit_underlying(
					Origin::signed(ADMIN),
					CurrencyId::DOT,
					ONE_HUNDRED
				));

				// Alice deposit to DOT pool
				let alice_deposited_amount_in_dot = 60_000 * DOLLARS;
				assert_ok!(MinterestProtocol::deposit_underlying(
					Origin::signed(ALICE),
					CurrencyId::DOT,
					alice_deposited_amount_in_dot
				));

				System::set_block_number(2);

				// Alice deposit to ETH pool
				let alice_deposited_amount_in_eth = 50_000 * DOLLARS;
				assert_ok!(MinterestProtocol::deposit_underlying(
					Origin::signed(ALICE),
					CurrencyId::ETH,
					alice_deposited_amount_in_eth
				));

				System::set_block_number(3);

				// Alice borrow from DOT pool
				let alice_borrowed_amount_in_dot = 50_000 * DOLLARS;
				assert_ok!(MinterestProtocol::borrow(
					Origin::signed(ALICE),
					CurrencyId::DOT,
					alice_borrowed_amount_in_dot
				));

				// Checking pool available liquidity
				assert_eq!(
					TestPools::get_pool_available_liquidity(CurrencyId::DOT),
					ONE_HUNDRED + alice_deposited_amount_in_dot - alice_borrowed_amount_in_dot
				);

				// Checking free balance DOT && MDOT in pool.
				assert_eq!(
					Currencies::free_balance(CurrencyId::DOT, &ALICE),
					ONE_HUNDRED - alice_deposited_amount_in_dot + alice_borrowed_amount_in_dot
				);
				assert_eq!(
					Currencies::free_balance(CurrencyId::ETH, &ALICE),
					ONE_HUNDRED - alice_deposited_amount_in_eth
				);
				let expected_amount_wrapped_tokens_in_dot =
					TestPools::convert_to_wrapped(CurrencyId::DOT, alice_deposited_amount_in_dot).unwrap();
				assert_eq!(
					Currencies::free_balance(CurrencyId::MDOT, &ALICE),
					expected_amount_wrapped_tokens_in_dot
				);
				let expected_amount_wrapped_tokens_in_eth =
					TestPools::convert_to_wrapped(CurrencyId::DOT, alice_deposited_amount_in_eth).unwrap();
				assert_eq!(
					Currencies::free_balance(CurrencyId::METH, &ALICE),
					expected_amount_wrapped_tokens_in_eth
				);

				// Checking total borrow for Alice DOT pool
				assert_eq!(
					TestPools::pool_user_data(CurrencyId::DOT, ALICE).total_borrowed,
					alice_borrowed_amount_in_dot
				);
				// Checking total borrow for DOT pool
				assert_eq!(
					TestPools::pools(CurrencyId::DOT).total_borrowed,
					alice_borrowed_amount_in_dot
				);

				System::set_block_number(4);

				// Alice try to redeem all from DOT pool
				assert_noop!(
					MinterestProtocol::redeem(Origin::signed(ALICE), CurrencyId::DOT),
					MinterestProtocolError::<Test>::RedeemControllerRejection
				);

				System::set_block_number(5);

				// Alice add liquidity to ETH pool
				let alice_deposited_amount_in_eth_secondary = 10_000 * DOLLARS;
				assert_ok!(MinterestProtocol::deposit_underlying(
					Origin::signed(ALICE),
					CurrencyId::ETH,
					alice_deposited_amount_in_eth_secondary
				));

				// Bob add liquidity to ETH pool
				let bob_deposited_amount_in_dot = 10_000 * DOLLARS;
				assert_ok!(MinterestProtocol::deposit_underlying(
					Origin::signed(BOB),
					CurrencyId::DOT,
					bob_deposited_amount_in_dot
				));

				System::set_block_number(6);

				// Alice redeem all DOTs
				assert_ok!(MinterestProtocol::redeem(Origin::signed(ALICE), CurrencyId::DOT));

				// Checking free balance DOT/MDOT && ETH/METH in pool.
				let expected_amount_redeemed_underlying_assets = 60000000136963397880000;
				assert_eq!(
					Currencies::free_balance(CurrencyId::DOT, &ALICE),
					ONE_HUNDRED - alice_deposited_amount_in_dot
						+ alice_borrowed_amount_in_dot
						+ expected_amount_redeemed_underlying_assets
				);
				assert_eq!(
					Currencies::free_balance(CurrencyId::ETH, &ALICE),
					ONE_HUNDRED - alice_deposited_amount_in_eth - alice_deposited_amount_in_eth_secondary
				);

				assert_eq!(Currencies::free_balance(CurrencyId::MDOT, &ALICE), 0);
				let expected_amount_wrapped_tokens_in_eth_summary = expected_amount_wrapped_tokens_in_eth
					+ TestPools::convert_to_wrapped(CurrencyId::ETH, alice_deposited_amount_in_eth_secondary).unwrap();
				assert_eq!(
					Currencies::free_balance(CurrencyId::METH, &ALICE),
					expected_amount_wrapped_tokens_in_eth_summary
				);

				// Checking total borrow for Alice DOT pool
				let expected_amount_accumulated_in_dot = 413602942444485;
				assert_eq!(
					TestPools::pool_user_data(CurrencyId::DOT, ALICE).total_borrowed,
					alice_borrowed_amount_in_dot
				);
				// Checking total borrow for DOT pool
				assert_eq!(
					TestPools::pools(CurrencyId::DOT).total_borrowed,
					alice_borrowed_amount_in_dot + expected_amount_accumulated_in_dot
				);

				System::set_block_number(7);

				// Alice try to redeem all from ETH pool
				assert_noop!(
					MinterestProtocol::redeem(Origin::signed(ALICE), CurrencyId::ETH),
					MinterestProtocolError::<Test>::RedeemControllerRejection
				);
			});
	}

	// Extrinsic `redeem`, description of scenario #2:
	// The user tries to redeem all assets in the first currency. He has loan in the second currency.
	// Initial exchange rate for all assets equal 1.0;
	// Collateral factor for all assets equal 0.9;
	// 1. Alice deposit 60 DOT;
	// 2. Alice borrow 50 ETH;
	// 3. Alice can't `redeem` 60 DOT: 0 DOT collateral < 50 ETH borrow;
	#[test]
	fn redeem_with_another_currency_borrowing() {
		ExtBuilder::default()
			.user_balance(ADMIN, CurrencyId::DOT, ONE_HUNDRED)
			.user_balance(ADMIN, CurrencyId::ETH, ONE_HUNDRED)
			.user_balance(ALICE, CurrencyId::DOT, ONE_HUNDRED)
			.pool_user_data(CurrencyId::DOT, ALICE, BALANCE_ZERO, RATE_ZERO, true, 0)
			.pool_balance(CurrencyId::DOT, BALANCE_ZERO)
			.build()
			.execute_with(|| {
				// Set initial balance
				assert_ok!(MinterestProtocol::deposit_underlying(
					Origin::signed(ADMIN),
					CurrencyId::DOT,
					ONE_HUNDRED
				));
				assert_ok!(MinterestProtocol::deposit_underlying(
					Origin::signed(ADMIN),
					CurrencyId::ETH,
					ONE_HUNDRED
				));
				// Alice deposit to DOT pool
				let alice_deposited_amount_in_dot = 60_000 * DOLLARS;
				assert_ok!(MinterestProtocol::deposit_underlying(
					Origin::signed(ALICE),
					CurrencyId::DOT,
					alice_deposited_amount_in_dot
				));

				System::set_block_number(2);

				// Alice borrow from ETH pool
				let alice_borrowed_amount_in_eth = 50_000 * DOLLARS;
				assert_ok!(MinterestProtocol::borrow(
					Origin::signed(ALICE),
					CurrencyId::ETH,
					alice_borrowed_amount_in_eth
				));

				// Checking free balance DOT && ETH for user.
				assert_eq!(
					Currencies::free_balance(CurrencyId::DOT, &ALICE),
					ONE_HUNDRED - alice_deposited_amount_in_dot
				);
				assert_eq!(
					Currencies::free_balance(CurrencyId::ETH, &ALICE),
					alice_borrowed_amount_in_eth
				);
				// Checking total borrow for Alice ETH pool
				assert_eq!(
					TestPools::pool_user_data(CurrencyId::ETH, ALICE).total_borrowed,
					alice_borrowed_amount_in_eth
				);
				// // Checking total borrow for ETH pool
				assert_eq!(
					TestPools::pools(CurrencyId::ETH).total_borrowed,
					alice_borrowed_amount_in_eth
				);

				System::set_block_number(3);

				// Alice redeem all DOTs
				assert_noop!(
					MinterestProtocol::redeem(Origin::signed(ALICE), CurrencyId::DOT),
					MinterestProtocolError::<Test>::RedeemControllerRejection
				);

				// Checking free balance DOT && ETH for user.
				// Expected previously values
				assert_eq!(
					Currencies::free_balance(CurrencyId::DOT, &ALICE),
					ONE_HUNDRED - alice_deposited_amount_in_dot
				);
				assert_eq!(
					Currencies::free_balance(CurrencyId::ETH, &ALICE),
					alice_borrowed_amount_in_eth
				);

				// Checking total borrow for Alice ETH pool
				assert_eq!(
					TestPools::pool_user_data(CurrencyId::ETH, ALICE).total_borrowed,
					alice_borrowed_amount_in_eth
				);
				// Checking total borrow for ETH pool
				assert_eq!(
					TestPools::pools(CurrencyId::ETH).total_borrowed,
					alice_borrowed_amount_in_eth
				);
			});
	}

	// Extrinsic `redeem`, description of scenario #3:
	// The user tries to redeem all assets in the first currency. He has loan in the second
	// currency and deposit in the third currency.
	// Initial exchange rate for all assets equal 1.0;
	// Collateral factor for all assets equal 0.9;
	// 1. Alice deposit 40 DOT;
	// 2. Alice deposit 40 BTC;
	// 3. Alice borrow 70 ETH;
	// 4. Alice can't `redeem` 40 DOT: (40 BTC * 0.9) collateral < 70 ETH borrow;
	// 5. Alice deposit 40 BTC;
	// 6. Alice redeem 40 DOT: (80 BTC * 0.9) collateral > 70 EHT borrow;
	// 7. Alice can't `redeem` 40 BTC: (40 BTC * 0.9) collateral < 70 ETH borrow;
	#[test]
	fn redeem_with_third_currency_borrowing() {
		ExtBuilder::default()
			.pool_initial(CurrencyId::DOT)
			.pool_initial(CurrencyId::ETH)
			.user_balance(ADMIN, CurrencyId::ETH, ONE_HUNDRED)
			.user_balance(ALICE, CurrencyId::DOT, ONE_HUNDRED)
			.user_balance(ALICE, CurrencyId::BTC, ONE_HUNDRED)
			.pool_user_data(CurrencyId::DOT, ALICE, BALANCE_ZERO, RATE_ZERO, true, 0)
			.pool_user_data(CurrencyId::BTC, ALICE, BALANCE_ZERO, RATE_ZERO, true, 0)
			.build()
			.execute_with(|| {
				// Set initial balance
				assert_ok!(MinterestProtocol::deposit_underlying(
					Origin::signed(ADMIN),
					CurrencyId::ETH,
					ONE_HUNDRED
				));

				// Alice deposit to DOT pool
				let alice_deposited_amount_in_dot = 40_000 * DOLLARS;
				assert_ok!(MinterestProtocol::deposit_underlying(
					Origin::signed(ALICE),
					CurrencyId::DOT,
					alice_deposited_amount_in_dot
				));

				System::set_block_number(2);

				// Alice deposit to BTC pool
				let alice_deposited_amount_in_btc = 40_000 * DOLLARS;
				assert_ok!(MinterestProtocol::deposit_underlying(
					Origin::signed(ALICE),
					CurrencyId::BTC,
					alice_deposited_amount_in_btc
				));

				System::set_block_number(3);

				// Alice borrow from ETH pool
				let alice_borrowed_amount_in_eth = 70_000 * DOLLARS;
				assert_ok!(MinterestProtocol::borrow(
					Origin::signed(ALICE),
					CurrencyId::ETH,
					alice_borrowed_amount_in_eth
				));

				// Checking free balance DOT && ETH && BTC for user.
				assert_eq!(
					Currencies::free_balance(CurrencyId::DOT, &ALICE),
					ONE_HUNDRED - alice_deposited_amount_in_dot
				);
				assert_eq!(
					Currencies::free_balance(CurrencyId::BTC, &ALICE),
					ONE_HUNDRED - alice_deposited_amount_in_btc
				);
				assert_eq!(
					Currencies::free_balance(CurrencyId::ETH, &ALICE),
					alice_borrowed_amount_in_eth
				);
				// Checking total borrow for Alice ETH pool
				assert_eq!(
					TestPools::pool_user_data(CurrencyId::ETH, ALICE).total_borrowed,
					alice_borrowed_amount_in_eth
				);
				// Checking total borrow for ETH pool
				assert_eq!(
					TestPools::pools(CurrencyId::ETH).total_borrowed,
					alice_borrowed_amount_in_eth
				);

				System::set_block_number(4);

				// Alice try to redeem all DOTs
				assert_noop!(
					MinterestProtocol::redeem(Origin::signed(ALICE), CurrencyId::DOT),
					MinterestProtocolError::<Test>::RedeemControllerRejection
				);

				System::set_block_number(5);

				// Alice add liquidity to BTC pool
				let alice_deposited_amount_in_btc_secondary = 40_000 * DOLLARS;
				assert_ok!(MinterestProtocol::deposit_underlying(
					Origin::signed(ALICE),
					CurrencyId::BTC,
					alice_deposited_amount_in_btc_secondary
				));

				System::set_block_number(6);

				// Alice redeem all DOTs
				let alice_current_balance_amount_in_m_dot = Currencies::free_balance(CurrencyId::MDOT, &ALICE);
				let alice_redeemed_amount_in_dot =
					TestPools::convert_from_wrapped(CurrencyId::MDOT, alice_current_balance_amount_in_m_dot).unwrap();
				assert_ok!(MinterestProtocol::redeem(Origin::signed(ALICE), CurrencyId::DOT));

				// Checking free balance DOT && ETH && BTC for user.
				assert_eq!(
					Currencies::free_balance(CurrencyId::DOT, &ALICE),
					ONE_HUNDRED - alice_deposited_amount_in_dot + alice_redeemed_amount_in_dot
				);
				assert_eq!(
					Currencies::free_balance(CurrencyId::BTC, &ALICE),
					ONE_HUNDRED - alice_deposited_amount_in_btc - alice_deposited_amount_in_btc_secondary
				);
				assert_eq!(
					Currencies::free_balance(CurrencyId::ETH, &ALICE),
					alice_borrowed_amount_in_eth
				);
				// Checking total borrow for Alice ETH pool
				assert_eq!(
					TestPools::pool_user_data(CurrencyId::ETH, ALICE).total_borrowed,
					alice_borrowed_amount_in_eth
				);
				// Checking total borrow for ETH pool
				assert_eq!(
					TestPools::pools(CurrencyId::ETH).total_borrowed,
					alice_borrowed_amount_in_eth
				);

				System::set_block_number(7);

				// Alice try to redeem all BTC.
				assert_noop!(
					MinterestProtocol::redeem(Origin::signed(ALICE), CurrencyId::BTC),
					MinterestProtocolError::<Test>::RedeemControllerRejection
				);
			});
	}

	// Extrinsic `redeem`, description of scenario #4:
	// It is possible to redeem assets from the extra liquidity from Admin.
	// 1. Admin deposit 10 DOT to pool;
	// 2. Alice deposit 20 DOT;
	// 3. Bob deposit 20 BTC;
	// 4. Bob deposit 10 DOT;
	// 5. Bob borrow 15 DOT;
	// 6. Alice redeem 20 DOT, pool extra liquidity equals 5 DOT;
	#[test]
	fn redeem_over_insurance() {
		ExtBuilder::default()
			.pool_initial(CurrencyId::DOT)
			.user_balance(ADMIN, CurrencyId::DOT, ONE_HUNDRED)
			.user_balance(ALICE, CurrencyId::DOT, ONE_HUNDRED)
			.user_balance(BOB, CurrencyId::DOT, ONE_HUNDRED)
			.user_balance(BOB, CurrencyId::BTC, ONE_HUNDRED)
			.pool_user_data(CurrencyId::DOT, ALICE, BALANCE_ZERO, RATE_ZERO, false, 0)
			.pool_user_data(CurrencyId::DOT, BOB, BALANCE_ZERO, RATE_ZERO, false, 0)
			.pool_user_data(CurrencyId::BTC, BOB, BALANCE_ZERO, RATE_ZERO, true, 0)
			.build()
			.execute_with(|| {
				// Set initial balance in pool
				assert_ok!(MinterestProtocol::deposit_underlying(
					Origin::signed(ADMIN),
					CurrencyId::DOT,
					10_000 * DOLLARS
				));
				// Alice deposit to DOT pool
				let alice_deposited_amount_in_dot = 20_000 * DOLLARS;
				assert_ok!(MinterestProtocol::deposit_underlying(
					Origin::signed(ALICE),
					CurrencyId::DOT,
					alice_deposited_amount_in_dot
				));

				System::set_block_number(2);

				// Bob deposit to BTC pool
				let bob_deposited_amount_in_btc = 20_000 * DOLLARS;
				assert_ok!(MinterestProtocol::deposit_underlying(
					Origin::signed(BOB),
					CurrencyId::BTC,
					bob_deposited_amount_in_btc
				));

				// Bob deposit to DOT pool
				let bob_deposited_amount_in_dot = 10_000 * DOLLARS;
				assert_ok!(MinterestProtocol::deposit_underlying(
					Origin::signed(BOB),
					CurrencyId::DOT,
					bob_deposited_amount_in_dot
				));

				System::set_block_number(3);

				// Bob borrow from DOT pool
				let bob_borrowed_amount_in_dot = 15_000 * DOLLARS;
				assert_ok!(MinterestProtocol::borrow(
					Origin::signed(BOB),
					CurrencyId::DOT,
					bob_borrowed_amount_in_dot
				));

				System::set_block_number(4);

				// Alice redeem all DOTs.
				let alice_current_balance_amount_in_m_dot = Currencies::free_balance(CurrencyId::MDOT, &ALICE);

				assert_ok!(MinterestProtocol::redeem(Origin::signed(ALICE), CurrencyId::DOT));

				let alice_redeemed_amount_in_dot =
					TestPools::convert_from_wrapped(CurrencyId::MDOT, alice_current_balance_amount_in_m_dot).unwrap();

				// Checking pool available liquidity.
				assert_eq!(
					TestPools::get_pool_available_liquidity(CurrencyId::DOT),
					10_000 * DOLLARS + alice_deposited_amount_in_dot - alice_redeemed_amount_in_dot
						+ bob_deposited_amount_in_dot
						- bob_borrowed_amount_in_dot
				);

				// Checking free balance DOT && BTC for user.
				assert_eq!(
					Currencies::free_balance(CurrencyId::DOT, &ALICE),
					ONE_HUNDRED - alice_deposited_amount_in_dot + alice_redeemed_amount_in_dot
				);
				assert_eq!(
					Currencies::free_balance(CurrencyId::DOT, &BOB),
					ONE_HUNDRED + bob_borrowed_amount_in_dot - bob_deposited_amount_in_dot
				);
				assert_eq!(
					Currencies::free_balance(CurrencyId::BTC, &BOB),
					ONE_HUNDRED - bob_deposited_amount_in_btc
				);
			});
	}

	// Extrinsic `borrow`, description of scenario #1:
	// The user cannot borrow without making a deposit first.
	// 1. Alice can't borrow 50 DOT: 0 collateral < 50 DOT borrow;
	#[test]
	fn borrow_with_insufficient_collateral_no_deposits() {
		ExtBuilder::default()
			.pool_initial(CurrencyId::DOT)
			.user_balance(ADMIN, CurrencyId::DOT, ONE_HUNDRED)
			.user_balance(ALICE, CurrencyId::DOT, ONE_HUNDRED)
			.pool_user_data(CurrencyId::DOT, ALICE, BALANCE_ZERO, RATE_ZERO, true, 0)
			.build()
			.execute_with(|| {
				// Set initial balance
				assert_ok!(MinterestProtocol::deposit_underlying(
					Origin::signed(ADMIN),
					CurrencyId::DOT,
					ONE_HUNDRED
				));

				// Alice try to borrow from DOT pool
				let alice_borrowed_amount_in_dot = 50_000 * DOLLARS;
				assert_noop!(
					MinterestProtocol::borrow(Origin::signed(ALICE), CurrencyId::DOT, alice_borrowed_amount_in_dot),
					MinterestProtocolError::<Test>::BorrowControllerRejection
				);

				// Checking pool available liquidity
				assert_eq!(TestPools::get_pool_available_liquidity(CurrencyId::DOT), ONE_HUNDRED);
			});
	}

	// Extrinsic `borrow`, description of scenario #2:
	// The user cannot borrow in the second currency unless he has
	// not enabled the first currency as collateral.
	// Initial exchange rate for all assets equal 1.0;
	// Collateral factor for all assets equal 0.9;
	// 1. Alice deposit 50 DOT;
	// 2. Alice can't borrow 50 ETH: 0 collateral < 50 ETH borrow;
	#[test]
	fn borrow_without_collateral_in_second_currency() {
		ExtBuilder::default()
			.user_balance(ADMIN, CurrencyId::DOT, ONE_HUNDRED)
			.user_balance(ADMIN, CurrencyId::ETH, ONE_HUNDRED)
			.user_balance(ALICE, CurrencyId::DOT, ONE_HUNDRED)
			.pool_user_data(CurrencyId::DOT, ALICE, BALANCE_ZERO, RATE_ZERO, false, 0)
			.build()
			.execute_with(|| {
				// Set initial balance
				assert_ok!(MinterestProtocol::deposit_underlying(
					Origin::signed(ADMIN),
					CurrencyId::DOT,
					ONE_HUNDRED
				));
				assert_ok!(MinterestProtocol::deposit_underlying(
					Origin::signed(ADMIN),
					CurrencyId::ETH,
					ONE_HUNDRED
				));

				// Alice deposit to DOT pool
				let alice_deposited_amount = 50_000 * DOLLARS;
				assert_ok!(MinterestProtocol::deposit_underlying(
					Origin::signed(ALICE),
					CurrencyId::DOT,
					alice_deposited_amount
				));

				System::set_block_number(2);

				// Alice try to borrow from ETH pool
				let alice_borrowed_amount = 50_000 * DOLLARS;
				assert_noop!(
					MinterestProtocol::borrow(Origin::signed(ALICE), CurrencyId::ETH, alice_borrowed_amount),
					MinterestProtocolError::<Test>::BorrowControllerRejection
				);

				// Checking pool available liquidity
				assert_eq!(
					TestPools::get_pool_available_liquidity(CurrencyId::DOT),
					ONE_HUNDRED + alice_deposited_amount
				);
				assert_eq!(TestPools::get_pool_available_liquidity(CurrencyId::ETH), ONE_HUNDRED);
			});
	}

	// Extrinsic `borrow`, description of scenario #3:
	// The user cannot borrow in the second currency if the collateral in the first currency
	// is insufficient.
	// Initial exchange rate for all assets equal 1.0;
	// Collateral factor for all assets equal 0.9;
	// 1. Alice deposit 50 DOT;
	// 2. Alice can't borrow 50 ETH: 50 DOT * 0.9 collateral < 50 ETH borrow;
	#[test]
	fn borrow_with_insufficient_collateral_in_second_currency() {
		ExtBuilder::default()
			.user_balance(ADMIN, CurrencyId::DOT, ONE_HUNDRED)
			.user_balance(ADMIN, CurrencyId::ETH, ONE_HUNDRED)
			.user_balance(ALICE, CurrencyId::DOT, ONE_HUNDRED)
			.pool_user_data(CurrencyId::DOT, ALICE, BALANCE_ZERO, RATE_ZERO, true, 0)
			.build()
			.execute_with(|| {
				// Set initial balance
				assert_ok!(MinterestProtocol::deposit_underlying(
					Origin::signed(ADMIN),
					CurrencyId::DOT,
					ONE_HUNDRED
				));
				assert_ok!(MinterestProtocol::deposit_underlying(
					Origin::signed(ADMIN),
					CurrencyId::ETH,
					ONE_HUNDRED
				));
				// Alice deposit to DOT pool
				let alice_deposited_amount = 50_000 * DOLLARS;
				assert_ok!(MinterestProtocol::deposit_underlying(
					Origin::signed(ALICE),
					CurrencyId::DOT,
					alice_deposited_amount
				));

				System::set_block_number(2);

				// Alice try to borrow from ETH pool
				let alice_borrowed_amount = 50_000 * DOLLARS;
				assert_noop!(
					MinterestProtocol::borrow(Origin::signed(ALICE), CurrencyId::ETH, alice_borrowed_amount),
					MinterestProtocolError::<Test>::BorrowControllerRejection
				);

				// Checking pool available liquidity
				assert_eq!(
					TestPools::get_pool_available_liquidity(CurrencyId::DOT),
					ONE_HUNDRED + alice_deposited_amount
				);
				assert_eq!(TestPools::get_pool_available_liquidity(CurrencyId::ETH), ONE_HUNDRED);
			});
	}

	// Extrinsic `borrow`, description of scenario #4:
	// The user can borrow in the second currency if the collateral in the first currency
	// is sufficient.
	// Initial exchange rate for all assets equal 1.0;
	// Collateral factor for all assets equal 0.9;
	// 1. Alice deposit 50 DOT;
	// 2. Alice can borrow 40 ETH: 50 DOT * 0.9 collateral > 40 ETH borrow;
	#[test]
	fn borrow_with_sufficient_collateral_in_second_currency() {
		ExtBuilder::default()
			.user_balance(ADMIN, CurrencyId::DOT, ONE_HUNDRED)
			.user_balance(ADMIN, CurrencyId::ETH, ONE_HUNDRED)
			.user_balance(ALICE, CurrencyId::DOT, ONE_HUNDRED)
			.pool_user_data(CurrencyId::DOT, ALICE, BALANCE_ZERO, RATE_ZERO, true, 0)
			.build()
			.execute_with(|| {
				// Set initial balance
				assert_ok!(MinterestProtocol::deposit_underlying(
					Origin::signed(ADMIN),
					CurrencyId::DOT,
					ONE_HUNDRED
				));
				assert_ok!(MinterestProtocol::deposit_underlying(
					Origin::signed(ADMIN),
					CurrencyId::ETH,
					ONE_HUNDRED
				));
				// Alice deposit to DOT pool
				let alice_deposited_amount = 50_000 * DOLLARS;
				assert_ok!(MinterestProtocol::deposit_underlying(
					Origin::signed(ALICE),
					CurrencyId::DOT,
					alice_deposited_amount
				));

				System::set_block_number(2);

				// Alice try to borrow from ETH pool
				let alice_borrowed_amount = 40_000 * DOLLARS;
				assert_ok!(MinterestProtocol::borrow(
					Origin::signed(ALICE),
					CurrencyId::ETH,
					alice_borrowed_amount
				));

				// Checking pool available liquidity
				assert_eq!(
					TestPools::get_pool_available_liquidity(CurrencyId::DOT),
					ONE_HUNDRED + alice_deposited_amount
				);
				assert_eq!(
					TestPools::get_pool_available_liquidity(CurrencyId::ETH),
					ONE_HUNDRED - alice_borrowed_amount
				);
				assert_eq!(
					Currencies::free_balance(CurrencyId::DOT, &ALICE),
					ONE_HUNDRED - alice_deposited_amount
				);
				assert_eq!(Currencies::free_balance(CurrencyId::ETH, &ALICE), alice_borrowed_amount);
				assert_eq!(TestPools::pools(CurrencyId::ETH).total_borrowed, alice_borrowed_amount);
				assert_eq!(
					TestPools::pool_user_data(CurrencyId::ETH, &ALICE).total_borrowed,
					alice_borrowed_amount
				);
			});
	}

	// Extrinsic `borrow`, description of scenario #5:
	// User can borrow up to borrow cap
	// Initial exchange rate for all assets equal 1.0;
	// Collateral factor for all assets equal 0.9;
	// 1. Alice deposit 50 DOT;
	// 2. Bob deposit 50 DOT;
	// 3. Admin sets borrow cap to 30 (in usd);
	// 4. Alice borrows 10 ETH (20 usd);
	// 5. Bob is unable to borrow 10 ETH
	// 6. Admin disables borrow cap;
	// 7. Bob is able to borrow 10 ETH
	//
	#[test]
	fn borrow_with_borrow_cap() {
		ExtBuilder::default()
			.pool_initial(CurrencyId::DOT)
			.pool_initial(CurrencyId::ETH)
			.user_balance(ADMIN, CurrencyId::DOT, ONE_HUNDRED)
			.user_balance(ADMIN, CurrencyId::ETH, ONE_HUNDRED)
			.user_balance(ALICE, CurrencyId::DOT, ONE_HUNDRED)
			.user_balance(BOB, CurrencyId::DOT, ONE_HUNDRED)
			.pool_user_data(CurrencyId::DOT, ALICE, BALANCE_ZERO, RATE_ZERO, true, 0)
			.pool_user_data(CurrencyId::DOT, BOB, BALANCE_ZERO, RATE_ZERO, true, 0)
			.build()
			.execute_with(|| {
				// Set initial balance
				assert_ok!(MinterestProtocol::deposit_underlying(
					Origin::signed(ADMIN),
					CurrencyId::DOT,
					ONE_HUNDRED
				));
				assert_ok!(MinterestProtocol::deposit_underlying(
					Origin::signed(ADMIN),
					CurrencyId::ETH,
					ONE_HUNDRED
				));
				// Alice deposit to DOT pool
				let alice_deposited_amount = 50_000 * DOLLARS;
				assert_ok!(MinterestProtocol::deposit_underlying(
					Origin::signed(ALICE),
					CurrencyId::DOT,
					alice_deposited_amount
				));
				// Bob deposit to DOT pool
				let bob_deposited_amount = 50_000 * DOLLARS;
				assert_ok!(MinterestProtocol::deposit_underlying(
					Origin::signed(BOB),
					CurrencyId::DOT,
					bob_deposited_amount
				));

				System::set_block_number(2);

				// ADMIN set borrow cap to 30 (in usd).
				assert_ok!(TestController::set_borrow_cap_mode(
					Origin::signed(ADMIN),
					CurrencyId::ETH,
					true,
					Some(30_000 * DOLLARS)
				));

				System::set_block_number(3);

				// Alice borrow from DOT pool
				let alice_borrowed_amount_in_eth = 10_000 * DOLLARS;
				assert_ok!(MinterestProtocol::borrow(
					Origin::signed(ALICE),
					CurrencyId::ETH,
					alice_borrowed_amount_in_eth
				));

				System::set_block_number(4);

				// Bob is unable to borrow
				// borrow cap = 30
				// borrowed at the moment = 20
				let over_borrow_cap_amount_in_eth = 10_000 * DOLLARS;
				assert_noop!(
					MinterestProtocol::borrow(Origin::signed(BOB), CurrencyId::ETH, over_borrow_cap_amount_in_eth),
					MinterestProtocolError::<Test>::BorrowControllerRejection
				);

				// ADMIN disable borrow cap.
				assert_ok!(TestController::set_borrow_cap_mode(
					Origin::signed(ADMIN),
					CurrencyId::ETH,
					false,
					None
				));

				// Bob try to borrow from ETH pool
				assert_ok!(MinterestProtocol::borrow(
					Origin::signed(BOB),
					CurrencyId::ETH,
					over_borrow_cap_amount_in_eth
				));
			});
	}

	// Extrinsic `transfer_wrapped`, description of scenario #1:
	// The user tries to transfer all assets in the first currency. He has loan in the first
	// currency.
	// Initial exchange rate for all assets equal 1.0;
	// Collateral factor for all assets equal 0.9;
	// 1. Alice deposit 60 DOT;
	// 2. Alice deposit 50 ETH;
	// 3. Alice borrow 50 DOT;
	// 4. Alice can't `transfer_wrapped` all deposited MDOT: 50 ETH * 0.9 collateral < 50 DOT borrow;
	// 5. Alice deposit 10 ETH;
	// 6. Alice `transfer_wrapped` all deposited MDOT;
	// 7. Alice can't `transfer_wrapped` all deposited METH.
	#[test]
	fn transfer_wrapped_with_current_currency_borrowing() {
		ExtBuilder::default()
			.pool_initial(CurrencyId::DOT)
			.pool_initial(CurrencyId::ETH)
			.user_balance(ADMIN, CurrencyId::DOT, ONE_HUNDRED)
			.user_balance(ALICE, CurrencyId::DOT, ONE_HUNDRED)
			.user_balance(ALICE, CurrencyId::ETH, ONE_HUNDRED)
			.pool_user_data(CurrencyId::DOT, ALICE, BALANCE_ZERO, RATE_ZERO, true, 0)
			.pool_user_data(CurrencyId::ETH, ALICE, BALANCE_ZERO, RATE_ZERO, true, 0)
			.build()
			.execute_with(|| {
				assert_ok!(MinterestProtocol::deposit_underlying(
					Origin::signed(ADMIN),
					CurrencyId::DOT,
					ONE_HUNDRED
				));
				// Alice deposit 60 DOT to pool.
				let alice_deposited_amount_in_dot = 60_000 * DOLLARS;
				assert_ok!(MinterestProtocol::deposit_underlying(
					Origin::signed(ALICE),
					CurrencyId::DOT,
					alice_deposited_amount_in_dot
				));

				System::set_block_number(2);

				// Alice deposit 50 ETH to pool.
				let alice_deposited_amount_in_eth = 50_000 * DOLLARS;
				assert_ok!(MinterestProtocol::deposit_underlying(
					Origin::signed(ALICE),
					CurrencyId::ETH,
					alice_deposited_amount_in_eth
				));

				System::set_block_number(3);

				// Alice borrow from DOT pool
				let alice_borrowed_amount_in_dot = 50_000 * DOLLARS;
				assert_ok!(MinterestProtocol::borrow(
					Origin::signed(ALICE),
					CurrencyId::DOT,
					alice_borrowed_amount_in_dot
				));

				// Checking pool available liquidity
				assert_eq!(
					TestPools::get_pool_available_liquidity(CurrencyId::DOT),
					ONE_HUNDRED + alice_deposited_amount_in_dot - alice_borrowed_amount_in_dot
				);

				// Checking Alice's free balance DOT && MDOT.
				assert_eq!(
					Currencies::free_balance(CurrencyId::DOT, &ALICE),
					ONE_HUNDRED - alice_deposited_amount_in_dot + alice_borrowed_amount_in_dot
				);
				assert_eq!(
					Currencies::free_balance(CurrencyId::ETH, &ALICE),
					ONE_HUNDRED - alice_deposited_amount_in_eth
				);
				let expected_amount_wrapped_tokens_in_dot =
					TestPools::convert_to_wrapped(CurrencyId::DOT, alice_deposited_amount_in_dot).unwrap();
				assert_eq!(
					Currencies::free_balance(CurrencyId::MDOT, &ALICE),
					expected_amount_wrapped_tokens_in_dot
				);
				let expected_amount_wrapped_tokens_in_eth =
					TestPools::convert_to_wrapped(CurrencyId::ETH, alice_deposited_amount_in_eth).unwrap();
				assert_eq!(
					Currencies::free_balance(CurrencyId::METH, &ALICE),
					expected_amount_wrapped_tokens_in_eth
				);

				// Checking total borrow for Alice DOT pool
				assert_eq!(
					TestPools::pool_user_data(CurrencyId::DOT, ALICE).total_borrowed,
					alice_borrowed_amount_in_dot
				);
				// Checking total borrow for DOT pool
				assert_eq!(
					TestPools::pools(CurrencyId::DOT).total_borrowed,
					alice_borrowed_amount_in_dot
				);

				System::set_block_number(4);

				// Alice try to transfer all from DOT pool
				assert_noop!(
					MinterestProtocol::transfer_wrapped(
						Origin::signed(ALICE),
						BOB,
						CurrencyId::MDOT,
						expected_amount_wrapped_tokens_in_dot
					),
					MinterestProtocolError::<Test>::RedeemControllerRejection
				);

				System::set_block_number(5);

				// Alice add liquidity to ETH pool
				let alice_deposited_amount_in_eth_secondary = 10_000 * DOLLARS;
				assert_ok!(MinterestProtocol::deposit_underlying(
					Origin::signed(ALICE),
					CurrencyId::ETH,
					alice_deposited_amount_in_eth_secondary
				));

				System::set_block_number(6);

				assert_ok!(MinterestProtocol::transfer_wrapped(
					Origin::signed(ALICE),
					BOB,
					CurrencyId::MDOT,
					expected_amount_wrapped_tokens_in_dot
				));

				// Checking MDOT free balance for ALICE and BOB.
				assert_eq!(Currencies::free_balance(CurrencyId::MDOT, &ALICE), 0);
				assert_eq!(
					Currencies::free_balance(CurrencyId::MDOT, &BOB),
					expected_amount_wrapped_tokens_in_dot
				);

				// Checking ALICE ETH/METH balance
				assert_eq!(
					Currencies::free_balance(CurrencyId::ETH, &ALICE),
					ONE_HUNDRED - alice_deposited_amount_in_eth - alice_deposited_amount_in_eth_secondary
				);
				let expected_amount_wrapped_tokens_in_eth_summary = expected_amount_wrapped_tokens_in_eth
					+ TestPools::convert_to_wrapped(CurrencyId::ETH, alice_deposited_amount_in_eth_secondary).unwrap();
				assert_eq!(
					Currencies::free_balance(CurrencyId::METH, &ALICE),
					expected_amount_wrapped_tokens_in_eth_summary
				);
				// Checking total borrow for Alice DOT pool
				assert_eq!(
					TestPools::pool_user_data(CurrencyId::DOT, ALICE).total_borrowed,
					alice_borrowed_amount_in_dot
				);
				// Checking total borrow for DOT pool
				assert_eq!(
					TestPools::pools(CurrencyId::DOT).total_borrowed,
					alice_borrowed_amount_in_dot
				);

				System::set_block_number(7);

				// Alice try to transfer all from ETH pool
				assert_noop!(
					MinterestProtocol::transfer_wrapped(
						Origin::signed(ALICE),
						BOB,
						CurrencyId::METH,
						expected_amount_wrapped_tokens_in_eth_summary
					),
					MinterestProtocolError::<Test>::RedeemControllerRejection
				);
			});
	}

	// Extrinsic `transfer_wrapped`, description of scenario #2:
	// The user tries to transfer all assets in the first currency. He has loan in the second currency.
	// Initial exchange rate for all assets equal 1.0;
	// Collateral factor for all assets equal 0.9;
	// 1. Alice deposit 60 DOT;
	// 2. Alice borrow 50 ETH;
	// 3. Alice can't `transfer_wrapped` all deposited MDOT: 0 DOT collateral < 50 ETH borrow;
	#[test]
	fn transfer_wrapped_with_another_currency_borrowing() {
		ExtBuilder::default()
			.pool_initial(CurrencyId::DOT)
			.pool_initial(CurrencyId::ETH)
			.user_balance(ADMIN, CurrencyId::DOT, ONE_HUNDRED)
			.user_balance(ADMIN, CurrencyId::ETH, ONE_HUNDRED)
			.user_balance(ALICE, CurrencyId::DOT, ONE_HUNDRED)
			.pool_user_data(CurrencyId::DOT, ALICE, BALANCE_ZERO, RATE_ZERO, true, 0)
			.build()
			.execute_with(|| {
				assert_ok!(MinterestProtocol::deposit_underlying(
					Origin::signed(ADMIN),
					CurrencyId::DOT,
					ONE_HUNDRED
				));
				assert_ok!(MinterestProtocol::deposit_underlying(
					Origin::signed(ADMIN),
					CurrencyId::ETH,
					ONE_HUNDRED
				));
				// Alice deposit to DOT pool
				let alice_deposited_amount_in_dot = 60_000 * DOLLARS;
				assert_ok!(MinterestProtocol::deposit_underlying(
					Origin::signed(ALICE),
					CurrencyId::DOT,
					alice_deposited_amount_in_dot
				));

				System::set_block_number(2);

				// Alice borrow from ETH pool
				let alice_borrowed_amount_in_eth = 50_000 * DOLLARS;
				assert_ok!(MinterestProtocol::borrow(
					Origin::signed(ALICE),
					CurrencyId::ETH,
					alice_borrowed_amount_in_eth
				));

				// Checking free balance DOT/MDOT && ETH for user.
				let expected_amount_wrapped_tokens_in_dot =
					TestPools::convert_to_wrapped(CurrencyId::DOT, alice_deposited_amount_in_dot).unwrap();
				assert_eq!(
					Currencies::free_balance(CurrencyId::MDOT, &ALICE),
					expected_amount_wrapped_tokens_in_dot
				);
				assert_eq!(
					Currencies::free_balance(CurrencyId::DOT, &ALICE),
					ONE_HUNDRED - alice_deposited_amount_in_dot
				);
				assert_eq!(
					Currencies::free_balance(CurrencyId::ETH, &ALICE),
					alice_borrowed_amount_in_eth
				);
				// Checking total borrow for Alice ETH pool
				assert_eq!(
					TestPools::pool_user_data(CurrencyId::ETH, ALICE).total_borrowed,
					alice_borrowed_amount_in_eth
				);
				// // Checking total borrow for ETH pool
				assert_eq!(
					TestPools::pools(CurrencyId::ETH).total_borrowed,
					alice_borrowed_amount_in_eth
				);

				System::set_block_number(3);

				// Alice try to transfer all MDOTs
				assert_noop!(
					MinterestProtocol::transfer_wrapped(
						Origin::signed(ALICE),
						BOB,
						CurrencyId::MDOT,
						expected_amount_wrapped_tokens_in_dot
					),
					MinterestProtocolError::<Test>::RedeemControllerRejection
				);

				// Checking free balance DOT && ETH for user.
				// Expected previously values
				assert_eq!(
					Currencies::free_balance(CurrencyId::MDOT, &ALICE),
					expected_amount_wrapped_tokens_in_dot
				);
				assert_eq!(
					Currencies::free_balance(CurrencyId::DOT, &ALICE),
					ONE_HUNDRED - alice_deposited_amount_in_dot
				);
				assert_eq!(
					Currencies::free_balance(CurrencyId::ETH, &ALICE),
					alice_borrowed_amount_in_eth
				);

				// Checking total borrow for Alice ETH pool
				assert_eq!(
					TestPools::pool_user_data(CurrencyId::ETH, ALICE).total_borrowed,
					alice_borrowed_amount_in_eth
				);
				// Checking total borrow for ETH pool
				assert_eq!(
					TestPools::pools(CurrencyId::ETH).total_borrowed,
					alice_borrowed_amount_in_eth
				);
			});
	}

	// Extrinsic `transfer_wrapped`, description of scenario #3:
	// The user tries to transfer all assets in the first currency. He has loan in the second
	// currency and deposit in the third currency.
	// Initial exchange rate for all assets equal 1.0;
	// Collateral factor for all assets equal 0.9;
	// 1. Alice deposit 40 DOT;
	// 2. Alice deposit 40 BTC;
	// 3. Alice borrow 70 ETH;
	// 4. Alice can't `transfer_wrapped` 40 MDOT;
	// 5. Alice deposit 30 BTC;
	// 4. Alice can't `transfer_wrapped` 40 MDOT;
	// 6. Alice `transfer_wrapped` 30 MDOT;
	// 7. Alice can't `transfer_wrapped` 40 MBTC;
	#[test]
	fn transfer_wrapped_with_third_currency_borrowing() {
		ExtBuilder::default()
			.pool_initial(CurrencyId::DOT)
			.pool_initial(CurrencyId::ETH)
			.pool_initial(CurrencyId::BTC)
			.user_balance(ADMIN, CurrencyId::ETH, ONE_HUNDRED)
			.user_balance(ALICE, CurrencyId::DOT, ONE_HUNDRED)
			.user_balance(ALICE, CurrencyId::BTC, ONE_HUNDRED)
			.pool_user_data(CurrencyId::DOT, ALICE, BALANCE_ZERO, RATE_ZERO, true, 0)
			.pool_user_data(CurrencyId::BTC, ALICE, BALANCE_ZERO, RATE_ZERO, true, 0)
			.build()
			.execute_with(|| {
				assert_ok!(MinterestProtocol::deposit_underlying(
					Origin::signed(ADMIN),
					CurrencyId::ETH,
					ONE_HUNDRED
				));
				// Alice deposit to DOT pool
				let alice_deposited_amount_in_dot = 40_000 * DOLLARS;
				assert_ok!(MinterestProtocol::deposit_underlying(
					Origin::signed(ALICE),
					CurrencyId::DOT,
					alice_deposited_amount_in_dot
				));

				System::set_block_number(2);

				// Alice deposit to BTC pool
				let alice_deposited_amount_in_btc = 40_000 * DOLLARS;
				assert_ok!(MinterestProtocol::deposit_underlying(
					Origin::signed(ALICE),
					CurrencyId::BTC,
					alice_deposited_amount_in_btc
				));

				System::set_block_number(3);

				// Alice borrow from ETH pool
				let alice_borrowed_amount_in_eth = 70_000 * DOLLARS;
				assert_ok!(MinterestProtocol::borrow(
					Origin::signed(ALICE),
					CurrencyId::ETH,
					alice_borrowed_amount_in_eth
				));

				System::set_block_number(4);

				// Checking free balance DOT/MDOT && ETH && BTC for user.
				let expected_amount_wrapped_tokens_in_dot =
					TestPools::convert_to_wrapped(CurrencyId::DOT, alice_deposited_amount_in_dot).unwrap();
				assert_eq!(
					Currencies::free_balance(CurrencyId::MDOT, &ALICE),
					expected_amount_wrapped_tokens_in_dot
				);
				assert_eq!(
					Currencies::free_balance(CurrencyId::DOT, &ALICE),
					ONE_HUNDRED - alice_deposited_amount_in_dot
				);
				assert_eq!(
					Currencies::free_balance(CurrencyId::BTC, &ALICE),
					ONE_HUNDRED - alice_deposited_amount_in_btc
				);
				assert_eq!(
					Currencies::free_balance(CurrencyId::ETH, &ALICE),
					alice_borrowed_amount_in_eth
				);
				// Checking total borrow for Alice ETH pool
				assert_eq!(
					TestPools::pool_user_data(CurrencyId::ETH, ALICE).total_borrowed,
					alice_borrowed_amount_in_eth
				);
				// Checking total borrow for ETH pool
				assert_eq!(
					TestPools::pools(CurrencyId::ETH).total_borrowed,
					alice_borrowed_amount_in_eth
				);

				// Alice try to transfer all MDOTs
				assert_noop!(
					MinterestProtocol::transfer_wrapped(
						Origin::signed(ALICE),
						BOB,
						CurrencyId::MDOT,
						expected_amount_wrapped_tokens_in_dot
					),
					MinterestProtocolError::<Test>::RedeemControllerRejection
				);

				System::set_block_number(5);

				// Alice add liquidity to BTC pool
				let alice_deposited_amount_in_btc_secondary = 30_000 * DOLLARS;
				assert_ok!(MinterestProtocol::deposit_underlying(
					Origin::signed(ALICE),
					CurrencyId::BTC,
					alice_deposited_amount_in_btc_secondary
				));

				System::set_block_number(6);

				// Alice try to transfer all MDOTs
				assert_noop!(
					MinterestProtocol::transfer_wrapped(
						Origin::signed(ALICE),
						BOB,
						CurrencyId::MDOT,
						expected_amount_wrapped_tokens_in_dot
					),
					MinterestProtocolError::<Test>::RedeemControllerRejection
				);

				// Alice transfer 30 MDOTs
				let transfer_amount_in_m_dot = 30_000 * DOLLARS;
				assert_ok!(MinterestProtocol::transfer_wrapped(
					Origin::signed(ALICE),
					BOB,
					CurrencyId::MDOT,
					transfer_amount_in_m_dot
				));

				// Checking MDOT free balance for ALICE and BOB.
				assert_eq!(
					Currencies::free_balance(CurrencyId::MDOT, &ALICE),
					expected_amount_wrapped_tokens_in_dot - transfer_amount_in_m_dot
				);
				assert_eq!(
					Currencies::free_balance(CurrencyId::MDOT, &BOB),
					transfer_amount_in_m_dot
				);

				// Checking pool available liquidity.
				assert_eq!(
					TestPools::get_pool_available_liquidity(CurrencyId::DOT),
					alice_deposited_amount_in_dot
				);
				// Checking free balance DOT && ETH && BTC for user.
				assert_eq!(
					Currencies::free_balance(CurrencyId::DOT, &ALICE),
					ONE_HUNDRED - alice_deposited_amount_in_dot
				);
				assert_eq!(
					Currencies::free_balance(CurrencyId::BTC, &ALICE),
					ONE_HUNDRED - alice_deposited_amount_in_btc - alice_deposited_amount_in_btc_secondary
				);
				assert_eq!(
					Currencies::free_balance(CurrencyId::ETH, &ALICE),
					alice_borrowed_amount_in_eth
				);
				// Checking total borrow for Alice ETH pool
				assert_eq!(
					TestPools::pool_user_data(CurrencyId::ETH, ALICE).total_borrowed,
					alice_borrowed_amount_in_eth
				);
				// Checking total borrow for ETH pool
				assert_eq!(
					TestPools::pools(CurrencyId::ETH).total_borrowed,
					alice_borrowed_amount_in_eth
				);

				System::set_block_number(7);

				let total_alice_deposited_amount_in_btc =
					alice_deposited_amount_in_btc + alice_deposited_amount_in_btc_secondary;
				let expected_amount_wrapped_tokens_in_btc =
					TestPools::convert_to_wrapped(CurrencyId::BTC, total_alice_deposited_amount_in_btc).unwrap();
				// Alice try to transfer all MBTC.
				assert_noop!(
					MinterestProtocol::transfer_wrapped(
						Origin::signed(ALICE),
						BOB,
						CurrencyId::MBTC,
						expected_amount_wrapped_tokens_in_btc
					),
					MinterestProtocolError::<Test>::RedeemControllerRejection
				);
			});
	}
}
