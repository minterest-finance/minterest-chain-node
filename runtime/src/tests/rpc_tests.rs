use super::*;
#[test]
fn demo_scenario_n2_without_interest_using_rpc_should_work() {
	ExtBuilder::default()
		.pool_initial(DOT)
		.pool_initial(ETH)
		.build()
		.execute_with(|| {
			// Set price = 2.00 USD for all pools.
			assert_ok!(set_oracle_price_for_all_pools(2));

			assert_ok!(MinterestProtocol::deposit_underlying(alice(), DOT, 100_000 * DOLLARS));
			System::set_block_number(200);
			assert_ok!(MinterestProtocol::deposit_underlying(alice(), ETH, 100_000 * DOLLARS));
			System::set_block_number(600);
			assert_ok!(MinterestProtocol::deposit_underlying(bob(), DOT, 80_000 * DOLLARS));
			System::set_block_number(1000);
			assert_ok!(MinterestProtocol::deposit_underlying(bob(), ETH, 50_000 * DOLLARS));
			System::set_block_number(2000);
			assert_ok!(MinterestProtocol::deposit_underlying(charlie(), DOT, 100_000 * DOLLARS));
			System::set_block_number(3000);
			assert_ok!(MinterestProtocol::deposit_underlying(charlie(), ETH, 50_000 * DOLLARS));
			System::set_block_number(4000);

			assert_noop!(
				MinterestProtocol::borrow(charlie(), DOT, 20_000 * DOLLARS),
				controller::Error::<Runtime>::InsufficientLiquidity
			);
			System::set_block_number(4100);
			assert_ok!(MinterestProtocol::enable_is_collateral(charlie(), DOT));
			System::set_block_number(4200);
			assert_ok!(MinterestProtocol::enable_is_collateral(charlie(), ETH));
			System::set_block_number(4300);
			assert_ok!(Controller::pause_operation(
				<Runtime as frame_system::Config>::Origin::root(),
				DOT,
				Operation::Borrow
			));
			System::set_block_number(4400);
			assert_noop!(
				MinterestProtocol::borrow(charlie(), DOT, 20_000 * DOLLARS),
				minterest_protocol::Error::<Runtime>::OperationPaused
			);
			System::set_block_number(5000);
			assert_ok!(Controller::resume_operation(
				<Runtime as frame_system::Config>::Origin::root(),
				DOT,
				Operation::Borrow
			));

			System::set_block_number(6000);
			assert_ok!(MinterestProtocol::borrow(charlie(), DOT, 20_000 * DOLLARS));
			assert_eq!(
				liquidity_pool_state_rpc(DOT),
				Some(PoolState {
					exchange_rate: Rate::one(),
					borrow_rate: Rate::from_inner(642857142),
					supply_rate: Rate::from_inner(41326530)
				})
			);
			System::set_block_number(7000);
			assert_ok!(MinterestProtocol::borrow(charlie(), ETH, 10_000 * DOLLARS));
			assert_eq!(
				liquidity_pool_state_rpc(ETH),
				Some(PoolState {
					exchange_rate: Rate::one(),
					borrow_rate: Rate::from_inner(450000000),
					supply_rate: Rate::from_inner(20250000)
				})
			);
			System::set_block_number(8000);
			assert_ok!(MinterestProtocol::borrow(charlie(), ETH, 20_000 * DOLLARS));
			assert_eq!(
				liquidity_pool_state_rpc(ETH),
				Some(PoolState {
					exchange_rate: Rate::from_inner(1000000020250000000),
					borrow_rate: Rate::from_inner(1350000175),
					supply_rate: Rate::from_inner(182250047)
				})
			);
			System::set_block_number(9000);
			assert_ok!(MinterestProtocol::borrow(charlie(), ETH, 70_000 * DOLLARS));
			assert_eq!(
				liquidity_pool_state_rpc(ETH),
				Some(PoolState {
					exchange_rate: Rate::from_inner(1000000202500050963),
					borrow_rate: Rate::from_inner(4500001113),
					supply_rate: Rate::from_inner(2025001001)
				})
			);
			System::set_block_number(10000);
			assert_ok!(MinterestProtocol::repay(charlie(), ETH, 50_000 * DOLLARS));
			assert_eq!(
				liquidity_pool_state_rpc(ETH),
				Some(PoolState {
					exchange_rate: Rate::from_inner(1000002227501463063),
					borrow_rate: Rate::from_inner(2250017263),
					supply_rate: Rate::from_inner(506257768)
				})
			);
			System::set_block_number(11000);
			assert_ok!(MinterestProtocol::borrow(charlie(), DOT, 50_000 * DOLLARS));
			assert_eq!(
				liquidity_pool_state_rpc(DOT),
				Some(PoolState {
					exchange_rate: Rate::from_inner(1000000206632652786),
					borrow_rate: Rate::from_inner(2250001601),
					supply_rate: Rate::from_inner(506250720)
				})
			);
			System::set_block_number(12000);
			assert_ok!(MinterestProtocol::repay(charlie(), DOT, 70_000 * DOLLARS));
			assert_eq!(
				liquidity_pool_state_rpc(DOT),
				Some(PoolState {
					exchange_rate: Rate::from_inner(1000000712883477935),
					borrow_rate: Rate::from_inner(7128),
					supply_rate: Rate::zero()
				})
			);
			System::set_block_number(13000);
			assert_ok!(MinterestProtocol::deposit_underlying(bob(), ETH, 10_000 * DOLLARS));
			System::set_block_number(13500);
			assert_ok!(MinterestProtocol::redeem(charlie(), ETH));
			System::set_block_number(14000);
			assert_ok!(MinterestProtocol::repay_all(charlie(), ETH));
			assert_eq!(
				liquidity_pool_state_rpc(ETH),
				Some(PoolState {
					exchange_rate: Rate::from_inner(1_000_004_371_397_298_691),
					borrow_rate: Rate::zero(),
					supply_rate: Rate::zero()
				})
			);
			System::set_block_number(15000);
			assert_ok!(MinterestProtocol::redeem_underlying(charlie(), DOT, 50_000 * DOLLARS));
			System::set_block_number(16000);
			assert_ok!(MinterestProtocol::repay_all(charlie(), DOT));
			assert_eq!(
				liquidity_pool_state_rpc(DOT),
				Some(PoolState {
					exchange_rate: Rate::from_inner(1_000_000_712_883_477_957),
					borrow_rate: Rate::zero(),
					supply_rate: Rate::zero()
				})
			);
			System::set_block_number(17000);
			assert_ok!(MinterestProtocol::redeem(charlie(), DOT));
			System::set_block_number(18000);
			assert_ok!(MinterestProtocol::redeem_underlying(bob(), DOT, 40_000 * DOLLARS));
			System::set_block_number(19000);
			assert_ok!(MinterestProtocol::redeem(bob(), DOT));
			assert_eq!(
				liquidity_pool_state_rpc(DOT),
				Some(PoolState {
					exchange_rate: Rate::from_inner(1_000_000_712_883_477_958),
					borrow_rate: Rate::zero(),
					supply_rate: Rate::zero()
				})
			);
			assert_ok!(MinterestProtocol::redeem(bob(), ETH));
			assert_eq!(
				liquidity_pool_state_rpc(ETH),
				Some(PoolState {
					exchange_rate: Rate::from_inner(1_000_004_371_397_298_690),
					borrow_rate: Rate::zero(),
					supply_rate: Rate::zero()
				})
			);
		});
}

