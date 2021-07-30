// Copyright 2019-2021 Parity Technologies (UK) Ltd.
// This file is part of Cumulus.

// Cumulus is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Cumulus is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Cumulus.  If not, see <http://www.gnu.org/licenses/>.

use cumulus_client_consensus_aura::{build_aura_consensus, BuildAuraConsensusParams, SlotProportion};
use cumulus_client_consensus_common::ParachainConsensus;
use cumulus_client_network::build_block_announce_validator;
use cumulus_client_service::{
	prepare_node_config, start_collator, start_full_node, StartCollatorParams, StartFullNodeParams,
};
use cumulus_primitives_core::ParaId;
use cumulus_primitives_parachain_inherent::MockValidationDataInherentDataProvider;

use sc_client_api::ExecutorProvider;
use sc_consensus::LongestChain;
use sc_consensus_aura::{ImportQueueParams, StartAuraParams};
use sc_executor::native_executor_instance;
use sc_finality_grandpa::SharedVoterState;
use sc_keystore::LocalKeystore;
use sc_network::NetworkService;
use sc_service::{
	error::Error as ServiceError, Configuration, PartialComponents, Role, TFullBackend, TFullClient, TaskManager,
};
use sc_telemetry::{Telemetry, TelemetryHandle, TelemetryWorker, TelemetryWorkerHandle};
use sp_api::ConstructRuntimeApi;
use sp_consensus::SlotData;
use sp_consensus_aura::sr25519::AuthorityPair as AuraPair;
use sp_keystore::SyncCryptoStorePtr;
use sp_runtime::traits::BlakeTwo256;
use std::{sync::Arc, time::Duration};
use substrate_prometheus_endpoint::Registry;

pub use sc_executor::NativeExecutor;

pub mod chain_spec;

type BlockNumber = u32;
type Header = sp_runtime::generic::Header<BlockNumber, sp_runtime::traits::BlakeTwo256>;
pub type Block = sp_runtime::generic::Block<Header, sp_runtime::OpaqueExtrinsic>;
type Hash = sp_core::H256;

type FullBackend = TFullBackend<Block>;

/// Maybe Standalone full select chain.
type MaybeFullSelectChain = Option<LongestChain<FullBackend, Block>>;

type MaybeGrandpaImportLink<RuntimeApi, Executor> = Option<(
	sc_finality_grandpa::GrandpaBlockImport<
		FullBackend,
		Block,
		TFullClient<Block, RuntimeApi, Executor>,
		LongestChain<FullBackend, Block>,
	>,
	sc_finality_grandpa::LinkHalf<Block, TFullClient<Block, RuntimeApi, Executor>, LongestChain<FullBackend, Block>>,
)>;

native_executor_instance!(
	pub ParachainRuntimeExecutor,
	minterest_parachain_runtime::api::dispatch,
	minterest_parachain_runtime::native_version,
);

native_executor_instance!(
	pub StandaloneRuntimeExecutor,
	minterest_standalone_runtime::api::dispatch,
	minterest_standalone_runtime::native_version,
	frame_benchmarking::benchmarking::HostFunctions,
);

/// Starts a `ServiceBuilder` for a full service.
///
/// Use this macro if you don't actually need the full service, but just the builder in order to
/// be able to perform chain operations.
pub fn new_partial<RuntimeApi, Executor, BIQ>(
	config: &Configuration,
	build_import_queue: BIQ,
	standalone: bool,
) -> Result<
	PartialComponents<
		TFullClient<Block, RuntimeApi, Executor>,
		FullBackend,
		MaybeFullSelectChain,
		sp_consensus::DefaultImportQueue<Block, TFullClient<Block, RuntimeApi, Executor>>,
		sc_transaction_pool::FullPool<Block, TFullClient<Block, RuntimeApi, Executor>>,
		(
			MaybeGrandpaImportLink<RuntimeApi, Executor>,
			Option<Telemetry>,
			Option<TelemetryWorkerHandle>,
		),
	>,
	sc_service::Error,
