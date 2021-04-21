//  Integration-tests for mnt-token pallet.

#[cfg(test)]
mod tests {
	use crate::tests::*;

	fn set_block_number_and_refresh_speeds(n: u64) {
		System::set_block_number(n);
		assert_ok!(TestMntToken::refresh_mnt_speeds());
	}

	fn test_mnt_speeds(speed_dot: Balance, speed_eth: Balance, speed_btc: Balance) {
		assert_eq!(TestMntToken::mnt_speeds(DOT), speed_dot);
		assert_eq!(TestMntToken::mnt_speeds(ETH), speed_eth);
		assert_eq!(TestMntToken::mnt_speeds(BTC), speed_btc);

		let sum_of_speeds =
			TestMntToken::mnt_speeds(DOT) + TestMntToken::mnt_speeds(ETH) + TestMntToken::mnt_speeds(BTC);

		// This condition is necessary due to rounding in mathematical calculations.
		if sum_of_speeds % (DOLLARS / 10) == 0 {
			assert_eq!(sum_of_speeds, TestMntToken::mnt_rate());
		} else {
			assert_eq!(sum_of_speeds + 1, TestMntToken::mnt_rate());
		}
	}

	// This scenario works with two users and three pools.
	// The test checks the parameters of the MNT token.
	// Initial parameters: 	DOT + ETH - enabled in mnt minting;
	// 						mnt_rate = 0.1 MNT per block;
	#[test]
	fn test_mnt_token_scenario_n_1() {
		ExtBuilder::default()
			.pool_initial(DOT)
			.pool_initial(ETH)
			.pool_initial(BTC)
			.user_balance(ADMIN, DOT, ONE_HUNDRED)
			.user_balance(ADMIN, ETH, ONE_HUNDRED)
			.user_balance(ADMIN, BTC, ONE_HUNDRED)
			.user_balance(ALICE, DOT, ONE_HUNDRED)
			.user_balance(ALICE, ETH, ONE_HUNDRED)
			.user_balance(ALICE, BTC, ONE_HUNDRED)
			.user_balance(BOB, DOT, ONE_HUNDRED)
			.user_balance(BOB, ETH, ONE_HUNDRED)
			.user_balance(BOB, BTC, ONE_HUNDRED)
			.mnt_account_balance(ONE_HUNDRED)
			.build()
			.execute_with(|| {
				// Set initial balance
				assert_ok!(MinterestProtocol::deposit_underlying(admin(), DOT, ONE_HUNDRED));
				assert_ok!(MinterestProtocol::deposit_underlying(admin(), ETH, ONE_HUNDRED));
				assert_ok!(MinterestProtocol::deposit_underlying(admin(), BTC, ONE_HUNDRED));

				set_block_number_and_refresh_speeds(10);

				// ALice deposit DOT and enable her assets in pools as collateral.
				assert_ok!(MinterestProtocol::deposit_underlying(alice(), DOT, ONE_HUNDRED));
				assert_ok!(MinterestProtocol::enable_is_collateral(alice(), DOT));
				set_block_number_and_refresh_speeds(20);

				assert_ok!(MinterestProtocol::borrow(alice(), ETH, 50_000 * DOLLARS));
				set_block_number_and_refresh_speeds(30);
				// There are borrow only in the ETH pool, so its speed = mnt_rate = 0.1
				test_mnt_speeds(0, 100_000_000_000_000_000, 0);
				assert_eq!(TestMntToken::mnt_accrued(ALICE), 0);

				// BOB deposit ETH and enable her assets in pools as collateral.
				assert_ok!(MinterestProtocol::deposit_underlying(bob(), ETH, ONE_HUNDRED));
				assert_eq!(TestMntToken::mnt_accrued(ALICE), 0);
				assert_eq!(TestMntToken::mnt_accrued(BOB), 0);
				set_block_number_and_refresh_speeds(40);

				assert_ok!(MinterestProtocol::enable_is_collateral(bob(), ETH));
				set_block_number_and_refresh_speeds(50);

				assert_ok!(MinterestProtocol::borrow(bob(), DOT, 20_000 * DOLLARS));
				set_block_number_and_refresh_speeds(60);

				test_mnt_speeds(28_571_427_653_061_254, 71_428_572_346_938_746, 0);
				assert_eq!(TestMntToken::mnt_accrued(ALICE), 0);
				assert_eq!(TestMntToken::mnt_accrued(BOB), 0);

				assert_ok!(MinterestProtocol::borrow(alice(), BTC, 30_000 * DOLLARS));
				set_block_number_and_refresh_speeds(70);

				// The BTC pool is excluded from the MNT-token distribution, so its speed is zero.
				test_mnt_speeds(28_571_427_653_061_254, 71_428_572_346_938_746, 0);
				assert_eq!(TestMntToken::mnt_accrued(ALICE), 0);
				assert_eq!(TestMntToken::mnt_accrued(BOB), 0);

				assert_ok!(TestMntToken::enable_mnt_minting(admin(), BTC));
				// DOT ~ 0.2; ETH ~ 0.5; BTC ~ 0.3
				test_mnt_speeds(19_999_999_550_000_010, 50_000_001_124_999_974, 29_999_999_325_000_015);
				assert_eq!(TestMntToken::mnt_accrued(ALICE), 0);
				assert_eq!(TestMntToken::mnt_accrued(BOB), 0);

				assert_ok!(MinterestProtocol::repay_all(alice(), ETH));
				set_block_number_and_refresh_speeds(80);

				// DOT = 0.4; ETH = 0; BTC = 0.6
				test_mnt_speeds(40_000_000_000_000_000, 0, 60_000_000_000_000_000);
				assert_eq!(TestMntToken::mnt_accrued(ALICE), 3_714_285_723_469_350_000);
				assert_eq!(TestMntToken::mnt_accrued(BOB), 0);

				assert_ok!(MinterestProtocol::repay_all(alice(), BTC));
				set_block_number_and_refresh_speeds(90);

				test_mnt_speeds(100_000_000_000_000_000, 0, 0);
				assert_eq!(TestMntToken::mnt_accrued(ALICE), 4_014_285_716_719_350_000);
				assert_eq!(TestMntToken::mnt_accrued(BOB), 0);

				assert_ok!(MinterestProtocol::redeem(alice(), DOT));
				set_block_number_and_refresh_speeds(100);

				test_mnt_speeds(100_000_000_000_000_000, 0, 0);
				assert_eq!(TestMntToken::mnt_accrued(ALICE), 4_457_142_852_734_650_000);
				assert_eq!(TestMntToken::mnt_accrued(BOB), 0);
			})
	}
}
