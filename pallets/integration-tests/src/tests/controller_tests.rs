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
	fn disable_collateral_internal_fails_if_not_cover_borrowing() {
		ExtBuilder::default()
			.pool_initial(CurrencyId::DOT)
			.pool_initial(CurrencyId::BTC)
			.pool_initial(CurrencyId::ETH)
			.user_balance(ALICE, CurrencyId::DOT, ONE_HUNDRED)
			.user_balance(ALICE, CurrencyId::BTC, ONE_HUNDRED)
			.user_balance(ALICE, CurrencyId::ETH, ONE_HUNDRED)
			.pool_user_data(CurrencyId::DOT, ALICE, BALANCE_ZERO, RATE_ZERO, false, 0)
			.pool_user_data(CurrencyId::BTC, ALICE, BALANCE_ZERO, RATE_ZERO, false, 0)
			.pool_user_data(CurrencyId::ETH, ALICE, BALANCE_ZERO, RATE_ZERO, false, 0)
			.build()
			.execute_with(|| {
				// ALICE deposit 60 DOT, 50 BTC, 40 ETH.
				assert_ok!(MinterestProtocol::deposit_underlying(
					alice(),
					CurrencyId::DOT,
					60 * DOLLARS
				));
				assert_ok!(MinterestProtocol::deposit_underlying(
					alice(),
					CurrencyId::BTC,
					50 * DOLLARS
				));
				assert_ok!(MinterestProtocol::deposit_underlying(
					alice(),
					CurrencyId::ETH,
					40 * DOLLARS
				));

				System::set_block_number(11);

				// Alice enable her assets in pools as collateral.
				assert_ok!(MinterestProtocol::enable_as_collateral(alice(), CurrencyId::DOT));
				assert_ok!(MinterestProtocol::enable_as_collateral(alice(), CurrencyId::BTC));
				assert_ok!(MinterestProtocol::enable_as_collateral(alice(), CurrencyId::ETH));

				System::set_block_number(21);

				// Alice transfer her 50 MBTC to BOB.
				assert_ok!(Currencies::transfer(alice(), BOB, CurrencyId::MBTC, 50 * DOLLARS));

				System::set_block_number(31);

				// Alice borrow 50 BTC.
				assert_ok!(MinterestProtocol::borrow(alice(), CurrencyId::BTC, 50 * DOLLARS));

				System::set_block_number(41);

				// Alice can't disable DOT as collateral (because ETH won't cover the borrowing).
				assert_noop!(
					MinterestProtocol::disable_collateral(alice(), CurrencyId::DOT),
					MinterestProtocolError::<Test>::CanotBeDisabledAsCollateral
				);

				System::set_block_number(51);

				// Alice can disable ETH as collateral (because DOT will cover the borrowing);
				assert_ok!(MinterestProtocol::disable_collateral(alice(), CurrencyId::ETH));
			});
	}

	// Extrinsic `set_insurance_factor`, description of scenario #2:
	// Pool insurance is increased if the insurance_factor is greater than zero.
	// 1. Alice deposit 40 DOT;
	// 2. Alice borrow 20 DOT;
	// 3. Set insurance factor equal 0.5.
	// 4. Alice repay full loan in DOTs, pool insurance increased.
	#[test]
	fn set_insurance_factor_greater_than_zero() {
		ExtBuilder::default()
			.pool_initial(CurrencyId::DOT)
			.user_balance(ALICE, CurrencyId::DOT, ONE_HUNDRED)
			.pool_user_data(CurrencyId::DOT, ALICE, BALANCE_ZERO, RATE_ZERO, true, 0)
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

				// Checking pool available liquidity
				assert_eq!(
					TestPools::get_pool_available_liquidity(CurrencyId::DOT),
					alice_deposited_amount - alice_borrowed_amount_in_dot
				);
				// Checking total insurance for DOT pool.
				assert_eq!(TestPools::pools(CurrencyId::DOT).total_insurance, BALANCE_ZERO);

				System::set_block_number(10);

				// Set insurance factor equal 0.5.
				assert_ok!(TestController::set_insurance_factor(
					admin(),
					CurrencyId::DOT,
					Rate::saturating_from_rational(1, 2)
				));

				// Alice repay full loan in DOTs.
				assert_ok!(MinterestProtocol::repay_all(Origin::signed(ALICE), CurrencyId::DOT));

				let expected_interest_accumulated: Balance = 720_000_000_000_000;

				// Checking pool available liquidity
				assert_eq!(
					TestPools::get_pool_available_liquidity(CurrencyId::DOT),
					alice_deposited_amount + expected_interest_accumulated
				);
				assert_eq!(
					TestPools::pools(CurrencyId::DOT).total_insurance,
					BALANCE_ZERO + (expected_interest_accumulated / 2)
				);
			});
	}

	// Extrinsic `set_insurance_factor`, description of scenario #1:
	// Pool insurance does not increase if the insurance_factor is zero.
	// 1. Alice deposit 40 DOT;
	// 2. Alice borrow 20 DOT;
	// 3. Set insurance factor equal to zero.
	// 4. Alice repay full loan in DOTs, pool total_insurance = 0.
	#[test]
	fn set_insurance_factor_equal_zero() {
		ExtBuilder::default()
			.pool_initial(CurrencyId::DOT)
			.user_balance(ALICE, CurrencyId::DOT, ONE_HUNDRED)
			.pool_user_data(CurrencyId::DOT, ALICE, BALANCE_ZERO, RATE_ZERO, true, 0)
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

				// Checking pool available liquidity
				assert_eq!(
					TestPools::get_pool_available_liquidity(CurrencyId::DOT),
					alice_deposited_amount - alice_borrowed_amount_in_dot
				);
				// Checking total insurance for DOT pool.
				assert_eq!(TestPools::pools(CurrencyId::DOT).total_insurance, BALANCE_ZERO);

				System::set_block_number(10);

				// Set insurance factor equal to zero.
				assert_ok!(TestController::set_insurance_factor(
					admin(),
					CurrencyId::DOT,
					RATE_ZERO
				));

				// Alice repay full loan in DOTs.
				assert_ok!(MinterestProtocol::repay_all(Origin::signed(ALICE), CurrencyId::DOT));

				// Checking pool total insurance.
				assert_eq!(TestPools::pools(CurrencyId::DOT).total_insurance, BALANCE_ZERO);
			});
	}
}