>
where
	RuntimeApi: ConstructRuntimeApi<Block, TFullClient<Block, RuntimeApi, Executor>> + Send + Sync + 'static,
	RuntimeApi::RuntimeApi: sp_transaction_pool::runtime_api::TaggedTransactionQueue<Block>
		+ sp_api::Metadata<Block>
		+ sp_session::SessionKeys<Block>
		+ sp_api::ApiExt<Block, StateBackend = sc_client_api::StateBackendFor<FullBackend, Block>>
		+ sp_offchain::OffchainWorkerApi<Block>
		+ sp_block_builder::BlockBuilder<Block>,
	sc_client_api::StateBackendFor<FullBackend, Block>: sp_api::StateBackend<BlakeTwo256>,
	Executor: sc_executor::NativeExecutionDispatch + 'static,
	BIQ: FnOnce(
		Arc<TFullClient<Block, RuntimeApi, Executor>>,
		&Configuration,
		&MaybeGrandpaImportLink<RuntimeApi, Executor>,
		Option<TelemetryHandle>,
		&TaskManager,
	) -> Result<
		sp_consensus::DefaultImportQueue<Block, TFullClient<Block, RuntimeApi, Executor>>,
		sc_service::Error,
	>,
{
	let telemetry = config
		.telemetry_endpoints
		.clone()
		.filter(|x| !x.is_empty())
		.map(|endpoints| -> Result<_, sc_telemetry::Error> {
			let worker = TelemetryWorker::new(16)?;
			let telemetry = worker.handle().new_telemetry(endpoints);
			Ok((worker, telemetry))
		})
		.transpose()?;

	let (client, backend, keystore_container, task_manager) = sc_service::new_full_parts::<Block, RuntimeApi, Executor>(
		&config,
		telemetry.as_ref().map(|(_, telemetry)| telemetry.handle()),
	)?;
	let client = Arc::new(client);

	let telemetry_worker_handle = telemetry.as_ref().map(|(worker, _)| worker.handle());

	let telemetry = telemetry.map(|(worker, telemetry)| {
		task_manager.spawn_handle().spawn("telemetry", worker.run());
		telemetry
	});

	let transaction_pool = sc_transaction_pool::BasicPool::new_full(
		config.transaction_pool.clone(),
		config.role.is_authority().into(),
		config.prometheus_registry(),
		task_manager.spawn_essential_handle(),
		client.clone(),
	);

	let (select_chain, maybe_grandpa) = if standalone {
		let select_chain = LongestChain::new(backend.clone());
		let grandpa = sc_finality_grandpa::block_import(
			client.clone(),
			&(client.clone() as Arc<_>),
			select_chain.clone(),
			telemetry.as_ref().map(|x| x.handle()),
		)?;
		(Some(select_chain), Some(grandpa))
	} else {
		(None, None)
	};

	let import_queue = build_import_queue(
		client.clone(),
		config,
		&maybe_grandpa,
		telemetry.as_ref().map(|telemetry| telemetry.handle()),
		&task_manager,
	)?;

	let params = PartialComponents {
		backend,
		client,
		import_queue,
		keystore_container,
		task_manager,
		transaction_pool,
		select_chain,
		other: (maybe_grandpa, telemetry, telemetry_worker_handle),
	};

	Ok(params)
}

