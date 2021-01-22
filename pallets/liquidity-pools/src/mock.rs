#![cfg(test)]

use frame_support::{impl_outer_event, impl_outer_origin, parameter_types};
pub use minterest_primitives::{Balance, CurrencyId};
use orml_currencies::Currency;
use sp_core::H256;
use sp_runtime::{testing::Header, traits::IdentityLookup, Perbill};

use super::*;
use crate::GenesisConfig;

mod liquidity_pools {
	pub use crate::Event;
}

impl_outer_origin! {
	pub enum Origin for Test {}
}

impl_outer_event! {
	pub enum TestEvent for Test {
		frame_system<T>,
		liquidity_pools,
		orml_currencies<T>,
		orml_tokens<T>,
	}
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Test;

// For testing the module, we construct most of a mock runtime. This means
// first constructing a configuration type (`Test`) which `impl`s each of the
// configuration traits of modules we want to use.
parameter_types! {
	pub const BlockHashCount: u64 = 250;
	pub const MaximumBlockWeight: u32 = 1024;
	pub const MaximumBlockLength: u32 = 2 * 1024;
	pub const AvailableBlockRatio: Perbill = Perbill::from_percent(75);
}

pub type AccountId = u32;
impl frame_system::Trait for Test {
	type BaseCallFilter = ();
	type Origin = Origin;
	type Call = ();
	type Index = u64;
	type BlockNumber = u64;
	// type Hash = Hash;
	type Hash = H256;
	type Hashing = ::sp_runtime::traits::BlakeTwo256;
	type AccountId = u32;
	// type Lookup = IdentityLookup<AccountId>;
	type Lookup = IdentityLookup<Self::AccountId>;
	// type Header = generic::Header<BlockNumber, BlakeTwo256>;
	type Header = Header;
	type Event = TestEvent;
	type BlockHashCount = BlockHashCount;
	type MaximumBlockWeight = MaximumBlockWeight;
	// type DbWeight = RocksDbWeight;
	type DbWeight = ();
	// type BlockExecutionWeight = BlockExecutionWeight;
	type BlockExecutionWeight = ();
	// type ExtrinsicBaseWeight = ExtrinsicBaseWeight;
	type ExtrinsicBaseWeight = ();
	type MaximumExtrinsicWeight = MaximumBlockWeight;
	type MaximumBlockLength = MaximumBlockLength;
	type AvailableBlockRatio = AvailableBlockRatio;
	// type Version = Version;
	type Version = ();
	// type PalletInfo = PalletInfo;
	type PalletInfo = ();
	// type AccountData = pallet_balances::AccountData<Balance>;
	type AccountData = ();
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type SystemWeightInfo = ();
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
	pub const GetNativeCurrencyId: CurrencyId = CurrencyId::MINT;
}

type NativeCurrency = Currency<Test, GetNativeCurrencyId>;

impl orml_currencies::Trait for Test {
	type Event = TestEvent;
	type MultiCurrency = orml_tokens::Module<Test>;
	type NativeCurrency = NativeCurrency;
	type GetNativeCurrencyId = GetNativeCurrencyId;
	type WeightInfo = ();
}

pub type System = frame_system::Module<Test>;

parameter_types! {
	pub const LiquidityPoolsModuleId: ModuleId = ModuleId(*b"min/pool");
	pub const InitialExchangeRate: Rate = Rate::from_inner(1_000_000_000_000_000_000);
}

impl Trait for Test {
	type Event = TestEvent;
	type MultiCurrency = orml_tokens::Module<Test>;
	type ModuleId = LiquidityPoolsModuleId;
	type InitialExchangeRate = InitialExchangeRate;
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

type Amount = i128;
pub type TestPools = Module<Test>;
pub const ALICE: AccountId = 1;
pub const DOLLARS: Balance = 1_000_000_000_000_000_000;
pub const ONE_HUNDRED_DOLLARS: Balance = 100 * DOLLARS;
pub const ONE_HUNDRED: Balance = 100;
pub const TEN_THOUSAND: Balance = 10_000 * DOLLARS;

impl ExtBuilder {
	pub fn user_balance(mut self, user: AccountId, currency_id: CurrencyId, balance: Balance) -> Self {
		self.endowed_accounts.push((user, currency_id, balance));
		self
	}

	pub fn pool_balance(mut self, currency_id: CurrencyId, balance: Balance) -> Self {
		self.endowed_accounts
			.push((TestPools::pools_account_id(), currency_id, balance));
		self
	}

	pub fn pool_with_params(
		mut self,
		pool_id: CurrencyId,
		total_borrowed: Balance,
		borrow_index: Rate,
		total_insurance: Balance,
	) -> Self {
		self.pools.push((
			pool_id,
			Pool {
				total_borrowed,
				borrow_index,
				total_insurance,
			},
		));
		self
	}

	pub fn pool_user_data_with_params(
		mut self,
		pool_id: CurrencyId,
		user: AccountId,
		total_borrowed: Balance,
		interest_index: Rate,
		collateral: bool,
	) -> Self {
		self.pool_user_data.push((
			pool_id,
			user,
			PoolUserData {
				total_borrowed,
				interest_index,
				collateral,
			},
		));
		self
	}

	pub fn pool_mock(mut self, pool_id: CurrencyId) -> Self {
		self.pools.push((
			pool_id,
			Pool {
				total_borrowed: Balance::default(),
				borrow_index: Rate::default(),
				total_insurance: Balance::default(),
			},
		));
		self
	}

	pub fn pool_total_borrowed(mut self, pool_id: CurrencyId, total_borrowed: Balance) -> Self {
		self.pools.push((
			pool_id,
			Pool {
				total_borrowed,
				borrow_index: Rate::one(),
				total_insurance: Balance::zero(),
			},
		));
		self
	}

	pub fn build(self) -> sp_io::TestExternalities {
		let mut t = frame_system::GenesisConfig::default().build_storage::<Test>().unwrap();

		orml_tokens::GenesisConfig::<Test> {
			endowed_accounts: self.endowed_accounts,
		}
		.assimilate_storage(&mut t)
		.unwrap();

		GenesisConfig::<Test> {
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
