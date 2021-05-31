#![cfg_attr(not(feature = "std"), no_std)]
// `construct_runtime!` does a lot of recursion and requires us to increase the limit to 256.
#![recursion_limit = "256"]
// The `large_enum_variant` warning originates from `construct_runtime` macro.
#![allow(clippy::large_enum_variant)]
#![allow(clippy::from_over_into)]

// Make the WASM binary available.
#[cfg(feature = "std")]
include!(concat!(env!("OUT_DIR"), "/wasm_binary.rs"));

mod benchmarking;
mod constants;
#[cfg(test)]
mod tests;
mod weights;
mod weights_test;

use crate::constants::fee::WeightToFee;
pub use controller_rpc_runtime_api::{BalanceInfo, HypotheticalLiquidityData, PoolState, UserPoolBalanceData};
pub use minterest_primitives::{
	currency::{
		CurrencyType::{UnderlyingAsset, WrappedToken},
		BTC, DOT, ETH, KSM, MBTC, MDOT, METH, MKSM, MNT,
	},
	AccountId, AccountIndex, Amount, Balance, BlockNumber, CurrencyId, DataProviderId, DigestItem, Hash, Index, Moment,
	Operation, Price, Rate, Signature,
};
pub use mnt_token_rpc_runtime_api::MntBalanceInfo;
use orml_currencies::BasicCurrencyAdapter;
use orml_traits::{create_median_value_data_provider, parameter_type_with_key, DataFeeder, DataProviderExtended};
// use pallet_grandpa::fg_primitives;
// use pallet_grandpa::{AuthorityId as GrandpaId, AuthorityList as GrandpaAuthorityList};
use pallet_traits::ControllerAPI;
use pallet_transaction_payment::{Multiplier, TargetedFeeAdjustment};
use sp_api::impl_runtime_apis;
pub use sp_consensus_aura::sr25519::AuthorityId as AuraId;
use sp_core::{
	crypto::KeyTypeId,
	u32_trait::{_1, _2, _3, _4},
	OpaqueMetadata,
};
use sp_runtime::traits::{AccountIdConversion, AccountIdLookup, BlakeTwo256, Block as BlockT, NumberFor, Zero};
use sp_runtime::{
	create_runtime_str, generic, impl_opaque_keys,
	transaction_validity::{TransactionPriority, TransactionSource, TransactionValidity},
	ApplyExtrinsicResult, DispatchResult, FixedPointNumber,
};
use sp_std::{cmp::Ordering, convert::TryFrom, prelude::*};
#[cfg(feature = "std")]
use sp_version::NativeVersion;
use sp_version::RuntimeVersion;

