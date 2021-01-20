/// Mocks for the minterest-model pallet.
use frame_support::{impl_outer_event, impl_outer_origin, parameter_types};
use sp_core::H256;
use sp_runtime::{testing::Header, traits::IdentityLookup, Perbill};

use super::*;

impl_outer_origin! {
	pub enum Origin for Test {}
}

mod minterest_model {
	pub use crate::Event;
}

impl_outer_event! {
	pub enum TestEvent for Test {
		frame_system<T>,
		minterest_model,
		accounts<T>,
	}
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Test;

parameter_types! {
	pub const BlockHashCount: u64 = 250;
	pub const MaximumBlockWeight: u32 = 1024;
	pub const MaximumBlockLength: u32 = 2 * 1024;
	pub const AvailableBlockRatio: Perbill = Perbill::from_percent(75);
}

type AccountId = u32;

impl system::Trait for Test {
	type Origin = Origin;
	type Call = ();
	type Index = u64;
	type BlockNumber = u64;
	type Hash = H256;
	type Hashing = ::sp_runtime::traits::BlakeTwo256;
	type AccountId = AccountId;
	type Lookup = IdentityLookup<Self::AccountId>;
	type Header = Header;
	type Event = TestEvent;
	type BlockHashCount = BlockHashCount;
	type MaximumExtrinsicWeight = MaximumBlockWeight;
	type MaximumBlockWeight = MaximumBlockWeight;
	type DbWeight = ();
	type BlockExecutionWeight = ();
	type ExtrinsicBaseWeight = ();
	type MaximumBlockLength = MaximumBlockLength;
	type AvailableBlockRatio = AvailableBlockRatio;
	type Version = ();
	type PalletInfo = ();
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type AccountData = ();
	type BaseCallFilter = ();
	type SystemWeightInfo = ();
}

parameter_types! {
	pub const MaxMembers: u32 = MAX_MEMBERS;
}

impl accounts::Trait for Test {
	type Event = TestEvent;
	type MaxMembers = MaxMembers;
}

parameter_types! {
	pub const BlocksPerYear: u128 = BLOCKS_PER_YEAR;
}

impl Trait for Test {
	type Event = TestEvent;
	type BlocksPerYear = BlocksPerYear;
}

pub type TestMinterestModel = Module<Test>;
pub type System = frame_system::Module<Test>;
pub const BLOCKS_PER_YEAR: u128 = 5_256_000;
pub const MAX_MEMBERS: u32 = 16;
pub const ALICE: AccountId = 1;
pub fn alice() -> Origin {
	Origin::signed(ALICE)
}
pub const BOB: AccountId = 2;
pub fn bob() -> Origin {
	Origin::signed(BOB)
}

pub fn new_test_ext() -> sp_io::TestExternalities {
	let mut t = frame_system::GenesisConfig::default().build_storage::<Test>().unwrap();

	crate::GenesisConfig {
		minterest_model_dates: vec![
			(
				CurrencyId::DOT,
				MinterestModelData {
					kink: Rate::saturating_from_rational(8, 10),
					base_rate_per_block: Rate::zero(),
					multiplier_per_block: Rate::saturating_from_rational(9, 1_000_000_000), // 0.047304 PerYear
					jump_multiplier_per_block: Rate::saturating_from_rational(207, 1_000_000_000), // 1.09 PerYear
				},
			),
			(
				CurrencyId::KSM,
				MinterestModelData {
					kink: Rate::saturating_from_rational(8, 10),
					base_rate_per_block: Rate::zero(),
					multiplier_per_block: Rate::saturating_from_rational(9, 1_000_000_000), // 0.047304 PerYear
					jump_multiplier_per_block: Rate::saturating_from_rational(207, 1_000_000_000), // 1.09 PerYear
				},
			),
			(
				CurrencyId::BTC,
				MinterestModelData {
					kink: Rate::saturating_from_rational(8, 10),
					base_rate_per_block: Rate::zero(),
					multiplier_per_block: Rate::saturating_from_rational(9, 1_000_000_000), // 0.047304 PerYear
					jump_multiplier_per_block: Rate::saturating_from_rational(207, 1_000_000_000), // 1.09 PerYear
				},
			),
		],
	}
	.assimilate_storage(&mut t)
	.unwrap();

	accounts::GenesisConfig::<Test> {
		allowed_accounts: vec![(ALICE, ())],
	}
	.assimilate_storage(&mut t)
	.unwrap();

	let mut ext: sp_io::TestExternalities = t.into();
	ext.execute_with(|| System::set_block_number(1));
	ext
}
