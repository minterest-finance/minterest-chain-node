//! Mocks for the minterest-protocol module.

use controller::ControllerData;
use frame_support::{impl_outer_event, impl_outer_origin, parameter_types};
use liquidity_pools::Pool;
use minterest_primitives::{Balance, CurrencyId, Rate};
use orml_currencies::Currency;
use sp_core::H256;
use sp_runtime::{
	testing::Header,
	traits::{IdentityLookup, Zero},
	FixedPointNumber, ModuleId, Perbill,
};

use super::*;

mod minterest_protocol {
	pub use crate::Event;
}

impl_outer_event! {
	pub enum Event for Test {
		frame_system<T>,
		orml_tokens<T>,
		orml_currencies<T>,
		liquidity_pools,
		minterest_protocol<T>,
		controller,
		oracle,
	}
}

impl_outer_origin! {
	pub enum Origin for Test where system = frame_system {}
}

#[derive(Clone, PartialEq, Eq)]
pub struct Test;
parameter_types! {
	pub const BlockHashCount: u64 = 250;
	pub const MaximumBlockWeight: u32 = 1024;
	pub const MaximumBlockLength: u32 = 2 * 1024;
	pub const AvailableBlockRatio: Perbill = Perbill::one();
	pub UnderlyingAssetId: Vec<CurrencyId> = vec![
		CurrencyId::DOT,
		CurrencyId::KSM,
		CurrencyId::BTC,
		CurrencyId::ETH,
	];
}

pub type AccountId = u32;
impl frame_system::Trait for Test {
	type Origin = Origin;
	type Call = ();
	type Index = u64;
	type BlockNumber = u64;
	type Hash = H256;
	type Hashing = ::sp_runtime::traits::BlakeTwo256;
	type AccountId = AccountId;
	type Lookup = IdentityLookup<Self::AccountId>;
	type Header = Header;
	type Event = Event;
	type BlockHashCount = BlockHashCount;
	type MaximumExtrinsicWeight = MaximumBlockWeight;
	type MaximumBlockWeight = MaximumBlockWeight;
	type DbWeight = ();
	type BlockExecutionWeight = ();
	type ExtrinsicBaseWeight = ();
	type MaximumBlockLength = MaximumBlockLength;
	type AvailableBlockRatio = AvailableBlockRatio;
	type Version = ();
	type PalletInfo = ();
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type AccountData = ();
	type BaseCallFilter = ();
	type SystemWeightInfo = ();
}

parameter_types! {
	pub const ExistentialDeposit: u64 = 1;
}

impl orml_tokens::Trait for Test {
	type Event = Event;
	type Balance = Balance;
	type Amount = Amount;
	type CurrencyId = CurrencyId;
	type OnReceived = ();
	type WeightInfo = ();
}

parameter_types! {
	pub const GetNativeCurrencyId: CurrencyId = CurrencyId::MINT;
}

type NativeCurrency = Currency<Test, GetNativeCurrencyId>;

impl orml_currencies::Trait for Test {
	type Event = Event;
	type MultiCurrency = orml_tokens::Module<Test>;
	type NativeCurrency = NativeCurrency;
	type GetNativeCurrencyId = GetNativeCurrencyId;
	type WeightInfo = ();
}

parameter_types! {
	pub const LiquidityPoolsModuleId: ModuleId = ModuleId(*b"min/pool");
}

impl liquidity_pools::Trait for Test {
	type Event = Event;
	type MultiCurrency = orml_tokens::Module<Test>;
	type ModuleId = LiquidityPoolsModuleId;
}

parameter_types! {
	pub const InitialExchangeRate: Rate = Rate::from_inner(1_000_000_000_000_000_000);
	pub MTokensId: Vec<CurrencyId> = vec![
		CurrencyId::MDOT,
		CurrencyId::MKSM,
		CurrencyId::MBTC,
		CurrencyId::METH,
	];
}

impl controller::Trait for Test {
	type Event = Event;
	type InitialExchangeRate = InitialExchangeRate;
	type MTokensId = MTokensId;
}

impl oracle::Trait for Test {
	type Event = Event;
}

impl Trait for Test {
	type Event = Event;
	type UnderlyingAssetId = UnderlyingAssetId;
	type Borrowing = MockBorrowing;
}

pub struct MockBorrowing;
impl Borrowing<AccountId> for MockBorrowing {
	fn update_state_on_borrow(
		_who: &AccountId,
		_underlying_asset_id: CurrencyId,
		_amount_borrowed: Balance,
		_account_borrows: Balance,
	) -> DispatchResult {
		Ok(())
	}

