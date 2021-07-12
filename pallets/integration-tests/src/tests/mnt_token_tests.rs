//  Integration tests for mnt-token pallet.

#[cfg(test)]
mod tests {
	use crate::tests::*;

	fn test_mnt_speeds(speed_dot: Balance, speed_eth: Balance, speed_btc: Balance) {
		assert_eq!(TestMntToken::mnt_speeds(DOT), speed_dot);
		assert_eq!(TestMntToken::mnt_speeds(ETH), speed_eth);
		assert_eq!(TestMntToken::mnt_speeds(BTC), speed_btc);
	}

	// This scenario works with two users and three pools.
	// The test checks the parameters of the MNT token.
	// Initial parameters: 	DOT + ETH - enabled in mnt minting;
	// 						mnt_speed = 0.1 MNT per block;
	// 1. Alice deposit() 100_000 DOT;
	// 2. Alice borrow() 50_000 ETH;
	// 3. Bob deposit() 100_000 ETH;
	// 4. Alice claim() [ETH];
	// 5. Alice claim() [DOT];
	// 6. Bob borrow() 20_000 DOT;
	// 7. Alice borrow() 30_000 BTC;
	// 8. Alice repay_all() ETH;
	// 9. Alice transfer_wrapped() to Bob 50_000 MDOT;
	// 10. Bob claim() [DOT];
	// 11. Alice repay_all() BTC;
	// 12. Alice redeem() 100_000 DOT;
	// 13. Alice claim() [DOT];
	// 14. Bob claim() [DOT];
	#[test]
	fn test_mnt_token_scenario_n_1() {
		ExtBuilder::default()
			.pool_initial(DOT)
			.pool_initial(ETH)
			.pool_initial(BTC)
			.user_balance(ADMIN, DOT, ONE_HUNDRED_THOUSAND)
			.user_balance(ADMIN, ETH, ONE_HUNDRED_THOUSAND)
			.user_balance(ADMIN, BTC, ONE_HUNDRED_THOUSAND)
			.user_balance(ALICE, DOT, ONE_HUNDRED_THOUSAND)
			.user_balance(ALICE, ETH, ONE_HUNDRED_THOUSAND)
			.user_balance(ALICE, BTC, ONE_HUNDRED_THOUSAND)
			.user_balance(BOB, DOT, ONE_HUNDRED_THOUSAND)
			.user_balance(BOB, ETH, ONE_HUNDRED_THOUSAND)
			.user_balance(BOB, BTC, ONE_HUNDRED_THOUSAND)
			.mnt_enabled_pools(vec![(DOT, DOLLARS / 10), (ETH, DOLLARS / 10)])
			.mnt_account_balance(ONE_HUNDRED_THOUSAND)
			.mnt_claim_threshold(dollars(100))
			.build()
			.execute_with(|| {
				assert!(mnt_token::MntSpeeds::<Test>::contains_key(DOT));
				assert!(mnt_token::MntSpeeds::<Test>::contains_key(ETH));
				assert!(!mnt_token::MntSpeeds::<Test>::contains_key(BTC));
				assert!(!mnt_token::MntSpeeds::<Test>::contains_key(KSM));
				// Set initial state of pools for distribution MNT tokens.
				vec![DOT, ETH, BTC].into_iter().for_each(|pool_id| {
					assert_ok!(MinterestProtocol::deposit_underlying(
						admin_origin(),
						pool_id,
						ONE_HUNDRED_THOUSAND
					));
					assert_ok!(MinterestProtocol::enable_is_collateral(admin_origin(), pool_id));
					assert_ok!(MinterestProtocol::borrow(admin_origin(), pool_id, 50_000 * DOLLARS));
				});

				System::set_block_number(10);

				// ALice deposits DOT and enables her DOT pool as a collateral.
				// At this moment Alice starts receiving (dot_speed / 2) MNT per block as a supplier
				assert_ok!(MinterestProtocol::deposit_underlying(
					alice_origin(),
					DOT,
					ONE_HUNDRED_THOUSAND
				));
				assert_ok!(MinterestProtocol::enable_is_collateral(alice_origin(), DOT));

				System::set_block_number(20);

				// At this moment Alice starts receiving (eth_speed / 2) MNT per block as a borrower
				assert_ok!(MinterestProtocol::borrow(alice_origin(), ETH, 50_000 * DOLLARS));

				// Accrued MNT tokens are equal to zero, since distribution occurs only at
				// the moment of repeated user interaction with the protocol
				// (deposit, redeem, borrow, repay, transfer, claim).
				assert_eq!(TestMntToken::mnt_accrued(ALICE), Balance::zero());
				assert_eq!(Tokens::free_balance(MNT, &ALICE), Balance::zero());

				// BOB deposits ETH.
				// At this moment Alice and Bob start receiving (eth_speed / 3) MNT per block as a suppliers
				assert_ok!(MinterestProtocol::deposit_underlying(
					bob_origin(),
					ETH,
					ONE_HUNDRED_THOUSAND
				));
				assert_eq!(TestMntToken::mnt_accrued(ALICE), 0);
				assert_eq!(TestMntToken::mnt_accrued(BOB), 0);

				System::set_block_number(30);

				// Alice started taking part in ETH pool distribution at block 20 as a borrower
				// mnt_balance = 0.1(eth_speed) * 10(delta_blocks) * 50(borrowed) / 100(total_borrow) = 0.5 MNT
				assert_ok!(MinterestProtocol::claim_mnt(alice_origin(), vec![ETH]));
				assert_eq!(TestMntToken::mnt_accrued(ALICE), Balance::zero());
				assert_eq!(Currencies::free_balance(MNT, &ALICE), 499_999_978_624_951_827);

				// Alice started taking part in DOT pool distribution at block 10 as a supplier
				// mnt_balance = 0.5 MNT + 0.1(dot_speed) * 20(delta_blocks) * 100(supply) / 200(total_supply) = 1.5
				// MNT
				assert_ok!(MinterestProtocol::claim_mnt(alice_origin(), vec![DOT]));
				assert_eq!(Currencies::free_balance(MNT, &ALICE), 1_499_999_969_512_351_993);

				assert_ok!(MinterestProtocol::enable_is_collateral(bob_origin(), ETH));

				System::set_block_number(40);

				// At this moment Bob starts receiving (dot_speed * 2/7) MNT per block as a borrower
				assert_ok!(MinterestProtocol::borrow(bob_origin(), DOT, 20_000 * DOLLARS));
				// At this moment Alice starts receiving (btc_speed * 3/8) MNT per block as a borrower
				assert_ok!(MinterestProtocol::borrow(alice_origin(), BTC, 30_000 * DOLLARS));
				assert_eq!(TestMntToken::mnt_accrued(ALICE), Balance::zero());
				assert_eq!(TestMntToken::mnt_accrued(BOB), Balance::zero());

				assert_ok!(TestMntToken::set_speed(admin_origin(), BTC, 2 * DOLLARS));
				test_mnt_speeds(
					100_000_000_000_000_000,
					100_000_000_000_000_000,
					2_000_000_000_000_000_000,
				);
				assert_eq!(TestMntToken::mnt_accrued(ALICE), Balance::zero());
				assert_eq!(TestMntToken::mnt_accrued(BOB), Balance::zero());

				// At this point Alice stops being a borrower, but still has unclaimed tokens for 10 blocks since
				// the last ETH claim mnt_accrued = 0.1(eth_speed) * 10(delta_blocks) * 50(borrowed) /
				// 100(total_borrow) = 0.5 MNT
				assert_ok!(MinterestProtocol::repay_all(alice_origin(), ETH));
				assert_eq!(TestMntToken::mnt_accrued(ALICE), 499_999_978_624_951_827);
				assert_eq!(TestMntToken::mnt_accrued(BOB), Balance::zero());

				// At this point Alice and Bob start receiving rewards as a suppliers -
				// (dot_speed * 0.25) per block each
				assert_ok!(MinterestProtocol::transfer_wrapped(
					alice_origin(),
					BOB,
					MDOT,
					50_000 * DOLLARS
				));
				// Alice should receive tokens as a supplier for a 10 blocks since the last claim
				// mnt_accrued = 0.5 MNT + 0.1(dot_speed) * 10(delta_blocks) * 100(supply) / 200(total supply) = 1
				// MNT
				assert_eq!(TestMntToken::mnt_accrued(ALICE), 999_999_974_068_651_910);

				System::set_block_number(50);

				// mnt_balance =
				//   (BORROW)  0.1(dot_speed) * 10(delta_blocks) * 20(borrowed) / 70(total_borrow) +
				//   (SUPPLY)  0.1(dot_speed) * 10(delta_blocks) * 50(supply) / 200(total supply) =
				// 0.285714286 + 0.25 = 0.535714286
				assert_ok!(MinterestProtocol::claim_mnt(bob_origin(), vec![DOT]));
				assert_eq!(Currencies::free_balance(MNT, &BOB), 535_714_265_951_558_142);
				assert_eq!(TestMntToken::mnt_accrued(BOB), Balance::zero());

				// Alice started taking part in BTC pool distribution at block 40 as a borrower
				// mnt_accrued = 1 MNT + 2(btc_speed) * 10(delta_blocks) * 30(borrowed) / 80(total_borrow) = 8.5 MNT
				assert_ok!(MinterestProtocol::repay_all(alice_origin(), BTC));
				assert_eq!(TestMntToken::mnt_accrued(ALICE), 8_499_999_151_412_486_286);
				assert_eq!(TestMntToken::mnt_accrued(BOB), Balance::zero());

				// Alice stops being a supplier but still has unclaimed tokens for 10 blocks since the last action
				// on DOT pool mnt_accrued = 8.5 MNT + 0.1(dot_speed) * 10(delta_blocks) * 50(supply) / 200(total
				// supply) = 8.75 MNT
				assert_ok!(MinterestProtocol::redeem(alice_origin(), DOT));

				System::set_block_number(100);

				assert_eq!(TestMntToken::mnt_accrued(ALICE), 8_749_999_144_578_086_369);
				assert_eq!(TestMntToken::mnt_accrued(BOB), Balance::zero());

				// mnt_balance = 1.5 (already claimed) + 8.75 (accrued) = 10.25 MNT
				assert_ok!(MinterestProtocol::claim_mnt(alice_origin(), vec![DOT]));
				assert_eq!(Currencies::free_balance(MNT, &ALICE), 10_249_999_114_090_438_362);

				// mnt_balance = 0.535714286 (already claimed) +
				//   (BORROW)  0.1(dot_speed) * 50(delta_blocks) * 20(borrowed) / 70(total_borrow) +
				//   (SUPPLY)  0.1(dot_speed) * 50(delta_blocks) * 50(supply) / 150(total supply) =
				// 0.535714286 + 1.666666667 + 1.428571429 = 3.630952382
				assert_ok!(MinterestProtocol::claim_mnt(bob_origin(), vec![DOT]));
				assert_eq!(Currencies::free_balance(MNT, &BOB), 3_630_952_250_985_538_854);
			})
	}

