//  Integration tests for mnt-token pallet.

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
			.mnt_enabled_pools(vec![DOT, ETH])
			.mnt_account_balance(ONE_HUNDRED)
			.build()
			.execute_with(|| {
				assert!(mnt_token::MntSpeeds::<Test>::contains_key(DOT));
				assert!(mnt_token::MntSpeeds::<Test>::contains_key(ETH));
				assert!(!mnt_token::MntSpeeds::<Test>::contains_key(BTC));
				assert!(!mnt_token::MntSpeeds::<Test>::contains_key(KSM));
				// Set initial state of pools for distribution MNT tokens.
				vec![DOT, ETH, BTC].into_iter().for_each(|pool_id| {
					assert_ok!(MinterestProtocol::deposit_underlying(admin(), pool_id, ONE_HUNDRED));
					assert_ok!(MinterestProtocol::enable_is_collateral(admin(), pool_id));
					assert_ok!(MinterestProtocol::borrow(admin(), pool_id, 50_000 * DOLLARS));
				});

				set_block_number_and_refresh_speeds(10);

				// ALice deposits DOT and enables her DOT pool as a collateral.
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

				// BOB deposits ETH.
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

				// The BTC pool is excluded from the MNT token distribution, so its speed is zero.
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

	// This scenario works with one user and three pools.
	// The test checks the parameters of the MNT token when new pool is created.
	// Initial parameters: 	DOT + ETH - enabled in mnt minting;
	// 						mnt_rate = 0.1 MNT per block;
	// 1. Alice deposit() 100_000 DOT;
	// 2. Alice borrow() 50_000 ETH;
	// 3. Init BTC pool;
	// 4. Alice borrow() 30_000 BTC;
	// 5. Enable MNT distribution for BTC pool;
	// 6. Alice repay_all() ETH;
	// 7. Alice claim() [DOT, ETH, BTC];
	#[test]
	fn test_mnt_token_scenario_n_2() {
		ExtBuilder::default()
			.set_controller_data(vec![
				(
					DOT,
					ControllerData {
						last_interest_accrued_block: 0,
						protocol_interest_factor: Rate::saturating_from_rational(1, 10),
						max_borrow_rate: Rate::saturating_from_rational(5, 1000),
						collateral_factor: Rate::saturating_from_rational(9, 10), // 90%
						borrow_cap: None,
						protocol_interest_threshold: PROTOCOL_INTEREST_TRANSFER_THRESHOLD,
					},
				),
				(
					ETH,
					ControllerData {
						last_interest_accrued_block: 0,
						protocol_interest_factor: Rate::saturating_from_rational(1, 10),
						max_borrow_rate: Rate::saturating_from_rational(5, 1000),
						collateral_factor: Rate::saturating_from_rational(9, 10), // 90%
						borrow_cap: None,
						protocol_interest_threshold: PROTOCOL_INTEREST_TRANSFER_THRESHOLD,
					},
				),
			])
			.set_minterest_model_params(vec![
				(
					DOT,
					MinterestModelData {
						kink: Rate::saturating_from_rational(8, 10),
						base_rate_per_block: Rate::zero(),
						multiplier_per_block: Rate::saturating_from_rational(9, 1_000_000_000), // 0.047304 PerYear
						jump_multiplier_per_block: Rate::saturating_from_rational(207, 1_000_000_000), // 1.09 PerYear
					},
				),
				(
					ETH,
					MinterestModelData {
						kink: Rate::saturating_from_rational(8, 10),
						base_rate_per_block: Rate::zero(),
						multiplier_per_block: Rate::saturating_from_rational(9, 1_000_000_000), // 0.047304 PerYear
						jump_multiplier_per_block: Rate::saturating_from_rational(207, 1_000_000_000), // 1.09 PerYear
					},
				),
			])
			.pool_initial(DOT)
			.pool_initial(ETH)
			.user_balance(ADMIN, DOT, ONE_HUNDRED)
			.user_balance(ADMIN, ETH, ONE_HUNDRED)
			.user_balance(ADMIN, BTC, ONE_HUNDRED)
			.user_balance(ALICE, DOT, ONE_HUNDRED)
			.user_balance(ALICE, ETH, ONE_HUNDRED)
			.user_balance(ALICE, BTC, ONE_HUNDRED)
			.user_balance(BOB, DOT, ONE_HUNDRED)
			.user_balance(BOB, ETH, ONE_HUNDRED)
			.user_balance(BOB, BTC, ONE_HUNDRED)
			.mnt_enabled_pools(vec![DOT, ETH])
			.mnt_account_balance(ONE_HUNDRED)
			.build()
			.execute_with(|| {
				assert!(mnt_token::MntSpeeds::<Test>::contains_key(DOT));
				assert!(mnt_token::MntSpeeds::<Test>::contains_key(ETH));
				assert!(!mnt_token::MntSpeeds::<Test>::contains_key(BTC));
				assert!(!mnt_token::MntSpeeds::<Test>::contains_key(KSM));
				// Set initial state of pools for distribution MNT tokens.
				vec![DOT, ETH].into_iter().for_each(|pool_id| {
					assert_ok!(MinterestProtocol::deposit_underlying(bob(), pool_id, ONE_HUNDRED));
					assert_ok!(MinterestProtocol::enable_is_collateral(bob(), pool_id));
					assert_ok!(MinterestProtocol::borrow(bob(), pool_id, 50_000 * DOLLARS));
				});

				set_block_number_and_refresh_speeds(10);

				// ALice deposits DOT and enables her DOT pool as a collateral.
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

				// Init BTC pool
				assert_ok!(MinterestProtocol::create_pool(
					admin(),
					BTC,
					PoolInitData {
						kink: Rate::saturating_from_rational(8, 10),
						base_rate_per_block: Rate::zero(),
						multiplier_per_block: Rate::saturating_from_rational(9, 1_000_000_000),
						jump_multiplier_per_block: Rate::saturating_from_rational(207, 1_000_000_000),
						protocol_interest_factor: Rate::saturating_from_rational(1, 10),
						max_borrow_rate: Rate::saturating_from_rational(5, 1000),
						collateral_factor: Rate::saturating_from_rational(9, 10),
						protocol_interest_threshold: PROTOCOL_INTEREST_TRANSFER_THRESHOLD,
						deviation_threshold: Rate::saturating_from_rational(1, 10),
						balance_ratio: Rate::saturating_from_rational(2, 10),
						max_attempts: 3,
						min_partial_liquidation_sum: 100 * DOLLARS,
						threshold: Rate::saturating_from_rational(103, 100),
						liquidation_fee: Rate::saturating_from_rational(105, 100),
					},
				));
				assert_ok!(MinterestProtocol::deposit_underlying(admin(), BTC, ONE_HUNDRED));
				assert_ok!(MinterestProtocol::enable_is_collateral(admin(), BTC));
				assert_ok!(MinterestProtocol::borrow(admin(), BTC, 50_000 * DOLLARS));

				assert_ok!(MinterestProtocol::borrow(alice(), BTC, 30_000 * DOLLARS));

				set_block_number_and_refresh_speeds(70);

				// The BTC pool is excluded from the MNT token distribution, so its speed is zero.
				test_mnt_speeds(33_333_333_283_333_335, 66_666_666_716_666_664, 0);
				assert_eq!(TestMntToken::mnt_accrued(ALICE), Balance::zero());

				assert_ok!(TestMntToken::enable_mnt_minting(admin(), BTC));
				test_mnt_speeds(21_739_130_719_754_245, 43_478_261_537_334_575, 34_782_607_742_911_179);
				assert_eq!(TestMntToken::mnt_accrued(ALICE), Balance::zero());

				assert_ok!(MinterestProtocol::repay_all(alice(), ETH));

				set_block_number_and_refresh_speeds(80);

				test_mnt_speeds(27_777_711_264_044_877, 27_777_952_513_478_936, 44_444_336_222_476_186);
				assert_eq!(TestMntToken::mnt_accrued(ALICE), 1_583_333_261_583_256_134);

				// Alice is able to claim rewards from all three pools
				assert_ok!(MinterestProtocol::claim_mnt(alice(), vec![DOT, ETH]));
				assert_eq!(Currencies::free_balance(MNT, &ALICE), 2_858_695_574_289_277_984);
				assert_ok!(MinterestProtocol::claim_mnt(alice(), vec![BTC]));
				assert_eq!(Currencies::free_balance(MNT, &ALICE), 2_989_130_353_325_167_984);
			})
	}

	// Test MNT distribution behaviour when users are using transfer_wrapped
	#[test]
	fn mnt_token_supplier_distribution_when_users_transferring_tokens() {
		ExtBuilder::default()
			.pool_initial(DOT)
			.pool_initial(ETH)
			.user_balance(ADMIN, DOT, ONE_HUNDRED)
			.user_balance(ALICE, DOT, ONE_HUNDRED)
			.user_balance(BOB, DOT, ONE_HUNDRED)
			.user_balance(CAROL, DOT, 2 * ONE_HUNDRED)
			.mnt_enabled_pools(vec![DOT, ETH])
			.mnt_account_balance(ONE_HUNDRED)
			.build()
			.execute_with(|| {
				// Set initial state of pools for distribution MNT tokens.
				assert_ok!(MinterestProtocol::deposit_underlying(admin(), DOT, ONE_HUNDRED));
				assert_ok!(MinterestProtocol::enable_is_collateral(admin(), DOT));
				assert_ok!(MinterestProtocol::borrow(admin(), DOT, 50_000 * DOLLARS));

				set_block_number_and_refresh_speeds(10);

				// Alice, Bob and Carol deposit DOT.
				assert_ok!(MinterestProtocol::deposit_underlying(alice(), DOT, ONE_HUNDRED));
				assert_ok!(MinterestProtocol::deposit_underlying(bob(), DOT, ONE_HUNDRED));
				assert_ok!(MinterestProtocol::deposit_underlying(carol(), DOT, 2 * ONE_HUNDRED));

				set_block_number_and_refresh_speeds(20);

				// Check that both Alice and Bob receive the same amount of MNT token since they
				// have equal DOT balance
				let mnt_balance_after_deposit = 199_999_999_270_900_013;
				assert_ok!(MinterestProtocol::claim_mnt(alice(), vec![DOT]));
				assert_ok!(MinterestProtocol::claim_mnt(bob(), vec![DOT]));
				assert_eq!(Currencies::free_balance(MNT, &ALICE), mnt_balance_after_deposit);
				assert_eq!(Currencies::free_balance(MNT, &BOB), mnt_balance_after_deposit);
				assert_eq!(Currencies::free_balance(MNT, &CAROL), BALANCE_ZERO);

				// Alice transfers all to Bob
				assert_ok!(MinterestProtocol::transfer_wrapped(
					alice(),
					BOB,
					MDOT,
					Currencies::free_balance(MDOT, &ALICE)
				));

				set_block_number_and_refresh_speeds(30);

				// Check that Alice received 0 MNT and Bob received approximately x2 comparing to
				// previous claim
				let mnt_bob_balance_after_transfer = mnt_balance_after_deposit + 399_999_998_541_800_026;
				assert_ok!(MinterestProtocol::claim_mnt(alice(), vec![DOT]));
				assert_ok!(MinterestProtocol::claim_mnt(bob(), vec![DOT]));
				assert_eq!(Currencies::free_balance(MNT, &ALICE), mnt_balance_after_deposit);
				assert_eq!(Currencies::free_balance(MNT, &BOB), mnt_bob_balance_after_transfer);

				// Bob transfers one third of its balance to Alice
				assert_ok!(MinterestProtocol::transfer_wrapped(
					bob(),
					ALICE,
					MDOT,
					Currencies::free_balance(MDOT, &BOB) / 3
				));

				set_block_number_and_refresh_speeds(40);

				// Test proportions 1:2. Amount of tokens Bob receive after claim must be twice
				// bigger comparing to claim amount for Alice.
				let mnt_alice_delta_after_second_transfer = 133_333_332_847_266_675;
				let mnt_alice_balance_after_second_transfer =
					mnt_balance_after_deposit + mnt_alice_delta_after_second_transfer;
				let mnt_bob_balance_after_second_transfer = mnt_bob_balance_after_transfer +
					mnt_alice_delta_after_second_transfer * 2 +
					/*calculation error, it is okay for such algorithms*/ 1;
				assert_ok!(MinterestProtocol::claim_mnt(alice(), vec![DOT]));
				assert_ok!(MinterestProtocol::claim_mnt(bob(), vec![DOT]));
				assert_eq!(
					Currencies::free_balance(MNT, &ALICE),
					mnt_alice_balance_after_second_transfer
				);
				assert_eq!(
					Currencies::free_balance(MNT, &BOB),
					mnt_bob_balance_after_second_transfer
				);

				// Make random transfers
				assert_ok!(MinterestProtocol::transfer_wrapped(
					alice(),
					BOB,
					MDOT,
					Currencies::free_balance(MDOT, &ALICE) / 2
				));
				assert_ok!(MinterestProtocol::transfer_wrapped(
					bob(),
					ALICE,
					MDOT,
					Currencies::free_balance(MDOT, &BOB) / 2
				));
				assert_ok!(MinterestProtocol::transfer_wrapped(
					alice(),
					BOB,
					MDOT,
					Currencies::free_balance(MDOT, &ALICE)
				));
				// Return the same proportions (1:2) eventually
				assert_ok!(MinterestProtocol::transfer_wrapped(
					bob(),
					ALICE,
					MDOT,
					Currencies::free_balance(MDOT, &BOB) / 3
				));

				set_block_number_and_refresh_speeds(100);

				// Test proportions 1:2 one more time. Transfers within one block doesn't affect
				// calculations
				let mnt_alice_delta_after_third_transfer = 799_999_997_083_933_386;
				let mnt_alice_balance_after_third_transfer =
					mnt_alice_balance_after_second_transfer + mnt_alice_delta_after_third_transfer;
				let mnt_bob_balance_after_third_transfer =
					mnt_bob_balance_after_second_transfer + mnt_alice_delta_after_third_transfer * 2;
				assert_ok!(MinterestProtocol::claim_mnt(alice(), vec![DOT]));
				assert_ok!(MinterestProtocol::claim_mnt(bob(), vec![DOT]));
				assert_eq!(
					Currencies::free_balance(MNT, &ALICE),
					mnt_alice_balance_after_third_transfer
				);
				assert_eq!(
					Currencies::free_balance(MNT, &BOB),
					mnt_bob_balance_after_third_transfer
				);

				assert_ok!(MinterestProtocol::claim_mnt(carol(), vec![DOT]));
				assert_eq!(
					Currencies::free_balance(MNT, &CAROL),
					mnt_alice_balance_after_third_transfer +
						mnt_bob_balance_after_third_transfer +
						/*calculation error, it is okay for such algorithms*/ 3
				);
			});
	}
}
