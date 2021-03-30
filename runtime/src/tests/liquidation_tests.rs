use super::*;

// TODO tests for liquidation
#[test]
fn test_liquidation() {
	ExtBuilder::default().build().execute_with(|| {
		assert_ok!(Currencies::deposit(
			DOT,
			&LiquidityPoolsModuleId::get().into_account(),
			dollars(200_u128)
		));
		assert_ok!(Currencies::deposit(
			DOT,
			&LiquidationPoolsModuleId::get().into_account(),
			dollars(40_u128)
		));
		assert_eq!(
			Currencies::free_balance(DOT, &LiquidityPoolsModuleId::get().into_account()),
			dollars(200_u128)
		);
		assert_eq!(
			Currencies::free_balance(DOT, &LiquidationPoolsModuleId::get().into_account()),
			dollars(40_u128)
		);
		assert_eq!(LiquidityPools::get_pool_available_liquidity(DOT), dollars(200_u128));
		assert_eq!(LiquidationPools::get_pool_available_liquidity(DOT), dollars(40_u128));
	});
}

#[test]
fn complete_liquidation_one_collateral_should_work() {
	ExtBuilder::default()
		.liquidity_pool_balance(CurrencyId::DOT, 110_000 * DOLLARS)
		.liquidation_pool_balance(CurrencyId::DOT, 100_000 * DOLLARS)
		.user_balance(ALICE::get(), CurrencyId::MDOT, 100_000 * DOLLARS)
		.user_balance(BOB::get(), CurrencyId::MDOT, 100_000 * DOLLARS)
		.pool_user_data(CurrencyId::DOT, ALICE::get(), 90_000 * DOLLARS, Rate::one(), true, 3)
		.pool_total_borrowed(CurrencyId::DOT, 90_000 * DOLLARS)
		.build()
		.execute_with(|| {
			// Set price = 2.00 USD for all polls.
			assert_ok!(set_oracle_price_for_all_pools(2));

			assert_ok!(RiskManager::liquidate_unsafe_loan(ALICE::get(), CurrencyId::DOT));

			let expected_event = Event::risk_manager(risk_manager::Event::LiquidateUnsafeLoan(
				ALICE::get(),
				180_000 * DOLLARS,
				CurrencyId::DOT,
				vec![CurrencyId::DOT],
				false,
			));
			assert!(System::events().iter().any(|record| record.event == expected_event));

			assert_eq!(
				Currencies::free_balance(CurrencyId::MDOT, &ALICE::get()),
				5_500 * DOLLARS
			);

			assert_eq!(
				LiquidityPools::get_pool_available_liquidity(CurrencyId::DOT),
				105_500 * DOLLARS
			);
			assert_eq!(
				LiquidationPools::get_pool_available_liquidity(CurrencyId::DOT),
				104_500 * DOLLARS
			);

			assert_eq!(LiquidityPools::pools(CurrencyId::DOT).total_borrowed, Balance::zero());
			assert_eq!(
				LiquidityPools::pool_user_data(CurrencyId::DOT, ALICE::get()).total_borrowed,
				Balance::zero()
			);

			assert_eq!(
				LiquidityPools::pool_user_data(CurrencyId::DOT, ALICE::get()).liquidation_attempts,
				0
			);
		})
}

