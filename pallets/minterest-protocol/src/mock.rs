//! Mocks for the minterest-protocol module.

use super::*;
use crate as minterest_protocol;
use controller::{ControllerData, PauseKeeper};
use frame_support::pallet_prelude::GenesisBuild;
use frame_support::{ord_parameter_types, parameter_types};
use frame_system as system;
use frame_system::EnsureSignedBy;
use liquidity_pools::{Pool, PoolUserData};
use minterest_primitives::{Balance, CurrencyId, CurrencyPair, Price, Rate};
use orml_currencies::Currency;
use orml_traits::parameter_type_with_key;
use pallet_traits::PriceProvider;
use sp_core::H256;
use sp_runtime::{
	testing::Header,
	traits::{AccountIdConversion, BlakeTwo256, IdentityLookup, Zero},
	FixedPointNumber, ModuleId,
};
use sp_std::cell::RefCell;

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;

// Configure a mock runtime to test the pallet.
frame_support::construct_runtime!(
	pub enum Test where
		Block = Block,
		NodeBlock = Block,
		UncheckedExtrinsic = UncheckedExtrinsic,
	{
		System: frame_system::{Module, Call, Config, Storage, Event<T>},
		Tokens: orml_tokens::{Module, Storage, Call, Event<T>, Config<T>},
		Currencies: orml_currencies::{Module, Call, Event<T>},
		Controller: controller::{Module, Storage, Call, Event, Config<T>},
		MinterestModel: minterest_model::{Module, Storage, Call, Event, Config},
		TestProtocol: minterest_protocol::{Module, Storage, Call, Event<T>},
		TestPools: liquidity_pools::{Module, Storage, Call, Config<T>},
	}
);

parameter_types! {
	pub const BlockHashCount: u64 = 250;
	pub const SS58Prefix: u8 = 42;
}

pub type AccountId = u64;

impl system::Config for Test {
	type BaseCallFilter = ();
	type BlockWeights = ();
	type BlockLength = ();
	type DbWeight = ();
	type Origin = Origin;
	type Call = Call;
	type Index = u64;
	type BlockNumber = u64;
	type Hash = H256;
	type Hashing = BlakeTwo256;
	type AccountId = u64;
	type Lookup = IdentityLookup<Self::AccountId>;
	type Header = Header;
	type Event = Event;
	type BlockHashCount = BlockHashCount;
	type Version = ();
	type PalletInfo = PalletInfo;
	type AccountData = ();
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type SystemWeightInfo = ();
	type SS58Prefix = SS58Prefix;
}

type Amount = i128;

parameter_type_with_key! {
	pub ExistentialDeposits: |currency_id: CurrencyId| -> Balance {
		Default::default()
	};
}

impl orml_tokens::Config for Test {
	type Event = Event;
	type Balance = Balance;
	type Amount = Amount;
	type CurrencyId = CurrencyId;
	type WeightInfo = ();
	type ExistentialDeposits = ExistentialDeposits;
	type OnDust = ();
}

parameter_types! {
	pub const GetNativeCurrencyId: CurrencyId = CurrencyId::MNT;
}

type NativeCurrency = Currency<Test, GetNativeCurrencyId>;

impl orml_currencies::Config for Test {
	type Event = Event;
	type MultiCurrency = orml_tokens::Module<Test>;
	type NativeCurrency = NativeCurrency;
	type GetNativeCurrencyId = GetNativeCurrencyId;
	type WeightInfo = ();
}

parameter_types! {
	pub const LiquidityPoolsModuleId: ModuleId = ModuleId(*b"min/lqdy");
	pub LiquidityPoolAccountId: AccountId = LiquidityPoolsModuleId::get().into_account();
	pub InitialExchangeRate: Rate = Rate::one();
	pub EnabledCurrencyPair: Vec<CurrencyPair> = vec![
		CurrencyPair::new(CurrencyId::DOT, CurrencyId::MDOT),
		CurrencyPair::new(CurrencyId::KSM, CurrencyId::MKSM),
		CurrencyPair::new(CurrencyId::BTC, CurrencyId::MBTC),
		CurrencyPair::new(CurrencyId::ETH, CurrencyId::METH),
	];
	pub EnabledUnderlyingAssetId: Vec<CurrencyId> = EnabledCurrencyPair::get().iter()
			.map(|currency_pair| currency_pair.underlying_id)
			.collect();
	pub EnabledWrappedTokensId: Vec<CurrencyId> = EnabledCurrencyPair::get().iter()
			.map(|currency_pair| currency_pair.wrapped_id)
			.collect();
}