	// This scenario works with one user and three pools.
	// The test checks the parameters of the MNT token when new pool is created.
	// Initial parameters: 	DOT + ETH - enabled in mnt minting;
	// 						mnt_speed = 0.1 MNT per block;
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
			.user_balance(ADMIN, DOT, ONE_HUNDRED_THOUSAND)
			.user_balance(ADMIN, ETH, ONE_HUNDRED_THOUSAND)
			.user_balance(ADMIN, BTC, ONE_HUNDRED_THOUSAND)
			.user_balance(ALICE, DOT, ONE_HUNDRED_THOUSAND)
			.user_balance(ALICE, ETH, ONE_HUNDRED_THOUSAND)
			.user_balance(ALICE, BTC, ONE_HUNDRED_THOUSAND)
			.user_balance(BOB, DOT, ONE_HUNDRED_THOUSAND)
			.user_balance(BOB, ETH, ONE_HUNDRED_THOUSAND)
			.user_balance(BOB, BTC, ONE_HUNDRED_THOUSAND)
			.mnt_enabled_pools(vec![(DOT, DOLLARS / 10), (ETH, DOLLARS / 10)])
			.mnt_account_balance(ONE_HUNDRED_THOUSAND)
			.mnt_claim_threshold(dollars(100))
			.build()
			.execute_with(|| {
				assert!(mnt_token::MntSpeeds::<Test>::contains_key(DOT));
				assert!(mnt_token::MntSpeeds::<Test>::contains_key(ETH));
				assert!(!mnt_token::MntSpeeds::<Test>::contains_key(BTC));
				assert!(!mnt_token::MntSpeeds::<Test>::contains_key(KSM));
				// Set initial state of pools for distribution MNT tokens.
				vec![DOT, ETH].into_iter().for_each(|pool_id| {
					assert_ok!(MinterestProtocol::deposit_underlying(
						bob_origin(),
						pool_id,
						ONE_HUNDRED_THOUSAND
					));
					assert_ok!(MinterestProtocol::enable_is_collateral(bob_origin(), pool_id));
					assert_ok!(MinterestProtocol::borrow(bob_origin(), pool_id, 50_000 * DOLLARS));
				});

				System::set_block_number(10);

				// ALice deposits DOT and enables her DOT pool as a collateral.
				assert_ok!(MinterestProtocol::deposit_underlying(
					alice_origin(),
					DOT,
					ONE_HUNDRED_THOUSAND
				));
				assert_ok!(MinterestProtocol::enable_is_collateral(alice_origin(), DOT));

				System::set_block_number(20);

				assert_ok!(MinterestProtocol::borrow(alice_origin(), ETH, 50_000 * DOLLARS));

				// Accrued MNT tokens are equal to zero, since distribution occurs only at
				// the moment of repeated user interaction with the protocol
				// (deposit, redeem, borrow, repay, transfer, claim).
				assert_eq!(TestMntToken::mnt_accrued(ALICE), Balance::zero());
				assert_eq!(Tokens::free_balance(MNT, &ALICE), Balance::zero());

				System::set_block_number(30);

				// Init BTC pool
				assert_ok!(MinterestProtocol::create_pool(
					admin_origin(),
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
						liquidation_threshold: Rate::saturating_from_rational(103, 100),
						liquidation_fee: Rate::saturating_from_rational(105, 100),
					},
				));
				assert_ok!(MinterestProtocol::deposit_underlying(
					admin_origin(),
					BTC,
					ONE_HUNDRED_THOUSAND
				));
				assert_ok!(MinterestProtocol::enable_is_collateral(admin_origin(), BTC));
				assert_ok!(MinterestProtocol::borrow(admin_origin(), BTC, 50_000 * DOLLARS));

				assert_ok!(MinterestProtocol::borrow(alice_origin(), BTC, 30_000 * DOLLARS));

				System::set_block_number(70);
				assert_eq!(TestMntToken::mnt_accrued(ALICE), Balance::zero());

				assert_ok!(TestMntToken::set_speed(admin_origin(), BTC, 2 * DOLLARS));
				test_mnt_speeds(
					100_000_000_000_000_000,
					100_000_000_000_000_000,
					2_000_000_000_000_000_000,
				);
				assert_eq!(TestMntToken::mnt_accrued(ALICE), Balance::zero());

				assert_ok!(MinterestProtocol::repay_all(alice_origin(), ETH));

				System::set_block_number(80);
				assert_eq!(TestMntToken::mnt_accrued(ALICE), 2_499_999_893_124_959_137);

				// Alice is able to claim rewards from all three pools
				assert_ok!(MinterestProtocol::claim_mnt(alice_origin(), vec![DOT, ETH]));
				assert_eq!(Currencies::free_balance(MNT, &ALICE), 5_999_999_861_231_159_718);
				assert_ok!(MinterestProtocol::claim_mnt(alice_origin(), vec![BTC]));
				assert_eq!(Currencies::free_balance(MNT, &ALICE), 13_499_999_861_231_159_718);
			})
	}

	// This scenario works with one user and two pools.
	// This test checks that is there is only one supplier and borrower
	// all distributed tokens go to this account.
	// Also it checks that for a single user amount of distributed tokens is the same
	// for pool created in genesis block and pool added later.
	// Initial parameters: 	ETH - enabled in mnt minting;
	// 						mnt_speed = 0.1 MNT per block;
	// 1. Alice deposit() 100_000 ETH;
	// 2. Alice borrow() 50_000 ETH;
	// 3. Alice claim_mnt() 20 * 0.1 = 2 MNT
	// 4. Disable MNT distribution for ETH pool;
	// 5. Init BTC pool;
	// 6. Alice deposit() 100_000 BTC;
	// 7. Alice borrow() 50_000 BTC;
	// 8. Enable MNT distribution for BTC pool;
	// 9. Alice claim_mnt() 20 * 0.1 = 2 MNT
	#[test]
	fn test_mnt_token_scenario_n_3() {
		ExtBuilder::default()
			.set_controller_data(vec![(
				ETH,
				ControllerData {
					last_interest_accrued_block: 0,
					protocol_interest_factor: Rate::saturating_from_rational(1, 10),
					max_borrow_rate: Rate::saturating_from_rational(5, 1000),
					collateral_factor: Rate::saturating_from_rational(9, 10), // 90%
					borrow_cap: None,
					protocol_interest_threshold: PROTOCOL_INTEREST_TRANSFER_THRESHOLD,
				},
			)])
			.set_minterest_model_params(vec![(
				ETH,
				MinterestModelData {
					kink: Rate::saturating_from_rational(8, 10),
					base_rate_per_block: Rate::zero(),
					multiplier_per_block: Rate::saturating_from_rational(9, 1_000_000_000), // 0.047304 PerYear
					jump_multiplier_per_block: Rate::saturating_from_rational(207, 1_000_000_000), // 1.09 PerYear
				},
			)])
			.pool_initial(ETH)
			.user_balance(ADMIN, ETH, ONE_HUNDRED_THOUSAND)
			.user_balance(ADMIN, BTC, ONE_HUNDRED_THOUSAND)
			.user_balance(ALICE, ETH, ONE_HUNDRED_THOUSAND)
			.user_balance(ALICE, BTC, ONE_HUNDRED_THOUSAND)
			.mnt_enabled_pools(vec![(ETH, DOLLARS / 10)])
			.mnt_account_balance(ONE_HUNDRED_THOUSAND)
			.build()
			.execute_with(|| {
				assert!(!mnt_token::MntSpeeds::<Test>::contains_key(DOT));
				assert!(mnt_token::MntSpeeds::<Test>::contains_key(ETH));
				assert!(!mnt_token::MntSpeeds::<Test>::contains_key(BTC));
				assert!(!mnt_token::MntSpeeds::<Test>::contains_key(KSM));
				// Set initial state of pools for distribution MNT tokens.
				System::set_block_number(10);
				// Alice starts taking part in the distribution (ETH) from block 10
				assert_ok!(MinterestProtocol::deposit_underlying(
					alice_origin(),
					ETH,
					ONE_HUNDRED_THOUSAND
				));
				assert_ok!(MinterestProtocol::enable_is_collateral(alice_origin(), ETH));
				assert_ok!(MinterestProtocol::borrow(alice_origin(), ETH, 50_000 * DOLLARS));

				// Accrued MNT tokens are equal to zero, since distribution occurs only at
				// the moment of repeated user interaction with the protocol
				// (deposit, redeem, borrow, repay, transfer, claim).
				assert_eq!(TestMntToken::mnt_accrued(ALICE), Balance::zero());
				assert_eq!(Currencies::free_balance(MNT, &ALICE), Balance::zero());

				System::set_block_number(20);
				// Only ETH pool is enabled
				test_mnt_speeds(0, 100_000_000_000_000_000, 0);

				assert_eq!(Currencies::free_balance(MNT, &ALICE), Balance::zero());
				assert_ok!(MinterestProtocol::claim_mnt(alice_origin(), vec![ETH]));

				// ETH speed = 0.1
				// block delta = 10
				// distributed_to_alice_for_eth_pool = 0.1 (speed) * 10 (blocks) * 2 (supply and borrow)
				let distributed_to_alice_for_eth_pool = 2_000_000_000_000_000_000;
				assert_eq!(Currencies::free_balance(MNT, &ALICE), distributed_to_alice_for_eth_pool);
				assert_eq!(
					Currencies::free_balance(MNT, &TestMntToken::get_account_id()),
					ONE_HUNDRED_THOUSAND - distributed_to_alice_for_eth_pool
				);
				assert_ok!(TestMntToken::set_speed(admin_origin(), ETH, Balance::zero()));

				// Init BTC pool
				assert_ok!(MinterestProtocol::create_pool(
					admin_origin(),
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
						liquidation_threshold: Rate::saturating_from_rational(103, 100),
						liquidation_fee: Rate::saturating_from_rational(105, 100),
					},
				));
				assert_ok!(MinterestProtocol::deposit_underlying(
					alice_origin(),
					BTC,
					ONE_HUNDRED_THOUSAND
				));
				assert_ok!(MinterestProtocol::enable_is_collateral(alice_origin(), BTC));
				assert_ok!(MinterestProtocol::borrow(alice_origin(), BTC, 50_000 * DOLLARS));
				// Set the same speed for BTC pool
				assert_ok!(TestMntToken::set_speed(admin_origin(), BTC, DOLLARS / 10));
				System::set_block_number(30);
				// Only BTC pool is enabled
				test_mnt_speeds(0, 0, 100_000_000_000_000_000);

				assert_eq!(Currencies::free_balance(MNT, &ALICE), distributed_to_alice_for_eth_pool);
				assert_ok!(MinterestProtocol::claim_mnt(alice_origin(), vec![BTC]));

				// Alice got the same amount of tokens for BTC pool
				let distributed_to_alice_for_btc_pool = 2_000_000_000_000_000_000;
				assert_eq!(
					Currencies::free_balance(MNT, &ALICE),
					distributed_to_alice_for_eth_pool + distributed_to_alice_for_btc_pool
				);
				assert_eq!(
					Currencies::free_balance(MNT, &TestMntToken::get_account_id()),
					ONE_HUNDRED_THOUSAND - (distributed_to_alice_for_eth_pool + distributed_to_alice_for_btc_pool)
				);
			})
	}

	// This scenarion works with one user and one pool.
	// It checks that distribution works correctly after being stopped and resumed.
	// Initial parameters: 	DOT - enabled in mnt minting;
	// 						mnt_speed = 10 MNT per block;
	// 1. Alice deposit() 100_000 DOT;
	// 2. Alice claim_mnt(DOT)
	// 3. Disable MNT distribution for DOT pool;
	// 4. Enable MNT distribution for DOT pool;
	// 5. Alice claim_mnt(DOT)
	#[test]
	fn test_mnt_token_scenario_n_4() {
		ExtBuilder::default()
			.pool_initial(DOT)
			.user_balance(ADMIN, DOT, ONE_HUNDRED_THOUSAND)
			.user_balance(ALICE, DOT, ONE_HUNDRED_THOUSAND)
			.user_balance(BOB, DOT, ONE_HUNDRED_THOUSAND)
			.mnt_enabled_pools(vec![(DOT, 10 * DOLLARS)])
			.mnt_account_balance(ONE_HUNDRED_THOUSAND)
			.build()
			.execute_with(|| {
				assert!(mnt_token::MntSpeeds::<Test>::contains_key(DOT));
				assert!(!mnt_token::MntSpeeds::<Test>::contains_key(ETH));
				assert!(!mnt_token::MntSpeeds::<Test>::contains_key(BTC));
				assert!(!mnt_token::MntSpeeds::<Test>::contains_key(KSM));
				// Initialize distribution of MNT tokens.
				assert_ok!(MinterestProtocol::deposit_underlying(
					admin_origin(),
					DOT,
					ONE_HUNDRED_THOUSAND
				));
				assert_ok!(MinterestProtocol::enable_is_collateral(admin_origin(), DOT));
				assert_ok!(MinterestProtocol::borrow(admin_origin(), DOT, 50_000 * DOLLARS));

				System::set_block_number(10);

				// ALice deposits DOT and enables her DOT pool as a collateral.
				// At this moment Alice starts receiving (dot_speed / 2) MNT per block as a supplier
				assert_ok!(MinterestProtocol::deposit_underlying(
					alice_origin(),
					DOT,
					ONE_HUNDRED_THOUSAND
				));
				assert_ok!(MinterestProtocol::enable_is_collateral(alice_origin(), DOT));

				System::set_block_number(20);

				// Accrued MNT tokens are equal to zero, since distribution occurs only at
				// the moment of repeated user interaction with the protocol
				// (deposit, redeem, borrow, repay, transfer, claim).
				assert_eq!(TestMntToken::mnt_accrued(ALICE), Balance::zero());
				assert_eq!(Tokens::free_balance(MNT, &ALICE), Balance::zero());

				// Alice started taking part in DOT pool distribution at block 10 as a supplier
				// mnt_balance = 10(dot_speed) * 10(delta_blocks) * 100(supply) / 200(total_supply) = 50 MNT
				assert_ok!(MinterestProtocol::claim_mnt(alice_origin(), vec![DOT]));
				assert_eq!(Currencies::free_balance(MNT, &ALICE), 49_999_999_544_374_908_303);

				// Disable DOT pool distribution
				assert_ok!(TestMntToken::set_speed(admin_origin(), DOT, Balance::zero()));

				System::set_block_number(30);

				assert_ok!(TestMntToken::set_speed(admin_origin(), DOT, 10 * DOLLARS));

				System::set_block_number(40);

				// DOT pool distribution was resumed at block 30
				// mnt_balance = 50 MNT (current) + 10(dot_speed) * 10(delta_blocks) * 100(supply) /
				// 200(total_supply) = 100 MNT
				assert_ok!(MinterestProtocol::claim_mnt(alice_origin(), vec![DOT]));
				assert_eq!(Currencies::free_balance(MNT, &ALICE), 99_999_999_088_749_816_606);
			})
	}

	// Test MNT distribution behaviour when users are using transfer_wrapped
	#[test]
	fn mnt_token_supplier_distribution_when_users_transferring_tokens() {
		ExtBuilder::default()
			.pool_initial(DOT)
			.user_balance(ADMIN, DOT, ONE_HUNDRED_THOUSAND)
			.user_balance(ALICE, DOT, ONE_HUNDRED_THOUSAND)
			.user_balance(BOB, DOT, ONE_HUNDRED_THOUSAND)
			.user_balance(CHARLIE, DOT, 2 * ONE_HUNDRED_THOUSAND)
			.mnt_enabled_pools(vec![(DOT, DOLLARS / 10)])
			.mnt_account_balance(ONE_HUNDRED_THOUSAND)
			.build()
			.execute_with(|| {
				// Set initial state of pools for distribution MNT tokens.
				assert_ok!(MinterestProtocol::deposit_underlying(
					admin_origin(),
					DOT,
					ONE_HUNDRED_THOUSAND
				));
				assert_ok!(MinterestProtocol::enable_is_collateral(admin_origin(), DOT));
				assert_ok!(MinterestProtocol::borrow(admin_origin(), DOT, 50_000 * DOLLARS));

				System::set_block_number(10);

				// Alice, Bob and Carol deposit DOT.
				assert_ok!(MinterestProtocol::deposit_underlying(
					alice_origin(),
					DOT,
					ONE_HUNDRED_THOUSAND
				));
				assert_ok!(MinterestProtocol::deposit_underlying(
					bob_origin(),
					DOT,
					ONE_HUNDRED_THOUSAND
				));
				assert_ok!(MinterestProtocol::deposit_underlying(
					charlie_origin(),
					DOT,
					2 * ONE_HUNDRED_THOUSAND
				));

				System::set_block_number(20);

				// Check that both Alice and Bob receive the same amount of MNT token since they
				// have equal DOT balance
				let mnt_balance_after_deposit = 199_999_999_270_900_013;
				assert_ok!(MinterestProtocol::claim_mnt(alice_origin(), vec![DOT]));
				assert_ok!(MinterestProtocol::claim_mnt(bob_origin(), vec![DOT]));
				assert_eq!(Currencies::free_balance(MNT, &ALICE), mnt_balance_after_deposit);
				assert_eq!(Currencies::free_balance(MNT, &BOB), mnt_balance_after_deposit);
				assert_eq!(Currencies::free_balance(MNT, &CHARLIE), Balance::zero());

				// Alice transfers all to Bob
				assert_ok!(MinterestProtocol::transfer_wrapped(
					alice_origin(),
					BOB,
					MDOT,
					Currencies::free_balance(MDOT, &ALICE)
				));

				System::set_block_number(30);

				// Check that Alice received 0 MNT and Bob received approximately x2 comparing to
				// previous claim
				let mnt_bob_balance_after_transfer = mnt_balance_after_deposit + 399_999_998_541_800_026;
				assert_ok!(MinterestProtocol::claim_mnt(alice_origin(), vec![DOT]));
				assert_ok!(MinterestProtocol::claim_mnt(bob_origin(), vec![DOT]));
				assert_eq!(Currencies::free_balance(MNT, &ALICE), mnt_balance_after_deposit);
				assert_eq!(Currencies::free_balance(MNT, &BOB), mnt_bob_balance_after_transfer);

				// Bob transfers one third of its balance to Alice
				assert_ok!(MinterestProtocol::transfer_wrapped(
					bob_origin(),
					ALICE,
					MDOT,
					Currencies::free_balance(MDOT, &BOB) / 3
				));

				System::set_block_number(40);

				// Test proportions 1:2. Amount of tokens Bob receive after claim must be twice
				// bigger comparing to claim amount for Alice.
				let mnt_alice_delta_after_second_transfer = 133_333_332_847_266_675;
				let mnt_alice_balance_after_second_transfer =
					mnt_balance_after_deposit + mnt_alice_delta_after_second_transfer;
				let mnt_bob_balance_after_second_transfer = mnt_bob_balance_after_transfer +
					mnt_alice_delta_after_second_transfer * 2 +
					/*calculation error, it is okay for such algorithms*/ 1;
				assert_ok!(MinterestProtocol::claim_mnt(alice_origin(), vec![DOT]));
				assert_ok!(MinterestProtocol::claim_mnt(bob_origin(), vec![DOT]));
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
					alice_origin(),
					BOB,
					MDOT,
					Currencies::free_balance(MDOT, &ALICE) / 2
				));
				assert_ok!(MinterestProtocol::transfer_wrapped(
					bob_origin(),
					ALICE,
					MDOT,
					Currencies::free_balance(MDOT, &BOB) / 2
				));
				assert_ok!(MinterestProtocol::transfer_wrapped(
					alice_origin(),
					BOB,
					MDOT,
					Currencies::free_balance(MDOT, &ALICE)
				));
				// Return the same proportions (1:2) eventually
				assert_ok!(MinterestProtocol::transfer_wrapped(
					bob_origin(),
					ALICE,
					MDOT,
					Currencies::free_balance(MDOT, &BOB) / 3
				));

				System::set_block_number(100);

				// Test proportions 1:2 one more time. Transfers within one block doesn't affect
				// calculations
				let mnt_alice_delta_after_third_transfer = 799_999_997_083_933_386;
				let mnt_alice_balance_after_third_transfer =
					mnt_alice_balance_after_second_transfer + mnt_alice_delta_after_third_transfer;
				let mnt_bob_balance_after_third_transfer =
					mnt_bob_balance_after_second_transfer + mnt_alice_delta_after_third_transfer * 2;
				assert_ok!(MinterestProtocol::claim_mnt(alice_origin(), vec![DOT]));
				assert_ok!(MinterestProtocol::claim_mnt(bob_origin(), vec![DOT]));
				assert_eq!(
					Currencies::free_balance(MNT, &ALICE),
					mnt_alice_balance_after_third_transfer
				);
				assert_eq!(
					Currencies::free_balance(MNT, &BOB),
					mnt_bob_balance_after_third_transfer
				);

				assert_ok!(MinterestProtocol::claim_mnt(charlie_origin(), vec![DOT]));
				assert_eq!(
					Currencies::free_balance(MNT, &CHARLIE),
					mnt_alice_balance_after_third_transfer +
						mnt_bob_balance_after_third_transfer +
						/*calculation error, it is okay for such algorithms*/ 3
				);
			});
	}
}
