use controller::{ControllerData, PauseKeeper};
use cumulus_primitives_core::ParaId;
use hex_literal::hex;
use liquidation_pools::LiquidationPoolData;
use liquidity_pools::Pool;
use minterest_model::MinterestModelData;
use node_minterest_runtime::{
	get_all_modules_accounts, AccountId, AuraConfig, Balance, BalancesConfig, ControllerConfig, GenesisConfig,
	LiquidationPoolsConfig, LiquidityPoolsConfig, MinterestCouncilMembershipConfig, MinterestModelConfig,
	MntTokenConfig, OperatorMembershipMinterestConfig, ParachainInfoConfig, PricesConfig, Rate, RiskManagerConfig,
	Signature, SudoConfig, SystemConfig, TokensConfig, WhitelistCouncilMembershipConfig, BTC, DOLLARS, DOT, ETH, KSM,
	PROTOCOL_INTEREST_TRANSFER_THRESHOLD, WASM_BINARY,
};
use risk_manager::RiskManagerData;
use sc_chain_spec::{ChainSpecExtension, ChainSpecGroup};
use sc_service::ChainType;
use sc_telemetry::TelemetryEndpoints;
use serde::{Deserialize, Serialize};
use serde_json::map::Map;
use sp_consensus_aura::sr25519::AuthorityId as AuraId;
use sp_core::{sr25519, Pair, Public};
use sp_finality_grandpa::AuthorityId as GrandpaId;
use sp_runtime::traits::One;
use sp_runtime::{
	traits::{IdentifyAccount, Verify, Zero},
	FixedPointNumber,
};

// The URL for the telemetry server.
// const STAGING_TELEMETRY_URL: &str = "wss://telemetry.polkadot.io/submit/";

const INITIAL_BALANCE: u128 = 100_000 * DOLLARS;
const INITIAL_TREASURY: u128 = 5_000_000 * DOLLARS;

// The URL for the telemetry server.
const STAGING_TELEMETRY_URL: &str = "wss://telemetry.polkadot.io/submit/";

/// Specialized `ChainSpec`. This is a specialization of the general Substrate ChainSpec type.
pub type ChainSpec = sc_service::GenericChainSpec<GenesisConfig, Extensions>;

/// Node `ChainSpec` extensions.
///
/// Additional parameters for some Substrate core modules,
/// customizable from the chain spec.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ChainSpecGroup, ChainSpecExtension)]
#[serde(rename_all = "camelCase")]
pub struct Extensions {
	/// The relay chain of the Parachain.
	pub relay_chain: String,
	/// The id of the Parachain.
	pub para_id: u32,
}

impl Extensions {
	/// Try to get the extension from the given `ChainSpec`.
	pub fn try_get(chain_spec: &dyn sc_service::ChainSpec) -> Option<&Self> {
		sc_chain_spec::get_extension(chain_spec.extensions())
	}
}

/// Generate a crypto pair from seed.
pub fn get_from_seed<TPublic: Public>(seed: &str) -> <TPublic::Pair as Pair>::Public {
	TPublic::Pair::from_string(&format!("//{}", seed), None)
		.expect("static values are valid; qed")
		.public()
}

type AccountPublic = <Signature as Verify>::Signer;

/// Generate an account ID from seed.
pub fn get_account_id_from_seed<TPublic: Public>(seed: &str) -> AccountId
where
	AccountPublic: From<<TPublic::Pair as Pair>::Public>,
{
	AccountPublic::from(get_from_seed::<TPublic>(seed)).into_account()
}

// /// Generate an Aura authority key.
// pub fn authority_keys_from_seed(seed: &str) -> (AuraId, GrandpaId) {
// 	(get_from_seed::<AuraId>(seed), get_from_seed::<GrandpaId>(seed))
// }

