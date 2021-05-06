//! # Test Helper Module
//!
//! ## Overview
//!
//! Contains macros with mocked implementations of several modules config traits
use minterest_primitives::{currency::TokenSymbol, CurrencyId};

pub const MNT: CurrencyId = CurrencyId::Native(TokenSymbol::MNT);
pub const DOT: CurrencyId = CurrencyId::UnderlyingAsset(TokenSymbol::DOT);
pub const MDOT: CurrencyId = CurrencyId::WrappedToken(TokenSymbol::MDOT);
pub const KSM: CurrencyId = CurrencyId::UnderlyingAsset(TokenSymbol::KSM);
pub const MKSM: CurrencyId = CurrencyId::WrappedToken(TokenSymbol::MKSM);
pub const BTC: CurrencyId = CurrencyId::UnderlyingAsset(TokenSymbol::BTC);
pub const MBTC: CurrencyId = CurrencyId::WrappedToken(TokenSymbol::MBTC);
pub const ETH: CurrencyId = CurrencyId::UnderlyingAsset(TokenSymbol::ETH);
pub const METH: CurrencyId = CurrencyId::WrappedToken(TokenSymbol::METH);

#[macro_export]
macro_rules! mock_impl_system_config {
	($target:ty) => {
		parameter_types! {
			pub const MockBlockHashCount: u64 = 250;
			pub const MockSS58Prefix: u8 = 42;
		}

		impl frame_system::Config for $target {
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
			type BlockHashCount = MockBlockHashCount;
			type Version = ();
			type PalletInfo = PalletInfo;
			type AccountData = pallet_balances::AccountData<Balance>;
			type OnNewAccount = ();
			type OnKilledAccount = ();
			type SystemWeightInfo = ();
			type SS58Prefix = MockSS58Prefix;
		}
	};
}

#[macro_export]
macro_rules! mock_impl_orml_tokens_config {
	($target:ty) => {
		parameter_type_with_key! {
			pub MockExistentialDeposits: |_currency_id: CurrencyId| -> Balance {
				Default::default()
			};
		}

		impl orml_tokens::Config for $target {
			type Event = Event;
			type Balance = Balance;
			type Amount = i128;
			type CurrencyId = CurrencyId;
			type WeightInfo = ();
			type ExistentialDeposits = MockExistentialDeposits;
			type OnDust = ();
		}
	};
}

#[macro_export]
macro_rules! mock_impl_orml_currencies_config {
	($target:ty) => {
		parameter_types! {
			pub const MockGetNativeCurrencyId: CurrencyId = MNT;
		}

		pub type Amount = i128;
		pub type BlockNumber = u64;
		type AdaptedBasicCurrency = orml_currencies::BasicCurrencyAdapter<$target, Balances, Amount, BlockNumber>;

		impl orml_currencies::Config for $target {
			type Event = Event;
			type MultiCurrency = orml_tokens::Module<$target>;
			type NativeCurrency = AdaptedBasicCurrency;
			type GetNativeCurrencyId = MockGetNativeCurrencyId;
			type WeightInfo = ();
		}
	};
}

#[macro_export]
macro_rules! mock_impl_liquidity_pools_config {
	($target:ty) => {
		impl liquidity_pools::Config for $target {
			type MultiCurrency = orml_currencies::Module<$target>;
			type PriceSource = MockPriceSource;
			type ModuleId = LiquidityPoolsModuleId;
			type LiquidityPoolAccountId = LiquidityPoolAccountId;
			type InitialExchangeRate = InitialExchangeRate;
			type EnabledUnderlyingAssetsIds = EnabledUnderlyingAssetsIds;
			type EnabledWrappedTokensId = EnabledWrappedTokensId;
		}
	};
}

#[macro_export]
macro_rules! mock_impl_liquidation_pools_config {
	($target:ty) => {
		parameter_types! {
			pub const MockLiquidityPoolsPriority: TransactionPriority = TransactionPriority::max_value() - 1;
		}

		impl liquidation_pools::Config for $target {
			type Event = Event;
			type MultiCurrency = orml_currencies::Module<$target>;
			type UnsignedPriority = MockLiquidityPoolsPriority;
			type PriceSource = MockPriceSource;
			type LiquidationPoolsModuleId = LiquidationPoolsModuleId;
			type LiquidationPoolAccountId = LiquidationPoolAccountId;
			type UpdateOrigin = EnsureSignedBy<ZeroAdmin, AccountId>;
			type LiquidityPoolsManager = liquidity_pools::Module<$target>;
			type Dex = dex::Module<$target>;
			type LiquidationPoolsWeightInfo = ();
		}

		/// An extrinsic type used for tests.
		pub type MockExtrinsic = TestXt<Call, ()>;

		impl<LocalCall> SendTransactionTypes<LocalCall> for $target
		where
			Call: From<LocalCall>,
		{
			type OverarchingCall = Call;
			type Extrinsic = MockExtrinsic;
		}
	};
}

