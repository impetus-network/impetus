//! Babe + GRANDPA + authority-discovery consensus service wiring for impetus.
//!
//! This file mirrors the Aura service in [`super::aura`] but swaps Aura for
//! Babe in the consensus-specific blocks. The networking, RPC plumbing,
//! GRANDPA spawn, transaction pool, telemetry, prometheus, and offchain
//! worker registration are intentionally identical to the Aura path; only
//! the consensus-specific blocks differ:
//!
//! * `sc_consensus_aura::slot_duration(&*client)` is replaced with
//!   `sc_consensus_babe::configuration(&*client)` (a `BabeConfiguration`
//!   that exposes `.slot_duration()`).
//! * The block-import stack becomes `Frontier(Babe(Grandpa(Client)))` --
//!   `sc_consensus_babe::block_import` returns a `(BabeBlockImport, BabeLink)`
//!   pair and the `BabeLink` is shared between the import queue and the
//!   authoring loop.
//! * Authorship uses `sc_consensus_babe::start_babe` with `BabeParams`.
//! * On authority nodes, an authority-discovery DHT worker is spawned after
//!   networking is up.
//!
//! Manual-seal is intentionally NOT supported on the Babe path -- the dev
//! profile remains on Aura via [`super::aura`].
//!
//! Because Babe's `block_import` requires both the `select_chain` and a
//! transaction-pool factory (for equivocation reporting), the partial-build
//! is inlined here rather than reusing `common::new_partial`.

use std::{path::Path, sync::Arc, time::Duration};

use futures::prelude::*;
// Substrate
use sc_client_api::{Backend as BackendT, BlockBackend};
use sc_consensus::BasicQueue;
use sc_consensus_grandpa::{BlockNumberOps, GrandpaPruningFilter};
use sc_executor::HostFunctions as HostFunctionsT;
use sc_network::{event::Event, NetworkEventStream};
use sc_network_sync::strategy::warp::{WarpSyncConfig, WarpSyncProvider};
use sc_service::{error::Error as ServiceError, Configuration, TaskManager};
use sc_telemetry::TelemetryWorker;
use sc_transaction_pool_api::OffchainTransactionPoolFactory;
use sp_api::ConstructRuntimeApi;
use sp_core::{H256, U256};
use sp_runtime::traits::{Block as BlockT, NumberFor};
// Runtime
use runtime_common::{AccountId, Balance, Nonce};

use crate::{
	client::{BaseRuntimeApiCollection, FullClient},
	eth::{
		db_config_dir, new_frontier_partial, spawn_frontier_tasks, BackendType,
		EthCompatRuntimeApiCollection, EthConfiguration, FrontierBackend, FrontierBlockImport,
		FrontierPartialComponents, StorageOverrideHandler,
	},
};

use super::common::GRANDPA_JUSTIFICATION_PERIOD;

/// Runtime-API collection required by the Babe service path.
///
/// Unlike [`crate::client::RuntimeApiCollection`] which assumes Aura, this
/// trait pulls in `BabeApi` + `AuthorityDiscoveryApi` for impetus.
pub trait BabeRuntimeApiCollection<Block, AccountId, Nonce, Balance>:
	BaseRuntimeApiCollection<Block>
	+ EthCompatRuntimeApiCollection<Block>
	+ sp_consensus_babe::BabeApi<Block>
	+ sp_consensus_grandpa::GrandpaApi<Block>
	+ sp_authority_discovery::AuthorityDiscoveryApi<Block>
	+ frame_system_rpc_runtime_api::AccountNonceApi<Block, AccountId, Nonce>
	+ pallet_transaction_payment_rpc_runtime_api::TransactionPaymentApi<Block, Balance>
where
	Block: BlockT,
	AccountId: scale_codec::Codec,
	Nonce: scale_codec::Codec,
	Balance: scale_codec::Codec + sp_runtime::traits::MaybeDisplay,
{
}

impl<Block, AccountId, Nonce, Balance, Api>
	BabeRuntimeApiCollection<Block, AccountId, Nonce, Balance> for Api
