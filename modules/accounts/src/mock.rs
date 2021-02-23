/// Mocks for the accounts pallet.
use crate as test_accounts;
use frame_support::parameter_types;
use frame_system as system;
use sp_core::H256;
use sp_runtime::{
	testing::Header,
	traits::{BlakeTwo256, IdentityLookup},
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
		TestAccounts: test_accounts::{Module, Storage, Call, Event<T>, Config<T>},
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

parameter_types! {
	pub const MaxMembers: u8 = 16;
}

impl test_accounts::Config for Test {
	type Event = Event;
	type MaxMembers = MaxMembers;
}

pub const ALICE: AccountId = 1;
pub const BOB: AccountId = 2;

pub struct ExternalityBuilder;

impl ExternalityBuilder {
	pub fn build() -> sp_io::TestExternalities {
		let storage = system::GenesisConfig::default().build_storage::<Test>().unwrap();
		let mut ext = sp_io::TestExternalities::from(storage);
		ext.execute_with(|| System::set_block_number(1));
		ext
	}
}