/// Start a node with the given parachain `Configuration` and relay chain `Configuration`.
///
/// This is the actual implementation that is abstract over the executor and the runtime api.
#[sc_tracing::logging::prefix_logs_with("Parachain")]
async fn start_parachain_node_impl<RuntimeApi, Executor, RB, BIQ, BIC>(
	parachain_config: Configuration,
	polkadot_config: Configuration,
	id: ParaId,
	rpc_ext_builder: RB,
	build_import_queue: BIQ,
	build_consensus: BIC,
) -> sc_service::error::Result<(TaskManager, Arc<TFullClient<Block, RuntimeApi, Executor>>)>
where
	RuntimeApi: ConstructRuntimeApi<Block, TFullClient<Block, RuntimeApi, Executor>> + Send + Sync + 'static,
	RuntimeApi::RuntimeApi: sp_transaction_pool::runtime_api::TaggedTransactionQueue<Block>
		+ sp_api::Metadata<Block>
		+ sp_session::SessionKeys<Block>
		+ sp_api::ApiExt<Block, StateBackend = sc_client_api::StateBackendFor<FullBackend, Block>>
		+ sp_offchain::OffchainWorkerApi<Block>
		+ sp_block_builder::BlockBuilder<Block>
		+ cumulus_primitives_core::CollectCollationInfo<Block>,
	sc_client_api::StateBackendFor<FullBackend, Block>: sp_api::StateBackend<BlakeTwo256>,
	Executor: sc_executor::NativeExecutionDispatch + 'static,
	RB: Fn(Arc<TFullClient<Block, RuntimeApi, Executor>>) -> jsonrpc_core::IoHandler<sc_rpc::Metadata> + Send + 'static,
	BIQ: FnOnce(
		Arc<TFullClient<Block, RuntimeApi, Executor>>,
		&Configuration,
		&MaybeGrandpaImportLink<RuntimeApi, Executor>,
		Option<TelemetryHandle>,
		&TaskManager,
	) -> Result<
		sp_consensus::DefaultImportQueue<Block, TFullClient<Block, RuntimeApi, Executor>>,
		sc_service::Error,
	>,
	BIC: FnOnce(
		Arc<TFullClient<Block, RuntimeApi, Executor>>,
		Option<&Registry>,
		Option<TelemetryHandle>,
		&TaskManager,
		&polkadot_service::NewFull<polkadot_service::Client>,
		Arc<sc_transaction_pool::FullPool<Block, TFullClient<Block, RuntimeApi, Executor>>>,
		Arc<NetworkService<Block, Hash>>,
		SyncCryptoStorePtr,
		bool,
	) -> Result<Box<dyn ParachainConsensus<Block>>, sc_service::Error>,
{
	if matches!(parachain_config.role, Role::Light) {
		return Err("Light client not supported!".into());
	}

	let parachain_config = prepare_node_config(parachain_config);

	let params = new_partial::<RuntimeApi, Executor, BIQ>(&parachain_config, build_import_queue, false)?;
	let (_, mut telemetry, telemetry_worker_handle) = params.other;

	let relay_chain_full_node =
		cumulus_client_service::build_polkadot_full_node(polkadot_config, telemetry_worker_handle).map_err(
			|e| match e {
				polkadot_service::Error::Sub(x) => x,
				s => format!("{}", s).into(),
			},
		)?;

	let client = params.client.clone();
	let backend = params.backend.clone();
	let block_announce_validator = build_block_announce_validator(
		relay_chain_full_node.client.clone(),
		id,
		Box::new(relay_chain_full_node.network.clone()),
		relay_chain_full_node.backend.clone(),
	);

	let force_authoring = parachain_config.force_authoring;
	let validator = parachain_config.role.is_authority();
	let prometheus_registry = parachain_config.prometheus_registry().cloned();
	let transaction_pool = params.transaction_pool.clone();
	let mut task_manager = params.task_manager;
	let import_queue = cumulus_client_service::SharedImportQueue::new(params.import_queue);
	let (network, system_rpc_tx, start_network) = sc_service::build_network(sc_service::BuildNetworkParams {
		config: &parachain_config,
		client: client.clone(),
		transaction_pool: transaction_pool.clone(),
		spawn_handle: task_manager.spawn_handle(),
		import_queue: import_queue.clone(),
		on_demand: None,
		block_announce_validator_builder: Some(Box::new(|_| block_announce_validator)),
	})?;

	let rpc_client = client.clone();
	let rpc_extensions_builder = Box::new(move |_, _| rpc_ext_builder(rpc_client.clone()));

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
		system_rpc_tx,
		telemetry: telemetry.as_mut(),
	})?;

	let announce_block = {
		let network = network.clone();
		Arc::new(move |hash, data| network.announce_block(hash, data))
	};

	if validator {
		let parachain_consensus = build_consensus(
			client.clone(),
			prometheus_registry.as_ref(),
			telemetry.as_ref().map(|t| t.handle()),
			&task_manager,
			&relay_chain_full_node,
			transaction_pool,
			network,
			params.keystore_container.sync_keystore(),
			force_authoring,
		)?;

		let spawner = task_manager.spawn_handle();

		let params = StartCollatorParams {
			para_id: id,
			block_status: client.clone(),
			announce_block,
			client: client.clone(),
			task_manager: &mut task_manager,
			relay_chain_full_node,
			spawner,
			parachain_consensus,
			import_queue,
		};

		start_collator(params).await?;
	} else {
		let params = StartFullNodeParams {
			client: client.clone(),
			announce_block,
			task_manager: &mut task_manager,
			para_id: id,
			relay_chain_full_node,
		};

		start_full_node(params)?;
	}

	start_network.start_network();

	Ok((task_manager, client))
}

