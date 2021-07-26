#![cfg_attr(not(feature = "std"), no_std)]
// `construct_runtime!` does a lot of recursion and requires us to increase the limit to 256.
#![recursion_limit = "256"]
// The `large_enum_variant` warning originates from `construct_runtime` macro.
#![allow(clippy::large_enum_variant)]
#![allow(clippy::from_over_into)]

// Make the WASM binary available.
#[cfg(feature = "std")]
include!(concat!(env!("OUT_DIR"), "/wasm_binary.rs"));

mod weights;

pub use controller_rpc_runtime_api::{
	BalanceInfo, HypotheticalLiquidityData, PoolState, ProtocolTotalValue, UserData, UserPoolBalanceData,
};
use frame_system::{EnsureOneOf, EnsureRoot};
use minterest_primitives::constants::fee::WeightToFee;
pub use minterest_primitives::{
	constants::{
		currency::DOLLARS,
		liquidation::{MAX_LIQUIDATION_FEE, PARTIAL_LIQUIDATION_MAX_ATTEMPTS, PARTIAL_LIQUIDATION_MIN_SUM},
		time::{BLOCKS_PER_YEAR, DAYS, SLOT_DURATION},
		INITIAL_EXCHANGE_RATE, MAX_BORROW_CAP, PROTOCOL_INTEREST_TRANSFER_THRESHOLD, TOTAL_ALLOCATION,
	},
	currency::{
		CurrencyType::{UnderlyingAsset, WrappedToken},
		BTC, DOT, ETH, KSM, MBTC, MDOT, METH, MKSM, MNT,
	},
	AccountId, AccountIndex, Amount, Balance, BlockNumber, CurrencyId, DataProviderId, DigestItem, Hash, Index,
	Interest, Moment, Operation, Price, Rate, Signature, VestingBucket,
};
pub use mnt_token_rpc_runtime_api::MntBalanceInfo;
use orml_currencies::BasicCurrencyAdapter;
use orml_traits::{create_median_value_data_provider, parameter_type_with_key, DataFeeder, DataProviderExtended};
use pallet_traits::{ControllerManager, LiquidityPoolStorageProvider, MntManager, PricesManager, WhitelistManager};
use pallet_transaction_payment::{CurrencyAdapter, Multiplier, TargetedFeeAdjustment};
use sp_api::impl_runtime_apis;
use sp_consensus_aura::sr25519::AuthorityId as AuraId;
use sp_core::{
	crypto::KeyTypeId,
	u32_trait::{_1, _2, _3, _4},
	OpaqueMetadata,
};
use sp_runtime::{
	create_runtime_str, generic, impl_opaque_keys,
	traits::{AccountIdConversion, AccountIdLookup, BlakeTwo256, Block as BlockT, One, Zero},
	transaction_validity::{TransactionPriority, TransactionSource, TransactionValidity},
	ApplyExtrinsicResult, DispatchResult, FixedPointNumber,
};
use sp_std::{cmp::Ordering, convert::TryFrom, prelude::*};
#[cfg(feature = "std")]
use sp_version::NativeVersion;
use sp_version::RuntimeVersion;

// A few exports that help ease life for downstream crates.
pub use frame_support::{
	construct_runtime, debug, parameter_types,
	traits::{KeyOwnerProofSystem, Randomness, SortedMembers},
	weights::{
		constants::{BlockExecutionWeight, ExtrinsicBaseWeight, RocksDbWeight, WEIGHT_PER_SECOND},
		DispatchClass, IdentityFee, Weight,
	},
	IterableStorageDoubleMap, PalletId, StorageValue,
};
pub use pallet_balances::Call as BalancesCall;
pub use pallet_timestamp::Call as TimestampCall;
#[cfg(any(feature = "std", test))]
pub use sp_runtime::BuildStorage;
pub use sp_runtime::{Perbill, Permill, Perquintill};

/// Opaque types. These are used by the CLI to instantiate machinery that don't need to know
/// the specifics of the runtime. They can then be made to be agnostic over specific formats
/// of data like extrinsics, allowing for them to continue syncing the network through upgrades
/// to even the core data structures.
pub mod opaque {
	use super::*;

	pub use sp_runtime::OpaqueExtrinsic as UncheckedExtrinsic;

	/// Opaque block header type.
	pub type Header = generic::Header<BlockNumber, BlakeTwo256>;
	/// Opaque block type.
	pub type Block = generic::Block<Header, UncheckedExtrinsic>;
	/// Opaque block identifier type.
	pub type BlockId = generic::BlockId<Block>;

	impl_opaque_keys! {
		pub struct SessionKeys {
			pub aura: Aura,
		}
	}
}

pub const VERSION: RuntimeVersion = RuntimeVersion {
	spec_name: create_runtime_str!("node-minterest"),
	impl_name: create_runtime_str!("node-minterest"),
	authoring_version: 1,
	spec_version: 1,
	impl_version: 1,
	apis: RUNTIME_API_VERSIONS,
	transaction_version: 1,
};

