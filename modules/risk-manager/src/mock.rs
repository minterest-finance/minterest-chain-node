/// Mocks for the RiskManager pallet.
use super::*;
use crate as risk_manager;
use frame_support::pallet_prelude::GenesisBuild;
use frame_support::parameter_types;
use frame_system as system;
use liquidity_pools::{Pool, PoolUserData};
use minterest_primitives::{Balance, CurrencyId, CurrencyPair, Rate};
use orml_traits::parameter_type_with_key;
use sp_core::H256;
use sp_runtime::{
	testing::{Header, TestXt},
	traits::{AccountIdConversion, BlakeTwo256, IdentityLookup, One},
	FixedPointNumber, ModuleId,
};

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;

// Configure a mock runtime to test the pallet.
frame_support::construct_runtime!(
	pub enum Test where
		Block = Block,
		NodeBlock = Block,
		UncheckedExtrinsic = UncheckedExtrinsic,
	{
		System: frame_system::{Module, Call, Config, Storage, Event<T>},
		Tokens: orml_tokens::{Module, Storage, Call, Event<T>, Config<T>},
		Controller: controller::{Module, Storage, Call, Event, Config<T>},
		Oracle: oracle::{Module},
		MinterestModel: minterest_model::{Module, Storage, Call, Event, Config},
		MinterestProtocol: minterest_protocol::{Module, Storage, Call, Event, Config},
		TestAccounts: accounts::{Module, Storage, Call, Event<T>, Config<T>},
		TestPools: liquidity_pools::{Module, Storage, Call, Config<T>},
		TestRiskManager: risk_manager::{Module, Storage, Call, Event<T>, Config, ValidateUnsigned},
		LiquidationPools: liquidation_pools::{Module, Storage, Call, Event}
	}
);

parameter_types! {
	pub const BlockHashCount: u64 = 250;
	pub const SS58Prefix: u8 = 42;
}

pub type AccountId = u64;

impl system::Config for Test {
	type BaseCallFilter = ();
	type BlockWeights = ();
	type BlockLength = ();
	type DbWeight = ();
	type Origin = Origin;
	type Call = Call;
	type Index = u64;
	type BlockNumber = u64;
	type Hash = H256;
	type Hashing = BlakeTwo256;
	type AccountId = u64;
	type Lookup = IdentityLookup<Self::AccountId>;
	type Header = Header;
	type Event = Event;
	type BlockHashCount = BlockHashCount;
	type Version = ();
	type PalletInfo = PalletInfo;
	type AccountData = ();
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type SystemWeightInfo = ();
	type SS58Prefix = SS58Prefix;
}

type Amount = i128;

parameter_type_with_key! {
	pub ExistentialDeposits: |currency_id: CurrencyId| -> Balance {
		Default::default()
	};
}

impl orml_tokens::Config for Test {
	type Event = Event;
	type Balance = Balance;
	type Amount = Amount;
	type CurrencyId = CurrencyId;
	type WeightInfo = ();
	type ExistentialDeposits = ExistentialDeposits;
	type OnDust = ();
}

parameter_types! {
	pub const MaxMembers: u8 = MAX_MEMBERS;
}

impl accounts::Config for Test {
	type Event = Event;
	type MaxMembers = MaxMembers;
}

parameter_types! {
	pub const LiquidityPoolsModuleId: ModuleId = ModuleId(*b"min/lqdy");
	pub LiquidityPoolAccountId: AccountId = LiquidityPoolsModuleId::get().into_account();
	pub InitialExchangeRate: Rate = Rate::one();
	pub EnabledCurrencyPair: Vec<CurrencyPair> = vec![
		CurrencyPair::new(CurrencyId::DOT, CurrencyId::MDOT),
		CurrencyPair::new(CurrencyId::KSM, CurrencyId::MKSM),
		CurrencyPair::new(CurrencyId::BTC, CurrencyId::MBTC),
		CurrencyPair::new(CurrencyId::ETH, CurrencyId::METH),
	];
	pub EnabledUnderlyingAssetId: Vec<CurrencyId> = EnabledCurrencyPair::get().iter()
			.map(|currency_pair| currency_pair.underlying_id)
			.collect();
	pub EnabledWrappedTokensId: Vec<CurrencyId> = EnabledCurrencyPair::get().iter()
			.map(|currency_pair| currency_pair.wrapped_id)
			.collect();
}

impl liquidity_pools::Config for Test {
	type MultiCurrency = orml_tokens::Module<Test>;
	type ModuleId = LiquidityPoolsModuleId;
	type LiquidityPoolAccountId = LiquidityPoolAccountId;
	type InitialExchangeRate = InitialExchangeRate;
	type EnabledCurrencyPair = EnabledCurrencyPair;
	type EnabledUnderlyingAssetId = EnabledUnderlyingAssetId;
	type EnabledWrappedTokensId = EnabledWrappedTokensId;
}

impl controller::Config for Test {
	type Event = Event;
	type LiquidityPoolsManager = liquidity_pools::Module<Test>;
}

impl oracle::Config for Test {}

parameter_types! {
	pub const BlocksPerYear: u128 = BLOCKS_PER_YEAR;
}

impl minterest_model::Config for Test {
	type Event = Event;
	type BlocksPerYear = BlocksPerYear;
}

parameter_types! {
	pub const LiquidationPoolsModuleId: ModuleId = ModuleId(*b"min/lqdn");
	pub LiquidationPoolAccountId: AccountId = LiquidationPoolsModuleId::get().into_account();
}

impl liquidation_pools::Config for Test {
	type Event = Event;
	type ModuleId = LiquidationPoolsModuleId;
	type LiquidationPoolAccountId = LiquidationPoolAccountId;
	type MultiCurrency = orml_tokens::Module<Test>;
}

impl minterest_protocol::Config for Test {
	type Event = Event;
	type Borrowing = liquidity_pools::Module<Test>;
	type ManagerLiquidityPools = liquidity_pools::Module<Test>;
}

parameter_types! {
	pub const RiskManagerPriority: TransactionPriority = TransactionPriority::max_value();
}

impl risk_manager::Config for Test {
	type Event = Event;
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

pub const BLOCKS_PER_YEAR: u128 = 5_256_000;
pub const MAX_MEMBERS: u8 = 16;
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
			member_count: u8::one(),
		}
		.assimilate_storage(&mut t)
		.unwrap();

		risk_manager::GenesisConfig {
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