#[macro_export]
macro_rules! mock_impl_controller_config {
	($target:ty, $acc:ident) => {
		parameter_types! {
			pub const MaxBorrowCap: Balance = 1_000_000_000_000_000_000_000_000;
		}

		impl controller::Config for $target {
			type Event = Event;
			type LiquidityPoolsManager = liquidity_pools::Module<$target>;
			type MaxBorrowCap = MaxBorrowCap;
			type UpdateOrigin = EnsureSignedBy<$acc, AccountId>;
			type ControllerWeightInfo = ();
		}
	};
}

#[macro_export]
macro_rules! mock_impl_minterest_model_config {
	($target:ty, $acc:ident) => {
		parameter_types! {
			pub const BlocksPerYear: u128 = 5_256_000;
		}

		impl minterest_model::Config for $target {
			type Event = Event;
			type BlocksPerYear = BlocksPerYear;
			type ModelUpdateOrigin = EnsureSignedBy<$acc, AccountId>;
			type WeightInfo = ();
		}
	};
}

#[macro_export]
macro_rules! mock_impl_dex_config {
	($target:ty) => {
		parameter_types! {
			pub const DexModuleId: ModuleId = ModuleId(*b"min/dexs");
			pub DexAccountId: AccountId = DexModuleId::get().into_account();
		}

		impl dex::Config for $target {
			type Event = Event;
			type MultiCurrency = orml_currencies::Module<$target>;
			type DexModuleId = DexModuleId;
			type DexAccountId = DexAccountId;
		}
	};
}

#[macro_export]
macro_rules! mock_impl_minterest_protocol_config {
	($target:ty) => {
		impl minterest_protocol::Config for $target {
			type Event = Event;
			type Borrowing = liquidity_pools::Module<$target>;
			type ManagerLiquidationPools = liquidation_pools::Module<$target>;
			type ManagerLiquidityPools = liquidity_pools::Module<$target>;
			type MntManager = mnt_token::Module<$target>;
			type WhitelistMembers = WhitelistMembers;
			type ProtocolWeightInfo = ();
			type ControllerAPI = controller::Module<$target>;
		}
	};
}

#[macro_export]
macro_rules! mock_impl_risk_manager_config {
	($target:ty, $acc:ident) => {
		parameter_types! {
			pub const RiskManagerPriority: TransactionPriority = TransactionPriority::max_value();
		}

		impl risk_manager::Config for $target {
			type Event = Event;
			type UnsignedPriority = RiskManagerPriority;
			type LiquidationPoolsManager = liquidation_pools::Module<$target>;
			type LiquidityPoolsManager = liquidity_pools::Module<$target>;
			type MntManager = mnt_token::Module<$target>;
			type RiskManagerUpdateOrigin = EnsureSignedBy<$acc, AccountId>;
			type RiskManagerWeightInfo = ();
			type ControllerAPI = controller::Module<$target>;
		}
	};
}

#[macro_export]
macro_rules! mock_impl_mnt_token_config {
	($target:ty, $acc:ident) => {
		parameter_types! {
			pub const SpeedRefreshPeriod: BlockNumber = 5;
		}

		impl mnt_token::Config for $target {
			type Event = Event;
			type PriceSource = MockPriceSource;
			type UpdateOrigin = EnsureSignedBy<$acc, AccountId>;
			type LiquidityPoolsManager = liquidity_pools::Module<$target>;
			type MultiCurrency = orml_currencies::Module<$target>;
			type ControllerAPI = controller::Module<$target>;
			type MntTokenAccountId = MntTokenAccountId;
			type SpeedRefreshPeriod = SpeedRefreshPeriod;
			type MntTokenWeightInfo = ();
		}
	};
}

#[macro_export]
macro_rules! mock_impl_balances_config {
	($target:ty) => {
		parameter_types! {
			pub const ExistentialDeposit: u128 = 500;
			pub const MaxLocks: u32 = 50;
		}

		impl pallet_balances::Config for $target {
			type MaxLocks = MaxLocks;
			type Balance = Balance;
			type Event = Event;
			type DustRemoval = ();
			type ExistentialDeposit = ExistentialDeposit;
			type AccountStore = frame_system::Pallet<$target>;
			type WeightInfo = pallet_balances::weights::SubstrateWeight<$target>;
		}
	};
}
