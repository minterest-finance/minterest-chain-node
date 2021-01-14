#![cfg(test)]

use frame_support::{impl_outer_event, impl_outer_origin, parameter_types};
use liquidity_pools::{Pool, PoolUserData};
pub use minterest_primitives::{Balance, CurrencyId, Rate};
use orml_currencies::Currency;
use sp_core::H256;
use sp_runtime::{
	testing::Header,
	traits::{IdentityLookup, Zero},
	ModuleId, Perbill,
};

use super::*;

mod controller {
	pub use crate::Event;
}

impl_outer_origin! {
	pub enum Origin for Runtime {}
}

impl_outer_event! {
	pub enum TestEvent for Runtime {
		frame_system<T>,
		orml_tokens<T>,
		orml_currencies<T>,
		liquidity_pools,
		controller,
		oracle,
		accounts<T>,
	}
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Runtime;

// For testing the module, we construct most of a mock runtime. This means
// first constructing a configuration type (`Runtime`) which `impl`s each of the
// configuration traits of modules we want to use.
parameter_types! {
	pub const BlockHashCount: u64 = 250;
	pub const MaximumBlockWeight: u32 = 1024;
	pub const MaximumBlockLength: u32 = 2 * 1024;
	pub const AvailableBlockRatio: Perbill = Perbill::from_percent(75);
}

pub type AccountId = u32;
impl system::Trait for Runtime {
	type BaseCallFilter = ();
	type Origin = Origin;
	type Call = ();
	type Index = u64;
	type BlockNumber = BlockNumber;
	type Hash = H256;
	type Hashing = ::sp_runtime::traits::BlakeTwo256;
	type AccountId = AccountId;
	type Lookup = IdentityLookup<Self::AccountId>;
	type Header = Header;
	type Event = TestEvent;
	type BlockHashCount = BlockHashCount;
	type MaximumBlockWeight = MaximumBlockWeight;
	type DbWeight = ();
	type BlockExecutionWeight = ();
	type ExtrinsicBaseWeight = ();
	type MaximumExtrinsicWeight = MaximumBlockWeight;
	type MaximumBlockLength = MaximumBlockLength;
	type AvailableBlockRatio = AvailableBlockRatio;
	type Version = ();
	type PalletInfo = ();
	type AccountData = ();
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type SystemWeightInfo = ();
}

parameter_types! {
	pub const ExistentialDeposit: u64 = 1;
}

type Amount = i128;

impl orml_tokens::Trait for Runtime {
	type Event = TestEvent;
	type Balance = Balance;
	type Amount = Amount;
	type CurrencyId = CurrencyId;
	type OnReceived = ();
	type WeightInfo = ();
}

parameter_types! {
	pub const GetNativeCurrencyId: CurrencyId = CurrencyId::MINT;
}

type NativeCurrency = Currency<Runtime, GetNativeCurrencyId>;

impl orml_currencies::Trait for Runtime {
	type Event = TestEvent;
	type MultiCurrency = orml_tokens::Module<Runtime>;
	type NativeCurrency = NativeCurrency;
	type GetNativeCurrencyId = GetNativeCurrencyId;
	type WeightInfo = ();
}

parameter_types! {
	pub const LiquidityPoolsModuleId: ModuleId = ModuleId(*b"min/pool");
}

impl liquidity_pools::Trait for Runtime {
	type Event = TestEvent;
	type MultiCurrency = orml_tokens::Module<Runtime>;
	type ModuleId = LiquidityPoolsModuleId;
}

impl oracle::Trait for Runtime {
	type Event = TestEvent;
}

parameter_types! {
	pub const MaxMembers: u32 = MAX_MEMBERS;
}

impl accounts::Trait for Runtime {
	type Event = TestEvent;
	type MaxMembers = MaxMembers;
}

parameter_types! {
	pub const InitialExchangeRate: Rate = Rate::from_inner(1_000_000_000_000_000_000);
	pub const BlocksPerYear: u128 = 5256000u128;
	pub MTokensId: Vec<CurrencyId> = vec![
			CurrencyId::MDOT,
			CurrencyId::MKSM,
			CurrencyId::MBTC,
			CurrencyId::METH,
		];
	pub UnderlyingAssetId: Vec<CurrencyId> = vec![
		CurrencyId::DOT,
		CurrencyId::KSM,
		CurrencyId::BTC,
		CurrencyId::ETH,
	];
}

impl Trait for Runtime {
	type Event = TestEvent;
	type InitialExchangeRate = InitialExchangeRate;
	type MTokensId = MTokensId;
	type BlocksPerYear = BlocksPerYear;
	type UnderlyingAssetId = UnderlyingAssetId;
}

pub type BlockNumber = u64;

pub type Controller = Module<Runtime>;
pub type TestPools = liquidity_pools::Module<Runtime>;
pub type System = frame_system::Module<Runtime>;
pub type Currencies = orml_currencies::Module<Runtime>;
pub const MAX_MEMBERS: u32 = 16;

pub struct ExtBuilder {
	endowed_accounts: Vec<(AccountId, CurrencyId, Balance)>,
	pools: Vec<(CurrencyId, Pool)>,
	pool_user_data: Vec<(AccountId, CurrencyId, PoolUserData)>,
}

impl Default for ExtBuilder {
	fn default() -> Self {
		Self {
			endowed_accounts: vec![],
			pools: vec![],
			pool_user_data: vec![],
		}
	}
}

pub const ALICE: AccountId = 1;
pub fn alice() -> Origin {
	Origin::signed(ALICE)
}
pub const BOB: AccountId = 2;
pub fn bob() -> Origin {
	Origin::signed(BOB)
}
pub const ONE_HUNDRED: Balance = 100;
pub const BLOCKS_PER_YEAR: u128 = 5_256_000;

impl ExtBuilder {
	pub fn user_balance(mut self, user: AccountId, currency_id: CurrencyId, balance: Balance) -> Self {
		self.endowed_accounts.push((user, currency_id, balance));
		self
	}

	pub fn pool_balance(mut self, currency_id: CurrencyId, balance: Balance) -> Self {
		self.endowed_accounts
			.push((TestPools::pools_account_id(), currency_id, balance));
		self
	}

	pub fn pool_total_borrowed(mut self, pool_id: CurrencyId, total_borrowed: Balance) -> Self {
		self.pools.push((
			pool_id,
			Pool {
				current_interest_rate: Rate::from_inner(0),
				total_borrowed,
				borrow_index: Rate::saturating_from_rational(1, 1),
				current_exchange_rate: Rate::from_inner(1),
				total_insurance: Balance::zero(),
			},
		));
		self
	}

	pub fn pool_total_insurance(mut self, pool_id: CurrencyId, total_insurance: Balance) -> Self {
		self.endowed_accounts
			.push((TestPools::pools_account_id(), pool_id, total_insurance));
		self.pools.push((
			pool_id,
			Pool {
				current_interest_rate: Rate::from_inner(0),
				total_borrowed: Balance::zero(),
				borrow_index: Rate::saturating_from_rational(1, 1),
				current_exchange_rate: Rate::from_inner(1),
				total_insurance,
			},
		));
		self
	}

	pub fn pool_mock(mut self, pool_id: CurrencyId) -> Self {
		self.pools.push((
			pool_id,
			Pool {
				current_interest_rate: Rate::from_inner(0),
				total_borrowed: Balance::zero(),
				borrow_index: Rate::saturating_from_rational(2, 1),
				current_exchange_rate: Rate::saturating_from_rational(1, 1),
				total_insurance: Balance::zero(),
			},
		));
		self
	}

	pub fn pool_user_data(
		mut self,
		user: AccountId,
		pool_id: CurrencyId,
		total_borrowed: Balance,
		interest_index: Rate,
		collateral: bool,
	) -> Self {
		self.pool_user_data.push((
			user,
			pool_id,
			PoolUserData {
				total_borrowed,
				interest_index,
				collateral,
			},
		));
		self
	}

	pub fn alice_deposit_60_dot(self) -> Self {
		self.user_balance(ALICE, CurrencyId::DOT, 40)
			.user_balance(ALICE, CurrencyId::MDOT, 60)
			.pool_balance(CurrencyId::DOT, 60)
			.pool_mock(CurrencyId::DOT)
			.pool_user_data(ALICE, CurrencyId::DOT, 0, Rate::from_inner(0), false)
	}

	pub fn alice_deposit_20_eth(self) -> Self {
		self.user_balance(ALICE, CurrencyId::ETH, 80)
			.user_balance(ALICE, CurrencyId::METH, 20)
			.pool_balance(CurrencyId::ETH, 20)
			.pool_mock(CurrencyId::ETH)
			.pool_user_data(ALICE, CurrencyId::ETH, 0, Rate::from_inner(0), false)
	}

	pub fn build(self) -> sp_io::TestExternalities {
		let mut t = frame_system::GenesisConfig::default()
			.build_storage::<Runtime>()
			.unwrap();

		orml_tokens::GenesisConfig::<Runtime> {
			endowed_accounts: self.endowed_accounts,
		}
		.assimilate_storage(&mut t)
		.unwrap();

		GenesisConfig::<Runtime> {
			controller_dates: vec![
				(
					CurrencyId::DOT,
					ControllerData {
						timestamp: 0,
						borrow_rate: Rate::from_inner(0),
						insurance_factor: Rate::saturating_from_rational(1, 10),
						max_borrow_rate: Rate::saturating_from_rational(5, 1000),
						kink: Rate::saturating_from_rational(8, 10),
						base_rate_per_block: Rate::from_inner(0),
						multiplier_per_block: Rate::saturating_from_rational(9, 1_000_000_000), // 0.047304 PerYear
						jump_multiplier_per_block: Rate::saturating_from_rational(207, 1_000_000_000), // 1.09 PerYear
						collateral_factor: Rate::saturating_from_rational(9, 10),               // 90%
					},
				),
				(
					CurrencyId::ETH,
					ControllerData {
						timestamp: 0,
						borrow_rate: Rate::from_inner(0),
						insurance_factor: Rate::saturating_from_rational(1, 10),
						max_borrow_rate: Rate::saturating_from_rational(5, 1000),
						kink: Rate::saturating_from_rational(8, 10),
						base_rate_per_block: Rate::from_inner(0),
						multiplier_per_block: Rate::saturating_from_rational(9, 1_000_000_000), // 0.047304 PerYear
						jump_multiplier_per_block: Rate::saturating_from_rational(207, 1_000_000_000), // 1.09 PerYear
						collateral_factor: Rate::saturating_from_rational(9, 10),               // 90%
					},
				),
				(
					CurrencyId::BTC,
					ControllerData {
						timestamp: 0,
						borrow_rate: Rate::from_inner(0),
						insurance_factor: Rate::saturating_from_rational(1, 10),
						max_borrow_rate: Rate::saturating_from_rational(5, 1000),
						kink: Rate::saturating_from_rational(8, 10),
						base_rate_per_block: Rate::from_inner(0),
						multiplier_per_block: Rate::saturating_from_rational(9, 1_000_000_000), // 0.047304 PerYear
						jump_multiplier_per_block: Rate::saturating_from_rational(207, 1_000_000_000), // 1.09 PerYear
						collateral_factor: Rate::saturating_from_rational(9, 10),               // 90%
					},
				),
			],
			pause_keepers: vec![
				(
					CurrencyId::ETH,
					PauseKeeper {
						deposit_paused: false,
						redeem_paused: false,
						borrow_paused: false,
						repay_paused: false,
					},
				),
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
					CurrencyId::KSM,
					PauseKeeper {
						deposit_paused: true,
						redeem_paused: true,
						borrow_paused: true,
						repay_paused: true,
					},
				),
				(
					CurrencyId::BTC,
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

		liquidity_pools::GenesisConfig::<Runtime> {
			pools: self.pools,
			pool_user_data: self.pool_user_data,
		}
		.assimilate_storage(&mut t)
		.unwrap();

		accounts::GenesisConfig::<Runtime> {
			allowed_accounts: vec![(ALICE, ())],
		}
		.assimilate_storage(&mut t)
		.unwrap();

		let mut ext = sp_io::TestExternalities::new(t);
		ext.execute_with(|| System::set_block_number(1));
		ext
	}
}
