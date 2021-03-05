/// Mocks for the liquidation-pools pallet.
use super::*;
use crate as liquidation_pools;
use frame_support::parameter_types;
use frame_system as system;
pub use minterest_primitives::{Balance, CurrencyId, CurrencyPair, Rate};
use orml_currencies::Currency;
use orml_traits::parameter_type_with_key;
use sp_core::H256;
use sp_io::TestExternalities;
use sp_runtime::testing::TestXt;
use sp_runtime::{
	testing::Header,
	traits::{BlakeTwo256, IdentityLookup, One},
	FixedPointNumber,
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
		//ORML palletts
		Tokens: orml_tokens::{Module, Storage, Call, Event<T>, Config<T>},
		Currencies: orml_currencies::{Module, Call, Event<T>},
		// Minterest pallets
		TestLiquidationPools: liquidation_pools::{Module, Storage, Call, Event<T>, ValidateUnsigned},
		TestLiquidityPools: liquidity_pools::{Module, Storage, Call, Config<T>},
		TestAccounts: accounts::{Module, Storage, Call, Event<T>, Config<T>},
	}
);

parameter_types! {
	pub const BlockHashCount: u64 = 250;
	pub const SS58Prefix: u8 = 42;
}

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
	pub const GetNativeCurrencyId: CurrencyId = CurrencyId::MNT;
}

type NativeCurrency = Currency<Test, GetNativeCurrencyId>;

impl orml_currencies::Config for Test {
	type Event = Event;
	type MultiCurrency = orml_tokens::Module<Test>;
	type NativeCurrency = NativeCurrency;
	type GetNativeCurrencyId = GetNativeCurrencyId;
	type WeightInfo = ();
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

parameter_types! {
	pub const MaxMembers: u8 = MAX_MEMBERS;
}

impl accounts::Config for Test {
	type Event = Event;
	type MaxMembers = MaxMembers;
}

impl oracle::Config for Test {}

parameter_types! {
	pub const LiquidationPoolsModuleId: ModuleId = ModuleId(*b"min/lqdn");
	pub LiquidationPoolAccountId: AccountId = LiquidationPoolsModuleId::get().into_account();
	pub const LiquidityPoolsPriority: TransactionPriority = TransactionPriority::max_value();
}

impl Config for Test {
	type Event = Event;
	type UnsignedPriority = LiquidityPoolsPriority;
	type LiquidationPoolsModuleId = LiquidationPoolsModuleId;
	type LiquidationPoolAccountId = LiquidationPoolAccountId;
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
type AccountId = u64;
pub type BlockNumber = u64;
pub const DOLLARS: u128 = 1_000_000_000_000_000_000u128;
pub const MAX_MEMBERS: u8 = 16;
pub const ADMIN: AccountId = 0;
pub fn admin() -> Origin {
	Origin::signed(ADMIN)
}
pub const ALICE: AccountId = 1;
pub fn alice() -> Origin {
	Origin::signed(ALICE)
}

pub struct ExternalityBuilder {
	liquidation_pools: Vec<(CurrencyId, LiquidationPool)>,
	liquidation_pool_params: LiquidationPoolCommonData<BlockNumber>,
}

impl Default for ExternalityBuilder {
	fn default() -> Self {
		Self {
			liquidation_pools: vec![(
				CurrencyId::DOT,
				LiquidationPool {
					deviation_threshold: Rate::saturating_from_rational(1, 10),
					balance_ratio: Rate::saturating_from_rational(2, 10),
				},
			)],
			liquidation_pool_params: LiquidationPoolCommonData {
				timestamp: 1,
				balancing_period: 600, // Blocks per 10 minutes.},
			},
		}
	}
}

impl ExternalityBuilder {
	pub fn pool_timestamp_and_period(mut self, timestamp: BlockNumber, balancing_period: u32) -> Self {
		self.liquidation_pool_params = LiquidationPoolCommonData {
			timestamp,
			balancing_period,
		};
		self
	}

	pub fn build(self) -> TestExternalities {
		let mut t = frame_system::GenesisConfig::default().build_storage::<Test>().unwrap();

		accounts::GenesisConfig::<Test> {
			allowed_accounts: vec![(ADMIN, ())],
			member_count: u8::one(),
		}
		.assimilate_storage(&mut t)
		.unwrap();

		liquidation_pools::GenesisConfig::<Test> {
			liquidation_pools: self.liquidation_pools,
			liquidation_pool_params: self.liquidation_pool_params,
		}
		.assimilate_storage(&mut t)
		.unwrap();

		let mut ext = TestExternalities::new(t);
		ext.execute_with(|| System::set_block_number(1));
		ext
	}
}
