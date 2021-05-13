///  Integration-tests for controller pallet.

#[cfg(test)]

mod tests {
	use crate::tests::*;

	// Description of the scenario:
	// The user cannot disable collateral for a specific asset, if he does not have enough
	// collateral in other assets to cover all his borrowing:
	//
	// Alice has 60 DOT collateral and 40 ETH collateral, and she has 50 BTC borrowing.
	// Exchange rate for all assets equal 1.0.
	// 1. Alice can't disable DOT as collateral (because 40 ETH won't cover 50 BTC borrowing);
	// 2. Alice can disable ETH as collateral (because 60 DOT will cover 50 BTC borrowing);
	#[test]
	fn disable_is_collateral_internal_fails_if_not_cover_borrowing() {
		ExtBuilder::default()
			.pool_initial(DOT)
			.pool_initial(BTC)
			.pool_initial(ETH)
			.user_balance(ALICE, DOT, ONE_HUNDRED)
			.user_balance(ALICE, BTC, ONE_HUNDRED)
			.user_balance(ALICE, ETH, ONE_HUNDRED)
			.pool_user_data(DOT, ALICE, BALANCE_ZERO, RATE_ZERO, false, 0)
			.pool_user_data(BTC, ALICE, BALANCE_ZERO, RATE_ZERO, false, 0)
			.pool_user_data(ETH, ALICE, BALANCE_ZERO, RATE_ZERO, false, 0)
			.build()
			.execute_with(|| {
				// ALICE deposit 60 DOT, 50 BTC, 40 ETH.
				assert_ok!(MinterestProtocol::deposit_underlying(alice(), DOT, 60 * DOLLARS));
				assert_ok!(MinterestProtocol::deposit_underlying(alice(), BTC, 50 * DOLLARS));
				assert_ok!(MinterestProtocol::deposit_underlying(alice(), ETH, 40 * DOLLARS));

				System::set_block_number(11);

				// Alice enable her assets in pools as collateral.
				assert_ok!(MinterestProtocol::enable_is_collateral(alice(), DOT));
				assert_ok!(MinterestProtocol::enable_is_collateral(alice(), BTC));
				assert_ok!(MinterestProtocol::enable_is_collateral(alice(), ETH));

				System::set_block_number(21);

				// Alice transfer her 50 MBTC to BOB.
				assert_ok!(Currencies::transfer(alice(), BOB, MBTC, 50 * DOLLARS));

				System::set_block_number(31);

				// Alice borrow 50 BTC.
				assert_ok!(MinterestProtocol::borrow(alice(), BTC, 50 * DOLLARS));

				System::set_block_number(41);

				// Alice can't disable DOT as collateral (because ETH won't cover the borrowing).
				assert_noop!(
					MinterestProtocol::disable_is_collateral(alice(), DOT),
					MinterestProtocolError::<Test>::IsCollateralCannotBeDisabled
				);

				System::set_block_number(51);

				// Alice can disable ETH as collateral (because DOT will cover the borrowing);
				assert_ok!(MinterestProtocol::disable_is_collateral(alice(), ETH));
			});
	}

	// Extrinsic `set_protocol_interest_factor`, description of scenario #2:
	// Pool interest is increased if the protocol_interest_factor is greater than zero.
	// 1. Alice deposit 40 DOT;
	// 2. Alice borrow 20 DOT;
	// 3. Set interest factor equal 0.5.
	// 4. Alice repay full loan in DOTs, pool interest increased.
	#[test]
	fn set_protocol_interest_factor_greater_than_zero() {
		ExtBuilder::default()
			.pool_initial(DOT)
			.pool_initial(ETH)
			.user_balance(ALICE, DOT, ONE_HUNDRED)
			.pool_user_data(DOT, ALICE, BALANCE_ZERO, RATE_ZERO, true, 0)
			.build()
			.execute_with(|| {
				// Alice deposit to DOT pool
				let alice_deposited_amount = 40_000 * DOLLARS;
				assert_ok!(MinterestProtocol::deposit_underlying(
					Origin::signed(ALICE),
					DOT,
					alice_deposited_amount
				));

				System::set_block_number(2);

				// Alice borrow from DOT pool
				let alice_borrowed_amount_in_dot = 20_000 * DOLLARS;
				assert_ok!(MinterestProtocol::borrow(
					Origin::signed(ALICE),
					DOT,
					alice_borrowed_amount_in_dot
				));

				// Checking pool available liquidity
				assert_eq!(
					TestPools::get_pool_available_liquidity(DOT),
					alice_deposited_amount - alice_borrowed_amount_in_dot
				);
				// Checking total interest for DOT pool.
				assert_eq!(TestPools::pools(DOT).total_protocol_interest, BALANCE_ZERO);

				System::set_block_number(10);

				// Set interest factor equal 0.5.
				assert_ok!(TestController::set_protocol_interest_factor(
					admin(),
					DOT,
					Rate::saturating_from_rational(1, 2)
				));

				// Alice repay full loan in DOTs.
				assert_ok!(MinterestProtocol::repay_all(Origin::signed(ALICE), DOT));

				let expected_interest_accumulated: Balance = 720_000_000_000_000;

				// Checking pool available liquidity
				assert_eq!(
					TestPools::get_pool_available_liquidity(DOT),
					alice_deposited_amount + expected_interest_accumulated
				);
				assert_eq!(
					TestPools::pools(DOT).total_protocol_interest,
					BALANCE_ZERO + (expected_interest_accumulated / 2)
				);
			});
	}

	// Extrinsic `set_protocol_interest_factor`, description of scenario #1:
	// Pool interest does not increase if the protocol_interest_factor is zero.
	// 1. Alice deposit 40 DOT;
	// 2. Alice borrow 20 DOT;
	// 3. Set interest factor equal to zero.
	// 4. Alice repay full loan in DOTs, pool total_protocol_interest = 0.
	#[test]
	fn set_protocol_interest_factor_equal_zero() {
		ExtBuilder::default()
			.pool_initial(DOT)
			.pool_initial(ETH)
			.user_balance(ALICE, DOT, ONE_HUNDRED)
			.pool_user_data(DOT, ALICE, BALANCE_ZERO, RATE_ZERO, true, 0)
			.build()
			.execute_with(|| {
				// Alice deposit to DOT pool
				let alice_deposited_amount = 40_000 * DOLLARS;
				assert_ok!(MinterestProtocol::deposit_underlying(
					Origin::signed(ALICE),
					DOT,
					alice_deposited_amount
				));

				System::set_block_number(2);

				// Alice borrow from DOT pool
				let alice_borrowed_amount_in_dot = 20_000 * DOLLARS;
				assert_ok!(MinterestProtocol::borrow(
					Origin::signed(ALICE),
					DOT,
					alice_borrowed_amount_in_dot
				));

				// Checking pool available liquidity
				assert_eq!(
					TestPools::get_pool_available_liquidity(DOT),
					alice_deposited_amount - alice_borrowed_amount_in_dot
				);
				// Checking total interest for DOT pool.
				assert_eq!(TestPools::pools(DOT).total_protocol_interest, BALANCE_ZERO);

				System::set_block_number(10);

				// Set interest factor equal to zero.
				assert_ok!(TestController::set_protocol_interest_factor(admin(), DOT, RATE_ZERO));

				// Alice repay full loan in DOTs.
				assert_ok!(MinterestProtocol::repay_all(Origin::signed(ALICE), DOT));

				// Checking pool total interest.
				assert_eq!(TestPools::pools(DOT).total_protocol_interest, BALANCE_ZERO);
			});
	}
}