pub fn development_config(id: ParaId) -> ChainSpec {
	let mut properties = Map::new();
	properties.insert("tokenDecimals".into(), 18.into());

	let wasm_binary = WASM_BINARY.expect("WASM binary was not build, please build it!");

	ChainSpec::from_genesis(
		// Name
		"Development",
		// ID
		"dev",
		ChainType::Local,
		move || {
			testnet_genesis(
				wasm_binary,
				// Initial PoA authorities
				// vec![authority_keys_from_seed("Alice")],
				// Sudo account
				get_account_id_from_seed::<sr25519::Public>("Alice"),
				vec![get_from_seed::<AuraId>("Alice"), get_from_seed::<AuraId>("Bob")],
				// Pre-funded accounts
				vec![
					get_account_id_from_seed::<sr25519::Public>("Alice"),
					get_account_id_from_seed::<sr25519::Public>("Bob"),
					get_account_id_from_seed::<sr25519::Public>("Alice//stash"),
					get_account_id_from_seed::<sr25519::Public>("Bob//stash"),
					// Eugene
					hex!["680ee3a95d0b19619d9483fdee34f5d0016fbadd7145d016464f6bfbb993b46b"].into(),
				],
				id,
			)
		},
		// Bootnodes
		vec![],
		// Telemetry
		None,
		// Protocol ID
		None,
		// Properties
		// Some(properties),
		// Extensions
		None,
		Extensions {
			relay_chain: "rococo-local".into(),
			para_id: id.into(),
		},
	)
}

pub fn local_testnet_config(id: ParaId) -> ChainSpec {
	let mut properties = Map::new();
	properties.insert("tokenDecimals".into(), 18.into());

	let wasm_binary = WASM_BINARY.expect("WASM binary was not build, please build it!");

	ChainSpec::from_genesis(
		// Name
		"Local Testnet",
		// ID
		"local_testnet",
		ChainType::Local,
		move || {
			testnet_genesis(
				wasm_binary,
				// Initial PoA authorities
				get_account_id_from_seed::<sr25519::Public>("Alice"),
				// Sudo account
				vec![get_from_seed::<AuraId>("Alice"), get_from_seed::<AuraId>("Bob")],
				// Pre-funded accounts
				vec![
					get_account_id_from_seed::<sr25519::Public>("Alice"),
					get_account_id_from_seed::<sr25519::Public>("Bob"),
					get_account_id_from_seed::<sr25519::Public>("Charlie"),
					get_account_id_from_seed::<sr25519::Public>("Dave"),
					get_account_id_from_seed::<sr25519::Public>("Eve"),
					get_account_id_from_seed::<sr25519::Public>("Ferdie"),
					get_account_id_from_seed::<sr25519::Public>("Alice//stash"),
					get_account_id_from_seed::<sr25519::Public>("Bob//stash"),
					get_account_id_from_seed::<sr25519::Public>("Charlie//stash"),
					get_account_id_from_seed::<sr25519::Public>("Dave//stash"),
					get_account_id_from_seed::<sr25519::Public>("Eve//stash"),
					get_account_id_from_seed::<sr25519::Public>("Ferdie//stash"),
				],
				id,
			)
		},
		// Bootnodes
		vec![],
		// Telemetry
		None,
		// Protocol ID
		None,
		// Properties
		// Some(properties),
		// Extensions
		None,
		Extensions {
			relay_chain: "rococo-local".into(),
			para_id: id.into(),
		},
	)
}

