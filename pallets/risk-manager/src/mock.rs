// Mocks for the RiskManager pallet.
use super::*;
use crate as risk_manager;
use frame_support::{ord_parameter_types, pallet_prelude::GenesisBuild, parameter_types, PalletId};
use frame_system::EnsureSignedBy;
use liquidity_pools::{Pool, PoolUserData};
use minterest_primitives::currency::CurrencyType::{UnderlyingAsset, WrappedToken};
pub use minterest_primitives::{Balance, Price, Rate};
use orml_traits::parameter_type_with_key;
use pallet_traits::PricesManager;
use sp_core::H256;
use sp_runtime::{
	testing::Header,
	traits::{AccountIdConversion, BlakeTwo256, IdentityLookup},
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
		Currencies: orml_currencies::{Pallet, Call, Event<T>},
		TestPools: liquidity_pools::{Pallet, Storage, Call, Config<T>},
		TestRiskManager: risk_manager::{Pallet, Storage, Call, Event<T>, Config<T>},
	}
);

ord_parameter_types! {
	pub const ZeroAdmin: AccountId = 0;
}

parameter_types! {
	pub const LiquidityPoolsPalletId: PalletId = PalletId(*b"lqdi/min");
	pub const LiquidationPoolsPalletId: PalletId = PalletId(*b"lqdn/min");
	pub LiquidityPoolAccountId: AccountId = LiquidityPoolsPalletId::get().into_account();
	pub LiquidationPoolAccountId: AccountId = LiquidationPoolsPalletId::get().into_account();
	pub InitialExchangeRate: Rate = Rate::one();
	pub EnabledUnderlyingAssetsIds: Vec<CurrencyId> = CurrencyId::get_enabled_tokens_in_protocol(UnderlyingAsset);
	pub EnabledWrappedTokensId: Vec<CurrencyId> = CurrencyId::get_enabled_tokens_in_protocol(WrappedToken);
}

mock_impl_system_config!(TestRuntime);
mock_impl_balances_config!(TestRuntime);
mock_impl_orml_tokens_config!(TestRuntime);
mock_impl_orml_currencies_config!(TestRuntime);
mock_impl_liquidity_pools_config!(TestRuntime);
mock_impl_risk_manager_config!(TestRuntime, ZeroAdmin);

// -----------------------------------------------------------------------------------------
// 										PRICE SOURCE
// -----------------------------------------------------------------------------------------
pub struct MockPriceSource;
impl PricesManager<CurrencyId> for MockPriceSource {
	fn get_underlying_price(_currency_id: CurrencyId) -> Option<Price> {
		Some(Price::one())
	}

	fn lock_price(_currency_id: CurrencyId) {}

	fn unlock_price(_currency_id: CurrencyId) {}
}

// -----------------------------------------------------------------------------------------
// 										EXTBUILDER
// -----------------------------------------------------------------------------------------
#[derive(Default)]
pub struct ExtBuilder {
	pools: Vec<(CurrencyId, Pool)>,
	pool_user_data: Vec<(CurrencyId, AccountId, PoolUserData)>,
}

impl ExtBuilder {
	// Set user data for particular pool
	// - 'pool_id': pool id
	// - 'user': user id
	// - 'borrowed': total balance (with accrued interest), after applying the most recent
	//   balance-changing action.
	// - 'interest_index': global borrow_index as of the most recent balance-changing action
	// - 'is_collateral': can pool be used as collateral for the current user
	// - 'liquidation_attempts': number of partial liquidations for debt
	pub fn set_pool_user_data(
		mut self,
		pool_id: CurrencyId,
		user: AccountId,
		borrowed: Balance,
		interest_index: Rate,
		is_collateral: bool,
	) -> Self {
		self.pool_user_data.push((
			pool_id,
			user,
			PoolUserData {
				borrowed,
				interest_index,
				is_collateral,
			},
		));
		self
	}

	// Build
	pub fn build(self) -> sp_io::TestExternalities {
		let mut storage = frame_system::GenesisConfig::default()
			.build_storage::<TestRuntime>()
			.unwrap();

		liquidity_pools::GenesisConfig::<TestRuntime> {
			pools: self.pools,
			pool_user_data: self.pool_user_data,
		}
		.assimilate_storage(&mut storage)
		.unwrap();

		let mut ext: sp_io::TestExternalities = storage.into();
		ext.execute_with(|| System::set_block_number(1));
		ext
	}
}
