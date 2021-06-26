//! Mocks for example module.

#![cfg(test)]

use crate as chainlink_price_adapter;
use frame_support::{construct_runtime, ord_parameter_types, parameter_types};
use frame_system::EnsureSignedBy;
use minterest_primitives::Balance;
use sp_runtime::testing::Header;
use sp_runtime::testing::H256;
use sp_runtime::traits::AccountIdConversion;
use sp_runtime::traits::BlakeTwo256;
use sp_runtime::traits::IdentityLookup;
use sp_runtime::ModuleId;
use test_helper::*;

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Runtime>;
type Block = frame_system::mocking::MockBlock<Runtime>;

construct_runtime!(
	pub enum Runtime where
		Block = Block,
		NodeBlock = Block,
		UncheckedExtrinsic = UncheckedExtrinsic
	{
		Balances: pallet_balances::{Module, Call, Storage, Config<T>, Event<T>},
		System: frame_system::{Module, Call, Event<T>},
		ChainlinkPriceAdapter: chainlink_price_adapter::{Module, Call, Event<T>, Storage},
		ChainlinkFeed: pallet_chainlink_feed::{Module, Call, Config<T>, Storage, Event<T>},
	}
);

mock_impl_system_config!(Runtime);
mock_impl_balances_config!(Runtime);

parameter_types! {
	pub const ChainlinkFeedModuleId: ModuleId = ModuleId(*b"chl/feed");
	pub ChainlinkModuleAccountId: AccountId = ChainlinkFeedModuleId::get().into_account();

	pub const ChainlinkPriceAdapterModuleId: ModuleId = ModuleId(*b"chl/prad");
	pub ChainlinkPriceAdapterAccountId: AccountId =  ChainlinkPriceAdapterModuleId::get().into_account();
}

pub const ADMIN: AccountId = 0;
pub fn admin() -> Origin {
	Origin::signed(ADMIN)
}
ord_parameter_types! {
	pub const ZeroAdmin: AccountId = 0;
}

pub type AccountId = u64;
pub type FeedId = u32;

const MIN_RESERVE: u128 = 100000;

parameter_types! {
	pub const StringLimit: u32 = 30;
	pub const OracleCountLimit: u32 = 25;
	pub const FeedLimit: FeedId = 100;
	pub const MinimumReserve: Balance = MIN_RESERVE;
}

impl pallet_chainlink_feed::Config for Runtime {
	type Event = Event;
	type FeedId = u32;
	type Value = u128;
	type Currency = Balances;
	type ModuleId = ChainlinkFeedModuleId;

	// TODO figure out about appropriate value
	type MinimumReserve = MinimumReserve;
	type StringLimit = StringLimit;
	type OracleCountLimit = OracleCountLimit;
	type FeedLimit = FeedLimit;
	type OnAnswerHandler = ();
	type WeightInfo = ();
}

impl chainlink_price_adapter::Config for Runtime {
	type Event = Event;
	type ChainlinkOracle = ChainlinkFeed;
	type PalletAccountId = ChainlinkModuleAccountId;
	type UpdateOrigin = EnsureSignedBy<ZeroAdmin, AccountId>;
}

pub fn test_externalities() -> sp_io::TestExternalities {
	let mut storage = frame_system::GenesisConfig::default()
		.build_storage::<Runtime>()
		.unwrap();

	pallet_balances::GenesisConfig::<Runtime> {
		balances: vec![(ADMIN, MIN_RESERVE)],
	}
	.assimilate_storage(&mut storage)
	.unwrap();

	let chainlink_adapter_account: AccountId = ChainlinkFeedModuleId::get().into_account();
	pallet_chainlink_feed::GenesisConfig::<Runtime> {
		pallet_admin: Some(ADMIN),
		feed_creators: vec![ChainlinkPriceAdapterAccountId::get()],
	}
	.assimilate_storage(&mut storage)
	.unwrap();

	let mut test_externalities = sp_io::TestExternalities::new(storage);
	test_externalities.execute_with(|| System::set_block_number(1));
	test_externalities
}
