//! Mocks for the minterest-protocol module.

use frame_support::{impl_outer_origin, impl_outer_event, parameter_types};
use sp_runtime::{
    traits::{IdentityLookup, Zero},
    testing::Header, Perbill, Permill,
};
use orml_currencies::Currency;
use sp_core::H256;
use minterest_primitives::{Balance, CurrencyId};
use liquidity_pools::Reserve;

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
		m_tokens<T>, liquidity_pools,
		minterest_protocol<T>,
	}
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Runtime;

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

impl m_tokens::Trait for Runtime {
    type Event = TestEvent;
    type MultiCurrency = orml_tokens::Module<Runtime>;
}

impl liquidity_pools::Trait for Runtime {
    type Event = TestEvent;
}

impl Trait for Runtime {
    type Event = TestEvent;
    type UnderlyingAssetId = UnderlyingAssetId;
}

pub const ALICE: AccountId = 1;
pub const BOB: AccountId = 2;
pub const ONE_MILL: Balance = 1_000_000;
pub const ONE_HUNDRED: Balance = 100;
pub type MinterestProtocol = Module<Runtime>;
pub type TestMTokens = m_tokens::Module<Runtime>;
pub type TestPools = liquidity_pools::Module<Runtime>;

pub struct ExtBuilder{
    endowed_accounts: Vec<(AccountId, CurrencyId, Balance)>,
    reserves: Vec<(CurrencyId, Reserve)>,
}

impl Default for ExtBuilder {
    fn default() -> Self {
        Self {
            endowed_accounts: vec![],
            reserves: vec![]
        }
    }
}

impl ExtBuilder {
    pub fn balances(mut self, endowed_accounts: Vec<(AccountId, CurrencyId, Balance)>) -> Self {
        self.endowed_accounts = endowed_accounts;
        self
    }

    pub fn create_reserves(mut self) -> Self {
        self.reserves = vec![
            (
                CurrencyId::ETH,
                Reserve{
                    total_balance: Balance::zero(),
                    current_liquidity_rate: Permill::one()
                },
            ),
            (
                CurrencyId::DOT,
                Reserve{
                    total_balance: Balance::zero(),
                    current_liquidity_rate: Permill::one()
                },
            ),
            (
                CurrencyId::KSM,
                Reserve{
                    total_balance: Balance::zero(),
                    current_liquidity_rate: Permill::one()
                },
            ),
            (
                CurrencyId::BTC,
                Reserve{
                    total_balance: Balance::zero(),
                    current_liquidity_rate: Permill::one()
                },
            ),
        ];
        self
    }

    pub fn one_million_mint_and_one_hundred_dots_for_alice_and_bob(self) -> Self {
        self.balances(vec![
            (ALICE, CurrencyId::MINT, ONE_MILL),
            (ALICE, CurrencyId::DOT, ONE_HUNDRED),
            (BOB, CurrencyId::MINT, ONE_MILL),
            (BOB, CurrencyId::DOT, ONE_HUNDRED),
        ])
    }

    pub fn build(self) -> sp_io::TestExternalities {
        let mut storage = frame_system::GenesisConfig::default()
            .build_storage::<Runtime>()
            .unwrap();

        orml_tokens::GenesisConfig::<Runtime> {
            endowed_accounts: self.endowed_accounts,
        }
            .assimilate_storage(&mut storage)
            .unwrap();

        let mut ext = sp_io::TestExternalities::new(storage);
        ext.execute_with(|| System::set_block_number(1));
        ext
    }
}
