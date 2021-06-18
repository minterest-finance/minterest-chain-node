/// Mocks for the RiskManager pallet.
use super::*;
use crate as risk_manager;
use controller::{ControllerData, PauseKeeper};
use frame_support::{ord_parameter_types, pallet_prelude::GenesisBuild, parameter_types, traits::Contains};
use frame_system::EnsureSignedBy;
use liquidation_pools::LiquidationPoolData;
use liquidity_pools::{Pool, PoolUserData};
pub use minterest_primitives::currency::CurrencyType::WrappedToken;
use minterest_primitives::{Balance, CurrencyId, Price, Rate};
use orml_traits::{parameter_type_with_key, DataProvider};
use sp_core::H256;
use sp_runtime::{
	testing::{Header, TestXt},
	traits::{AccountIdConversion, BlakeTwo256, IdentityLookup},
	FixedPointNumber, ModuleId,
};
use sp_std::cell::RefCell;
use std::collections::HashMap;
use std::thread;
pub use test_helper::*;

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;

pub type Extrinsic = TestXt<Call, ()>;
// Configure a mock runtime to test the pallet.
frame_support::construct_runtime!(
	pub enum Test where
		Block = Block,
		NodeBlock = Block,
		UncheckedExtrinsic = UncheckedExtrinsic,
	{
		System: frame_system::{Module, Call, Config, Storage, Event<T>},
		Balances: pallet_balances::{Module, Call, Storage, Config<T>, Event<T>},
		Currencies: orml_currencies::{Module, Call, Event<T>},
		Tokens: orml_tokens::{Module, Storage, Call, Event<T>, Config<T>},
		Controller: controller::{Module, Storage, Call, Event, Config<T>},
		TestMinterestModel: minterest_model::{Module, Storage, Call, Event, Config},
		TestMinterestProtocol: minterest_protocol::{Module, Storage, Call, Event<T>},
		TestPools: liquidity_pools::{Module, Storage, Call, Config<T>},
		TestRiskManager: risk_manager::{Module, Storage, Call, Event<T>, Config, ValidateUnsigned},
		LiquidationPools: liquidation_pools::{Module, Storage, Call, Event<T>, Config<T>, ValidateUnsigned},
		TestDex: dex::{Module, Storage, Call, Event<T>},
		TestMntToken: mnt_token::{Module, Storage, Call, Event<T>, Config<T>},
	}
);

ord_parameter_types! {
	pub const ZeroAdmin: AccountId = 0;
}

parameter_types! {
	pub const LiquidityPoolsModuleId: ModuleId = ModuleId(*b"lqdi/min");
	pub const LiquidationPoolsModuleId: ModuleId = ModuleId(*b"lqdn/min");
	pub const MntTokenModuleId: ModuleId = ModuleId(*b"min/mntt");
	pub LiquidityPoolAccountId: AccountId = LiquidityPoolsModuleId::get().into_account();
	pub LiquidationPoolAccountId: AccountId = LiquidationPoolsModuleId::get().into_account();
	pub MntTokenAccountId: AccountId = MntTokenModuleId::get().into_account();
	pub InitialExchangeRate: Rate = Rate::one();
	pub EnabledUnderlyingAssetsIds: Vec<CurrencyId> = CurrencyId::get_enabled_tokens_in_protocol(UnderlyingAsset);
	pub EnabledWrappedTokensId: Vec<CurrencyId> = CurrencyId::get_enabled_tokens_in_protocol(WrappedToken);
}

pub struct MockDataProvider;
impl DataProvider<CurrencyId, Price> for MockDataProvider {
	// This function is only called after the token price is unlocked
	fn get(currency_id: &CurrencyId) -> Option<Price> {
		match currency_id {
			&DOT => Some(Price::saturating_from_integer(5)),
			&BTC => {
				// This sleep is need to emulate hard computation in offchain worker.
				let one_sec = std::time::Duration::from_millis(1000);
				thread::sleep(one_sec);
				Some(Price::saturating_from_integer(2))
			}
			&KSM => Some(Price::saturating_from_integer(2)),
			&ETH => Some(Price::saturating_from_integer(2)),
			_ => panic!("Price for this currency wasn't set"),
		}
	}
}

pub struct WhitelistMembers;
mock_impl_system_config!(Test);
mock_impl_orml_tokens_config!(Test);
mock_impl_orml_currencies_config!(Test);
mock_impl_liquidity_pools_config!(Test);
mock_impl_liquidation_pools_config!(Test);
mock_impl_controller_config!(Test, ZeroAdmin);
mock_impl_minterest_model_config!(Test, ZeroAdmin);
mock_impl_dex_config!(Test);
mock_impl_minterest_protocol_config!(Test, ZeroAdmin);
mock_impl_risk_manager_config!(Test, ZeroAdmin);
mock_impl_mnt_token_config!(Test, ZeroAdmin);
mock_impl_balances_config!(Test);

