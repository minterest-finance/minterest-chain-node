pub use controller::{ControllerData, PauseKeeper};
use frame_support::{
	construct_runtime, ord_parameter_types,
	pallet_prelude::{GenesisBuild, TransactionPriority},
	parameter_types,
	traits::Contains,
	PalletId,
};
pub use frame_system::{offchain::SendTransactionTypes, EnsureSignedBy};
use liquidation_pools::LiquidationPoolData;

pub use liquidity_pools::{Pool, PoolUserData};
use minterest_model::MinterestModelData;
pub use test_helper::*;

pub use minterest_primitives::{
	currency::CurrencyType::{UnderlyingAsset, WrappedToken},
	Balance, CurrencyId, Price, Rate,
};
use orml_traits::{parameter_type_with_key, DataFeeder, DataProvider};
use pallet_traits::{PoolsManager, PricesManager};
use sp_runtime::{
	testing::{Header, TestXt, H256},
	traits::{AccountIdConversion, BlakeTwo256, IdentityLookup, One, Zero},
	FixedPointNumber,
};
use sp_std::{cell::RefCell, marker::PhantomData};

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
		MinterestProtocol: minterest_protocol::{Pallet, Storage, Call, Event<T>},
		TestPools: liquidity_pools::{Pallet, Storage, Call, Config<T>},
		TestLiquidationPools: liquidation_pools::{Pallet, Storage, Call, Event<T>, Config<T>},
		TestController: controller::{Pallet, Storage, Call, Event, Config<T>},
		TestMinterestModel: minterest_model::{Pallet, Storage, Call, Event, Config<T>},
		TestDex: dex::{Pallet, Storage, Call, Event<T>},
		TestMntToken: mnt_token::{Pallet, Storage, Call, Event<T>, Config<T>},
		TestRiskManager: risk_manager::{Pallet, Storage, Call, Event<T>, Config<T>},
		TestWhitelist: whitelist_module::{Pallet, Storage, Call, Event<T>, Config<T>},
		TestPrices: module_prices::{Pallet, Storage, Call, Event<T>},
	}
);

parameter_types! {
		pub const LiquidityPoolsPalletId: PalletId = PalletId(*b"lqdy/min");
		pub const LiquidationPoolsPalletId: PalletId = PalletId(*b"lqdn/min");
		pub const MntTokenPalletId: PalletId = PalletId(*b"min/mntt");
		pub LiquidityPoolAccountId: AccountId = LiquidityPoolsPalletId::get().into_account();
		pub LiquidationPoolAccountId: AccountId = LiquidationPoolsPalletId::get().into_account();
		pub MntTokenAccountId: AccountId = MntTokenPalletId::get().into_account();
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
	pub const OneAlice: AccountId = 1;
}

pub struct WhitelistMembers;