#[test]
fn complete_liquidation_multi_collateral_should_work() {
	ExtBuilder::default()
		.liquidity_pool_balance(CurrencyId::DOT, 160_000 * DOLLARS)
		.liquidity_pool_balance(CurrencyId::ETH, 50_000 * DOLLARS)
		.liquidation_pool_balance(CurrencyId::DOT, 100_000 * DOLLARS)
		.liquidation_pool_balance(CurrencyId::ETH, 100_000 * DOLLARS)
		.user_balance(ALICE::get(), CurrencyId::MDOT, 50_000 * DOLLARS)
		.user_balance(ALICE::get(), CurrencyId::METH, 50_000 * DOLLARS)
		.user_balance(BOB::get(), CurrencyId::MDOT, 100_000 * DOLLARS)
		.user_balance(CHARLIE::get(), CurrencyId::MDOT, 100_000 * DOLLARS)
		.pool_user_data(CurrencyId::DOT, ALICE::get(), 90_000 * DOLLARS, Rate::one(), true, 3)
		.pool_user_data(CurrencyId::ETH, ALICE::get(), 0, Rate::one(), true, 0)
		.pool_total_borrowed(CurrencyId::DOT, 90_000 * DOLLARS)
		.build()
		.execute_with(|| {
			// Set price = 2.00 USD for all polls.
			assert_ok!(set_oracle_price_for_all_pools(2));

			assert_ok!(RiskManager::liquidate_unsafe_loan(ALICE::get(), CurrencyId::DOT));

			let expected_event = Event::risk_manager(risk_manager::Event::LiquidateUnsafeLoan(
				ALICE::get(),
				180_000 * DOLLARS,
				CurrencyId::DOT,
				vec![CurrencyId::DOT, CurrencyId::ETH],
				false,
			));
			assert!(System::events().iter().any(|record| record.event == expected_event));

			assert_eq!(
				Currencies::free_balance(CurrencyId::MDOT, &ALICE::get()),
				Balance::zero()
			);
			assert_eq!(
				Currencies::free_balance(CurrencyId::METH, &ALICE::get()),
				5_500 * DOLLARS
			);

			assert_eq!(
				LiquidityPools::get_pool_available_liquidity(CurrencyId::DOT),
				200_000 * DOLLARS
			);
			assert_eq!(
				LiquidityPools::get_pool_available_liquidity(CurrencyId::ETH),
				5_500 * DOLLARS
			);

			assert_eq!(
				LiquidationPools::get_pool_available_liquidity(CurrencyId::DOT),
				60_000 * DOLLARS
			);
			assert_eq!(
				LiquidationPools::get_pool_available_liquidity(CurrencyId::ETH),
				144_500 * DOLLARS
			);

			assert_eq!(LiquidityPools::pools(CurrencyId::DOT).total_borrowed, Balance::zero());
			assert_eq!(
				LiquidityPools::pool_user_data(CurrencyId::DOT, ALICE::get()).total_borrowed,
				Balance::zero()
			);

			assert_eq!(
				LiquidityPools::pool_user_data(CurrencyId::DOT, ALICE::get()).liquidation_attempts,
				0
			);
		})
}

#[test]
fn partial_liquidation_one_collateral_should_work() {
	ExtBuilder::default()
		.liquidity_pool_balance(CurrencyId::DOT, 110_000 * DOLLARS)
		.liquidation_pool_balance(CurrencyId::DOT, 100_000 * DOLLARS)
		.user_balance(ALICE::get(), CurrencyId::MDOT, 100_000 * DOLLARS)
		.user_balance(BOB::get(), CurrencyId::MDOT, 100_000 * DOLLARS)
		.pool_user_data(CurrencyId::DOT, ALICE::get(), 90_000 * DOLLARS, Rate::one(), true, 0)
		.pool_total_borrowed(CurrencyId::DOT, 90_000 * DOLLARS)
		.build()
		.execute_with(|| {
			// Set price = 2.00 USD for all polls.
			assert_ok!(set_oracle_price_for_all_pools(2));

			assert_ok!(RiskManager::liquidate_unsafe_loan(ALICE::get(), CurrencyId::DOT));

			let expected_event = Event::risk_manager(risk_manager::Event::LiquidateUnsafeLoan(
				ALICE::get(),
				54_000 * DOLLARS,
				CurrencyId::DOT,
				vec![CurrencyId::DOT],
				true,
			));
			assert!(System::events().iter().any(|record| record.event == expected_event));

			assert_eq!(
				Currencies::free_balance(CurrencyId::MDOT, &ALICE::get()),
				71_650 * DOLLARS
			);

			assert_eq!(
				LiquidityPools::get_pool_available_liquidity(CurrencyId::DOT),
				108_650 * DOLLARS
			);
			assert_eq!(
				LiquidationPools::get_pool_available_liquidity(CurrencyId::DOT),
				101_350 * DOLLARS
			);

			assert_eq!(LiquidityPools::pools(CurrencyId::DOT).total_borrowed, 63_000 * DOLLARS);
			assert_eq!(
				LiquidityPools::pool_user_data(CurrencyId::DOT, ALICE::get()).total_borrowed,
				63_000 * DOLLARS
			);

			assert_eq!(
				LiquidityPools::pool_user_data(CurrencyId::DOT, ALICE::get()).liquidation_attempts,
				1
			);
		})
}

