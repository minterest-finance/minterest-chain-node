//! Service and ServiceFactory implementation. Specialized wrapper over substrate service.
#![allow(clippy::type_complexity)]

use cumulus_client_consensus_relay_chain::{build_relay_chain_consensus, BuildRelayChainConsensusParams};
use cumulus_client_network::build_block_announce_validator;
use cumulus_client_service::{
	prepare_node_config, start_collator, start_full_node, StartCollatorParams, StartFullNodeParams,
};
use cumulus_primitives_core::ParaId;

use cumulus_primitives_core::relay_chain::v1::CollatorPair;
use node_minterest_runtime::{self, opaque::Block, RuntimeApi};
use sc_client_api::{ExecutorProvider, RemoteBackend};
pub use sc_executor::NativeExecutor;
use sc_executor::{native_executor_instance, NativeExecutionDispatch};
use sc_finality_grandpa::SharedVoterState;
use sc_keystore::LocalKeystore;
use sc_service::{error::Error as ServiceError, Configuration, TFullClient, TaskManager};
use sc_telemetry::{Telemetry, TelemetryWorkerHandle};
use sp_api::ConstructRuntimeApi;
use sp_consensus_aura::sr25519::AuthorityPair as AuraPair;
use sp_inherents::InherentDataProviders;
use sp_runtime::traits::BlakeTwo256;
use sp_trie::PrefixedMemoryDB;
use std::sync::Arc;
use std::time::Duration;

pub mod chain_spec;

// Our native executor instance.
native_executor_instance!(
	pub Executor,
	node_minterest_runtime::api::dispatch,
	node_minterest_runtime::native_version,
	frame_benchmarking::benchmarking::HostFunctions,
);

type FullClient<RuntimeApi, Executor> = sc_service::TFullClient<Block, RuntimeApi, Executor>;
type FullBackend = sc_service::TFullBackend<Block>;
type FullSelectChain = sc_consensus::LongestChain<FullBackend, Block>;

pub fn new_partial<RuntimeApi, Executor>(
	config: &Configuration,
) -> Result<
	sc_service::PartialComponents<
		FullClient<RuntimeApi, Executor>,
		FullBackend,
		(),
		sp_consensus::import_queue::BasicQueue<Block, PrefixedMemoryDB<BlakeTwo256>>,
		sc_transaction_pool::FullPool<Block, FullClient<RuntimeApi, Executor>>,
		(Option<Telemetry>, Option<TelemetryWorkerHandle>),
	>,
	sc_service::Error,
>
where
	RuntimeApi: ConstructRuntimeApi<Block, FullClient<RuntimeApi, Executor>> + Send + Sync + 'static,
	RuntimeApi::RuntimeApi: RuntimeApiCollection<StateBackend = sc_client_api::StateBackendFor<FullBackend, Block>>, /* FIXME: WTF? */
	RuntimeApi::RuntimeApi: sp_consensus_aura::AuraApi<Block, AuraId>,
	Executor: NativeExecutionDispatch + 'static,
{
	if config.keystore_remote.is_some() {
		return Err(ServiceError::Other("Remote Keystores are not supported.".to_string()));
	}
	let inherent_data_providers = sp_inherents::InherentDataProviders::new();

	let (client, backend, keystore_container, task_manager) =
		sc_service::new_full_parts::<Block, RuntimeApi, Executor>(&config)?;
	let client = Arc::new(client);

	let select_chain = sc_consensus::LongestChain::new(backend.clone());

	let transaction_pool = sc_transaction_pool::BasicPool::new_full(
		config.transaction_pool.clone(),
		config.role.is_authority().into(),
		config.prometheus_registry(),
		task_manager.spawn_handle(),
		client.clone(),
	);

	// FIXME: should be removed
	// let (grandpa_block_import, grandpa_link) =
	// 	sc_finality_grandpa::block_import(client.clone(), &(client.clone() as Arc<_>),
	// select_chain.clone())?;

	// FIXME: ?
	// let _aura_block_import =
	// 	sc_consensus_aura::AuraBlockImport::<_, _, _, AuraPair>::new(grandpa_block_import.clone(),
	// client.clone());

	// let import_queue = sc_consensus_aura::import_queue::<_, _, _, AuraPair, _, _>(
	// 	sc_consensus_aura::slot_duration(&*client)?,
	// 	aura_block_import.clone(),
	// 	Some(Box::new(grandpa_block_import)),
	// 	client.clone(),
	// 	inherent_data_providers.clone(),
	// 	&task_manager.spawn_handle(),
	// 	config.prometheus_registry(),
	// 	sp_consensus::CanAuthorWithNativeVersion::new(client.executor().clone()),
	// )?;

	// FIXME: What about cumulus ?
	let import_queue = cumulus_client_consensus_relay_chain::import_queue(
		client.clone(),
		client.clone(),
		inherent_data_providers.clone(),
		&task_manager.spawn_essential_handle(),
		registry,
	)?;

	Ok(sc_service::PartialComponents {
		backend,
		client,
		import_queue,
		keystore_container,
		task_manager,
		transaction_pool,
		inherent_data_providers,
		select_chain: (),
		other: (None, None), // FIXME: do we need aura block import && grandpa_link?
	})
}