#[test]
fn test_rates_using_rpc() {
	ExtBuilder::default()
		.pool_initial(DOT)
		.pool_initial(ETH)
		.build()
		.execute_with(|| {
			// Set price = 2.00 USD for all pools.
			assert_ok!(set_oracle_price_for_all_pools(2));

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
			// Bob borrow balance equal zero
			assert_eq!(
				get_user_borrow_per_asset_rpc(BOB::get(), DOT),
				Some(BalanceInfo {
					amount: Balance::zero()
				})
			);

			System::set_block_number(20);

			assert_ok!(MinterestProtocol::borrow(bob(), DOT, dollars(100_000)));
			assert_ok!(MinterestProtocol::repay(bob(), DOT, dollars(30_000)));
			assert_eq!(pool_balance(DOT), dollars(80_000));
			// exchange_rate = (80 - 0 + 70) / 150 = 1
			assert_eq!(
				liquidity_pool_state_rpc(DOT),
				Some(PoolState {
					exchange_rate: Rate::one(),
					borrow_rate: Rate::from_inner(4_200_000_000),
					supply_rate: Rate::from_inner(1_764_000_000)
				})
			);
			// Bob borrow balance = (100_000 DOT - 30_000 DOT)= 70_000 DOT
			assert_eq!(
				get_user_borrow_per_asset_rpc(BOB::get(), DOT),
				Some(BalanceInfo {
					amount: dollars(70_000)
				})
			);

			System::set_block_number(30);

			assert_ok!(MinterestProtocol::deposit_underlying(charlie(), DOT, dollars(20_000)));
			assert_ok!(MinterestProtocol::deposit_underlying(charlie(), ETH, dollars(30_000)));
			// supply rate and borrow rate decreased
			assert_eq!(
				liquidity_pool_state_rpc(DOT),
				Some(PoolState {
					exchange_rate: Rate::from_inner(1_000_000_017_640_000_000),
					borrow_rate: Rate::from_inner(3_705_882_450),
					supply_rate: Rate::from_inner(1_373_356_473)
				})
			);
			// Bob borrow balance = 70_000 DOT + accrued borrow
			assert_eq!(
				get_user_borrow_per_asset_rpc(BOB::get(), DOT),
				Some(BalanceInfo {
					amount: 70_000_002_940_000_000_000_000
				})
			);

			System::set_block_number(40);

			assert_ok!(MinterestProtocol::enable_is_collateral(charlie(), DOT));
			assert_ok!(MinterestProtocol::enable_is_collateral(charlie(), ETH));
			assert_ok!(MinterestProtocol::borrow(charlie(), DOT, dollars(20_000)));
			// supply rate and borrow rate increased
			assert_eq!(
				liquidity_pool_state_rpc(DOT),
				Some(PoolState {
					exchange_rate: Rate::from_inner(1_000_000_031_373_564_979),
					borrow_rate: Rate::from_inner(4_764_706_035),
					supply_rate: Rate::from_inner(2_270_242_360)
				})
			);
			// Charlie borrow balance = 20_000 DOT = 20_000 DOT
			assert_eq!(
				get_user_borrow_per_asset_rpc(CHARLIE::get(), DOT),
				Some(BalanceInfo {
					amount: dollars(20_000)
				})
			);
		});
}

