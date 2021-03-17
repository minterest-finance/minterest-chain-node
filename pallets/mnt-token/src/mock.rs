#![cfg(test)]

use crate as mnt_token;
use frame_support::{construct_runtime, ord_parameter_types, pallet_prelude::GenesisBuild, parameter_types};
use frame_system::EnsureSignedBy;
use minterest_primitives::{Balance, CurrencyId, Price};
use orml_traits::parameter_type_with_key;
use pallet_traits::{LiquidityPoolsTotalProvider, PoolsManager, PriceProvider};
use sp_runtime::FixedPointNumber;

const POOL_TOTAL_BORROWED: Balance = 50;

parameter_type_with_key! {
	pub ExistentialDeposits: |_currency_id: CurrencyId| -> Balance {
		Default::default()
	};
}

parameter_types! {
	pub const BlockHashCount: u32 = 250;
}

pub type AccountId = u64;

pub const ADMIN: AccountId = 0;
pub fn admin() -> Origin {
	Origin::signed(ADMIN)
}

impl frame_system::Config for Runtime {
	type BaseCallFilter = ();
	type Origin = Origin;
	type Index = u64;
	type BlockNumber = u64;
	type Call = Call;
	type Hash = sp_runtime::testing::H256;
	type Hashing = sp_runtime::traits::BlakeTwo256;
	type AccountId = u64;
	type Lookup = sp_runtime::traits::IdentityLookup<Self::AccountId>;
	type Header = sp_runtime::testing::Header;
	type Event = Event;
	type BlockHashCount = BlockHashCount;
	type BlockWeights = ();
	type BlockLength = ();
	type DbWeight = ();
	type Version = ();
	type PalletInfo = PalletInfo;
	type AccountData = ();
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type SystemWeightInfo = ();
	type SS58Prefix = ();
}

type Amount = i128;
impl orml_tokens::Config for Runtime {
	type Event = Event;
	type Balance = Balance;
	type Amount = Amount;
	type CurrencyId = CurrencyId;
	type WeightInfo = ();
	type ExistentialDeposits = ExistentialDeposits;
	type OnDust = ();
}

pub struct MockPriceSource;

impl PriceProvider<CurrencyId> for MockPriceSource {
	fn get_underlying_price(currency_id: CurrencyId) -> Option<Price> {
		match currency_id {
			CurrencyId::DOT => return Some(Price::saturating_from_rational(5, 10)), // 0.5 USD
			CurrencyId::ETH => return Some(Price::saturating_from_rational(15, 10)), // 1.5 USD
			CurrencyId::KSM => return Some(Price::saturating_from_integer(2)),      // 2 USD
			CurrencyId::BTC => return Some(Price::saturating_from_integer(3)),      // 2 USD
			_ => panic!("Currency price not implemented"),
		}
	}

	fn lock_price(_currency_id: CurrencyId) {}

	fn unlock_price(_currency_id: CurrencyId) {}
}

ord_parameter_types! {
	pub const ZeroAdmin: AccountId = 0;
}

pub struct MockLiquidityPoolManager;

impl<AccountId> PoolsManager<AccountId> for MockLiquidityPoolManager {
	fn pools_account_id() -> AccountId {
		unimplemented!()
	}

	fn get_pool_available_liquidity(_pool_id: CurrencyId) -> Balance {
		unimplemented!()
	}

	fn pool_exists(_underlying_asset_id: &CurrencyId) -> bool {
		true
	}
}

pub struct MockLiquidityPoolsTotalProvider;

impl LiquidityPoolsTotalProvider for MockLiquidityPoolsTotalProvider {
	fn get_pool_total_borrowed(_pool_id: CurrencyId) -> Balance {
		POOL_TOTAL_BORROWED
	}

	fn get_pool_total_insurance(_pool_id: CurrencyId) -> Balance {
		unimplemented!()
	}
}

impl mnt_token::Config for Runtime {
	type Event = Event;
	type PriceSource = MockPriceSource;
	type UpdateOrigin = EnsureSignedBy<ZeroAdmin, AccountId>;
	type LiquidityPoolsManager = MockLiquidityPoolManager;
	type LiquidityPoolsTotalProvider = MockLiquidityPoolsTotalProvider;
}

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Runtime>;
type Block = frame_system::mocking::MockBlock<Runtime>;

construct_runtime!(
	pub enum Runtime where
		Block = Block,
		NodeBlock = Block,
		UncheckedExtrinsic = UncheckedExtrinsic
	{
		Tokens: orml_tokens::{Module, Storage, Call, Event<T>, Config<T>},
		System: frame_system::{Module, Call, Event<T>},
		MntToken: mnt_token::{Module, Storage, Call, Event<T>, Config<T>},
	}
);

pub fn new_test_ext() -> sp_io::TestExternalities {
	let mut t = frame_system::GenesisConfig::default()
		.build_storage::<Runtime>()
		.unwrap();
	mnt_token::GenesisConfig::<Runtime> { ..Default::default() }
		.assimilate_storage(&mut t)
		.unwrap();
	let mut ext = sp_io::TestExternalities::new(t);
	ext.execute_with(|| System::set_block_number(1));
	ext
}
