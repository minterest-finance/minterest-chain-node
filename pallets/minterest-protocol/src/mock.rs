//! Mocks for the minterest-protocol module.

use frame_support::{impl_outer_event, impl_outer_origin, parameter_types};
use liquidity_pools::Reserve;
use minterest_primitives::{Balance, CurrencyId};
use orml_currencies::Currency;
use sp_core::H256;
use sp_runtime::{
	testing::Header,
	traits::{IdentityLookup, Zero},
	Perbill, Permill,
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
		m_tokens<T>,
		liquidity_pools,
		minterest_protocol<T>,
		controller,
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

impl m_tokens::Trait for Test {
	type Event = Event;
	type MultiCurrency = orml_tokens::Module<Test>;
}

impl liquidity_pools::Trait for Test {
	type Event = Event;
}

impl controller::Trait for Test {
	type Event = Event;
	type MultiCurrency = orml_currencies::Module<Test>;
}

impl Trait for Test {
	type Event = Event;
	type UnderlyingAssetId = UnderlyingAssetId;
	type Borrowing = MockBorrowing;
}

pub struct MockBorrowing;
impl Borrowing<AccountId> for MockBorrowing {
	fn update_state_on_borrow(
		_underlying_asset_id: CurrencyId,
		_amount_borrowed: Balance,
		_who: &AccountId,
	) -> DispatchResult {
		Ok(())
	}

	fn update_state_on_repay(
		_underlying_asset_id: CurrencyId,
		_amount_borrowed: Balance,
		_who: &AccountId,
	) -> DispatchResult {
		Ok(())
	}
}

type Amount = i128;

pub const ALICE: AccountId = 1;
pub const BOB: AccountId = 2;
pub const ONE_MILL: Balance = 1_000_000;
pub const ONE_HUNDRED: Balance = 100;
pub type MinterestProtocol = Module<Test>;
pub type TestMTokens = m_tokens::Module<Test>;
pub type TestPools = liquidity_pools::Module<Test>;

pub(crate) fn new_test_ext() -> sp_io::TestExternalities {
	let mut t = frame_system::GenesisConfig::default().build_storage::<Test>().unwrap();

	orml_tokens::GenesisConfig::<Test> {
		endowed_accounts: vec![
			(ALICE, CurrencyId::MINT, ONE_MILL),
			(ALICE, CurrencyId::DOT, ONE_HUNDRED),
			(BOB, CurrencyId::MINT, ONE_MILL),
			(BOB, CurrencyId::DOT, ONE_HUNDRED),
		],
	}
	.assimilate_storage(&mut t)
	.unwrap();

	liquidity_pools::GenesisConfig {
		reserves: vec![
			(
				CurrencyId::ETH,
				Reserve {
					total_balance: Balance::zero(),
					current_liquidity_rate: Permill::one(),
				},
			),
			(
				CurrencyId::DOT,
				Reserve {
					total_balance: Balance::zero(),
					current_liquidity_rate: Permill::one(),
				},
			),
			(
				CurrencyId::KSM,
				Reserve {
					total_balance: Balance::zero(),
					current_liquidity_rate: Permill::one(),
				},
			),
			(
				CurrencyId::BTC,
				Reserve {
					total_balance: Balance::zero(),
					current_liquidity_rate: Permill::one(),
				},
			),
		],
	}
	.assimilate_storage(&mut t)
	.unwrap();

	t.into()
}