pub fn minterest_turbo_testnet_config(id: ParaId) -> ChainSpec {
	let mut properties = Map::new();
	properties.insert("tokenDecimals".into(), 18.into());

	let wasm_binary = WASM_BINARY.expect("WASM binary was not build, please build it!");

	ChainSpec::from_genesis(
		"Minterest Turbo",
		"turbo-latest",
		ChainType::Live,
		move || {
			testnet_genesis(
				wasm_binary,
				// Initial PoA authorities
				hex!["680ee3a95d0b19619d9483fdee34f5d0016fbadd7145d016464f6bfbb993b46b"].into(),
				// Sudo account
				// 5ER9G3d2V4EEq8VjEbjkGbMdgprvtCntTYu9itCRJNHTkWYX
				vec![get_from_seed::<AuraId>("Alice"), get_from_seed::<AuraId>("Bob")],
				// Pre-funded accounts
				vec![
					hex!["680ee3a95d0b19619d9483fdee34f5d0016fbadd7145d016464f6bfbb993b46b"].into(),
					get_account_id_from_seed::<sr25519::Public>("Alice"),
					get_account_id_from_seed::<sr25519::Public>("Bob"),
					get_account_id_from_seed::<sr25519::Public>("Alice//stash"),
					get_account_id_from_seed::<sr25519::Public>("Bob//stash"),
				],
				id,
			)
		},
		// Bootnodes
		vec![],
		// Telemetry
		Some(
			TelemetryEndpoints::new(vec![(STAGING_TELEMETRY_URL.to_string(), 0)])
				.expect("Staging telemetry url is valid; qed"),
		),
		// Protocol ID
		Some("turbo-latest"),
		// Properties
		// Some(properties),
		// Extensions
		None,
		Extensions {
			relay_chain: "rococo-turbo".into(),
			para_id: id.into(),
		},
	)
}

