use super::*;

/*
Description of scenario #1:

Collateral factor = 90% for all pools.
Alice - supplier, Bob - borrower.
1. Bob made DOT and ETH deposit into the system and set both as collateral.
2. Bob borrowed BTC.
3. Ethereum price decreased.
4. The first partial liquidation.
5. Bob redeems all the collateral DOT.
6. Bob redeems ETH and left only 1 token in the protocol.
7. Bitcoin price has increased.
8. Complete liquidation.
 */
#[test]
fn liquidation_scenario_n1() {
	ExtBuilder::default()
		.pool_initial(DOT)
		.pool_initial(ETH)
		.pool_initial(BTC)
		.liquidation_pool_balance(DOT, dollars(20_000))
		.liquidation_pool_balance(ETH, dollars(20_000))
		.liquidation_pool_balance(BTC, dollars(20_000))
		.pool_user_data(DOT, BOB::get(), Balance::zero(), Rate::one(), false, 0)
		.pool_user_data(ETH, BOB::get(), Balance::zero(), Rate::one(), false, 0)
		.pool_user_data(BTC, BOB::get(), Balance::zero(), Rate::one(), false, 0)
		.build()
		.execute_with(|| {
			// Set prices for currencies.
			assert_ok!(MinterestOracle::feed_values(
				origin_of(ORACLE1::get().clone()),
				vec![
					(DOT, Rate::saturating_from_integer(50)),
					(ETH, Rate::saturating_from_integer(2_000)),
					(BTC, Rate::saturating_from_integer(50_000)),
					(KSM, Rate::saturating_from_integer(500)),
				]
			));
			run_to_block(1);

			assert_ok!(MinterestProtocol::deposit_underlying(alice(), DOT, dollars(100_000)));
			run_to_block(100);
			assert_ok!(MinterestProtocol::deposit_underlying(alice(), ETH, dollars(100_000)));
			run_to_block(200);
			assert_ok!(MinterestProtocol::deposit_underlying(alice(), BTC, dollars(100_000)));
			run_to_block(300);

			assert_ok!(MinterestProtocol::deposit_underlying(bob(), DOT, dollars(50_000)));
			run_to_block(400);
			assert_ok!(MinterestProtocol::deposit_underlying(bob(), ETH, dollars(100_000)));
			run_to_block(500);

			assert_ok!(MinterestProtocol::enable_is_collateral(bob(), DOT));
			run_to_block(550);
			assert_ok!(MinterestProtocol::enable_is_collateral(bob(), ETH));
			run_to_block(600);

			assert_ok!(MinterestProtocol::borrow(bob(), BTC, dollars(3500)));
			run_to_block(700);

			assert_ok!(MinterestOracle::feed_values(
				origin_of(ORACLE1::get().clone()),
				vec![(ETH, Rate::saturating_from_integer(1910))]
			));
			run_to_block(800);

			assert_eq!(LiquidityPools::get_pool_available_liquidity(BTC), dollars(96_500));
			assert_eq!(LiquidityPools::get_pool_available_liquidity(ETH), dollars(200_000));

			// ------------------- FIRST PARTIAL LIQUIDATION -----------------------------
			// sum_collateral = 50_000 DOT * 50 + 100_000 ETH * 1_910 = $193_500_000;
			// sum_borrow = $175_000_000 > sum_collateral = $193_500_000 * 0.9 = $174_150_000;
			// Call partial liquidation:
			assert_ok!(RiskManager::liquidate_unsafe_loan(BOB::get(), BTC));

			/*
			seize_amount = 1.05 * 0.3 * $175_000_000 = $55_125_000 = 28_861 ETH;
			repay_amount = 0.3 * $175_000_000 = $52_500_000 = 1050 BTC;
			current sum_collateral = $193_500_000 - $55_125_000 = $138_375_000;
			current sum_borrow = $175_000_000 - $52_500_000 = $122_500_000 < sum_collateral = $138_375_000 * 0.9 = $124_537_500;
			NOTE: 0.3 - temporary factor for partial liquidation;
			 */
			let expected_event = Event::risk_manager(risk_manager::Event::LiquidateUnsafeLoan(
				BOB::get(),
				52_500_003_307_499_999_999_999_924, // repay_amount = $52_500_000;
				BTC,                                // liquidated_pool_id;
				vec![ETH],                          // seized_pools
				true,
			));
			assert!(System::events().iter().any(|record| record.event == expected_event));

			assert_eq!(
				LiquidityPools::pool_user_data(BTC, BOB::get()),
				PoolUserData {
					total_borrowed: 2_450_000_154_350_000_000_000, // 3500 BTC - 1_050 BTC = 2_450 BTC
					interest_index: Rate::from_inner(1_000_000_063_000_000_000),
					is_collateral: false,
					liquidation_attempts: 1,
				}
			);

			// Borrowed liquidity pool balance: 96_500 BTC + 1_050 BTC =  97_550 BTC;
			assert_eq!(
				LiquidityPools::get_pool_available_liquidity(BTC),
				97_550_000_066_150_000_000_000
			);
			// Collateralizing liquidity pool balance: 200_000 ETH - 28_861 ETH = 171_139 ETH
			assert_eq!(
				LiquidityPools::get_pool_available_liquidity(ETH),
				171_138_741_637_238_219_895_288
			);
			// Borrowed liquidation pool balance: 20_000 ETH + 28_861 ETH = 48_861 ETH
			assert_eq!(
				LiquidationPools::get_pool_available_liquidity(ETH),
				48_861_258_362_761_780_104_712
			);
			// Collateralizing liquidation pool balance: 20_000 BTC - 1_050 BTC = 18_950 BTC
			assert_eq!(
				LiquidationPools::get_pool_available_liquidity(BTC),
				18_949_999_933_850_000_000_000
			);
			// Borrower balance in wrapped tokens (balance - seize_amount):
			// 100_000 METH - 28_861 METH = 71_139 METH
			assert_eq!(
				Currencies::free_balance(METH, &BOB::get()),
				71_138_741_637_238_219_895_288
			);
			// current borrower account oversupply = $138_375_000 * 0.9 - $122_500_000 = $2_037_500;
			assert_eq!(
				Controller::get_hypothetical_account_liquidity(&BOB::get(), BTC, 0, 0),
				Ok((2_037_489_156_912_500_000_000_072, 0)),
			);

			// Here we want to get a complete liquidation. To do this, the user's
			// debt must be less than $100_000
			run_to_block(900);
			assert_ok!(MinterestProtocol::repay(bob(), BTC, dollars(2449)));
			run_to_block(950);
			assert_ok!(MinterestProtocol::redeem(bob(), DOT));
			assert_ok!(MinterestProtocol::redeem_underlying(bob(), ETH, dollars(71_109)));
			// current borrower account oversupply = $1_115;
			assert_eq!(
				Controller::get_hypothetical_account_liquidity(&BOB::get(), BTC, 0, 0),
				Ok((1_115_455_787_170_597_550_072, 0)),
			);

			// Bitcoin price has increased.
			assert_ok!(MinterestOracle::feed_values(
				origin_of(ORACLE1::get().clone()),
				vec![(BTC, Rate::saturating_from_integer(52_000))]
			));
			run_to_block(1000);

			// ------------------- COMPLETE LIQUIDATION -----------------------------
			// Call complete liquidation:
			assert_ok!(RiskManager::liquidate_unsafe_loan(BOB::get(), BTC));

			let expected_event = Event::risk_manager(risk_manager::Event::LiquidateUnsafeLoan(
				BOB::get(),
				52_010_835_370_810_769_596_505, // repay_amount = $52_010;
				BTC,                            // liquidated_pool_id;
				vec![ETH],                      // seized_pools
				false,
			));
			assert!(System::events().iter().any(|record| record.event == expected_event));

			assert_eq!(
				LiquidityPools::get_pool_available_liquidity(BTC),
				100_000_000_274_522_515_591_723 // 100_000 BTC;
			);
			assert_eq!(
				LiquidityPools::get_pool_available_liquidity(ETH),
				100_001_149_293_186_203_503_625 // 100_001 ETH;
			);
			assert_eq!(
				LiquidationPools::get_pool_available_liquidity(ETH),
				48_889_850_706_813_796_496_375 // 48_889 ETH;
			);
			assert_eq!(
				LiquidationPools::get_pool_available_liquidity(BTC),
				18_948_999_725_477_484_408_277 // 18_948 BTC;
			);
			assert_eq!(
				Currencies::free_balance(METH, &BOB::get()),
				1_149_293_186_203_503_625 // 1 METH ~ $2_000 - earned interest;
			);

			assert_eq!(
				LiquidityPools::pool_user_data(BTC, BOB::get()),
				PoolUserData {
					total_borrowed: 0,
					interest_index: Rate::from_inner(1_000_000_085_059_004_489),
					is_collateral: false,
					liquidation_attempts: 0,
				}
			);
		});
}