pub struct MockPriceSource;

impl PriceProvider<CurrencyId> for MockPriceSource {
	fn get_underlying_price(_currency_id: CurrencyId) -> Option<Price> {
		Some(Price::one())
	}

	fn lock_price(_currency_id: CurrencyId) {}

	fn unlock_price(_currency_id: CurrencyId) {}
}

impl liquidity_pools::Config for Test {
	type MultiCurrency = orml_tokens::Module<Test>;
	type PriceSource = MockPriceSource;
	type ModuleId = LiquidityPoolsModuleId;
	type LiquidityPoolAccountId = LiquidityPoolAccountId;
	type InitialExchangeRate = InitialExchangeRate;
	type EnabledCurrencyPair = EnabledCurrencyPair;
	type EnabledUnderlyingAssetId = EnabledUnderlyingAssetId;
	type EnabledWrappedTokensId = EnabledWrappedTokensId;
}

parameter_types! {
	pub const MaxBorrowCap: Balance = MAX_BORROW_CAP;
}

ord_parameter_types! {
	pub const OneAlice: AccountId = 1;
}

impl controller::Config for Test {
	type Event = Event;
	type LiquidityPoolsManager = liquidity_pools::Module<Test>;
	type MaxBorrowCap = MaxBorrowCap;
	type UpdateOrigin = EnsureSignedBy<OneAlice, AccountId>;
	type WeightInfo = ();
}

parameter_types! {
	pub const BlocksPerYear: u128 = 5256000;
}

impl minterest_model::Config for Test {
	type Event = Event;
	type BlocksPerYear = BlocksPerYear;
	type ModelUpdateOrigin = EnsureSignedBy<OneAlice, AccountId>;
}

thread_local! {
	static TWO: RefCell<Vec<u64>> = RefCell::new(vec![2]);
}

pub struct Two;
impl Contains<u64> for Two {
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

impl minterest_protocol::Config for Test {
	type Event = Event;
	type Borrowing = liquidity_pools::Module<Test>;
	type ManagerLiquidityPools = liquidity_pools::Module<Test>;
	type WhitelistMembers = Two;
}

pub const ALICE: AccountId = 1;
pub fn alice() -> Origin {
	Origin::signed(ALICE)
}
pub const BOB: AccountId = 2;
pub fn bob() -> Origin {
	Origin::signed(BOB)
}
pub const DOLLARS: Balance = 1_000_000_000_000_000_000;
pub const ONE_MILL_DOLLARS: Balance = 1_000_000 * DOLLARS;
pub const ONE_HUNDRED_DOLLARS: Balance = 100 * DOLLARS;
pub const TEN_THOUSAND_DOLLARS: Balance = 10_000 * DOLLARS;
pub const MAX_BORROW_CAP: Balance = 1_000_000_000_000_000_000_000_000;

pub struct ExtBuilder {
	endowed_accounts: Vec<(AccountId, CurrencyId, Balance)>,
}

impl Default for ExtBuilder {
	fn default() -> Self {
		Self {
			endowed_accounts: vec![
				// seed: initial DOTs. Initial MINT to pay for gas.
				(ALICE, CurrencyId::MNT, ONE_MILL_DOLLARS),
				(ALICE, CurrencyId::DOT, ONE_HUNDRED_DOLLARS),
				(ALICE, CurrencyId::ETH, ONE_HUNDRED_DOLLARS),
				(ALICE, CurrencyId::KSM, ONE_HUNDRED_DOLLARS),
				(BOB, CurrencyId::MNT, ONE_MILL_DOLLARS),
				(BOB, CurrencyId::DOT, ONE_HUNDRED_DOLLARS),
				// seed: initial insurance, equal 10_000$
				(TestPools::pools_account_id(), CurrencyId::ETH, TEN_THOUSAND_DOLLARS),
				(TestPools::pools_account_id(), CurrencyId::DOT, TEN_THOUSAND_DOLLARS),
				// seed: initial insurance = 10_000$, initial pool balance = 1_000_000$
				(TestPools::pools_account_id(), CurrencyId::KSM, ONE_MILL_DOLLARS),
			],
		}
	}
}
impl ExtBuilder {
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
			pools: vec![
				(
					CurrencyId::ETH,
					Pool {
						total_borrowed: Balance::zero(),
						borrow_index: Rate::saturating_from_rational(1, 1),
						total_insurance: TEN_THOUSAND_DOLLARS,
					},
				),
				(
					CurrencyId::DOT,
					Pool {
						total_borrowed: Balance::zero(),
						borrow_index: Rate::saturating_from_rational(1, 1),
						total_insurance: TEN_THOUSAND_DOLLARS,
					},
				),
				(
					CurrencyId::KSM,
					Pool {
						total_borrowed: Balance::zero(),
						borrow_index: Rate::saturating_from_rational(1, 1),
						total_insurance: TEN_THOUSAND_DOLLARS,
					},
				),
			],
			pool_user_data: vec![
				(
					CurrencyId::DOT,
					ALICE,
					PoolUserData {
						total_borrowed: 0,
						interest_index: Rate::from_inner(0),
						collateral: true,
						liquidation_attempts: 0,
					},
				),
				(
					CurrencyId::ETH,
					ALICE,
					PoolUserData {
						total_borrowed: 0,
						interest_index: Rate::from_inner(0),
						collateral: false,
						liquidation_attempts: 0,
					},
				),
				(
					CurrencyId::KSM,
					ALICE,
					PoolUserData {
						total_borrowed: 0,
						interest_index: Rate::from_inner(0),
						collateral: true,
						liquidation_attempts: 0,
					},
				),
				(
					CurrencyId::BTC,
					ALICE,
					PoolUserData {
						total_borrowed: 0,
						interest_index: Rate::from_inner(0),
						collateral: true,
						liquidation_attempts: 0,
					},
				),
				(
					CurrencyId::DOT,
					BOB,
					PoolUserData {
						total_borrowed: 0,
						interest_index: Rate::from_inner(0),
						collateral: true,
						liquidation_attempts: 0,
					},
				),
				(
					CurrencyId::BTC,
					BOB,
					PoolUserData {
						total_borrowed: 0,
						interest_index: Rate::from_inner(0),
						collateral: true,
						liquidation_attempts: 0,
					},
				),
			],
		}
		.assimilate_storage(&mut t)
		.unwrap();

