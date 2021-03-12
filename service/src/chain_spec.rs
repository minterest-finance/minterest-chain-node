use controller::{ControllerData, PauseKeeper};
use hex_literal::hex;
use liquidation_pools::{LiquidationPool, LiquidationPoolCommonData};
use liquidity_pools::Pool;
use minterest_model::MinterestModelData;
use node_minterest_runtime::{
	AccountId, AuraConfig, Balance, BalancesConfig, ControllerConfig, CurrencyId, GenesisConfig, GrandpaConfig,
	LiquidationPoolsConfig, LiquidityPoolsConfig, MinterestCouncilMembershipConfig, MinterestModelConfig,
	MinterestOracleConfig, OperatorMembershipMinterestConfig, RiskManagerConfig, Signature, SudoConfig, SystemConfig,
	TokensConfig, WhitelistCouncilMembershipConfig, DOLLARS, WASM_BINARY,
};
use risk_manager::RiskManagerData;
use sc_service::ChainType;
use sc_telemetry::TelemetryEndpoints;
use serde_json::map::Map;
use sp_consensus_aura::sr25519::AuthorityId as AuraId;
use sp_core::{sr25519, Pair, Public};
use sp_finality_grandpa::AuthorityId as GrandpaId;
use sp_runtime::{
	traits::{IdentifyAccount, Verify, Zero},
	FixedPointNumber, FixedU128,
};

// The URL for the telemetry server.
// const STAGING_TELEMETRY_URL: &str = "wss://telemetry.polkadot.io/submit/";

const INITIAL_BALANCE: u128 = 100_000 * DOLLARS;

// The URL for the telemetry server.
const STAGING_TELEMETRY_URL: &str = "wss://telemetry.polkadot.io/submit/";

/// Specialized `ChainSpec`. This is a specialization of the general Substrate ChainSpec type.
pub type ChainSpec = sc_service::GenericChainSpec<GenesisConfig>;

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

/// Generate an Aura authority key.
pub fn authority_keys_from_seed(seed: &str) -> (AuraId, GrandpaId) {
	(get_from_seed::<AuraId>(seed), get_from_seed::<GrandpaId>(seed))
}

pub fn development_config() -> Result<ChainSpec, String> {
	let mut properties = Map::new();
	properties.insert("tokenDecimals".into(), 18.into());

	let wasm_binary = WASM_BINARY.ok_or_else(|| "Development wasm binary not available".to_string())?;

	Ok(ChainSpec::from_genesis(
		// Name
		"Development",
		// ID
		"dev",
		ChainType::Development,
		move || {
			testnet_genesis(
				wasm_binary,
				// Initial PoA authorities
				vec![authority_keys_from_seed("Alice")],
				// Sudo account
				get_account_id_from_seed::<sr25519::Public>("Alice"),
				// Pre-funded accounts
				vec![
					// liquidation pool
					hex!["6d6f646c6d696e2f6c71646e0000000000000000000000000000000000000000"].into(),
					hex!["680ee3a95d0b19619d9483fdee34f5d0016fbadd7145d016464f6bfbb993b46b"].into(),
					get_account_id_from_seed::<sr25519::Public>("Alice"),
					get_account_id_from_seed::<sr25519::Public>("Bob"),
					get_account_id_from_seed::<sr25519::Public>("Alice//stash"),
					get_account_id_from_seed::<sr25519::Public>("Bob//stash"),
				],
				true,
			)
		},
		// Bootnodes
		vec![],
		// Telemetry
		None,
		// Protocol ID
		None,
		// Properties
		Some(properties),
		// Extensions
		None,
	))
}

pub fn local_testnet_config() -> Result<ChainSpec, String> {
	let mut properties = Map::new();
	properties.insert("tokenDecimals".into(), 18.into());

	let wasm_binary = WASM_BINARY.ok_or_else(|| "Development wasm binary not available".to_string())?;

	Ok(ChainSpec::from_genesis(
		// Name
		"Local Testnet",
		// ID
		"local_testnet",
		ChainType::Local,
		move || {
			testnet_genesis(
				wasm_binary,
				// Initial PoA authorities
				vec![authority_keys_from_seed("Alice"), authority_keys_from_seed("Bob")],
				// Sudo account
				get_account_id_from_seed::<sr25519::Public>("Alice"),
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
				true,
			)
		},
		// Bootnodes
		vec![],
		// Telemetry
		None,
		// Protocol ID
		None,
		// Properties
		Some(properties),
		// Extensions
		None,
	))
}

