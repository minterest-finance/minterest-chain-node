//! Mocks for the whitelist module.
use super::*;
use crate as whitelist_module;
use frame_support::{construct_runtime, ord_parameter_types, parameter_types};
use frame_system::EnsureSignedBy;
use minterest_primitives::Balance;
use sp_core::H256;
use sp_runtime::{
	testing::Header,
	traits::{BlakeTwo256, IdentityLookup},
};
use test_helper::{mock_impl_balances_config, mock_impl_system_config, mock_impl_whitelist_module_config};

pub type AccountId = u64;

pub type Block = sp_runtime::generic::Block<Header, UncheckedExtrinsic>;
pub type UncheckedExtrinsic = sp_runtime::generic::UncheckedExtrinsic<u32, Call, u32, ()>;

// Configure a mock runtime to test the pallet.
construct_runtime!(
	pub enum Test where
		Block = Block,
		NodeBlock = Block,
		UncheckedExtrinsic = UncheckedExtrinsic,
	{
		System: frame_system::{Pallet, Call, Storage, Config, Event<T>},
		Whitelist: whitelist_module::{Pallet, Storage, Call, Event<T>, Config<T>},
		PalletBalances: pallet_balances::{Pallet, Call, Storage, Config<T>, Event<T>},
	}
);

mock_impl_system_config!(Test);
mock_impl_balances_config!(Test);
mock_impl_whitelist_module_config!(Test, ZeroAdmin);

ord_parameter_types! {
	pub const ZeroAdmin: AccountId = 0;
}

pub const ADMIN: AccountId = 0;
pub const ALICE: AccountId = 1;
pub const BOB: AccountId = 2;
pub const CHARLIE: AccountId = 3;
pub fn admin() -> Origin {
	Origin::signed(ADMIN)
}
pub fn alice() -> Origin {
	Origin::signed(ALICE)
}

#[derive(Default)]
pub struct ExternalityBuilder {
	members: Vec<AccountId>,
}

impl ExternalityBuilder {
	pub fn set_members(mut self, members: Vec<AccountId>) -> Self {
		self.members = members;
		self
	}

	pub fn build(self) -> sp_io::TestExternalities {
		let mut storage = frame_system::GenesisConfig::default().build_storage::<Test>().unwrap();

		whitelist_module::GenesisConfig::<Test> {
			members: self.members,
			whitelist_mode: false,
		}
		.assimilate_storage(&mut storage)
		.unwrap();

		let mut ext: sp_io::TestExternalities = storage.into();
		ext.execute_with(|| System::set_block_number(1));
		ext
	}
}
