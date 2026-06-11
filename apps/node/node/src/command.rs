use futures::TryFutureExt;
// Substrate
use sc_cli::{ChainSpec, SubstrateCli};
use sc_service::DatabaseSource;
// Frontier
use fc_db::kv::frontier_database_dir;

use crate::{
	chain_spec,
	cli::{Cli, Subcommand},
	service::{self, db_config_dir},
};

impl SubstrateCli for Cli {
	fn impl_name() -> String {
		"Frontier Node".into()
	}

	fn impl_version() -> String {
		env!("SUBSTRATE_CLI_IMPL_VERSION").into()
	}

	fn description() -> String {
		env!("CARGO_PKG_DESCRIPTION").into()
	}

	fn author() -> String {
		env!("CARGO_PKG_AUTHORS").into()
	}

	fn support_url() -> String {
		"support.anonymous.an".into()
	}

	fn copyright_start_year() -> i32 {
		2021
	}

	fn load_spec(&self, id: &str) -> Result<Box<dyn ChainSpec>, String> {
		Ok(match id {
			"dev" => {
				let enable_manual_seal = self.sealing.is_some();
				Box::new(chain_spec::development_config(enable_manual_seal))
			}
			"impetus" | "mainnet" => Box::new(chain_spec::impetus_production_config()),
			"impetus_dev_npos" => Box::new(chain_spec::impetus_config()),
			"" | "impulse" | "testnet" => Box::new(chain_spec::impulse_config()),
			path => Box::new(chain_spec::ChainSpec::from_json_file(
				std::path::PathBuf::from(path),
			)?),
		})
	}
}