#[test]
fn test_get_protocol_total_value_rpc() {
	ExtBuilder::default()
		.pool_initial(DOT)
		.pool_initial(ETH)
		.build()
		.execute_with(|| {
			assert_ok!(set_oracle_prices(vec![
				(DOT, Price::saturating_from_integer(2)),
				(ETH, Price::saturating_from_integer(3)),
			]));
			assert_ok!(lock_price(DOT));
			assert_ok!(lock_price(ETH));

			assert_ok!(MinterestProtocol::deposit_underlying(alice(), DOT, dollars(100_000)));
			// pool_total_supply: 100 DOT * 2
			// pool_total_borrow: 0
			// tvl: 100 DOT * 2
			// pool_total_protocol_interest: 0
			assert_eq!(
				get_protocol_total_values_rpc(),
				Some(ProtocolTotalValue {
					pool_total_supply_in_usd: dollars(200_000),
					pool_total_borrow_in_usd: Balance::zero(),
					tvl_in_usd: dollars(200_000),
					pool_total_interest_in_usd: Balance::zero(),
				})
			);

			assert_ok!(MinterestProtocol::deposit_underlying(alice(), ETH, dollars(100_000)));
			// pool_total_supply: 100 DOT * 2 + 100 ETH * 3
			// pool_total_borrow: 0
			// tvl: 100 DOT * 2 + 100 ETH * 3
			// pool_total_protocol_interest: 0
			assert_eq!(
				get_protocol_total_values_rpc(),
				Some(ProtocolTotalValue {
					pool_total_supply_in_usd: dollars(500_000),
					pool_total_borrow_in_usd: Balance::zero(),
					tvl_in_usd: dollars(500_000),
					pool_total_interest_in_usd: Balance::zero(),
				})
			);

			System::set_block_number(10);
			// Values haven`t changed due to no borrows
			// pool_total_supply: 100 DOT * 2 + 100 ETH * 3
			// pool_total_borrow: 0
			// tvl: 100 DOT * 2 + 100 ETH * 3
			// pool_total_protocol_interest: 0
			assert_eq!(
				get_protocol_total_values_rpc(),
				Some(ProtocolTotalValue {
					pool_total_supply_in_usd: dollars(500_000),
					pool_total_borrow_in_usd: Balance::zero(),
					tvl_in_usd: dollars(500_000),
					pool_total_interest_in_usd: Balance::zero(),
				})
			);

			assert_ok!(MinterestProtocol::deposit_underlying(bob(), DOT, dollars(50_000)));
			assert_ok!(MinterestProtocol::deposit_underlying(bob(), ETH, dollars(70_000)));
			assert_ok!(MinterestProtocol::enable_is_collateral(bob(), DOT));
			assert_ok!(MinterestProtocol::enable_is_collateral(bob(), ETH));
			// pool_total_supply: 150 DOT * 2 + 170 ETH * 3
			// pool_total_borrow: 0
			// tvl:  150 DOT * 2 + 170 ETH * 3
			// pool_total_protocol_interest: 0
			assert_eq!(
				get_protocol_total_values_rpc(),
				Some(ProtocolTotalValue {
					pool_total_supply_in_usd: dollars(810_000),
					pool_total_borrow_in_usd: Balance::zero(),
					tvl_in_usd: dollars(810_000),
					pool_total_interest_in_usd: Balance::zero(),
				})
			);

			System::set_block_number(20);

			assert_ok!(MinterestProtocol::borrow(bob(), DOT, dollars(70_000)));
			// Interest is 0 with regards to block delta is 0
			// pool_total_supply: 80 DOT * 2 + 170 ETH * 3
			// pool_total_borrow: 70 DOT * 2
			// tvl:  150 DOT * 2 + 170 ETH * 3
			// pool_total_protocol_interest: 0
			assert_eq!(
				get_protocol_total_values_rpc(),
				Some(ProtocolTotalValue {
					pool_total_supply_in_usd: dollars(670_000),
					pool_total_borrow_in_usd: dollars(140_000),
					tvl_in_usd: dollars(810_000),
					pool_total_interest_in_usd: Balance::zero(),
				})
			);

			System::set_block_number(30);
			/*
			Calculate interest_accumulated && pool_protocol_interest for DOT pool
			utilization_rate =
			 pool_borrow / (pool_supply - pool_protocol_interest + pool_borrow) = 70_000 / (80_000 - 0 + 70_000) = 0.4666
			borrow_rate =
			 utilization_rate * multiplier_per_block = 0.4666 * 0,000000009 = 0,0000000042
			dot_pool_interest_accumulated =
			 borrow_rate * block_delta * pool_borrow = 0,0000000042 * 10 * 70 000 = 0,00294
			dot_pool_protocol_interest = 0.1 * interest_accumulated = 0.000294

			pool_total_supply: 80 DOT * 2 + 170 ETH * 3 = 80 * 2 + 170 * 3
			pool_total_borrow: (70 + dot_pool_interest_accumulated) DOT * 2 = (70 + 0.00294) * 2
			tvl:  (150 + dot_pool_interest_accumulated - dot_pool_protocol_interest) DOT * 2 + 170 ETH * 3 = (150 + 0.00294 - 0.000294) * 2 + 170 * 3
			pool_total_interest: dot_pool_protocol_interest * 2 = 0.000294 * 2
			*/
			assert_eq!(
				get_protocol_total_values_rpc(),
				Some(ProtocolTotalValue {
					pool_total_supply_in_usd: dollars(670_000),
					pool_total_borrow_in_usd: 140_000_005_880_000_000_000_000,
					tvl_in_usd: 810_000_005_292_000_000_000_000,
					pool_total_interest_in_usd: 588_000_000_000_000
				})
			);

			assert_ok!(MinterestProtocol::repay_all(bob(), DOT));
			// In case when pool_borrow is zero, we subtract pool_protocol_interest from pool_supply.
			// pool_total_supply: (150 + interest_accumulated) DOT * 2 + 170 ETH * 3 =
			// (150 + 0.00294) * 2 + 170 * 3
			// pool_total_borrow: 0
			// tvl:  (150 + dot_pool_interest_accumulated - dot_pool_protocol_interest) DOT * 2 + 170 ETH * 3 =
			// (150 + 0.00294 - 0.000294) * 2 + 170 * 3
			// pool_total_interest: dot_pool_protocol_interest * 2 = 0.000294 * 2
			assert_eq!(
				get_protocol_total_values_rpc(),
				Some(ProtocolTotalValue {
					pool_total_supply_in_usd: 810_000_005_880_000_000_000_000,
					pool_total_borrow_in_usd: Balance::zero(),
					tvl_in_usd: 810_000_005_292_000_000_000_000,
					pool_total_interest_in_usd: 588_000_000_000_000
				})
			);

			// Do redeem to show case when pool_supply_wrap is equal to zero && pool has protocol_interest
			assert_ok!(MinterestProtocol::redeem(bob(), DOT));
			assert_ok!(MinterestProtocol::redeem(alice(), DOT));

			let dot_pool_protocol_interest = LiquidityPools::pools(DOT).total_protocol_interest;
			assert_eq!(dot_pool_protocol_interest, 294_000_000_000_000);

			// pool_total_supply: 170 ETH * 3 + dot_pool_protocol_interest * 2 = (170 * 3) + (0.000294 * 2)
			// pool_total_borrow: 0
			// tvl:  170 ETH * 3
			// pool_total_interest: dot_pool_protocol_interest * 2 = 0.000294 * 2
			assert_eq!(
				get_protocol_total_values_rpc(),
				Some(ProtocolTotalValue {
					pool_total_supply_in_usd: 510_000_000_588_000_000_000_000,
					pool_total_borrow_in_usd: Balance::zero(),
					tvl_in_usd: dollars(510_000),
					pool_total_interest_in_usd: 588_000_000_000_000
				})
			);

			assert_ok!(MinterestProtocol::enable_is_collateral(alice(), ETH));
			assert_ok!(MinterestProtocol::borrow(alice(), DOT, dot_pool_protocol_interest / 2));
			// pool_total_supply: 170 ETH * 3 + dot_pool_protocol_interest * 2 = (170 * 3) + 0.000147 * 2
			// pool_total_borrow: dot_pool_protocol_interest / 2 * 2 = 0.000294 / 2 * 2
			// tvl:  170 ETH * 3
			// pool_total_interest: dot_pool_protocol_interest * 2 = 0.000294 * 2
			assert_eq!(
				get_protocol_total_values_rpc(),
				Some(ProtocolTotalValue {
					pool_total_supply_in_usd: 510_000_000_294_000_000_000_000,
					pool_total_borrow_in_usd: 294_000_000_000_000,
					tvl_in_usd: dollars(510_000),
					pool_total_interest_in_usd: 588_000_000_000_000
				})
			);

			assert_ok!(MinterestProtocol::borrow(alice(), DOT, dot_pool_protocol_interest / 2));
			// pool_total_supply: 170 ETH * 3 = 170 * 3
			// pool_total_borrow: dot_pool_protocol_interest * 2 = 0.000294 * 2
			// tvl:  170 ETH * 3
			// pool_total_interest: dot_pool_protocol_interest * 2 = 0.000294 * 2
			assert_eq!(
				get_protocol_total_values_rpc(),
				Some(ProtocolTotalValue {
					pool_total_supply_in_usd: 510_000_000_000_000_000_000_000,
					pool_total_borrow_in_usd: 588_000_000_000_000,
					tvl_in_usd: dollars(510_000),
					pool_total_interest_in_usd: 588_000_000_000_000
				})
			);

			assert_ok!(MinterestProtocol::deposit_underlying(alice(), DOT, dollars(1)));
			// pool_total_supply: 170 ETH * 3 + 1 DOT * 2 = 170 * 3 + 1 * 2
			// pool_total_borrow: dot_pool_protocol_interest * 2 = 0.000294 * 2
			// tvl:  170 ETH * 3 + 1 DOT * 2
			// pool_total_interest: dot_pool_protocol_interest * 2 = 0.000294 * 2
			assert_eq!(
				get_protocol_total_values_rpc(),
				Some(ProtocolTotalValue {
					pool_total_supply_in_usd: 510_002_000_000_000_000_000_000,
					pool_total_borrow_in_usd: 588_000_000_000_000,
					tvl_in_usd: 510_002_000_000_000_000_000_000,
					pool_total_interest_in_usd: 588_000_000_000_000
				})
			);
		});
}

