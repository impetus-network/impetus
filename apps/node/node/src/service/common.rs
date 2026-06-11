//! Shared node service components used by all consensus engines.
//!
//! Contains client type aliases, the generic `new_partial` setup function, and
//! the GRANDPA block-import plumbing shared between Aura (dev/impulse) and
//! future Babe (impetus) service implementations.

use std::{path::Path, sync::Arc};

// Substrate
use sc_consensus::{BasicQueue, BoxBlockImport};
use sc_consensus_grandpa::{GrandpaPruningFilter};
use sc_executor::HostFunctions as HostFunctionsT;
use sc_service::{error::Error as ServiceError, Configuration, PartialComponents, TaskManager};
use sc_telemetry::{TelemetryHandle, TelemetryWorker};
use sp_api::ConstructRuntimeApi;
use sp_core::H256;
use sp_runtime::traits::Block as BlockT;

use crate::{
	client::{BaseRuntimeApiCollection, FullBackend, FullClient},
	eth::{
		db_config_dir, BackendType, EthCompatRuntimeApiCollection, EthConfiguration,
		FrontierBackend, StorageOverride, StorageOverrideHandler,
	},
};

/// Only enable the benchmarking host functions when we actually want to benchmark.
#[cfg(feature = "runtime-benchmarks")]
pub type HostFunctions = (
	sp_io::SubstrateHostFunctions,
	frame_benchmarking::benchmarking::HostFunctions,
	cumulus_primitives_proof_size_hostfunction::storage_proof_size::HostFunctions,
);
/// Otherwise we use empty host functions for ext host functions.
#[cfg(not(feature = "runtime-benchmarks"))]
pub type HostFunctions = (
	sp_io::SubstrateHostFunctions,
	cumulus_primitives_proof_size_hostfunction::storage_proof_size::HostFunctions,
);

pub type FullSelectChain<B> = sc_consensus::LongestChain<FullBackend<B>, B>;
pub type GrandpaBlockImport<B, C> =
	sc_consensus_grandpa::GrandpaBlockImport<FullBackend<B>, B, C, FullSelectChain<B>>;
pub type GrandpaLinkHalf<B, C> = sc_consensus_grandpa::LinkHalf<B, C, FullSelectChain<B>>;

/// The minimum period of blocks on which justifications will be
/// imported and generated.
pub const GRANDPA_JUSTIFICATION_PERIOD: u32 = 512;

pub fn new_partial<B, RA, HF, BIQ>(
	config: &Configuration,
	eth_config: &EthConfiguration,
	build_import_queue: BIQ,
) -> Result<
	PartialComponents<
		FullClient<B, RA, HF>,
		FullBackend<B>,
		FullSelectChain<B>,
		BasicQueue<B>,
		sc_transaction_pool::TransactionPoolHandle<B, FullClient<B, RA, HF>>,
		(
			Option<sc_telemetry::Telemetry>,
			BoxBlockImport<B>,
			GrandpaLinkHalf<B, FullClient<B, RA, HF>>,
			FrontierBackend<B, FullClient<B, RA, HF>>,
			Arc<dyn StorageOverride<B>>,
		),
	>,
	ServiceError,
>
where
	B: BlockT<Hash = H256>,
	RA: ConstructRuntimeApi<B, FullClient<B, RA, HF>>,
	RA: Send + Sync + 'static,
	RA::RuntimeApi: BaseRuntimeApiCollection<B> + EthCompatRuntimeApiCollection<B>,
	HF: HostFunctionsT + 'static,
	BIQ: FnOnce(
		Arc<FullClient<B, RA, HF>>,
		&Configuration,
		&EthConfiguration,
		&TaskManager,
		Option<TelemetryHandle>,
		GrandpaBlockImport<B, FullClient<B, RA, HF>>,
	) -> Result<(BasicQueue<B>, BoxBlockImport<B>), ServiceError>,
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

	let (client, backend, keystore_container, task_manager) =
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

	let (import_queue, block_import) = build_import_queue(
		client.clone(),
		config,
		eth_config,
		&task_manager,
		telemetry.as_ref().map(|x| x.handle()),
		grandpa_block_import,
	)?;

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

	Ok(PartialComponents {
		client,
		backend,
		keystore_container,
		task_manager,
		select_chain,
		import_queue,
		transaction_pool,
		other: (
			telemetry,
			block_import,
			grandpa_link,
			frontier_backend,
			storage_override,
		),
	})
}
