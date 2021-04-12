/// Mocks for the liquidation-pools pallet.
use super::*;
use crate as liquidation_pools;
use frame_support::{ord_parameter_types, parameter_types};
use frame_system as system;
use frame_system::EnsureSignedBy;
pub use minterest_primitives::currency::{DOT, MDOT, MNT};
use minterest_primitives::Price;
pub use minterest_primitives::{Balance, CurrencyId, Rate};
use orml_currencies::Currency;
use orml_traits::parameter_type_with_key;
use pallet_traits::PriceProvider;
use sp_core::H256;
use sp_io::TestExternalities;
use sp_runtime::testing::TestXt;
use sp_runtime::{
	testing::Header,
	traits::{BlakeTwo256, IdentityLookup},
	FixedPointNumber,
};
use test_helper::*;

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
		TestPools: liquidity_pools::{Module, Storage, Call, Config<T>},
		TestDex: dex::{Module, Storage, Call, Event<T>}
	}
);

mock_impl_system_config!(Test);
mock_impl_liquidity_pools_config!(Test);
mock_impl_orml_tokens_config!(Test);
mock_impl_orml_currencies_config!(Test, MNT);
mock_impl_dex_config!(Test);

parameter_types! {
	pub const LiquidityPoolsModuleId: ModuleId = ModuleId(*b"min/lqdy");
	pub LiquidityPoolAccountId: AccountId = LiquidityPoolsModuleId::get().into_account();
	pub InitialExchangeRate: Rate = Rate::one();
	pub EnabledUnderlyingAssetsIds: Vec<CurrencyId> = CurrencyId::get_enabled_underlying_assets_ids();
	pub EnabledWrappedTokensId: Vec<CurrencyId> = CurrencyId::get_enabled_wrapped_tokens_ids();
}

pub struct MockPriceSource;

impl PriceProvider<CurrencyId> for MockPriceSource {
	fn get_underlying_price(_currency_id: CurrencyId) -> Option<Price> {
		Some(Price::one())
	}

	fn lock_price(_currency_id: CurrencyId) {}

	fn unlock_price(_currency_id: CurrencyId) {}
}

parameter_types! {
	pub const LiquidationPoolsModuleId: ModuleId = ModuleId(*b"min/lqdn");
	pub LiquidationPoolAccountId: AccountId = LiquidationPoolsModuleId::get().into_account();
	pub const LiquidityPoolsPriority: TransactionPriority = TransactionPriority::max_value();
}

ord_parameter_types! {
	pub const ZeroAdmin: AccountId = 0;
}

impl Config for Test {
	type Event = Event;
	type MultiCurrency = orml_tokens::Module<Test>;
	type UnsignedPriority = LiquidityPoolsPriority;
	type PriceSource = MockPriceSource;
	type LiquidationPoolsModuleId = LiquidationPoolsModuleId;
	type LiquidationPoolAccountId = LiquidationPoolAccountId;
	type UpdateOrigin = EnsureSignedBy<ZeroAdmin, AccountId>;
	type LiquidityPoolsManager = liquidity_pools::Module<Test>;
	type Dex = dex::Module<Test>;
	type LiquidationPoolsWeightInfo = ();
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

type AccountId = u64;
pub type BlockNumber = u64;
pub const ADMIN: AccountId = 0;
pub fn admin() -> Origin {
	Origin::signed(ADMIN)
}
pub const ALICE: AccountId = 1;
pub fn alice() -> Origin {
	Origin::signed(ALICE)
}

pub struct ExternalityBuilder {
	liquidation_pools: Vec<(CurrencyId, LiquidationPoolData)>,
	balancing_period: BlockNumber,
}

impl Default for ExternalityBuilder {
	fn default() -> Self {
		Self {
			liquidation_pools: vec![(
				DOT,
				LiquidationPoolData {
					deviation_threshold: Rate::saturating_from_rational(1, 10),
					balance_ratio: Rate::saturating_from_rational(2, 10),
				},
			)],
			balancing_period: 600, // Blocks per 10 minutes
		}
	}
}

impl ExternalityBuilder {
	pub fn build(self) -> TestExternalities {
		let mut t = frame_system::GenesisConfig::default().build_storage::<Test>().unwrap();

		liquidation_pools::GenesisConfig::<Test> {
			liquidation_pools: self.liquidation_pools,
			balancing_period: self.balancing_period,
		}
		.assimilate_storage(&mut t)
		.unwrap();

		let mut ext = TestExternalities::new(t);
		ext.execute_with(|| System::set_block_number(1));
		ext
	}
}