/// Parse and run command line arguments
pub fn run() -> sc_cli::Result<()> {
	let cli = Cli::from_args();

	match &cli.subcommand {
		Some(Subcommand::Key(cmd)) => cmd.run(&cli),
		Some(Subcommand::BuildSpec(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			runner.sync_run(|config| cmd.run(config.chain_spec, config.network))
		}
		Some(Subcommand::CheckBlock(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			runner.async_run(|mut config| {
				let (future, task_manager): (
					std::pin::Pin<Box<dyn futures::Future<Output = sc_cli::Result<()>>>>,
					_,
				) = match service::new_chain_ops(&mut config, &cli.eth)? {
					service::ChainOps::Impetus(client, _, import_queue, task_manager, _) => {
						(Box::pin(cmd.run(client, import_queue)), task_manager)
					}
					service::ChainOps::Impulse(client, _, import_queue, task_manager, _) => {
						(Box::pin(cmd.run(client, import_queue)), task_manager)
					}
				};
				Ok((future, task_manager))
			})
		}
		Some(Subcommand::ExportBlocks(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			runner.async_run(|mut config| {
				let (future, task_manager): (
					std::pin::Pin<Box<dyn futures::Future<Output = sc_cli::Result<()>>>>,
					_,
				) = match service::new_chain_ops(&mut config, &cli.eth)? {
					service::ChainOps::Impetus(client, _, _, task_manager, _) => {
						(Box::pin(cmd.run(client, config.database)), task_manager)
					}
					service::ChainOps::Impulse(client, _, _, task_manager, _) => {
						(Box::pin(cmd.run(client, config.database)), task_manager)
					}
				};
				Ok((future, task_manager))
			})
		}
		Some(Subcommand::ExportState(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			runner.async_run(|mut config| {
				let (future, task_manager): (
					std::pin::Pin<Box<dyn futures::Future<Output = sc_cli::Result<()>>>>,
					_,
				) = match service::new_chain_ops(&mut config, &cli.eth)? {
					service::ChainOps::Impetus(client, _, _, task_manager, _) => {
						(Box::pin(cmd.run(client, config.chain_spec)), task_manager)
					}
					service::ChainOps::Impulse(client, _, _, task_manager, _) => {
						(Box::pin(cmd.run(client, config.chain_spec)), task_manager)
					}
				};
				Ok((future, task_manager))
			})
		}
		Some(Subcommand::ImportBlocks(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			runner.async_run(|mut config| {
				let (future, task_manager): (
					std::pin::Pin<Box<dyn futures::Future<Output = sc_cli::Result<()>>>>,
					_,
				) = match service::new_chain_ops(&mut config, &cli.eth)? {
					service::ChainOps::Impetus(client, _, import_queue, task_manager, _) => {
						(Box::pin(cmd.run(client, import_queue)), task_manager)
					}
					service::ChainOps::Impulse(client, _, import_queue, task_manager, _) => {
						(Box::pin(cmd.run(client, import_queue)), task_manager)
					}
				};
				Ok((future, task_manager))
			})
		}
		Some(Subcommand::PurgeChain(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			runner.sync_run(|config| {
				// Remove Frontier offchain db
				let db_config_dir = db_config_dir(&config);
				match cli.eth.frontier_backend_type {
					crate::eth::BackendType::KeyValue => {
						let frontier_database_config = match config.database {
							#[cfg(feature = "rocksdb")]
							DatabaseSource::RocksDb { .. } => DatabaseSource::RocksDb {
								path: frontier_database_dir(&db_config_dir, "db"),
								cache_size: 0,
							},
							DatabaseSource::ParityDb { .. } => DatabaseSource::ParityDb {
								path: frontier_database_dir(&db_config_dir, "paritydb"),
							},
							_ => {
								return Err(format!(
									"Cannot purge `{:?}` database",
									config.database
								)
								.into())
							}
						};
						cmd.run(frontier_database_config)?;
					}
					crate::eth::BackendType::Sql => {
						let db_path = db_config_dir.join("sql");
						match std::fs::remove_dir_all(&db_path) {
							Ok(_) => {
								println!("{:?} removed.", &db_path);
							}
							Err(ref err) if err.kind() == std::io::ErrorKind::NotFound => {
								eprintln!("{:?} did not exist.", &db_path);
							}
							Err(err) => {
								return Err(
									format!("Cannot purge `{db_path:?}` database: {err:?}").into()
								)
							}
						};
					}
				};
				cmd.run(config.database)
			})
		}
		Some(Subcommand::Revert(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			runner.async_run(|mut config| {
				let (future, task_manager): (
					std::pin::Pin<Box<dyn futures::Future<Output = sc_cli::Result<()>>>>,
					_,
				) = match service::new_chain_ops(&mut config, &cli.eth)? {
					service::ChainOps::Impetus(client, backend, _, task_manager, _) => {
						let aux_revert = Box::new(move |client, _, blocks| {
							sc_consensus_grandpa::revert(client, blocks)?;
							Ok(())
						});
						(
							Box::pin(cmd.run(client, backend, Some(aux_revert))),
							task_manager,
						)
					}
					service::ChainOps::Impulse(client, backend, _, task_manager, _) => {
						let aux_revert = Box::new(move |client, _, blocks| {
							sc_consensus_grandpa::revert(client, blocks)?;
							Ok(())
						});
						(
							Box::pin(cmd.run(client, backend, Some(aux_revert))),
							task_manager,
						)
					}
				};
				Ok((future, task_manager))
			})
		}
		#[cfg(feature = "runtime-benchmarks")]
		Some(Subcommand::Benchmark(cmd)) => {
			use crate::benchmarking::inherent_benchmark_data;
			use crate::chain_spec::Network;
			use frame_benchmarking_cli::{
				BenchmarkCmd, ExtrinsicFactory, SUBSTRATE_REFERENCE_HARDWARE,
			};

			let runner = cli.create_runner(cmd)?;
			match cmd {
				BenchmarkCmd::Pallet(cmd) => runner.sync_run(|config| {
					match Network::from_spec_id(config.chain_spec.id()) {
						Network::Impetus => cmd
							.run_with_spec::<runtime_common::Hashing, ()>(Some(config.chain_spec)),
						Network::Impulse => cmd
							.run_with_spec::<runtime_common::Hashing, ()>(Some(config.chain_spec)),
					}
				}),
				BenchmarkCmd::Block(cmd) => runner.sync_run(|mut config| {
					let ops = service::new_chain_ops(&mut config, &cli.eth)?;
					match ops {
						service::ChainOps::Impetus(client, _, _, _, _) => cmd.run(client),
						service::ChainOps::Impulse(client, _, _, _, _) => cmd.run(client),
					}
				}),
				BenchmarkCmd::Storage(cmd) => runner.sync_run(|mut config| {
					let ops = service::new_chain_ops(&mut config, &cli.eth)?;
					match ops {
						service::ChainOps::Impetus(client, backend, _, _, _) => {
							let db = backend.expose_db();
							let storage = backend.expose_storage();
							let shared_cache = backend.expose_shared_trie_cache();
							cmd.run(config, client, db, storage, shared_cache)
						}
						service::ChainOps::Impulse(client, backend, _, _, _) => {
							let db = backend.expose_db();
							let storage = backend.expose_storage();
							let shared_cache = backend.expose_shared_trie_cache();
							cmd.run(config, client, db, storage, shared_cache)
						}
					}
				}),
				BenchmarkCmd::Overhead(cmd) => runner.sync_run(|mut config| {
					let chain_name = config.chain_spec.name().to_string();
					let ops = service::new_chain_ops(&mut config, &cli.eth)?;
					match ops {
						service::ChainOps::Impetus(client, _, _, _, _) => {
							let ext_builder =
								crate::benchmarking::impetus::RemarkBuilder::new(client.clone());
							cmd.run(
								chain_name,
								client,
								inherent_benchmark_data()?,
								Vec::new(),
								&ext_builder,
								false,
							)
						}
						service::ChainOps::Impulse(client, _, _, _, _) => {
							let ext_builder =
								crate::benchmarking::impulse::RemarkBuilder::new(client.clone());
							cmd.run(
								chain_name,
								client,
								inherent_benchmark_data()?,
								Vec::new(),
								&ext_builder,
								false,
							)
						}
					}
				}),
				BenchmarkCmd::Extrinsic(cmd) => runner.sync_run(|mut config| {
					let ops = service::new_chain_ops(&mut config, &cli.eth)?;
					match ops {
						service::ChainOps::Impetus(client, _, _, _, _) => {
							let ext_factory = ExtrinsicFactory(vec![
								Box::new(crate::benchmarking::impetus::RemarkBuilder::new(
									client.clone(),
								)),
								Box::new(crate::benchmarking::impetus::TransferKeepAliveBuilder::new(
									client.clone(),
									runtime_common::admin_account(),
									1_000_000_000_000_000_000u128,
								)),
							]);
							cmd.run(client, inherent_benchmark_data()?, Vec::new(), &ext_factory)
						}
						service::ChainOps::Impulse(client, _, _, _, _) => {
							let ext_factory = ExtrinsicFactory(vec![
								Box::new(crate::benchmarking::impulse::RemarkBuilder::new(
									client.clone(),
								)),
								Box::new(crate::benchmarking::impulse::TransferKeepAliveBuilder::new(
									client.clone(),
									runtime_common::admin_account(),
									1_000_000_000_000_000_000u128,
								)),
							]);
							cmd.run(client, inherent_benchmark_data()?, Vec::new(), &ext_factory)
						}
					}
				}),
				BenchmarkCmd::Machine(cmd) => {
					runner.sync_run(|config| cmd.run(&config, SUBSTRATE_REFERENCE_HARDWARE.clone()))
				}
			}
		}
		#[cfg(not(feature = "runtime-benchmarks"))]
		Some(Subcommand::Benchmark) => Err("Benchmarking wasn't enabled when building the node. \
			You can enable it with `--features runtime-benchmarks`."
			.into()),
		Some(Subcommand::FrontierDb(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			runner.sync_run(|mut config| {
				let ops = service::new_chain_ops(&mut config, &cli.eth)?;
				match ops {
					service::ChainOps::Impetus(client, _, _, _, frontier_backend) => {
						let frontier_backend = match frontier_backend {
							fc_db::Backend::KeyValue(kv) => kv,
							_ => panic!("Only fc_db::Backend::KeyValue supported"),
						};
						cmd.run(client, frontier_backend)
					}
					service::ChainOps::Impulse(client, _, _, _, frontier_backend) => {
						let frontier_backend = match frontier_backend {
							fc_db::Backend::KeyValue(kv) => kv,
							_ => panic!("Only fc_db::Backend::KeyValue supported"),
						};
						cmd.run(client, frontier_backend)
					}
				}
			})
		}
		None => {
			let runner = cli.create_runner(&cli.run)?;
			runner.run_node_until_exit(|config| async move {
				service::build_full(config, cli.eth, cli.sealing)
					.map_err(Into::into)
					.await
			})
		}
	}
}
