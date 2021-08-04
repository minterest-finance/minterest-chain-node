//! # Test Helper Pallet
//!
//! ## Overview
//!
//! Contains constants, functions and macros with mocked implementations
//! of several modules config traits

pub mod offchain_ext;
pub use currency_mock::*;
pub use users_mock::*;

pub mod currency_mock {
	use frame_support::sp_runtime::FixedPointNumber;
	pub use minterest_primitives::{Balance, CurrencyId, OriginalAsset, Price, WrapToken};
	pub use OriginalAsset::{BTC, DOT, ETH, KSM};

	pub const MNT: CurrencyId = CurrencyId::Original(OriginalAsset::MNT);
	// pub const DOT: CurrencyId = CurrencyId::Original(OriginalAsset::DOT);
	// pub const KSM: CurrencyId = CurrencyId::Original(OriginalAsset::KSM);
	// pub const BTC: CurrencyId = CurrencyId::Original(OriginalAsset::BTC);
	// pub const ETH: CurrencyId = CurrencyId::Original(OriginalAsset::ETH);
	pub const MDOT: CurrencyId = CurrencyId::Wrap(WrapToken::DOT);
	pub const MKSM: CurrencyId = CurrencyId::Wrap(WrapToken::KSM);
	pub const MBTC: CurrencyId = CurrencyId::Wrap(WrapToken::BTC);
	pub const METH: CurrencyId = CurrencyId::Wrap(WrapToken::ETH);

	pub const DOLLARS: Balance = 1_000_000_000_000_000_000;
	pub fn dollars(amount: u128) -> u128 {
		amount.saturating_mul(Price::accuracy())
	}

	pub const ONE_HUNDRED: Balance = 100 * DOLLARS;
	pub const TEN_THOUSAND: Balance = 10_000 * DOLLARS;
	pub const ONE_HUNDRED_THOUSAND: Balance = 100_000 * DOLLARS;
	pub const ONE_MILL: Balance = 1_000_000 * DOLLARS;

	pub const PROTOCOL_INTEREST_TRANSFER_THRESHOLD: Balance = 1_000 * DOLLARS;
}

pub mod users_mock {
	use frame_support::traits::OriginTrait;

	pub type AccountId = u64;

	pub const ADMIN: AccountId = 0;
	pub const ALICE: AccountId = 1;
	pub const BOB: AccountId = 2;
	pub const CHARLIE: AccountId = 3;

	pub fn admin_origin<Origin: OriginTrait<AccountId = AccountId>>() -> Origin {
		Origin::signed(ADMIN)
	}
	pub fn alice_origin<Origin: OriginTrait<AccountId = AccountId>>() -> Origin {
		Origin::signed(ALICE)
	}
	pub fn bob_origin<Origin: OriginTrait<AccountId = AccountId>>() -> Origin {
		Origin::signed(BOB)
	}
	pub fn charlie_origin<Origin: OriginTrait<AccountId = AccountId>>() -> Origin {
		Origin::signed(CHARLIE)
	}
}

#[macro_export]
macro_rules! mock_impl_system_config {
	($target:ty, $account_id:ty) => {
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
			type AccountId = $account_id;
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
			type OnSetCode = ();
		}
	};

	($target:ty) => {
		mock_impl_system_config!($target, u64);
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
			type MaxLocks = MaxLocks;
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
			type MultiCurrency = orml_tokens::Pallet<$target>;
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
			type MultiCurrency = orml_currencies::Pallet<$target>;
			type PriceSource = MockPriceSource;
			type PalletId = LiquidityPoolsPalletId;
			type LiquidityPoolAccountId = LiquidityPoolAccountId;
			type InitialExchangeRate = InitialExchangeRate;
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
			type MultiCurrency = orml_currencies::Pallet<$target>;
			type UnsignedPriority = MockLiquidityPoolsPriority;
			type PriceSource = MockPriceSource;
			type LiquidationPoolsPalletId = LiquidationPoolsPalletId;
			type UpdateOrigin = EnsureSignedBy<ZeroAdmin, AccountId>;
			type LiquidityPoolsManager = liquidity_pools::Pallet<$target>;
			type LiquidationPoolAccountId = LiquidationPoolAccountId;
			type Dex = dex::Pallet<$target>;
			type LiquidationPoolsWeightInfo = ();
			type ControllerManager = controller::Pallet<$target>;
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
			type MultiCurrency = orml_currencies::Pallet<$target>;
			type PriceSource = MockPriceSource;
			type LiquidityPoolsManager = liquidity_pools::Pallet<$target>;
			type MinterestModelManager = minterest_model::Pallet<$target>;
			type MaxBorrowCap = MaxBorrowCap;
			type UpdateOrigin = EnsureSignedBy<$acc, AccountId>;
			type ControllerWeightInfo = ();
			type MntManager = mnt_token::Pallet<$target>;
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
			pub const DexPalletId: PalletId = PalletId(*b"min/dexs");
			pub DexAccountId: AccountId = DexPalletId::get().into_account();
		}

		impl dex::Config for $target {
			type Event = Event;
			type MultiCurrency = orml_currencies::Pallet<$target>;
			type DexPalletId = DexPalletId;
			type DexAccountId = DexAccountId;
		}
	};
}

