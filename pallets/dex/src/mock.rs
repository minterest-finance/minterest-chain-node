//! Mocks for dex module.

#![cfg(test)]

use super::*;
use crate as dex;
use frame_support::{construct_runtime, ord_parameter_types, parameter_types};
use frame_system::offchain::SendTransactionTypes;
use frame_system::EnsureSignedBy;
pub use minterest_primitives::currency::{OriginalAsset, WrapToken};
pub(crate) use minterest_primitives::{Balance, CurrencyId, Price, Rate};
use orml_traits::parameter_type_with_key;
pub(crate) use pallet_traits::PricesManager;
use sp_core::H256;
use sp_runtime::{
	testing::{Header, TestXt},
	traits::{AccountIdConversion, BlakeTwo256, IdentityLookup, One},
};
use sp_std::cell::RefCell;

pub use test_helper::*;

// -----------------------------------------------------------------------------------------
// 									CONSTRUCT RUNTIME
// -----------------------------------------------------------------------------------------
type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<TestRuntime>;
type Block = frame_system::mocking::MockBlock<TestRuntime>;

construct_runtime!(
	pub enum TestRuntime where
		Block = Block,
		NodeBlock = Block,
		UncheckedExtrinsic = UncheckedExtrinsic
	{
		System: frame_system::{Pallet, Call, Event<T>},
		Balances: pallet_balances::{Pallet, Call, Storage, Event<T>},
		Currencies: orml_currencies::{Pallet, Call, Event<T>},
		Tokens: orml_tokens::{Pallet, Storage, Call, Event<T>, Config<T>},
		TestPools: liquidity_pools::{Pallet, Storage, Call, Config<T>},
		TestLiquidationPools: liquidation_pools::{Pallet, Storage, Call, Event<T>, Config<T>, ValidateUnsigned},
		TestDex: dex::{Pallet, Storage, Call, Event<T>},
	}
);

ord_parameter_types! {
	pub const ZeroAdmin: AccountId = 0;
}

parameter_types! {
	pub const LiquidityPoolsPalletId: PalletId = PalletId(*b"lqdy/min");
	pub const LiquidationPoolsPalletId: PalletId = PalletId(*b"lqdn/min");
	pub LiquidityPoolAccountId: AccountId = LiquidityPoolsPalletId::get().into_account();
	pub LiquidationPoolAccountId: AccountId = LiquidationPoolsPalletId::get().into_account();
	pub InitialExchangeRate: Rate = Rate::one();
}

mock_impl_system_config!(TestRuntime);
mock_impl_orml_tokens_config!(TestRuntime);
mock_impl_orml_currencies_config!(TestRuntime);
mock_impl_liquidity_pools_config!(TestRuntime);
mock_impl_liquidation_pools_config!(TestRuntime);
mock_impl_dex_config!(TestRuntime);
mock_impl_balances_config!(TestRuntime);
// -----------------------------------------------------------------------------------------
// 									MOCK PRICE
// -----------------------------------------------------------------------------------------
thread_local! {
	static UNDERLYING_PRICE: RefCell<Option<Price>> = RefCell::new(Some(Price::one()));
}

pub struct MockPriceSource;
impl MockPriceSource {
	pub fn set_underlying_price(price: Option<Price>) {
		UNDERLYING_PRICE.with(|v| *v.borrow_mut() = price);
	}
}

impl PricesManager<OriginalAsset> for MockPriceSource {
	fn get_underlying_price(_asset: OriginalAsset) -> Option<Price> {
		UNDERLYING_PRICE.with(|v| *v.borrow_mut())
	}
	fn lock_price(_asset: OriginalAsset) {}
	fn unlock_price(_asset: OriginalAsset) {}
}

// -----------------------------------------------------------------------------------------
// 									EXTBUILDER
// -----------------------------------------------------------------------------------------
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
	/// Set balance of the liquidation pool
	/// - 'pool_id': pool id
	/// - 'balance': balance to set
	pub fn set_liquidation_pool_balance(
		mut self,
		pool_account: AccountId,
		pool_id: OriginalAsset,
		balance: Balance,
	) -> Self {
		self.endowed_accounts
			//TestLiquidationPools::pools_account_id()
			.push((pool_account, pool_id.into(), balance));
		self
	}

	/// Set DEX balance
	/// - 'asset': asset
	/// - 'balance': balance value
	pub fn set_dex_balance(mut self, account_id: AccountId, asset: OriginalAsset, balance: Balance) -> Self {
		self.endowed_accounts
			//TestDex::dex_account_id()
			.push((account_id, asset.into(), balance));
		self
	}

	pub fn build(self) -> sp_io::TestExternalities {
		let mut t = frame_system::GenesisConfig::default()
			.build_storage::<TestRuntime>()
			.unwrap();

		orml_tokens::GenesisConfig::<TestRuntime> {
			balances: self.endowed_accounts,
		}
		.assimilate_storage(&mut t)
		.unwrap();

		let mut ext = sp_io::TestExternalities::new(t);
		ext.execute_with(|| System::set_block_number(1));
		ext
	}
}