#[test]
fn test_get_utilization_rate_rpc() {
	ExtBuilder::default()
		.pool_initial(DOT)
		.pool_initial(ETH)
		.build()
		.execute_with(|| {
			// Set price = 2.00 USD for all pools.
			assert_ok!(set_oracle_price_for_all_pools(2));

			assert_ok!(MinterestProtocol::deposit_underlying(alice(), DOT, dollars(100_000)));
			assert_ok!(MinterestProtocol::deposit_underlying(alice(), ETH, dollars(100_000)));

			System::set_block_number(10);

			assert_ok!(MinterestProtocol::deposit_underlying(bob(), DOT, dollars(50_000)));
			assert_ok!(MinterestProtocol::deposit_underlying(bob(), ETH, dollars(70_000)));
			assert_ok!(MinterestProtocol::enable_is_collateral(bob(), DOT));
			assert_ok!(MinterestProtocol::enable_is_collateral(bob(), ETH));
			// No borrows -> utilization rates equal to 0
			assert_eq!(get_utilization_rate_rpc(DOT), Some(Rate::zero()));
			assert_eq!(get_utilization_rate_rpc(ETH), Some(Rate::zero()));

			System::set_block_number(20);

			assert_ok!(MinterestProtocol::borrow(bob(), DOT, dollars(70_000)));
			assert_eq!(pool_balance(DOT), dollars(80_000));
			// 70 / (80 + 70) = 0.466666667
			assert_eq!(
				get_utilization_rate_rpc(DOT),
				Some(Rate::from_inner(466_666_666_666_666_667))
			);
			assert_eq!(get_utilization_rate_rpc(ETH), Some(Rate::zero()));

			System::set_block_number(100);
			// Utilization rate grows with time as interest is accrued
			assert_eq!(
				get_utilization_rate_rpc(DOT),
				Some(Rate::from_inner(466_666_757_610_653_833))
			);
			assert_eq!(get_utilization_rate_rpc(ETH), Some(Rate::zero()));
		});
}