/// The version information used to identify this runtime when compiled natively.
#[cfg(feature = "std")]
pub fn native_version() -> NativeVersion {
	NativeVersion {
		runtime_version: VERSION,
		can_author_with: Default::default(),
	}
}

// Pallet accounts of runtime
parameter_types! {
	pub const MntTokenPalletId: PalletId = PalletId(*b"min/mntt");
	pub const LiquidationPoolsPalletId: PalletId = PalletId(*b"min/lqdn");
	pub const DexPalletId: PalletId = PalletId(*b"min/dexs");
	pub const LiquidityPoolsPalletId: PalletId = PalletId(*b"min/lqdy");
}

// Do not change the order of modules. Used for genesis block.
pub fn get_all_modules_accounts() -> Vec<AccountId> {
	vec![
		MntTokenPalletId::get().into_account(),
		LiquidationPoolsPalletId::get().into_account(),
		DexPalletId::get().into_account(),
		LiquidityPoolsPalletId::get().into_account(),
	]
}

const NORMAL_DISPATCH_RATIO: Perbill = Perbill::from_percent(75);

parameter_types! {
	pub const Version: RuntimeVersion = VERSION;
	pub const BlockHashCount: BlockNumber = 2400;
	/// We allow for 2 seconds of compute with a 6 second average block time.
	pub BlockWeights: frame_system::limits::BlockWeights = frame_system::limits::BlockWeights
		::with_sensible_defaults(2 * WEIGHT_PER_SECOND, NORMAL_DISPATCH_RATIO);
	pub BlockLength: frame_system::limits::BlockLength = frame_system::limits::BlockLength
		::max_with_normal_ratio(5 * 1024 * 1024, NORMAL_DISPATCH_RATIO);
	pub const SS58Prefix: u8 = 42;
}

// Configure FRAME pallets to include in runtime.

impl frame_system::Config for Runtime {
	/// The basic call filter to use in dispatchable.
	type BaseCallFilter = ();
	/// Block & extrinsics weights: base values and limits.
	type BlockWeights = BlockWeights;
	/// The maximum length of a block (in bytes).
	type BlockLength = BlockLength;
	/// The identifier used to distinguish between accounts.
	type AccountId = AccountId;
	/// The aggregated dispatch type that is available for extrinsics.
	type Call = Call;
	/// The lookup mechanism to get account ID from whatever is passed in dispatchers.
	type Lookup = AccountIdLookup<AccountId, ()>;
	/// The index type for storing how many extrinsics an account has signed.
	type Index = Index;
	/// The index type for blocks.
	type BlockNumber = BlockNumber;
	/// The type for hashing blocks and tries.
	type Hash = Hash;
	/// The hashing algorithm used.
	type Hashing = BlakeTwo256;
	/// The header type.
	type Header = generic::Header<BlockNumber, BlakeTwo256>;
	/// The ubiquitous event type.
	type Event = Event;
	/// The ubiquitous origin type.
	type Origin = Origin;
	/// Maximum number of block number to block hash mappings to keep (oldest pruned first).
	type BlockHashCount = BlockHashCount;
	/// The weight of database operations that the runtime can invoke.
	type DbWeight = RocksDbWeight;
	/// Version of the runtime.
	type Version = Version;
	/// Converts a module to the index of the module in `construct_runtime!`.
	///
	/// This type is being generated by `construct_runtime!`.
	type PalletInfo = PalletInfo;
	/// What to do if a new account is created.
	type OnNewAccount = ();
	/// What to do if an account is fully reaped from the system.
	type OnKilledAccount = ();
	/// The data to be stored in an account.
	type AccountData = pallet_balances::AccountData<Balance>;
	/// Weight information for the extrinsics of this pallet.
	type SystemWeightInfo = ();
	/// This is used as an identifier of the chain. 42 is the generic substrate prefix.
	type SS58Prefix = SS58Prefix;
	/// What to do if the user wants the code set to something.
	type OnSetCode = cumulus_pallet_parachain_system::ParachainSetCode<Self>;
}

impl pallet_aura::Config for Runtime {
	type AuthorityId = AuraId;
}

const MAXIMUM_BLOCK_WEIGHT: Weight = WEIGHT_PER_SECOND * 2;

parameter_types! {
	// pub const ReservedXcmpWeight: Weight = MAXIMUM_BLOCK_WEIGHT / 4;
	pub const ReservedDmpWeight: Weight = MAXIMUM_BLOCK_WEIGHT / 4;
}

impl cumulus_pallet_parachain_system::Config for Runtime {
	type Event = Event;
	type OnValidationData = ();
	type SelfParaId = parachain_info::Pallet<Runtime>;
	type OutboundXcmpMessageSource = ();
	type DmpMessageHandler = ();
	type ReservedDmpWeight = ReservedDmpWeight;
	type XcmpMessageHandler = ();
	type ReservedXcmpWeight = ();
}

impl parachain_info::Config for Runtime {}

impl cumulus_pallet_aura_ext::Config for Runtime {}

parameter_types! {
	pub const MinimumPeriod: u64 = SLOT_DURATION / 2;
}

