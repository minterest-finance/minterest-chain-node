//! Mocks for the prices module.

use super::*;
use crate as module_prices;
use frame_support::{ord_parameter_types, parameter_types};
use frame_system::EnsureSignedBy;
use minterest_primitives::{Balance, CurrencyId};
use sp_core::H256;
use sp_runtime::{
	testing::Header,
	traits::{BlakeTwo256, IdentityLookup, Zero},
	FixedPointNumber,
};
pub use test_helper::*;

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;

// Configure a mock runtime to test the pallet.
frame_support::construct_runtime!(
	pub enum Test where
		Block = Block,
		NodeBlock = Block,
		UncheckedExtrinsic = UncheckedExtrinsic,
	{
		System: frame_system::{Pallet, Call, Config, Storage, Event<T>},
		Balances: pallet_balances::{Pallet, Call, Storage, Config<T>, Event<T>},
		Prices: module_prices::{Pallet, Storage, Call, Event<T>},
	}
);

mock_impl_system_config!(Test);
mock_impl_balances_config!(Test);

pub struct MockDataProvider;
impl DataProvider<CurrencyId, Price> for MockDataProvider {
	fn get(currency_id: &CurrencyId) -> Option<Price> {
		match currency_id {
			&MNT => Some(Price::zero()),
			&BTC => Some(Price::saturating_from_integer(48_000)),
			&DOT => Some(Price::saturating_from_integer(40)),
			&ETH => Some(Price::saturating_from_integer(1_500)),
			&KSM => Some(Price::saturating_from_integer(250)),
			_ => None,
		}
	}
}

impl DataFeeder<CurrencyId, Price, AccountId> for MockDataProvider {
	fn feed_value(_: AccountId, _: CurrencyId, _: Price) -> sp_runtime::DispatchResult {
		Ok(())
	}
}

ord_parameter_types! {
	pub const One: AccountId = 1;
}

impl module_prices::Config for Test {
	type Event = Event;
	type Source = MockDataProvider;
	type LockOrigin = EnsureSignedBy<One, AccountId>;
	type WeightInfo = ();
}

pub struct ExtBuilder;

impl Default for ExtBuilder {
	fn default() -> Self {
		ExtBuilder
	}
}

impl ExtBuilder {
	pub fn build(self) -> sp_io::TestExternalities {
		let t = frame_system::GenesisConfig::default().build_storage::<Test>().unwrap();

		t.into()
	}
}