/// Test that returned values are changed after some blocks passed
#[test]
fn test_user_balances_using_rpc() {
	ExtBuilder::default()
		.pool_initial(DOT)
		.pool_initial(ETH)
		.build()
		.execute_with(|| {
			// Set price = 2.00 USD for all pools.
			assert_ok!(set_oracle_price_for_all_pools(2));

			assert_eq!(
				get_total_supply_and_borrowed_usd_balance_rpc(ALICE::get()),
				Some(UserPoolBalanceData {
					total_supply: dollars(0),
					total_borrowed: dollars(0)
				})
			);
			assert_eq!(
				get_total_supply_and_borrowed_usd_balance_rpc(BOB::get()),
				Some(UserPoolBalanceData {
					total_supply: dollars(0),
					total_borrowed: dollars(0)
				})
			);

			assert_ok!(MinterestProtocol::deposit_underlying(bob(), DOT, dollars(50_000)));
			assert_ok!(MinterestProtocol::deposit_underlying(bob(), ETH, dollars(70_000)));

			assert_eq!(
				get_total_supply_and_borrowed_usd_balance_rpc(ALICE::get()),
				Some(UserPoolBalanceData {
					total_supply: dollars(0),
					total_borrowed: dollars(0)
				})
			);
			assert_eq!(
				get_total_supply_and_borrowed_usd_balance_rpc(BOB::get()),
				Some(UserPoolBalanceData {
					total_supply: dollars(240_000),
					total_borrowed: dollars(0)
				})
			);
			assert_eq!(
				get_user_borrow_per_asset_rpc(BOB::get(), DOT),
				Some(BalanceInfo {
					amount: Balance::zero()
				})
			);

			assert_ok!(MinterestProtocol::enable_is_collateral(bob(), DOT));
			assert_ok!(MinterestProtocol::enable_is_collateral(bob(), ETH));
			System::set_block_number(20);

			assert_ok!(MinterestProtocol::borrow(bob(), DOT, dollars(50_000)));
			assert_eq!(
				get_total_supply_and_borrowed_usd_balance_rpc(BOB::get()),
				Some(UserPoolBalanceData {
					total_supply: dollars(240_000),
					total_borrowed: dollars(100_000)
				})
			);
			assert_eq!(
				get_user_borrow_per_asset_rpc(BOB::get(), DOT),
				Some(BalanceInfo {
					amount: dollars(50_000)
				})
			);

			assert_ok!(MinterestProtocol::repay(bob(), DOT, dollars(30_000)));
			assert_eq!(
				get_total_supply_and_borrowed_usd_balance_rpc(BOB::get()),
				Some(UserPoolBalanceData {
					total_supply: dollars(240_000),
					total_borrowed: dollars(40_000)
				})
			);
			assert_eq!(
				get_user_borrow_per_asset_rpc(BOB::get(), DOT),
				Some(BalanceInfo {
					amount: dollars(20_000)
				})
			);

			System::set_block_number(30);
			let account_data = get_total_supply_and_borrowed_usd_balance_rpc(BOB::get()).unwrap_or_default();
			assert!(account_data.total_supply > dollars(240_000));
			assert!(account_data.total_borrowed > dollars(40_000));
			assert!(get_user_borrow_per_asset_rpc(BOB::get(), DOT).unwrap().amount > dollars(20_000));
		});
}

#[test]
fn test_get_hypothetical_account_liquidity_rpc() {
	ExtBuilder::default()
		.pool_initial(DOT)
		.pool_initial(ETH)
		.build()
		.execute_with(|| {
			// Set price = 2.00 USD for all pools.
			assert_ok!(set_oracle_price_for_all_pools(2));

			assert_ok!(MinterestProtocol::deposit_underlying(bob(), DOT, dollars(50_000)));
			assert_ok!(MinterestProtocol::deposit_underlying(bob(), ETH, dollars(70_000)));
			System::set_block_number(20);

			assert_eq!(
				get_hypothetical_account_liquidity_rpc(ALICE::get()),
				Some(HypotheticalLiquidityData { liquidity: 0 })
			);
			assert_eq!(
				get_hypothetical_account_liquidity_rpc(BOB::get()),
				Some(HypotheticalLiquidityData { liquidity: 0 })
			);

			assert_ok!(MinterestProtocol::enable_is_collateral(bob(), DOT));
			assert_ok!(MinterestProtocol::enable_is_collateral(bob(), ETH));
			assert_ok!(MinterestProtocol::borrow(bob(), DOT, dollars(50_000)));

			// Check positive liquidity
			assert_eq!(
				get_hypothetical_account_liquidity_rpc(BOB::get()),
				Some(HypotheticalLiquidityData {
					liquidity: 116_000_000_000_000_000_000_000
				})
			);

			System::set_block_number(100_000_000);
			assert_ok!(MinterestProtocol::deposit_underlying(bob(), DOT, 1));
			assert_ok!(MinterestProtocol::deposit_underlying(bob(), ETH, 1));

			// Check negative liquidity
			assert_eq!(
				get_hypothetical_account_liquidity_rpc(BOB::get()),
				Some(HypotheticalLiquidityData {
					liquidity: -212_319_934_335_999_999_999_998
				})
			);
		});
}

/// Test that free balance has increased by a (total_supply - total_borrowed) after repay all and
/// redeem
#[test]
fn test_free_balance_is_ok_after_repay_all_and_redeem_using_balance_rpc() {
	ExtBuilder::default()
		.pool_initial(DOT)
		.pool_initial(ETH)
		.build()
		.execute_with(|| {
			// Set price = 2.00 USD for all pools.
			assert_ok!(set_oracle_price_for_all_pools(2));

			assert_ok!(MinterestProtocol::deposit_underlying(bob(), DOT, dollars(50_000)));
			System::set_block_number(50);
			assert_ok!(MinterestProtocol::enable_is_collateral(bob(), DOT));
			System::set_block_number(100);
			assert_ok!(MinterestProtocol::borrow(bob(), DOT, dollars(30_000)));
			System::set_block_number(150);
			assert_ok!(MinterestProtocol::repay(bob(), DOT, dollars(10_000)));
			System::set_block_number(200);

			let account_data_before_repay_all =
				get_total_supply_and_borrowed_usd_balance_rpc(BOB::get()).unwrap_or_default();

			let oracle_price = Prices::get_underlying_price(DOT).unwrap();

			let bob_balance_before_repay_all = Currencies::free_balance(DOT, &BOB::get());

			let expected_free_balance_bob = bob_balance_before_repay_all
				+ (Rate::from_inner(
					account_data_before_repay_all.total_supply - account_data_before_repay_all.total_borrowed,
				) / oracle_price)
					.into_inner();

			assert_ok!(MinterestProtocol::repay_all(bob(), DOT));
			assert_ok!(MinterestProtocol::redeem(bob(), DOT));

			assert_eq!(Currencies::free_balance(DOT, &BOB::get()), expected_free_balance_bob);
		})
}

