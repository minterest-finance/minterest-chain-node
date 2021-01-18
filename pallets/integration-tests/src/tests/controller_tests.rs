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
			.pool_user_data(ALICE, CurrencyId::DOT, BALANCE_ZERO, RATE_ZERO, false)
			.pool_user_data(ALICE, CurrencyId::BTC, BALANCE_ZERO, RATE_ZERO, false)
			.pool_user_data(ALICE, CurrencyId::ETH, BALANCE_ZERO, RATE_ZERO, false)
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
}
