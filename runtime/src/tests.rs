use crate::{
	AccountId, Balance, Block,
	CurrencyId::{self, DOT, ETH},
	LiquidationPoolsModuleId, LiquidityPoolsModuleId, Rate, Runtime, DOLLARS,
};
use controller::{ControllerData, PauseKeeper};
use controller_rpc_runtime_api::runtime_decl_for_ControllerApi::ControllerApi;
use controller_rpc_runtime_api::PoolState;
use frame_support::{assert_noop, assert_ok, parameter_types};
use liquidity_pools::Pool;
use minterest_model::MinterestModelData;
use minterest_primitives::{Operation, Price};
use orml_traits::MultiCurrency;
use pallet_traits::PoolsManager;
use sp_runtime::traits::{AccountIdConversion, Zero};
use sp_runtime::FixedPointNumber;

type MinterestProtocol = minterest_protocol::Module<Runtime>;
type LiquidityPools = liquidity_pools::Module<Runtime>;
type LiquidationPools = liquidation_pools::Module<Runtime>;
type Controller = controller::Module<Runtime>;
type Currencies = orml_currencies::Module<Runtime>;
type System = frame_system::Module<Runtime>;

parameter_types! {
	pub ALICE: AccountId = AccountId::from([1u8; 32]);
	pub BOB: AccountId = AccountId::from([2u8; 32]);
	pub CHARLIE: AccountId = AccountId::from([3u8; 32]);
}

struct ExtBuilder {
	endowed_accounts: Vec<(AccountId, CurrencyId, Balance)>,
}

impl Default for ExtBuilder {
	fn default() -> Self {
		Self {
			endowed_accounts: vec![
				// seed: initial assets. Initial MINT to pay for gas.
				(ALICE::get(), CurrencyId::MINT, 100_000 * DOLLARS),
				(ALICE::get(), CurrencyId::DOT, 100_000 * DOLLARS),
				(ALICE::get(), CurrencyId::ETH, 100_000 * DOLLARS),
				(BOB::get(), CurrencyId::MINT, 100_000 * DOLLARS),
				(BOB::get(), CurrencyId::DOT, 100_000 * DOLLARS),
				(BOB::get(), CurrencyId::ETH, 100_000 * DOLLARS),
				(CHARLIE::get(), CurrencyId::MINT, 100_000 * DOLLARS),
				(CHARLIE::get(), CurrencyId::DOT, 100_000 * DOLLARS),
				(CHARLIE::get(), CurrencyId::ETH, 100_000 * DOLLARS),
			],
		}
	}
}

impl ExtBuilder {
	pub fn build(self) -> sp_io::TestExternalities {
		let mut t = frame_system::GenesisConfig::default()
			.build_storage::<Runtime>()
			.unwrap();

		orml_tokens::GenesisConfig::<Runtime> {
			endowed_accounts: self.endowed_accounts,
		}
		.assimilate_storage(&mut t)
		.unwrap();

		liquidity_pools::GenesisConfig::<Runtime> {
			pools: vec![
				(
					CurrencyId::DOT,
					Pool {
						total_borrowed: Balance::zero(),
						borrow_index: Rate::one(),
						total_insurance: Balance::zero(),
					},
				),
				(
					CurrencyId::ETH,
					Pool {
						total_borrowed: Balance::zero(),
						borrow_index: Rate::one(),
						total_insurance: Balance::zero(),
					},
				),
			],
			pool_user_data: vec![],
		}
		.assimilate_storage(&mut t)
		.unwrap();

		controller::GenesisConfig::<Runtime> {
			controller_dates: vec![
				(
					CurrencyId::DOT,
					ControllerData {
						timestamp: 0,
						insurance_factor: Rate::saturating_from_rational(1, 10),  // 10%
						max_borrow_rate: Rate::saturating_from_rational(5, 1000), // 0.5%
						collateral_factor: Rate::saturating_from_rational(9, 10), // 90%
					},
				),
				(
					CurrencyId::ETH,
					ControllerData {
						timestamp: 0,
						insurance_factor: Rate::saturating_from_rational(1, 10),  // 10%
						max_borrow_rate: Rate::saturating_from_rational(5, 1000), // 0.5%
						collateral_factor: Rate::saturating_from_rational(9, 10), // 90%
					},
				),
			],
			pause_keepers: vec![
				(
					CurrencyId::DOT,
					PauseKeeper {
						deposit_paused: false,
						redeem_paused: false,
						borrow_paused: false,
						repay_paused: false,
					},
				),
				(
					CurrencyId::ETH,
					PauseKeeper {
						deposit_paused: false,
						redeem_paused: false,
						borrow_paused: false,
						repay_paused: false,
					},
				),
			],
		}
		.assimilate_storage(&mut t)
		.unwrap();

		minterest_model::GenesisConfig {
			minterest_model_dates: vec![
				(
					CurrencyId::DOT,
					MinterestModelData {
						kink: Rate::saturating_from_rational(8, 10),
						base_rate_per_block: Rate::zero(),
						multiplier_per_block: Rate::saturating_from_rational(9, 1_000_000_000), // 0.047304 PerYear
						jump_multiplier_per_block: Rate::saturating_from_rational(207, 1_000_000_000), // 1.09 PerYear
					},
				),
				(
					CurrencyId::ETH,
					MinterestModelData {
						kink: Rate::saturating_from_rational(8, 10),
						base_rate_per_block: Rate::zero(),
						multiplier_per_block: Rate::saturating_from_rational(9, 1_000_000_000), // 0.047304 PerYear
						jump_multiplier_per_block: Rate::saturating_from_rational(207, 1_000_000_000), // 1.09 PerYear
					},
				),
			],
		}
		.assimilate_storage(&mut t)
		.unwrap();

		accounts::GenesisConfig::<Runtime> {
			allowed_accounts: vec![(ALICE::get(), ())],
		}
		.assimilate_storage(&mut t)
		.unwrap();

		let mut ext: sp_io::TestExternalities = t.into();
		ext.execute_with(|| System::set_block_number(1));
		ext
	}
}

