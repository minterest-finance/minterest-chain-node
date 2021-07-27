/// Mocks for the liquidation-pools pallet.
use super::*;
use crate as liquidation_pools;
use frame_support::{ord_parameter_types, parameter_types, PalletId};
use frame_system::EnsureSignedBy;
use minterest_primitives::Price;
pub use minterest_primitives::{currency::CurrencyType::WrappedToken, Balance, CurrencyId, Rate};
use orml_traits::parameter_type_with_key;
use pallet_traits::PricesManager;
use sp_core::H256;
use sp_io::TestExternalities;
use sp_runtime::testing::TestXt;
use sp_runtime::{
	testing::Header,
	traits::{BlakeTwo256, IdentityLookup},
	FixedPointNumber,
};
use sp_std::cell::RefCell;
use std::collections::HashMap;
pub use test_helper::*;

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;

// Configure a mock runtime to test the pallet.
frame_support::construct_runtime!(
	pub enum Test where
		Block = Block,
		NodeBlock = Block,
		UncheckedExtrinsic = UncheckedExtrinsic,
	{
		System: frame_system::{Pallet, Call, Config, Storage, Event<T>},
		Balances: pallet_balances::{Pallet, Call, Storage, Config<T>, Event<T>},
		//ORML palletts
		Tokens: orml_tokens::{Pallet, Storage, Call, Event<T>, Config<T>},
		Currencies: orml_currencies::{Pallet, Call, Event<T>},
		// Minterest pallets
		TestLiquidationPools: liquidation_pools::{Pallet, Storage, Call, Event<T>, ValidateUnsigned},
		TestLiquidityPools: liquidity_pools::{Pallet, Storage, Call, Config<T>},
		TestDex: dex::{Pallet, Storage, Call, Event<T>}
	}
);

mock_impl_system_config!(Test);
mock_impl_liquidity_pools_config!(Test);
mock_impl_orml_tokens_config!(Test);
mock_impl_orml_currencies_config!(Test);
mock_impl_dex_config!(Test);
mock_impl_balances_config!(Test);

parameter_types! {
	pub const LiquidityPoolsPalletId: PalletId = PalletId(*b"lqdy/min");
	pub LiquidityPoolAccountId: AccountId = LiquidityPoolsPalletId::get().into_account();
	pub InitialExchangeRate: Rate = Rate::one();
	pub EnabledUnderlyingAssetsIds: Vec<CurrencyId> = CurrencyId::get_enabled_tokens_in_protocol(UnderlyingAsset);
	pub EnabledWrappedTokensId: Vec<CurrencyId> = CurrencyId::get_enabled_tokens_in_protocol(WrappedToken);
}

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
		UNDERLYING_PRICE.with(|v| v.borrow().get(&currency_id).copied())
	}

	fn lock_price(_currency_id: CurrencyId) {}

	fn unlock_price(_currency_id: CurrencyId) {}
}

parameter_types! {
	pub const LiquidationPoolsPalletId: PalletId = PalletId(*b"lqdn/min");
	pub LiquidationPoolAccountId: AccountId = LiquidationPoolsPalletId::get().into_account();
	pub const LiquidityPoolsPriority: TransactionPriority = TransactionPriority::max_value();
}

ord_parameter_types! {
	pub const ZeroAdmin: AccountId = 0;
}

impl Config for Test {
	type Event = Event;
	type MultiCurrency = orml_tokens::Pallet<Test>;
	type UnsignedPriority = LiquidityPoolsPriority;
	type PriceSource = MockPriceSource;
	type LiquidationPoolsPalletId = LiquidationPoolsPalletId;
	type LiquidationPoolAccountId = LiquidationPoolAccountId;
	type UpdateOrigin = EnsureSignedBy<ZeroAdmin, AccountId>;
	type LiquidityPoolsManager = liquidity_pools::Pallet<Test>;
	type Dex = dex::Pallet<Test>;
	type LiquidationPoolsWeightInfo = ();
}

/// An extrinsic type used for tests.
pub type Extrinsic = TestXt<Call, ()>;

impl<LocalCall> SendTransactionTypes<LocalCall> for Test
where
	Call: From<LocalCall>,
{
	type OverarchingCall = Call;
	type Extrinsic = Extrinsic;
}

pub fn admin() -> Origin {
	Origin::signed(ADMIN)
}

pub struct ExternalityBuilder {
	endowed_accounts: Vec<(AccountId, CurrencyId, Balance)>,
	liquidity_pools: Vec<(CurrencyId, PoolData)>,
	liquidation_pools: Vec<(CurrencyId, LiquidationPoolData)>,
}