// FIXME: after this shoukd be fns
// 		start_node_impl:
// 			Start a node with the given parachain `Configuration` and relay chain
// 			`Configuration`.
//      		This is the actual implementation that is abstract over the executor and the
// 				runtime api.

// start_node: Start a normal parachain node.
//
// Acala has extra funtionality, for why?

#[sc_tracing::logging::prefix_logs_with("Parachain")]
async fn start_node_impl<RB, RuntimeApi, Executor>(
	parachain_config: Configuration,
	collator_key: CollatorPair,
	polkadot_config: Configuration,
	id: ParaId,
	validator: bool,
	rpc_ext_builder: RB,
) -> sc_service::error::Result<(TaskManager, Arc<FullClient<RuntimeApi, Executor>>)>
where
	RB: Fn(Arc<FullClient<RuntimeApi, Executor>>) -> jsonrpc_core::IoHandler<sc_rpc::Metadata> + Send + 'static,
	RuntimeApi: ConstructRuntimeApi<Block, FullClient<RuntimeApi, Executor>> + Send + Sync + 'static,
	RuntimeApi::RuntimeApi: RuntimeApiCollection<StateBackend = sc_client_api::StateBackendFor<FullBackend, Block>>,
	RuntimeApi::RuntimeApi: sp_consensus_aura::AuraApi<Block, AuraId>,
	Executor: NativeExecutionDispatch + 'static,
{
	if matches!(parachain_config.role, Role::Light) {
		return Err("Light client not supported!".into());
	}

	let parachain_config = prepare_node_config(parachain_config);

	let params = new_partial(&parachain_config)?;
	params
		.inherent_data_providers
		.register_provider(sp_timestamp::InherentDataProvider)
		.unwrap();
	let (mut telemetry, telemetry_worker_handle) = params.other;

	let polkadot_full_node = cumulus_client_service::build_polkadot_full_node(
		polkadot_config,
		collator_key.clone(),
		telemetry_worker_handle,
	)
	.map_err(|e| match e {
		polkadot_service::Error::Sub(x) => x,
		s => format!("{}", s).into(),
	})?;

	let client = params.client.clone();
	let backend = params.backend.clone();
	let block_announce_validator = build_block_announce_validator(
		polkadot_full_node.client.clone(),
		id,
		Box::new(polkadot_full_node.network.clone()),
		polkadot_full_node.backend.clone(),
	);

	let prometheus_registry = parachain_config.prometheus_registry().cloned();
	let transaction_pool = params.transaction_pool.clone();
	let mut task_manager = params.task_manager;
	let import_queue = params.import_queue;
	let (network, network_status_sinks, system_rpc_tx, start_network) =
		sc_service::build_network(sc_service::BuildNetworkParams {
			config: &parachain_config,
			client: client.clone(),
			transaction_pool: transaction_pool.clone(),
			spawn_handle: task_manager.spawn_handle(),
			import_queue,
			on_demand: None,
			block_announce_validator_builder: Some(Box::new(|_| block_announce_validator)),
		})?;

	if parachain_config.offchain_worker.enabled {
		sc_service::build_offchain_workers(
			&parachain_config,
			task_manager.spawn_handle(),
			client.clone(),
			network.clone(),
		);
	}

	let rpc_client = client.clone();
	let rpc_extensions_builder = Box::new(move |_, _| rpc_ext_builder(rpc_client.clone()));

	// FIXME: or this?
	// let rpc_extensions_builder = {
	// 	let client = client.clone();
	// 	let transaction_pool = transaction_pool.clone();
	//
	// 	Box::new(move |deny_unsafe, _| -> acala_rpc::RpcExtension {
	// 		let deps = acala_rpc::FullDeps {
	// 			client: client.clone(),
	// 			pool: transaction_pool.clone(),
	// 			deny_unsafe,
	// 		};
	//
	// 		acala_rpc::create_full(deps)
	// 	})
	// };

	sc_service::spawn_tasks(sc_service::SpawnTasksParams {
		on_demand: None,
		remote_blockchain: None,
		rpc_extensions_builder,
		client: client.clone(),
		transaction_pool: transaction_pool.clone(),
		task_manager: &mut task_manager,
		config: parachain_config,
		keystore: params.keystore_container.sync_keystore(),
		backend: backend.clone(),
		network: network.clone(),
		network_status_sinks,
		system_rpc_tx,
		telemetry: telemetry.as_mut(),
	})?;

	let announce_block = {
		let network = network.clone();
		Arc::new(move |hash, data| network.announce_block(hash, data))
	};

	if validator {
		let proposer_factory = sc_basic_authorship::ProposerFactory::with_proof_recording(
			task_manager.spawn_handle(),
			client.clone(),
			transaction_pool,
			prometheus_registry.as_ref(),
			telemetry.as_ref().map(|x| x.handle()),
		);
		let spawner = task_manager.spawn_handle();

		let parachain_consensus = build_relay_chain_consensus(BuildRelayChainConsensusParams {
			para_id: id,
			proposer_factory,
			inherent_data_providers: params.inherent_data_providers,
			block_import: client.clone(),
			relay_chain_client: polkadot_full_node.client.clone(),
			relay_chain_backend: polkadot_full_node.backend.clone(),
		});

		let params = StartCollatorParams {
			para_id: id,
			block_status: client.clone(),
			announce_block,
			client: client.clone(),
			task_manager: &mut task_manager,
			collator_key,
			relay_chain_full_node: polkadot_full_node,
			spawner,
			backend,
			parachain_consensus,
		};

		start_collator(params).await?;
	} else {
		let params = StartFullNodeParams {
			client: client.clone(),
			announce_block,
			task_manager: &mut task_manager,
			para_id: id,
			polkadot_full_node,
		};

		start_full_node(params)?;
	}

	start_network.start_network();

	Ok((task_manager, client))
}

