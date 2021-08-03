use super::*;
use crate as controller;
use frame_support::{
	construct_runtime, ord_parameter_types, pallet_prelude::GenesisBuild, parameter_types, sp_io::TestExternalities,
	PalletId,
};
use frame_system::EnsureSignedBy;
use minterest_model::MinterestModelData;
pub(crate) use minterest_primitives::Price;
pub use minterest_primitives::{
	currency::{OriginalAsset, WrapToken},
	Balance, CurrencyId, Interest, Rate,
};
use orml_traits::parameter_type_with_key;
use pallet_traits::PricesManager;
use sp_core::H256;
use sp_runtime::{
	testing::Header,
	traits::{AccountIdConversion, BlakeTwo256, IdentityLookup, One},
};
use sp_std::{cell::RefCell, vec};
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
		UncheckedExtrinsic = UncheckedExtrinsic,
	{
		System: frame_system::{Pallet, Call, Config, Storage, Event<T>},
		Balances: pallet_balances::{Pallet, Call, Storage, Config<T>, Event<T>},
		Tokens: orml_tokens::{Pallet, Storage, Call, Event<T>, Config<T>},
		Currencies: orml_currencies::{Pallet, Call, Event<T>},
		TestPools: liquidity_pools::{Pallet, Storage, Call, Config<T>},
		TestController: controller::{Pallet, Storage, Call, Event, Config<T>},
		TestMinterestModel: minterest_model::{Pallet, Storage, Call, Event, Config<T>},
		TestMntToken: mnt_token::{Pallet, Storage, Call, Event<T>, Config<T>},
	}
);

ord_parameter_types! {
	pub const OneAlice: AccountId = 1;
}

mock_impl_system_config!(TestRuntime);
mock_impl_orml_tokens_config!(TestRuntime);
mock_impl_orml_currencies_config!(TestRuntime);
mock_impl_liquidity_pools_config!(TestRuntime);
mock_impl_minterest_model_config!(TestRuntime, OneAlice);
mock_impl_controller_config!(TestRuntime, OneAlice);
mock_impl_mnt_token_config!(TestRuntime, OneAlice);
mock_impl_balances_config!(TestRuntime);

parameter_types! {
	pub const LiquidityPoolsPalletId: PalletId = PalletId(*b"min/lqdy");
	pub LiquidityPoolAccountId: AccountId = LiquidityPoolsPalletId::get().into_account();
	pub const MntTokenPalletId: PalletId = PalletId(*b"min/mntt");
	pub MntTokenAccountId: AccountId = MntTokenPalletId::get().into_account();
	pub InitialExchangeRate: Rate = Rate::one();
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
	pub endowed_accounts: Vec<(AccountId, CurrencyId, Balance)>,
	pub pools: Vec<(OriginalAsset, PoolData)>,
	pub pool_user_data: Vec<(OriginalAsset, AccountId, PoolUserData)>,
	pub controller_params: Vec<(OriginalAsset, ControllerData<u64>)>,
	pub pause_keepers: Vec<(OriginalAsset, PauseKeeper)>,
	pub minterest_model_params: Vec<(OriginalAsset, MinterestModelData)>,
}

// Default values for ExtBuilder.
// By default you runtime will be configured with this values for corresponding fields.
impl Default for ExtBuilder {
	fn default() -> Self {
		Self {
			endowed_accounts: vec![],
			pools: vec![],
			pool_user_data: vec![],
			controller_params: Vec::new(),
			pause_keepers: vec![],
			minterest_model_params: vec![],
		}
	}
}

impl ExtBuilder {
	// Initialize pool
	// - 'pool_id': pool currency / id
	// - 'borrowed': value of currency borrowed from the pool_id
	// - 'borrow_index': index, describing change of borrow interest rate
	// - 'protocol_interest': interest of the protocol
	pub fn init_pool(
		mut self,
		pool_id: OriginalAsset,
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
		pool_id: OriginalAsset,
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
	// Set controller data for the current pool
	// - 'pool_id': pool id
	// - 'last_interest_accrued_block': block number that interest was last accrued at.
	// - 'protocol_interest_factor': defines the portion of borrower interest that is converted
	// into protocol interest.
	// - 'max_borrow_rate': maximum borrow rate.
	// - 'collateral_factor': this multiplier represents which share of the supplied value can be used
	//   as a collateral for loans. For instance, 0.9 allows 90% of total pool value to be used as a
	//   collateral. Must be between 0 and 1.
	// - 'borrow_cap': maximum total borrow amount per pool in usd. No value means infinite borrow cap.
	// - protocol_interest_threshold': minimum protocol interest needed to transfer it to liquidation
	//   pool
	pub fn set_controller_data(
		mut self,
		pool_id: OriginalAsset,
		last_interest_accrued_block: u64,
		protocol_interest_factor: Rate,
		max_borrow_rate: Rate,
		collateral_factor: Rate,
		borrow_cap: Option<Balance>,
		protocol_interest_threshold: Balance,
	) -> Self {
		self.controller_params.push((
			pool_id,
			ControllerData {
				last_interest_accrued_block,
				protocol_interest_factor,
				max_borrow_rate,
				collateral_factor,
				borrow_cap,
				protocol_interest_threshold,
			},
		));
		self
	}

	// Set pausekeeper state.
	// - 'asset': currency identifier
	// - 'is_paused': pause / unpause all keepers for current currency
	pub fn set_pause_keeper(mut self, asset: OriginalAsset, is_paused: bool) -> Self {
		self.pause_keepers.push((
			asset,
			if is_paused {
				PauseKeeper::all_paused()
			} else {
				PauseKeeper::all_unpaused()
			},
		));
		self
	}

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

	// Set balance for the particular user
	// - 'user': id of users account
	// - 'currency_id': currency
	// - 'balance': balance value to set
	pub fn set_user_balance(mut self, user: AccountId, currency_id: CurrencyId, balance: Balance) -> Self {
		self.endowed_accounts.push((user, currency_id, balance));
		self
	}

	// Set balance for the particular pool
	// - 'pool_id': pool id
	// - 'balance': balance value to set
	pub fn set_pool_balance(mut self, account_id: AccountId, pool_id: OriginalAsset, balance: Balance) -> Self {
		self.endowed_accounts
			//TestPools::pools_account_id()
			.push((account_id, pool_id.into(), balance));
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
				.filter(|&(_, currency_id, _)| currency_id != MNT)
				.collect::<Vec<_>>(),
		}
		.assimilate_storage(&mut t)
		.unwrap();

		controller::GenesisConfig::<TestRuntime> {
			controller_params: self.controller_params,
			pause_keepers: self.pause_keepers,
		}
		.assimilate_storage(&mut t)
		.unwrap();

		liquidity_pools::GenesisConfig::<TestRuntime> {
			pools: self.pools,
			pool_user_data: self.pool_user_data,
		}
		.assimilate_storage(&mut t)
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