/// Build the import queue for the rococo parachain runtime.
pub fn parachain_build_import_queue(
	client: Arc<TFullClient<Block, minterest_parachain_runtime::RuntimeApi, ParachainRuntimeExecutor>>,
	config: &Configuration,
	_maybe_grandpa: &MaybeGrandpaImportLink<minterest_parachain_runtime::RuntimeApi, ParachainRuntimeExecutor>,
	telemetry: Option<TelemetryHandle>,
	task_manager: &TaskManager,
) -> Result<
	sp_consensus::DefaultImportQueue<
		Block,
		TFullClient<Block, minterest_parachain_runtime::RuntimeApi, ParachainRuntimeExecutor>,
	>,
	sc_service::Error,
> {
	let slot_duration = cumulus_client_consensus_aura::slot_duration(&*client)?;

	cumulus_client_consensus_aura::import_queue::<sp_consensus_aura::sr25519::AuthorityPair, _, _, _, _, _, _>(
		cumulus_client_consensus_aura::ImportQueueParams {
			block_import: client.clone(),
			client: client.clone(),
			create_inherent_data_providers: move |_, _| async move {
				let time = sp_timestamp::InherentDataProvider::from_system_time();

				let slot = sp_consensus_aura::inherents::InherentDataProvider::from_timestamp_and_duration(
					*time,
					slot_duration.slot_duration(),
				);

				Ok((time, slot))
			},
			registry: config.prometheus_registry().clone(),
			can_author_with: sp_consensus::CanAuthorWithNativeVersion::new(client.executor().clone()),
			spawner: &task_manager.spawn_essential_handle(),
			telemetry,
		},
	)
	.map_err(Into::into)
}

/// Start a rococo parachain node.
pub async fn start_parachain_node(
	parachain_config: Configuration,
	polkadot_config: Configuration,
	para_id: ParaId,
) -> sc_service::error::Result<(
	TaskManager,
	Arc<TFullClient<Block, minterest_parachain_runtime::RuntimeApi, ParachainRuntimeExecutor>>,
)> {
	start_parachain_node_impl::<minterest_parachain_runtime::RuntimeApi, ParachainRuntimeExecutor, _, _, _>(
		parachain_config,
		polkadot_config,
		para_id,
		|_| Default::default(),
		parachain_build_import_queue,
		|client,
		 prometheus_registry,
		 telemetry,
		 task_manager,
		 relay_chain_node,
		 transaction_pool,
		 sync_oracle,
		 keystore,
		 force_authoring| {
			let slot_duration = cumulus_client_consensus_aura::slot_duration(&*client)?;

			let proposer_factory = sc_basic_authorship::ProposerFactory::with_proof_recording(
				task_manager.spawn_handle(),
				client.clone(),
				transaction_pool,
				prometheus_registry.clone(),
				telemetry.clone(),
			);

			let relay_chain_backend = relay_chain_node.backend.clone();
			let relay_chain_client = relay_chain_node.client.clone();
			Ok(build_aura_consensus::<
				sp_consensus_aura::sr25519::AuthorityPair,
				_,
				_,
				_,
				_,
				_,
				_,
				_,
				_,
				_,
			>(BuildAuraConsensusParams {
				proposer_factory,
				create_inherent_data_providers: move |_, (relay_parent, validation_data)| {
					let parachain_inherent =
						cumulus_primitives_parachain_inherent::ParachainInherentData::create_at_with_client(
							relay_parent,
							&relay_chain_client,
							&*relay_chain_backend,
							&validation_data,
							para_id,
						);
					async move {
						let time = sp_timestamp::InherentDataProvider::from_system_time();

						let slot = sp_consensus_aura::inherents::InherentDataProvider::from_timestamp_and_duration(
							*time,
							slot_duration.slot_duration(),
						);

						let parachain_inherent = parachain_inherent.ok_or_else(|| {
							Box::<dyn std::error::Error + Send + Sync>::from("Failed to create parachain inherent")
						})?;
						Ok((time, slot, parachain_inherent))
					}
				},
				block_import: client.clone(),
				relay_chain_client: relay_chain_node.client.clone(),
				relay_chain_backend: relay_chain_node.backend.clone(),
				para_client: client.clone(),
				backoff_authoring_blocks: Option::<()>::None,
				sync_oracle,
				keystore,
				force_authoring,
				slot_duration,
				// We got around 500ms for proposing
				block_proposal_slot_portion: SlotProportion::new(1f32 / 24f32),
				// And a maximum of 750ms if slots are skipped
				max_block_proposal_slot_portion: Some(SlotProportion::new(1f32 / 16f32)),
				telemetry,
			}))
		},
	)
	.await
}

