//! Node service dispatch.
//!
//! `common` holds the parts shared between Aura and Babe (client setup,
//! GRANDPA block import, Frontier backend, telemetry). `aura` powers
//! impulse + dev. `babe` (Task 12) will power impetus. `command.rs` picks
//! the path by chain spec id (Task 13).

pub mod aura;
pub mod babe;
pub mod common;

// Re-export the HostFunctions type alias so callers (command.rs, benchmarking.rs)
// can continue to use `service::HostFunctions`.
pub use common::HostFunctions;

// Re-export new_partial for callers that construct PartialComponents directly.
pub use common::new_partial;

// Re-export the Aura import-queue builder used by new_chain_ops below.
use aura::build_aura_grandpa_import_queue;

pub use crate::eth::{db_config_dir, EthConfiguration};

use std::sync::Arc;

use sc_consensus::BasicQueue;
use sc_service::{error::Error as ServiceError, Configuration, PartialComponents, TaskManager};
use crate::{
	cli::Sealing,
	client::{FullBackend, FullClient},
	eth::FrontierBackend,
};

/// Dispatch enum returned by `new_chain_ops`, one variant per supported runtime.
pub enum ChainOps {
	Impetus(
		Arc<FullClient<impetus_runtime::opaque::Block, impetus_runtime::RuntimeApi, HostFunctions>>,
		Arc<FullBackend<impetus_runtime::opaque::Block>>,
		BasicQueue<impetus_runtime::opaque::Block>,
		TaskManager,
		FrontierBackend<
			impetus_runtime::opaque::Block,
			FullClient<impetus_runtime::opaque::Block, impetus_runtime::RuntimeApi, HostFunctions>,
		>,
	),
	Impulse(
		Arc<FullClient<impulse_runtime::opaque::Block, impulse_runtime::RuntimeApi, HostFunctions>>,
		Arc<FullBackend<impulse_runtime::opaque::Block>>,
		BasicQueue<impulse_runtime::opaque::Block>,
		TaskManager,
		FrontierBackend<
			impulse_runtime::opaque::Block,
			FullClient<impulse_runtime::opaque::Block, impulse_runtime::RuntimeApi, HostFunctions>,
		>,
	),
}

pub async fn build_full(
	config: Configuration,
	eth_config: EthConfiguration,
	sealing: Option<Sealing>,
) -> Result<TaskManager, ServiceError> {
	use crate::chain_spec::Network;

	match Network::from_spec_id(config.chain_spec.id()) {
		Network::Impetus => {
			// Impetus uses Babe + GRANDPA + authority-discovery. Sealing is
			// only meaningful on the Aura path (manual seal for dev); the
			// Babe path ignores it.
			let _ = sealing;
			babe::new_full::<
				impetus_runtime::opaque::Block,
				impetus_runtime::RuntimeApi,
				HostFunctions,
				sc_network::NetworkWorker<_, _>,
				impetus_runtime::TransactionConverter<impetus_runtime::opaque::Block>,
			>(config, eth_config)
			.await
		}
		Network::Impulse => {
			aura::new_full::<
				impulse_runtime::opaque::Block,
				impulse_runtime::RuntimeApi,
				HostFunctions,
				sc_network::NetworkWorker<_, _>,
				impulse_runtime::TransactionConverter<impulse_runtime::opaque::Block>,
			>(config, eth_config, sealing)
			.await
		}
	}
}

pub fn new_chain_ops(
	config: &mut Configuration,
	eth_config: &EthConfiguration,
) -> Result<ChainOps, ServiceError> {
	use crate::chain_spec::Network;

	config.keystore = sc_service::config::KeystoreConfig::InMemory;
	match Network::from_spec_id(config.chain_spec.id()) {
		Network::Impetus => {
			let (client, backend, import_queue, task_manager, frontier_backend) =
				babe::new_partial_for_chain_ops::<
					impetus_runtime::opaque::Block,
					impetus_runtime::RuntimeApi,
					HostFunctions,
				>(config, eth_config)?;
			Ok(ChainOps::Impetus(
				client,
				backend,
				import_queue,
				task_manager,
				frontier_backend,
			))
		}
		Network::Impulse => {
			let PartialComponents {
				client,
				backend,
				import_queue,
				task_manager,
				other,
				..
			} = new_partial::<
				impulse_runtime::opaque::Block,
				impulse_runtime::RuntimeApi,
				HostFunctions,
				_,
			>(config, eth_config, build_aura_grandpa_import_queue)?;
			Ok(ChainOps::Impulse(
				client,
				backend,
				import_queue,
				task_manager,
				other.3,
			))
		}
	}
}
