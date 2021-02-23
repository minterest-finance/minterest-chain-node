/// Mocks for the minterest-model pallet.
use super::*;
use crate as minterest_model;
use frame_support::parameter_types;
use frame_support::traits::GenesisBuild;
use frame_system as system;
use minterest_primitives::{Balance, CurrencyId, CurrencyPair, Rate};
use orml_traits::parameter_type_with_key;
use sp_core::H256;
use sp_runtime::traits::One;
use sp_runtime::{
	testing::Header,
	traits::{BlakeTwo256, IdentityLookup},
	ModuleId,
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
		Oracle: oracle::{Module, Storage, Call, Event},
		TestMinterestModel: minterest_model::{Module, Storage, Call, Event, Config},
		TestAccounts: accounts::{Module, Storage, Call, Event<T>, Config<T>},
		TestPools: liquidity_pools::{Module, Storage, Call, Event, Config<T>},
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
	pub const LiquidityPoolsModuleId: ModuleId = ModuleId(*b"min/pool");
	pub const InitialExchangeRate: Rate = Rate::from_inner(1_000_000_000_000_000_000);
	pub EnabledCurrencyPair: Vec<CurrencyPair> = vec![
		CurrencyPair::new(CurrencyId::DOT, CurrencyId::MDOT),
		CurrencyPair::new(CurrencyId::KSM, CurrencyId::MKSM),
		CurrencyPair::new(CurrencyId::BTC, CurrencyId::MBTC),
		CurrencyPair::new(CurrencyId::ETH, CurrencyId::METH),
	];
}

impl liquidity_pools::Config for Test {
	type Event = Event;
	type MultiCurrency = orml_tokens::Module<Test>;
	type ModuleId = LiquidityPoolsModuleId;
	type InitialExchangeRate = InitialExchangeRate;
	type EnabledCurrencyPair = EnabledCurrencyPair;
}

impl oracle::Config for Test {
	type Event = Event;
}

parameter_types! {
	pub const BlocksPerYear: u128 = BLOCKS_PER_YEAR;
}

impl minterest_model::Config for Test {
	type Event = Event;
	type BlocksPerYear = BlocksPerYear;
}

pub const BLOCKS_PER_YEAR: u128 = 5_256_000;
pub const MAX_MEMBERS: u8 = 16;
pub const ALICE: AccountId = 1;
pub fn alice() -> Origin {
	Origin::signed(ALICE)
}
pub const BOB: AccountId = 2;
pub fn bob() -> Origin {
	Origin::signed(BOB)
}

pub fn new_test_ext() -> sp_io::TestExternalities {
	let mut t = frame_system::GenesisConfig::default().build_storage::<Test>().unwrap();

	crate::GenesisConfig {
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
				CurrencyId::KSM,
				MinterestModelData {
					kink: Rate::saturating_from_rational(8, 10),
					base_rate_per_block: Rate::zero(),
					multiplier_per_block: Rate::saturating_from_rational(9, 1_000_000_000), // 0.047304 PerYear
					jump_multiplier_per_block: Rate::saturating_from_rational(207, 1_000_000_000), // 1.09 PerYear
				},
			),
			(
				CurrencyId::BTC,
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

	accounts::GenesisConfig::<Test> {
		allowed_accounts: vec![(ALICE, ())],
		member_count: u8::one(),
	}
	.assimilate_storage(&mut t)
	.unwrap();

	let mut ext: sp_io::TestExternalities = t.into();
	ext.execute_with(|| System::set_block_number(1));
	ext
}
