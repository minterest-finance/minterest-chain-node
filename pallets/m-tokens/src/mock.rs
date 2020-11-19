/// Mocks for the m-tokens module.

use frame_support::{impl_outer_event, impl_outer_origin, parameter_types};
use sp_core::H256;
use sp_runtime::{testing::Header, traits::IdentityLookup, Perbill};
use orml_currencies::Currency;
pub use minterest_primitives::{Balance, CurrencyId};

use super::*;

impl_outer_origin! {
	pub enum Origin for Runtime {}
}

mod m_tokens {
    pub use crate::Event;
}

impl_outer_event! {
	pub enum TestEvent for Runtime {
		frame_system<T>,
		orml_tokens<T>, orml_currencies<T>,
		m_tokens<T>,
	}
}

// Workaround for https://github.com/rust-lang/rust/issues/26925 . Remove when sorted.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Runtime;
parameter_types! {
	pub const BlockHashCount: u64 = 250;
	pub const MaximumBlockWeight: u32 = 1024;
	pub const MaximumBlockLength: u32 = 2 * 1024;
    pub const AvailableBlockRatio: Perbill = Perbill::one();
}

type AccountId = u32;
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


impl Trait for Runtime {
    type Event = TestEvent;
    type MultiCurrency = orml_currencies::Module<Runtime>;
}

pub const ALICE: AccountId = 1;
pub const BOB: AccountId = 2;
pub type MTokens = Module<Runtime>;

pub struct ExtBuilder{
    endowed_accounts: Vec<(AccountId, CurrencyId, Balance)>,
}

impl Default for ExtBuilder {
    fn default() -> Self {
        Self {
            endowed_accounts: vec![],
        }
    }
}

pub const ONE_MILL: Balance = 1_000_000;
impl ExtBuilder {
    pub fn balances(mut self, endowed_accounts: Vec<(AccountId, CurrencyId, Balance)>) -> Self {
        self.endowed_accounts = endowed_accounts;
        self
    }

    pub fn build(self) -> sp_io::TestExternalities {
        let mut t = frame_system::GenesisConfig::default()
            .build_storage::<Runtime>()
            .unwrap();

        //FIXME
        // orml_tokens::GenesisConfig::<Runtime> {
        //     endowed_accounts: self.endowed_accounts,
        // }
        // .assimilate_storage(&mut t)
        // .unwrap();

        let mut ext = sp_io::TestExternalities::new(t);
        ext.execute_with(|| System::set_block_number(1));
        ext
    }
}
