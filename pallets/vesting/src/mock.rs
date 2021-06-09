//! Mocks for the vesting module.

#![cfg(test)]

use super::*;
use frame_support::{construct_runtime, ord_parameter_types, parameter_types, traits::GenesisBuild};
use frame_system::EnsureSignedBy;
use sp_core::H256;
use sp_runtime::{testing::Header, traits::IdentityLookup};

use crate as vesting;
use minterest_primitives::constants::currency::DOLLARS;
use minterest_primitives::{AccountId, Balance};

parameter_types! {
	pub const BlockHashCount: u64 = 250;
}

impl frame_system::Config for Runtime {
	type Origin = Origin;
	type Call = Call;
	type Index = u64;
	type BlockNumber = u64;
	type Hash = H256;
	type Hashing = ::sp_runtime::traits::BlakeTwo256;
	type AccountId = AccountId;
	type Lookup = IdentityLookup<Self::AccountId>;
	type Header = Header;
	type Event = Event;
	type BlockHashCount = BlockHashCount;
	type BlockWeights = ();
	type BlockLength = ();
	type Version = ();
	type PalletInfo = PalletInfo;
	type AccountData = pallet_balances::AccountData<u128>;
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type DbWeight = ();
	type BaseCallFilter = ();
	type SystemWeightInfo = ();
	type SS58Prefix = ();
}

parameter_types! {
	pub const ExistentialDeposit: u128 = DOLLARS; // 1 MNT token
	pub const MinVestedTransfer: u128 = 5 * DOLLARS;
	pub const MaxVestingSchedules: u32 = 2;
}

impl pallet_balances::Config for Runtime {
	type Balance = Balance;
	type DustRemoval = ();
	type Event = Event;
	type ExistentialDeposit = ExistentialDeposit;
	type AccountStore = frame_system::Module<Runtime>;
	type MaxLocks = ();
	type WeightInfo = ();
}

ord_parameter_types! {
	pub const ADMIN: AccountId = AccountId::from([0u8; 32]);
}

impl Config for Runtime {
	type Event = Event;
	type Currency = PalletBalances;
	type MinVestedTransfer = MinVestedTransfer;
	type VestedTransferOrigin = EnsureSignedBy<ADMIN, AccountId>;
	type WeightInfo = ();
	type MaxVestingSchedules = MaxVestingSchedules;
}

pub type Block = sp_runtime::generic::Block<Header, UncheckedExtrinsic>;
pub type UncheckedExtrinsic = sp_runtime::generic::UncheckedExtrinsic<u32, Call, u32, ()>;

construct_runtime!(
	pub enum Runtime where
		Block = Block,
		NodeBlock = Block,
		UncheckedExtrinsic = UncheckedExtrinsic,
	{
		System: frame_system::{Module, Call, Storage, Config, Event<T>},
		Vesting: vesting::{Module, Storage, Call, Event<T>, Config<T>},
		PalletBalances: pallet_balances::{Module, Call, Storage, Config<T>, Event<T>},
	}
);

parameter_types! {
	pub ALICE: AccountId = AccountId::from([1u8; 32]);
	pub BOB: AccountId = AccountId::from([2u8; 32]);
	pub CHARLIE: AccountId = AccountId::from([3u8; 32]);

}

#[derive(Default)]
pub struct ExtBuilder;

impl ExtBuilder {
	pub fn build() -> sp_io::TestExternalities {
		let mut t = frame_system::GenesisConfig::default()
			.build_storage::<Runtime>()
			.unwrap();

		pallet_balances::GenesisConfig::<Runtime> {
			balances: vec![
				(ALICE::get(), 100 * DOLLARS),
				(CHARLIE::get(), 30 * DOLLARS),
				(VestingBucket::Marketing.bucket_account_id().unwrap(), 1000 * DOLLARS),
				(
					VestingBucket::StrategicPartners.bucket_account_id().unwrap(),
					1000 * DOLLARS,
				),
				(VestingBucket::Team.bucket_account_id().unwrap(), 1000 * DOLLARS),
			],
		}
		.assimilate_storage(&mut t)
		.unwrap();

		vesting::GenesisConfig::<Runtime> {
			vesting: vec![(VestingBucket::Team, CHARLIE::get(), 20 * DOLLARS)], // who, start, amount
		}
		.assimilate_storage(&mut t)
		.unwrap();

		t.into()
	}
}