pub fn minterest_turbo_testnet_config() -> Result<ChainSpec, String> {
	let mut properties = Map::new();
	properties.insert("tokenDecimals".into(), 18.into());

	let wasm_binary = WASM_BINARY.ok_or_else(|| "Development wasm binary not available".to_string())?;

	Ok(ChainSpec::from_genesis(
		"Minterest Turbo",
		"turbo-latest",
		ChainType::Live,
		move || {
			testnet_genesis(
				wasm_binary,
				// Initial PoA authorities
				vec![authority_keys_from_seed("Alice")],
				// Sudo account
				// 5ER9G3d2V4EEq8VjEbjkGbMdgprvtCntTYu9itCRJNHTkWYX
				hex!["680ee3a95d0b19619d9483fdee34f5d0016fbadd7145d016464f6bfbb993b46b"].into(),
				// Pre-funded accounts
				vec![
					hex!["680ee3a95d0b19619d9483fdee34f5d0016fbadd7145d016464f6bfbb993b46b"].into(),
					get_account_id_from_seed::<sr25519::Public>("Alice"),
					get_account_id_from_seed::<sr25519::Public>("Bob"),
					get_account_id_from_seed::<sr25519::Public>("Alice//stash"),
					get_account_id_from_seed::<sr25519::Public>("Bob//stash"),
				],
				true,
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
		Some(properties),
		// Extensions
		Default::default(),
	))
}

/// Configure initial storage state for FRAME pallets.
fn testnet_genesis(
	wasm_binary: &[u8],
	initial_authorities: Vec<(AuraId, GrandpaId)>,
	root_key: AccountId,
	endowed_accounts: Vec<AccountId>,
	_enable_println: bool,
) -> GenesisConfig {
	GenesisConfig {
		frame_system: Some(SystemConfig {
			// Add Wasm runtime to storage.
			code: wasm_binary.to_vec(),
			changes_trie_config: Default::default(),
		}),
		pallet_balances: Some(BalancesConfig {
			// Configure endowed accounts with initial balance of 1 << 60.
			balances: endowed_accounts.iter().cloned().map(|k| (k, 1 << 60)).collect(),
		}),
		pallet_aura: Some(AuraConfig {
			authorities: initial_authorities.iter().map(|x| (x.0.clone())).collect(),
		}),
		pallet_grandpa: Some(GrandpaConfig {
			authorities: initial_authorities.iter().map(|x| (x.1.clone(), 1)).collect(),
		}),
		pallet_sudo: Some(SudoConfig {
			// Assign network admin rights.
			key: root_key.clone(),
		}),
		orml_tokens: Some(TokensConfig {
			endowed_accounts: endowed_accounts
				.iter()
				.flat_map(|x| {
					vec![
						(x.clone(), CurrencyId::DOT, INITIAL_BALANCE),
						(x.clone(), CurrencyId::ETH, INITIAL_BALANCE),
						(x.clone(), CurrencyId::KSM, INITIAL_BALANCE),
						(x.clone(), CurrencyId::BTC, INITIAL_BALANCE),
					]
				})
				.collect(),
		}),
		liquidity_pools: Some(LiquidityPoolsConfig {
			pools: vec![
				(
					CurrencyId::ETH,
					Pool {
						total_borrowed: Balance::zero(),
						borrow_index: FixedU128::one(),
						total_insurance: Balance::zero(),
					},
				),
				(
					CurrencyId::DOT,
					Pool {
						total_borrowed: Balance::zero(),
						borrow_index: FixedU128::one(),
						total_insurance: Balance::zero(),
					},
				),
				(
					CurrencyId::KSM,
					Pool {
						total_borrowed: Balance::zero(),
						borrow_index: FixedU128::one(),
						total_insurance: Balance::zero(),
					},
				),
				(
					CurrencyId::BTC,
					Pool {
						total_borrowed: Balance::zero(),
						borrow_index: FixedU128::one(),
						total_insurance: Balance::zero(),
					},
				),
			],
			pool_user_data: vec![],
		}),
		controller: Some(ControllerConfig {
			controller_dates: vec![
				(
					CurrencyId::ETH,
					ControllerData {
						timestamp: 0,
						insurance_factor: FixedU128::saturating_from_rational(1, 10),
						max_borrow_rate: FixedU128::saturating_from_rational(5, 1000),
						collateral_factor: FixedU128::saturating_from_rational(9, 10), // 90%
						borrow_cap: None,
					},
				),
				(
					CurrencyId::DOT,
					ControllerData {
						timestamp: 0,
						insurance_factor: FixedU128::saturating_from_rational(1, 10),
						max_borrow_rate: FixedU128::saturating_from_rational(5, 1000),
						collateral_factor: FixedU128::saturating_from_rational(9, 10), // 90%
						borrow_cap: None,
					},
				),
				(
					CurrencyId::KSM,
					ControllerData {
						timestamp: 0,
						insurance_factor: FixedU128::saturating_from_rational(1, 10),
						max_borrow_rate: FixedU128::saturating_from_rational(5, 1000),
						collateral_factor: FixedU128::saturating_from_rational(9, 10), // 90%
						borrow_cap: None,
					},
				),
				(
					CurrencyId::BTC,
					ControllerData {
						timestamp: 0,
						insurance_factor: FixedU128::saturating_from_rational(1, 10),
						max_borrow_rate: FixedU128::saturating_from_rational(5, 1000),
						collateral_factor: FixedU128::saturating_from_rational(9, 10), // 90%
						borrow_cap: None,
					},
				),
			],
			pause_keepers: vec![
				(
					CurrencyId::ETH,
					PauseKeeper {
						deposit_paused: false,
						redeem_paused: false,
						borrow_paused: false,
						repay_paused: false,
						transfer_paused: false,
					},
				),
				(
					CurrencyId::DOT,
					PauseKeeper {
						deposit_paused: false,
						redeem_paused: false,
						borrow_paused: false,
						repay_paused: false,
						transfer_paused: false,
					},
				),
				(
					CurrencyId::KSM,
					PauseKeeper {
						deposit_paused: false,
						redeem_paused: false,
						borrow_paused: false,
						repay_paused: false,
						transfer_paused: false,
					},
				),
				(
					CurrencyId::BTC,
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
		}),
		minterest_model: Some(MinterestModelConfig {
			minterest_model_dates: vec![
				(
					CurrencyId::ETH,
					MinterestModelData {
						kink: FixedU128::saturating_from_rational(8, 10), // 0.8 = 80 %
						base_rate_per_block: FixedU128::zero(),
						multiplier_per_block: FixedU128::saturating_from_rational(9, 1_000_000_000), // 0.047304 PerYear
						jump_multiplier_per_block: FixedU128::saturating_from_rational(207, 1_000_000_000), // 1.09 PerYear
					},
				),
				(
					CurrencyId::DOT,
					MinterestModelData {
						kink: FixedU128::saturating_from_rational(8, 10), // 0.8 = 80 %
						base_rate_per_block: FixedU128::zero(),
						multiplier_per_block: FixedU128::saturating_from_rational(9, 1_000_000_000), // 0.047304 PerYear
						jump_multiplier_per_block: FixedU128::saturating_from_rational(207, 1_000_000_000), // 1.09 PerYear
					},
				),
				(
					CurrencyId::KSM,
					MinterestModelData {
						kink: FixedU128::saturating_from_rational(8, 10), // 0.8 = 80 %
						base_rate_per_block: FixedU128::zero(),
						multiplier_per_block: FixedU128::saturating_from_rational(9, 1_000_000_000), // 0.047304 PerYear
						jump_multiplier_per_block: FixedU128::saturating_from_rational(207, 1_000_000_000), // 1.09 PerYear
					},
				),
				(
					CurrencyId::BTC,
					MinterestModelData {
						kink: FixedU128::saturating_from_rational(8, 10), // 0.8 = 80 %
						base_rate_per_block: FixedU128::zero(),
						multiplier_per_block: FixedU128::saturating_from_rational(9, 1_000_000_000), // 0.047304 PerYear
						jump_multiplier_per_block: FixedU128::saturating_from_rational(207, 1_000_000_000), // 1.09 PerYear
					},
				),
			],
		}),
		risk_manager: Some(RiskManagerConfig {
			risk_manager_dates: vec![
				(
					CurrencyId::ETH,
					RiskManagerData {
						max_attempts: 2,
						min_sum: 200_000 * DOLLARS,                               // In USD. FIXME: temporary value.
						threshold: FixedU128::saturating_from_rational(103, 100), // 3%
						liquidation_incentive: FixedU128::saturating_from_rational(105, 100), // 5%
					},
				),
				(
					CurrencyId::DOT,
					RiskManagerData {
						max_attempts: 2,
						min_sum: 100_000 * DOLLARS,                               // In USD. FIXME: temporary value.
						threshold: FixedU128::saturating_from_rational(103, 100), // 3%
						liquidation_incentive: FixedU128::saturating_from_rational(105, 100), // 5%
					},
				),
				(
					CurrencyId::KSM,
					RiskManagerData {
						max_attempts: 2,
						min_sum: 200_000 * DOLLARS,                               // In USD. FIXME: temporary value.
						threshold: FixedU128::saturating_from_rational(103, 100), // 3%
						liquidation_incentive: FixedU128::saturating_from_rational(105, 100), // 5%
					},
				),
				(
					CurrencyId::BTC,
					RiskManagerData {
						max_attempts: 2,
						min_sum: 200_000 * DOLLARS,                               // In USD. FIXME: temporary value.
						threshold: FixedU128::saturating_from_rational(103, 100), // 3%
						liquidation_incentive: FixedU128::saturating_from_rational(105, 100), // 5%
					},
				),
			],
		}),
		liquidation_pools: Some(LiquidationPoolsConfig {
			liquidation_pool_params: (LiquidationPoolCommonData {
				timestamp: 1,
				balancing_period: 600, // Blocks per 10 minutes.
			}),
			liquidation_pools: vec![
				(
					CurrencyId::DOT,
					LiquidationPool {
						deviation_threshold: FixedU128::saturating_from_rational(1, 10),
						balance_ratio: FixedU128::saturating_from_rational(2, 10),
					},
				),
				(
					CurrencyId::ETH,
					LiquidationPool {
						deviation_threshold: FixedU128::saturating_from_rational(1, 10),
						balance_ratio: FixedU128::saturating_from_rational(2, 10),
					},
				),
				(
					CurrencyId::BTC,
					LiquidationPool {
						deviation_threshold: FixedU128::saturating_from_rational(1, 10),
						balance_ratio: FixedU128::saturating_from_rational(2, 10),
					},
				),
				(
					CurrencyId::KSM,
					LiquidationPool {
						deviation_threshold: FixedU128::saturating_from_rational(1, 10),
						balance_ratio: FixedU128::saturating_from_rational(2, 10),
					},
				),
			],
		}),
		pallet_collective_Instance1: Some(Default::default()),
		pallet_membership_Instance1: Some(MinterestCouncilMembershipConfig {
			members: vec![root_key.clone()],
			phantom: Default::default(),
		}),
		pallet_collective_Instance2: Some(Default::default()),
		pallet_membership_Instance2: Some(WhitelistCouncilMembershipConfig {
			members: vec![root_key],
			phantom: Default::default(),
		}),
		pallet_membership_Instance3: Some(OperatorMembershipMinterestConfig {
			members: endowed_accounts.clone(),
			phantom: Default::default(),
		}),
		orml_oracle_Instance1: Some(MinterestOracleConfig {
			members: Default::default(), // initialized by OperatorMembership
			phantom: Default::default(),
		}),
		mnt_token: Some(Default::default()),
	}
}
