#![cfg_attr(not(feature = "std"), no_std)]
// `construct_runtime!` does a lot of recursion and requires us to increase the limit to 256.
#![recursion_limit = "256"]
// The `large_enum_variant` warning originates from `construct_runtime` macro.
#![allow(clippy::large_enum_variant)]
#![allow(clippy::from_over_into)]

// Make the WASM binary available.
#[cfg(feature = "std")]
include!(concat!(env!("OUT_DIR"), "/wasm_binary.rs"));

mod constants;
#[cfg(test)]
mod tests;

pub use controller_rpc_runtime_api::PoolState;
pub use controller_rpc_runtime_api::UserPoolBalanceData;
use orml_currencies::BasicCurrencyAdapter;
use orml_traits::{create_median_value_data_provider, parameter_type_with_key, DataFeeder, DataProviderExtended};
use pallet_grandpa::fg_primitives;
use pallet_grandpa::{AuthorityId as GrandpaId, AuthorityList as GrandpaAuthorityList};
use pallet_transaction_payment::CurrencyAdapter;
use sp_api::impl_runtime_apis;
use sp_consensus_aura::sr25519::AuthorityId as AuraId;
use sp_core::{
	crypto::KeyTypeId,
	u32_trait::{_1, _2, _3, _4},
	OpaqueMetadata,
};
use sp_runtime::traits::{AccountIdConversion, AccountIdLookup, BlakeTwo256, Block as BlockT, NumberFor, Zero};
use sp_runtime::{
	create_runtime_str, generic, impl_opaque_keys,
	transaction_validity::{TransactionPriority, TransactionSource, TransactionValidity},
	ApplyExtrinsicResult, DispatchResult, ModuleId,
};
use sp_std::prelude::*;
#[cfg(feature = "std")]
use sp_version::NativeVersion;
use sp_version::RuntimeVersion;

pub use minterest_primitives::{
	AccountId, AccountIndex, Amount, Balance, BlockNumber, CurrencyId, CurrencyPair, DataProviderId, DigestItem, Hash,
	Index, Moment, Price, Rate, Signature,
};

// A few exports that help ease life for downstream crates.
pub use frame_support::{
	construct_runtime, debug, parameter_types,
	traits::{KeyOwnerProofSystem, Randomness},
	weights::{
		constants::{BlockExecutionWeight, ExtrinsicBaseWeight, RocksDbWeight, WEIGHT_PER_SECOND},
		DispatchClass, IdentityFee, Weight,
	},
	IterableStorageDoubleMap, StorageValue,
};
pub use pallet_balances::Call as BalancesCall;
pub use pallet_timestamp::Call as TimestampCall;
#[cfg(any(feature = "std", test))]
pub use sp_runtime::BuildStorage;
pub use sp_runtime::{Perbill, Permill};

pub use constants::{currency::*, time::*, *};
use frame_support::traits::Contains;
use frame_system::{EnsureOneOf, EnsureRoot};

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
			pub grandpa: Grandpa,
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
	pub const LiquidityPoolsModuleId: ModuleId = ModuleId(*b"min/lqdy");
	pub const LiquidationPoolsModuleId: ModuleId = ModuleId(*b"min/lqdn");
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
}

impl pallet_aura::Config for Runtime {
	type AuthorityId = AuraId;
}

impl pallet_grandpa::Config for Runtime {
	type Event = Event;
	type Call = Call;

	type KeyOwnerProofSystem = ();

	type KeyOwnerProof = <Self::KeyOwnerProofSystem as KeyOwnerProofSystem<(KeyTypeId, GrandpaId)>>::Proof;

	type KeyOwnerIdentification =
		<Self::KeyOwnerProofSystem as KeyOwnerProofSystem<(KeyTypeId, GrandpaId)>>::IdentificationTuple;

	type HandleEquivocation = ();

	type WeightInfo = ();
}

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
	pub const TransactionByteFee: Balance = 1;
}