	fn update_state_on_repay(
		_who: &AccountId,
		_underlying_asset_id: CurrencyId,
		_amount_borrowed: Balance,
		_account_borrows: Balance,
	) -> DispatchResult {
		Ok(())
	}
}

type Amount = i128;

pub const ADMIN: AccountId = 0;
pub const ALICE: AccountId = 1;
pub const BOB: AccountId = 2;
pub const ONE_MILL: Balance = 1_000_000;
pub const ONE_HUNDRED: Balance = 100;
pub type MinterestProtocol = Module<Test>;
pub type TestPools = liquidity_pools::Module<Test>;
pub type Currencies = orml_currencies::Module<Test>;
pub type System = frame_system::Module<Test>;

pub(crate) fn new_test_ext() -> sp_io::TestExternalities {
	let mut t = frame_system::GenesisConfig::default().build_storage::<Test>().unwrap();

	orml_tokens::GenesisConfig::<Test> {
		endowed_accounts: vec![
			(ALICE, CurrencyId::MINT, ONE_MILL),
			(ALICE, CurrencyId::DOT, ONE_HUNDRED),
			(BOB, CurrencyId::MINT, ONE_MILL),
			(BOB, CurrencyId::DOT, ONE_HUNDRED),
			(ADMIN, CurrencyId::MINT, ONE_MILL),
			(ADMIN, CurrencyId::DOT, ONE_HUNDRED),
		],
	}
	.assimilate_storage(&mut t)
	.unwrap();

	liquidity_pools::GenesisConfig::<Test> {
		pools: vec![
			(
				CurrencyId::ETH,
				Pool {
					current_interest_rate: Rate::from_inner(0),
					total_borrowed: Balance::zero(),
					borrow_index: Rate::saturating_from_rational(1, 1),
					current_exchange_rate: Rate::from_inner(1),
					is_lock: true,
					total_insurance: Balance::zero(),
				},
			),
			(
				CurrencyId::DOT,
				Pool {
					current_interest_rate: Rate::from_inner(0),
					total_borrowed: Balance::zero(),
					borrow_index: Rate::saturating_from_rational(1, 1),
					current_exchange_rate: Rate::from_inner(1),
					is_lock: true,
					total_insurance: Balance::zero(),
				},
			),
			(
				CurrencyId::KSM,
				Pool {
					current_interest_rate: Rate::from_inner(0),
					total_borrowed: Balance::zero(),
					borrow_index: Rate::saturating_from_rational(1, 1),
					current_exchange_rate: Rate::from_inner(1),
					is_lock: true,
					total_insurance: Balance::zero(),
				},
			),
			(
				CurrencyId::BTC,
				Pool {
					current_interest_rate: Rate::from_inner(0),
					total_borrowed: Balance::zero(),
					borrow_index: Rate::saturating_from_rational(1, 1),
					current_exchange_rate: Rate::from_inner(1),
					is_lock: true,
					total_insurance: Balance::zero(),
				},
			),
		],
		pool_user_data: vec![],
	}
	.assimilate_storage(&mut t)
	.unwrap();

	controller::GenesisConfig::<Test> {
		controller_dates: vec![
			(
				CurrencyId::ETH,
				ControllerData {
					timestamp: 0,
					borrow_rate: Rate::from_inner(0),
					insurance_factor: Rate::saturating_from_rational(1, 10),
					max_borrow_rate: Rate::saturating_from_rational(1, 1),
					collateral_factor: Rate::saturating_from_rational(9, 10), // 90%
				},
			),
			(
				CurrencyId::DOT,
				ControllerData {
					timestamp: 0,
					borrow_rate: Rate::from_inner(0),
					insurance_factor: Rate::saturating_from_rational(1, 10),
					max_borrow_rate: Rate::saturating_from_rational(1, 1),
					collateral_factor: Rate::saturating_from_rational(9, 10), // 90%
				},
			),
			(
				CurrencyId::KSM,
				ControllerData {
					timestamp: 0,
					borrow_rate: Rate::from_inner(0),
					insurance_factor: Rate::saturating_from_rational(1, 10),
					max_borrow_rate: Rate::saturating_from_rational(1, 1),
					collateral_factor: Rate::saturating_from_rational(9, 10), // 90%
				},
			),
			(
				CurrencyId::BTC,
				ControllerData {
					timestamp: 0,
					borrow_rate: Rate::from_inner(0),
					insurance_factor: Rate::saturating_from_rational(1, 10),
					max_borrow_rate: Rate::saturating_from_rational(1, 1),
					collateral_factor: Rate::saturating_from_rational(9, 10), // 90%
				},
			),
		],
	}
	.assimilate_storage(&mut t)
	.unwrap();
	let mut ext: sp_io::TestExternalities = t.into();
	ext.execute_with(|| System::set_block_number(1));
	ext
}