#[macro_export]
macro_rules! mock_impl_minterest_protocol_config {
	($target:ty, $acc:ident) => {
		impl minterest_protocol::Config for $target {
			type Event = Event;
			type MultiCurrency = orml_currencies::Pallet<$target>;
			type ManagerLiquidationPools = liquidation_pools::Pallet<$target>;
			type ManagerLiquidityPools = liquidity_pools::Pallet<$target>;
			type MntManager = mnt_token::Pallet<$target>;
			type ProtocolWeightInfo = ();
			type ControllerManager = controller::Pallet<$target>;
			type MinterestModelManager = TestMinterestModel;
			type CreatePoolOrigin = EnsureSignedBy<$acc, AccountId>;
			type UserLiquidationAttempts = risk_manager::Pallet<$target>;
			type RiskManager = risk_manager::Pallet<$target>;
			type WhitelistManager = whitelist_module::Pallet<$target>;
		}
	};
}

#[macro_export]
macro_rules! mock_impl_risk_manager_config {
	($target:ty, $acc:ident, $min_sum_mock:ty) => {
		parameter_types! {
			pub const RiskManagerPriority: TransactionPriority = TransactionPriority::max_value();
			pub const PartialLiquidationMaxAttempts: u8 = 3_u8;
			pub const MaxLiquidationFee: Rate = Rate::from_inner(500_000_000_000_000_000);
			pub const RiskManagerWorkerMaxDurationMs: u64 = 2000_u64;
		}

		impl risk_manager::Config for $target {
			type Event = Event;
			type UnsignedPriority = RiskManagerPriority;
			type PriceSource = MockPriceSource;
			type UserCollateral = liquidity_pools::Pallet<$target>;
			type PartialLiquidationMinSum = $min_sum_mock;
			type PartialLiquidationMaxAttempts = PartialLiquidationMaxAttempts;
			type MaxLiquidationFee = MaxLiquidationFee;
			type RiskManagerUpdateOrigin = EnsureSignedBy<$acc, AccountId>;
			type ControllerManager = controller::Pallet<$target>;
			type LiquidityPoolsManager = liquidity_pools::Pallet<$target>;
			type LiquidationPoolsManager = liquidation_pools::Pallet<$target>;
			type MinterestProtocolManager = minterest_protocol::Pallet<$target>;
			type OffchainWorkerMaxDurationMs = RiskManagerWorkerMaxDurationMs;
			type MultiCurrency = orml_currencies::Pallet<$target>;
		}
	};

	($target:ty, $acc:ident) => {
		parameter_types! {
			pub const RiskManagerPriority: TransactionPriority = TransactionPriority::max_value();
			pub const PartialLiquidationMinSum: Balance = 10_000 * DOLLARS;
			pub const PartialLiquidationMaxAttempts: u8 = 3_u8;
			pub const MaxLiquidationFee: Rate = Rate::from_inner(500_000_000_000_000_000);
			pub const RiskManagerWorkerMaxDurationMs: u64 = 2000_u64;
		}

		impl risk_manager::Config for $target {
			type Event = Event;
			type UnsignedPriority = RiskManagerPriority;
			type PriceSource = MockPriceSource;
			type UserCollateral = liquidity_pools::Pallet<$target>;
			type PartialLiquidationMinSum = PartialLiquidationMinSum;
			type PartialLiquidationMaxAttempts = PartialLiquidationMaxAttempts;
			type MaxLiquidationFee = MaxLiquidationFee;
			type RiskManagerUpdateOrigin = EnsureSignedBy<$acc, AccountId>;
			type ControllerManager = controller::Pallet<$target>;
			type LiquidityPoolsManager = liquidity_pools::Pallet<$target>;
			type LiquidationPoolsManager = liquidation_pools::Pallet<$target>;
			type MinterestProtocolManager = minterest_protocol::Pallet<$target>;
			type OffchainWorkerMaxDurationMs = RiskManagerWorkerMaxDurationMs;
			type MultiCurrency = orml_currencies::Pallet<$target>;
		}
	};
}

#[macro_export]
macro_rules! mock_impl_mnt_token_config {
	($target:ty, $acc:ident) => {
		impl mnt_token::Config for $target {
			type Event = Event;
			type PriceSource = MockPriceSource;
			type UpdateOrigin = EnsureSignedBy<$acc, AccountId>;
			type LiquidityPoolsManager = liquidity_pools::Pallet<$target>;
			type MultiCurrency = orml_currencies::Pallet<$target>;
			type ControllerManager = controller::Pallet<$target>;
			type MntTokenAccountId = MntTokenAccountId;
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
			pub const MaxReserves: u32 = 256;
		}

		impl pallet_balances::Config for $target {
			type MaxLocks = MaxLocks;
			type Balance = Balance;
			type Event = Event;
			type DustRemoval = ();
			type ExistentialDeposit = ExistentialDeposit;
			type AccountStore = frame_system::Pallet<$target>;
			type WeightInfo = pallet_balances::weights::SubstrateWeight<$target>;
			type MaxReserves = MaxReserves;
			type ReserveIdentifier = [u8; 8];
		}
	};
}

#[macro_export]
macro_rules! mock_impl_whitelist_module_config {
	($target:ty, $acc:ident) => {
		parameter_types! {
			pub const MaxMembersWhitelistMode: u8 = 16;
		}

		impl whitelist_module::Config for $target {
			type Event = Event;
			type MaxMembers = MaxMembersWhitelistMode;
			type WhitelistOrigin = EnsureSignedBy<$acc, AccountId>;
			type WhitelistWeightInfo = ();
		}
	};
}

#[macro_export]
macro_rules! mock_impl_prices_module_config {
	($target:ty, $acc:ident) => {
		impl module_prices::Config for $target {
			type Event = Event;
			type Source = MockDataProvider;
			type LockOrigin = EnsureSignedBy<$acc, AccountId>;
			type WeightInfo = ();
		}
	};
}
