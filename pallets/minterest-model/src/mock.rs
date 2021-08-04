// Mocks for the minterest-model pallet.
use super::*;
use crate as minterest_model;
use frame_support::{ord_parameter_types, parameter_types, PalletId};
use frame_system::EnsureSignedBy;
use minterest_primitives::currency::OriginalAsset;
pub use minterest_primitives::{Balance, CurrencyId, Price, Rate};
use orml_traits::parameter_type_with_key;
use pallet_traits::PricesManager;
use sp_core::H256;
use sp_runtime::traits::AccountIdConversion;
use sp_runtime::{
	testing::Header,
	traits::{BlakeTwo256, IdentityLookup},
};
pub use test_helper::*;

// -----------------------------------------------------------------------------------------
// 									CONSTRUCT RUNTIME
// -----------------------------------------------------------------------------------------
type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<TestRuntime>;
type Block = frame_system::mocking::MockBlock<TestRuntime>;

// Configure a mock runtime to test the pallet.
frame_support::construct_runtime!(
	pub enum TestRuntime where
		Block = Block,
		NodeBlock = Block,
		UncheckedExtrinsic = UncheckedExtrinsic,
	{
		System: frame_system::{Pallet, Call, Config, Storage, Event<T>},
		Balances: pallet_balances::{Pallet, Call, Storage, Config<T>, Event<T>},
		Tokens: orml_tokens::{Pallet, Storage, Call, Event<T>, Config<T>},
		TestMinterestModel: minterest_model::{Pallet, Storage, Call, Event, Config<T>},
	}
);

ord_parameter_types! {
	pub const OneAlice: AccountId = 1;
}

mock_impl_system_config!(TestRuntime);
mock_impl_orml_tokens_config!(TestRuntime);
mock_impl_minterest_model_config!(TestRuntime, OneAlice);
mock_impl_balances_config!(TestRuntime);

parameter_types! {
	pub const LiquidityPoolsPalletId: PalletId = PalletId(*b"min/lqdy");
	pub LiquidityPoolAccountId: AccountId = LiquidityPoolsPalletId::get().into_account();
	pub InitialExchangeRate: Rate = Rate::one();
}

// -----------------------------------------------------------------------------------------
// 										PRICE SOURCE
// -----------------------------------------------------------------------------------------
pub struct MockPriceSource;

impl PricesManager<OriginalAsset> for MockPriceSource {
	fn get_underlying_price(_asset: OriginalAsset) -> Option<Price> {
		Some(Price::one())
	}
	fn lock_price(_asset: OriginalAsset) {}
	fn unlock_price(_asset: OriginalAsset) {}
}

// -----------------------------------------------------------------------------------------
// 										EXTBUILDER
// -----------------------------------------------------------------------------------------
pub struct ExtBuilder {
	pub minterest_model_params: Vec<(OriginalAsset, MinterestModelData)>,
}

impl Default for ExtBuilder {
	fn default() -> Self {
		Self {
			minterest_model_params: vec![],
		}
	}
}

impl ExtBuilder {
	// Set minterest model parameters
	// - 'asset': currency identifier
	// - 'kink': the utilization point at which the jump multiplier is applied
	// - 'base_rate_per_block': the base interest rate which is the y-intercept when utilization rate is
	//   0
	// - 'multiplier_per_block': the multiplier of utilization rate that gives the slope of the interest
	//   rate
	// - 'jump_multiplier_per_block': the multiplier of utilization rate after hitting a specified
	//   utilization point - kink
	pub fn set_minterest_model_params(
		mut self,
		asset: OriginalAsset,
		kink: Rate,
		base_rate_per_block: Rate,
		multiplier_per_block: Rate,
		jump_multiplier_per_block: Rate,
	) -> Self {
		self.minterest_model_params.push((
			asset,
			MinterestModelData {
				kink,
				base_rate_per_block,
				multiplier_per_block,
				jump_multiplier_per_block,
			},
		));
		self
	}

	// Builds GenesisConfig for all pallets from ExtBuilder data
	pub fn build(self) -> sp_io::TestExternalities {
		let mut t = frame_system::GenesisConfig::default()
			.build_storage::<TestRuntime>()
			.unwrap();

		minterest_model::GenesisConfig::<TestRuntime> {
			minterest_model_params: self.minterest_model_params,
			_phantom: Default::default(),
		}
		.assimilate_storage(&mut t)
		.unwrap();

		let mut ext = sp_io::TestExternalities::new(t);
		ext.execute_with(|| System::set_block_number(1));
		ext
	}
}