impl pallet_timestamp::Config for Runtime {
	/// A timestamp: milliseconds since the unix epoch.
	type Moment = Moment;
	type OnTimestampSet = Aura;
	type MinimumPeriod = MinimumPeriod;
	type WeightInfo = ();
}

parameter_types! {
	pub const ExistentialDeposit: Balance = DOLLARS; // 1 MNT
	pub const MaxLocks: u32 = 50;
	pub const MaxReserves: u32 = 256;
}

impl pallet_balances::Config for Runtime {
	type MaxLocks = MaxLocks;
	/// The type for recording an account's balance.
	type Balance = Balance;
	/// The ubiquitous event type.
	type Event = Event;
	/// The maximum number of named reserves that can exist on an account.
	type MaxReserves = MaxReserves;
	/// The id type for named reserves.
	type ReserveIdentifier = [u8; 8];
	type DustRemoval = ();
	type ExistentialDeposit = ExistentialDeposit;
	type AccountStore = System;
	type WeightInfo = pallet_balances::weights::SubstrateWeight<Runtime>;
}

parameter_types! {
	pub TransactionByteFee: Balance = 3_570_000_000_000_000;
	pub const TargetBlockFullness: Perquintill = Perquintill::from_percent(25);
	pub AdjustmentVariable: Multiplier = Multiplier::saturating_from_rational(1, 100_000);
	// FIXME: Temporary value to get multiplier equal to 1
	pub MinimumMultiplier: Multiplier = Multiplier::one();
}

impl pallet_transaction_payment::Config for Runtime {
	type OnChargeTransaction = CurrencyAdapter<Balances, ()>;
	type TransactionByteFee = TransactionByteFee;
	type WeightToFee = WeightToFee;
	type FeeMultiplierUpdate = TargetedFeeAdjustment<Self, TargetBlockFullness, AdjustmentVariable, MinimumMultiplier>;
}

impl pallet_sudo::Config for Runtime {
	type Event = Event;
	type Call = Call;
}

type EnsureRootOrTwoThirdsMinterestCouncil = EnsureOneOf<
	AccountId,
	EnsureRoot<AccountId>,
	pallet_collective::EnsureProportionMoreThan<_2, _3, AccountId, MinterestCouncilInstance>,
>;

type EnsureRootOrThreeFourthsMinterestCouncil = EnsureOneOf<
	AccountId,
	EnsureRoot<AccountId>,
	pallet_collective::EnsureProportionMoreThan<_3, _4, AccountId, MinterestCouncilInstance>,
>;

type EnsureRootOrHalfMinterestCouncil = EnsureOneOf<
	AccountId,
	EnsureRoot<AccountId>,
	pallet_collective::EnsureProportionMoreThan<_1, _2, AccountId, MinterestCouncilInstance>,
>;

parameter_types! {
	pub const MinterestCouncilMotionDuration: BlockNumber = 7 * DAYS;
	pub const MinterestCouncilMaxProposals: u32 = 100;
	pub const MinterestCouncilMaxMembers: u32 = 100;
}

type MinterestCouncilInstance = pallet_collective::Instance1;
impl pallet_collective::Config<MinterestCouncilInstance> for Runtime {
	type Origin = Origin;
	type Proposal = Call;
	type Event = Event;
	type MotionDuration = MinterestCouncilMotionDuration;
	type MaxProposals = MinterestCouncilMaxProposals;
	type MaxMembers = MinterestCouncilMaxMembers;
	type DefaultVote = pallet_collective::PrimeDefaultVote;
	type WeightInfo = ();
}

type MinterestCouncilMembershipInstance = pallet_membership::Instance1;
impl pallet_membership::Config<MinterestCouncilMembershipInstance> for Runtime {
	type Event = Event;
	type AddOrigin = EnsureRootOrThreeFourthsMinterestCouncil;
	type RemoveOrigin = EnsureRootOrThreeFourthsMinterestCouncil;
	type SwapOrigin = EnsureRootOrThreeFourthsMinterestCouncil;
	type ResetOrigin = EnsureRootOrThreeFourthsMinterestCouncil;
	type PrimeOrigin = EnsureRootOrThreeFourthsMinterestCouncil;
	type MembershipInitialized = MinterestCouncil;
	type MembershipChanged = MinterestCouncil;
	type MaxMembers = MinterestCouncilMaxMembers;
	type WeightInfo = ();
}

// It is possible to remove MinterestOracle and this pallets after implementing chainlink.
// If we decided to save it. Need to fix MembershipInitialized and MembershipChanged types.
// TODO MIN-446
type OperatorMembershipInstanceMinterest = pallet_membership::Instance2;
impl pallet_membership::Config<OperatorMembershipInstanceMinterest> for Runtime {
	type Event = Event;
	type AddOrigin = EnsureRootOrTwoThirdsMinterestCouncil;
	type RemoveOrigin = EnsureRootOrTwoThirdsMinterestCouncil;
	type SwapOrigin = EnsureRootOrTwoThirdsMinterestCouncil;
	type ResetOrigin = EnsureRootOrTwoThirdsMinterestCouncil;
	type PrimeOrigin = EnsureRootOrTwoThirdsMinterestCouncil;
	type MembershipInitialized = ();
	type MembershipChanged = MinterestOracle;
	type MaxMembers = MinterestCouncilMaxMembers;
	type WeightInfo = ();
}