thread_local! {
	static UNDERLYING_PRICE: RefCell<HashMap<CurrencyId, Price>> = RefCell::new(
		[
			(DOT, Price::one()),
			(ETH, Price::one()),
			(BTC, Price::one()),
			(KSM, Price::one()),
		]
		.iter()
		.cloned()
		.collect());
}

pub struct MockPriceSource;
impl MockPriceSource {
	pub fn set_underlying_price(currency_id: CurrencyId, price: Price) {
		UNDERLYING_PRICE.with(|v| v.borrow_mut().insert(currency_id, price));
	}
}

impl PricesManager<CurrencyId> for MockPriceSource {
	fn get_underlying_price(currency_id: CurrencyId) -> Option<Price> {
		if currency_id == BTC {
			// This sleep is need to emulate hard computation in offchain worker.
			let one_sec = std::time::Duration::from_millis(1000);
			thread::sleep(one_sec);
		}
		UNDERLYING_PRICE.with(|v| v.borrow().get(&currency_id).copied())
	}

	fn lock_price(_currency_id: CurrencyId) {}

	fn unlock_price(_currency_id: CurrencyId) {}
}

ord_parameter_types! {
		pub const Four: AccountId = 4;
}

thread_local! {
	static TWO: RefCell<Vec<u64>> = RefCell::new(vec![2]);
}

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

pub struct ExtBuilder {
	endowed_accounts: Vec<(AccountId, CurrencyId, Balance)>,
	pools: Vec<(CurrencyId, Pool)>,
	pool_user_data: Vec<(CurrencyId, AccountId, PoolUserData)>,
}

impl Default for ExtBuilder {
	fn default() -> Self {
		Self {
			endowed_accounts: vec![],
			pools: vec![
				(
					DOT,
					Pool {
						total_borrowed: Balance::zero(),
						borrow_index: Rate::one(),
						total_protocol_interest: Balance::zero(),
					},
				),
				(
					ETH,
					Pool {
						total_borrowed: Balance::zero(),
						borrow_index: Rate::one(),
						total_protocol_interest: Balance::zero(),
					},
				),
				(
					BTC,
					Pool {
						total_borrowed: Balance::zero(),
						borrow_index: Rate::one(),
						total_protocol_interest: Balance::zero(),
					},
				),
				(
					KSM,
					Pool {
						total_borrowed: Balance::zero(),
						borrow_index: Rate::one(),
						total_protocol_interest: Balance::zero(),
					},
				),
			],
			pool_user_data: vec![],
		}
	}
}

