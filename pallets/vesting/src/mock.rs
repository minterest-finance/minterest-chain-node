//! Mocks for the vesting module.
use super::*;
use crate as vesting;
use frame_support::{construct_runtime, ord_parameter_types, parameter_types, traits::GenesisBuild};
use frame_system::EnsureSignedBy;
use minterest_primitives::{constants::currency::DOLLARS, AccountId, Balance};
use sp_core::H256;
use sp_runtime::{
	testing::Header,
	traits::{BlakeTwo256, IdentityLookup},
};
use test_helper::{mock_impl_balances_config, mock_impl_system_config};

pub type Block = sp_runtime::generic::Block<Header, UncheckedExtrinsic>;
pub type UncheckedExtrinsic = sp_runtime::generic::UncheckedExtrinsic<u32, Call, u32, ()>;

// Configure a mock runtime to test the pallet.
construct_runtime!(
	pub enum Runtime where
		Block = Block,
		NodeBlock = Block,
		UncheckedExtrinsic = UncheckedExtrinsic,
	{
		System: frame_system::{Pallet, Call, Storage, Config, Event<T>},
		Vesting: vesting::{Pallet, Storage, Call, Event<T>, Config<T>},
		PalletBalances: pallet_balances::{Pallet, Call, Storage, Config<T>, Event<T>},
	}
);

parameter_types! {
	pub const BlockHashCount: u64 = 250;
	pub const MinVestedTransfer: u128 = 5 * DOLLARS;
	pub const MaxVestingSchedules: u32 = 2;
	pub VestingBucketsInfo: Vec<(VestingBucket, u8, u8, Balance)> = VestingBucket::get_vesting_buckets_info();
}

ord_parameter_types! {
	pub const ADMIN: AccountId = AccountId::from([0u8; 32]);
	pub const ALICE: AccountId = AccountId::from([1u8; 32]);
	pub const BOB: AccountId = AccountId::from([2u8; 32]);
	pub const CHARLIE: AccountId = AccountId::from([3u8; 32]);
	pub const BucketMarketing: AccountId = VestingBucket::Marketing.bucket_account_id().unwrap();
	pub const BucketTeam: AccountId = VestingBucket::Team.bucket_account_id().unwrap();
	pub const BucketStrategicPartners: AccountId = VestingBucket::StrategicPartners.bucket_account_id().unwrap();
}

mock_impl_balances_config!(Runtime);
mock_impl_system_config!(Runtime, AccountId);

impl Config for Runtime {
	type Event = Event;
	type Currency = PalletBalances;
	type MinVestedTransfer = MinVestedTransfer;
	type VestedTransferOrigin = EnsureSignedBy<ADMIN, AccountId>;
	type WeightInfo = ();
	type MaxVestingSchedules = MaxVestingSchedules;
	type VestingBucketsInfo = VestingBucketsInfo;
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
				(BucketMarketing::get(), 1000 * DOLLARS),
				(BucketStrategicPartners::get(), 1000 * DOLLARS),
				(BucketTeam::get(), 1000 * DOLLARS),
			],
		}
		.assimilate_storage(&mut t)
		.unwrap();

		vesting::GenesisConfig::<Runtime> {
			vesting: vec![(VestingBucket::PrivateSale, CHARLIE::get(), 20 * DOLLARS)], // bucket, who, amount
		}
		.assimilate_storage(&mut t)
		.unwrap();

		t.into()
	}
}