/// Test that difference between total_borrowed returned by RPC before and after repay is equal to
/// repay amount
#[test]
fn test_total_borrowed_difference_is_ok_before_and_after_repay_using_balance_rpc() {
	ExtBuilder::default()
		.pool_initial(DOT)
		.pool_initial(ETH)
		.build()
		.execute_with(|| {
			// Set price = 2.00 USD for all pools.
			assert_ok!(set_oracle_price_for_all_pools(2));

			assert_ok!(MinterestProtocol::deposit_underlying(bob(), DOT, dollars(50_000)));
			System::set_block_number(50);
			assert_ok!(MinterestProtocol::enable_is_collateral(bob(), DOT));
			System::set_block_number(100);
			assert_ok!(MinterestProtocol::borrow(bob(), DOT, dollars(30_000)));
			System::set_block_number(150);

			let account_data_before_repay =
				get_total_supply_and_borrowed_usd_balance_rpc(BOB::get()).unwrap_or_default();

			let oracle_price = Prices::get_underlying_price(DOT).unwrap();

			assert_ok!(MinterestProtocol::repay(bob(), DOT, dollars(10_000)));
			let account_data_after_repay =
				get_total_supply_and_borrowed_usd_balance_rpc(BOB::get()).unwrap_or_default();

			assert_eq!(
				LiquidityPools::pool_user_data(DOT, BOB::get()).total_borrowed,
				(Rate::from_inner(account_data_after_repay.total_borrowed) / oracle_price).into_inner()
			);
			assert_eq!(
				dollars(10_000),
				(Rate::from_inner(account_data_before_repay.total_borrowed - account_data_after_repay.total_borrowed)
					/ oracle_price)
					.into_inner()
			);
		})
}

/// Test that difference between total_borrowed returned by RPC before and after borrow is equal to
/// borrow amount
#[test]
fn test_total_borrowed_difference_is_ok_before_and_after_borrow_using_balance_rpc() {
	ExtBuilder::default()
		.pool_initial(DOT)
		.pool_initial(ETH)
		.build()
		.execute_with(|| {
			// Set price = 2.00 USD for all pools.
			assert_ok!(set_oracle_price_for_all_pools(2));

			assert_ok!(MinterestProtocol::deposit_underlying(bob(), DOT, dollars(50_000)));
			System::set_block_number(50);
			assert_ok!(MinterestProtocol::enable_is_collateral(bob(), DOT));
			System::set_block_number(100);

			let account_data_before_borrow =
				get_total_supply_and_borrowed_usd_balance_rpc(BOB::get()).unwrap_or_default();

			let oracle_price = Prices::get_underlying_price(DOT).unwrap();

			assert_ok!(MinterestProtocol::borrow(bob(), DOT, dollars(30_000)));
			let account_data_after_borrow =
				get_total_supply_and_borrowed_usd_balance_rpc(BOB::get()).unwrap_or_default();

			assert_eq!(
				LiquidityPools::pool_user_data(DOT, BOB::get()).total_borrowed,
				(Rate::from_inner(account_data_after_borrow.total_borrowed) / oracle_price).into_inner()
			);
			assert_eq!(
				dollars(30_000),
				(Rate::from_inner(
					account_data_after_borrow.total_borrowed - account_data_before_borrow.total_borrowed
				) / oracle_price)
					.into_inner()
			);
		})
}

/// Test that difference between total_supply returned by RPC before and after deposit_underlying is
/// equal to deposit amount
#[test]
fn test_total_borrowed_difference_is_ok_before_and_after_deposit_using_balance_rpc() {
	ExtBuilder::default()
		.pool_initial(DOT)
		.pool_initial(ETH)
		.build()
		.execute_with(|| {
			// Set price = 2.00 USD for all pools.
			assert_ok!(set_oracle_price_for_all_pools(2));

			assert_ok!(MinterestProtocol::deposit_underlying(bob(), DOT, dollars(50_000)));
			System::set_block_number(50);
			assert_ok!(MinterestProtocol::enable_is_collateral(bob(), DOT));
			System::set_block_number(100);

			let account_data_before_deposit =
				get_total_supply_and_borrowed_usd_balance_rpc(BOB::get()).unwrap_or_default();

			let oracle_price = Prices::get_underlying_price(DOT).unwrap();

			assert_ok!(MinterestProtocol::deposit_underlying(bob(), DOT, dollars(30_000)));
			let account_data_after_deposit =
				get_total_supply_and_borrowed_usd_balance_rpc(BOB::get()).unwrap_or_default();

			assert_eq!(
				dollars(30_000),
				(Rate::from_inner(account_data_after_deposit.total_supply - account_data_before_deposit.total_supply)
					/ oracle_price)
					.into_inner()
			);
		})
}

#[test]
fn is_admin_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		assert_eq!(is_admin_rpc(ALICE::get()), Some(false));
		assert_ok!(MinterestCouncilMembership::add_member(
			<Runtime as frame_system::Config>::Origin::root(),
			ALICE::get()
		));
		assert_eq!(is_admin_rpc(ALICE::get()), Some(true));
		assert_eq!(is_admin_rpc(BOB::get()), Some(false));
	})
}

// Test RPC behavior after changing state by standard protocol operations and changing oracle
// price for collateral asset.
#[test]
fn get_user_total_collateral_rpc_should_work() {
	ExtBuilder::default()
		.pool_initial(DOT)
		.pool_initial(ETH)
		.pool_initial(BTC)
		.pool_initial(KSM)
		.build()
		.execute_with(|| {
			// Set price = 2.00 USD for all pools.
			assert_ok!(set_oracle_price_for_all_pools(2));

			assert_ok!(MinterestProtocol::deposit_underlying(alice(), DOT, dollars(50_000)));
			assert_ok!(MinterestProtocol::deposit_underlying(alice(), BTC, dollars(50_000)));
			assert_ok!(MinterestProtocol::enable_is_collateral(alice(), DOT));
			assert_eq!(get_user_total_collateral_rpc(ALICE::get()), dollars(90_000));

			run_to_block(50);

			assert_ok!(MinterestProtocol::deposit_underlying(alice(), ETH, dollars(50_000)));
			assert_ok!(MinterestProtocol::enable_is_collateral(alice(), ETH));
			assert_eq!(get_user_total_collateral_rpc(ALICE::get()), dollars(180_000));

			run_to_block(100);

			assert_ok!(MinterestProtocol::deposit_underlying(bob(), DOT, dollars(100_000)));
			assert_ok!(MinterestProtocol::enable_is_collateral(bob(), DOT));
			assert_ok!(MinterestProtocol::borrow(bob(), DOT, 70_000 * DOLLARS));

			assert_eq!(get_user_total_collateral_rpc(ALICE::get()), dollars(180_000));
			assert_eq!(get_user_total_collateral_rpc(BOB::get()), dollars(180_000));

			run_to_block(200);

			assert_eq!(
				get_user_total_collateral_rpc(ALICE::get()),
				180_000_015_876_000_000_000_000
			);
			assert_eq!(
				get_user_total_collateral_rpc(BOB::get()),
				180_000_031_752_000_000_000_000
			);

			run_to_block(300);

			assert_ok!(MinterestProtocol::disable_is_collateral(alice(), ETH));

			run_to_block(400);

			assert_eq!(
				get_user_total_collateral_rpc(ALICE::get()),
				90_000_047_628_000_000_000_000
			);
			assert_eq!(
				get_user_total_collateral_rpc(BOB::get()),
				180_000_095_256_000_000_000_000
			);

			run_to_block(500);

			assert_ok!(MinterestProtocol::transfer_wrapped(
				alice(),
				BOB::get(),
				MDOT,
				dollars(50_000)
			));

			run_to_block(600);

			let expected_bob_collateral = 180_000_238_140_000_000_000_000 + dollars(90_000);

			assert_eq!(get_user_total_collateral_rpc(ALICE::get()), Balance::zero());
			assert_eq!(get_user_total_collateral_rpc(BOB::get()), expected_bob_collateral);

			// Change the price from 2 USD to 4 USD for DOT.
			assert_ok!(MinterestOracle::feed_values(
				origin_of(ORACLE1::get().clone()),
				vec![(DOT, Rate::saturating_from_integer(4))]
			));
			assert_ok!(Prices::unlock_price(origin_root(), DOT));

			assert_eq!(get_user_total_collateral_rpc(BOB::get()), expected_bob_collateral * 2);
		})
}

