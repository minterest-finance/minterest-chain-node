#![cfg(test)]

use crate as mnt_token;
use frame_support::{construct_runtime, ord_parameter_types, pallet_prelude::*, parameter_types};
use frame_system::EnsureSignedBy;
use liquidity_pools::{Pool, PoolUserData};
use minterest_primitives::{Amount, Balance, BlockNumber, CurrencyId, CurrencyPair, Price, Rate};
use orml_currencies::BasicCurrencyAdapter;
use orml_traits::parameter_type_with_key;
use pallet_traits::PriceProvider;
use sp_runtime::{
	testing::{Header, H256},
	traits::{AccountIdConversion, BlakeTwo256, IdentityLookup, Zero},
	FixedPointNumber, ModuleId,
};
use test_helper::*;

parameter_types! {
	pub const LiquidityPoolsModuleId: ModuleId = ModuleId(*b"min/lqdy");
	pub LiquidityPoolAccountId: AccountId = LiquidityPoolsModuleId::get().into_account();
	pub const MntTokenModuleId: ModuleId = ModuleId(*b"min/mntt");
	pub MntTokenAccountId: AccountId = MntTokenModuleId::get().into_account();
	pub InitialExchangeRate: Rate = Rate::one();
	pub EnabledCurrencyPair: Vec<CurrencyPair> = vec![
		CurrencyPair::new(CurrencyId::DOT, CurrencyId::MDOT),
		CurrencyPair::new(CurrencyId::KSM, CurrencyId::MKSM),
		CurrencyPair::new(CurrencyId::BTC, CurrencyId::MBTC),
		CurrencyPair::new(CurrencyId::ETH, CurrencyId::METH),
	];
	pub EnabledUnderlyingAssetsIds: Vec<CurrencyId> = EnabledCurrencyPair::get().iter()
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
ord_parameter_types! {
	pub const ZeroAdmin: AccountId = 0;
	pub const BlockHashCount: u64 = 250;
}

pub struct MockPriceSource;

mock_impl_system_config!(Runtime, pallet_balances::AccountData<Balance>);
mock_impl_orml_tokens_config!(Runtime);

pub type MinterestToken = BasicCurrencyAdapter<Runtime, Balances, Amount, BlockNumber>;
parameter_types! {
	pub const GetNativeCurrencyId: CurrencyId = CurrencyId::MNT;
}

impl orml_currencies::Config for Runtime {
	type Event = Event;
	type MultiCurrency = orml_tokens::Module<Runtime>;
	type NativeCurrency = MinterestToken;
	type GetNativeCurrencyId = GetNativeCurrencyId;
	type WeightInfo = ();
}

mock_impl_liquidity_pools_config!(Runtime);
mock_impl_minterest_model_config!(Runtime, ZeroAdmin);
mock_impl_controller_config!(Runtime, ZeroAdmin);

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

construct_runtime!(
	pub enum Runtime where
		Block = Block,
		NodeBlock = Block,
		UncheckedExtrinsic = UncheckedExtrinsic
	{
		System: frame_system::{Module, Call, Event<T>},
		Tokens: orml_tokens::{Module, Storage, Call, Event<T>, Config<T>},
		Balances: pallet_balances::{Module, Call, Storage, Config<T>, Event<T>},
		Currencies: orml_currencies::{Module, Call, Event<T>},
		MntToken: mnt_token::{Module, Storage, Call, Event<T>, Config<T>},
		TestPools: liquidity_pools::{Module, Storage, Call, Config<T>},
		MinterestModel: minterest_model::{Module, Storage, Call, Event, Config},
		Controller: controller::{Module, Storage, Call, Event, Config<T>},
	}
);

parameter_types! {
	pub const ExistentialDeposit: u128 = 500;
	pub const MaxLocks: u32 = 50;
}
impl pallet_balances::Config for Runtime {
	type MaxLocks = MaxLocks;
	type Balance = Balance;
	type Event = Event;
	type DustRemoval = ();
	type ExistentialDeposit = ExistentialDeposit;
	type AccountStore = System;
	type WeightInfo = pallet_balances::weights::SubstrateWeight<Runtime>;
}

impl mnt_token::Config for Runtime {
	type Event = Event;
	type PriceSource = MockPriceSource;
	type UpdateOrigin = EnsureSignedBy<ZeroAdmin, AccountId>;
	type LiquidityPoolsManager = liquidity_pools::Module<Runtime>;
	type EnabledCurrencyPair = EnabledCurrencyPair;
	type EnabledUnderlyingAssetsIds = EnabledUnderlyingAssetsIds;
	type MultiCurrency = Currencies;
	type ControllerAPI = Controller;
	type MntTokenAccountId = MntTokenAccountId;
}

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Runtime>;
type Block = frame_system::mocking::MockBlock<Runtime>;

pub struct ExtBuilder {
	pools: Vec<(CurrencyId, Pool)>,
	pool_user_data: Vec<(CurrencyId, AccountId, PoolUserData)>,
	minted_pools: Vec<CurrencyId>,
	endowed_accounts: Vec<(AccountId, CurrencyId, Balance)>,
	mnt_rate: Balance,
	mnt_claim_treshold: Balance,
}

pub const ALICE: AccountId = 1;
pub const BOB: AccountId = 2;
pub const DOLLARS: Balance = 1_000_000_000_000_000_000;

impl Default for ExtBuilder {
	fn default() -> Self {
		Self {
			pools: vec![],
			minted_pools: vec![],
			pool_user_data: vec![],
			endowed_accounts: vec![],
			mnt_claim_treshold: Balance::zero(),
			mnt_rate: Balance::zero(),
		}
	}
}

impl ExtBuilder {
	pub fn enable_minting_for_all_pools(mut self) -> Self {
		self.minted_pools = vec![CurrencyId::KSM, CurrencyId::DOT, CurrencyId::ETH, CurrencyId::BTC];
		self
	}

	pub fn set_mnt_rate(mut self, rate: u128) -> Self {
		self.mnt_rate = rate * DOLLARS;
		self
	}

	pub fn set_mnt_claim_treshold(mut self, treshold: u128) -> Self {
		self.mnt_claim_treshold = treshold * DOLLARS;
		self
	}

	pub fn pool_total_borrowed(mut self, pool_id: CurrencyId, total_borrowed: Balance) -> Self {
		self.pools.push((
			pool_id,
			Pool {
				total_borrowed,
				borrow_index: Rate::saturating_from_rational(15, 10),
				total_protocol_interest: Balance::zero(),
			},
		));
		self
	}

	pub fn mnt_account_balance(mut self, balance: Balance) -> Self {
		self.endowed_accounts
			.push((MntToken::get_account_id(), CurrencyId::MNT, balance));
		self
	}

	pub fn pool_user_data(
		mut self,
		pool_id: CurrencyId,
		user: AccountId,
		total_borrowed: Balance,
		interest_index: Rate,
		is_collateral: bool,
		liquidation_attempts: u8,
	) -> Self {
		self.pool_user_data.push((
			pool_id,
			user,
			PoolUserData {
				total_borrowed,
				interest_index,
				is_collateral,
				liquidation_attempts,
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

		pallet_balances::GenesisConfig::<Runtime> {
			balances: self
				.endowed_accounts
				.clone()
				.into_iter()
				.filter(|(_, currency_id, _)| *currency_id == CurrencyId::MNT)
				.map(|(account_id, _, initial_balance)| (account_id, initial_balance))
				.collect::<Vec<_>>(),
		}
		.assimilate_storage(&mut t)
		.unwrap();

		orml_tokens::GenesisConfig::<Runtime> {
			endowed_accounts: self
				.endowed_accounts
				.into_iter()
				.filter(|(_, currency_id, _)| *currency_id != CurrencyId::MNT)
				.collect::<Vec<_>>(),
		}
		.assimilate_storage(&mut t)
		.unwrap();

		mnt_token::GenesisConfig::<Runtime> {
			mnt_rate: self.mnt_rate,
			mnt_claim_treshold: self.mnt_claim_treshold,
			minted_pools: self.minted_pools,
			phantom: PhantomData,
		}
		.assimilate_storage(&mut t)
		.unwrap();

		let mut ext = sp_io::TestExternalities::new(t);
		ext.execute_with(|| System::set_block_number(1));
		ext
	}
}
