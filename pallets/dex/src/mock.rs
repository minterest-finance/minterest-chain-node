//! Mocks for dex module.

#![cfg(test)]

use super::*;
use crate as dex;
use frame_support::{construct_runtime, ord_parameter_types, parameter_types, PalletId};
use frame_system::offchain::SendTransactionTypes;
use frame_system::EnsureSignedBy;
pub use minterest_primitives::{
	currency::CurrencyType::{UnderlyingAsset, WrappedToken},
	currency::DOT,
};
pub(crate) use minterest_primitives::{Balance, CurrencyId, Price, Rate};
use orml_traits::parameter_type_with_key;
pub(crate) use pallet_traits::{PoolsManager, PriceProvider};
use sp_core::H256;
use sp_runtime::{
	testing::{Header, TestXt},
	traits::{AccountIdConversion, BlakeTwo256, IdentityLookup},
	FixedPointNumber,
};
pub use test_helper::*;

pub type AccountId = u64;

ord_parameter_types! {
	pub const ZeroAdmin: AccountId = 0;
}

parameter_types! {
	pub const LiquidityPoolsModuleId: PalletId = PalletId(*b"min/lqdy");
	pub const LiquidationPoolsModuleId: PalletId = PalletId(*b"min/lqdn");
	pub LiquidityPoolAccountId: AccountId = LiquidityPoolsModuleId::get().into_account();
	pub LiquidationPoolAccountId: AccountId = LiquidationPoolsModuleId::get().into_account();
	pub InitialExchangeRate: Rate = Rate::one();
	pub EnabledUnderlyingAssetsIds: Vec<CurrencyId> = CurrencyId::get_enabled_tokens_in_protocol(UnderlyingAsset);
	pub EnabledWrappedTokensId: Vec<CurrencyId> = CurrencyId::get_enabled_tokens_in_protocol(WrappedToken);
}

mock_impl_system_config!(Runtime);
mock_impl_orml_tokens_config!(Runtime);
mock_impl_orml_currencies_config!(Runtime);
mock_impl_liquidity_pools_config!(Runtime);
mock_impl_liquidation_pools_config!(Runtime);
mock_impl_dex_config!(Runtime);
mock_impl_balances_config!(Runtime);

pub struct MockPriceSource;

impl PriceProvider<CurrencyId> for MockPriceSource {
	fn get_underlying_price(_currency_id: CurrencyId) -> Option<Price> {
		Some(Price::one())
	}

	fn lock_price(_currency_id: CurrencyId) {}

	fn unlock_price(_currency_id: CurrencyId) {}
}

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Runtime>;
type Block = frame_system::mocking::MockBlock<Runtime>;

construct_runtime!(
	pub enum Runtime where
		Block = Block,
		NodeBlock = Block,
		UncheckedExtrinsic = UncheckedExtrinsic
	{
		System: frame_system::{Module, Call, Event<T>},
		Balances: pallet_balances::{Module, Call, Storage, Config<T>, Event<T>},
		Currencies: orml_currencies::{Module, Call, Event<T>},
		Tokens: orml_tokens::{Module, Storage, Call, Event<T>, Config<T>},
		TestPools: liquidity_pools::{Module, Storage, Call, Config<T>},
		LiquidationPools: liquidation_pools::{Module, Storage, Call, Event<T>, Config<T>, ValidateUnsigned},
		TestDex: dex::{Module, Storage, Call, Event<T>},
	}
);

pub const DOLLARS: Balance = 1_000_000_000_000_000_000;
pub fn dollars<T: Into<u128>>(d: T) -> Balance {
	DOLLARS.saturating_mul(d.into())
}

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
	pub fn _liquidation_pool_balance(mut self, currency_id: CurrencyId, balance: Balance) -> Self {
		self.endowed_accounts
			.push((LiquidationPools::pools_account_id(), currency_id, balance));
		self
	}

	pub fn dex_balance(mut self, currency_id: CurrencyId, balance: Balance) -> Self {
		self.endowed_accounts
			.push((TestDex::dex_account_id(), currency_id, balance));
		self
	}

	pub fn build(self) -> sp_io::TestExternalities {
		let mut t = frame_system::GenesisConfig::default()
			.build_storage::<Runtime>()
			.unwrap();

		orml_tokens::GenesisConfig::<Runtime> {
			endowed_accounts: self.endowed_accounts,
		}
		.assimilate_storage(&mut t)
		.unwrap();

		let mut ext = sp_io::TestExternalities::new(t);
		ext.execute_with(|| System::set_block_number(1));
		ext
	}
}