impl minterest_protocol::Config for Runtime {
	type Event = Event;
	type MultiCurrency = Currencies;
	type ManagerLiquidationPools = LiquidationPools;
	type ManagerLiquidityPools = LiquidityPools;
	type MntManager = MntToken;
	type ProtocolWeightInfo = weights::minterest_protocol::WeightInfo<Runtime>;
	type ControllerManager = Controller;
	type MinterestModelManager = MinterestModel;
	type CreatePoolOrigin = EnsureRootOrHalfMinterestCouncil;
	type UserLiquidationAttempts = RiskManager;
	type RiskManager = RiskManager;
	type WhitelistManager = Whitelist;
}

parameter_type_with_key! {
	pub ExistentialDeposits: |_currency_id: CurrencyId| -> Balance {
		Zero::zero()
	};
}

impl orml_tokens::Config for Runtime {
	type Event = Event;
	type Balance = Balance;
	type Amount = Amount;
	type CurrencyId = CurrencyId;
	type WeightInfo = ();
	type ExistentialDeposits = ExistentialDeposits;
	type OnDust = ();
	type MaxLocks = MaxLocks;
}

parameter_types! {
	pub const GetMinterestCurrencyId: CurrencyId = MNT;
}

pub type MinterestToken = BasicCurrencyAdapter<Runtime, Balances, Amount, BlockNumber>;

impl orml_currencies::Config for Runtime {
	type Event = Event;
	type MultiCurrency = Tokens;
	type NativeCurrency = MinterestToken;
	type GetNativeCurrencyId = GetMinterestCurrencyId;
	type WeightInfo = ();
}

parameter_types! {
	pub LiquidityPoolAccountId: AccountId = LiquidityPoolsPalletId::get().into_account();
	pub const InitialExchangeRate: Rate = INITIAL_EXCHANGE_RATE;
	pub EnabledUnderlyingAssetsIds: Vec<CurrencyId> = CurrencyId::get_enabled_tokens_in_protocol(UnderlyingAsset);
	pub EnabledWrappedTokensId: Vec<CurrencyId> = CurrencyId::get_enabled_tokens_in_protocol(WrappedToken);
}

impl liquidity_pools::Config for Runtime {
	type MultiCurrency = Currencies;
	type PriceSource = Prices;
	type PalletId = LiquidityPoolsPalletId;
	type LiquidityPoolAccountId = LiquidityPoolAccountId;
	type InitialExchangeRate = InitialExchangeRate;
	type EnabledUnderlyingAssetsIds = EnabledUnderlyingAssetsIds;
	type EnabledWrappedTokensId = EnabledWrappedTokensId;
}

parameter_types! {
	pub const MaxBorrowCap: Balance = MAX_BORROW_CAP;
}

impl controller::Config for Runtime {
	type Event = Event;
	type MultiCurrency = Currencies;
	type PriceSource = Prices;
	type LiquidityPoolsManager = LiquidityPools;
	type MinterestModelManager = MinterestModel;
	type MaxBorrowCap = MaxBorrowCap;
	type UpdateOrigin = EnsureRootOrHalfMinterestCouncil;
	type ControllerWeightInfo = weights::controller::WeightInfo<Runtime>;
	type MntManager = MntToken;
}

impl module_prices::Config for Runtime {
	type Event = Event;
	type Source = AggregatedDataProvider;
	type LockOrigin = EnsureRootOrTwoThirdsMinterestCouncil;
	type WeightInfo = weights::prices::WeightInfo<Runtime>;
}

parameter_types! {
	pub const BlocksPerYear: u128 = BLOCKS_PER_YEAR;
}

impl minterest_model::Config for Runtime {
	type Event = Event;
	type BlocksPerYear = BlocksPerYear;
	type ModelUpdateOrigin = EnsureRootOrHalfMinterestCouncil;
	type WeightInfo = weights::minterest_model::WeightInfo<Runtime>;
}

parameter_types! {
	pub const RiskManagerPriority: TransactionPriority = TransactionPriority::max_value();
	pub const LiquidityPoolsPriority: TransactionPriority = TransactionPriority::max_value() - 1;
	pub const PartialLiquidationMinSum: Balance = PARTIAL_LIQUIDATION_MIN_SUM;
	pub const PartialLiquidationMaxAttempts: u8 = PARTIAL_LIQUIDATION_MAX_ATTEMPTS;
	pub const MaxLiquidationFee: Rate = MAX_LIQUIDATION_FEE;
}