/// Build the import queue for the rococo parachain runtime.
pub fn standalone_build_import_queue(
	client: Arc<TFullClient<Block, minterest_standalone_runtime::RuntimeApi, StandaloneRuntimeExecutor>>,
	config: &Configuration,
	maybe_grandpa: &MaybeGrandpaImportLink<minterest_standalone_runtime::RuntimeApi, StandaloneRuntimeExecutor>,
	telemetry: Option<TelemetryHandle>,
	task_manager: &TaskManager,
) -> Result<
	sp_consensus::DefaultImportQueue<
		Block,
		TFullClient<Block, minterest_standalone_runtime::RuntimeApi, StandaloneRuntimeExecutor>,
	>,
	sc_service::Error,
> {
	let slot_duration = sc_consensus_aura::slot_duration(&*client)?.slot_duration();

	let grandpa_block_import = maybe_grandpa
		.as_ref()
		.map(|g| g.0.clone())
		.expect("In Standalone mode `maybe_grandpa` will have some value");

	sc_consensus_aura::import_queue::<AuraPair, _, _, _, _, _, _>(ImportQueueParams {
		block_import: grandpa_block_import.clone(),
		justification_import: Some(Box::new(grandpa_block_import)),
		client: client.clone(),
		create_inherent_data_providers: move |_, ()| async move {
			let timestamp = sp_timestamp::InherentDataProvider::from_system_time();

			let slot = sp_consensus_aura::inherents::InherentDataProvider::from_timestamp_and_duration(
				*timestamp,
				slot_duration,
			);

			Ok((timestamp, slot))
		},
		spawner: &task_manager.spawn_essential_handle(),
		registry: config.prometheus_registry().clone(),
		can_author_with: sp_consensus::CanAuthorWithNativeVersion::new(client.executor().clone()),
		check_for_equivocation: Default::default(),
		telemetry,
	})
	.map_err(Into::into)
}

fn remote_keystore(_url: &str) -> Result<Arc<LocalKeystore>, &'static str> {
	// FIXME: actual keystore to be implemented here
	//        must return a real type (NOT `LocalKeystore`) that
	//        implements `CryptoStore` and `SyncCryptoStore`
	Err("Remote Keystore not supported.")
}