#[test]
fn partial_liquidation_multi_collateral_should_work() {
	ExtBuilder::default()
		.liquidity_pool_balance(CurrencyId::DOT, 130_000 * DOLLARS)
		.liquidity_pool_balance(CurrencyId::ETH, 80_000 * DOLLARS)
		.liquidation_pool_balance(CurrencyId::DOT, 100_000 * DOLLARS)
		.liquidation_pool_balance(CurrencyId::ETH, 100_000 * DOLLARS)
		.user_balance(ALICE::get(), CurrencyId::MDOT, 20_000 * DOLLARS)
		.user_balance(ALICE::get(), CurrencyId::METH, 80_000 * DOLLARS)
		.user_balance(BOB::get(), CurrencyId::MDOT, 100_000 * DOLLARS)
		.user_balance(CHARLIE::get(), CurrencyId::MDOT, 100_000 * DOLLARS)
		.pool_user_data(CurrencyId::DOT, ALICE::get(), 90_000 * DOLLARS, Rate::one(), true, 0)
		.pool_user_data(CurrencyId::ETH, ALICE::get(), 0, Rate::one(), true, 0)
		.pool_total_borrowed(CurrencyId::DOT, 90_000 * DOLLARS)
		.build()
		.execute_with(|| {
			// Set price = 2.00 USD for all polls.
			assert_ok!(set_oracle_price_for_all_pools(2));

			assert_ok!(RiskManager::liquidate_unsafe_loan(ALICE::get(), CurrencyId::DOT));

			let expected_event = Event::risk_manager(risk_manager::Event::LiquidateUnsafeLoan(
				ALICE::get(),
				54_000 * DOLLARS,
				CurrencyId::DOT,
				vec![CurrencyId::ETH],
				true,
			));
			assert!(System::events().iter().any(|record| record.event == expected_event));

			assert_eq!(
				Currencies::free_balance(CurrencyId::MDOT, &ALICE::get()),
				20_000 * DOLLARS
			);
			assert_eq!(
				Currencies::free_balance(CurrencyId::METH, &ALICE::get()),
				51_650 * DOLLARS
			);

			assert_eq!(
				LiquidityPools::get_pool_available_liquidity(CurrencyId::DOT),
				157_000 * DOLLARS
			);
			assert_eq!(
				LiquidityPools::get_pool_available_liquidity(CurrencyId::ETH),
				51_650 * DOLLARS
			);

			assert_eq!(
				LiquidationPools::get_pool_available_liquidity(CurrencyId::DOT),
				73_000 * DOLLARS
			);
			assert_eq!(
				LiquidationPools::get_pool_available_liquidity(CurrencyId::ETH),
				128_350 * DOLLARS
			);

			assert_eq!(LiquidityPools::pools(CurrencyId::DOT).total_borrowed, 63_000 * DOLLARS);
			assert_eq!(
				LiquidityPools::pool_user_data(CurrencyId::DOT, ALICE::get()).total_borrowed,
				63_000 * DOLLARS
			);

			assert_eq!(
				LiquidityPools::pool_user_data(CurrencyId::DOT, ALICE::get()).liquidation_attempts,
				1
			);
		})
}

#[test]
fn complete_liquidation_should_not_work() {
	ExtBuilder::default()
		.liquidity_pool_balance(CurrencyId::DOT, 60_000 * DOLLARS)
		.liquidity_pool_balance(CurrencyId::ETH, 50_000 * DOLLARS)
		.user_balance(ALICE::get(), CurrencyId::MDOT, 50_000 * DOLLARS)
		.user_balance(ALICE::get(), CurrencyId::METH, 50_000 * DOLLARS)
		.user_balance(CHARLIE::get(), CurrencyId::MDOT, 100_000 * DOLLARS)
		.pool_user_data(CurrencyId::DOT, ALICE::get(), 90_000 * DOLLARS, Rate::one(), true, 3)
		.pool_user_data(CurrencyId::ETH, ALICE::get(), 0, Rate::one(), false, 0)
		.pool_total_borrowed(CurrencyId::DOT, 90_000 * DOLLARS)
		.build()
		.execute_with(|| {
			// Set price = 2.00 USD for all polls.
			assert_ok!(set_oracle_price_for_all_pools(2));

			assert_err!(
				RiskManager::liquidate_unsafe_loan(ALICE::get(), CurrencyId::DOT),
				minterest_protocol::Error::<Runtime>::NotEnoughUnderlyingAsset
			);
		})
}

#[test]
fn partial_liquidation_should_not_work() {
	ExtBuilder::default()
		.liquidity_pool_balance(CurrencyId::DOT, 20_000 * DOLLARS)
		.liquidity_pool_balance(CurrencyId::ETH, 15_000 * DOLLARS)
		.user_balance(ALICE::get(), CurrencyId::MDOT, 10_000 * DOLLARS)
		.user_balance(ALICE::get(), CurrencyId::METH, 15_000 * DOLLARS)
		.user_balance(CHARLIE::get(), CurrencyId::MDOT, 100_000 * DOLLARS)
		.pool_user_data(CurrencyId::DOT, ALICE::get(), 90_000 * DOLLARS, Rate::one(), true, 2)
		.pool_user_data(CurrencyId::BTC, ALICE::get(), 0, Rate::one(), true, 0)
		.pool_total_borrowed(CurrencyId::DOT, 90_000 * DOLLARS)
		.build()
		.execute_with(|| {
			// Set price = 2.00 USD for all polls.
			assert_ok!(set_oracle_price_for_all_pools(2));

			assert_err!(
				RiskManager::liquidate_unsafe_loan(ALICE::get(), CurrencyId::DOT),
				minterest_protocol::Error::<Runtime>::NotEnoughUnderlyingAsset
			);
		})
}