impl risk_manager::Config for Runtime {
	type Event = Event;
	type UnsignedPriority = RiskManagerPriority;
	type PriceSource = Prices;
	type UserCollateral = LiquidityPools;
	type PartialLiquidationMinSum = PartialLiquidationMinSum;
	type PartialLiquidationMaxAttempts = PartialLiquidationMaxAttempts;
	type MaxLiquidationFee = MaxLiquidationFee;
	type RiskManagerUpdateOrigin = EnsureRootOrHalfMinterestCouncil;
	type ControllerManager = Controller;
	type LiquidityPoolsManager = LiquidityPools;
	type LiquidationPoolsManager = LiquidationPools;
	type MinterestProtocolManager = MinterestProtocol;
}

parameter_types! {
	pub MntTokenAccountId: AccountId = MntTokenPalletId::get().into_account();
}

impl mnt_token::Config for Runtime {
	type Event = Event;
	type PriceSource = Prices;
	type UpdateOrigin = EnsureRootOrTwoThirdsMinterestCouncil;
	type LiquidityPoolsManager = LiquidityPools;
	type MultiCurrency = Currencies;
	type ControllerManager = Controller;
	type MntTokenAccountId = MntTokenAccountId;
	type MntTokenWeightInfo = weights::mnt_token::WeightInfo<Runtime>;
}

impl<C> frame_system::offchain::SendTransactionTypes<C> for Runtime
where
	Call: From<C>,
{
	type OverarchingCall = Call;
	type Extrinsic = UncheckedExtrinsic;
}

parameter_types! {
	pub LiquidationPoolAccountId: AccountId = LiquidationPoolsPalletId::get().into_account();
}

impl liquidation_pools::Config for Runtime {
	type Event = Event;
	type MultiCurrency = Currencies;
	type UnsignedPriority = LiquidityPoolsPriority;
	type LiquidationPoolAccountId = LiquidationPoolAccountId;
	type PriceSource = Prices;
	type LiquidationPoolsPalletId = LiquidationPoolsPalletId;
	type UpdateOrigin = EnsureRootOrHalfMinterestCouncil;
	type LiquidityPoolsManager = LiquidityPools;
	type Dex = Dex;
	type LiquidationPoolsWeightInfo = weights::liquidation_pools::WeightInfo<Runtime>;
}

parameter_types! {
	pub const MinimumCount: u32 = 1;
	pub const ExpiresIn: Moment = 1000 * 60 * 60; // 60 mins
	pub ZeroAccountId: AccountId = AccountId::from([0u8; 32]);
	pub const MaxHasDispatchedSize: u32 = 100;
}

pub type TimeStampedPrice = orml_oracle::TimestampedValue<Price, minterest_primitives::Moment>;
type MinterestDataProvider = orml_oracle::Instance1;
impl orml_oracle::Config<MinterestDataProvider> for Runtime {
	type Event = Event;
	type OnNewData = ();
	type CombineData = orml_oracle::DefaultCombineData<Runtime, MinimumCount, ExpiresIn, MinterestDataProvider>;
	type Time = Timestamp;
	type OracleKey = CurrencyId;
	type OracleValue = Price;
	type RootOperatorAccountId = ZeroAccountId;
	type WeightInfo = ();
	type Members = OperatorMembershipMinterest;
	type MaxHasDispatchedSize = MaxHasDispatchedSize;
}

create_median_value_data_provider!(
	AggregatedDataProvider,
	CurrencyId,
	Price,
	TimeStampedPrice,
	[MinterestOracle]
);
// Aggregated data provider cannot feed.
impl DataFeeder<CurrencyId, Price, AccountId> for AggregatedDataProvider {
	fn feed_value(_: AccountId, _: CurrencyId, _: Price) -> DispatchResult {
		Err("Not supported".into())
	}
}

parameter_types! {
	pub DexAccountId: AccountId = DexPalletId::get().into_account();
}

impl dex::Config for Runtime {
	type Event = Event;
	type MultiCurrency = Currencies;
	type DexPalletId = DexPalletId;
	type DexAccountId = DexAccountId;
}

parameter_types! {
	pub MinVestedTransfer: Balance = DOLLARS; // 1 USD
	pub const MaxVestingSchedules: u32 = 2;
	pub VestingBucketsInfo: Vec<(VestingBucket, u8, u8, Balance)> = VestingBucket::get_vesting_buckets_info();
}

impl module_vesting::Config for Runtime {
	type Event = Event;
	type Currency = pallet_balances::Pallet<Runtime>;
	type MinVestedTransfer = MinVestedTransfer;
	type VestedTransferOrigin = EnsureRootOrTwoThirdsMinterestCouncil;
	type WeightInfo = weights::vesting::WeightInfo<Runtime>;
	type MaxVestingSchedules = MaxVestingSchedules;
	type VestingBucketsInfo = VestingBucketsInfo;
}

parameter_types! {
	pub const MaxMembersWhitelistMode: u8 = 100;
}

impl whitelist_module::Config for Runtime {
	type Event = Event;
	type MaxMembers = MaxMembersWhitelistMode;
	type WhitelistOrigin = EnsureRootOrHalfMinterestCouncil;
	type WhitelistWeightInfo = weights::whitelist::WeightInfo<Runtime>;
}

