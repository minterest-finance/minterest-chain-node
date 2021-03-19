#![cfg(test)]

use crate as mnt_token;
use frame_support::{construct_runtime, ord_parameter_types, pallet_prelude::GenesisBuild, parameter_types};
use frame_system::EnsureSignedBy;
use minterest_primitives::{Balance, CurrencyId, CurrencyPair, Price, Rate};
use orml_traits::parameter_type_with_key;
use pallet_traits::{LiquidityPoolsTotalProvider, PriceProvider};
use sp_runtime::{DispatchError, FixedPointNumber};
use sp_std::result;

const POOL_TOTAL_BORROWED: Balance = 50;

parameter_type_with_key! {
	pub ExistentialDeposits: |_currency_id: CurrencyId| -> Balance {
		Default::default()
	};
}

parameter_types! {
	pub const BlockHashCount: u32 = 250;
	pub EnabledCurrencyPair: Vec<CurrencyPair> = vec![
		CurrencyPair::new(CurrencyId::DOT, CurrencyId::MDOT),
		CurrencyPair::new(CurrencyId::KSM, CurrencyId::MKSM),
		CurrencyPair::new(CurrencyId::BTC, CurrencyId::MBTC),
		CurrencyPair::new(CurrencyId::ETH, CurrencyId::METH),
	];
	pub EnabledUnderlyingAssetId: Vec<CurrencyId> = EnabledCurrencyPair::get().iter()
			.map(|currency_pair| currency_pair.underlying_id)
			.collect();
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
			CurrencyId::BTC => return Some(Price::saturating_from_integer(3)),      // 3 USD
			_ => return None,
		}
	}

	fn lock_price(_currency_id: CurrencyId) {}

	fn unlock_price(_currency_id: CurrencyId) {}
}

ord_parameter_types! {
	pub const ZeroAdmin: AccountId = 0;
}

pub struct MockLiquidityPoolsTotalProvider;

impl LiquidityPoolsTotalProvider for MockLiquidityPoolsTotalProvider {
	fn get_pool_total_borrowed(_pool_id: CurrencyId) -> result::Result<Balance, DispatchError> {
		Ok(POOL_TOTAL_BORROWED)
	}

	fn get_pool_total_insurance(_pool_id: CurrencyId) -> result::Result<Balance, DispatchError> {
		unimplemented!()
	}
}

impl mnt_token::Config for Runtime {
	type Event = Event;
	type PriceSource = MockPriceSource;
	type UpdateOrigin = EnsureSignedBy<ZeroAdmin, AccountId>;
	type LiquidityPoolsTotalProvider = MockLiquidityPoolsTotalProvider;
	type EnabledUnderlyingAssetId = EnabledUnderlyingAssetId;
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

pub fn new_test_ext_with_prepared_mnt_speeds() -> sp_io::TestExternalities {
	let mut t = frame_system::GenesisConfig::default()
		.build_storage::<Runtime>()
		.unwrap();
	mnt_token::GenesisConfig::<Runtime> { ..Default::default() }
		.assimilate_storage(&mut t)
		.unwrap();
	let mut ext = sp_io::TestExternalities::new(t);
	ext.execute_with(|| {
		System::set_block_number(1);
		MntToken::enable_mnt_minting(admin(), CurrencyId::DOT).unwrap();
		MntToken::enable_mnt_minting(admin(), CurrencyId::KSM).unwrap();
		MntToken::enable_mnt_minting(admin(), CurrencyId::ETH).unwrap();
		MntToken::enable_mnt_minting(admin(), CurrencyId::BTC).unwrap();
		let mnt_rate = Rate::saturating_from_integer(10);
		MntToken::set_mnt_rate(admin(), mnt_rate).unwrap();
	});
	ext
}
