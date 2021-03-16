//! Mocks for dex module.

#![cfg(test)]

use super::*;
use crate as dex;
use frame_support::{construct_runtime, parameter_types};
pub use minterest_primitives::CurrencyId;
use orml_traits::parameter_type_with_key;
use sp_runtime::traits::AccountIdConversion;

parameter_types!(
	pub const SomeConst: u64 = 10;
	pub const BlockHashCount: u32 = 250;
);

pub type AccountId = u64;

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

parameter_type_with_key! {
	pub ExistentialDeposits: |_currency_id: CurrencyId| -> Balance {
		Default::default()
	};
}

impl orml_tokens::Config for Runtime {
	type Event = Event;
	type Balance = Balance;
	type Amount = Amount;
	type CurrencyId = CurrencyId;
	type WeightInfo = ();
	type ExistentialDeposits = ExistentialDeposits;
	type OnDust = ();
}

parameter_types! {
	pub const DexModuleId: ModuleId = ModuleId(*b"min/dexs");
	pub DexAccountId: AccountId = DexModuleId::get().into_account();
}

impl dex::Config for Runtime {
	type Event = Event;
	type MultiCurrency = orml_tokens::Module<Runtime>;
	type DexModuleId = DexModuleId;
	type DexAccountId = DexAccountId;
}

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Runtime>;
type Block = frame_system::mocking::MockBlock<Runtime>;

construct_runtime!(
	pub enum Runtime where
		Block = Block,
		NodeBlock = Block,
		UncheckedExtrinsic = UncheckedExtrinsic
	{
		System: frame_system::{Module, Call, Event<T>},
		Tokens: orml_tokens::{Module, Storage, Call, Event<T>, Config<T>},
		TestDex: dex::{Module, Storage, Call, Event<T>},
	}
);

pub const DOLLARS: Balance = 1_000_000_000_000_000_000;
pub fn dollars<T: Into<u128>>(d: T) -> Balance {
	DOLLARS.saturating_mul(d.into())
}

pub struct ExtBuilder {
	endowed_accounts: Vec<(AccountId, CurrencyId, Balance)>,
}

impl Default for ExtBuilder {
	fn default() -> Self {
		Self {
			endowed_accounts: vec![],
		}
	}
}

impl ExtBuilder {
	pub fn dex_balance(mut self, currency_id: CurrencyId, balance: Balance) -> Self {
		self.endowed_accounts
			.push((TestDex::dex_account_id(), currency_id, balance));
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

		let mut ext = sp_io::TestExternalities::new(t);
		ext.execute_with(|| System::set_block_number(1));
		ext
	}
}
