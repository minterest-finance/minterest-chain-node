/// Mocks for the RiskManager pallet.
use frame_support::{impl_outer_dispatch, impl_outer_event, impl_outer_origin, parameter_types};
use minterest_primitives::{Balance, CurrencyId, CurrencyPair, Rate};
use sp_core::H256;
use sp_runtime::{testing::Header, traits::IdentityLookup, FixedPointNumber, ModuleId, Perbill};

use super::*;
use liquidity_pools::{Pool, PoolUserData};
use sp_runtime::testing::TestXt;

impl_outer_origin! {
	pub enum Origin for Test {}
}

mod risk_manager {
	pub use crate::Event;
}

impl_outer_event! {
	pub enum TestEvent for Test {
		frame_system<T>,
		orml_tokens<T>,
		accounts<T>,
		liquidity_pools,
		liquidation_pools,
		minterest_protocol<T>,
		risk_manager<T>,
		controller,
		minterest_model,
		oracle,

	}
}

impl_outer_dispatch! {
	pub enum Call for Test where origin: Origin {
		risk_manager::TestRiskManager,
	}
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Test;

parameter_types! {
	pub const BlockHashCount: u64 = 250;
	pub const MaximumBlockWeight: u32 = 1024;
	pub const MaximumBlockLength: u32 = 2 * 1024;
	pub const AvailableBlockRatio: Perbill = Perbill::from_percent(75);
}

type AccountId = u32;

impl frame_system::Trait for Test {
	type Origin = Origin;
	type Call = Call;
	type Index = u64;
	type BlockNumber = u64;
	type Hash = H256;
	type Hashing = ::sp_runtime::traits::BlakeTwo256;
	type AccountId = AccountId;
	type Lookup = IdentityLookup<Self::AccountId>;
	type Header = Header;
	type Event = TestEvent;
	type BlockHashCount = BlockHashCount;
	type MaximumExtrinsicWeight = MaximumBlockWeight;
	type MaximumBlockWeight = MaximumBlockWeight;
	type DbWeight = ();
	type BlockExecutionWeight = ();
	type ExtrinsicBaseWeight = ();
	type MaximumBlockLength = MaximumBlockLength;
	type AvailableBlockRatio = AvailableBlockRatio;
	type Version = ();
	type PalletInfo = ();
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type AccountData = ();
	type BaseCallFilter = ();
	type SystemWeightInfo = ();
}

parameter_types! {
	pub const ExistentialDeposit: u64 = 1;
}

impl orml_tokens::Trait for Test {
	type Event = TestEvent;
	type Balance = Balance;
	type Amount = Amount;
	type CurrencyId = CurrencyId;
	type OnReceived = ();
	type WeightInfo = ();
}

parameter_types! {
	pub const MaxMembers: u32 = MAX_MEMBERS;
}

impl accounts::Trait for Test {
	type Event = TestEvent;
	type MaxMembers = MaxMembers;
}

parameter_types! {
	pub const LiquidityPoolsModuleId: ModuleId = ModuleId(*b"min/lqdy");
	pub const InitialExchangeRate: Rate = Rate::from_inner(1_000_000_000_000_000_000);
	pub EnabledCurrencyPair: Vec<CurrencyPair> = vec![
		CurrencyPair::new(CurrencyId::DOT, CurrencyId::MDOT),
		CurrencyPair::new(CurrencyId::KSM, CurrencyId::MKSM),
		CurrencyPair::new(CurrencyId::BTC, CurrencyId::MBTC),
		CurrencyPair::new(CurrencyId::ETH, CurrencyId::METH),
	];
}

impl liquidity_pools::Trait for Test {
	type Event = TestEvent;
	type MultiCurrency = orml_tokens::Module<Test>;
	type ModuleId = LiquidityPoolsModuleId;
	type InitialExchangeRate = InitialExchangeRate;
	type EnabledCurrencyPair = EnabledCurrencyPair;
}

impl controller::Trait for Test {
	type Event = TestEvent;
	type LiquidityPoolsManager = liquidity_pools::Module<Test>;
}

impl oracle::Trait for Test {
	type Event = TestEvent;
}

parameter_types! {
	pub const BlocksPerYear: u128 = BLOCKS_PER_YEAR;
}

impl minterest_model::Trait for Test {
	type Event = TestEvent;
	type BlocksPerYear = BlocksPerYear;
}

parameter_types! {
	pub const LiquidationPoolsModuleId: ModuleId = ModuleId(*b"min/lqdn");
}

impl liquidation_pools::Trait for Test {
	type Event = TestEvent;
	type ModuleId = LiquidationPoolsModuleId;
	type MultiCurrency = orml_tokens::Module<Test>;
}

impl minterest_protocol::Trait for Test {
	type Event = TestEvent;
	type Borrowing = liquidity_pools::Module<Test>;
	type ManagerLiquidityPools = liquidity_pools::Module<Test>;
}

parameter_types! {
	pub const RiskManagerPriority: TransactionPriority = TransactionPriority::max_value();
}

impl Trait for Test {
	type Event = TestEvent;
	type UnsignedPriority = RiskManagerPriority;
	type LiquidationPoolsManager = liquidation_pools::Module<Test>;
	type LiquidityPoolsManager = liquidity_pools::Module<Test>;
}

/// An extrinsic type used for tests.
pub type Extrinsic = TestXt<Call, ()>;

impl<LocalCall> SendTransactionTypes<LocalCall> for Test
where
	Call: From<LocalCall>,
{
	type OverarchingCall = Call;
	type Extrinsic = Extrinsic;
}

type Amount = i128;

pub type TestRiskManager = Module<Test>;
pub type System = frame_system::Module<Test>;
pub const BLOCKS_PER_YEAR: u128 = 5_256_000;
pub const MAX_MEMBERS: u32 = 16;
pub const ONE_HUNDRED: Balance = 100;
pub const DOLLARS: Balance = 1_000_000_000_000_000_000;
pub const ADMIN: AccountId = 0;
pub fn admin() -> Origin {
	Origin::signed(ADMIN)
}
pub const ALICE: AccountId = 1;
pub fn alice() -> Origin {
	Origin::signed(ALICE)
}

pub struct ExtBuilder {
	endowed_accounts: Vec<(AccountId, CurrencyId, Balance)>,
	pools: Vec<(CurrencyId, Pool)>,
	pool_user_data: Vec<(CurrencyId, AccountId, PoolUserData)>,
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

impl ExtBuilder {
	pub fn build(self) -> sp_io::TestExternalities {
		let mut t = frame_system::GenesisConfig::default().build_storage::<Test>().unwrap();

		orml_tokens::GenesisConfig::<Test> {
			endowed_accounts: self.endowed_accounts,
		}
		.assimilate_storage(&mut t)
		.unwrap();

		liquidity_pools::GenesisConfig::<Test> {
			pools: self.pools,
			pool_user_data: self.pool_user_data,
		}
		.assimilate_storage(&mut t)
		.unwrap();

		accounts::GenesisConfig::<Test> {
			allowed_accounts: vec![(ADMIN, ())],
		}
		.assimilate_storage(&mut t)
		.unwrap();

		GenesisConfig {
			risk_manager_dates: vec![
				(
					CurrencyId::DOT,
					RiskManagerData {
						max_attempts: 3,
						min_sum: ONE_HUNDRED * DOLLARS,
						threshold: Rate::saturating_from_rational(103, 100),
						liquidation_incentive: Rate::saturating_from_rational(105, 100),
					},
				),
				(
					CurrencyId::BTC,
					RiskManagerData {
						max_attempts: 3,
						min_sum: ONE_HUNDRED * DOLLARS,
						threshold: Rate::saturating_from_rational(103, 100),
						liquidation_incentive: Rate::saturating_from_rational(105, 100),
					},
				),
				(
					CurrencyId::ETH,
					RiskManagerData {
						max_attempts: 3,
						min_sum: ONE_HUNDRED * DOLLARS,
						threshold: Rate::saturating_from_rational(103, 100),
						liquidation_incentive: Rate::saturating_from_rational(105, 100),
					},
				),
			],
		}
		.assimilate_storage(&mut t)
		.unwrap();

		let mut ext = sp_io::TestExternalities::new(t);
		ext.execute_with(|| System::set_block_number(1));
		ext
	}
}
