use super::*;

#[test]
fn whitelist_mode_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		// Set price = 2.00 USD for all pools.
		assert_ok!(set_oracle_price_for_all_pools(2));
		System::set_block_number(1);
		assert_ok!(MinterestProtocol::deposit_underlying(bob(), DOT, dollars(10_000)));
		System::set_block_number(2);

		assert_ok!(Controller::switch_whitelist_mode(
			<Runtime as frame_system::Config>::Origin::root()
		));
		System::set_block_number(3);

		// In whitelist mode only members of 'WhitelistCouncil' can work with protocols.
		assert_noop!(
			MinterestProtocol::deposit_underlying(bob(), DOT, dollars(5_000)),
			BadOrigin
		);
		System::set_block_number(4);

		assert_ok!(WhitelistCouncilMembership::add_member(
			<Runtime as frame_system::Config>::Origin::root(),
			BOB::get()
		));
		System::set_block_number(5);

		assert_ok!(MinterestProtocol::deposit_underlying(bob(), DOT, dollars(10_000)));
	})
}

//------------ Protocol interest transfer tests ----------------------

// Protocol interest should be transferred to liquidation pool after block is finalized
#[test]
fn protocol_interest_transfer_should_work() {
	ExtBuilder::default()
		.pool_initial(DOT)
		.pool_initial(ETH)
		.build()
		.execute_with(|| {
			// Set price = 2.00 USD for all pools.
			assert_ok!(set_oracle_price_for_all_pools(2));

			// Set interest factor equal 0.75.
			assert_ok!(Controller::set_protocol_interest_factor(
				origin_root(),
				DOT,
				Rate::saturating_from_rational(3, 4)
			));

			assert_ok!(MinterestProtocol::deposit_underlying(alice(), DOT, dollars(100_000)));
			assert_ok!(MinterestProtocol::deposit_underlying(alice(), ETH, dollars(100_000)));

			System::set_block_number(10);

			assert_ok!(MinterestProtocol::deposit_underlying(bob(), DOT, dollars(50_000)));
			assert_ok!(MinterestProtocol::deposit_underlying(bob(), ETH, dollars(70_000)));
			assert_ok!(MinterestProtocol::enable_is_collateral(bob(), DOT));
			assert_ok!(MinterestProtocol::enable_is_collateral(bob(), ETH));
			// exchange_rate = (150 - 0 + 0) / 150 = 1
			assert_eq!(
				liquidity_pool_state_rpc(DOT),
				Some(PoolState {
					exchange_rate: Rate::one(),
					borrow_rate: Rate::zero(),
					supply_rate: Rate::zero()
				})
			);

			System::set_block_number(20);

			assert_ok!(MinterestProtocol::borrow(bob(), DOT, dollars(100_000)));
			assert_eq!(LiquidityPools::pools(DOT).total_protocol_interest, Balance::zero());

			System::set_block_number(1000);
			assert_ok!(MinterestProtocol::repay(bob(), DOT, dollars(10_000)));
			assert_eq!(pool_balance(DOT), dollars(60_000));
			MinterestProtocol::on_finalize(1000);
			// Not reached threshold, pool balances should stay the same
			assert_eq!(
				LiquidityPools::pools(DOT).total_protocol_interest,
				441_000_000_000_000_000u128
			);

			System::set_block_number(10000000);

			assert_ok!(MinterestProtocol::repay(bob(), DOT, dollars(20_000)));
			assert_eq!(pool_balance(DOT), dollars(80_000));

			let total_protocol_interest: Balance = 3_645_120_550_951_706_945_733;
			assert_eq!(
				LiquidityPools::pools(DOT).total_protocol_interest,
				total_protocol_interest
			);

			let liquidity_pool_dot_balance = LiquidityPools::get_pool_available_liquidity(DOT);
			let liquidation_pool_dot_balance = LiquidationPools::get_pool_available_liquidity(DOT);

			// Threshold is reached. Transfer total_protocol_interest to liquidation pool
			MinterestProtocol::on_finalize(10000000);

			let transferred_to_liquidation_pool = total_protocol_interest;
			assert_eq!(LiquidityPools::pools(DOT).total_protocol_interest, Balance::zero());
			assert_eq!(
				LiquidityPools::get_pool_available_liquidity(DOT),
				liquidity_pool_dot_balance - transferred_to_liquidation_pool
			);
			assert_eq!(
				LiquidationPools::get_pool_available_liquidity(DOT),
				liquidation_pool_dot_balance + transferred_to_liquidation_pool
			);
		});
}