where
	Block: BlockT,
	AccountId: scale_codec::Codec,
	Nonce: scale_codec::Codec,
	Balance: scale_codec::Codec + sp_runtime::traits::MaybeDisplay,
	Api: BaseRuntimeApiCollection<Block>
		+ EthCompatRuntimeApiCollection<Block>
		+ sp_consensus_babe::BabeApi<Block>
		+ sp_consensus_grandpa::GrandpaApi<Block>
		+ sp_authority_discovery::AuthorityDiscoveryApi<Block>
		+ frame_system_rpc_runtime_api::AccountNonceApi<Block, AccountId, Nonce>
		+ pallet_transaction_payment_rpc_runtime_api::TransactionPaymentApi<Block, Balance>,
{
}

/// Builds a new full Babe-powered service for impetus.
pub async fn new_full<B, RA, HF, NB, CT>(
	mut config: Configuration,
	eth_config: EthConfiguration,
) -> Result<TaskManager, ServiceError>
where
	B: BlockT<Hash = H256> + Unpin,
	NumberFor<B>: BlockNumberOps,
	<B as BlockT>::Header: Unpin,
	RA: ConstructRuntimeApi<B, FullClient<B, RA, HF>>,
	RA: Send + Sync + 'static,
	RA::RuntimeApi: BabeRuntimeApiCollection<B, AccountId, Nonce, Balance>,
	HF: HostFunctionsT + 'static,
	NB: sc_network::NetworkBackend<B, <B as BlockT>::Hash>,
	CT: fp_rpc::ConvertTransaction<<B as BlockT>::Extrinsic> + Default + Send + Sync + 'static,
{
	// -----------------------------------------------------------------
	// Partial build: telemetry, client, backend, select_chain, grandpa,
	// frontier backend, transaction pool, babe block-import + import queue.
	// -----------------------------------------------------------------

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

	let executor = sc_service::new_wasm_executor(&config.executor);

	let (client, backend, keystore_container, mut task_manager) =
		sc_service::new_full_parts_record_import::<B, RA, _>(
			&config,
			telemetry.as_ref().map(|(_, telemetry)| telemetry.handle()),
			executor,
			true,
			vec![Arc::new(GrandpaPruningFilter)],
		)?;
	let client = Arc::new(client);

	// Dev/single-node convenience: when the spec is `Development`, the node
	// runs as authority, and `IMPETUS_INJECT_ALL_DEV_KEYS=1` is set, inject
	// the four `//Alice..//Dave` session-key sets into the keystore so a
	// single process can claim slots on behalf of all four genesis
	// validators. Used by `cargo test` and single-node smoke runs.
	//
	// In a multi-node dev setup (one process per validator, each with its
	// own `--alice` / `--bob` / `--charlie` / `--dave`), the env var must
	// be unset so each node only holds its own key. Holding every key on
	// every node would author every slot in parallel and the rest of the
	// network would correctly flag it as equivocation.
	//
	// Production specs use `ChainType::Live` and skip this branch
	// unconditionally regardless of the env var.
	let inject_all_dev_keys = matches!(
		config.chain_spec.chain_type(),
		sc_chain_spec::ChainType::Development,
	) && std::env::var("IMPETUS_INJECT_ALL_DEV_KEYS")
		.map(|v| v == "1")
		.unwrap_or(false);
	if config.role.is_authority() && inject_all_dev_keys {
		use sp_core::crypto::key_types::{AUTHORITY_DISCOVERY, BABE, GRANDPA, IM_ONLINE};

		let keystore = keystore_container.keystore();
		let sr25519_key_types = [BABE, IM_ONLINE, AUTHORITY_DISCOVERY];
		for seed in ["//Alice", "//Bob", "//Charlie", "//Dave"] {
			for key_type in &sr25519_key_types {
				keystore
					.sr25519_generate_new(*key_type, Some(seed))
					.map_err(|e| ServiceError::Other(format!(
						"failed to insert {key_type:?} key for {seed}: {e:?}"
					)))?;
			}
			keystore
				.ed25519_generate_new(GRANDPA, Some(seed))
				.map_err(|e| ServiceError::Other(format!(
					"failed to insert GRANDPA key for {seed}: {e:?}"
				)))?;
		}
	}

	let mut telemetry = telemetry.map(|(worker, telemetry)| {
		task_manager
			.spawn_handle()
			.spawn("telemetry", None, worker.run());
		telemetry
	});

	let select_chain = sc_consensus::LongestChain::new(backend.clone());
	let (grandpa_block_import, grandpa_link) = sc_consensus_grandpa::block_import(
		client.clone(),
		GRANDPA_JUSTIFICATION_PERIOD,
		&client,
		select_chain.clone(),
		telemetry.as_ref().map(|x| x.handle()),
	)?;

	let storage_override = Arc::new(StorageOverrideHandler::<B, _, _>::new(client.clone()));
	let frontier_backend = match eth_config.frontier_backend_type {
		BackendType::KeyValue => FrontierBackend::KeyValue(Arc::new(fc_db::kv::Backend::open(
			Arc::clone(&client),
			&config.database,
			&db_config_dir(&config),
		)?)),
		BackendType::Sql => {
			let db_path = db_config_dir(&config).join("sql");
			std::fs::create_dir_all(&db_path).expect("failed creating sql db directory");
			let backend = futures::executor::block_on(fc_db::sql::Backend::new(
				fc_db::sql::BackendConfig::Sqlite(fc_db::sql::SqliteBackendConfig {
					path: Path::new("sqlite:///")
						.join(db_path)
						.join("frontier.db3")
						.to_str()
						.unwrap(),
					create_if_missing: true,
					thread_count: eth_config.frontier_sql_backend_thread_count,
					cache_size: eth_config.frontier_sql_backend_cache_size,
				}),
				eth_config.frontier_sql_backend_pool_size,
				std::num::NonZeroU32::new(eth_config.frontier_sql_backend_num_ops_timeout),
				storage_override.clone(),
			))
			.unwrap_or_else(|err| panic!("failed creating sql backend: {err:?}"));
			FrontierBackend::Sql(Arc::new(backend))
		}
	};

	let transaction_pool = Arc::from(
		sc_transaction_pool::Builder::new(
			task_manager.spawn_essential_handle(),
			client.clone(),
			config.role.is_authority().into(),
		)
		.with_options(config.transaction_pool.clone())
		.with_prometheus(config.prometheus_registry())
		.build(),
	);

	// Wire Babe over GRANDPA, then wrap with Frontier.
	let babe_config = sc_consensus_babe::configuration(&*client)?;
	let slot_duration = babe_config.slot_duration();
	let (babe_block_import, babe_link) = sc_consensus_babe::block_import(
		babe_config,
		grandpa_block_import.clone(),
		client.clone(),
		move |_, ()| async move {
			// Match the proposer's slot-aligned timestamp (see
			// `create_inherent_data_providers` below) so the import-queue
			// verifier regenerates the same timestamp the proposer baked
			// into the block. A wall-clock provider here would diverge
			// during dev slot drift and fail `Digest item must match
			// that calculated`.
			let wall_timestamp = sp_timestamp::InherentDataProvider::from_system_time();
			let slot_ms = slot_duration.as_millis();
			let aligned_ms = (wall_timestamp.timestamp().as_millis() / slot_ms) * slot_ms;
			let timestamp = sp_timestamp::InherentDataProvider::new(aligned_ms.into());
			let slot =
				sp_consensus_babe::inherents::InherentDataProvider::from_timestamp_and_slot_duration(
					*timestamp,
					slot_duration,
				);
			Ok::<_, Box<dyn std::error::Error + Send + Sync>>((slot, timestamp))
		},
		select_chain.clone(),
		OffchainTransactionPoolFactory::new(transaction_pool.clone()),
	)?;
	let frontier_block_import = FrontierBlockImport::new(babe_block_import.clone(), client.clone());

	// IMPORTANT: keep `babe_worker_handle` alive for the lifetime of the
	// service. `sc_consensus_babe::import_queue` internally spawns an
	// essential `babe-worker` task that polls a channel whose sender lives
	// inside the returned `BabeWorkerHandle`. If we drop the handle here, the
	// sender is dropped, the channel closes, the worker future completes,
	// and `task_manager` reports `Essential task babe-worker failed`. The
	// handle is also the RPC entry point used by `fc-rpc` (epoch lookup,
	// inherent data), so we forward it into the RPC builder closure below.
	let (import_queue, babe_worker_handle) =
		sc_consensus_babe::import_queue(sc_consensus_babe::ImportQueueParams {
			link: babe_link.clone(),
			block_import: frontier_block_import,
			justification_import: Some(Box::new(grandpa_block_import)),
			client: client.clone(),
			slot_duration,
			spawner: &task_manager.spawn_essential_handle(),
			registry: config.prometheus_registry(),
			telemetry: telemetry.as_ref().map(|x| x.handle()),
		})?;

	// -----------------------------------------------------------------
	// Network setup -- mirrors the Aura path.
	// -----------------------------------------------------------------

	let FrontierPartialComponents {
		filter_pool,
		fee_history_cache,
		fee_history_cache_limit,
	} = new_frontier_partial(&eth_config)?;

	let maybe_registry = config.prometheus_config.as_ref().map(|cfg| &cfg.registry);
	let mut net_config = sc_network::config::FullNetworkConfiguration::<_, _, NB>::new(
		&config.network,
		maybe_registry.cloned(),
	);
	let peer_store_handle = net_config.peer_store_handle();
	let metrics = NB::register_notification_metrics(maybe_registry);

	let grandpa_protocol_name = sc_consensus_grandpa::protocol_standard_name(
		&client
			.block_hash(0u32.into())
			.ok()
			.flatten()
			.expect("Genesis block exists; qed"),
		&config.chain_spec,
	);

	let (grandpa_protocol_config, grandpa_notification_service) =
		sc_consensus_grandpa::grandpa_peers_set_config::<_, NB>(
			grandpa_protocol_name.clone(),
			metrics.clone(),
			peer_store_handle,
		);

	net_config.add_notification_protocol(grandpa_protocol_config);
	let warp_sync: Arc<dyn WarpSyncProvider<B>> =
		Arc::new(sc_consensus_grandpa::warp_proof::NetworkProvider::new(
			backend.clone(),
			grandpa_link.shared_authority_set().clone(),
			Vec::new(),
		));
	let warp_sync_config = Some(WarpSyncConfig::WithProvider(warp_sync));

	let (network, system_rpc_tx, tx_handler_controller, sync_service) =
		sc_service::build_network(sc_service::BuildNetworkParams {
			config: &config,
			net_config,
			client: client.clone(),
			transaction_pool: transaction_pool.clone(),
			spawn_handle: task_manager.spawn_handle(),
			spawn_essential_handle: task_manager.spawn_essential_handle(),
			import_queue,
			block_announce_validator_builder: None,
			warp_sync_config,
			block_relay: None,
			metrics,
		})?;

	if config.offchain_worker.enabled {
		let offchain_workers =
			sc_offchain::OffchainWorkers::new(sc_offchain::OffchainWorkerOptions {
				runtime_api_provider: client.clone(),
				is_validator: config.role.is_authority(),
				keystore: Some(keystore_container.keystore()),
				offchain_db: backend.offchain_storage(),
				transaction_pool: Some(OffchainTransactionPoolFactory::new(
					transaction_pool.clone(),
				)),
				network_provider: Arc::new(network.clone()),
				enable_http_requests: true,
				custom_extensions: |_| vec![],
			})?;
		task_manager.spawn_handle().spawn(
			"offchain-workers-runner",
			"offchain-worker",
			offchain_workers
				.run(client.clone(), task_manager.spawn_handle())
				.boxed(),
		);
	}

	let role = config.role;
	// Force-author for the single-node dev shortcut (one process owning all
	// four validator keys via `IMPETUS_INJECT_ALL_DEV_KEYS=1`): without a
	// peer set, Babe's default "wait for the network" gating would skip
	// every proposal slot and block production would stall at #0.
	//
	// Multi-node dev setups rely on natural slot-lottery and must NOT
	// force-author: doing so causes slot/timestamp drift and the
	// `Unexpected epoch change` rollback. Live/Local specs honor the CLI
	// flag as configured.
	let force_authoring =
		config.force_authoring || (config.role.is_authority() && inject_all_dev_keys);
	let allow_non_global_ips_in_dht = config.network.allow_non_globals_in_dht;
	let name = config.network.node_name.clone();
	let frontier_backend = Arc::new(frontier_backend);
	let enable_grandpa = !config.disable_grandpa;
	let prometheus_registry = config.prometheus_registry().cloned();

	// Sinks for pubsub notifications.
	fc_mapping_sync::set_max_pending_notifications_per_subscriber(
		eth_config.pubsub_max_pending_notifications,
	);
	let pubsub_notification_sinks: fc_mapping_sync::EthereumBlockNotificationSinks<
		fc_mapping_sync::EthereumBlockNotification<B>,
	> = Default::default();
	let pubsub_notification_sinks = Arc::new(pubsub_notification_sinks);

	// for ethereum-compatibility rpc.
	config.rpc.id_provider = Some(Box::new(fc_rpc::EthereumSubIdProvider));

	let rpc_builder = {
		let client = client.clone();
		let pool = transaction_pool.clone();
		let network = network.clone();
		let sync_service = sync_service.clone();
		let babe_link = babe_link.clone();

		let is_authority = role.is_authority();
		let enable_dev_signer = eth_config.enable_dev_signer;
		let max_past_logs = eth_config.max_past_logs;
		let max_block_range = eth_config.max_block_range;
		let execute_gas_limit_multiplier = eth_config.execute_gas_limit_multiplier;
		let rpc_allow_unprotected_txs = eth_config.rpc_allow_unprotected_txs;
		let filter_pool = filter_pool.clone();
		let frontier_backend = frontier_backend.clone();
		let pubsub_notification_sinks = pubsub_notification_sinks.clone();
		let storage_override = storage_override.clone();
		let fee_history_cache = fee_history_cache.clone();
		let block_data_cache = Arc::new(fc_rpc::EthBlockDataCacheTask::new(
			task_manager.spawn_handle(),
			storage_override.clone(),
			eth_config.eth_log_block_cache,
			eth_config.eth_statuses_cache,
			prometheus_registry.clone(),
		));

		let slot_duration = babe_link.config().slot_duration();
		let target_gas_price = eth_config.target_gas_price;
		let pending_create_inherent_data_providers = move |_, ()| async move {
			let current = sp_timestamp::InherentDataProvider::from_system_time();
			let next_slot = current.timestamp().as_millis() + slot_duration.as_millis();
			let timestamp = sp_timestamp::InherentDataProvider::new(next_slot.into());
			let slot =
				sp_consensus_babe::inherents::InherentDataProvider::from_timestamp_and_slot_duration(
					*timestamp,
					slot_duration,
				);
			let dynamic_fee = fp_dynamic_fee::InherentDataProvider(U256::from(target_gas_price));
			Ok((slot, timestamp, dynamic_fee))
		};

		Box::new(move |subscription_task_executor| {
			let eth_deps = crate::rpc::EthDeps {
				client: client.clone(),
				pool: pool.clone(),
				converter: Some(CT::default()),
				is_authority,
				enable_dev_signer,
				network: network.clone(),
				sync: sync_service.clone(),
				frontier_backend: match &*frontier_backend {
					fc_db::Backend::KeyValue(b) => b.clone(),
					fc_db::Backend::Sql(b) => b.clone(),
				},
				storage_override: storage_override.clone(),
				block_data_cache: block_data_cache.clone(),
				filter_pool: filter_pool.clone(),
				max_past_logs,
				max_block_range,
				fee_history_cache: fee_history_cache.clone(),
				fee_history_cache_limit,
				execute_gas_limit_multiplier,
				rpc_allow_unprotected_txs,
				forced_parent_hashes: None,
				pending_create_inherent_data_providers,
				// Babe path: omit the consensus digest for pending blocks.
				// `fc-rpc` has no `BabeConsensusDataProvider` yet, and the
				// Plan 1 runtime returns empty pending digest information.
				pending_consensus_data_provider: None,
			};
			let deps = crate::rpc::FullDeps {
				client: client.clone(),
				pool: pool.clone(),
				// Babe never speaks manual-seal.
				command_sink: None,
				eth: eth_deps,
			};
			crate::rpc::create_full(
				deps,
				subscription_task_executor,
				pubsub_notification_sinks.clone(),
			)
			.map_err(Into::into)
		})
	};

	// Derive state_pruning_blocks from the node's --state-pruning so the mapping-sync
	// worker can skip past pruned blocks during catch-up (KV backend only).
	let state_pruning_blocks = config.state_pruning.as_ref().and_then(|mode| {
		if let sc_service::PruningMode::Constrained(c) = mode {
			c.max_blocks.map(u64::from)
		} else {
			None
		}
	});

	let _rpc_handlers = sc_service::spawn_tasks(sc_service::SpawnTasksParams {
		config,
		client: client.clone(),
		backend: backend.clone(),
		task_manager: &mut task_manager,
		keystore: keystore_container.keystore(),
		transaction_pool: transaction_pool.clone(),
		rpc_builder,
		network: network.clone(),
		system_rpc_tx,
		tx_handler_controller,
		sync_service: sync_service.clone(),
		telemetry: telemetry.as_mut(),
		tracing_execute_block: None,
	})?;

	// Pin the `BabeWorkerHandle` to the service lifetime so the channel it
	// wraps stays open and the essential `babe-worker` task does not exit
	// immediately on first poll.
	task_manager.keep_alive(babe_worker_handle);

	spawn_frontier_tasks(
		&task_manager,
		client.clone(),
		backend,
		frontier_backend,
		filter_pool,
		storage_override,
		fee_history_cache,
		fee_history_cache_limit,
		state_pruning_blocks,
		sync_service.clone(),
		pubsub_notification_sinks,
	)
	.await;

	if role.is_authority() {
		let proposer_factory = sc_basic_authorship::ProposerFactory::new(
			task_manager.spawn_handle(),
			client.clone(),
			transaction_pool.clone(),
			prometheus_registry.as_ref(),
			telemetry.as_ref().map(|x| x.handle()),
		);

		let slot_duration = babe_link.config().slot_duration();
		// Snap the timestamp inherent to the slot boundary so that
		// `pallet_babe::on_initialize` sees `timestamp / slot_duration ==
		// CurrentSlot` exactly. Without this, wall-clock drift across a slot
		// boundary during proposal would set timestamp_slot = N+1 while the
		// pre-digest's slot is N, and the runtime asserts equality
		// (`Timestamp slot must match CurrentSlot`), panicking the proposer.
		// Production-shape long sessions tolerate brief drift, but
		// runtime-test-fast (5-block sessions) and slow debug builds make
		// drift more frequent.
		let create_inherent_data_providers = move |_, ()| async move {
			let wall_timestamp = sp_timestamp::InherentDataProvider::from_system_time();
			let slot_ms = slot_duration.as_millis();
			let aligned_ms = (wall_timestamp.timestamp().as_millis() / slot_ms) * slot_ms;
			let timestamp = sp_timestamp::InherentDataProvider::new(aligned_ms.into());
			let slot = sp_consensus_babe::inherents::InherentDataProvider::from_timestamp_and_slot_duration(
				*timestamp,
				slot_duration,
			);
			Ok((slot, timestamp))
		};

		let babe_params = sc_consensus_babe::BabeParams {
			keystore: keystore_container.keystore(),
			client: client.clone(),
			select_chain,
			env: proposer_factory,
			block_import: babe_block_import,
			sync_oracle: sync_service.clone(),
			justification_sync_link: sync_service.clone(),
			create_inherent_data_providers,
			force_authoring,
			backoff_authoring_blocks: Option::<()>::None,
			babe_link: babe_link.clone(),
			block_proposal_slot_portion: sc_consensus_babe::SlotProportion::new(0.5),
			max_block_proposal_slot_portion: None,
			telemetry: telemetry.as_ref().map(|x| x.handle()),
		};

		let babe = sc_consensus_babe::start_babe(babe_params)?;
		// `SpawnEssentialTaskHandle::spawn_blocking` already wraps the future
		// in `AssertUnwindSafe(..).catch_unwind()` internally and reports
		// failure via the essential-task channel. It consumes the panic
		// payload, so an outer `catch_unwind` cannot recover the message —
		// adding one here would be dead code.
		task_manager.spawn_essential_handle().spawn_blocking(
			"babe-proposer",
			Some("block-authoring"),
			babe,
		);
	}

	// Authority-discovery worker — populates the DHT with validator addresses
	// so peers can establish direct connections. The worker reads the live
	// authority set from pallet-authority-discovery (wired in Plan 2 via
	// pallet-session storage). Single-node dev runs leave the DHT empty but
	// the worker still runs harmlessly (R6 in the spec).
	if role.is_authority() {
		let authority_discovery_role =
			sc_authority_discovery::Role::PublishAndDiscover(keystore_container.keystore());
		let dht_event_stream =
			network
				.event_stream("authority-discovery")
				.filter_map(|e| async move {
					match e {
						Event::Dht(e) => Some(e),
						_ => None,
					}
				});
		let (authority_discovery_worker, _service) =
			sc_authority_discovery::new_worker_and_service_with_config(
				sc_authority_discovery::WorkerConfig {
					publish_non_global_ips: allow_non_global_ips_in_dht,
					..Default::default()
				},
				client.clone(),
				Arc::new(network.clone()),
				Box::pin(dht_event_stream),
				authority_discovery_role,
				prometheus_registry.clone(),
				task_manager.spawn_handle(),
			);

		task_manager.spawn_handle().spawn(
			"authority-discovery-worker",
			Some("networking"),
			authority_discovery_worker.run(),
		);
	}

	if enable_grandpa {
		// if the node isn't actively participating in consensus then it doesn't
		// need a keystore, regardless of which protocol we use below.
		let keystore = if role.is_authority() {
			Some(keystore_container.keystore())
		} else {
			None
		};

		let grandpa_config = sc_consensus_grandpa::Config {
			// FIXME #1578 make this available through chainspec
			gossip_duration: Duration::from_millis(333),
			justification_generation_period: GRANDPA_JUSTIFICATION_PERIOD,
			name: Some(name),
			observer_enabled: false,
			keystore,
			local_role: role,
			telemetry: telemetry.as_ref().map(|x| x.handle()),
			protocol_name: grandpa_protocol_name,
		};

		// start the full GRANDPA voter
		let grandpa_voter =
			sc_consensus_grandpa::run_grandpa_voter(sc_consensus_grandpa::GrandpaParams {
				config: grandpa_config,
				link: grandpa_link,
				network,
				sync: sync_service,
				notification_service: grandpa_notification_service,
				voting_rule: sc_consensus_grandpa::VotingRulesBuilder::default().build(),
				prometheus_registry,
				shared_voter_state: sc_consensus_grandpa::SharedVoterState::empty(),
				telemetry: telemetry.as_ref().map(|x| x.handle()),
				offchain_tx_pool_factory: OffchainTransactionPoolFactory::new(transaction_pool),
			})?;

		// the GRANDPA voter task is considered infallible, i.e.
		// if it fails we take down the service with it.
		task_manager
			.spawn_essential_handle()
			.spawn_blocking("grandpa-voter", None, grandpa_voter);
	}

	Ok(task_manager)
}