// A few exports that help ease life for downstream crates.
pub use frame_support::{
	construct_runtime, debug, match_type, parameter_types,
	traits::{All, KeyOwnerProofSystem, Randomness},
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

pub use constants::{currency::*, time::*, *};
use frame_support::traits::Contains;
use frame_system::{EnsureOneOf, EnsureRoot};
use pallet_traits::PriceProvider;
// Polkadot & XCM imports
use pallet_xcm::XcmPassthrough;
use polkadot_parachain::primitives::Sibling;
use xcm::v0::Xcm;
use xcm::v0::{BodyId, Junction::*, MultiAsset, MultiLocation, MultiLocation::*, NetworkId};
use xcm_builder::{
	AccountId32Aliases, AllowTopLevelPaidExecutionFrom, AllowUnpaidExecutionFrom, CurrencyAdapter, EnsureXcmOrigin,
	FixedWeightBounds, IsConcrete, LocationInverter, NativeAsset, ParentAsSuperuser, ParentIsDefault,
	RelayChainAsNative, SiblingParachainAsNative, SiblingParachainConvertsVia, SignedAccountId32AsNative,
	SignedToAccountId32, SovereignSignedViaLocation, TakeWeightCredit, UsingComponents,
};
use xcm_executor::{Config, XcmExecutor};

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
			// FIXME: should be only aura?
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

// Module accounts of runtime
parameter_types! {
	pub const MntTokenModuleId: PalletId = PalletId(*b"min/mntt");
	pub const LiquidationPoolsModuleId: PalletId = PalletId(*b"min/lqdn");
	pub const DexModuleId: PalletId = PalletId(*b"min/dexs");
	pub const LiquidityPoolsModuleId: PalletId = PalletId(*b"min/lqdy");
}

// Do not change the order of modules. Used for test genesis block.
pub fn get_all_modules_accounts() -> Vec<AccountId> {
	vec![
		MntTokenModuleId::get().into_account(),
		LiquidationPoolsModuleId::get().into_account(),
		DexModuleId::get().into_account(),
		LiquidityPoolsModuleId::get().into_account(),
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

	// FIXME: frame_system version should be changed to
	//    frame-system = { git = "https://github.com/paritytech/substrate", branch = "rococo-v1", default-features = false }
	type OnSetCode = cumulus_pallet_parachain_system::ParachainSetCode<Self>;
}

//FIXME:
// to toml -> cumulus-pallet-parachain-system = { git = "https://github.com/paritytech/cumulus", branch = "rococo-v1", default-features = false }
impl cumulus_pallet_parachain_system::Config for Runtime {
	type Event = Event;
	type OnValidationData = ();
	type SelfParaId = ParachainInfo;
	type DmpMessageHandler = ();
	type OutboundXcmpMessageSource = ();
	type XcmpMessageHandler = ();
	type ReservedXcmpWeight = ();
}

//FIXME:
// to toml -> impl parachain_info::Config for Runtime {}
impl parachain_info::Config for Runtime {}

impl cumulus_pallet_aura_ext::Config for Runtime {}

impl pallet_aura::Config for Runtime {
	type AuthorityId = AuraId;
}

parameter_types! {
	pub const ReservedXcmpWeight: Weight = MAXIMUM_BLOCK_WEIGHT / 4;
	pub const ReservedDmpWeight: Weight = MAXIMUM_BLOCK_WEIGHT / 4;
}

impl cumulus_pallet_parachain_system::Config for Runtime {
	type Event = Event;
	type OnValidationData = ();
	type SelfParaId = parachain_info::Pallet<Runtime>;
	type OutboundXcmpMessageSource = XcmpQueue;
	type DmpMessageHandler = DmpQueue;
	type ReservedDmpWeight = ReservedDmpWeight;
	type XcmpMessageHandler = XcmpQueue;
	type ReservedXcmpWeight = ReservedXcmpWeight;
}

parameter_types! {
	pub const RelayLocation: MultiLocation = X1(Parent);
	pub const RelayNetwork: NetworkId = NetworkId::Polkadot;
	pub RelayOrigin: Origin = cumulus_pallet_xcm::Origin::Relay.into();
	pub Ancestry: MultiLocation = X1(Parachain(ParachainInfo::parachain_id().into()));
}

/// Type for specifying how a `MultiLocation` can be converted into an `AccountId`. This is used
/// when determining ownership of accounts for asset transacting and when attempting to use XCM
/// `Transact` in order to determine the dispatch Origin.
pub type LocationToAccountId = (
	// The parent (Relay-chain) origin converts to the default `AccountId`.
	ParentIsDefault<AccountId>,
	// Sibling parachain origins convert to AccountId via the `ParaId::into`.
	SiblingParachainConvertsVia<Sibling, AccountId>,
	// Straight up local `AccountId32` origins just alias directly to `AccountId`.
	AccountId32Aliases<RelayNetwork, AccountId>,
);

/// Means for transacting assets on this chain.
pub type LocalAssetTransactor = CurrencyAdapter<
	// Use this currency:
	Balances,
	// Use this currency when it is a fungible asset matching the given location or name:
	IsConcrete<RelayLocation>,
	// Do a simple punn to convert an AccountId32 MultiLocation into a native chain account ID:
	LocationToAccountId,
	// Our chain's account ID type (we can't get away without mentioning it explicitly):
	AccountId,
	// We don't track any teleports.
	(),
>;

/// This is the type we use to convert an (incoming) XCM origin into a local `Origin` instance,
/// ready for dispatching a transaction with Xcm's `Transact`. There is an `OriginKind` which can
/// biases the kind of local `Origin` it will become.
pub type XcmOriginToTransactDispatchOrigin = (
	// Sovereign account converter; this attempts to derive an `AccountId` from the origin location
	// using `LocationToAccountId` and then turn that into the usual `Signed` origin. Useful for
	// foreign chains who want to have a local sovereign account on this chain which they control.
	SovereignSignedViaLocation<LocationToAccountId, Origin>,
	// Native converter for Relay-chain (Parent) location; will converts to a `Relay` origin when
	// recognised.
	RelayChainAsNative<RelayOrigin, Origin>,
	// Native converter for sibling Parachains; will convert to a `SiblingPara` origin when
	// recognised.
	SiblingParachainAsNative<cumulus_pallet_xcm::Origin, Origin>,
	// Superuser converter for the Relay-chain (Parent) location. This will allow it to issue a
	// transaction from the Root origin.
	ParentAsSuperuser<Origin>,
	// Native signed account converter; this just converts an `AccountId32` origin into a normal
	// `Origin::Signed` origin of the same 32-byte value.
	SignedAccountId32AsNative<RelayNetwork, Origin>,
	// Xcm origins can be represented natively under the Xcm pallet's Xcm origin.
	XcmPassthrough<Origin>,
);

parameter_types! {
	// One XCM operation is 1_000_000 weight - almost certainly a conservative estimate.
	pub UnitWeightCost: Weight = 1_000_000;
	// One UNIT buys 1 second of weight.
	pub const WeightPrice: (MultiLocation, u128) = (X1(Parent), UNIT);
}

match_type! {
	pub type ParentOrParentsUnitPlurality: impl Contains<MultiLocation> = {
		X1(Parent) | X2(Parent, Plurality { id: BodyId::Unit, .. })
	};
}

pub type Barrier = (
	TakeWeightCredit,
	AllowTopLevelPaidExecutionFrom<All<MultiLocation>>,
	AllowUnpaidExecutionFrom<ParentOrParentsUnitPlurality>,
	// ^^^ Parent & its unit plurality gets free execution
);

pub struct XcmConfig;
impl Config for XcmConfig {
	type Call = Call;
	type XcmSender = XcmRouter;
	// How to withdraw and deposit an asset.
	type AssetTransactor = LocalAssetTransactor;
	type OriginConverter = XcmOriginToTransactDispatchOrigin;
	type IsReserve = NativeAsset;
	type IsTeleporter = NativeAsset; // <- should be enough to allow teleportation of ROC
	type LocationInverter = LocationInverter<Ancestry>;
	type Barrier = Barrier;
	type Weigher = FixedWeightBounds<UnitWeightCost, Call>;
	type Trader = UsingComponents<IdentityFee<Balance>, RelayLocation, AccountId, Balances, ()>;
	type ResponseHandler = (); // Don't handle responses for now.
}

/// No local origins on this chain are allowed to dispatch XCM sends/executions.
pub type LocalOriginToLocation = (SignedToAccountId32<Origin, AccountId, RelayNetwork>,);

/// The means for routing XCM messages which are not for local execution into the right message
/// queues.
pub type XcmRouter = (
	// Two routers - use UMP to communicate with the relay chain:
	cumulus_primitives_utility::ParentAsUmp<ParachainSystem>,
	// ..and XCMP to communicate with the sibling chains.
	XcmpQueue,
);

impl pallet_xcm::Config for Runtime {
	type Event = Event;
	type SendXcmOrigin = EnsureXcmOrigin<Origin, LocalOriginToLocation>;
	type XcmRouter = XcmRouter;
	type ExecuteXcmOrigin = EnsureXcmOrigin<Origin, LocalOriginToLocation>;
	type XcmExecuteFilter = All<(MultiLocation, Xcm<Call>)>;
	type XcmExecutor = XcmExecutor<XcmConfig>;
	type XcmTeleportFilter = All<(MultiLocation, Vec<MultiAsset>)>;
	type XcmReserveTransferFilter = ();
	type Weigher = FixedWeightBounds<UnitWeightCost, Call>;
}

impl cumulus_pallet_xcm::Config for Runtime {
	type Event = Event;
	type XcmExecutor = XcmExecutor<XcmConfig>;
}

impl cumulus_pallet_xcmp_queue::Config for Runtime {
	type Event = Event;
	type XcmExecutor = XcmExecutor<XcmConfig>;
	type ChannelInfo = ParachainSystem;
}

impl cumulus_pallet_dmp_queue::Config for Runtime {
	type Event = Event;
	type XcmExecutor = XcmExecutor<XcmConfig>;
	type ExecuteOverweightOrigin = frame_system::EnsureRoot<AccountId>;
}

// impl pallet_grandpa::Config for Runtime {
// 	type Event = Event;
// 	type Call = Call;
//
// 	type KeyOwnerProofSystem = ();
//
// 	type KeyOwnerProof = <Self::KeyOwnerProofSystem as KeyOwnerProofSystem<(KeyTypeId,
// GrandpaId)>>::Proof;
//
// 	type KeyOwnerIdentification =
// 		<Self::KeyOwnerProofSystem as KeyOwnerProofSystem<(KeyTypeId, GrandpaId)>>::IdentificationTuple;
//
// 	type HandleEquivocation = ();
//
// 	type WeightInfo = ();
// }

parameter_types! {
	pub const MinimumPeriod: u64 = SLOT_DURATION / 2;
}

impl pallet_timestamp::Config for Runtime {
	/// A timestamp: milliseconds since the unix epoch.
	type Moment = Moment;
	type OnTimestampSet = (); // FIXME: () ?
	type MinimumPeriod = MinimumPeriod;
	type WeightInfo = ();
}

parameter_types! {
	pub const ExistentialDeposit: u128 = 500;
	pub const MaxLocks: u32 = 50;
}

impl pallet_balances::Config for Runtime {
	type MaxLocks = MaxLocks;
	/// The type for recording an account's balance.
	type Balance = Balance;
	/// The ubiquitous event type.
	type Event = Event;
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
	type OnChargeTransaction = pallet_transaction_payment::CurrencyAdapter<Balances, ()>;
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
}

parameter_types! {
	pub const WhitelistCouncilMotionDuration: BlockNumber = 7 * DAYS;
	pub const WhitelistCouncilMaxProposals: u32 = 100;
	pub const WhitelistCouncilMaxMembers: u32 = 100;
}

type WhitelistCouncilInstance = pallet_collective::Instance2;
impl pallet_collective::Config<WhitelistCouncilInstance> for Runtime {
	type Origin = Origin;
	type Proposal = Call;
	type Event = Event;
	type MotionDuration = WhitelistCouncilMotionDuration;
	type MaxProposals = WhitelistCouncilMaxProposals;
	type MaxMembers = WhitelistCouncilMaxMembers;
	type DefaultVote = pallet_collective::PrimeDefaultVote;
	type WeightInfo = ();
}

type WhitelistCouncilMembershipInstance = pallet_membership::Instance2;
impl pallet_membership::Config<WhitelistCouncilMembershipInstance> for Runtime {
	type Event = Event;
	type AddOrigin = EnsureRootOrThreeFourthsMinterestCouncil;
	type RemoveOrigin = EnsureRootOrThreeFourthsMinterestCouncil;
	type SwapOrigin = EnsureRootOrThreeFourthsMinterestCouncil;
	type ResetOrigin = EnsureRootOrThreeFourthsMinterestCouncil;
	type PrimeOrigin = EnsureRootOrThreeFourthsMinterestCouncil;
	type MembershipInitialized = WhitelistCouncil;
	type MembershipChanged = WhitelistCouncil;
}

type OperatorMembershipInstanceMinterest = pallet_membership::Instance3;
impl pallet_membership::Config<OperatorMembershipInstanceMinterest> for Runtime {
	type Event = Event;
	type AddOrigin = EnsureRootOrTwoThirdsMinterestCouncil;
	type RemoveOrigin = EnsureRootOrTwoThirdsMinterestCouncil;
	type SwapOrigin = EnsureRootOrTwoThirdsMinterestCouncil;
	type ResetOrigin = EnsureRootOrTwoThirdsMinterestCouncil;
	type PrimeOrigin = EnsureRootOrTwoThirdsMinterestCouncil;
	type MembershipInitialized = MinterestOracle;
	type MembershipChanged = MinterestOracle;
}

impl minterest_protocol::Config for Runtime {
	type Event = Event;
	type Borrowing = LiquidityPools;
	type ManagerLiquidationPools = LiquidationPools;
	type ManagerLiquidityPools = LiquidityPools;
	type MntManager = MntToken;
	type WhitelistMembers = WhitelistCouncilProvider;
	type ProtocolWeightInfo = weights::minterest_protocol::WeightInfo<Runtime>;
	type ControllerAPI = Controller;
}

pub struct WhitelistCouncilProvider;
impl Contains<AccountId> for WhitelistCouncilProvider {
	fn contains(who: &AccountId) -> bool {
		WhitelistCouncil::is_member(who)
	}

	fn sorted_members() -> Vec<AccountId> {
		WhitelistCouncil::members()
	}

	#[cfg(feature = "runtime-benchmarks")]
	fn add(_: &AccountId) {
		todo!()
	}
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
	pub LiquidityPoolAccountId: AccountId = LiquidityPoolsModuleId::get().into_account();
	pub const InitialExchangeRate: Rate = INITIAL_EXCHANGE_RATE;
	pub EnabledUnderlyingAssetsIds: Vec<CurrencyId> = CurrencyId::get_enabled_tokens_in_protocol(UnderlyingAsset);
	pub EnabledWrappedTokensId: Vec<CurrencyId> = CurrencyId::get_enabled_tokens_in_protocol(WrappedToken);
}

impl liquidity_pools::Config for Runtime {
	type MultiCurrency = Currencies;
	type PriceSource = Prices;
	type ModuleId = LiquidityPoolsModuleId;
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
	type LiquidityPoolsManager = LiquidityPools;
	type MaxBorrowCap = MaxBorrowCap;
	type UpdateOrigin = EnsureRootOrHalfMinterestCouncil;
	type ControllerWeightInfo = weights::controller::WeightInfo<Runtime>;
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
}

impl risk_manager::Config for Runtime {
	type Event = Event;
	type UnsignedPriority = RiskManagerPriority;
	type LiquidationPoolsManager = LiquidationPools;
	type LiquidityPoolsManager = LiquidityPools;
	type MntManager = MntToken;
	type RiskManagerUpdateOrigin = EnsureRootOrHalfMinterestCouncil;
	type RiskManagerWeightInfo = weights::risk_manager::WeightInfo<Runtime>;
	type ControllerAPI = Controller;
}

parameter_types! {
	pub MntTokenAccountId: AccountId = MntTokenModuleId::get().into_account();
	pub RefreshSpeedPeriod: BlockNumber = REFRESH_SPEED_PERIOD;
}

impl mnt_token::Config for Runtime {
	type Event = Event;
	type PriceSource = Prices;
	type UpdateOrigin = EnsureRootOrTwoThirdsMinterestCouncil;
	type LiquidityPoolsManager = LiquidityPools;
	type MultiCurrency = Currencies;
	type ControllerAPI = Controller;
	type MntTokenAccountId = MntTokenAccountId;
	type MntTokenWeightInfo = weights::mnt_token::WeightInfo<Runtime>;
	type SpeedRefreshPeriod = RefreshSpeedPeriod;
}

impl<C> frame_system::offchain::SendTransactionTypes<C> for Runtime
where
	Call: From<C>,
{
	type OverarchingCall = Call;
	type Extrinsic = UncheckedExtrinsic;
}

parameter_types! {
	pub LiquidationPoolAccountId: AccountId = LiquidationPoolsModuleId::get().into_account();
}

impl liquidation_pools::Config for Runtime {
	type Event = Event;
	type MultiCurrency = Currencies;
	type UnsignedPriority = LiquidityPoolsPriority;
	type PriceSource = Prices;
	type LiquidationPoolsModuleId = LiquidationPoolsModuleId;
	type LiquidationPoolAccountId = LiquidationPoolAccountId;
	type UpdateOrigin = EnsureRootOrHalfMinterestCouncil;
	type LiquidityPoolsManager = LiquidityPools;
	type Dex = Dex;
	type LiquidationPoolsWeightInfo = weights::liquidation_pools::WeightInfo<Runtime>;
}

parameter_types! {
	pub const MinimumCount: u32 = 1;
	pub const ExpiresIn: Moment = 1000 * 60 * 60; // 60 mins
	pub ZeroAccountId: AccountId = AccountId::from([0u8; 32]);
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
	pub DexAccountId: AccountId = DexModuleId::get().into_account();
}

impl dex::Config for Runtime {
	type Event = Event;
	type MultiCurrency = Currencies;
	type DexModuleId = DexModuleId;
	type DexAccountId = DexAccountId;
}

// Create the runtime by composing the FRAME pallets that were previously configured.
construct_runtime!(
	pub enum Runtime where
		Block = Block,
		NodeBlock = opaque::Block,
		UncheckedExtrinsic = UncheckedExtrinsic
	{
		System: frame_system::{Pallet, Call, Config, Storage, Event<T>},
		RandomnessCollectiveFlip: pallet_randomness_collective_flip::{Pallet, Call, Storage},
		Timestamp: pallet_timestamp::{Pallet, Call, Storage, Inherent},

		Balances: pallet_balances::{Pallet, Call, Storage, Config<T>, Event<T>},
		TransactionPayment: pallet_transaction_payment::{Pallet, Storage},

		// Consensus & Staking
		Aura: pallet_aura::{Pallet, Config<T>},
		AuraExt: cumulus_pallet_aura_ext::{Pallet, Config},
		// FIXME: Parachain
		ParachainSystem: cumulus_pallet_parachain_system::{Pallet, Call, Storage, Inherent, Event<T>},
		ParachainInfo: parachain_info::{Pallet, Storage, Config},

		// XCM helpers
		XcmpQueue: cumulus_pallet_xcmp_queue::{Pallet, Call, Storage, Event<T>},
		PolkadotXcm: pallet_xcm::{Pallet, Call, Event<T>, Origin},
		CumulusXcm: cumulus_pallet_xcm::{Pallet, Call, Event<T>, Origin},
		DmpQueue: cumulus_pallet_dmp_queue::{Pallet, Call, Storage, Event<T>},

		// Governance
		MinterestCouncil: pallet_collective::<Instance1>::{Pallet, Call, Storage, Origin<T>, Event<T>, Config<T>},
		MinterestCouncilMembership: pallet_membership::<Instance1>::{Pallet, Call, Storage, Event<T>, Config<T>},
		WhitelistCouncil: pallet_collective::<Instance2>::{Pallet, Call, Storage, Origin<T>, Event<T>, Config<T>},
		WhitelistCouncilMembership: pallet_membership::<Instance2>::{Pallet, Call, Storage, Event<T>, Config<T>},

		//ORML palletts
		Tokens: orml_tokens::{Pallet, Storage, Call, Event<T>, Config<T>},
		Currencies: orml_currencies::{Pallet, Call, Event<T>},

		// Oracle and Prices
		MinterestOracle: orml_oracle::<Instance1>::{Pallet, Storage, Call, Config<T>, Event<T>},
		Prices: module_prices::{Pallet, Storage, Call, Event<T>, Config<T>},

		// OperatorMembership must be placed after Oracle or else will have race condition on initialization
		OperatorMembershipMinterest: pallet_membership::<Instance3>::{Pallet, Call, Storage, Event<T>, Config<T>},

		// Minterest pallets
		MinterestProtocol: minterest_protocol::{Pallet, Call, Event<T>},
		LiquidityPools: liquidity_pools::{Pallet, Storage, Call, Config<T>},
		Controller: controller::{Pallet, Storage, Call, Event, Config<T>},
		MinterestModel: minterest_model::{Pallet, Storage, Call, Event, Config},
		RiskManager: risk_manager::{Pallet, Storage, Call, Event<T>, Config, ValidateUnsigned},
		LiquidationPools: liquidation_pools::{Pallet, Storage, Call, Event<T>, Config<T>, ValidateUnsigned},
		MntToken: mnt_token::{Pallet, Storage, Call, Event<T>, Config<T>},
		Dex: dex::{Pallet, Storage, Call, Event<T>},
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
	frame_executive::Executive<Runtime, Block, frame_system::ChainContext<Runtime>, Runtime, AllModules>;

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

		fn random_seed() -> <Block as BlockT>::Hash {
			RandomnessCollectiveFlip::random_seed()
		}
	}

	impl sp_transaction_pool::runtime_api::TaggedTransactionQueue<Block> for Runtime {
		fn validate_transaction(
			source: TransactionSource,
			tx: <Block as BlockT>::Extrinsic,
		) -> TransactionValidity {
			Executive::validate_transaction(source, tx)
		}
	}

	impl sp_offchain::OffchainWorkerApi<Block> for Runtime {
		fn offchain_worker(header: &<Block as BlockT>::Header) {
			Executive::offchain_worker(header)
		}
	}

	impl sp_consensus_aura::AuraApi<Block, AuraId> for Runtime {
		fn slot_duration() -> u64 {
			Aura::slot_duration()
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

	// impl fg_primitives::GrandpaApi<Block> for Runtime {
	// 	fn grandpa_authorities() -> GrandpaAuthorityList {
	// 		Grandpa::grandpa_authorities()
	// 	}
	//
	// 	fn submit_report_equivocation_unsigned_extrinsic(
	// 		_equivocation_proof: fg_primitives::EquivocationProof<
	// 			<Block as BlockT>::Hash,
	// 			NumberFor<Block>,
	// 		>,
	// 		_key_owner_proof: fg_primitives::OpaqueKeyOwnershipProof,
	// 	) -> Option<()> {
	// 		None
	// 	}
	//
	// 	fn generate_key_ownership_proof(
	// 		_set_id: fg_primitives::SetId,
	// 		_authority_id: GrandpaId,
	// 	) -> Option<fg_primitives::OpaqueKeyOwnershipProof> {
	// 		// NOTE: this is the only implementation possible since we've
	// 		// defined our key owner proof type as a bottom type (i.e. a type
	// 		// with no values).
	// 		None
	// 	}
	// }

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

	impl controller_rpc_runtime_api::ControllerApi<Block, AccountId> for Runtime {
		fn liquidity_pool_state(pool_id: CurrencyId) -> Option<PoolState> {
			let exchange_rate = Controller::get_liquidity_pool_exchange_rate(pool_id)?;
			let (borrow_rate, supply_rate) = Controller::get_liquidity_pool_borrow_and_supply_rates(pool_id)?;

			Some(PoolState { exchange_rate, borrow_rate, supply_rate })
		}

		fn get_total_supply_and_borrowed_usd_balance(account_id: AccountId) -> Option<UserPoolBalanceData> {
			let (total_supply, total_borrowed) = Controller::get_total_supply_and_borrowed_usd_balance(&account_id).ok()?;

			Some(UserPoolBalanceData {total_supply, total_borrowed})
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

			Some(HypotheticalLiquidityData{ liquidity: res })
		}

		fn is_admin(caller: AccountId) -> Option<bool> {
				Some(MinterestCouncil::is_member(&caller))
		}

		fn get_user_total_collateral(account_id: AccountId) -> Option<BalanceInfo> {
				Some(BalanceInfo{amount: Controller::get_user_total_collateral(account_id).ok()?})
		}

		fn get_user_borrow_per_asset(account_id: AccountId, underlying_asset_id: CurrencyId) -> Option<BalanceInfo> {
				Some(BalanceInfo{amount: Controller::get_user_borrow_per_asset(&account_id, underlying_asset_id).ok()?})
		}
	}

	impl mnt_token_rpc_runtime_api::MntTokenApi<Block, AccountId> for Runtime {
		fn get_unclaimed_mnt_balance(account_id: AccountId) -> Option<MntBalanceInfo> {
				Some(MntBalanceInfo{amount: MntToken::get_unclaimed_mnt_balance(&account_id).ok()?})
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

	impl prices_rpc_runtime_api::PricesApi<Block> for Runtime {
		fn  get_current_price(currency_id: CurrencyId) -> Option<Price> {
			Prices::get_underlying_price(currency_id)
		}

		fn  get_all_locked_prices() -> Vec<(CurrencyId, Option<Price>)> {
			CurrencyId::get_enabled_tokens_in_protocol(UnderlyingAsset)
				.into_iter()
				.map(|currency_id| (currency_id, Prices::locked_price(currency_id)))
				.collect()
		}

		fn get_all_freshest_prices() -> Vec<(CurrencyId, Option<Price>)> {
			Prices::get_all_freshest_prices()
		}
	}

	#[cfg(feature = "runtime-benchmarks")]
	impl frame_benchmarking::Benchmark<Block> for Runtime {
		fn dispatch_benchmark(
			config: frame_benchmarking::BenchmarkConfig
		) -> Result<Vec<frame_benchmarking::BenchmarkBatch>, sp_runtime::RuntimeString> {
			use frame_benchmarking::{Benchmarking, BenchmarkBatch, TrackedStorageKey};
			use orml_benchmarking::add_benchmark;

			let whitelist: Vec<TrackedStorageKey> = vec![
				// Block Number
				hex_literal::hex!("26aa394eea5630e07c48ae0c9558cef702a5c1b19ab7a04f536c519aca4983ac").to_vec().into(),
				// Total Issuance
				hex_literal::hex!("c2261276cc9d1f8598ea4b6a74b15c2f57c875e4cff74148e4628f264b974c80").to_vec().into(),
				// Execution Phase
				hex_literal::hex!("26aa394eea5630e07c48ae0c9558cef7ff553b5a9862a516939d82b3d3d8661a").to_vec().into(),
				// Event Count
				hex_literal::hex!("26aa394eea5630e07c48ae0c9558cef70a98fdbe9ce6c55837576c60c7af3850").to_vec().into(),
				// System Events
				hex_literal::hex!("26aa394eea5630e07c48ae0c9558cef780d41e5e16056765bc8461851072c9d7").to_vec().into(),
			];

			let mut batches = Vec::<BenchmarkBatch>::new();
			let params = (&config, &whitelist);

			add_benchmark!(params, batches, controller, benchmarking::controller);
			add_benchmark!(params, batches, minterest_model, benchmarking::minterest_model);
			add_benchmark!(params, batches, module_prices, benchmarking::prices);
			add_benchmark!(params, batches, risk_manager, benchmarking::risk_manager);
			add_benchmark!(params, batches, liquidation_pools, benchmarking::liquidation_pools);
			add_benchmark!(params, batches, minterest_protocol, benchmarking::minterest_protocol);
			add_benchmark!(params, batches, mnt_token, benchmarking::mnt_token);

			if batches.is_empty() { return Err("Benchmark not found for this pallet.".into()) }
			Ok(batches)
		}
	}
}

cumulus_pallet_parachain_system::register_validate_block!(
	Runtime,
	cumulus_pallet_aura_ext::BlockExecutor::<Runtime, Executive>,
);
