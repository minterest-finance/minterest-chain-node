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
	// 1. Alice deposit() 100_000 DOT;
	// 2. Alice borrow() 50_000 ETH;
	// 3. Alice claim() [ETH];
	// 4. Alice claim() [DOT];
	// 5. Bob borrow() 20_000 DOT;
	// 6. Alice borrow() 30_000 BTC;
	// 7. Alice repay_all() ETH;
	// 8. Alice transfer_wrapped() to Bob 50_000 MDOT;
	// 9. Bob claim() [DOT];
	// 10. Alice repay_all() BTC;
	// 11. Alice redeem() 100_000 DOT;
	// 12. Alice claim() [DOT];
	// 13. Bob claim() [DOT];
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
				// Set initial state of pools for distribution MNT tokens.
				assert_ok!(MinterestProtocol::deposit_underlying(admin(), DOT, ONE_HUNDRED));
				assert_ok!(MinterestProtocol::deposit_underlying(admin(), ETH, ONE_HUNDRED));
				assert_ok!(MinterestProtocol::deposit_underlying(admin(), BTC, ONE_HUNDRED));
				assert_ok!(MinterestProtocol::enable_is_collateral(admin(), DOT));
				assert_ok!(MinterestProtocol::enable_is_collateral(admin(), ETH));
				assert_ok!(MinterestProtocol::enable_is_collateral(admin(), BTC));
				assert_ok!(MinterestProtocol::borrow(admin(), DOT, 50_000 * DOLLARS));
				assert_ok!(MinterestProtocol::borrow(admin(), ETH, 50_000 * DOLLARS));
				assert_ok!(MinterestProtocol::borrow(admin(), BTC, 50_000 * DOLLARS));

				set_block_number_and_refresh_speeds(10);

				// ALice deposit DOT and enable her DOT pool as collateral.
				assert_ok!(MinterestProtocol::deposit_underlying(alice(), DOT, ONE_HUNDRED));
				assert_ok!(MinterestProtocol::enable_is_collateral(alice(), DOT));

				set_block_number_and_refresh_speeds(20);

				assert_ok!(MinterestProtocol::borrow(alice(), ETH, 50_000 * DOLLARS));

				// Accrued MNT tokens are equal to zero, since distribution occurs only at
				// the moment of repeated user interaction with the protocol
				// (deposit, redeem, borrow, repay, transfer, claim).
				assert_eq!(TestMntToken::mnt_accrued(ALICE), Balance::zero());
				assert_eq!(Tokens::free_balance(MNT, &ALICE), Balance::zero());

				set_block_number_and_refresh_speeds(30);

				// There are borrow in all pool, but BTC pool excluded from MNT distribution.
				test_mnt_speeds(33_333_333_283_333_335, 66_666_666_716_666_664, 0);

				// BOB deposit ETH and enable his assets in pools as collateral.
				assert_ok!(MinterestProtocol::deposit_underlying(bob(), ETH, ONE_HUNDRED));
				assert_eq!(TestMntToken::mnt_accrued(ALICE), 0);
				assert_eq!(TestMntToken::mnt_accrued(BOB), 0);

				set_block_number_and_refresh_speeds(40);

				assert_ok!(MinterestProtocol::claim_mnt(alice(), vec![ETH]));
				assert_eq!(TestMntToken::mnt_accrued(ALICE), Balance::zero());
				assert_eq!(Currencies::free_balance(MNT, &ALICE), 583_333_303_583_252_543);

				assert_ok!(MinterestProtocol::claim_mnt(alice(), vec![DOT]));
				assert_eq!(Currencies::free_balance(MNT, &ALICE), 1_249_999_968_987_352_566);

				assert_ok!(MinterestProtocol::enable_is_collateral(bob(), ETH));

				set_block_number_and_refresh_speeds(50);

				assert_ok!(MinterestProtocol::borrow(bob(), DOT, 20_000 * DOLLARS));

				set_block_number_and_refresh_speeds(60);

				test_mnt_speeds(41_176_429_955_924_386, 58_823_570_044_075_613, 0);

				assert_ok!(MinterestProtocol::borrow(alice(), BTC, 30_000 * DOLLARS));

				set_block_number_and_refresh_speeds(70);

				// The BTC pool is excluded from the MNT-token distribution, so its speed is zero.
				test_mnt_speeds(41_176_429_955_924_386, 58_823_570_044_075_613, 0);
				assert_eq!(TestMntToken::mnt_accrued(ALICE), Balance::zero());
				assert_eq!(TestMntToken::mnt_accrued(BOB), Balance::zero());

				assert_ok!(TestMntToken::enable_mnt_minting(admin(), BTC));
				test_mnt_speeds(27_999_980_560_014_495, 40_000_039_329_970_918, 31_999_980_110_014_586);
				assert_eq!(TestMntToken::mnt_accrued(ALICE), Balance::zero());
				assert_eq!(TestMntToken::mnt_accrued(BOB), Balance::zero());

				assert_ok!(MinterestProtocol::repay_all(alice(), ETH));

				set_block_number_and_refresh_speeds(80);

				test_mnt_speeds(34_999_982_354_379_901, 25_000_034_903_116_419, 39_999_982_742_503_679);
				assert_eq!(TestMntToken::mnt_accrued(ALICE), 960_784_860_312_994_443);
				assert_eq!(TestMntToken::mnt_accrued(BOB), Balance::zero());

				assert_ok!(MinterestProtocol::transfer_wrapped(
					alice(),
					BOB,
					MDOT,
					50_000 * DOLLARS
				));
				assert_ok!(MinterestProtocol::claim_mnt(bob(), vec![DOT]));
				assert_eq!(Currencies::free_balance(MNT, &BOB), 292_884_845_268_385_804);

				set_block_number_and_refresh_speeds(90);

				test_mnt_speeds(34_999_982_354_379_901, 25_000_034_903_116_419, 39_999_982_742_503_679);
				assert_eq!(TestMntToken::mnt_accrued(ALICE), 1_639_999_855_537_001_155);
				assert_eq!(TestMntToken::mnt_accrued(BOB), Balance::zero());

				assert_ok!(MinterestProtocol::repay_all(alice(), BTC));

				set_block_number_and_refresh_speeds(100);

				test_mnt_speeds(41_176_394_739_223_584, 29_411_766_418_684_159, 29_411_838_842_092_256);
				assert_eq!(TestMntToken::mnt_accrued(ALICE), 1_909_999_671_430_830_035);
				assert_eq!(TestMntToken::mnt_accrued(BOB), Balance::zero());

				assert_ok!(MinterestProtocol::redeem(alice(), DOT));

				set_block_number_and_refresh_speeds(110);

				assert_eq!(TestMntToken::mnt_accrued(ALICE), 2_084_999_578_418_583_309);
				assert_eq!(TestMntToken::mnt_accrued(BOB), Balance::zero());

				assert_ok!(MinterestProtocol::claim_mnt(alice(), vec![DOT]));
				assert_eq!(Currencies::free_balance(MNT, &ALICE), 3_334_999_547_405_935_875);

				assert_ok!(MinterestProtocol::claim_mnt(bob(), vec![DOT]));
				assert_eq!(Currencies::free_balance(MNT, &BOB), 922_786_119_436_287_883);
			})
	}
}