/// Start a normal parachain node.
pub async fn start_node<RuntimeApi, Executor>(
	parachain_config: Configuration,
	collator_key: CollatorPair,
	polkadot_config: Configuration,
	id: ParaId,
	validator: bool,
) -> sc_service::error::Result<(TaskManager, Arc<FullClient<RuntimeApi, Executor>>)>
where
	RuntimeApi: ConstructRuntimeApi<Block, FullClient<RuntimeApi, Executor>> + Send + Sync + 'static,
	RuntimeApi: ConstructRuntimeApi<Block, FullClient<RuntimeApi, Executor>> + Send + Sync + 'static,
	RuntimeApi::RuntimeApi: RuntimeApiCollection<StateBackend = sc_client_api::StateBackendFor<FullBackend, Block>>,
	RuntimeApi::RuntimeApi: sp_consensus_aura::AuraApi<Block, AuraId>,
	Executor: NativeExecutionDispatch + 'static,
{
	start_node_impl(parachain_config, collator_key, polkadot_config, id, validator, |_| {
		Default::default()
	})
	.await
}
//
// // don't need this fns ?
// fn remote_keystore(_url: &str) -> Result<Arc<LocalKeystore>, &'static str> {
// 	// FIXME: actual keystore to be implemented here
// 	//        must return a real type (NOT `LocalKeystore`) that
// 	//        implements `CryptoStore` and `SyncCryptoStore`
// 	Err("Remote Keystore not supported.")
// }
//
// /// Builds a new service for a full client.
// pub fn new_full(mut config: Configuration) -> Result<TaskManager, ServiceError> {
// 	let sc_service::PartialComponents {
// 		client,
// 		backend,
// 		mut task_manager,
// 		import_queue,
// 		mut keystore_container,
// 		select_chain,
// 		transaction_pool,
// 		inherent_data_providers,
// 		other: (block_import, grandpa_link),
// 	} = new_partial(&config)?;
//
// 	if let Some(url) = &config.keystore_remote {
// 		match remote_keystore(url) {
// 			Ok(k) => keystore_container.set_remote_keystore(k),
// 			Err(e) => {
// 				return Err(ServiceError::Other(format!(
// 					"Error hooking up remote keystore for {}: {}",
// 					url, e
// 				)))
// 			}
// 		};
// 	}
//
// 	config
// 		.network
// 		.extra_sets
// 		.push(sc_finality_grandpa::grandpa_peers_set_config());
//
// 	let (network, network_status_sinks, system_rpc_tx, network_starter) =
// 		sc_service::build_network(sc_service::BuildNetworkParams {
// 			config: &config,
// 			client: client.clone(),
// 			transaction_pool: transaction_pool.clone(),
// 			spawn_handle: task_manager.spawn_handle(),
// 			import_queue,
// 			on_demand: None,
// 			block_announce_validator_builder: None,
// 		})?;
//
// 	if config.offchain_worker.enabled {
// 		sc_service::build_offchain_workers(
// 			&config,
// 			backend.clone(),
// 			task_manager.spawn_handle(),
// 			client.clone(),
// 			network.clone(),
// 		);
// 	}
//
// 	let role = config.role.clone();
// 	let force_authoring = config.force_authoring;
// 	let backoff_authoring_blocks: Option<()> = None;
// 	let name = config.network.node_name.clone();
// 	let enable_grandpa = !config.disable_grandpa;
// 	let prometheus_registry = config.prometheus_registry().cloned();
//
// 	let rpc_extensions_builder = {
// 		let client = client.clone();
// 		let pool = transaction_pool.clone();
//
// 		Box::new(move |deny_unsafe, _| {
// 			let deps = minterest_rpc::FullDeps {
// 				client: client.clone(),
// 				pool: pool.clone(),
// 				deny_unsafe,
// 			};
//
// 			minterest_rpc::create_full(deps)
// 		})
// 	};
//
// 	let (_rpc_handlers, telemetry_connection_notifier) =
// sc_service::spawn_tasks(sc_service::SpawnTasksParams { 		network: network.clone(),
// 		client: client.clone(),
// 		keystore: keystore_container.sync_keystore(),
// 		task_manager: &mut task_manager,
// 		transaction_pool: transaction_pool.clone(),
// 		rpc_extensions_builder,
// 		on_demand: None,
// 		remote_blockchain: None,
// 		backend,
// 		network_status_sinks,
// 		system_rpc_tx,
// 		config,
// 	})?;
//
// 	if role.is_authority() {
// 		let proposer = sc_basic_authorship::ProposerFactory::new(
// 			task_manager.spawn_handle(),
// 			client.clone(),
// 			transaction_pool,
// 			prometheus_registry.as_ref(),
// 		);
//
// 		let can_author_with = sp_consensus::CanAuthorWithNativeVersion::new(client.executor().clone());
//
// 		let aura = sc_consensus_aura::start_aura::<_, _, _, _, _, AuraPair, _, _, _, _>(
// 			sc_consensus_aura::slot_duration(&*client)?,
// 			client.clone(),
// 			select_chain,
// 			block_import,
// 			proposer,
// 			network.clone(),
// 			inherent_data_providers,
// 			force_authoring,
// 			backoff_authoring_blocks,
// 			keystore_container.sync_keystore(),
// 			can_author_with,
// 		)?;
//
// 		// the AURA authoring task is considered essential, i.e. if it
// 		// fails we take down the service with it.
// 		task_manager.spawn_essential_handle().spawn_blocking("aura", aura);
// 	}
//
// 	// if the node isn't actively participating in consensus then it doesn't
// 	// need a keystore, regardless of which protocol we use below.
// 	let keystore = if role.is_authority() {
// 		Some(keystore_container.sync_keystore())
// 	} else {
// 		None
// 	};
//
// 	let grandpa_config = sc_finality_grandpa::Config {
// 		// FIXME #1578 make this available through chainspec
// 		gossip_duration: Duration::from_millis(333),
// 		justification_period: 512,
// 		name: Some(name),
// 		observer_enabled: false,
// 		keystore,
// 		is_authority: role.is_network_authority(),
// 	};
//
// 	if enable_grandpa {
// 		// start the full GRANDPA voter
// 		// NOTE: non-authorities could run the GRANDPA observer protocol, but at
// 		// this point the full voter should provide better guarantees of block
// 		// and vote data availability than the observer. The observer has not
// 		// been tested extensively yet and having most nodes in a network run it
// 		// could lead to finality stalls.
// 		let grandpa_config = sc_finality_grandpa::GrandpaParams {
// 			config: grandpa_config,
// 			link: grandpa_link,
// 			network,
// 			telemetry_on_connect: telemetry_connection_notifier.map(|x| x.on_connect_stream()),
// 			voting_rule: sc_finality_grandpa::VotingRulesBuilder::default().build(),
// 			prometheus_registry,
// 			shared_voter_state: SharedVoterState::empty(),
// 		};
//
// 		// the GRANDPA voter task is considered infallible, i.e.
// 		// if it fails we take down the service with it.
// 		task_manager
// 			.spawn_essential_handle()
// 			.spawn_blocking("grandpa-voter", sc_finality_grandpa::run_grandpa_voter(grandpa_config)?);
// 	}
//
// 	network_starter.start_network();
// 	Ok(task_manager)
// }
//
// /// Builds a new service for a light client.
// pub fn new_light(mut config: Configuration) -> Result<TaskManager, ServiceError> {
// 	let (client, backend, keystore_container, mut task_manager, on_demand) =
// 		sc_service::new_light_parts::<Block, RuntimeApi, Executor>(&config)?;
//
// 	config
// 		.network
// 		.extra_sets
// 		.push(sc_finality_grandpa::grandpa_peers_set_config());
//
// 	let select_chain = sc_consensus::LongestChain::new(backend.clone());
//
// 	let transaction_pool = Arc::new(sc_transaction_pool::BasicPool::new_light(
// 		config.transaction_pool.clone(),
// 		config.prometheus_registry(),
// 		task_manager.spawn_handle(),
// 		client.clone(),
// 		on_demand.clone(),
// 	));
//
// 	let (grandpa_block_import, _) =
// 		sc_finality_grandpa::block_import(client.clone(), &(client.clone() as Arc<_>), select_chain)?;
//
// 	let aura_block_import =
// 		sc_consensus_aura::AuraBlockImport::<_, _, _, AuraPair>::new(grandpa_block_import.clone(),
// client.clone());
//
// 	let import_queue = sc_consensus_aura::import_queue::<_, _, _, AuraPair, _, _>(
// 		sc_consensus_aura::slot_duration(&*client)?,
// 		aura_block_import,
// 		Some(Box::new(grandpa_block_import)),
// 		client.clone(),
// 		InherentDataProviders::new(),
// 		&task_manager.spawn_handle(),
// 		config.prometheus_registry(),
// 		sp_consensus::NeverCanAuthor,
// 	)?;
//
// 	let (network, network_status_sinks, system_rpc_tx, network_starter) =
// 		sc_service::build_network(sc_service::BuildNetworkParams {
// 			config: &config,
// 			client: client.clone(),
// 			transaction_pool: transaction_pool.clone(),
// 			spawn_handle: task_manager.spawn_handle(),
// 			import_queue,
// 			on_demand: Some(on_demand.clone()),
// 			block_announce_validator_builder: None,
// 		})?;
//
// 	if config.offchain_worker.enabled {
// 		sc_service::build_offchain_workers(
// 			&config,
// 			backend.clone(),
// 			task_manager.spawn_handle(),
// 			client.clone(),
// 			network.clone(),
// 		);
// 	}
//
// 	sc_service::spawn_tasks(sc_service::SpawnTasksParams {
// 		remote_blockchain: Some(backend.remote_blockchain()),
// 		transaction_pool,
// 		task_manager: &mut task_manager,
// 		on_demand: Some(on_demand),
// 		rpc_extensions_builder: Box::new(|_, _| ()),
// 		config,
// 		client,
// 		keystore: keystore_container.sync_keystore(),
// 		backend,
// 		network,
// 		network_status_sinks,
// 		system_rpc_tx,
// 	})?;
//
// 	network_starter.start_network();
//
// 	Ok(task_manager)
// }