impl Default for ExternalityBuilder {
	fn default() -> Self {
		Self {
			endowed_accounts: vec![],
			liquidity_pools: vec![
				(
					DOT,
					PoolData {
						borrowed: Balance::zero(),
						borrow_index: Rate::one(),
						protocol_interest: Balance::zero(),
					},
				),
				(
					ETH,
					PoolData {
						borrowed: Balance::zero(),
						borrow_index: Rate::one(),
						protocol_interest: Balance::zero(),
					},
				),
				(
					BTC,
					PoolData {
						borrowed: Balance::zero(),
						borrow_index: Rate::one(),
						protocol_interest: Balance::zero(),
					},
				),
				/*(
					KSM,
					PoolData {
						borrowed: Balance::zero(),
						borrow_index: Rate::one(),
						protocol_interest: Balance::zero(),
					},
				),*/
			],
			liquidation_pools: vec![
				(
					DOT,
					LiquidationPoolData {
						deviation_threshold: Rate::saturating_from_rational(1, 10),
						balance_ratio: Rate::saturating_from_rational(2, 10),
						max_ideal_balance_usd: None,
					},
				),
				(
					ETH,
					LiquidationPoolData {
						deviation_threshold: Rate::saturating_from_rational(1, 10),
						balance_ratio: Rate::saturating_from_rational(2, 10),
						max_ideal_balance_usd: None,
					},
				),
				(
					BTC,
					LiquidationPoolData {
						deviation_threshold: Rate::saturating_from_rational(1, 10),
						balance_ratio: Rate::saturating_from_rational(2, 10),
						max_ideal_balance_usd: None,
					},
				),
				(
					KSM,
					LiquidationPoolData {
						deviation_threshold: Rate::saturating_from_rational(1, 10),
						balance_ratio: Rate::saturating_from_rational(2, 10),
						max_ideal_balance_usd: None,
					},
				),
			],
		}
	}
}

impl ExternalityBuilder {
	pub fn user_balance(mut self, user: AccountId, currency_id: CurrencyId, balance: Balance) -> Self {
		self.endowed_accounts.push((user, currency_id, balance));
		self
	}

	/*pub fn pool_remove(mut self, pool_id: CurrencyId) -> Self {
		self.liquidity_pools.retain(|&(currency_id, _)| currency_id != pool_id);
		self
	}*/
	pub fn set_pool_borrow_underlying(mut self, currency_id: CurrencyId, balance: Balance) -> Self {
		self.liquidity_pools.push((
			currency_id,
			PoolData {
				borrowed: balance,
				borrow_index: Rate::one(),
				protocol_interest: Balance::zero(),
			},
		));
		self
	}
	/*pub fn liquidity_total_borrow_pool(mut self, currency_id: CurrencyId, balance: Balance) -> Self {
		for &mut (currency, ref mut pool) in self.liquidity_pools.iter_mut() {
			if currency == currency_id {
				*pool = PoolData {
					borrowed: balance,
					borrow_index: Rate::one(),
					protocol_interest: Balance::zero(),
				};
				return self;
			}
		}
		self
	}*/

	pub fn liquidation_pool_balance(mut self, currency_id: CurrencyId, balance: Balance) -> Self {
		self.endowed_accounts
			.push((TestLiquidationPools::pools_account_id(), currency_id, balance));
		self
	}

	pub fn dex_balance(mut self, currency_id: CurrencyId, balance: Balance) -> Self {
		self.endowed_accounts
			.push((TestDex::dex_account_id(), currency_id, balance));
		self
	}

	pub fn build(self) -> TestExternalities {
		let mut t = frame_system::GenesisConfig::default().build_storage::<Test>().unwrap();

		orml_tokens::GenesisConfig::<Test> {
			balances: self.endowed_accounts,
		}
		.assimilate_storage(&mut t)
		.unwrap();

		liquidity_pools::GenesisConfig::<Test> {
			pools: self.liquidity_pools,
			pool_user_data: vec![],
		}
		.assimilate_storage(&mut t)
		.unwrap();

		liquidation_pools::GenesisConfig::<Test> {
			liquidation_pools: self.liquidation_pools,
			phantom: PhantomData,
		}
		.assimilate_storage(&mut t)
		.unwrap();

		let mut ext = TestExternalities::new(t);
		ext.execute_with(|| System::set_block_number(1));
		ext
	}
}

pub(crate) fn set_prices_for_assets(prices: Vec<(CurrencyId, Price)>) {
	prices.into_iter().for_each(|(currency_id, price)| {
		MockPriceSource::set_underlying_price(currency_id, price);
	});
}

pub(crate) fn liquidation_pool_balance(pool_id: CurrencyId) -> Balance {
	Currencies::free_balance(pool_id, &TestLiquidationPools::pools_account_id())
}
