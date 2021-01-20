//! Mocks for the minterest-protocol module.

use frame_support::{impl_outer_event, impl_outer_origin, parameter_types};
use liquidity_pools::{Pool, PoolUserData};
use minterest_primitives::{Balance, CurrencyId, Rate};
use orml_currencies::Currency;
use sp_core::H256;
use sp_runtime::{
	testing::Header,
	traits::{IdentityLookup, Zero},
	FixedPointNumber, ModuleId, Perbill,
};

use super::*;
use controller::{ControllerData, PauseKeeper};

mod minterest_protocol {
	pub use crate::Event;
}

impl_outer_event! {
	pub enum TestEvent for Test {
		frame_system<T>,
		orml_tokens<T>,
		orml_currencies<T>,
		liquidity_pools,
		minterest_protocol<T>,
		controller,
		oracle,
		accounts<T>,
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
	type Event = TestEvent;
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
	type Event = TestEvent;
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
	type Event = TestEvent;
	type MultiCurrency = orml_tokens::Module<Test>;
	type NativeCurrency = NativeCurrency;
	type GetNativeCurrencyId = GetNativeCurrencyId;
	type WeightInfo = ();
}

parameter_types! {
	pub const LiquidityPoolsModuleId: ModuleId = ModuleId(*b"min/pool");
	pub const InitialExchangeRate: Rate = Rate::from_inner(1_000_000_000_000_000_000);
}

impl liquidity_pools::Trait for Test {
	type Event = TestEvent;
	type MultiCurrency = orml_tokens::Module<Test>;
	type ModuleId = LiquidityPoolsModuleId;
	type InitialExchangeRate = InitialExchangeRate;
}

parameter_types! {
	pub const BlocksPerYear: u128 = 5256000;
	pub MTokensId: Vec<CurrencyId> = vec![
		CurrencyId::MDOT,
		CurrencyId::MKSM,
		CurrencyId::MBTC,
		CurrencyId::METH,
	];
	pub UnderlyingAssetId: Vec<CurrencyId> = vec![
		CurrencyId::DOT,
		CurrencyId::KSM,
		CurrencyId::BTC,
		CurrencyId::ETH,
	];
}

impl controller::Trait for Test {
	type Event = TestEvent;
	type BlocksPerYear = BlocksPerYear;
	type UnderlyingAssetId = UnderlyingAssetId;
	type MTokensId = MTokensId;
}

impl oracle::Trait for Test {
	type Event = TestEvent;
}

parameter_types! {
	pub const MaxMembers: u32 = MAX_MEMBERS;
}

impl accounts::Trait for Test {
	type Event = TestEvent;
	type MaxMembers = MaxMembers;
}

impl Trait for Test {
	type Event = TestEvent;
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
pub const MAX_MEMBERS: u32 = 16;
pub type TestProtocol = Module<Test>;
pub type TestPools = liquidity_pools::Module<Test>;
pub type Currencies = orml_currencies::Module<Test>;
pub type System = frame_system::Module<Test>;

pub struct ExtBuilder {
	endowed_accounts: Vec<(AccountId, CurrencyId, Balance)>,
}

impl Default for ExtBuilder {
	fn default() -> Self {
		Self {
			endowed_accounts: vec![
				// seed: initial DOTs. Initial MINT to pay for gas.
				(ALICE, CurrencyId::MINT, ONE_MILL_DOLLARS),
				(ALICE, CurrencyId::DOT, ONE_HUNDRED_DOLLARS),
				(ALICE, CurrencyId::ETH, ONE_HUNDRED_DOLLARS),
				(BOB, CurrencyId::MINT, ONE_MILL_DOLLARS),
				(BOB, CurrencyId::DOT, ONE_HUNDRED_DOLLARS),
				// seed: initial insurance, equal 10_000$
				(TestPools::pools_account_id(), CurrencyId::ETH, TEN_THOUSAND_DOLLARS),
				(TestPools::pools_account_id(), CurrencyId::DOT, TEN_THOUSAND_DOLLARS),
				(TestPools::pools_account_id(), CurrencyId::KSM, TEN_THOUSAND_DOLLARS),
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
						current_exchange_rate: Rate::from_inner(1),
						total_insurance: TEN_THOUSAND_DOLLARS,
					},
				),
				(
					CurrencyId::DOT,
					Pool {
						total_borrowed: Balance::zero(),
						borrow_index: Rate::saturating_from_rational(1, 1),
						current_exchange_rate: Rate::from_inner(1),
						total_insurance: TEN_THOUSAND_DOLLARS,
					},
				),
				(
					CurrencyId::KSM,
					Pool {
						total_borrowed: Balance::zero(),
						borrow_index: Rate::saturating_from_rational(1, 1),
						current_exchange_rate: Rate::from_inner(1),
						total_insurance: TEN_THOUSAND_DOLLARS,
					},
				),
			],
			pool_user_data: vec![
				(
					ALICE,
					CurrencyId::DOT,
					PoolUserData {
						total_borrowed: 0,
						interest_index: Rate::from_inner(0),
						collateral: true,
					},
				),
				(
					ALICE,
					CurrencyId::ETH,
					PoolUserData {
						total_borrowed: 0,
						interest_index: Rate::from_inner(0),
						collateral: false,
					},
				),
				(
					ALICE,
					CurrencyId::KSM,
					PoolUserData {
						total_borrowed: 0,
						interest_index: Rate::from_inner(0),
						collateral: true,
					},
				),
				(
					ALICE,
					CurrencyId::BTC,
					PoolUserData {
						total_borrowed: 0,
						interest_index: Rate::from_inner(0),
						collateral: true,
					},
				),
				(
					BOB,
					CurrencyId::DOT,
					PoolUserData {
						total_borrowed: 0,
						interest_index: Rate::from_inner(0),
						collateral: true,
					},
				),
				(
					BOB,
					CurrencyId::BTC,
					PoolUserData {
						total_borrowed: 0,
						interest_index: Rate::from_inner(0),
						collateral: true,
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
						supply_rate: Rate::from_inner(0),
						borrow_rate: Rate::from_inner(0),
						insurance_factor: Rate::saturating_from_rational(1, 10),  // 10%
						max_borrow_rate: Rate::saturating_from_rational(5, 1000), // 0.5%
						kink: Rate::saturating_from_rational(8, 10),              // 80%
						base_rate_per_block: Rate::from_inner(0),
						multiplier_per_block: Rate::saturating_from_rational(9, 1_000_000_000), // 0.047304 PerYear
						jump_multiplier_per_block: Rate::saturating_from_rational(207, 1_000_000_000), // 1.09 PerYear
						collateral_factor: Rate::saturating_from_rational(9, 10),               // 90%
					},
				),
				(
					CurrencyId::DOT,
					ControllerData {
						timestamp: 0,
						supply_rate: Rate::from_inner(0),
						borrow_rate: Rate::from_inner(0),
						insurance_factor: Rate::saturating_from_rational(1, 10),  // 10%
						max_borrow_rate: Rate::saturating_from_rational(5, 1000), // 0.5%
						kink: Rate::saturating_from_rational(8, 10),              // 80%
						base_rate_per_block: Rate::from_inner(0),
						multiplier_per_block: Rate::saturating_from_rational(9, 1_000_000_000), // 0.047304 PerYear
						jump_multiplier_per_block: Rate::saturating_from_rational(207, 1_000_000_000), // 1.09 PerYear
						collateral_factor: Rate::saturating_from_rational(9, 10),               // 90%
					},
				),
				(
					CurrencyId::KSM,
					ControllerData {
						timestamp: 0,
						supply_rate: Rate::from_inner(0),
						borrow_rate: Rate::from_inner(0),
						insurance_factor: Rate::saturating_from_rational(1, 10),  // 10%
						max_borrow_rate: Rate::saturating_from_rational(5, 1000), // 0.5%
						kink: Rate::saturating_from_rational(8, 10),              // 80%
						base_rate_per_block: Rate::from_inner(0),
						multiplier_per_block: Rate::saturating_from_rational(9, 1_000_000_000), // 0.047304 PerYear
						jump_multiplier_per_block: Rate::saturating_from_rational(207, 1_000_000_000), // 1.09 PerYear
						collateral_factor: Rate::saturating_from_rational(9, 10),               // 90%
					},
				),
				(
					CurrencyId::BTC,
					ControllerData {
						timestamp: 0,
						supply_rate: Rate::from_inner(0),
						borrow_rate: Rate::from_inner(0),
						insurance_factor: Rate::saturating_from_rational(1, 10),  // 10%
						max_borrow_rate: Rate::saturating_from_rational(5, 1000), // 0.5%
						kink: Rate::saturating_from_rational(8, 10),              // 80%
						base_rate_per_block: Rate::from_inner(0),
						multiplier_per_block: Rate::saturating_from_rational(9, 1_000_000_000), // 0.047304 PerYear
						jump_multiplier_per_block: Rate::saturating_from_rational(207, 1_000_000_000), // 1.09 PerYear
						collateral_factor: Rate::saturating_from_rational(9, 10),               // 90%
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
					},
				),
				(
					CurrencyId::DOT,
					PauseKeeper {
						deposit_paused: false,
						redeem_paused: false,
						borrow_paused: false,
						repay_paused: false,
					},
				),
				(
					CurrencyId::KSM,
					PauseKeeper {
						deposit_paused: false,
						redeem_paused: false,
						borrow_paused: false,
						repay_paused: false,
					},
				),
				(
					CurrencyId::BTC,
					PauseKeeper {
						deposit_paused: false,
						redeem_paused: false,
						borrow_paused: false,
						repay_paused: false,
					},
				),
			],
		}
		.assimilate_storage(&mut t)
		.unwrap();

		accounts::GenesisConfig::<Test> {
			allowed_accounts: vec![(ALICE, ())],
		}
		.assimilate_storage(&mut t)
		.unwrap();

		let mut ext: sp_io::TestExternalities = t.into();
		ext.execute_with(|| System::set_block_number(1));
		ext
	}
}