/// Configure initial storage state for FRAME pallets.
fn testnet_genesis(
	wasm_binary: &[u8],
	root_key: AccountId,
	initial_authorities: Vec<(AuraId)>,
	endowed_accounts: Vec<AccountId>,
	id: ParaId,
) -> GenesisConfig {
	GenesisConfig {
		frame_system: SystemConfig {
			// Add Wasm runtime to storage.
			code: wasm_binary.to_vec(),
			changes_trie_config: Default::default(),
		},
		parachain_info: ParachainInfoConfig { parachain_id: id },
		pallet_balances: BalancesConfig {
			// Configure endowed accounts with initial balance of INITIAL_BALANCE.
			balances: endowed_accounts
				.iter()
				.cloned()
				.map(|k| (k, INITIAL_BALANCE))
				.chain(
					get_all_modules_accounts()
						.get(0) // mnt-token module
						.map(|x| (x.clone(), INITIAL_TREASURY)),
				)
				.collect(),
		},
		pallet_aura: AuraConfig {
			authorities: initial_authorities,
		},
		pallet_sudo: SudoConfig {
			// Assign network admin rights.
			key: root_key.clone(),
		},
		cumulus_pallet_aura_ext: Default::default(),
		orml_tokens: TokensConfig {
			endowed_accounts: endowed_accounts
				.iter()
				.chain(get_all_modules_accounts()[1..3].iter()) // liquidation_pools + DEXes
				.flat_map(|x| {
					vec![
						(x.clone(), DOT, INITIAL_BALANCE),
						(x.clone(), ETH, INITIAL_BALANCE),
						(x.clone(), KSM, INITIAL_BALANCE),
						(x.clone(), BTC, INITIAL_BALANCE),
					]
				})
				.collect(),
		},
		liquidity_pools: LiquidityPoolsConfig {
			pools: vec![
				(
					ETH,
					Pool {
						total_borrowed: Balance::zero(),
						borrow_index: Rate::one(),
						total_protocol_interest: Balance::zero(),
					},
				),
				(
					DOT,
					Pool {
						total_borrowed: Balance::zero(),
						borrow_index: Rate::one(),
						total_protocol_interest: Balance::zero(),
					},
				),
				(
					KSM,
					Pool {
						total_borrowed: Balance::zero(),
						borrow_index: Rate::one(),
						total_protocol_interest: Balance::zero(),
					},
				),
				(
					BTC,
					Pool {
						total_borrowed: Balance::zero(),
						borrow_index: Rate::one(),
						total_protocol_interest: Balance::zero(),
					},
				),
			],
			pool_user_data: vec![],
		},
		controller: ControllerConfig {
			controller_dates: vec![
				(
					ETH,
					ControllerData {
						last_interest_accrued_block: 0,
						protocol_interest_factor: Rate::saturating_from_rational(1, 10),
						max_borrow_rate: Rate::saturating_from_rational(5, 1000),
						collateral_factor: Rate::saturating_from_rational(9, 10), // 90%
						borrow_cap: None,
						protocol_interest_threshold: PROTOCOL_INTEREST_TRANSFER_THRESHOLD,
					},
				),
				(
					DOT,
					ControllerData {
						last_interest_accrued_block: 0,
						protocol_interest_factor: Rate::saturating_from_rational(1, 10),
						max_borrow_rate: Rate::saturating_from_rational(5, 1000),
						collateral_factor: Rate::saturating_from_rational(9, 10), // 90%
						borrow_cap: None,
						protocol_interest_threshold: PROTOCOL_INTEREST_TRANSFER_THRESHOLD,
					},
				),
				(
					KSM,
					ControllerData {
						last_interest_accrued_block: 0,
						protocol_interest_factor: Rate::saturating_from_rational(1, 10),
						max_borrow_rate: Rate::saturating_from_rational(5, 1000),
						collateral_factor: Rate::saturating_from_rational(9, 10), // 90%
						borrow_cap: None,
						protocol_interest_threshold: PROTOCOL_INTEREST_TRANSFER_THRESHOLD,
					},
				),
				(
					BTC,
					ControllerData {
						last_interest_accrued_block: 0,
						protocol_interest_factor: Rate::saturating_from_rational(1, 10),
						max_borrow_rate: Rate::saturating_from_rational(5, 1000),
						collateral_factor: Rate::saturating_from_rational(9, 10), // 90%
						borrow_cap: None,
						protocol_interest_threshold: PROTOCOL_INTEREST_TRANSFER_THRESHOLD,
					},
				),
			],
			pause_keepers: vec![
				(
					ETH,
					PauseKeeper {
						deposit_paused: false,
						redeem_paused: false,
						borrow_paused: false,
						repay_paused: false,
						transfer_paused: false,
					},
				),
				(
					DOT,
					PauseKeeper {
						deposit_paused: false,
						redeem_paused: false,
						borrow_paused: false,
						repay_paused: false,
						transfer_paused: false,
					},
				),
				(
					KSM,
					PauseKeeper {
						deposit_paused: false,
						redeem_paused: false,
						borrow_paused: false,
						repay_paused: false,
						transfer_paused: false,
					},
				),
				(
					BTC,
					PauseKeeper {
						deposit_paused: false,
						redeem_paused: false,
						borrow_paused: false,
						repay_paused: false,
						transfer_paused: false,
					},
				),
			],
			whitelist_mode: false,
		},
		minterest_model: MinterestModelConfig {
			minterest_model_params: vec![
				(
					ETH,
					MinterestModelData {
						kink: Rate::saturating_from_rational(8, 10), // 0.8 = 80 %
						base_rate_per_block: Rate::zero(),
						multiplier_per_block: Rate::saturating_from_rational(9, 1_000_000_000), // 0.047304 PerYear
						jump_multiplier_per_block: Rate::saturating_from_rational(207, 1_000_000_000), // 1.09 PerYear
					},
				),
				(
					DOT,
					MinterestModelData {
						kink: Rate::saturating_from_rational(8, 10), // 0.8 = 80 %
						base_rate_per_block: Rate::zero(),
						multiplier_per_block: Rate::saturating_from_rational(9, 1_000_000_000), // 0.047304 PerYear
						jump_multiplier_per_block: Rate::saturating_from_rational(207, 1_000_000_000), // 1.09 PerYear
					},
				),
				(
					KSM,
					MinterestModelData {
						kink: Rate::saturating_from_rational(8, 10), // 0.8 = 80 %
						base_rate_per_block: Rate::zero(),
						multiplier_per_block: Rate::saturating_from_rational(9, 1_000_000_000), // 0.047304 PerYear
						jump_multiplier_per_block: Rate::saturating_from_rational(207, 1_000_000_000), // 1.09 PerYear
					},
				),
				(
					BTC,
					MinterestModelData {
						kink: Rate::saturating_from_rational(8, 10), // 0.8 = 80 %
						base_rate_per_block: Rate::zero(),
						multiplier_per_block: Rate::saturating_from_rational(9, 1_000_000_000), // 0.047304 PerYear
						jump_multiplier_per_block: Rate::saturating_from_rational(207, 1_000_000_000), // 1.09 PerYear
					},
				),
			],
		},
		risk_manager: RiskManagerConfig {
			risk_manager_dates: vec![
				(
					ETH,
					RiskManagerData {
						max_attempts: 2,
						min_partial_liquidation_sum: 200_000 * DOLLARS, // In USD. FIXME: temporary value.
						threshold: Rate::saturating_from_rational(103, 100), // 3%
						liquidation_fee: Rate::saturating_from_rational(105, 100), // 5%
					},
				),
				(
					DOT,
					RiskManagerData {
						max_attempts: 2,
						min_partial_liquidation_sum: 100_000 * DOLLARS, // In USD. FIXME: temporary value.
						threshold: Rate::saturating_from_rational(103, 100), // 3%
						liquidation_fee: Rate::saturating_from_rational(105, 100), // 5%
					},
				),
				(
					KSM,
					RiskManagerData {
						max_attempts: 2,
						min_partial_liquidation_sum: 200_000 * DOLLARS, // In USD. FIXME: temporary value.
						threshold: Rate::saturating_from_rational(103, 100), // 3%
						liquidation_fee: Rate::saturating_from_rational(105, 100), // 5%
					},
				),
				(
					BTC,
					RiskManagerData {
						max_attempts: 2,
						min_partial_liquidation_sum: 200_000 * DOLLARS, // In USD. FIXME: temporary value.
						threshold: Rate::saturating_from_rational(103, 100), // 3%
						liquidation_fee: Rate::saturating_from_rational(105, 100), // 5%
					},
				),
			],
		},
		liquidation_pools: LiquidationPoolsConfig {
			balancing_period: 10, // FIXME: temporary value.
			liquidation_pools: vec![
				(
					DOT,
					LiquidationPoolData {
						deviation_threshold: Rate::saturating_from_rational(1, 10),
						balance_ratio: Rate::saturating_from_rational(2, 10),
						max_ideal_balance: None,
					},
				),
				(
					ETH,
					LiquidationPoolData {
						deviation_threshold: Rate::saturating_from_rational(1, 10),
						balance_ratio: Rate::saturating_from_rational(2, 10),
						max_ideal_balance: None,
					},
				),
				(
					BTC,
					LiquidationPoolData {
						deviation_threshold: Rate::saturating_from_rational(1, 10),
						balance_ratio: Rate::saturating_from_rational(2, 10),
						max_ideal_balance: None,
					},
				),
				(
					KSM,
					LiquidationPoolData {
						deviation_threshold: Rate::saturating_from_rational(1, 10),
						balance_ratio: Rate::saturating_from_rational(2, 10),
						max_ideal_balance: None,
					},
				),
			],
		},
		module_prices: PricesConfig {
			locked_price: vec![
				(DOT, Rate::saturating_from_integer(2)),
				(KSM, Rate::saturating_from_integer(2)),
				(ETH, Rate::saturating_from_integer(2)),
				(BTC, Rate::saturating_from_integer(2)),
			],
			_phantom: Default::default(),
		},
		pallet_collective_Instance1: Default::default(),
		pallet_membership_Instance1: MinterestCouncilMembershipConfig {
			members: vec![root_key.clone()],
			phantom: Default::default(),
		},
		pallet_collective_Instance2: Default::default(),
		pallet_membership_Instance2: WhitelistCouncilMembershipConfig {
			members: vec![root_key],
			phantom: Default::default(),
		},
		pallet_membership_Instance3: OperatorMembershipMinterestConfig {
			members: endowed_accounts.clone(),
			phantom: Default::default(),
		},
		mnt_token: MntTokenConfig {
			mnt_rate: 10 * DOLLARS,
			mnt_claim_threshold: 0, // disable by default
			minted_pools: vec![DOT, ETH, KSM, BTC],
			_phantom: Default::default(),
		},
	}
}
