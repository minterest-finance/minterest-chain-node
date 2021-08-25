use controller::{ControllerData, PauseKeeper};
use cumulus_primitives_core::ParaId;
use hex_literal::hex;
use liquidation_pools::LiquidationPoolData;
use liquidity_pools::PoolData;
use minterest_model::MinterestModelData;
use minterest_parachain_runtime as parachain_runtime;
use minterest_parachain_runtime::{
	get_all_modules_accounts, AccountId, Balance, ExistentialDeposit, MntTokenPalletId, Signature, BTC, DOLLARS, DOT,
	ETH, KSM, MNT, PROTOCOL_INTEREST_TRANSFER_THRESHOLD, TOTAL_ALLOCATION,
};
use minterest_primitives::currency::GetDecimals;
use minterest_primitives::{VestingBucket, VestingScheduleJson};
use minterest_standalone_runtime as standalone_runtime;
use sc_chain_spec::{ChainSpecExtension, ChainSpecGroup};
use sc_service::ChainType;
use serde::{Deserialize, Serialize};
use serde_json::map::Map;
use sp_consensus_aura::sr25519::AuthorityId as AuraId;
use sp_core::{sr25519, Pair, Public};
use sp_finality_grandpa::AuthorityId as GrandpaId;
use sp_runtime::{
	traits::{AccountIdConversion, IdentifyAccount, One, Verify, Zero},
	FixedPointNumber, FixedU128,
};
use sp_std::collections::btree_map::BTreeMap;
use std::collections::{HashMap, HashSet};

const INITIAL_BALANCE: u128 = 100_000 * DOLLARS;
const INITIAL_TREASURY: u128 = 5_000_000 * DOLLARS;

/// Specialized `ChainSpec`. This is a specialization of the general Substrate ChainSpec type.
pub type ChainSpec = sc_service::GenericChainSpec<parachain_runtime::GenesisConfig, Extensions>;
pub type StandaloneChainSpec = sc_service::GenericChainSpec<standalone_runtime::GenesisConfig, Extensions>;

/// Generate a crypto pair from seed.
pub fn get_from_seed<TPublic: Public>(seed: &str) -> <TPublic::Pair as Pair>::Public {
	TPublic::Pair::from_string(&format!("//{}", seed), None)
		.expect("static values are valid; qed")
		.public()
}

/// The extensions for the [`ChainSpec`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ChainSpecGroup, ChainSpecExtension)]
#[serde(deny_unknown_fields)]
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

type AccountPublic = <Signature as Verify>::Signer;

/// Generate an account ID from seed.
pub fn get_account_id_from_seed<TPublic: Public>(seed: &str) -> AccountId
where
	AccountPublic: From<<TPublic::Pair as Pair>::Public>,
{
	AccountPublic::from(get_from_seed::<TPublic>(seed)).into_account()
}

/// Generate an Aura authority key.
pub fn authority_keys_from_seed(seed: &str) -> AuraId {
	get_from_seed::<AuraId>(seed)
}

pub fn get_parachain_spec(id: ParaId) -> ChainSpec {
	ChainSpec::from_genesis(
		"Local Testnet",
		"local_testnet",
		ChainType::Local,
		move || {
			minterest_genesis(
				get_account_id_from_seed::<sr25519::Public>("Alice"),
				vec![get_from_seed::<AuraId>("Alice"), get_from_seed::<AuraId>("Bob")],
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
					// Eugene
					hex!["680ee3a95d0b19619d9483fdee34f5d0016fbadd7145d016464f6bfbb993b46b"].into(),
					// Marina
					hex!["567983d191ec6de996ed8546fc9ef65ce698e3c15f19de1c4011436096b8a915"].into(),
					// Julia
					hex!["cedd4c175ec803c5d3f5b2d3e9e63c8ec73aff05414acd0981f60ae24079bc44"].into(),
					// Polina
					hex!["2e314191e6f8a49b0fdd374dd243b20cc8b1f32a44ba512692ad5e8c5d251c7f"].into(),
					hex!["6ae90e9d3f0b4f1161a12024b46c7b44030bedbc4772260f1836262b37806d15"].into(),
					hex!["38099e3930713a1fdae1419be266ea78ff353752a83033acbe215e190cb0cf2b"].into(),
					hex!["267e9faef0221b88501b0b943222b3d9f052e8308de28bc86f10780e8f9c5b0a"].into(),
					//Kirill
					hex!["48c9efb4a2f9af46aa67db1142adba5a762734ea06a4e3f933309d31ce00504c"].into(),
				],
				id,
			)
		},
		vec![],
		None,
		None,
		Some({
			let mut properties = Map::new();
			properties.insert("tokenDecimals".into(), 18.into());
			properties
		}),
		Extensions {
			relay_chain: "rococo-local".into(),
			para_id: id.into(),
		},
	)
}

