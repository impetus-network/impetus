use crate::{
	AccountId, BalancesConfig, EVMChainIdConfig, EVMConfig, EthereumConfig,
	GaslessRegistryConfig, ManualSealConfig, RuntimeGenesisConfig, SudoConfig,
};
use runtime_common::{admin_account, endowed_accounts};
use sp_consensus_grandpa::AuthorityId as GrandpaId;
#[allow(unused_imports)]
use sp_core::ecdsa;
use sp_core::{H160, U256};
use sp_genesis_builder::PresetId;
use sp_std::prelude::*;

/// Impetus chain ID
const IMPETUS_CHAIN_ID: u64 = 388266;

/// Generate a chain spec for use with the development service.
pub fn development() -> serde_json::Value {
	testnet_genesis(
		admin_account(),
		endowed_accounts(),
		vec![],
		IMPETUS_CHAIN_ID,
		false,
	)
}

/// Configure initial storage state for FRAME modules.
fn testnet_genesis(
	sudo_key: AccountId,
	endowed_accounts: Vec<AccountId>,
	_initial_authorities: Vec<GrandpaId>,
	chain_id: u64,
	enable_manual_seal: bool,
) -> serde_json::Value {
	let evm_accounts = {
		let mut map = sp_std::collections::btree_map::BTreeMap::new();
		for account in &endowed_accounts {
			map.insert(
				H160::from(*account),
				fp_evm::GenesisAccount {
					balance: U256::from(1_000_000u128) * U256::from(1_000_000_000_000_000_000u128),
					code: Default::default(),
					nonce: Default::default(),
					storage: Default::default(),
				},
			);
		}
		map
	};

	// Plan 2 adds many NPoS-related fields to `RuntimeGenesisConfig`. T18
	// rewrites this preset with real session keys, bonded stakers, and bag
	// thresholds; for now `Default::default()` is enough to keep the runtime
	// compiling, and a dev-mode chain spec will still boot because no NPoS
	// extrinsic is touched before T18 lands.
	let config = RuntimeGenesisConfig {
		system: Default::default(),
		babe: Default::default(),
		base_fee: Default::default(),
		grandpa: Default::default(),
		balances: BalancesConfig {
			balances: endowed_accounts
				.iter()
				.cloned()
				.map(|k| (k, 1 << 110))
				.collect(),
			..Default::default()
		},
		ethereum: EthereumConfig {
			..Default::default()
		},
		evm: EVMConfig {
			accounts: evm_accounts.into_iter().collect(),
			..Default::default()
		},
		evm_chain_id: EVMChainIdConfig {
			chain_id,
			..Default::default()
		},
		manual_seal: ManualSealConfig {
			enable: enable_manual_seal,
			..Default::default()
		},
		sudo: SudoConfig {
			key: Some(sudo_key),
		},
		// Dev preset has no vesting schedules; production sets them via the
		// chain-spec genesis patch (team allocation).
		vesting: Default::default(),
		transaction_payment: Default::default(),
		assets: Default::default(),
		gasless_registry: GaslessRegistryConfig {
			rules: vec![],
			_phantom: Default::default(),
		},
		session: Default::default(),
		staking: Default::default(),
		treasury: Default::default(),
		nomination_pools: Default::default(),
		im_online: Default::default(),
		authority_discovery: Default::default(),
	};

	serde_json::to_value(&config).expect("Could not build genesis config.")
}

/// Provides the JSON representation of predefined genesis config for given `id`.
pub fn get_preset(id: &PresetId) -> Option<Vec<u8>> {
	let patch = match id.as_str() {
		sp_genesis_builder::DEV_RUNTIME_PRESET => development(),
		_ => return None,
	};
	Some(
		serde_json::to_string(&patch)
			.expect("serialization to json is expected to work. qed.")
			.into_bytes(),
	)
}
