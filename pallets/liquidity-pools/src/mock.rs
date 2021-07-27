#![cfg(test)]

use super::*;
use crate as liquidity_pools;
use frame_support::{
	construct_runtime, pallet_prelude::GenesisBuild, parameter_types, sp_io::TestExternalities, PalletId,
};

pub use minterest_primitives::currency::CurrencyType::WrappedToken;
use minterest_primitives::Price;
pub use minterest_primitives::{Balance, CurrencyId};
use orml_traits::parameter_type_with_key;
use pallet_traits::PricesManager;
use sp_core::H256;
use sp_runtime::{
	testing::Header,
	traits::{BlakeTwo256, IdentityLookup},
};
use sp_std::{cell::RefCell, vec};

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
		Tokens: orml_tokens::{Pallet, Storage, Call, Event<T>, Config<T>},
		Currencies: orml_currencies::{Pallet, Call, Event<T>},
		TestPools: liquidity_pools::{Pallet, Storage, Call, Config<T>},
	}
);

mock_impl_system_config!(TestRuntime);
mock_impl_orml_tokens_config!(TestRuntime);
mock_impl_orml_currencies_config!(TestRuntime);
mock_impl_liquidity_pools_config!(TestRuntime);
mock_impl_balances_config!(TestRuntime);

parameter_types! {
	pub const LiquidityPoolsPalletId: PalletId = PalletId(*b"min/lqdy");
	pub LiquidityPoolAccountId: AccountId = LiquidityPoolsPalletId::get().into_account();
	pub InitialExchangeRate: Rate = Rate::one();
	pub EnabledUnderlyingAssetsIds: Vec<CurrencyId> = CurrencyId::get_enabled_tokens_in_protocol(UnderlyingAsset);
	pub EnabledWrappedTokensId: Vec<CurrencyId> = CurrencyId::get_enabled_tokens_in_protocol(WrappedToken);
}

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

impl PricesManager<CurrencyId> for MockPriceSource {
	fn get_underlying_price(_currency_id: CurrencyId) -> Option<Price> {
		UNDERLYING_PRICE.with(|v| *v.borrow_mut())
	}
	fn lock_price(_currency_id: CurrencyId) {}
	fn unlock_price(_currency_id: CurrencyId) {}
}

// -----------------------------------------------------------------------------------------
// 									EXTBUILDER
// -----------------------------------------------------------------------------------------
pub struct ExtBuilder {
	pub endowed_accounts: Vec<(AccountId, CurrencyId, Balance)>,
	pub pools: Vec<(CurrencyId, PoolData)>,
	pub pool_user_data: Vec<(CurrencyId, AccountId, PoolUserData)>,
}

// Default values for ExtBuilder.
// By default you runtime will be configured with this values for corresponding fields.
impl Default for ExtBuilder {
	fn default() -> Self {
		Self {
			endowed_accounts: vec![],
			pools: vec![],
			pool_user_data: vec![],
		}
	}
}

impl ExtBuilder {
	// Initialize pool with default parameters:
	// borrowed: 0, borrow_index: 1, protocol_interest: 0
	// - 'pool_id': pool currency / id
	pub fn init_pool_default(mut self, pool_id: CurrencyId) -> Self {
		self.pools.push((
			pool_id,
			PoolData {
				borrowed: Balance::default(),
				borrow_index: Rate::default(),
				protocol_interest: Balance::default(),
			},
		));
		self
	}

	// Initialize pool
	// - 'pool_id': pool currency / id
	// - 'borrowed': value of currency borrowed from the pool_id
	// - 'borrow_index': index, describing change of borrow interest rate
	// - 'protocol_interest': interest of the protocol
	pub fn init_pool(
		mut self,
		pool_id: CurrencyId,
		borrowed: Balance,
		borrow_index: Rate,
		protocol_interest: Balance,
	) -> Self {
		self.pools.push((
			pool_id,
			PoolData {
				borrowed,
				borrow_index,
				protocol_interest,
			},
		));
		self
	}

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

	// Set balance for the particular pool
	// - 'currency_id': pool id
	// - 'balance': balance value to set
	pub fn set_pool_balance(mut self, account_id: AccountId, currency_id: CurrencyId, balance: Balance) -> Self {
		self.endowed_accounts
			//TestPools::pools_account_id()
			.push((account_id, currency_id, balance));
		self
	}

	// Set balance for the particular user
	// - 'user': id of users account
	// - 'currency_id': currency
	// - 'balance': balance value to set
	pub fn set_user_balance(mut self, user: AccountId, currency_id: CurrencyId, balance: Balance) -> Self {
		self.endowed_accounts.push((user, currency_id, balance));
		self
	}

	// Build Externalities
	pub fn build(self) -> TestExternalities {
		let mut t = frame_system::GenesisConfig::default()
			.build_storage::<TestRuntime>()
			.unwrap();

		orml_tokens::GenesisConfig::<TestRuntime> {
			balances: self
				.endowed_accounts
				.into_iter()
				.filter(|(_, currency_id, _)| *currency_id != MNT)
				.collect::<Vec<_>>(),
		}
		.assimilate_storage(&mut t)
		.unwrap();

		liquidity_pools::GenesisConfig::<TestRuntime> {
			pools: self.pools,
			pool_user_data: self.pool_user_data,
		}
		.assimilate_storage(&mut t)
		.unwrap();

		let mut ext = sp_io::TestExternalities::new(t);
		ext.execute_with(|| System::set_block_number(1));
		ext
	}
}