/*
Description of scenario:
This scenario handles the case, when user has not enough collateral to cover liquidation.
This is a rare but possible case (may be caused by Flash Crashes of BTC or outage of oracles).
This is a VERY painful case.
The algorithm performs liquidation, the borrow balance remains with the user.
 */
#[test]
fn liquidation_not_enough_collateral() {
	ExtBuilder::default()
		.pool_initial(DOT)
		.pool_initial(ETH)
		.pool_initial(BTC)
		.liquidation_pool_balance(DOT, dollars(1_000_000))
		.liquidation_pool_balance(ETH, dollars(1_000_000))
		.liquidation_pool_balance(BTC, dollars(1_000_000))
		.pool_user_data(DOT, ALICE::get(), Balance::zero(), Rate::one(), false, 0)
		.pool_user_data(ETH, ALICE::get(), Balance::zero(), Rate::one(), false, 0)
		.pool_user_data(BTC, ALICE::get(), Balance::zero(), Rate::one(), false, 3)
		.build()
		.execute_with(|| {
			// Set prices for currencies.
			assert_ok!(MinterestOracle::feed_values(
				origin_of(ORACLE1::get().clone()),
				vec![
					(DOT, Rate::saturating_from_integer(2)),
					(ETH, Rate::saturating_from_integer(2)),
					(BTC, Rate::saturating_from_integer(2)),
					(KSM, Rate::saturating_from_integer(2)),
				]
			));
			assert_ok!(MinterestProtocol::deposit_underlying(bob(), BTC, dollars(100_000)));
			run_to_block(1);
			assert_ok!(MinterestProtocol::deposit_underlying(alice(), DOT, dollars(50_000)));
			assert_ok!(MinterestProtocol::enable_is_collateral(alice(), DOT));
			run_to_block(10);
			assert_ok!(MinterestProtocol::deposit_underlying(alice(), ETH, dollars(50_000)));
			assert_ok!(MinterestProtocol::enable_is_collateral(alice(), ETH));
			run_to_block(20);
			assert_ok!(MinterestProtocol::borrow(alice(), BTC, dollars(50_000)));
			run_to_block(30);
			assert_ok!(MinterestOracle::feed_values(
				origin_of(ORACLE1::get().clone()),
				vec![(BTC, Rate::saturating_from_integer(100))]
			));
			run_to_block(40);
			assert_ok!(RiskManager::liquidate_unsafe_loan(ALICE::get(), BTC));
			let expected_event = Event::risk_manager(risk_manager::Event::LiquidateUnsafeLoan(
				ALICE::get(),
				190_476_190_476_190_476_190_476, // repay_amount = $190_476;
				BTC,                             // liquidated_pool_id;
				vec![DOT, ETH],                  // seized_pools
				false,
			));
			assert!(System::events().iter().any(|record| record.event == expected_event));

			assert_eq!(
				LiquidityPools::pool_user_data(BTC, ALICE::get()),
				PoolUserData {
					total_borrowed: 48_095_242_595_238_095_238_095, // 50_000 BTC - 1904.76 BTC = 48_095.24 BTC
					interest_index: Rate::from_inner(1_000_000_090_000_000_000),
					is_collateral: false,
					liquidation_attempts: 0,
				}
			);

			// Borrowed liquidity pool balance: 50_000 BTC + 1904.76 BTC =  51_904.76 BTC;
			assert_eq!(
				LiquidityPools::get_pool_available_liquidity(BTC),
				51_904_761_904_761_904_761_905
			);
			// Collateralizing liquidity pool balance: 0 ETH
			assert_eq!(LiquidityPools::get_pool_available_liquidity(ETH), Balance::zero());
			// Collateralizing liquidity pool balance: 0 DOT
			assert_eq!(LiquidityPools::get_pool_available_liquidity(DOT), Balance::zero());
			// Borrowed liquidation pool balance: 1_000_000 BTC - 1904.76 BTC = 998_095.24 BTC
			assert_eq!(
				LiquidationPools::get_pool_available_liquidity(BTC),
				998_095_238_095_238_095_238_095
			);
			// Collateralizing liquidation pool balance: 1_000_000 DOT + 50_000 DOT = 1_050_000 DOT
			assert_eq!(
				LiquidationPools::get_pool_available_liquidity(DOT),
				1_050_000_000_000_000_000_000_000
			);
			// Borrower balance in wrapped tokens (balance - seize_amount):
			// 50_000 METH - 50_000 METH = 0 METH
			assert_eq!(Currencies::free_balance(METH, &ALICE::get()), Balance::zero());
			// 50_000 MDOT - 50_000 MDOT = 0 MDOT
			assert_eq!(Currencies::free_balance(MDOT, &ALICE::get()), Balance::zero());
			// current borrower account shortfall = $4_809_524;
			assert_eq!(
				Controller::get_hypothetical_account_liquidity(&ALICE::get(), BTC, 0, 0),
				Ok((0, 4_809_524_259_523_809_523_809_500)),
			);
			// Borrower total collateral equal zero
			assert_eq!(Controller::get_user_total_collateral(ALICE::get()), Ok(Balance::zero()));
			// Borrower total borrow equal: shortfall / BTC price ($100)
			assert_eq!(
				Controller::get_user_borrow_per_asset(&ALICE::get(), BTC),
				Ok(4_809_524_259_523_809_523_809_500 / 100)
			);
		})
}