// Create the runtime by composing the FRAME pallets that were previously configured.
construct_runtime!(
	pub enum Runtime where
		Block = Block,
		NodeBlock = opaque::Block,
		UncheckedExtrinsic = UncheckedExtrinsic
	{
		System: frame_system::{Pallet, Call, Config, Storage, Event<T>},
		Timestamp: pallet_timestamp::{Pallet, Call, Storage, Inherent},

		// Tokens & Related
		Balances: pallet_balances::{Pallet, Call, Storage, Config<T>, Event<T>},
		Tokens: orml_tokens::{Pallet, Storage, Call, Event<T>, Config<T>},
		Currencies: orml_currencies::{Pallet, Call, Event<T>},
		Vesting: module_vesting::{Pallet, Storage, Call, Event<T>, Config<T>},
		TransactionPayment: pallet_transaction_payment::{Pallet, Storage},

		// Parachain
		ParachainSystem: cumulus_pallet_parachain_system::{Pallet, Call, Config, Storage, Inherent, Event<T>},
		ParachainInfo: parachain_info::{Pallet, Storage, Config},

		// Consensus & Staking
		Aura: pallet_aura::{Pallet, Config<T>},
		AuraExt: cumulus_pallet_aura_ext::{Pallet, Config},

		// Governance
		MinterestCouncil: pallet_collective::<Instance1>::{Pallet, Call, Storage, Origin<T>, Event<T>, Config<T>},
		MinterestCouncilMembership: pallet_membership::<Instance1>::{Pallet, Call, Storage, Event<T>, Config<T>},

		// Oracle and Prices
		// OperatorMembership must be placed after Oracle or else will have race condition on initialization
		MinterestOracle: orml_oracle::<Instance1>::{Pallet, Storage, Call, Event<T>},
		Prices: module_prices::{Pallet, Storage, Call, Event<T>, Config<T>},
		OperatorMembershipMinterest: pallet_membership::<Instance2>::{Pallet, Call, Storage, Event<T>, Config<T>},

		// Minterest pallets
		MinterestProtocol: minterest_protocol::{Pallet, Call, Event<T>},
		LiquidityPools: liquidity_pools::{Pallet, Storage, Call, Config<T>},
		Controller: controller::{Pallet, Storage, Call, Event, Config<T>},
		MinterestModel: minterest_model::{Pallet, Storage, Call, Event, Config<T>},
		RiskManager: risk_manager::{Pallet, Storage, Call, Event<T>, Config<T>, ValidateUnsigned},
		LiquidationPools: liquidation_pools::{Pallet, Storage, Call, Event<T>, Config<T>, ValidateUnsigned},
		MntToken: mnt_token::{Pallet, Storage, Call, Event<T>, Config<T>},
		Dex: dex::{Pallet, Storage, Call, Event<T>},
		Whitelist: whitelist_module::{Pallet, Storage, Call, Event<T>, Config<T>},

		// Dev
		Sudo: pallet_sudo::{Pallet, Call, Config<T>, Storage, Event<T>},
	}
);

/// The address format for describing accounts.
pub type Address = sp_runtime::MultiAddress<AccountId, ()>;
/// Block header type as expected by this runtime.
pub type Header = generic::Header<BlockNumber, BlakeTwo256>;
/// Block type as expected by this runtime.
pub type Block = generic::Block<Header, UncheckedExtrinsic>;
/// A Block signed with a Justification
pub type SignedBlock = generic::SignedBlock<Block>;
/// BlockId type as expected by this runtime.
pub type BlockId = generic::BlockId<Block>;
/// The SignedExtension to the basic transaction logic.
pub type SignedExtra = (
	frame_system::CheckSpecVersion<Runtime>,
	frame_system::CheckTxVersion<Runtime>,
	frame_system::CheckGenesis<Runtime>,
	frame_system::CheckEra<Runtime>,
	frame_system::CheckNonce<Runtime>,
	frame_system::CheckWeight<Runtime>,
	pallet_transaction_payment::ChargeTransactionPayment<Runtime>,
);
/// Unchecked extrinsic type as expected by this runtime.
pub type UncheckedExtrinsic = generic::UncheckedExtrinsic<Address, Call, Signature, SignedExtra>;
/// Extrinsic type that has already been checked.
pub type CheckedExtrinsic = generic::CheckedExtrinsic<AccountId, Call, SignedExtra>;
/// Executive: handles dispatch to the various pallets.
pub type Executive =
	frame_executive::Executive<Runtime, Block, frame_system::ChainContext<Runtime>, Runtime, AllPallets>;