impl pallet_transaction_payment::Config for Runtime {
	type OnChargeTransaction = CurrencyAdapter<Balances, ()>;
	type TransactionByteFee = TransactionByteFee;
	type WeightToFee = IdentityFee<Balance>;
	type FeeMultiplierUpdate = ();
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

impl m_tokens::Config for Runtime {
	type Event = Event;
	type MultiCurrency = Currencies;
}

impl minterest_protocol::Config for Runtime {
	type Event = Event;
	type Borrowing = LiquidityPools;
	type ManagerLiquidityPools = LiquidityPools;
	type WhitelistMembers = WhitelistCouncilProvider;
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

pub struct MinterestCouncilProvider;
impl Contains<AccountId> for MinterestCouncilProvider {
	fn contains(who: &AccountId) -> bool {
		MinterestCouncil::is_member(who)
	}

	fn sorted_members() -> Vec<AccountId> {
		MinterestCouncil::members()
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
	pub const GetMinterestCurrencyId: CurrencyId = CurrencyId::MNT;
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
	pub EnabledCurrencyPair: Vec<CurrencyPair> = vec![
		CurrencyPair::new(CurrencyId::DOT, CurrencyId::MDOT),
		CurrencyPair::new(CurrencyId::KSM, CurrencyId::MKSM),
		CurrencyPair::new(CurrencyId::BTC, CurrencyId::MBTC),
		CurrencyPair::new(CurrencyId::ETH, CurrencyId::METH),
	];
	pub EnabledUnderlyingAssetId: Vec<CurrencyId> = EnabledCurrencyPair::get().iter()
			.map(|currency_pair| currency_pair.underlying_id)
			.collect();
	pub EnabledWrappedTokensId: Vec<CurrencyId> = EnabledCurrencyPair::get().iter()
			.map(|currency_pair| currency_pair.wrapped_id)
			.collect();
}

impl liquidity_pools::Config for Runtime {
	type MultiCurrency = Currencies;
	type PriceSource = Prices;
	type ModuleId = LiquidityPoolsModuleId;
	type LiquidityPoolAccountId = LiquidityPoolAccountId;
	type InitialExchangeRate = InitialExchangeRate;
	type EnabledCurrencyPair = EnabledCurrencyPair;
	type EnabledUnderlyingAssetId = EnabledUnderlyingAssetId;
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
}

impl module_prices::Config for Runtime {
	type Event = Event;
	type Source = AggregatedDataProvider;
	type LockOrigin = EnsureRootOrTwoThirdsMinterestCouncil;
	type EnabledUnderlyingAssetId = EnabledUnderlyingAssetId;
}

parameter_types! {
	pub const BlocksPerYear: u128 = BLOCKS_PER_YEAR;
}

impl minterest_model::Config for Runtime {
	type Event = Event;
	type BlocksPerYear = BlocksPerYear;
	type ModelUpdateOrigin = EnsureRootOrHalfMinterestCouncil;
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
	type RiskManagerUpdateOrigin = EnsureRootOrHalfMinterestCouncil;
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
	type UnsignedPriority = LiquidityPoolsPriority;
	type LiquidationPoolsModuleId = LiquidationPoolsModuleId;
	type LiquidationPoolAccountId = LiquidationPoolAccountId;
	type UpdateOrigin = EnsureRootOrHalfMinterestCouncil;
	type LiquidityPoolsManager = LiquidityPools;
	type DEX = Dex;
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

impl dex::Config for Runtime {
	type Event = Event;
}

// Create the runtime by composing the FRAME pallets that were previously configured.
construct_runtime!(
	pub enum Runtime where
		Block = Block,
		NodeBlock = opaque::Block,
		UncheckedExtrinsic = UncheckedExtrinsic
	{
		System: frame_system::{Module, Call, Config, Storage, Event<T>},
		RandomnessCollectiveFlip: pallet_randomness_collective_flip::{Module, Call, Storage},
		Timestamp: pallet_timestamp::{Module, Call, Storage, Inherent},

		Balances: pallet_balances::{Module, Call, Storage, Config<T>, Event<T>},
		TransactionPayment: pallet_transaction_payment::{Module, Storage},

		// Consensus & Staking
		Aura: pallet_aura::{Module, Config<T>},
		Grandpa: pallet_grandpa::{Module, Call, Storage, Config, Event},

		// Governance
		MinterestCouncil: pallet_collective::<Instance1>::{Module, Call, Storage, Origin<T>, Event<T>, Config<T>},
		MinterestCouncilMembership: pallet_membership::<Instance1>::{Module, Call, Storage, Event<T>, Config<T>},
		WhitelistCouncil: pallet_collective::<Instance2>::{Module, Call, Storage, Origin<T>, Event<T>, Config<T>},
		WhitelistCouncilMembership: pallet_membership::<Instance2>::{Module, Call, Storage, Event<T>, Config<T>},

		//ORML palletts
		Tokens: orml_tokens::{Module, Storage, Call, Event<T>, Config<T>},
		Currencies: orml_currencies::{Module, Call, Event<T>},

		// Oracle and Prices
		MinterestOracle: orml_oracle::<Instance1>::{Module, Storage, Call, Config<T>, Event<T>},
		Prices: module_prices::{Module, Storage, Call, Event<T>},

		// OperatorMembership must be placed after Oracle or else will have race condition on initialization
		OperatorMembershipMinterest: pallet_membership::<Instance3>::{Module, Call, Storage, Event<T>, Config<T>},

		// Minterest pallets
		MTokens: m_tokens::{Module, Storage, Call, Event<T>},
		MinterestProtocol: minterest_protocol::{Module, Call, Event<T>},
		LiquidityPools: liquidity_pools::{Module, Storage, Call, Config<T>},
		Controller: controller::{Module, Storage, Call, Event, Config<T>},
		MinterestModel: minterest_model::{Module, Storage, Call, Event, Config},
		RiskManager: risk_manager::{Module, Storage, Call, Event<T>, Config, ValidateUnsigned},
		LiquidationPools: liquidation_pools::{Module, Storage, Call, Event<T>, Config<T>, ValidateUnsigned},
		Dex: dex::{Module, Storage, Call, Event<T>},

		// Dev
		Sudo: pallet_sudo::{Module, Call, Config<T>, Storage, Event<T>},
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

	impl fg_primitives::GrandpaApi<Block> for Runtime {
		fn grandpa_authorities() -> GrandpaAuthorityList {
			Grandpa::grandpa_authorities()
		}

		fn submit_report_equivocation_unsigned_extrinsic(
			_equivocation_proof: fg_primitives::EquivocationProof<
				<Block as BlockT>::Hash,
				NumberFor<Block>,
			>,
			_key_owner_proof: fg_primitives::OpaqueKeyOwnershipProof,
		) -> Option<()> {
			None
		}

		fn generate_key_ownership_proof(
			_set_id: fg_primitives::SetId,
			_authority_id: GrandpaId,
		) -> Option<fg_primitives::OpaqueKeyOwnershipProof> {
			// NOTE: this is the only implementation possible since we've
			// defined our key owner proof type as a bottom type (i.e. a type
			// with no values).
			None
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

		fn is_admin(caller: AccountId) -> Option<bool> {
				Some(MinterestCouncilProvider::contains(&caller))
		}
	}

	impl orml_oracle_rpc_runtime_api::OracleApi<
		Block,
		DataProviderId,
		CurrencyId,
		TimeStampedPrice,
	> for Runtime {
		fn get_value(provider_id: DataProviderId ,key: CurrencyId) -> Option<TimeStampedPrice> {
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

	#[cfg(feature = "runtime-benchmarks")]
	impl frame_benchmarking::Benchmark<Block> for Runtime {
		fn dispatch_benchmark(
			config: frame_benchmarking::BenchmarkConfig
		) -> Result<Vec<frame_benchmarking::BenchmarkBatch>, sp_runtime::RuntimeString> {
			use frame_benchmarking::{Benchmarking, BenchmarkBatch, add_benchmark, TrackedStorageKey};

			use frame_system_benchmarking::Module as SystemBench;
			impl frame_system_benchmarking::Config for Runtime {}

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

			add_benchmark!(params, batches, frame_system, SystemBench::<Runtime>);
			add_benchmark!(params, batches, pallet_balances, Balances);
			add_benchmark!(params, batches, pallet_timestamp, Timestamp);

			if batches.is_empty() { return Err("Benchmark not found for this pallet.".into()) }
			Ok(batches)
		}
	}
}