/// This test checks get_user_underlying_balance_per_asset RPC.
///
/// Test scenario:
/// - Bob deposits 90 to ETH pool
/// - Alice deposits 100 to ETH pool
/// - Balance converted to underlying is equal to deposited amount
/// - Bob borrows 40 from ETH pool
/// - Balance converted to underlying is still equal to deposited amount
/// - Jump to block #100
/// - Balance converted to underlying has increased
/// - Alice redeems money from protocol
/// - Alice ETH balance increased by a value that was previously returned by RPC
#[test]
fn test_get_user_underlying_balance_per_asset_rpc() {
	ExtBuilder::default().pool_initial(ETH).build().execute_with(|| {
		assert_ok!(MinterestProtocol::deposit_underlying(bob(), ETH, dollars(90_000)));
		// Alice deposited ALL ETH tokens to protocol
		assert_ok!(MinterestProtocol::deposit_underlying(alice(), ETH, dollars(100_000)));
		assert_eq!(
			get_user_underlying_balance_per_asset_rpc(ALICE::get(), ETH),
			Some(BalanceInfo {
				amount: dollars(100_000)
			})
		);
		// Bob makes a borrow to update exchange rate
		assert_ok!(MinterestProtocol::enable_is_collateral(bob(), ETH));
		assert_ok!(MinterestProtocol::borrow(bob(), ETH, dollars(40_000)));
		// Alice Underlying balance hasn`t changed in the block when first borrow occurred
		assert_eq!(
			get_user_underlying_balance_per_asset_rpc(ALICE::get(), ETH),
			Some(BalanceInfo {
				amount: dollars(100_000)
			})
		);
		// Skip some blocks to accrue interest. Bob repay his borrow
		System::set_block_number(100);
		// 3554127423600000 - this is interest that alice earn for depositing
		let alice_balance_after_borrowing = dollars(100_000) + 3554127423600000;
		assert_eq!(
			get_user_underlying_balance_per_asset_rpc(ALICE::get(), ETH),
			Some(BalanceInfo {
				amount: alice_balance_after_borrowing
			})
		);
		// Converts ALL mTokens into a specified quantity of the underlying asset
		assert_ok!(MinterestProtocol::redeem(alice(), ETH));
		// Check that value got by rpc and value after redeem the same
		assert_eq!(
			Currencies::free_balance(ETH, &ALICE::get()),
			alice_balance_after_borrowing
		);
	})
}

#[test]
fn get_all_locked_prices_rpc_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		assert_ok!(set_oracle_price_for_all_pools(10_000));

		CurrencyId::get_enabled_tokens_in_protocol(minterest_primitives::currency::CurrencyType::UnderlyingAsset)
			.into_iter()
			.for_each(|pool_id| {
				assert_ok!(Prices::lock_price(origin_root(), pool_id));
			});

		// Check that locked prices are returned
		// By default all price set to 10_000
		let locked_prices = get_all_locked_prices();
		for (_currency_id, price) in locked_prices {
			assert_eq!(price, Some(Price::saturating_from_integer(10_000)));
		}
		// Unlock price for DOT, check that None will be returned for this currency
		assert_ok!(unlock_price(DOT));
		let locked_prices = get_all_locked_prices();
		for (currency_id, price) in locked_prices {
			match currency_id {
				DOT => {
					assert_eq!(price, None);
				}
				ETH | BTC | KSM => {
					assert_eq!(price, Some(Price::saturating_from_integer(10_000)));
				}
				_ => panic!("Unexpected token!"),
			}
		}
	});
}

#[test]
// Check that fresh prices will be returned
// Prices set to 10_000
fn get_all_freshest_prices_rpc_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		assert_ok!(set_oracle_price_for_all_pools(10_000));
		let fresh_prices = get_all_freshest_prices();
		for (_currency_id, price) in fresh_prices {
			assert_eq!(price, Some(Price::saturating_from_integer(10_000)));
		}
	});
}

