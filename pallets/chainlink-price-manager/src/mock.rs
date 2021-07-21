//! Mocks for example module.

#![cfg(test)]

use crate as chainlink_price_adapter;
use frame_support::{construct_runtime, ord_parameter_types, parameter_types, PalletId};
use frame_system::offchain::{SendTransactionTypes, SubmitTransaction};
use frame_system::EnsureSignedBy;
use minterest_primitives::{Balance, Price};
use sp_runtime::testing::Header;
use sp_runtime::testing::TestXt;
use sp_runtime::testing::H256;
use sp_runtime::traits::AccountIdConversion;
use sp_runtime::traits::BlakeTwo256;
use sp_runtime::traits::IdentityLookup;
use test_helper::users_mock::*;
use test_helper::*;

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Runtime>;
type Block = frame_system::mocking::MockBlock<Runtime>;

construct_runtime!(
	pub enum Runtime where
		Block = Block,
		NodeBlock = Block,
		UncheckedExtrinsic = UncheckedExtrinsic
	{
		Balances: pallet_balances::{Pallet, Call, Storage, Config<T>, Event<T>},
		System: frame_system::{Pallet, Call, Event<T>},
		ChainlinkPriceManager: chainlink_price_adapter::{Pallet, Call, Event<T>, Storage},
		ChainlinkFeed: pallet_chainlink_feed::{Pallet, Call, Config<T>, Storage, Event<T>},
	}
);

mock_impl_system_config!(Runtime);
mock_impl_balances_config!(Runtime);

parameter_types! {
	pub const ChainlinkFeedPalletId: PalletId = PalletId(*b"chl/feed");
	pub ChainlinkPalletAccountId: AccountId = ChainlinkFeedPalletId::get().into_account();

	pub const ChainlinkPriceManagerPalletId: PalletId = PalletId(*b"chl/prad");
	pub ChainlinkPriceManagerAccountId: AccountId =  ChainlinkPriceManagerPalletId::get().into_account();
}

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
	type PalletId = ChainlinkFeedPalletId;

	// TODO figure out about appropriate value
	type MinimumReserve = MinimumReserve;
	type StringLimit = StringLimit;
	type OracleCountLimit = OracleCountLimit;
	type FeedLimit = FeedLimit;
	type OnAnswerHandler = ();
	type WeightInfo = ();
}

pub type TransactionPriority = u64;
ord_parameter_types! {
	pub const LiquidityPoolsPriority: TransactionPriority = TransactionPriority::max_value();

}

/// An extrinsic type used for tests.
pub type Extrinsic = TestXt<Call, ()>;

impl<LocalCall> SendTransactionTypes<LocalCall> for Runtime
where
	Call: From<LocalCall>,
{
	type OverarchingCall = Call;
	type Extrinsic = Extrinsic;
}

impl chainlink_price_adapter::Config for Runtime {
	type Event = Event;
	type PalletAccountId = ChainlinkPalletAccountId;
	type UnsignedPriority = LiquidityPoolsPriority;
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

	pallet_chainlink_feed::GenesisConfig::<Runtime> {
		pallet_admin: Some(ADMIN),
		feed_creators: vec![ALICE],
	}
	.assimilate_storage(&mut storage)
	.unwrap();

	let mut test_externalities = sp_io::TestExternalities::new(storage);
	test_externalities.execute_with(|| System::set_block_number(1));
	test_externalities
}