impl Contains<u64> for WhitelistMembers {
	fn contains(who: &AccountId) -> bool {
		TWO.with(|v| v.borrow().contains(who))
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
mock_impl_controller_config!(TestRuntime, OneAlice);
mock_impl_minterest_model_config!(TestRuntime, OneAlice);
mock_impl_dex_config!(TestRuntime);
mock_impl_mnt_token_config!(TestRuntime, OneAlice);
mock_impl_risk_manager_config!(TestRuntime, OneAlice);
mock_impl_whitelist_module_config!(TestRuntime, OneAlice);
mock_impl_minterest_protocol_config!(TestRuntime, OneAlice);
mock_impl_prices_module_config!(TestRuntime, OneAlice);
// -----------------------------------------------------------------------------------------
// 										PRICE SOURCE
// -----------------------------------------------------------------------------------------
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
// 										DATA PROVIDER
// -----------------------------------------------------------------------------------------
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
// -----------------------------------------------------------------------------------------
// 									EXTERNALITY BUILDER
// -----------------------------------------------------------------------------------------
/// ExtBuilder declaration.
/// ExtBuilder is a struct to store configuration of your test runtime.
//TODO: Rename to ExtBuilder after full tests rework
pub struct ExtBuilderNew {
	pub endowed_accounts: Vec<(AccountId, CurrencyId, Balance)>,
	pub pools: Vec<(CurrencyId, Pool)>,
	pub pool_user_data: Vec<(CurrencyId, AccountId, PoolUserData)>,
	pub liquidation_pools: Vec<(CurrencyId, LiquidationPoolData)>,
	pub minted_pools: Vec<(CurrencyId, Balance)>,
	pub mnt_claim_threshold: Balance,
	pub controller_params: Vec<(CurrencyId, ControllerData<BlockNumber>)>,
	pub pause_keepers: Vec<(CurrencyId, PauseKeeper)>,
	pub minterest_model_params: Vec<(CurrencyId, MinterestModelData)>,
	pub locked_price: Vec<(CurrencyId, Price)>,
}

/// Default values for ExtBuilder.
/// By default you runtime will be configured with this values for corresponding fields.
impl Default for ExtBuilderNew {
	fn default() -> Self {
		Self {
			endowed_accounts: vec![],
			pools: vec![],
			pool_user_data: vec![],
			liquidation_pools: vec![],
			minted_pools: vec![],
			mnt_claim_threshold: Balance::zero(),
			controller_params: Vec::new(),
			pause_keepers: vec![],
			minterest_model_params: vec![],
			locked_price: vec![],
		}
	}
}

// -----------------------------------------------------------------------------------------
// 										CONFIGURATION TRAITS
// -----------------------------------------------------------------------------------------
/// Configuration traits.
/// Below, you will find a set of functions for configuration of test runtime.
/// Those functions allow you to set system variables, such as pools, balances, and rates,
/// to implement various test scenarios.

// pool_moc -> init_pool_default
// pool_total_borrowed -> init_pool
// liquidity_pool -> init_pool
impl ExtBuilderNew {
	/// Set balance for the particular user
	/// - 'user': id of users account
	/// - 'currency_id': currency
	/// - 'balance': balance value to set
	pub fn set_user_balance(mut self, user: AccountId, currency_id: CurrencyId, balance: Balance) -> Self {
		self.endowed_accounts.push((user, currency_id, balance));
		self
	}

	/// Set balance for the particular pool
	/// - 'currency_id': pool id
	/// - 'balance': balance value to set
	pub fn set_pool_balance(mut self, currency_id: CurrencyId, balance: Balance) -> Self {
		self.endowed_accounts
			.push((TestPools::pools_account_id(), currency_id, balance));
		self
	}

	/// Set DEX balance
	/// - 'currency_id': currency id
	/// - 'balance': balance value
	pub fn set_dex_balance(mut self, currency_id: CurrencyId, balance: Balance) -> Self {
		self.endowed_accounts
			.push((TestDex::dex_account_id(), currency_id, balance));
		self
	}

	/// Initialize pool with default parameters:
	/// borrowed: 0, borrow_index: 1, protocol_interest: 0
	/// - 'pool_id': pool currency / id
	pub fn init_pool_default(mut self, pool_id: CurrencyId) -> Self {
		self.pools.push((
			pool_id,
			Pool {
				borrowed: Balance::default(),
				borrow_index: Rate::default(),
				protocol_interest: Balance::default(),
			},
		));
		self
	}

	/// Initialize pool
	/// - 'pool_id': pool currency / id
	/// - 'borrowed': value of currency borrowed from the pool_id
	/// - 'borrow_index': index, describing change of borrow interest rate
	/// - 'protocol_interest': interest of the protocol
	pub fn init_pool(
		mut self,
		pool_id: CurrencyId,
		borrowed: Balance,
		borrow_index: Rate,
		protocol_interest: Balance,
	) -> Self {
		self.pools.push((
			pool_id,
			Pool {
				borrowed,
				borrow_index,
				protocol_interest,
			},
		));
		self
	}

	/// Set user data for particular pool
	/// - 'pool_id': pool id
	/// - 'user': user id
	/// - 'borrowed': total balance (with accrued interest), after applying the most recent
	///   balance-changing action.
	/// - 'interest_index': global borrow_index as of the most recent balance-changing action
	/// - 'is_collateral': can pool be used as collateral for the current user
	/// - 'liquidation_attempts': number of partial liquidations for debt
	pub fn set_pool_user_data(
		mut self,
		pool_id: CurrencyId,
		user: AccountId,
		borrowed: Balance,
		interest_index: Rate,
		is_collateral: bool,
		liquidation_attempts: u8,
	) -> Self {
		self.pool_user_data.push((
			pool_id,
			user,
			PoolUserData {
				borrowed,
				interest_index,
				is_collateral,
				liquidation_attempts,
			},
		));
		self
	}

	/// Enable minting for particular pools
	/// - 'pools': list of pools with mnt_speeds
	pub fn mnt_enabled_pools(mut self, pools: Vec<(CurrencyId, Balance)>) -> Self {
		self.minted_pools = pools;
		self
	}

	/// Enable minting for all pools
	/// - 'speed': mnt minting speed
	pub fn enable_minting_for_all_pools(mut self, speed: Balance) -> Self {
		self.minted_pools = vec![(KSM, speed), (DOT, speed), (ETH, speed), (BTC, speed)];
		self
	}

	/// Set mnt threshold
	/// - 'threshold': mnt transfer threshold
	pub fn set_mnt_claim_threshold(mut self, threshold: u128) -> Self {
		self.mnt_claim_threshold = threshold * DOLLARS;
		self
	}

	/// Set mnt balance for mnt pallets account
	/// - 'balance': mnt balance
	pub fn set_mnt_account_balance(mut self, balance: Balance) -> Self {
		self.endowed_accounts
			.push((TestMntToken::get_account_id(), MNT, balance));
		self
	}

	/// Initialize liquidation pool
	/// - 'pool_id': pool id
	/// - 'deviation_threshold': threshold
	/// - 'balance_ratio': represents the percentage of working pool value to be covered by value in
	///   Liquidation Poll.
	/// - 'max_ideal_balance': maximum ideal balance during pool balancing
	pub fn init_liquidation_pool(
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

	/// Set balance of the liquidation pool
	/// - 'currency_id': pool / currency id
	/// - 'balance': balance to set
	pub fn set_liquidation_pool_balance(mut self, currency_id: CurrencyId, balance: Balance) -> Self {
		self.endowed_accounts
			.push((TestLiquidationPools::pools_account_id(), currency_id, balance));
		self
	}

	/// Set controller data for the current pool
	/// - 'currency_id': pool / currency id
	/// - 'last_interest_accrued_block': block number that interest was last accrued at.
	/// - 'protocol_interest_factor': defines the portion of borrower interest that is converted
	/// into protocol interest.
	/// - 'max_borrow_rate': maximum borrow rate.
	/// - 'collateral_factor': this multiplier represents which share of the supplied value can be
	///   used as a collateral for loans. For instance, 0.9 allows 90% of total pool value to be
	///   used as a collateral. Must be between 0 and 1.
	/// - 'borrow_cap': maximum total borrow amount per pool in usd. No value means infinite borrow
	///   cap.
	/// - protocol_interest_threshold': minimum protocol interest needed to transfer it to
	///   liquidation pool
	pub fn set_controller_data(
		mut self,
		currency_id: CurrencyId,
		last_interest_accrued_block: BlockNumber,
		protocol_interest_factor: Rate,
		max_borrow_rate: Rate,
		collateral_factor: Rate,
		borrow_cap: Option<Balance>,
		protocol_interest_threshold: Balance,
	) -> Self {
		self.controller_params.push((
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

	/// Set pausekeeper state.
	/// - 'currency_id': currency identifier
	/// - 'is_paused': pause / unpause all keepers for current currency
	pub fn set_pause_keeper(mut self, currency_id: CurrencyId, is_paused: bool) -> Self {
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

	/// Set minterest model parameters
	/// - 'currency_id': currency identifier
	/// - 'kink': the utilization point at which the jump multiplier is applied
	/// - 'base_rate_per_block': the base interest rate which is the y-intercept when utilization
	///   rate is 0
	/// - 'multiplier_per_block': the multiplier of utilization rate that gives the slope of the
	///   interest rate
	/// - 'jump_multiplier_per_block': the multiplier of utilization rate after hitting a specified
	///   utilization point - kink
	pub fn set_minterest_model_params(
		mut self,
		currency_id: CurrencyId,
		kink: Rate,
		base_rate_per_block: Rate,
		multiplier_per_block: Rate,
		jump_multiplier_per_block: Rate,
	) -> Self {
		self.minterest_model_params.push((
			currency_id,
			MinterestModelData {
				kink,
				base_rate_per_block,
				multiplier_per_block,
				jump_multiplier_per_block,
			},
		));
		self
	}

	/// Set locked price for the currency
	/// - `currency_id` : currency identifier
	/// - `price`: locked price
	pub fn set_locked_price(mut self, currency_id: CurrencyId, price: Price) -> Self {
		self.locked_price.push((currency_id, price));
		self
	}

	/// Builds GenesisConfig for all pallets from ExtBuilder data
	pub fn build(self) -> sp_io::TestExternalities {
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
			controller_params: self.controller_params,
			pause_keepers: self.pause_keepers,
		}
		.assimilate_storage(&mut t)
		.unwrap();

		minterest_model::GenesisConfig::<TestRuntime> {
			minterest_model_params: self.minterest_model_params,
			_phantom: Default::default(),
		}
		.assimilate_storage(&mut t)
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
