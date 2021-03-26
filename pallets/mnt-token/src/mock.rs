#![cfg(test)]

use crate as mnt_token;
use frame_support::{construct_runtime, ord_parameter_types, pallet_prelude::*, parameter_types};
use frame_system::EnsureSignedBy;
use liquidity_pools::{Pool, PoolUserData};
use minterest_primitives::{Balance, CurrencyId, CurrencyPair, Price, Rate};
use orml_traits::parameter_type_with_key;
use pallet_traits::PriceProvider;
use sp_runtime::{
	traits::{AccountIdConversion, Zero},
	FixedPointNumber, ModuleId,
};

parameter_type_with_key! {
	pub ExistentialDeposits: |_currency_id: CurrencyId| -> Balance {
		Default::default()
	};
}

parameter_types! {
	pub const BlockHashCount: u64 = 250;
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

impl liquidity_pools::Config for Runtime {
	type MultiCurrency = orml_tokens::Module<Runtime>;
	type PriceSource = MockPriceSource;
	type ModuleId = LiquidityPoolsModuleId;
	type LiquidityPoolAccountId = LiquidityPoolAccountId;
	type InitialExchangeRate = InitialExchangeRate;
	type EnabledCurrencyPair = EnabledCurrencyPair;
	type EnabledUnderlyingAssetId = EnabledUnderlyingAssetId;
	type EnabledWrappedTokensId = EnabledWrappedTokensId;
}

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

impl mnt_token::Config for Runtime {
	type Event = Event;
	type PriceSource = MockPriceSource;
	type UpdateOrigin = EnsureSignedBy<ZeroAdmin, AccountId>;
	type LiquidityPoolsManager = liquidity_pools::Module<Runtime>;
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
		TestPools: liquidity_pools::{Module, Storage, Call, Config<T>},
	}
);
pub struct ExtBuilder {
	pools: Vec<(CurrencyId, Pool)>,
	pool_user_data: Vec<(CurrencyId, AccountId, PoolUserData)>,
	minted_pools: Vec<CurrencyId>,
}
pub const DOLLARS: Balance = 1_000_000_000_000_000_000;

impl Default for ExtBuilder {
	fn default() -> Self {
		Self {
			pools: vec![],
			minted_pools: vec![],
			pool_user_data: vec![],
		}
	}
}

impl ExtBuilder {
	pub fn enable_minting_for_all_pools(mut self) -> Self {
		self.minted_pools = vec![CurrencyId::KSM, CurrencyId::DOT, CurrencyId::ETH, CurrencyId::BTC];
		self
	}

	pub fn pool_total_borrowed(mut self, pool_id: CurrencyId, total_borrowed: Balance) -> Self {
		self.pools.push((
			pool_id,
			Pool {
				total_borrowed,
				borrow_index: Rate::one(),
				total_protocol_interest: Balance::zero(),
			},
		));
		self
	}
	pub fn build(self) -> sp_io::TestExternalities {
		let mut t = frame_system::GenesisConfig::default()
			.build_storage::<Runtime>()
			.unwrap();

		liquidity_pools::GenesisConfig::<Runtime> {
			pools: self.pools,
			pool_user_data: self.pool_user_data,
		}
		.assimilate_storage(&mut t)
		.unwrap();

		mnt_token::GenesisConfig::<Runtime> {
			mnt_rate: Rate::zero(),
			minted_pools: self.minted_pools,
			_marker: PhantomData,
		}
		.assimilate_storage(&mut t)
		.unwrap();

		let mut ext = sp_io::TestExternalities::new(t);
		ext.execute_with(|| System::set_block_number(1));
		ext
	}
}
