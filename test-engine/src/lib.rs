pub use test_helper::*;

use controller::{ControllerData, PauseKeeper};
use liquidation_pools::LiquidationPoolData;
use liquidity_pools::{Pool, PoolUserData};
//use minterest_model::MinterestModelData;

use frame_support::{
	ord_parameter_types,
	pallet_prelude::{GenesisBuild, TransactionPriority},
	parameter_types,
	traits::Contains,
};
pub use frame_system::{offchain::SendTransactionTypes, EnsureSignedBy};

pub use minterest_primitives::{
	currency::CurrencyType::{UnderlyingAsset, WrappedToken},
	Balance, CurrencyId, Price, Rate,
};
use orml_traits::parameter_type_with_key;
use pallet_traits::{PoolsManager, PricesManager};
use sp_runtime::{
	testing::{Header, TestXt, H256},
	traits::{AccountIdConversion, BlakeTwo256, IdentityLookup, Zero},
	FixedPointNumber, ModuleId,
};
use sp_std::{cell::RefCell, marker::PhantomData};

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<TestRuntime>;
type Block = frame_system::mocking::MockBlock<TestRuntime>;

frame_support::construct_runtime!(
	pub enum TestRuntime where
		Block = Block,
		NodeBlock = Block,
		UncheckedExtrinsic = UncheckedExtrinsic,
	{
		System: frame_system::{Module, Call, Config, Storage, Event<T>},
		Balances: pallet_balances::{Module, Call, Storage, Config<T>, Event<T>},
		Tokens: orml_tokens::{Module, Storage, Call, Event<T>, Config<T>},
		Currencies: orml_currencies::{Module, Call, Event<T>},
		MinterestProtocol: minterest_protocol::{Module, Storage, Call, Event<T>},
		TestPools: liquidity_pools::{Module, Storage, Call, Config<T>},
		TestLiquidationPools: liquidation_pools::{Module, Storage, Call, Event<T>, Config<T>},
		TestController: controller::{Module, Storage, Call, Event, Config<T>},
		TestMinterestModel: minterest_model::{Module, Storage, Call, Event, Config<T>},
		TestDex: dex::{Module, Storage, Call, Event<T>},
		TestMntToken: mnt_token::{Module, Storage, Call, Event<T>, Config<T>},
		TestRiskManager: risk_manager::{Module, Storage, Call, Event<T>, Config<T>},
	}
);

parameter_types! {
		pub const LiquidityPoolsModuleId: ModuleId = ModuleId(*b"lqdy/min");
		pub const LiquidationPoolsModuleId: ModuleId = ModuleId(*b"lqdn/min");
		pub const MntTokenModuleId: ModuleId = ModuleId(*b"min/mntt");
		pub LiquidityPoolAccountId: AccountId = LiquidityPoolsModuleId::get().into_account();
		pub LiquidationPoolAccountId: AccountId = LiquidationPoolsModuleId::get().into_account();
		pub MntTokenAccountId: AccountId = MntTokenModuleId::get().into_account();
		pub InitialExchangeRate: Rate = Rate::one();
		pub EnabledUnderlyingAssetsIds: Vec<CurrencyId> = CurrencyId::get_enabled_tokens_in_protocol(UnderlyingAsset);
		pub EnabledWrappedTokensId: Vec<CurrencyId> = CurrencyId::get_enabled_tokens_in_protocol(WrappedToken);
}

thread_local! {
	static UNDERLYING_PRICE: RefCell<Option<Price>> = RefCell::new(Some(Price::one()));
	static TWO: RefCell<Vec<u64>> = RefCell::new(vec![2]);
}

ord_parameter_types! {
	pub const ZeroAdmin: AccountId = 0;
}

pub struct WhitelistMembers;

impl Contains<u64> for WhitelistMembers {
	fn contains(who: &AccountId) -> bool {
		TWO.with(|v| v.borrow().contains(who))
	}

	fn sorted_members() -> Vec<u64> {
		TWO.with(|v| v.borrow().clone())
	}

	#[cfg(feature = "runtime-benchmarks")]
	fn add(new: &u128) {
		TWO.with(|v| {
			let mut members = v.borrow_mut();
			members.push(*new);
			members.sort();
		})
	}
}

mock_impl_system_config!(TestRuntime);
mock_impl_balances_config!(TestRuntime);
mock_impl_orml_tokens_config!(TestRuntime);
mock_impl_orml_currencies_config!(TestRuntime);
mock_impl_liquidity_pools_config!(TestRuntime);
mock_impl_liquidation_pools_config!(TestRuntime);
mock_impl_controller_config!(TestRuntime, ZeroAdmin);
mock_impl_minterest_model_config!(TestRuntime, ZeroAdmin);
mock_impl_dex_config!(TestRuntime);
mock_impl_minterest_protocol_config!(TestRuntime, ZeroAdmin);
mock_impl_mnt_token_config!(TestRuntime, ZeroAdmin);
mock_impl_risk_manager_config!(TestRuntime, ZeroAdmin);

pub struct MockPriceSource;

