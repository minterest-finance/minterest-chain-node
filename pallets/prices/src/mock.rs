//! Mocks for the prices module.

use super::*;
use crate as module_prices;
use frame_support::{construct_runtime, ord_parameter_types, parameter_types};
use frame_system::EnsureSignedBy;
use sp_core::H256;
use sp_runtime::{
	testing::Header,
	traits::{BlakeTwo256, IdentityLookup, Zero},
	FixedPointNumber,
};
pub use test_helper::*;

// -----------------------------------------------------------------------------------------
// 									CONSTRUCT RUNTIME
// -----------------------------------------------------------------------------------------
type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<TestRuntime>;
type Block = frame_system::mocking::MockBlock<TestRuntime>;

// Configure a mock runtime to test the pallet.
construct_runtime!(
	pub enum TestRuntime where
		Block = Block,
		NodeBlock = Block,
		UncheckedExtrinsic = UncheckedExtrinsic,
	{
		System: frame_system::{Pallet, Call, Config, Storage, Event<T>},
		Balances: pallet_balances::{Pallet, Call, Storage, Config<T>, Event<T>},
		TestPrices: module_prices::{Pallet, Storage, Call, Event<T>},
	}
);

ord_parameter_types! {
	pub const OneAlice: AccountId = 1;
}

mock_impl_system_config!(TestRuntime);
mock_impl_balances_config!(TestRuntime);
mock_impl_prices_module_config!(TestRuntime, OneAlice);

// -----------------------------------------------------------------------------------------
// 										DATA PROVIDER
// -----------------------------------------------------------------------------------------
pub struct MockDataProvider;
impl DataProvider<OriginalAsset, Price> for MockDataProvider {
	fn get(asset: &OriginalAsset) -> Option<Price> {
		match &asset {
			OriginalAsset::MNT => Some(Price::zero()),
			OriginalAsset::BTC => Some(Price::saturating_from_integer(48_000)),
			OriginalAsset::DOT => Some(Price::saturating_from_integer(40)),
			OriginalAsset::ETH => Some(Price::saturating_from_integer(1_500)),
			OriginalAsset::KSM => Some(Price::saturating_from_integer(250)),
		}
	}
}

impl DataFeeder<OriginalAsset, Price, AccountId> for MockDataProvider {
	fn feed_value(_: AccountId, _: OriginalAsset, _: Price) -> sp_runtime::DispatchResult {
		Ok(())
	}
}

// -----------------------------------------------------------------------------------------
// 									EXTBUILDER
// -----------------------------------------------------------------------------------------
pub struct ExtBuilder {
	pub locked_price: Vec<(OriginalAsset, Price)>,
}

impl Default for ExtBuilder {
	fn default() -> Self {
		Self { locked_price: vec![] }
	}
}

impl ExtBuilder {
	// Set locked price for the currency
	// - `asset` : currency identifier
	// - `price`: locked price
	pub fn set_locked_price(mut self, asset: OriginalAsset, price: Price) -> Self {
		self.locked_price.push((asset, price));
		self
	}

	// Build
	pub fn build(self) -> sp_io::TestExternalities {
		let mut t = frame_system::GenesisConfig::default()
			.build_storage::<TestRuntime>()
			.unwrap();

		module_prices::GenesisConfig::<TestRuntime> {
			locked_price: self.locked_price,
			_phantom: Default::default(),
		}
		.assimilate_storage(&mut t)
		.unwrap();

		let mut ext = sp_io::TestExternalities::new(t);
		ext.execute_with(|| System::set_block_number(1));
		ext
	}
}