#[test]
fn get_unclaimed_mnt_balance_should_work() {
	ExtBuilder::default()
		.mnt_account_balance(1_000_000 * DOLLARS)
		.enable_minting_for_all_pools(10 * DOLLARS)
		.pool_initial(DOT)
		.pool_initial(KSM)
		.pool_initial(ETH)
		.pool_initial(BTC)
		.build()
		.execute_with(|| {
			// Set initial state of pools for distribution MNT tokens.
			assert_ok!(MinterestProtocol::deposit_underlying(bob(), DOT, 100_000 * DOLLARS));
			assert_ok!(MinterestProtocol::enable_is_collateral(bob(), DOT));
			assert_ok!(MinterestProtocol::borrow(bob(), DOT, 50_000 * DOLLARS));

			run_to_block(10);

			// ALice deposits DOT and enables her DOT pool as a collateral.
			assert_ok!(MinterestProtocol::deposit_underlying(alice(), DOT, 50_000 * DOLLARS));
			assert_ok!(MinterestProtocol::enable_is_collateral(alice(), DOT));

			run_to_block(15);

			// Calculation of the balance of Alice in MNT tokens (only supply distribution):
			// supplier_mnt_accrued = previous_balance + speed_DOT * block_delta * alice_supply / total_supply;
			// supplier_mnt_accrued = 0 + 10 * 5 * 50 / 150 = 16.66 MNT;
			assert_eq!(get_unclaimed_mnt_balance_rpc(ALICE::get()), 16_666_666_464_166_653_690);

			assert_ok!(MinterestProtocol::deposit_underlying(alice(), DOT, 10_000 * DOLLARS));

			run_to_block(20);

			// Calculation of the balance of Alice in MNT tokens (only supply distribution):
			// supplier_mnt_accrued = previous_balance + speed_DOT * block_delta * alice_supply / total_supply;
			// supplier_mnt_accrued = 0 + 10 * 5 * 60 / 160 = 18.75 MNT;
			assert_eq!(get_unclaimed_mnt_balance_rpc(ALICE::get()), 18_749_999_777_636_673_218);
			assert_eq!(
				Currencies::free_balance(MNT, &ALICE::get()),
				100_035_416_666_241_803_326_908
			);
			// In the test environment, the test storage changes.
			assert_eq!(get_unclaimed_mnt_balance_rpc(ALICE::get()), Balance::zero());

			assert_ok!(MinterestProtocol::borrow(alice(), DOT, 20_000 * DOLLARS));

			run_to_block(30);

			assert_eq!(get_unclaimed_mnt_balance_rpc(ALICE::get()), 66_071_426_707_059_137_419);
			// In the test environment, the test storage changes.
			assert_eq!(get_unclaimed_mnt_balance_rpc(ALICE::get()), Balance::zero());

			assert_ok!(MinterestProtocol::deposit_underlying(alice(), DOT, 10_000 * DOLLARS));

			run_to_block(40);

			assert_eq!(get_unclaimed_mnt_balance_rpc(ALICE::get()), 69_747_897_200_110_984_655);
			// In the test environment, the test storage changes.
			assert_eq!(get_unclaimed_mnt_balance_rpc(ALICE::get()), Balance::zero());
		})
}

#[test]
fn get_mnt_borrow_and_supply_rates_should_work() {
	ExtBuilder::default()
		.mnt_enabled_pools(vec![
			(DOT, (5 * DOLLARS) / 2),
			(ETH, 5 * DOLLARS),
			(BTC, (5 * DOLLARS) / 2),
		])
		.pool_initial(DOT)
		.pool_initial(ETH)
		.pool_initial(BTC)
		.pool_initial(KSM)
		.build()
		.execute_with(|| {
			assert_ok!(MinterestProtocol::deposit_underlying(alice(), DOT, 10_000 * DOLLARS));
			assert_ok!(MinterestProtocol::deposit_underlying(alice(), ETH, 15_000 * DOLLARS));
			assert_ok!(MinterestProtocol::deposit_underlying(bob(), BTC, 25_000 * DOLLARS));

			LiquidityPools::enable_is_collateral_internal(&ALICE::get(), DOT);
			LiquidityPools::enable_is_collateral_internal(&ALICE::get(), ETH);
			LiquidityPools::enable_is_collateral_internal(&BOB::get(), BTC);

			assert_ok!(MinterestProtocol::borrow(alice(), DOT, 5_000 * DOLLARS));
			assert_ok!(MinterestProtocol::borrow(bob(), ETH, 10_000 * DOLLARS));
			assert_ok!(MinterestProtocol::borrow(alice(), BTC, 5_000 * DOLLARS));

			run_to_block(5);
			assert_eq!(MntToken::mnt_speeds(DOT), 2_500_000_000_000_000_000);
			assert_eq!(MntToken::mnt_speeds(ETH), 5_000_000_000_000_000_000);
			assert_eq!(MntToken::mnt_speeds(BTC), 2_500_000_000_000_000_000);

			// Borrow and Supply rates per block
			// Prices: DOT[0] = 2 USD, ETH[1] = 2 USD, BTC[3] = 2 USD, MNT[4] = 4 USD
			// Expected borrow_rate = mnt_speed * mnt_price / (total_borrow * price):
			// DOT: 2.5 * 4 / (5000 * 2) = 0.001
			// ETH: 5 * 4 / (10000 * 2) = 0.001
			// BTC: 2.5 * 4 / (5000 * 2) = 0.001
			// Expected supply_rate = mnt_speed * mnt_price / (total_supply * price):
			// DOT: 2.5 * 4 / (10000 * 2) = 0.0005
			// ETH: 5 * 4 / (15000 * 2) = 0.00066
			// BTC: 2.5 * 4 / (25000 * 2) = 0.0002
			assert_eq!(
				get_mnt_borrow_and_supply_rates(DOT),
				(
					Rate::saturating_from_rational(1, 1000),
					Rate::saturating_from_rational(5, 10000)
				)
			);
			assert_eq!(
				get_mnt_borrow_and_supply_rates(ETH),
				(
					Rate::saturating_from_rational(1, 1000),
					Rate::from_inner(666_666_666_666_666)
				)
			);
			assert_eq!(
				get_mnt_borrow_and_supply_rates(BTC),
				(
					Rate::saturating_from_rational(1, 1000),
					Rate::saturating_from_rational(2, 10000)
				)
			);
			// Check that (0,0) will be returned for pool with 0 borrow
			assert_ok!(MinterestProtocol::deposit_underlying(alice(), KSM, 10_000 * DOLLARS));
			run_to_block(7);
			assert_eq!(
				get_mnt_borrow_and_supply_rates(KSM),
				(Rate::saturating_from_integer(0), Rate::saturating_from_integer(0))
			);
		});
}

#[test]
fn pool_exists_should_work() {
	ExtBuilder::default().pool_initial(DOT).build().execute_with(|| {
		assert_eq!(pool_exists_rpc(DOT), true);
		assert_eq!(pool_exists_rpc(ETH), false);
	});
}