		controller::GenesisConfig::<Test> {
			controller_dates: vec![
				(
					CurrencyId::ETH,
					ControllerData {
						timestamp: 0,
						insurance_factor: Rate::saturating_from_rational(1, 10),  // 10%
						max_borrow_rate: Rate::saturating_from_rational(5, 1000), // 0.5%
						collateral_factor: Rate::saturating_from_rational(9, 10), // 90%
						borrow_cap: None,
					},
				),
				(
					CurrencyId::DOT,
					ControllerData {
						timestamp: 0,
						insurance_factor: Rate::saturating_from_rational(1, 10),  // 10%
						max_borrow_rate: Rate::saturating_from_rational(5, 1000), // 0.5%
						collateral_factor: Rate::saturating_from_rational(9, 10), // 90%
						borrow_cap: None,
					},
				),
				(
					CurrencyId::KSM,
					ControllerData {
						timestamp: 0,
						insurance_factor: Rate::saturating_from_rational(1, 10),  // 10%
						max_borrow_rate: Rate::saturating_from_rational(5, 1000), // 0.5%
						collateral_factor: Rate::saturating_from_rational(9, 10), // 90%
						borrow_cap: None,
					},
				),
				(
					CurrencyId::BTC,
					ControllerData {
						timestamp: 0,
						insurance_factor: Rate::saturating_from_rational(1, 10),  // 10%
						max_borrow_rate: Rate::saturating_from_rational(5, 1000), // 0.5%
						collateral_factor: Rate::saturating_from_rational(9, 10), // 90%
						borrow_cap: None,
					},
				),
			],
			pause_keepers: vec![
				(
					CurrencyId::ETH,
					PauseKeeper {
						deposit_paused: false,
						redeem_paused: false,
						borrow_paused: false,
						repay_paused: false,
						transfer_paused: false,
					},
				),
				(
					CurrencyId::DOT,
					PauseKeeper {
						deposit_paused: false,
						redeem_paused: false,
						borrow_paused: false,
						repay_paused: false,
						transfer_paused: false,
					},
				),
				(
					CurrencyId::KSM,
					PauseKeeper {
						deposit_paused: true,
						redeem_paused: true,
						borrow_paused: true,
						repay_paused: true,
						transfer_paused: true,
					},
				),
				(
					CurrencyId::BTC,
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

		let mut ext: sp_io::TestExternalities = t.into();
		ext.execute_with(|| System::set_block_number(1));
		ext
	}
}