fn pool_balance(pool_id: CurrencyId) -> Balance {
	Currencies::free_balance(pool_id, &LiquidityPools::pools_account_id())
}

fn pool_total_insurance(pool_id: CurrencyId) -> Balance {
	LiquidityPools::pools(pool_id).total_insurance
}

fn liquidity_pool_state_rpc(currency_id: CurrencyId) -> Option<PoolState> {
	<Runtime as ControllerApi<Block>>::liquidity_pool_state(currency_id)
}

fn dollars(amount: u128) -> u128 {
	amount.saturating_mul(Price::accuracy())
}

fn alice() -> <Runtime as frame_system::Trait>::Origin {
	<Runtime as frame_system::Trait>::Origin::signed((ALICE::get()).clone())
}

fn bob() -> <Runtime as frame_system::Trait>::Origin {
	<Runtime as frame_system::Trait>::Origin::signed((BOB::get()).clone())
}

fn charlie() -> <Runtime as frame_system::Trait>::Origin {
	<Runtime as frame_system::Trait>::Origin::signed((CHARLIE::get()).clone())
}

#[test]
fn test_rates_using_rpc() {
	ExtBuilder::default().build().execute_with(|| {
		assert_ok!(Controller::deposit_insurance(alice(), DOT, dollars(100_000)));
		assert_ok!(Controller::deposit_insurance(alice(), ETH, dollars(100_000)));
		assert_eq!(pool_total_insurance(DOT), dollars(100_000));
		assert_eq!(pool_total_insurance(ETH), dollars(100_000));

		System::set_block_number(10);

		assert_ok!(MinterestProtocol::deposit_underlying(bob(), DOT, dollars(50_000)));
		assert_ok!(MinterestProtocol::deposit_underlying(bob(), ETH, dollars(70_000)));
		assert_ok!(MinterestProtocol::enable_as_collateral(bob(), DOT));
		assert_ok!(MinterestProtocol::enable_as_collateral(bob(), ETH));
		// exchange_rate = (150 - 100 + 0) / 50 = 1
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
		assert_ok!(MinterestProtocol::repay(bob(), DOT, dollars(30_000)));
		assert_eq!(pool_balance(DOT), dollars(80_000));
		// exchange_rate = (80 - 100 + 70) / 50 = 1
		assert_eq!(
			liquidity_pool_state_rpc(DOT),
			Some(PoolState {
				exchange_rate: Rate::one(),
				borrow_rate: Rate::from_inner(239_040_000_000),
				supply_rate: Rate::from_inner(301_190_400_000)
			})
		);

		System::set_block_number(30);

		assert_ok!(MinterestProtocol::deposit_underlying(charlie(), DOT, dollars(20_000)));
		assert_ok!(MinterestProtocol::deposit_underlying(charlie(), ETH, dollars(30_000)));
		// supply rate and borrow rate decreased
		assert_eq!(
			liquidity_pool_state_rpc(DOT),
			Some(PoolState {
				exchange_rate: Rate::from_inner(1_000_003_011_904_000_000),
				borrow_rate: Rate::from_inner(172_800_039_584),
				supply_rate: Rate::from_inner(155_520_072_800)
			})
		);

		System::set_block_number(40);

		assert_ok!(MinterestProtocol::enable_as_collateral(charlie(), DOT));
		assert_ok!(MinterestProtocol::enable_as_collateral(charlie(), ETH));
		assert_ok!(MinterestProtocol::borrow(charlie(), DOT, dollars(20_000)));
		// supply rate and borrow rate increased
		assert_eq!(
			liquidity_pool_state_rpc(DOT),
			Some(PoolState {
				exchange_rate: Rate::from_inner(1_000_004_567_109_412_126),
				borrow_rate: Rate::from_inner(220_114_178_542),
				supply_rate: Rate::from_inner(254_703_421_247)
			})
		);
	});
}

#[test]
fn demo_scenario_n2_without_insurance_should_work() {
	ExtBuilder::default().build().execute_with(|| {
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
			minterest_protocol::Error::<Runtime>::BorrowControllerRejection
		);
		System::set_block_number(4100);
		assert_ok!(MinterestProtocol::enable_as_collateral(charlie(), DOT));
		System::set_block_number(4200);
		assert_ok!(MinterestProtocol::enable_as_collateral(charlie(), ETH));
		System::set_block_number(4300);
		assert_ok!(Controller::pause_specific_operation(alice(), DOT, Operation::Borrow));
		System::set_block_number(4400);
		assert_noop!(
			MinterestProtocol::borrow(charlie(), DOT, 20_000 * DOLLARS),
			minterest_protocol::Error::<Runtime>::OperationPaused
		);
		System::set_block_number(5000);
		assert_ok!(Controller::unpause_specific_operation(alice(), DOT, Operation::Borrow));

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