pub fn get_standalone_dev_spec() -> StandaloneChainSpec {
	StandaloneChainSpec::from_genesis(
		"Standalone Development",
		"dev",
		ChainType::Development,
		move || {
			standalone_dev_genesis(
				get_account_id_from_seed::<sr25519::Public>("Alice"),
				vec![(get_from_seed::<AuraId>("Alice"), get_from_seed::<GrandpaId>("Alice"))],
				vec![
					get_account_id_from_seed::<sr25519::Public>("Alice"),
					get_account_id_from_seed::<sr25519::Public>("Bob"),
					get_account_id_from_seed::<sr25519::Public>("Charlie"),
					get_account_id_from_seed::<sr25519::Public>("Dave"),
					get_account_id_from_seed::<sr25519::Public>("Eve"),
					get_account_id_from_seed::<sr25519::Public>("Ferdie"),
					get_account_id_from_seed::<sr25519::Public>("Alice//stash"),
					get_account_id_from_seed::<sr25519::Public>("Bob//stash"),
					// Eugene
					hex!["680ee3a95d0b19619d9483fdee34f5d0016fbadd7145d016464f6bfbb993b46b"].into(),
					// Marina
					hex!["ec1686827c6d2bf042501bccaebd8383c6a9ca892c8e63faaa620e549696eb01"].into(),
					// Julia
					hex!["cedd4c175ec803c5d3f5b2d3e9e63c8ec73aff05414acd0981f60ae24079bc44"].into(),
					// Polina
					hex!["2e314191e6f8a49b0fdd374dd243b20cc8b1f32a44ba512692ad5e8c5d251c7f"].into(),
					hex!["6ae90e9d3f0b4f1161a12024b46c7b44030bedbc4772260f1836262b37806d15"].into(),
					hex!["38099e3930713a1fdae1419be266ea78ff353752a83033acbe215e190cb0cf2b"].into(),
					hex!["267e9faef0221b88501b0b943222b3d9f052e8308de28bc86f10780e8f9c5b0a"].into(),
				],
			)
		},
		vec![],
		None,
		None,
		Some({
			let mut properties = Map::new();
			properties.insert("tokenDecimals".into(), 18.into());
			properties
		}),
		Extensions {
			relay_chain: "rococo-local".into(),
			para_id: 2000,
		},
	)
}

pub fn get_standalone_local_spec() -> StandaloneChainSpec {
	StandaloneChainSpec::from_genesis(
		"Standalone Local",
		"standalone-local",
		ChainType::Local,
		move || {
			standalone_dev_genesis(
				get_account_id_from_seed::<sr25519::Public>("Alice"),
				vec![
					(get_from_seed::<AuraId>("Alice"), get_from_seed::<GrandpaId>("Alice")),
					(get_from_seed::<AuraId>("Bob"), get_from_seed::<GrandpaId>("Bob")),
				],
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
					// Eugene
					hex!["680ee3a95d0b19619d9483fdee34f5d0016fbadd7145d016464f6bfbb993b46b"].into(),
					// Marina
					hex!["ec1686827c6d2bf042501bccaebd8383c6a9ca892c8e63faaa620e549696eb01"].into(),
					// Julia
					hex!["cedd4c175ec803c5d3f5b2d3e9e63c8ec73aff05414acd0981f60ae24079bc44"].into(),
					// Polina
					hex!["2e314191e6f8a49b0fdd374dd243b20cc8b1f32a44ba512692ad5e8c5d251c7f"].into(),
					hex!["6ae90e9d3f0b4f1161a12024b46c7b44030bedbc4772260f1836262b37806d15"].into(),
					hex!["38099e3930713a1fdae1419be266ea78ff353752a83033acbe215e190cb0cf2b"].into(),
					hex!["267e9faef0221b88501b0b943222b3d9f052e8308de28bc86f10780e8f9c5b0a"].into(),
				],
			)
		},
		vec![],
		None,
		None,
		Some({
			let mut properties = Map::new();
			properties.insert("tokenDecimals".into(), 18.into());
			properties
		}),
		Extensions {
			relay_chain: "rococo-local".into(),
			para_id: 2000,
		},
	)
}