impl ExtBuilder {
	pub fn pool_init(mut self, pool_id: CurrencyId) -> Self {
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

	pub fn pool_user_data(
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

	pub fn pool_total_borrowed(mut self, pool_id: CurrencyId, total_borrowed: Balance) -> Self {
		self.pools.push((
			pool_id,
			Pool {
				total_borrowed,
				borrow_index: Rate::one(),
				total_protocol_interest: Balance::zero(),
			},
		));
		self
	}

	pub fn liquidity_pool_balance(mut self, currency_id: CurrencyId, balance: Balance) -> Self {
		self.endowed_accounts
			.push((TestPools::pools_account_id(), currency_id, balance));
		self
	}

	pub fn liquidation_pool_balance(mut self, currency_id: CurrencyId, balance: Balance) -> Self {
		self.endowed_accounts
			.push((LiquidationPools::pools_account_id(), currency_id, balance));
		self
	}

	pub fn user_balance(mut self, user: AccountId, currency_id: CurrencyId, balance: Balance) -> Self {
		self.endowed_accounts.push((user, currency_id, balance));
		self
	}

	pub fn build(self) -> sp_io::TestExternalities {
		let mut t = frame_system::GenesisConfig::default().build_storage::<Test>().unwrap();

		orml_tokens::GenesisConfig::<Test> {
			endowed_accounts: self.endowed_accounts,
		}
		.assimilate_storage(&mut t)
		.unwrap();

		liquidity_pools::GenesisConfig::<Test> {
			pools: self.pools,
			pool_user_data: self.pool_user_data,
		}
		.assimilate_storage(&mut t)
		.unwrap();

		risk_manager::GenesisConfig {
			risk_manager_params: vec![
				(
					DOT,
					RiskManagerData {
						max_attempts: 3,
						min_partial_liquidation_sum: ONE_HUNDRED * DOLLARS,
						threshold: Rate::saturating_from_rational(103, 100),
						liquidation_fee: Rate::saturating_from_rational(105, 100),
					},
				),
				(
					BTC,
					RiskManagerData {
						max_attempts: 3,
						min_partial_liquidation_sum: ONE_HUNDRED * DOLLARS,
						threshold: Rate::saturating_from_rational(103, 100),
						liquidation_fee: Rate::saturating_from_rational(105, 100),
					},
				),
				(
					ETH,
					RiskManagerData {
						max_attempts: 3,
						min_partial_liquidation_sum: ONE_HUNDRED * DOLLARS,
						threshold: Rate::saturating_from_rational(103, 100),
						liquidation_fee: Rate::saturating_from_rational(105, 100),
					},
				),
			],
		}
		.assimilate_storage::<Test>(&mut t)
		.unwrap();

		liquidation_pools::GenesisConfig::<Test> {
			phantom: PhantomData,
			liquidation_pools: vec![
				(
					DOT,
					LiquidationPoolData {
						deviation_threshold: Rate::saturating_from_rational(1, 10),
						balance_ratio: Rate::saturating_from_rational(2, 10),
						max_ideal_balance: None,
					},
				),
				(
					ETH,
					LiquidationPoolData {
						deviation_threshold: Rate::saturating_from_rational(1, 10),
						balance_ratio: Rate::saturating_from_rational(2, 10),
						max_ideal_balance: None,
					},
				),
				(
					BTC,
					LiquidationPoolData {
						deviation_threshold: Rate::saturating_from_rational(1, 10),
						balance_ratio: Rate::saturating_from_rational(8, 10),
						max_ideal_balance: None,
					},
				),
				(
					KSM,
					LiquidationPoolData {
						deviation_threshold: Rate::saturating_from_rational(1, 10),
						balance_ratio: Rate::saturating_from_rational(2, 10),
						max_ideal_balance: None,
					},
				),
			],
		}
		.assimilate_storage(&mut t)
		.unwrap();
		controller::GenesisConfig::<Test> {
			controller_dates: vec![
				(
					DOT,
					ControllerData {
						last_interest_accrued_block: 1,
						protocol_interest_factor: Rate::saturating_from_rational(1, 10),
						max_borrow_rate: Rate::saturating_from_rational(5, 1000),
						collateral_factor: Rate::saturating_from_rational(9, 10), // 90%
						borrow_cap: None,
						protocol_interest_threshold: PROTOCOL_INTEREST_TRANSFER_THRESHOLD,
					},
				),
				(
					ETH,
					ControllerData {
						last_interest_accrued_block: 1,
						protocol_interest_factor: Rate::saturating_from_rational(1, 10),
						max_borrow_rate: Rate::saturating_from_rational(5, 1000),
						collateral_factor: Rate::saturating_from_rational(9, 10), // 90
						borrow_cap: None,
						protocol_interest_threshold: PROTOCOL_INTEREST_TRANSFER_THRESHOLD,
					},
				),
				(
					BTC,
					ControllerData {
						last_interest_accrued_block: 1,
						protocol_interest_factor: Rate::saturating_from_rational(1, 10),
						max_borrow_rate: Rate::saturating_from_rational(5, 1000),
						collateral_factor: Rate::saturating_from_rational(9, 10), // 90%
						borrow_cap: None,
						protocol_interest_threshold: PROTOCOL_INTEREST_TRANSFER_THRESHOLD,
					},
				),
			],
			pause_keepers: vec![
				(
					ETH,
					PauseKeeper {
						deposit_paused: false,
						redeem_paused: false,
						borrow_paused: false,
						repay_paused: false,
						transfer_paused: false,
					},
				),
				(
					DOT,
					PauseKeeper {
						deposit_paused: false,
						redeem_paused: false,
						borrow_paused: false,
						repay_paused: false,
						transfer_paused: false,
					},
				),
				(
					KSM,
					PauseKeeper {
						deposit_paused: false,
						redeem_paused: false,
						borrow_paused: false,
						repay_paused: false,
						transfer_paused: false,
					},
				),
				(
					BTC,
					PauseKeeper {
						deposit_paused: false,
						redeem_paused: false,
						borrow_paused: false,
						repay_paused: false,
						transfer_paused: false,
					},
				),
			],
			whitelist_mode: false,
		}
		.assimilate_storage(&mut t)
		.unwrap();

		let mut ext = sp_io::TestExternalities::new(t);
		ext.execute_with(|| System::set_block_number(1));
		ext
	}
}

pub(crate) fn set_price_for_all_assets(price: Price) {
	CurrencyId::get_enabled_tokens_in_protocol(UnderlyingAsset)
		.iter()
		.for_each(|&currency_id| {
			MockPriceSource::set_underlying_price(currency_id, price);
		})
}

pub(crate) fn set_prices_for_assets(prices: Vec<(CurrencyId, Price)>) {
	prices.into_iter().for_each(|(currency_id, price)| {
		MockPriceSource::set_underlying_price(currency_id, price);
	});
}