pub fn start_standalone_node(mut config: Configuration) -> Result<TaskManager, ServiceError> {
	let sc_service::PartialComponents {
		client,
		backend,
		mut task_manager,
		import_queue,
		mut keystore_container,
		select_chain: maybe_select_chain,
		transaction_pool,
		other: (maybe_grandpa, mut telemetry, _),
	} = new_partial(&config, standalone_build_import_queue, true)?;

	if let Some(url) = &config.keystore_remote {
		match remote_keystore(url) {
			Ok(k) => keystore_container.set_remote_keystore(k),
			Err(e) => {
				return Err(ServiceError::Other(format!(
					"Error hooking up remote keystore for {}: {}",
					url, e
				)))
			}
		};
	}

	let (block_import, grandpa_link) = maybe_grandpa.expect("`maybe_grandpa` should be some on standalone");

	config
		.network
		.extra_sets
		.push(sc_finality_grandpa::grandpa_peers_set_config());

	let (network, system_rpc_tx, network_starter) = sc_service::build_network(sc_service::BuildNetworkParams {
		config: &config,
		client: client.clone(),
		transaction_pool: transaction_pool.clone(),
		spawn_handle: task_manager.spawn_handle(),
		import_queue,
		on_demand: None,
		block_announce_validator_builder: None,
	})?;

	if config.offchain_worker.enabled {
		sc_service::build_offchain_workers(&config, task_manager.spawn_handle(), client.clone(), network.clone());
	}

	let role = config.role.clone();
	let force_authoring = config.force_authoring;
	let backoff_authoring_blocks: Option<()> = None;
	let name = config.network.node_name.clone();
	let enable_grandpa = !config.disable_grandpa;
	let prometheus_registry = config.prometheus_registry().cloned();

	let rpc_extensions_builder = {
		let client = client.clone();
		let pool = transaction_pool.clone();

		Box::new(move |deny_unsafe, _| {
			let deps = minterest_rpc::FullDeps {
				client: client.clone(),
				pool: pool.clone(),
				deny_unsafe,
			};

			minterest_rpc::create_full(deps)
		})
	};

	let _rpc_handlers = sc_service::spawn_tasks(sc_service::SpawnTasksParams {
		network: network.clone(),
		client: client.clone(),
		keystore: keystore_container.sync_keystore(),
		task_manager: &mut task_manager,
		transaction_pool: transaction_pool.clone(),
		rpc_extensions_builder,
		on_demand: None,
		remote_blockchain: None,
		backend,
		system_rpc_tx,
		config,
		telemetry: telemetry.as_mut(),
	})?;

	let select_chain = maybe_select_chain.expect("In Standalone mode `new_partial` will return some `select_chain`");

	if role.is_authority() {
		let proposer_factory = sc_basic_authorship::ProposerFactory::new(
			task_manager.spawn_handle(),
			client.clone(),
			transaction_pool,
			prometheus_registry.as_ref(),
			telemetry.as_ref().map(|x| x.handle()),
		);

		let can_author_with = sp_consensus::CanAuthorWithNativeVersion::new(client.executor().clone());

		let slot_duration = sc_consensus_aura::slot_duration(&*client)?;
		let raw_slot_duration = slot_duration.slot_duration();

		let aura = sc_consensus_aura::start_aura::<AuraPair, _, _, _, _, _, _, _, _, _, _, _>(StartAuraParams {
			slot_duration,
			client: client.clone(),
			select_chain,
			block_import: block_import,
			proposer_factory,
			create_inherent_data_providers: move |_, ()| async move {
				let timestamp = sp_timestamp::InherentDataProvider::from_system_time();

				let slot = sp_consensus_aura::inherents::InherentDataProvider::from_timestamp_and_duration(
					*timestamp,
					raw_slot_duration,
				);
				let inherent_data = MockValidationDataInherentDataProvider {
					current_para_block: 0,
					relay_offset: 1000,
					relay_blocks_per_para_block: 2,
				};

				Ok((timestamp, slot, inherent_data))
			},
			force_authoring,
			backoff_authoring_blocks,
			keystore: keystore_container.sync_keystore(),
			can_author_with,
			sync_oracle: network.clone(),
			justification_sync_link: network.clone(),
			block_proposal_slot_portion: SlotProportion::new(2f32 / 3f32),
			max_block_proposal_slot_portion: Some(SlotProportion::new(1f32 / 16f32)),
			telemetry: telemetry.as_ref().map(|x| x.handle()),
		})?;

		// the AURA authoring task is considered essential, i.e. if it
		// fails we take down the service with it.
		task_manager.spawn_essential_handle().spawn_blocking("aura", aura);
	}

	// if the node isn't actively participating in consensus then it doesn't
	// need a keystore, regardless of which protocol we use below.
	let keystore = if role.is_authority() {
		Some(keystore_container.sync_keystore())
	} else {
		None
	};

	let grandpa_config = sc_finality_grandpa::Config {
		// FIXME #1578 make this available through chainspec
		gossip_duration: Duration::from_millis(333),
		justification_period: 512,
		name: Some(name),
		observer_enabled: false,
		keystore,
		local_role: role,
		telemetry: telemetry.as_ref().map(|x| x.handle()),
	};

	if enable_grandpa {
		// start the full GRANDPA voter
		// NOTE: non-authorities could run the GRANDPA observer protocol, but at
		// this point the full voter should provide better guarantees of block
		// and vote data availability than the observer. The observer has not
		// been tested extensively yet and having most nodes in a network run it
		// could lead to finality stalls.
		let grandpa_config = sc_finality_grandpa::GrandpaParams {
			config: grandpa_config,
			link: grandpa_link,
			network,
			voting_rule: sc_finality_grandpa::VotingRulesBuilder::default().build(),
			prometheus_registry,
			shared_voter_state: SharedVoterState::empty(),
			telemetry: telemetry.as_ref().map(|x| x.handle()),
		};

		// the GRANDPA voter task is considered infallible, i.e.
		// if it fails we take down the service with it.
		task_manager
			.spawn_essential_handle()
			.spawn_blocking("grandpa-voter", sc_finality_grandpa::run_grandpa_voter(grandpa_config)?);
	}

	network_starter.start_network();

	Ok(task_manager)
}