/// Configure initial storage state for FRAME pallets.
/// This initial storage state is used in `local_testnet_config` and
/// `minterest_turbo_testnet_config`.
fn minterest_genesis(
	root_key: AccountId,
	initial_authorities: Vec<AuraId>,
	endowed_accounts: Vec<AccountId>,
	para_id: ParaId,
) -> parachain_runtime::GenesisConfig {
	// Reading the initial allocations from the file.
	let allocated_accounts_json = &include_bytes!("../../resources/dev-minterest-allocation-MNT.json")[..];
	let allocated_list_parsed: HashMap<VestingBucket, Vec<VestingScheduleJson<AccountId, Balance>>> =
		serde_json::from_slice(allocated_accounts_json).unwrap();

	let allocated_list = allocated_list_parsed
		.iter()
		.flat_map(|(_bucket, schedules)| {
			schedules
				.iter()
				.map(|schedule| (schedule.account.clone(), schedule.amount))
		})
		.collect::<Vec<(AccountId, Balance)>>();

	// Initial allocation calculation
	let initial_allocations = calculate_initial_allocations(endowed_accounts, allocated_list);
	// Vesting calculation
	let vesting_list = calculate_vesting_list(allocated_list_parsed);

	// Reading whitelist members from the file.
	let white_list_members_json = &include_bytes!("../../resources/whitelist-members.json")[..];
	let whitelist_members: Vec<AccountId> = serde_json::from_slice(white_list_members_json).unwrap();

	parachain_runtime::GenesisConfig {
		system: parachain_runtime::SystemConfig {
			code: parachain_runtime::WASM_BINARY
				.expect("WASM binary was not build, please build it!")
				.to_vec(),
			changes_trie_config: Default::default(),
		},
		balances: parachain_runtime::BalancesConfig {
			balances: initial_allocations,
		},
		parachain_info: parachain_runtime::ParachainInfoConfig { parachain_id: para_id },
		aura: parachain_runtime::AuraConfig {
			authorities: initial_authorities.to_vec(),
		},
		sudo: parachain_runtime::SudoConfig {
			// Assign network admin rights.
			key: root_key.clone(),
		},
		tokens: parachain_runtime::TokensConfig { balances: vec![] },
		liquidity_pools: parachain_runtime::LiquidityPoolsConfig {
			pools: vec![
				(
					ETH,
					PoolData {
						borrowed: Balance::zero(),
						borrow_index: FixedU128::one(),
						protocol_interest: Balance::zero(),
					},
				),
				(
					DOT,
					PoolData {
						borrowed: Balance::zero(),
						borrow_index: FixedU128::one(),
						protocol_interest: Balance::zero(),
					},
				),
				(
					KSM,
					PoolData {
						borrowed: Balance::zero(),
						borrow_index: FixedU128::one(),
						protocol_interest: Balance::zero(),
					},
				),
				(
					BTC,
					PoolData {
						borrowed: Balance::zero(),
						borrow_index: FixedU128::one(),
						protocol_interest: Balance::zero(),
					},
				),
			],
			pool_user_data: vec![],
		},
		controller: parachain_runtime::ControllerConfig {
			controller_params: vec![
				(
					ETH,
					ControllerData {
						last_interest_accrued_block: 0,
						protocol_interest_factor: FixedU128::saturating_from_rational(1, 10),
						max_borrow_rate: FixedU128::saturating_from_rational(5, 1000),
						collateral_factor: FixedU128::saturating_from_rational(9, 10), // 90%
						borrow_cap: None,
						protocol_interest_threshold: PROTOCOL_INTEREST_TRANSFER_THRESHOLD,
					},
				),
				(
					DOT,
					ControllerData {
						last_interest_accrued_block: 0,
						protocol_interest_factor: FixedU128::saturating_from_rational(1, 10),
						max_borrow_rate: FixedU128::saturating_from_rational(5, 1000),
						collateral_factor: FixedU128::saturating_from_rational(9, 10), // 90%
						borrow_cap: None,
						protocol_interest_threshold: PROTOCOL_INTEREST_TRANSFER_THRESHOLD,
					},
				),
				(
					KSM,
					ControllerData {
						last_interest_accrued_block: 0,
						protocol_interest_factor: FixedU128::saturating_from_rational(1, 10),
						max_borrow_rate: FixedU128::saturating_from_rational(5, 1000),
						collateral_factor: FixedU128::saturating_from_rational(9, 10), // 90%
						borrow_cap: None,
						protocol_interest_threshold: PROTOCOL_INTEREST_TRANSFER_THRESHOLD,
					},
				),
				(
					BTC,
					ControllerData {
						last_interest_accrued_block: 0,
						protocol_interest_factor: FixedU128::saturating_from_rational(1, 10),
						max_borrow_rate: FixedU128::saturating_from_rational(5, 1000),
						collateral_factor: FixedU128::saturating_from_rational(9, 10), // 90%
						borrow_cap: None,
						protocol_interest_threshold: PROTOCOL_INTEREST_TRANSFER_THRESHOLD,
					},
				),
			],
			pause_keepers: vec![
				(ETH, PauseKeeper::all_unpaused()),
				(DOT, PauseKeeper::all_unpaused()),
				(KSM, PauseKeeper::all_unpaused()),
				(BTC, PauseKeeper::all_unpaused()),
			],
		},
		minterest_model: parachain_runtime::MinterestModelConfig {
			minterest_model_params: vec![
				(
					ETH,
					MinterestModelData {
						kink: FixedU128::saturating_from_rational(8, 10), // 0.8 = 80 %
						base_rate_per_block: FixedU128::zero(),
						multiplier_per_block: FixedU128::saturating_from_rational(9, 1_000_000_000), // 0.047304 PerYear
						jump_multiplier_per_block: FixedU128::saturating_from_rational(207, 1_000_000_000), // 1.09 PerYear
					},
				),
				(
					DOT,
					MinterestModelData {
						kink: FixedU128::saturating_from_rational(8, 10), // 0.8 = 80 %
						base_rate_per_block: FixedU128::zero(),
						multiplier_per_block: FixedU128::saturating_from_rational(9, 1_000_000_000), // 0.047304 PerYear
						jump_multiplier_per_block: FixedU128::saturating_from_rational(207, 1_000_000_000), // 1.09 PerYear
					},
				),
				(
					KSM,
					MinterestModelData {
						kink: FixedU128::saturating_from_rational(8, 10), // 0.8 = 80 %
						base_rate_per_block: FixedU128::zero(),
						multiplier_per_block: FixedU128::saturating_from_rational(9, 1_000_000_000), // 0.047304 PerYear
						jump_multiplier_per_block: FixedU128::saturating_from_rational(207, 1_000_000_000), // 1.09 PerYear
					},
				),
				(
					BTC,
					MinterestModelData {
						kink: FixedU128::saturating_from_rational(8, 10), // 0.8 = 80 %
						base_rate_per_block: FixedU128::zero(),
						multiplier_per_block: FixedU128::saturating_from_rational(9, 1_000_000_000), // 0.047304 PerYear
						jump_multiplier_per_block: FixedU128::saturating_from_rational(207, 1_000_000_000), // 1.09 PerYear
					},
				),
			],
			_phantom: Default::default(),
		},
		risk_manager: parachain_runtime::RiskManagerConfig {
			liquidation_fee: vec![
				(DOT, FixedU128::saturating_from_rational(5, 100)),
				(ETH, FixedU128::saturating_from_rational(5, 100)),
				(BTC, FixedU128::saturating_from_rational(5, 100)),
				(KSM, FixedU128::saturating_from_rational(5, 100)),
			],
			liquidation_threshold: FixedU128::saturating_from_rational(103, 100),
			_phantom: Default::default(),
		},
		liquidation_pools: parachain_runtime::LiquidationPoolsConfig {
			phantom: Default::default(),
			liquidation_pools: vec![
				(
					DOT,
					LiquidationPoolData {
						deviation_threshold: FixedU128::saturating_from_rational(1, 10),
						balance_ratio: FixedU128::saturating_from_rational(2, 10),
						max_ideal_balance_usd: None,
					},
				),
				(
					ETH,
					LiquidationPoolData {
						deviation_threshold: FixedU128::saturating_from_rational(1, 10),
						balance_ratio: FixedU128::saturating_from_rational(2, 10),
						max_ideal_balance_usd: None,
					},
				),
				(
					BTC,
					LiquidationPoolData {
						deviation_threshold: FixedU128::saturating_from_rational(1, 10),
						balance_ratio: FixedU128::saturating_from_rational(2, 10),
						max_ideal_balance_usd: None,
					},
				),
				(
					KSM,
					LiquidationPoolData {
						deviation_threshold: FixedU128::saturating_from_rational(1, 10),
						balance_ratio: FixedU128::saturating_from_rational(2, 10),
						max_ideal_balance_usd: None,
					},
				),
			],
		},
		prices: parachain_runtime::PricesConfig {
			locked_price: vec![
				(DOT, FixedU128::saturating_from_integer(2)),
				(KSM, FixedU128::saturating_from_integer(2)),
				(ETH, FixedU128::saturating_from_integer(2)),
				(BTC, FixedU128::saturating_from_integer(2)),
				(MNT, FixedU128::saturating_from_integer(2)),
			],
			_phantom: Default::default(),
		},
		minterest_council: Default::default(),
		minterest_council_membership: parachain_runtime::MinterestCouncilMembershipConfig {
			members: vec![root_key.clone()],
			phantom: Default::default(),
		},
		operator_membership_minterest: parachain_runtime::OperatorMembershipMinterestConfig {
			members: vec![root_key.clone()],
			phantom: Default::default(),
		},
		mnt_token: parachain_runtime::MntTokenConfig {
			mnt_claim_threshold: 0, // disable by default
			minted_pools: vec![
				(DOT, (237977549 * DOLLARS) / 1_000_000_000),
				(ETH, (237977549 * DOLLARS) / 1_000_000_000),
				(KSM, (237977549 * DOLLARS) / 1_000_000_000),
				(BTC, (237977549 * DOLLARS) / 1_000_000_000),
			],
			_phantom: Default::default(),
		},
		vesting: parachain_runtime::VestingConfig { vesting: vesting_list },
		whitelist: parachain_runtime::WhitelistConfig {
			members: whitelist_members,
			whitelist_mode: false,
		},
		chainlink_feed: parachain_runtime::ChainlinkFeedConfig {
			pallet_admin: Some(root_key.clone()),
			feed_creators: vec![root_key],
		},
		aura_ext: Default::default(),
		parachain_system: Default::default(),
	}
}