/// Minimal Babe partial build used by `new_chain_ops` for subcommands
/// (`check-block`, `revert`, `export-blocks`, ...). Returns only the bits
/// `command.rs` actually inspects.
///
/// Mirrors the head of [`new_full`] up to the import queue but skips
/// networking, RPC, authoring, GRANDPA, and authority-discovery.
pub fn new_partial_for_chain_ops<B, RA, HF>(
	config: &Configuration,
	eth_config: &EthConfiguration,
) -> Result<
	(
		Arc<FullClient<B, RA, HF>>,
		Arc<crate::client::FullBackend<B>>,
		BasicQueue<B>,
		TaskManager,
		FrontierBackend<B, FullClient<B, RA, HF>>,
	),
	ServiceError,
>
where
	B: BlockT<Hash = H256> + Unpin,
	NumberFor<B>: BlockNumberOps,
	<B as BlockT>::Header: Unpin,
	RA: ConstructRuntimeApi<B, FullClient<B, RA, HF>>,
	RA: Send + Sync + 'static,
	RA::RuntimeApi: BabeRuntimeApiCollection<B, AccountId, Nonce, Balance>,
	HF: HostFunctionsT + 'static,
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

	let executor = sc_service::new_wasm_executor(&config.executor);
	let (client, backend, _keystore_container, task_manager) =
		sc_service::new_full_parts_record_import::<B, RA, _>(
			config,
			telemetry.as_ref().map(|(_, telemetry)| telemetry.handle()),
			executor,
			true,
			vec![Arc::new(GrandpaPruningFilter)],
		)?;
	let client = Arc::new(client);

	let telemetry = telemetry.map(|(worker, telemetry)| {
		task_manager
			.spawn_handle()
			.spawn("telemetry", None, worker.run());
		telemetry
	});

	let select_chain = sc_consensus::LongestChain::new(backend.clone());
	let (grandpa_block_import, _grandpa_link) = sc_consensus_grandpa::block_import(
		client.clone(),
		GRANDPA_JUSTIFICATION_PERIOD,
		&client,
		select_chain.clone(),
		telemetry.as_ref().map(|x| x.handle()),
	)?;

	let storage_override = Arc::new(StorageOverrideHandler::<B, _, _>::new(client.clone()));
	let frontier_backend = match eth_config.frontier_backend_type {
		BackendType::KeyValue => FrontierBackend::KeyValue(Arc::new(fc_db::kv::Backend::open(
			Arc::clone(&client),
			&config.database,
			&db_config_dir(config),
		)?)),
		BackendType::Sql => {
			let db_path = db_config_dir(config).join("sql");
			std::fs::create_dir_all(&db_path).expect("failed creating sql db directory");
			let backend = futures::executor::block_on(fc_db::sql::Backend::new(
				fc_db::sql::BackendConfig::Sqlite(fc_db::sql::SqliteBackendConfig {
					path: Path::new("sqlite:///")
						.join(db_path)
						.join("frontier.db3")
						.to_str()
						.unwrap(),
					create_if_missing: true,
					thread_count: eth_config.frontier_sql_backend_thread_count,
					cache_size: eth_config.frontier_sql_backend_cache_size,
				}),
				eth_config.frontier_sql_backend_pool_size,
				std::num::NonZeroU32::new(eth_config.frontier_sql_backend_num_ops_timeout),
				storage_override.clone(),
			))
			.unwrap_or_else(|err| panic!("failed creating sql backend: {err:?}"));
			FrontierBackend::Sql(Arc::new(backend))
		}
	};

	let transaction_pool = Arc::from(
		sc_transaction_pool::Builder::new(
			task_manager.spawn_essential_handle(),
			client.clone(),
			config.role.is_authority().into(),
		)
		.with_options(config.transaction_pool.clone())
		.with_prometheus(config.prometheus_registry())
		.build(),
	);

	let babe_config = sc_consensus_babe::configuration(&*client)?;
	let slot_duration = babe_config.slot_duration();
	let (babe_block_import, babe_link) = sc_consensus_babe::block_import(
		babe_config,
		grandpa_block_import.clone(),
		client.clone(),
		move |_, ()| async move {
			// chain-ops path: align timestamp to slot for consistency with
			// the live import queue in `new_full`.
			let wall_timestamp = sp_timestamp::InherentDataProvider::from_system_time();
			let slot_ms = slot_duration.as_millis();
			let aligned_ms = (wall_timestamp.timestamp().as_millis() / slot_ms) * slot_ms;
			let timestamp = sp_timestamp::InherentDataProvider::new(aligned_ms.into());
			let slot =
				sp_consensus_babe::inherents::InherentDataProvider::from_timestamp_and_slot_duration(
					*timestamp,
					slot_duration,
				);
			Ok::<_, Box<dyn std::error::Error + Send + Sync>>((slot, timestamp))
		},
		select_chain,
		OffchainTransactionPoolFactory::new(transaction_pool),
	)?;
	let frontier_block_import = FrontierBlockImport::new(babe_block_import, client.clone());

	// chain-ops path: no authoring loop runs, so dropping the worker handle is intentional.
	let (import_queue, _babe_worker_handle) =
		sc_consensus_babe::import_queue(sc_consensus_babe::ImportQueueParams {
			link: babe_link,
			block_import: frontier_block_import,
			justification_import: Some(Box::new(grandpa_block_import)),
			client: client.clone(),
			slot_duration,
			spawner: &task_manager.spawn_essential_handle(),
			registry: config.prometheus_registry(),
			telemetry: telemetry.as_ref().map(|x| x.handle()),
		})?;

	Ok((client, backend, import_queue, task_manager, frontier_backend))
}
