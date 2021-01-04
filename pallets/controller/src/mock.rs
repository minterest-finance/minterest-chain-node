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

parameter_types! {
	pub const InitialExchangeRate: Rate = Rate::from_inner(1_000_000_000_000_000_000);
	pub const BlocksPerYear: u128 = 5256000;
}

impl Trait for Runtime {
	type Event = TestEvent;
	type InitialExchangeRate = InitialExchangeRate;
	type BlocksPerYear = BlocksPerYear;
}

pub type BlockNumber = u64;

pub type Controller = Module<Runtime>;
pub type TestPools = liquidity_pools::Module<Runtime>;
pub type System = frame_system::Module<Runtime>;
pub type Currencies = orml_currencies::Module<Runtime>;

pub struct ExtBuilder {
	endowed_accounts: Vec<(AccountId, CurrencyId, Balance)>,
	pools: Vec<(CurrencyId, Pool)>,
	pool_user_data: Vec<(AccountId, CurrencyId, PoolUserData<BlockNumber>)>,
}

impl Default for ExtBuilder {
	fn default() -> Self {
		Self {
			endowed_accounts: vec![
				(ALICE, CurrencyId::MDOT, ONE_HUNDRED),
				(ALICE, CurrencyId::MINT, ONE_MILL),
			],
			pools: vec![
				(
					CurrencyId::DOT,
					Pool {
						current_interest_rate: Rate::from_inner(0),
						total_borrowed: Balance::zero(),
						borrow_index: Rate::saturating_from_rational(1, 1),
						current_exchange_rate: Rate::saturating_from_rational(1, 1),
						is_lock: false,
						total_insurance: Balance::zero(),
					},
				),
				(
					CurrencyId::BTC,
					Pool {
						current_interest_rate: Rate::from_inner(0),
						total_borrowed: Balance::zero(),
						borrow_index: Rate::saturating_from_rational(1, 1),
						current_exchange_rate: Rate::saturating_from_rational(1, 1),
						is_lock: true,
						total_insurance: Balance::zero(),
					},
				),
			],
			pool_user_data: vec![],
		}
	}
}

pub const ALICE: AccountId = 1;
pub const ONE_MILL: Balance = 1_000_000;
pub const ONE_HUNDRED: Balance = 100;

impl ExtBuilder {
	pub fn exchange_rate_less_than_one(mut self) -> Self {
		self.endowed_accounts.extend_from_slice(&[
			(ALICE, CurrencyId::DOT, ONE_HUNDRED),
			(ALICE, CurrencyId::MBTC, ONE_HUNDRED),
		]);
		self
	}

	pub fn exchange_rate_greater_than_one(mut self) -> Self {
		self.endowed_accounts.extend_from_slice(&[
			(ALICE, CurrencyId::DOT, ONE_HUNDRED),
			(ALICE, CurrencyId::BTC, ONE_HUNDRED),
			(ALICE, CurrencyId::MBTC, 1),
		]);
		self
	}

	pub fn one_hundred_dots_for_alice(mut self) -> Self {
		self.endowed_accounts.push((ALICE, CurrencyId::DOT, ONE_HUNDRED));
		self
	}

	pub fn set_alice_total_borrowed_and_interest_index(mut self) -> Self {
		self.pool_user_data = vec![(
			ALICE,
			CurrencyId::DOT,
			PoolUserData {
				total_borrowed: 100,
				interest_index: Rate::saturating_from_rational(2, 1),
				collateral: true,
				timestamp: 2,
			},
		)];
		self
	}

	pub fn set_alice_interest_index(mut self) -> Self {
		self.pool_user_data = vec![(
			ALICE,
			CurrencyId::DOT,
			PoolUserData {
				total_borrowed: Balance::zero(),
				interest_index: Rate::saturating_from_rational(2, 1),
				collateral: true,
				timestamp: 2,
			},
		)];
		self
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
			controller_dates: vec![(
				CurrencyId::DOT,
				ControllerData {
					timestamp: 1,
					borrow_rate: Rate::from_inner(0),
					insurance_factor: Rate::saturating_from_rational(1, 10),
					max_borrow_rate: Rate::saturating_from_rational(5, 1000),
					kink: Rate::saturating_from_rational(8, 10),
					base_rate_per_block: Rate::from_inner(0),
					multiplier_per_block: Rate::saturating_from_rational(9, 1_000_000_000),
					jump_multiplier_per_block: Rate::saturating_from_rational(2, 1),
				},
			)],
		}
		.assimilate_storage(&mut t)
		.unwrap();

		liquidity_pools::GenesisConfig::<Runtime> {
			pools: self.pools,
			pool_user_data: self.pool_user_data,
		}
		.assimilate_storage(&mut t)
		.unwrap();

		let mut ext = sp_io::TestExternalities::new(t);
		ext.execute_with(|| System::set_block_number(1));
		ext
	}
}