impl_runtime_apis! {
	impl sp_api::Core<Block> for Runtime {
		fn version() -> RuntimeVersion {
			VERSION
		}

		fn execute_block(block: Block) {
			Executive::execute_block(block)
		}

		fn initialize_block(header: &<Block as BlockT>::Header) {
			Executive::initialize_block(header)
		}
	}

	impl sp_api::Metadata<Block> for Runtime {
		fn metadata() -> OpaqueMetadata {
			Runtime::metadata().into()
		}
	}

	impl sp_block_builder::BlockBuilder<Block> for Runtime {
		fn apply_extrinsic(extrinsic: <Block as BlockT>::Extrinsic) -> ApplyExtrinsicResult {
			Executive::apply_extrinsic(extrinsic)
		}

		fn finalize_block() -> <Block as BlockT>::Header {
			Executive::finalize_block()
		}

		fn inherent_extrinsics(data: sp_inherents::InherentData) -> Vec<<Block as BlockT>::Extrinsic> {
			data.create_extrinsics()
		}

		fn check_inherents(
			block: Block,
			data: sp_inherents::InherentData,
		) -> sp_inherents::CheckInherentsResult {
			data.check_extrinsics(&block)
		}
	}

	impl sp_transaction_pool::runtime_api::TaggedTransactionQueue<Block> for Runtime {
		fn validate_transaction(
			source: TransactionSource,
			tx: <Block as BlockT>::Extrinsic,
			block_hash: <Block as BlockT>::Hash,
		) -> TransactionValidity {
			Executive::validate_transaction(source, tx, block_hash)
		}
	}

	impl sp_offchain::OffchainWorkerApi<Block> for Runtime {
		fn offchain_worker(header: &<Block as BlockT>::Header) {
			Executive::offchain_worker(header)
		}
	}

	impl sp_consensus_aura::AuraApi<Block, AuraId> for Runtime {
		fn slot_duration() -> sp_consensus_aura::SlotDuration {
			sp_consensus_aura::SlotDuration::from_millis(Aura::slot_duration())
		}

		fn authorities() -> Vec<AuraId> {
			Aura::authorities()
		}
	}

	impl sp_session::SessionKeys<Block> for Runtime {
		fn generate_session_keys(seed: Option<Vec<u8>>) -> Vec<u8> {
			opaque::SessionKeys::generate(seed)
		}

		fn decode_session_keys(
			encoded: Vec<u8>,
		) -> Option<Vec<(Vec<u8>, KeyTypeId)>> {
			opaque::SessionKeys::decode_into_raw_public_keys(&encoded)
		}
	}

	impl frame_system_rpc_runtime_api::AccountNonceApi<Block, AccountId, Index> for Runtime {
		fn account_nonce(account: AccountId) -> Index {
			System::account_nonce(account)
		}
	}

	impl pallet_transaction_payment_rpc_runtime_api::TransactionPaymentApi<Block, Balance> for Runtime {
		fn query_info(
			uxt: <Block as BlockT>::Extrinsic,
			len: u32,
		) -> pallet_transaction_payment_rpc_runtime_api::RuntimeDispatchInfo<Balance> {
			TransactionPayment::query_info(uxt, len)
		}
		fn query_fee_details(
			uxt: <Block as BlockT>::Extrinsic,
			len: u32,
		) -> pallet_transaction_payment::FeeDetails<Balance> {
			TransactionPayment::query_fee_details(uxt, len)
		}
	}

	impl controller_rpc_runtime_api::ControllerRuntimeApi<Block, AccountId> for Runtime {
		// TODO: Fill RPC with real data according to task MIN-483
		fn get_user_data(_account_id: AccountId) -> Option<UserData> {
			Some(UserData { total_collateral_in_usd: Balance::one(), total_supply_in_usd: Balance::one(), total_borrow_in_usd: Balance::one(), total_supply_apy: Rate::one(), total_borrow_apy: Rate::one(), net_apy: Rate::one() })
		}

		fn get_protocol_total_values() -> Option<ProtocolTotalValue> {
		let (pool_total_supply_in_usd, pool_total_borrow_in_usd, tvl_in_usd, pool_total_protocol_interest_in_usd) = Controller::get_protocol_total_values().ok()?;
				Some(ProtocolTotalValue{pool_total_supply_in_usd, pool_total_borrow_in_usd, tvl_in_usd, pool_total_protocol_interest_in_usd })
		}

		fn liquidity_pool_state(pool_id: CurrencyId) -> Option<PoolState> {
			let (exchange_rate, borrow_rate, supply_rate) = Controller::get_pool_exchange_borrow_and_supply_rates(pool_id)?;
			Some(PoolState { exchange_rate, borrow_rate, supply_rate })
		}

		fn get_pool_utilization_rate(pool_id: CurrencyId) -> Option<Rate> {
			Controller::get_pool_utilization_rate(pool_id)
		}

		fn get_user_total_supply_and_borrow_balance_in_usd(account_id: AccountId) -> Option<UserPoolBalanceData> {
			let (total_supply_in_usd, total_borrowed_in_usd) = Controller::get_user_total_supply_and_borrow_balance_in_usd(&account_id).ok()?;

			Some(UserPoolBalanceData {total_supply_in_usd, total_borrowed_in_usd})
		}

		fn get_hypothetical_account_liquidity(account_id: AccountId) -> Option<HypotheticalLiquidityData> {
			let (excess, shortfall) = Controller::get_hypothetical_account_liquidity(&account_id, MNT, 0, 0).ok()?;
			let res = match excess.cmp(&shortfall) {
				Ordering::Less => {
					let amount = Amount::try_from(shortfall).ok()?;
					amount.checked_neg()?
				},
				_ => Amount::try_from(excess).ok()?
			};

			Some(HypotheticalLiquidityData{ liquidity_in_usd: res })
		}

		fn is_admin(caller: AccountId) -> Option<bool> {
				Some(MinterestCouncil::is_member(&caller))
		}

		fn get_user_total_collateral(account_id: AccountId) -> Option<BalanceInfo> {
				Some(BalanceInfo{amount: Controller::get_user_total_collateral(account_id).ok()?})
		}

		fn get_user_borrow_per_asset(account_id: AccountId, underlying_asset_id: CurrencyId) -> Option<BalanceInfo> {
				Some(BalanceInfo{amount: Controller::get_user_borrow_underlying_balance(&account_id, underlying_asset_id).ok()?})
		}

		fn get_user_underlying_balance_per_asset(account_id: AccountId, pool_id: CurrencyId) -> Option<BalanceInfo> {
				Some(BalanceInfo{amount: Controller::get_user_supply_underlying_balance(&account_id, pool_id).ok()?})
		}

		fn pool_exists(underlying_asset_id: CurrencyId) -> bool {
			LiquidityPools::pool_exists(&underlying_asset_id)
		}

		fn get_user_total_supply_borrow_and_net_apy(account_id: AccountId) -> Option<(Interest, Interest, Interest)> {
			Controller::get_user_total_supply_borrow_and_net_apy(account_id).ok()
		}
	}

	impl mnt_token_rpc_runtime_api::MntTokenRuntimeApi<Block, AccountId> for Runtime {
		fn get_user_total_unclaimed_mnt_balance(account_id: AccountId) -> Option<MntBalanceInfo> {
				Some(MntBalanceInfo{amount: MntToken::get_user_total_unclaimed_mnt_balance(&account_id).ok()?})
		}

		fn get_pool_mnt_borrow_and_supply_rates(pool_id: CurrencyId) -> Option<(Rate, Rate)> {
			MntToken::get_pool_mnt_borrow_and_supply_rates(pool_id).ok()
		}
	}

	impl whitelist_rpc_runtime_api::WhitelistRuntimeApi<Block, AccountId> for Runtime {
		fn is_whitelist_member(who: AccountId) -> bool {
				Whitelist::is_whitelist_member(&who)
		}
	}

	impl orml_oracle_rpc_runtime_api::OracleApi<
		Block,
		DataProviderId,
		CurrencyId,
		TimeStampedPrice,
	> for Runtime {
		fn get_value(provider_id: DataProviderId, key: CurrencyId) -> Option<TimeStampedPrice> {
			match provider_id {
				DataProviderId::Minterest => MinterestOracle::get_no_op(&key),
				DataProviderId::Aggregated => <AggregatedDataProvider as DataProviderExtended<_, _>>::get_no_op(&key)
			}
		}

		fn get_all_values(provider_id: DataProviderId) -> Vec<(CurrencyId, Option<TimeStampedPrice>)> {
			match provider_id {
				DataProviderId::Minterest => MinterestOracle::get_all_values(),
				DataProviderId::Aggregated => <AggregatedDataProvider as DataProviderExtended<_, _>>::get_all_values()
			}
		}
	}

	impl prices_rpc_runtime_api::PricesRuntimeApi<Block> for Runtime {
		fn  get_current_price(currency_id: CurrencyId) -> Option<Price> {
			Prices::get_underlying_price(currency_id)
		}

		fn  get_all_locked_prices() -> Vec<(CurrencyId, Option<Price>)> {
			CurrencyId::get_enabled_tokens_in_protocol(UnderlyingAsset)
				.into_iter()
				.map(|currency_id| (currency_id, Prices::locked_price_storage(currency_id)))
				.collect()
		}

		fn get_all_freshest_prices() -> Vec<(CurrencyId, Option<Price>)> {
			Prices::get_all_freshest_prices()
		}
	}

	impl cumulus_primitives_core::CollectCollationInfo<Block> for Runtime {
		fn collect_collation_info() -> cumulus_primitives_core::CollationInfo {
			ParachainSystem::collect_collation_info()
		}
	}
}

struct CheckInherents;

impl cumulus_pallet_parachain_system::CheckInherents<Block> for CheckInherents {
	fn check_inherents(
		block: &Block,
		relay_state_proof: &cumulus_pallet_parachain_system::RelayChainStateProof,
	) -> sp_inherents::CheckInherentsResult {
		let relay_chain_slot = relay_state_proof
			.read_slot()
			.expect("Could not read the relay chain slot from the proof");

		let inherent_data = cumulus_primitives_timestamp::InherentDataProvider::from_relay_chain_slot_and_duration(
			relay_chain_slot,
			sp_std::time::Duration::from_secs(6),
		)
		.create_inherent_data()
		.expect("Could not create the timestamp inherent data");

		inherent_data.check_extrinsics(&block)
	}
}

cumulus_pallet_parachain_system::register_validate_block! {
	Runtime = Runtime,
	BlockExecutor = cumulus_pallet_aura_ext::BlockExecutor::<Runtime, Executive>,
	CheckInherents = CheckInherents,
}