impl MockPriceSource {
	fn set_underlying_price(price: Option<Price>) {
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

//TODO: Rename to ExtBuilder after full tests rework
pub struct ExtBuilderNew {
	pub endowed_accounts: Vec<(AccountId, CurrencyId, Balance)>,
	pub pools: Vec<(CurrencyId, Pool)>,
	pub pool_user_data: Vec<(CurrencyId, AccountId, PoolUserData)>,
	pub liquidation_pools: Vec<(CurrencyId, LiquidationPoolData)>,
	pub minted_pools: Vec<(CurrencyId, Balance)>,
	pub mnt_claim_threshold: Balance,
	pub controller_dates: Vec<(CurrencyId, ControllerData<BlockNumber>)>,
	pub pause_keepers: Vec<(CurrencyId, PauseKeeper)>,
	pub whitelist_mode: bool,
	//pub minterest_model_params: Vec<(CurrencyId, MinterestModelData)>,
}

impl Default for ExtBuilderNew {
	fn default() -> Self {
		Self {
			endowed_accounts: vec![],
			pools: vec![],
			pool_user_data: vec![],
			liquidation_pools: vec![],
			minted_pools: vec![],
			mnt_claim_threshold: Balance::zero(),
			controller_dates: Vec::new(),
			pause_keepers: vec![],
			whitelist_mode: false,
			//minterest_model_params: vec![],
		}
	}
}

pub trait BuildExternalities {
	fn build(self) -> sp_io::TestExternalities;
}

impl BuildExternalities for ExtBuilderNew {
	fn build(self) -> sp_io::TestExternalities {
		let mut t = frame_system::GenesisConfig::default()
			.build_storage::<TestRuntime>()
			.unwrap();

		pallet_balances::GenesisConfig::<TestRuntime> {
			balances: self
				.endowed_accounts
				.clone()
				.into_iter()
				.filter(|(_, currency_id, _)| *currency_id == MNT)
				.map(|(account_id, _, initial_balance)| (account_id, initial_balance))
				.collect::<Vec<_>>(),
		}
		.assimilate_storage(&mut t)
		.unwrap();

		orml_tokens::GenesisConfig::<TestRuntime> {
			endowed_accounts: self
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

		liquidation_pools::GenesisConfig::<TestRuntime> {
			liquidation_pools: self.liquidation_pools,
			phantom: PhantomData,
		}
		.assimilate_storage(&mut t)
		.unwrap();

		mnt_token::GenesisConfig::<TestRuntime> {
			mnt_claim_threshold: self.mnt_claim_threshold,
			minted_pools: self.minted_pools,
			_phantom: PhantomData,
		}
		.assimilate_storage(&mut t)
		.unwrap();

		controller::GenesisConfig::<TestRuntime> {
			controller_dates: self.controller_dates,
			pause_keepers: self.pause_keepers,
			whitelist_mode: self.whitelist_mode,
		}
		.assimilate_storage(&mut t)
		.unwrap();

		let mut ext = sp_io::TestExternalities::new(t);
		ext.execute_with(|| System::set_block_number(1));
		ext
	}
}
//// ---------------

pub trait BalanceTestConfigurator {
	// TODO: refactor using generics
	// TODO: Add description
	fn user_balance(self, user: AccountId, currency_id: CurrencyId, balance: Balance) -> Self;
	// TODO: Add description
	fn pool_balance(self, currency_id: CurrencyId, balance: Balance) -> Self;
	// TODO: Add description
	fn dex_balance(self, currency_id: CurrencyId, balance: Balance) -> Self;
}

// pool_moc -> pool_init_defaults
// pool_total_borrowed -> pool_init_params
// liquidity_pool -> pool_init
pub trait PoolTestConfigurator {
	///TODO: Add description
	fn pool_init_default(self, pool_id: CurrencyId) -> Self;
	///TODO: Add description
	fn pool_init_params(
		self,
		pool_id: CurrencyId,
		total_borrowed: Balance,
		borrow_index: Rate,
		total_protocol_interest: Balance,
	) -> Self;
	// TODO: Add description
	fn pool_user_data(
		self,
		pool_id: CurrencyId,
		user: AccountId,
		total_borrowed: Balance,
		interest_index: Rate,
		is_collateral: bool,
		liquidation_attempts: u8,
	) -> Self;
}

pub trait LiqudationPoolTestConfigurator {
	// TODO: Add description
	fn liquidation_pool_init_param(
		self,
		pool_id: CurrencyId,
		deviation_threshold: Rate,
		balance_ratio: Rate,
		max_ideal_balance: Option<Balance>,
	) -> Self;
	// TODO: Add description
	fn liquidation_pool_balance(self, currency_id: CurrencyId, balance: Balance) -> Self;
}

pub trait MntTestConfigurator {
	// TODO: Add description
	fn mnt_enabled_pools(self, pools: Vec<(CurrencyId, Balance)>) -> Self;
	// TODO: Add description
	fn enable_minting_for_all_pools(self, speed: Balance) -> Self;
	// TODO: Add description
	fn mnt_claim_threshold(self, threshold: Balance) -> Self;
	// TODO: Add description
	fn mnt_account_balance(self, balance: Balance) -> Self;
}

pub trait ControllerTestConfigurator {
	// TODO: Add description
	fn controller_data(
		self,
		currency_id: CurrencyId,
		last_interest_accrued_block: BlockNumber,
		protocol_interest_factor: Rate,
		max_borrow_rate: Rate,
		collateral_factor: Rate,
		borrow_cap: Option<Balance>,
		protocol_interest_threshold: Balance,
	) -> Self;
	// TODO: Add description for
	fn pause_keeper(self, currency_id: CurrencyId, is_paused: bool) -> Self;
	// TODO: Add description for
	fn white_list(self, is_activated: bool) -> Self;
}

// ######################
// TRAITS IMPLEMENTATION
// ######################
impl BalanceTestConfigurator for ExtBuilderNew {
	fn user_balance(mut self, user: AccountId, currency_id: CurrencyId, balance: Balance) -> Self {
		self.endowed_accounts.push((user, currency_id, balance));
		self
	}

	fn pool_balance(mut self, currency_id: CurrencyId, balance: Balance) -> Self {
		self.endowed_accounts
			.push((TestPools::pools_account_id(), currency_id, balance));
		self
	}

	fn dex_balance(mut self, currency_id: CurrencyId, balance: Balance) -> Self {
		self.endowed_accounts
			.push((TestDex::dex_account_id(), currency_id, balance));
		self
	}
}

impl PoolTestConfigurator for ExtBuilderNew {
	fn pool_init_default(mut self, pool_id: CurrencyId) -> Self {
		self.pools.push((
			pool_id,
			Pool {
				total_borrowed: Balance::zero(),
				borrow_index: Rate::one(),
				total_protocol_interest: Balance::zero(),
			},
		));
		self
	}

	fn pool_init_params(
		mut self,
		pool_id: CurrencyId,
		total_borrowed: Balance,
		borrow_index: Rate,
		total_protocol_interest: Balance,
	) -> Self {
		self.pools.push((
			pool_id,
			Pool {
				total_borrowed,
				borrow_index,
				total_protocol_interest,
			},
		));
		self
	}

	fn pool_user_data(
		mut self,
		pool_id: CurrencyId,
		user: AccountId,
		total_borrowed: Balance,
		interest_index: Rate,
		is_collateral: bool,
		liquidation_attempts: u8,
	) -> Self {
		self.pool_user_data.push((
			pool_id,
			user,
			PoolUserData {
				total_borrowed,
				interest_index,
				is_collateral,
				liquidation_attempts,
			},
		));
		self
	}
}

impl MntTestConfigurator for ExtBuilderNew {
	fn mnt_enabled_pools(mut self, pools: Vec<(CurrencyId, Balance)>) -> Self {
		self.minted_pools = pools;
		self
	}

	fn enable_minting_for_all_pools(mut self, speed: Balance) -> Self {
		self.minted_pools = vec![(KSM, speed), (DOT, speed), (ETH, speed), (BTC, speed)];
		self
	}

	fn mnt_claim_threshold(mut self, threshold: u128) -> Self {
		self.mnt_claim_threshold = threshold * DOLLARS;
		self
	}

	fn mnt_account_balance(mut self, balance: Balance) -> Self {
		self.endowed_accounts
			.push((TestMntToken::get_account_id(), MNT, balance));
		self
	}
}

impl LiqudationPoolTestConfigurator for ExtBuilderNew {
	fn liquidation_pool_init_param(
		mut self,
		pool_id: CurrencyId,
		deviation_threshold: Rate,
		balance_ratio: Rate,
		max_ideal_balance: Option<Balance>,
	) -> Self {
		self.liquidation_pools.push((
			pool_id,
			LiquidationPoolData {
				deviation_threshold,
				balance_ratio,
				max_ideal_balance,
			},
		));
		self
	}

	fn liquidation_pool_balance(mut self, currency_id: CurrencyId, balance: Balance) -> Self {
		self.endowed_accounts
			.push((TestLiquidationPools::pools_account_id(), currency_id, balance));
		self
	}
}

impl ControllerTestConfigurator for ExtBuilderNew {
	fn controller_data(
		mut self,
		currency_id: CurrencyId,
		last_interest_accrued_block: BlockNumber,
		protocol_interest_factor: Rate,
		max_borrow_rate: Rate,
		collateral_factor: Rate,
		borrow_cap: Option<Balance>,
		protocol_interest_threshold: Balance,
	) -> Self {
		self.controller_dates.push((
			currency_id,
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

	fn pause_keeper(mut self, currency_id: CurrencyId, is_paused: bool) -> Self {
		self.pause_keepers.push((
			currency_id,
			if is_paused {
				PauseKeeper::all_paused()
			} else {
				PauseKeeper::all_unpaused()
			},
		));
		self
	}

	fn white_list(mut self, is_activated: bool) -> Self {
		self.whitelist_mode = is_activated;
		self
	}
}
