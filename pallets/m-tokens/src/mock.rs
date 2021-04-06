/// Mocks for the m-tokens module.
use crate as m_tokens;
use frame_support::pallet_prelude::GenesisBuild;
use frame_support::parameter_types;
use frame_system as system;
pub use minterest_primitives::{Balance, CurrencyId};
use orml_currencies::Currency;
use orml_traits::parameter_type_with_key;
use sp_core::H256;
use sp_runtime::{
	testing::Header,
	traits::{BlakeTwo256, IdentityLookup},
};
use test_helper::{
	mock_impl_system_config,
	mock_impl_orml_tokens_config,
	mock_impl_orml_currencies_config,
};

pub type AccountId = u64;
type Amount = i128;

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Runtime>;
type Block = frame_system::mocking::MockBlock<Runtime>;

// Configure a mock runtime to test the pallet.
frame_support::construct_runtime!(
	pub enum Runtime where
		Block = Block,
		NodeBlock = Block,
		UncheckedExtrinsic = UncheckedExtrinsic,
	{
		System: frame_system::{Module, Call, Config, Storage, Event<T>},
		Tokens: orml_tokens::{Module, Storage, Call, Event<T>, Config<T>},
		Currencies: orml_currencies::{Module, Call, Event<T>},
		MTokens: m_tokens::{Module, Storage, Call, Event<T>},
	}
);

mock_impl_system_config!(Runtime);
mock_impl_orml_tokens_config!(Runtime);
mock_impl_orml_currencies_config!(Runtime, CurrencyId::MNT);

impl m_tokens::Config for Runtime {
	type Event = Event;
	type MultiCurrency = orml_currencies::Module<Runtime>;
}

pub const ALICE: AccountId = 1;
pub const BOB: AccountId = 2;
pub const ONE_MILL: Balance = 1_000_000;

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
	pub fn balances(mut self, endowed_accounts: Vec<(AccountId, CurrencyId, Balance)>) -> Self {
		self.endowed_accounts = endowed_accounts;
		self
	}

	pub fn one_million_mnt_and_mdot_for_alice(self) -> Self {
		self.balances(vec![
			(ALICE, CurrencyId::MNT, ONE_MILL),
			(ALICE, CurrencyId::MDOT, ONE_MILL),
		])
	}

	pub fn build(self) -> sp_io::TestExternalities {
		let mut storage = frame_system::GenesisConfig::default()
			.build_storage::<Runtime>()
			.unwrap();

		orml_tokens::GenesisConfig::<Runtime> {
			endowed_accounts: self.endowed_accounts,
		}
		.assimilate_storage(&mut storage)
		.unwrap();

		let mut ext = sp_io::TestExternalities::new(storage);
		ext.execute_with(|| System::set_block_number(1));
		ext
	}
}
