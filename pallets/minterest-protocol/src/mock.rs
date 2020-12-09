//! Mocks for the minterest-protocol module.

use frame_support::{impl_outer_origin, impl_outer_event, parameter_types};
use sp_runtime::{traits::IdentityLookup, testing::Header, Perbill, DispatchResult};
use orml_currencies::Currency;
use sp_core::H256;
use minterest_primitives::{Balance, CurrencyId};

use super::*;

impl_outer_origin! {
	pub enum Origin for Runtime {}
}

mod minterest_protocol {
    pub use crate::Event;
}

impl_outer_event! {
	pub enum TestEvent for Runtime {
		frame_system<T>,
		orml_tokens<T>, orml_currencies<T>,
		m_tokens<T>, minterest_protocol<T>,
	}
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Runtime;
parameter_types! {
	pub const BlockHashCount: u64 = 250;
	pub const MaximumBlockWeight: u32 = 1024;
	pub const MaximumBlockLength: u32 = 2 * 1024;
    pub const AvailableBlockRatio: Perbill = Perbill::one();
    pub UnderlyingAssetId: Vec<CurrencyId> = vec![CurrencyId::DOT];
}

pub type AccountId = u32;
impl frame_system::Trait for Runtime {
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

pub type System = frame_system::Module<Runtime>;

type Amount = i128;

impl orml_tokens::Trait for Runtime {
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

type NativeCurrency = Currency<Runtime, GetNativeCurrencyId>;

impl orml_currencies::Trait for Runtime {
    type Event = TestEvent;
    type MultiCurrency = orml_tokens::Module<Runtime>;
    type NativeCurrency = NativeCurrency;
    type GetNativeCurrencyId = GetNativeCurrencyId;
    type WeightInfo = ();
}

pub struct MockLiquidityPools;

impl LiquidityPools for MockLiquidityPools {
    fn add_liquidity(currency_id: &CurrencyId, amount: &Balance) -> DispatchResult {
        Ok(()) // TODO
    }

    fn withdraw_liquidity(currency_id: &CurrencyId, amount: &Balance) -> DispatchResult {
        Ok(()) // TODO
    }
}

impl m_tokens::Trait for Runtime {
    type Event = TestEvent;
    type MultiCurrency = orml_tokens::Module<Runtime>;
}

pub type TestMTokens = m_tokens::Module<Runtime>;

impl Trait for Runtime {
    type Event = TestEvent;
    type UnderlyingAssetId = UnderlyingAssetId;
    type LiqudityPools = MockLiquidityPools;
}