fn standalone_dev_genesis(
	root_key: AccountId,
	initial_authorities: Vec<(AuraId, GrandpaId)>,
	endowed_accounts: Vec<AccountId>,
) -> standalone_runtime::GenesisConfig {
	standalone_runtime::GenesisConfig {
		system: standalone_runtime::SystemConfig {
			code: standalone_runtime::WASM_BINARY
				.expect("WASM binary was not build, please build it!")
				.to_vec(),
			changes_trie_config: Default::default(),
		},
		balances: standalone_runtime::BalancesConfig {
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
		aura: standalone_runtime::AuraConfig {
			authorities: initial_authorities.iter().map(|x| (x.0.clone())).collect(),
		},
		grandpa: standalone_runtime::GrandpaConfig {
			authorities: initial_authorities.iter().map(|x| (x.1.clone(), 1)).collect(),
		},
		sudo: standalone_runtime::SudoConfig {
			// Assign network admin rights.
			key: root_key.clone(),
		},
		tokens: standalone_runtime::TokensConfig {
			balances: endowed_accounts
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
		liquidity_pools: standalone_runtime::LiquidityPoolsConfig {
			pools: vec![
				(
					ETH,
					PoolData {
						borrowed: Balance::zero(),
						borrow_index: FixedU128::one(),
						protocol_interest: Balance::zero(),
					},
				),
				(
					DOT,
					PoolData {
						borrowed: Balance::zero(),
						borrow_index: FixedU128::one(),
						protocol_interest: Balance::zero(),
					},
				),
				(
					KSM,
					PoolData {
						borrowed: Balance::zero(),
						borrow_index: FixedU128::one(),
						protocol_interest: Balance::zero(),
					},
				),
				(
					BTC,
					PoolData {
						borrowed: Balance::zero(),
						borrow_index: FixedU128::one(),
						protocol_interest: Balance::zero(),
					},
				),
			],
			pool_user_data: vec![],
		},
		controller: standalone_runtime::ControllerConfig {
			controller_params: vec![
				(
					ETH,
					ControllerData {
						last_interest_accrued_block: 0,
						protocol_interest_factor: FixedU128::saturating_from_rational(1, 10),
						max_borrow_rate: FixedU128::saturating_from_rational(5, 1000),
						collateral_factor: FixedU128::saturating_from_rational(9, 10), // 90%
						borrow_cap: None,
						protocol_interest_threshold: PROTOCOL_INTEREST_TRANSFER_THRESHOLD,
					},
				),
				(
					DOT,
					ControllerData {
						last_interest_accrued_block: 0,
						protocol_interest_factor: FixedU128::saturating_from_rational(1, 10),
						max_borrow_rate: FixedU128::saturating_from_rational(5, 1000),
						collateral_factor: FixedU128::saturating_from_rational(9, 10), // 90%
						borrow_cap: None,
						protocol_interest_threshold: PROTOCOL_INTEREST_TRANSFER_THRESHOLD,
					},
				),
				(
					KSM,
					ControllerData {
						last_interest_accrued_block: 0,
						protocol_interest_factor: FixedU128::saturating_from_rational(1, 10),
						max_borrow_rate: FixedU128::saturating_from_rational(5, 1000),
						collateral_factor: FixedU128::saturating_from_rational(9, 10), // 90%
						borrow_cap: None,
						protocol_interest_threshold: PROTOCOL_INTEREST_TRANSFER_THRESHOLD,
					},
				),
				(
					BTC,
					ControllerData {
						last_interest_accrued_block: 0,
						protocol_interest_factor: FixedU128::saturating_from_rational(1, 10),
						max_borrow_rate: FixedU128::saturating_from_rational(5, 1000),
						collateral_factor: FixedU128::saturating_from_rational(9, 10), // 90%
						borrow_cap: None,
						protocol_interest_threshold: PROTOCOL_INTEREST_TRANSFER_THRESHOLD,
					},
				),
			],
			pause_keepers: vec![
				(ETH, PauseKeeper::all_unpaused()),
				(DOT, PauseKeeper::all_unpaused()),
				(KSM, PauseKeeper::all_unpaused()),
				(BTC, PauseKeeper::all_unpaused()),
			],
		},
		minterest_model: standalone_runtime::MinterestModelConfig {
			minterest_model_params: vec![
				(
					ETH,
					MinterestModelData {
						kink: FixedU128::saturating_from_rational(8, 10), // 0.8 = 80 %
						base_rate_per_block: FixedU128::zero(),
						multiplier_per_block: FixedU128::saturating_from_rational(9, 1_000_000_000), // 0.047304 PerYear
						jump_multiplier_per_block: FixedU128::saturating_from_rational(207, 1_000_000_000), // 1.09 PerYear
					},
				),
				(
					DOT,
					MinterestModelData {
						kink: FixedU128::saturating_from_rational(8, 10), // 0.8 = 80 %
						base_rate_per_block: FixedU128::zero(),
						multiplier_per_block: FixedU128::saturating_from_rational(9, 1_000_000_000), // 0.047304 PerYear
						jump_multiplier_per_block: FixedU128::saturating_from_rational(207, 1_000_000_000), // 1.09 PerYear
					},
				),
				(
					KSM,
					MinterestModelData {
						kink: FixedU128::saturating_from_rational(8, 10), // 0.8 = 80 %
						base_rate_per_block: FixedU128::zero(),
						multiplier_per_block: FixedU128::saturating_from_rational(9, 1_000_000_000), // 0.047304 PerYear
						jump_multiplier_per_block: FixedU128::saturating_from_rational(207, 1_000_000_000), // 1.09 PerYear
					},
				),
				(
					BTC,
					MinterestModelData {
						kink: FixedU128::saturating_from_rational(8, 10), // 0.8 = 80 %
						base_rate_per_block: FixedU128::zero(),
						multiplier_per_block: FixedU128::saturating_from_rational(9, 1_000_000_000), // 0.047304 PerYear
						jump_multiplier_per_block: FixedU128::saturating_from_rational(207, 1_000_000_000), // 1.09 PerYear
					},
				),
			],
			_phantom: Default::default(),
		},
		risk_manager: standalone_runtime::RiskManagerConfig {
			liquidation_fee: vec![
				(DOT, FixedU128::saturating_from_rational(5, 100)), // 5%
				(ETH, FixedU128::saturating_from_rational(5, 100)), // 5%
				(BTC, FixedU128::saturating_from_rational(5, 100)), // 5%
				(KSM, FixedU128::saturating_from_rational(5, 100)), // 5%
			],
			liquidation_threshold: FixedU128::saturating_from_rational(3, 100), // 3%
			_phantom: Default::default(),
		},
		liquidation_pools: standalone_runtime::LiquidationPoolsConfig {
			phantom: Default::default(),
			liquidation_pools: vec![
				(
					DOT,
					LiquidationPoolData {
						deviation_threshold: FixedU128::saturating_from_rational(1, 10),
						balance_ratio: FixedU128::saturating_from_rational(2, 10),
						max_ideal_balance_usd: None,
					},
				),
				(
					ETH,
					LiquidationPoolData {
						deviation_threshold: FixedU128::saturating_from_rational(1, 10),
						balance_ratio: FixedU128::saturating_from_rational(2, 10),
						max_ideal_balance_usd: None,
					},
				),
				(
					BTC,
					LiquidationPoolData {
						deviation_threshold: FixedU128::saturating_from_rational(1, 10),
						balance_ratio: FixedU128::saturating_from_rational(2, 10),
						max_ideal_balance_usd: None,
					},
				),
				(
					KSM,
					LiquidationPoolData {
						deviation_threshold: FixedU128::saturating_from_rational(1, 10),
						balance_ratio: FixedU128::saturating_from_rational(2, 10),
						max_ideal_balance_usd: None,
					},
				),
			],
		},
		prices: standalone_runtime::PricesConfig {
			locked_price: vec![
				(DOT, FixedU128::saturating_from_integer(2)),
				(KSM, FixedU128::saturating_from_integer(2)),
				(ETH, FixedU128::saturating_from_integer(2)),
				(BTC, FixedU128::saturating_from_integer(2)),
				(MNT, FixedU128::saturating_from_integer(2)),
			],
			_phantom: Default::default(),
		},
		minterest_council: Default::default(),
		minterest_council_membership: standalone_runtime::MinterestCouncilMembershipConfig {
			members: vec![root_key.clone()],
			phantom: Default::default(),
		},
		operator_membership_minterest: standalone_runtime::OperatorMembershipMinterestConfig {
			members: endowed_accounts.clone(),
			phantom: Default::default(),
		},
		mnt_token: standalone_runtime::MntTokenConfig {
			mnt_claim_threshold: 0, // disable by default
			minted_pools: vec![
				(DOT, 2 * DOLLARS),
				(ETH, 2 * DOLLARS),
				(KSM, 2 * DOLLARS),
				(BTC, 2 * DOLLARS),
			],
			_phantom: Default::default(),
		},
		vesting: standalone_runtime::VestingConfig { vesting: vec![] },
		whitelist: standalone_runtime::WhitelistConfig {
			members: endowed_accounts.clone(),
			whitelist_mode: false,
		},
		chainlink_feed: standalone_runtime::ChainlinkFeedConfig {
			pallet_admin: Some(root_key.clone()),
			feed_creators: vec![root_key],
		},
		aura_ext: Default::default(),
		parachain_system: Default::default(),
		parachain_info: Default::default(),
	}
}

/// Calculates the total allocation and generates a list of accounts with balance for allocation.
///
/// - `ed_accounts`: accounts to which the existential balance should be deposited
/// - `allocated_list`: vector of accounts with their initial allocations
///
/// Return:
/// `vec[(account_id, allocation)]` - vector of accounts with their initial allocations
pub(crate) fn calculate_initial_allocations(
	ed_accounts: Vec<AccountId>,
	allocated_list: Vec<(AccountId, Balance)>,
) -> Vec<(AccountId, Balance)> {
	// Initial allocation calculation
	let existential_deposit = ExistentialDeposit::get();
	let mut total_allocated = Balance::zero();

	// Calculation existential balance for the pallets accounts and sudo account.
	let existential_balances: Vec<(AccountId, Balance)> = ed_accounts
		.into_iter()
		.map(|account_id| (account_id, existential_deposit))
		.collect();
	let total_existential = existential_balances.iter().map(|(_, x)| x).sum::<u128>();

	// The mnt-token pallet balance: community_bucket_total_amount - total_existential
	let mnt_token_pallet_balance = VestingBucket::Community
		.total_amount()
		.checked_sub(total_existential)
		.expect("overflow in the calculation of the mnt-token pallet balance");

	let initial_allocations = existential_balances
		.into_iter()
		.chain(vec![(MntTokenPalletId::get().into_account(), mnt_token_pallet_balance)])
		.chain(allocated_list)
		.fold(
			BTreeMap::<AccountId, Balance>::new(),
			|mut acc, (account_id, amount)| {
				// merge duplicated accounts
				if let Some(balance) = acc.get_mut(&account_id) {
					*balance = balance
						.checked_add(amount)
						.expect("balance cannot overflow when building genesis");
				} else {
					acc.insert(account_id.clone(), amount);
				}

				total_allocated = total_allocated
					.checked_add(amount)
					.expect("total insurance cannot overflow when building genesis");
				acc
			},
		)
		.into_iter()
		.collect::<Vec<(AccountId, Balance)>>();

	// check total allocated
	assert_eq!(
		total_allocated,
		TOTAL_ALLOCATION,
		"Total allocation must be equal to 100,000,030 MNT tokens, but passed: {} MNT",
		total_allocated / 10_u128.pow(MNT.decimals())
	);
	initial_allocations
}

/// Checks vesting buckets and generates a list of vesting.
///
/// - `allocated_list_parsed`: a HashMap of the following type:
/// "PrivateSale": [
///     {
///       "account": "5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY",
///       "amount": 10000000000000000000000000
///     }]
/// Return:
/// `vesting_list` - vector of accounts with their initial vesting.
pub(crate) fn calculate_vesting_list(
	allocated_list_parsed: HashMap<VestingBucket, Vec<VestingScheduleJson<AccountId, Balance>>>,
) -> Vec<(VestingBucket, AccountId, Balance)> {
	let mut vesting_list: Vec<(VestingBucket, AccountId, Balance)> = Vec::new();

	assert_eq!(
		allocated_list_parsed.len(),
		7_usize,
		"The total number of buckets in the allocation json file must be seven, but passed: {}",
		allocated_list_parsed.len()
	);

	for (bucket, schedules) in allocated_list_parsed.iter() {
		let total_bucket_amount: Balance = schedules.iter().map(|schedule| schedule.amount).sum();
		assert_eq!(
			total_bucket_amount,
			bucket.total_amount(),
			"The total amount of distributed tokens must be equal to the number of tokens in the bucket."
		);

		// Calculate vesting schedules.
		for schedule_json in schedules.iter() {
			vesting_list.push((*bucket, schedule_json.account.clone(), schedule_json.amount));
		}
	}

	// ensure no duplicates exist.
	let mut uniq = HashSet::new();
	assert!(
		vesting_list
			.iter()
			.map(|(_, account, _)| account)
			.cloned()
			.all(move |x| uniq.insert(x)),
		"duplicate vesting accounts in genesis."
	);

	vesting_list
}
