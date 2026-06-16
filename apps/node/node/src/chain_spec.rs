use std::collections::BTreeMap;
use std::str::FromStr;

use sc_chain_spec::{ChainType, Properties};
use serde::Deserialize;
use sp_authority_discovery::AuthorityId as AuthorityDiscoveryId;
use sp_consensus_aura::sr25519::AuthorityId as AuraId;
use sp_consensus_babe::AuthorityId as BabeId;
use sp_consensus_grandpa::AuthorityId as GrandpaId;
use sp_core::{Pair, Public, H160, U256};

use frame_support::PalletId;
use impetus_runtime::SessionKeys as ImpetusSessionKeys;
use pallet_im_online::sr25519::AuthorityId as ImOnlineId;
use runtime_common::{admin_account, endowed_accounts, AccountId, Balance};
use sp_runtime::traits::AccountIdConversion;

/// JSON shape of `IMPETUS_VALIDATOR_KEYS_FILE`: one entry per genesis
/// validator, holding the stash H160 and the four session-key pubkeys the
/// operator generated offline. The order is irrelevant — entries are matched
/// by stash to `production_validator_accounts()`.
#[derive(Debug, Deserialize)]
struct ValidatorKeysEntry {
	stash: H160,
	babe: sp_core::sr25519::Public,
	grandpa: sp_core::ed25519::Public,
	im_online: sp_core::sr25519::Public,
	authority_discovery: sp_core::sr25519::Public,
}

/// JSON shape of `IMPETUS_ALLOCATION_FILE`: one entry per allocation bucket.
/// `balance` is serialized as a decimal string to avoid JSON f64 precision
/// loss on 1e27-range planck amounts.
/// `bucket` distinguishes the team allocation (which receives a vesting
/// schedule) from community, staking_reserve, and liquidity buckets.
#[derive(Debug, Deserialize)]
struct AllocationEntry {
	address: H160,
	#[serde(deserialize_with = "deserialize_u128_from_str")]
	balance: u128,
	bucket: String,
}

fn deserialize_u128_from_str<'de, D>(deserializer: D) -> Result<u128, D::Error>
where
	D: serde::Deserializer<'de>,
{
	use serde::de::Error;
	// Accept both JSON strings ("123") and bare numbers (123) for flexibility.
	#[derive(Deserialize)]
	#[serde(untagged)]
	enum StringOrNumber {
		Str(String),
		Num(u128),
	}
	match StringOrNumber::deserialize(deserializer)? {
		StringOrNumber::Str(s) => {
			s.trim().parse::<u128>().map_err(|e| {
				D::Error::custom(format!("balance field is not a valid u128 string: {e}"))
			})
		}
		StringOrNumber::Num(n) => Ok(n),
	}
}

/// Load the operator-supplied genesis allocation JSON if
/// `IMPETUS_ALLOCATION_FILE` is set. Returns `None` when the env var is
/// unset (caller decides whether to fall back to placeholders or panic).
fn load_allocation_from_env() -> Option<Vec<AllocationEntry>> {
	let path = std::env::var("IMPETUS_ALLOCATION_FILE").ok()?;
	let bytes = std::fs::read(&path).unwrap_or_else(|err| {
		panic!("failed to read IMPETUS_ALLOCATION_FILE={path}: {err}")
	});
	let entries: Vec<AllocationEntry> = serde_json::from_slice(&bytes)
		.unwrap_or_else(|err| {
			panic!("failed to parse IMPETUS_ALLOCATION_FILE={path}: {err}")
		});
	Some(entries)
}

fn from_seed<TPublic: Public>(seed: &str) -> <TPublic::Pair as Pair>::Public {
	TPublic::Pair::from_string(&format!("//{seed}"), None)
		.expect("static values are valid; qed")
		.public()
}

fn authority_keys_from_seed(s: &str) -> (AuraId, GrandpaId) {
	(from_seed::<AuraId>(s), from_seed::<GrandpaId>(s))
}

/// Build the stash AccountId + opaque session keys for one impetus genesis
/// validator. The stash is the Hardhat-derived H160 (Plan 1 baseline) and the
/// four session keys are derived from the matching `//Alice`..`//Dave` seeds
/// so `--alice` / `--bob` / `--charlie` / `--dave` keystore injection lines up
/// with the genesis session entries.
fn impetus_session_keys(seed: &str) -> (AccountId, ImpetusSessionKeys) {
	let stash = match seed {
		"Alice" => H160::from_slice(&hex_literal::hex!(
			"f39Fd6e51aad88F6F4ce6aB8827279cffFb92266"
		)),
		"Bob" => H160::from_slice(&hex_literal::hex!(
			"70997970C51812dc3A010C7d01b50e0d17dc79C8"
		)),
		"Charlie" => H160::from_slice(&hex_literal::hex!(
			"3C44CdDdB6a900fa2b585dd299e03d12FA4293BC"
		)),
		"Dave" => H160::from_slice(&hex_literal::hex!(
			"90F79bf6EB2c4f870365E785982E1f101E93b906"
		)),
		_ => panic!("unknown impetus validator seed: {seed}"),
	}
	.into();
	let keys = ImpetusSessionKeys {
		babe: from_seed::<BabeId>(seed),
		grandpa: from_seed::<GrandpaId>(seed),
		im_online: from_seed::<ImOnlineId>(seed),
		authority_discovery: from_seed::<AuthorityDiscoveryId>(seed),
	};
	(stash, keys)
}

/// Hardhat dev account #4 — the lone genesis nominator on impetus dev-NPoS.
const NOMINATOR_STASH_HEX: [u8; 20] =
	hex_literal::hex!("15d34AAf54267DB7D7c367839AAf71A00a2C6A65");

pub type ChainSpec = sc_service::GenericChainSpec;

const UNITS: Balance = 1_000_000_000_000_000_000;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Network {
	Impetus,
	Impulse,
}

impl Network {
	pub fn from_spec_id(id: &str) -> Self {
		match id {
			"impetus" | "mainnet" | "impetus_dev_npos" => Network::Impetus,
			_ => Network::Impulse,
		}
	}
}

pub struct ChainProfile {
	pub display_name: &'static str,
	pub spec_id: &'static str,
	pub evm_chain_id: u64,
	pub token_symbol: &'static str,
	pub ss58_prefix: u32,
	pub chain_type: ChainType,
	pub manual_seal: bool,
	/// libp2p protocol id — namespaces the gossip/sync protocols so nodes on a
	/// different network never accidentally peer with this one. Must be unique
	/// per network.
	pub protocol_id: &'static str,
}

fn impetus_profile() -> ChainProfile {
	ChainProfile {
		display_name: "Impetus (Dev NPoS)",
		spec_id: "impetus_dev_npos",
		evm_chain_id: 388266,
		token_symbol: "IPT",
		ss58_prefix: 11434,
		// Plan 1 ships only the dev-NPoS spec. ChainType::Development unlocks
		// `--alice` key auto-insertion so smoke runs can author blocks. A
		// future mainnet spec will set Live and require operators to manage
		// session keys explicitly.
		chain_type: ChainType::Development,
		manual_seal: false,
		protocol_id: "impetus",
	}
}

fn impetus_production_profile() -> ChainProfile {
	ChainProfile {
		display_name: "Impetus",
		spec_id: "impetus",
		evm_chain_id: 388266,
		token_symbol: "IPT",
		ss58_prefix: 11434,
		chain_type: ChainType::Live,
		manual_seal: false,
		protocol_id: "impetus",
	}
}

/// Hex-literal placeholders for the production validator stashes, used ONLY on
/// the `IMPETUS_ALLOW_PLACEHOLDER_KEYS=1` rehearsal path. The real launch path
/// derives stashes from the operator-supplied `IMPETUS_VALIDATOR_KEYS_FILE`
/// (each entry already carries its `stash`), so these literals never reach a
/// value-bearing chain.
const PRODUCTION_VALIDATOR_STASHES: [[u8; 20]; 4] = [
	hex_literal::hex!("1111111111111111111111111111111111111111"),
	hex_literal::hex!("2222222222222222222222222222222222222222"),
	hex_literal::hex!("3333333333333333333333333333333333333333"),
	hex_literal::hex!("4444444444444444444444444444444444444444"),
];

/// The dev/testnet admin address whose mnemonic + private key were committed to
/// git history (`infra/.env.example`) and are therefore permanently
/// compromised. The production spec refuses to pin this address as sudo so a
/// real-money launch cannot inherit a publicly-known sudo key.
const BURNED_DEV_SUDO: [u8; 20] = hex_literal::hex!("d2aE0A2139dC83Cb920e3cd7B9F640922D14b872");

/// Obviously-fake sudo placeholder used ONLY on the rehearsal path
/// (`IMPETUS_ALLOW_PLACEHOLDER_KEYS=1` with no `IMPETUS_SUDO_ADDRESS`). It has
/// no known private key, so a rehearsal spec can never be mistaken for a real
/// launch and the burned dev key is never reintroduced.
const PLACEHOLDER_SUDO: [u8; 20] = hex_literal::hex!("dEAD00000000000000000000000000000000dEAD");

/// Resolve the production sudo / admin account.
///
/// Reads `IMPETUS_SUDO_ADDRESS` from the environment and delegates the policy
/// to the pure [`resolve_production_sudo`] so the logic stays unit-testable
/// without process-global env vars.
fn production_sudo_account(allow_placeholder: bool) -> AccountId {
	resolve_production_sudo(std::env::var("IMPETUS_SUDO_ADDRESS").ok(), allow_placeholder)
}

/// Pure policy for the production sudo key.
///
/// `raw` is the `0x`-prefixed H160 supplied per-deployment (so the founding key
/// is never baked into source). On the real launch path
/// (`allow_placeholder == false`) the address is mandatory. The rehearsal path
/// falls back to an obviously-fake [`PLACEHOLDER_SUDO`] (never the burned dev
/// key). The burned dev sudo is rejected on every path.
fn resolve_production_sudo(raw: Option<String>, allow_placeholder: bool) -> AccountId {
	let account = match raw {
		Some(raw) => {
			let h160 = H160::from_str(raw.trim()).unwrap_or_else(|err| {
				panic!("IMPETUS_SUDO_ADDRESS is not a valid 0x H160 address: {err}")
			});
			AccountId::from(h160)
		}
		None => {
			if !allow_placeholder {
				panic!(
					"impetus production chain spec requires IMPETUS_SUDO_ADDRESS \
					(the freshly-generated sudo H160) to be set. The previous dev \
					admin key was committed to git and is permanently compromised. \
					For tests / rehearsals only, set IMPETUS_ALLOW_PLACEHOLDER_KEYS=1."
				);
			}
			AccountId::from(H160::from(PLACEHOLDER_SUDO))
		}
	};

	// Reject the burned dev sudo on EVERY path (including the rehearsal opt-in):
	// pinning a publicly-known sudo key into a ChainType::Live spec is never
	// intended, and there is no legitimate reason to rehearse with it.
	if H160::from(account) == H160::from(BURNED_DEV_SUDO) {
		panic!(
			"refusing to build the impetus production spec with the burned dev \
			sudo address 0xd2aE0A2139dC83Cb920e3cd7B9F640922D14b872 — its mnemonic \
			and private key are public in git history. Generate a fresh key with \
			`cast wallet new-mnemonic --words 24 --accounts 1` and set \
			IMPETUS_SUDO_ADDRESS to its account #0."
		);
	}

	account
}

fn production_validator_accounts() -> [AccountId; 4] {
	PRODUCTION_VALIDATOR_STASHES.map(|bytes| H160::from_slice(&bytes).into())
}

/// Validator stashes for the production genesis: the real stash from each
/// operator key entry when available (the launch path), otherwise the
/// placeholder set (rehearsal path only).
fn production_stashes(real_keys: Option<&[ValidatorKeysEntry]>) -> Vec<AccountId> {
	match real_keys {
		Some(entries) if !entries.is_empty() => {
			entries.iter().map(|e| AccountId::from(e.stash)).collect()
		}
		_ => production_validator_accounts().to_vec(),
	}
}

fn impulse_profile() -> ChainProfile {
	ChainProfile {
		display_name: "Impulse Testnet",
		spec_id: "impulse",
		evm_chain_id: 322644,
		token_symbol: "IPL",
		ss58_prefix: 11348,
		chain_type: ChainType::Live,
		manual_seal: false,
		protocol_id: "impulse",
	}
}

fn dev_profile(manual_seal: bool) -> ChainProfile {
	ChainProfile {
		display_name: "Impulse Dev",
		spec_id: "dev",
		evm_chain_id: 322644,
		token_symbol: "IPL",
		ss58_prefix: 11348,
		chain_type: ChainType::Development,
		manual_seal,
		protocol_id: "impulse-dev",
	}
}

pub fn impetus_config() -> ChainSpec {
	let profile = impetus_profile();
	let wasm = impetus_runtime::WASM_BINARY.expect("Impetus WASM not built");
	ChainSpec::builder(wasm, Default::default())
		.with_name(profile.display_name)
		.with_id(profile.spec_id)
		.with_chain_type(profile.chain_type.clone())
		.with_protocol_id(profile.protocol_id)
		.with_properties(properties(&profile))
		.with_genesis_config_patch(impetus_genesis_patch(
			admin_account(),
			endowed_accounts(),
			profile.evm_chain_id,
		))
		.build()
}

/// Production Impetus mainnet chain spec.
///
/// Differs from `impetus_config()` in three ways:
/// 1. `ChainType::Live` — chain spec is treated as production, no `--alice`
///    keystore auto-insertion.
/// 2. `spec_id = "impetus"` — what the `impetus` / `mainnet` CLI aliases
///    actually resolve to (the dev-NPoS variant remains accessible via
///    `--chain impetus_dev_npos`).
/// 3. Validator stashes are 4 placeholder `0x1111..` through `0x4444..`
///    addresses with **empty** session keys at genesis. Operators must run
///    `scripts/dump-session-keys.ts` and submit `setKeys` via the Session
///    precompile within the first session, otherwise no blocks are produced.
pub fn impetus_production_config() -> ChainSpec {
	// Refuse to build the production spec with deterministic placeholder
	// session keys: those have no private key on any validator host, so the
	// genesis Babe/GRANDPA authority set cannot sign block #1 and the chain
	// stalls forever. Operators supply real keys via
	// `IMPETUS_VALIDATOR_KEYS_FILE`. Tests and intentional dev rehearsals
	// opt out via `IMPETUS_ALLOW_PLACEHOLDER_KEYS=1`.
	let real_keys = load_validator_keys_from_env();
	let allocation = load_allocation_from_env();
	let allow_placeholder =
		std::env::var("IMPETUS_ALLOW_PLACEHOLDER_KEYS").as_deref() == Ok("1");
	if real_keys.is_none() && !allow_placeholder {
		panic!(
			"impetus production chain spec refuses to build with placeholder \
			session keys. Each validator operator must generate session keys \
			offline (e.g. via `subkey generate --scheme sr25519` for babe / \
			im_online / authority_discovery and `--scheme ed25519` for grandpa), \
			submit them to the coordinator, and the coordinator must point \
			`IMPETUS_VALIDATOR_KEYS_FILE` at a JSON array of \
			{{stash, babe, grandpa, im_online, authority_discovery}} entries. \
			Also set `IMPETUS_SUDO_ADDRESS` to the freshly-generated sudo H160 \
			before running `build-spec --chain impetus --raw`. For tests and \
			internal rehearsals only, set `IMPETUS_ALLOW_PLACEHOLDER_KEYS=1`."
		);
	}
	if allocation.is_none() && !allow_placeholder {
		panic!(
			"impetus production chain spec requires IMPETUS_ALLOCATION_FILE \
			(a JSON array of {{address, balance, bucket}} entries totalling 1B IPT). \
			For tests / rehearsals only, set IMPETUS_ALLOW_PLACEHOLDER_KEYS=1."
		);
	}
	impetus_production_config_inner(real_keys, allocation, allow_placeholder)
}

/// Inner builder shared by `impetus_production_config()` and the chain-spec
/// unit tests. `allow_placeholder` is threaded explicitly (rather than read
/// from the environment) so the test suite can exercise the rehearsal path
/// without setting process-global env vars.
fn impetus_production_config_inner(
	real_keys: Option<Vec<ValidatorKeysEntry>>,
	allocation: Option<Vec<AllocationEntry>>,
	allow_placeholder: bool,
) -> ChainSpec {
	let profile = impetus_production_profile();
	let wasm = impetus_runtime::WASM_BINARY.expect("Impetus WASM not built");

	// Validate inputs before resolving the sudo key so a misconfigured launch
	// surfaces the keys problem deterministically. On the real launch path, an
	// empty / absent keys file must NOT silently fall back to the 0x1111..
	// placeholder validators; the rehearsal opt-in is the only placeholder path.
	if !allow_placeholder
		&& real_keys.as_ref().map(|k| k.is_empty()).unwrap_or(true)
	{
		panic!(
			"impetus production launch path requires a non-empty \
			IMPETUS_VALIDATOR_KEYS_FILE (real operator session keys). Refusing to \
			fall back to placeholder validators on a ChainType::Live spec. For \
			tests / rehearsals only, set IMPETUS_ALLOW_PLACEHOLDER_KEYS=1."
		);
	}

	let sudo = production_sudo_account(allow_placeholder);

	ChainSpec::builder(wasm, Default::default())
		.with_name(profile.display_name)
		.with_id(profile.spec_id)
		.with_chain_type(profile.chain_type.clone())
		.with_protocol_id(profile.protocol_id)
		.with_properties(properties(&profile))
		.with_genesis_config_patch(impetus_production_genesis_patch(
			sudo,
			profile.evm_chain_id,
			real_keys,
			allocation,
			allow_placeholder,
		))
		.build()
}

/// Load the operator-supplied genesis session keys JSON if
/// `IMPETUS_VALIDATOR_KEYS_FILE` is set. Returns `None` when the env var is
/// unset (caller decides whether to fall back to placeholders or panic).
fn load_validator_keys_from_env() -> Option<Vec<ValidatorKeysEntry>> {
	let path = std::env::var("IMPETUS_VALIDATOR_KEYS_FILE").ok()?;
	let bytes = std::fs::read(&path).unwrap_or_else(|err| {
		panic!("failed to read IMPETUS_VALIDATOR_KEYS_FILE={path}: {err}")
	});
	let entries: Vec<ValidatorKeysEntry> = serde_json::from_slice(&bytes)
		.unwrap_or_else(|err| {
			panic!("failed to parse IMPETUS_VALIDATOR_KEYS_FILE={path}: {err}")
		});
	Some(entries)
}

pub fn impulse_config() -> ChainSpec {
	let profile = impulse_profile();
	let wasm = impulse_runtime::WASM_BINARY.expect("Impulse WASM not built");
	build_spec(&profile, wasm)
}

pub fn development_config(enable_manual_seal: bool) -> ChainSpec {
	let profile = dev_profile(enable_manual_seal);
	let wasm = impulse_runtime::WASM_BINARY.expect("Impulse WASM not built (used by dev)");
	build_spec(&profile, wasm)
}

fn build_spec(profile: &ChainProfile, wasm: &[u8]) -> ChainSpec {
	ChainSpec::builder(wasm, Default::default())
		.with_name(profile.display_name)
		.with_id(profile.spec_id)
		.with_chain_type(profile.chain_type.clone())
		.with_protocol_id(profile.protocol_id)
		.with_properties(properties(profile))
		.with_genesis_config_patch(genesis_patch(
			admin_account(),
			endowed_accounts(),
			vec![authority_keys_from_seed("Alice")],
			profile.evm_chain_id,
			profile.manual_seal,
		))
		.build()
}

fn properties(profile: &ChainProfile) -> Properties {
	let mut props = Properties::new();
	props.insert("tokenDecimals".into(), 18.into());
	props.insert("tokenSymbol".into(), profile.token_symbol.into());
	props.insert("ss58Format".into(), profile.ss58_prefix.into());
	props.insert("isEthereum".into(), true.into());
	props
}

fn genesis_patch(
	sudo_key: AccountId,
	endowed: Vec<AccountId>,
	initial_authorities: Vec<(AuraId, GrandpaId)>,
	chain_id: u64,
	enable_manual_seal: bool,
) -> serde_json::Value {
	let evm_accounts: BTreeMap<H160, fp_evm::GenesisAccount> = endowed
		.iter()
		.map(|account| {
			(
				H160::from(*account),
				fp_evm::GenesisAccount {
					// No balance here: pallet_balances is the sole funding source. Setting it
					// in both genesis configs double-counts into TotalIssuance.
					balance: U256::zero(),
					code: Default::default(),
					nonce: Default::default(),
					storage: Default::default(),
				},
			)
		})
		.collect();

	serde_json::json!({
		"sudo": { "key": Some(sudo_key) },
		"balances": {
			"balances": endowed
				.iter()
				.cloned()
				.map(|k| (k, 1_000_000u128 * UNITS))
				.collect::<Vec<_>>()
		},
		"aura": {
			"authorities": initial_authorities.iter().map(|x| x.0.clone()).collect::<Vec<_>>()
		},
		"grandpa": {
			"authorities": initial_authorities.iter().map(|x| (x.1.clone(), 1u64)).collect::<Vec<_>>()
		},
		"evmChainId": { "chainId": chain_id },
		"evm": { "accounts": evm_accounts },
		"manualSeal": { "enable": enable_manual_seal },
		"gaslessRegistry": { "rules": [] }
	})
}

fn impetus_genesis_patch(
	sudo_key: AccountId,
	endowed: Vec<AccountId>,
	chain_id: u64,
) -> serde_json::Value {
	let validators: [(AccountId, ImpetusSessionKeys); 4] = [
		impetus_session_keys("Alice"),
		impetus_session_keys("Bob"),
		impetus_session_keys("Charlie"),
		impetus_session_keys("Dave"),
	];
	let nominator: AccountId = H160::from_slice(&NOMINATOR_STASH_HEX).into();

	// Build-time pre-conditions (R10 in the spec): every staked account must
	// already be pre-funded via `endowed_accounts()`, otherwise pallet-staking
	// will silently drop the entry at genesis.
	assert!(
		validators.iter().all(|(s, _)| endowed.contains(s)),
		"every impetus genesis validator stash must be pre-funded via endowed_accounts()"
	);
	assert!(
		endowed.contains(&nominator),
		"impetus genesis nominator stash must be pre-funded via endowed_accounts()"
	);

	let evm_accounts: BTreeMap<H160, fp_evm::GenesisAccount> = endowed
		.iter()
		.map(|account| {
			(
				H160::from(*account),
				fp_evm::GenesisAccount {
					// No balance here: pallet_balances is the sole funding source. Setting it
					// in both genesis configs double-counts into TotalIssuance.
					balance: U256::zero(),
					code: Default::default(),
					nonce: Default::default(),
					storage: Default::default(),
				},
			)
		})
		.collect();

	let stakers: Vec<_> = validators
		.iter()
		.map(|(stash, _)| {
			serde_json::json!([stash, stash, 2_000u128 * UNITS, "Validator"])
		})
		.chain(std::iter::once(serde_json::json!([
			nominator,
			nominator,
			5_000u128 * UNITS,
			{ "Nominator": [validators[0].0, validators[1].0, validators[2].0] }
		])))
		.collect();

	let session_keys: Vec<_> = validators
		.iter()
		.map(|(stash, keys)| serde_json::json!([stash, stash, keys]))
		.collect();

	serde_json::json!({
		"sudo": { "key": Some(sudo_key) },
		"balances": {
			"balances": endowed
				.iter()
				.cloned()
				.map(|k| (k, 1_000_000u128 * UNITS))
				.collect::<Vec<_>>()
		},
		// `babe.authorities` and `grandpa.authorities` are intentionally left
		// empty here -- `pallet-session::GenesisConfig::build` runs after this
		// patch and pumps the genesis authority sets into Babe + GRANDPA via
		// `SessionHandler::on_genesis_session` (which calls
		// `Babe::initialize_genesis_authorities` / `Grandpa::...`). Setting
		// either array here triggers the runtime assertion
		// `Authorities are already initialized!` and the node aborts at
		// `BuildGenesisConfig::build` before producing block #0.
		"babe": {
			"authorities": [],
			"epochConfig": {
				"c": [1, 4],
				"allowed_slots": "PrimaryAndSecondaryVRFSlots",
			},
		},
		"grandpa": { "authorities": [] },
		"session": { "keys": session_keys },
		"staking": {
			"validatorCount": 4u32,
			"minimumValidatorCount": 1u32,
			"invulnerables": [],
			"forceEra": "NotForcing",
			"slashRewardFraction": 100_000_000u32,
			"stakers": stakers,
			"minNominatorBond": 10u128 * UNITS,
			"minValidatorBond": 1_000u128 * UNITS,
			"maxValidatorCount": Some::<u32>(32),
			"maxNominatorCount": Some::<u32>(1024),
		},
		"nominationPools": {
			"minJoinBond": 10u128 * UNITS,
			"minCreateBond": 1_000u128 * UNITS,
			"maxPools": Some::<u32>(32),
			"maxMembers": Some::<u32>(1_024),
			"maxMembersPerPool": Some::<u32>(256),
			"globalMaxCommission": Some::<u32>(100_000_000),
		},
		"treasury": {},
		"evmChainId": { "chainId": chain_id },
		"evm": { "accounts": evm_accounts },
		"gaslessRegistry": { "rules": [] },
	})
}

/// Placeholder allocation entries used on the rehearsal path
/// (`IMPETUS_ALLOW_PLACEHOLDER_KEYS=1`, no allocation file). Four obviously-fake
/// addresses covering all buckets so the 1B assert still holds and `build-spec`
/// works for smoke-testing.
fn placeholder_allocation() -> Vec<AllocationEntry> {
	vec![
		AllocationEntry {
			address: H160::from(hex_literal::hex!(
				"AAAA000000000000000000000000000000000001"
			)),
			balance: 300_000_000u128 * UNITS,
			bucket: "community".to_string(),
		},
		AllocationEntry {
			address: H160::from(hex_literal::hex!(
				"AAAA000000000000000000000000000000000002"
			)),
			balance: 150_000_000u128 * UNITS,
			bucket: "team".to_string(),
		},
		AllocationEntry {
			address: H160::from(hex_literal::hex!(
				"AAAA000000000000000000000000000000000003"
			)),
			balance: 150_000_000u128 * UNITS,
			bucket: "staking_reserve".to_string(),
		},
		AllocationEntry {
			address: H160::from(hex_literal::hex!(
				"AAAA000000000000000000000000000000000004"
			)),
			balance: 50_000_000u128 * UNITS,
			bucket: "liquidity".to_string(),
		},
	]
}

fn impetus_production_genesis_patch(
	sudo_key: AccountId,
	chain_id: u64,
	real_keys: Option<Vec<ValidatorKeysEntry>>,
	allocation: Option<Vec<AllocationEntry>>,
	allow_placeholder: bool,
) -> serde_json::Value {
	// -----------------------------------------------------------------
	// 1. Validator stashes
	// -----------------------------------------------------------------
	// Real launch: stashes come from the operator-supplied key entries (each
	// already carries its `stash`). Rehearsal path (placeholder keys): fall back
	// to the `0x1111..` literals. Never silently mix the two.
	let validator_stashes = production_stashes(real_keys.as_deref());
	assert!(
		!validator_stashes.is_empty(),
		"production genesis needs at least one validator stash"
	);

	// -----------------------------------------------------------------
	// 2. Allocation entries
	// -----------------------------------------------------------------
	// Real launch path requires the file. Rehearsal falls back to obviously-fake
	// placeholders so build-spec still works and the 1B assert holds.
	let alloc_entries = allocation.unwrap_or_else(|| {
		if !allow_placeholder {
			panic!(
				"impetus production launch path requires IMPETUS_ALLOCATION_FILE. \
				For tests / rehearsals only, set IMPETUS_ALLOW_PLACEHOLDER_KEYS=1."
			);
		}
		placeholder_allocation()
	});

	// -----------------------------------------------------------------
	// 3. Treasury pallet account
	// -----------------------------------------------------------------
	let treasury_account: AccountId =
		PalletId(*b"py/trsry").into_account_truncating();

	// -----------------------------------------------------------------
	// 4. Build the canonical balances map
	//    sudo(1_000) + treasury(249_999_000) + each allocation addr
	//    + a fixed 100M validator bucket split evenly across the stashes
	// -----------------------------------------------------------------
	let mut balances: BTreeMap<AccountId, u128> = BTreeMap::new();

	// sudo: 1,000 IPT (gas / operational carve from treasury bucket)
	*balances.entry(sudo_key).or_default() += 1_000u128 * UNITS;

	// treasury pallet account: 249,999,000 IPT
	*balances.entry(treasury_account).or_default() += 249_999_000u128 * UNITS;

	// allocation buckets (community, team, staking_reserve, liquidity)
	for entry in &alloc_entries {
		let account: AccountId = AccountId::from(entry.address);
		*balances.entry(account).or_default() += entry.balance;
	}

	// Validator bucket: a FIXED 100,000,000 IPT split evenly across however many
	// validators the genesis actually has (4 placeholders on the rehearsal path,
	// 5+ real stashes on launch). Splitting the fixed bucket — rather than giving
	// each a fixed 20M — keeps the total at exactly 1B regardless of validator
	// count. Any indivisible remainder goes to the first stash so the bucket sums
	// to exactly 100M.
	const VALIDATOR_BUCKET: u128 = 100_000_000;
	let n = validator_stashes.len() as u128;
	let per_validator = (VALIDATOR_BUCKET / n) * UNITS;
	let remainder = (VALIDATOR_BUCKET * UNITS) - (per_validator * n);
	for (i, stash) in validator_stashes.iter().enumerate() {
		let amount = if i == 0 { per_validator + remainder } else { per_validator };
		*balances.entry(*stash).or_default() += amount;
	}

	// -----------------------------------------------------------------
	// 5. Build-time supply assert: sum the ACTUAL map, must equal 1B IPT.
	//    This cannot drift — it counts every entry we just inserted.
	// -----------------------------------------------------------------
	let total: u128 = balances.values().sum();
	assert_eq!(
		total,
		1_000_000_000u128 * UNITS,
		"genesis total supply {total} != 1,000,000,000 IPT hard cap"
	);

	// -----------------------------------------------------------------
	// 6. Session keys
	// -----------------------------------------------------------------
	let keys_for = |stash: &AccountId| -> ImpetusSessionKeys {
		let h160: H160 = (*stash).into();
		if let Some(entries) = real_keys.as_ref() {
			let entry = entries
				.iter()
				.find(|e| e.stash == h160)
				.unwrap_or_else(|| {
					panic!(
						"IMPETUS_VALIDATOR_KEYS_FILE missing session keys for \
						validator stash {h160:?}"
					)
				});
			return ImpetusSessionKeys {
				babe: BabeId::from(entry.babe),
				grandpa: GrandpaId::from(entry.grandpa),
				im_online: ImOnlineId::from(entry.im_online),
				authority_discovery: AuthorityDiscoveryId::from(entry.authority_discovery),
			};
		}
		// Placeholder path: each role byte differs so pallet-session's
		// uniqueness assertion across all four keys × all validators holds.
		let mut seed = [0u8; 32];
		seed[..20].copy_from_slice(h160.as_bytes());
		let mk = |role_tag: u8| {
			let mut k = seed;
			k[31] = role_tag;
			k
		};
		ImpetusSessionKeys {
			babe: BabeId::from(sp_core::sr25519::Public::from_raw(mk(0x01))),
			grandpa: GrandpaId::from(sp_core::ed25519::Public::from_raw(mk(0x02))),
			im_online: ImOnlineId::from(sp_core::sr25519::Public::from_raw(mk(0x03))),
			authority_discovery: AuthorityDiscoveryId::from(
				sp_core::sr25519::Public::from_raw(mk(0x04)),
			),
		}
	};
	let session_keys: Vec<_> = validator_stashes
		.iter()
		.map(|stash| serde_json::json!([stash, stash, keys_for(stash)]))
		.collect();

	// -----------------------------------------------------------------
	// 7. Stakers: each validator bonds 1,000 IPT (a subset of its share of the
	//    100M validator bucket)
	// -----------------------------------------------------------------
	let stakers: Vec<_> = validator_stashes
		.iter()
		.map(|stash| serde_json::json!([stash, stash, 1_000u128 * UNITS, "Validator"]))
		.collect();
	let validator_count = validator_stashes.len() as u32;

	// `minimumValidatorCount` is the floor below which staking refuses to elect a
	// validator group and the chain halts. Defaults to the full set (safest), but
	// operators can lower it via `IMPETUS_MIN_VALIDATOR_COUNT` to tolerate some
	// validators being offline (e.g. during a hosting-provider redeploy) without
	// halting. Clamped to `1..=validator_count`. For BFT finality the active set
	// should still stay above 2/3 honest, so do not set this below what your
	// operators can keep online.
	let min_validator_count = std::env::var("IMPETUS_MIN_VALIDATOR_COUNT")
		.ok()
		.and_then(|raw| raw.trim().parse::<u32>().ok())
		.map(|n| n.clamp(1, validator_count))
		.unwrap_or(validator_count);

	// -----------------------------------------------------------------
	// 8. EVM accounts register every balances-map address in the EVM with ZERO
	//    balance. pallet_balances is the sole funding source — setting a balance
	//    here too would double-count into TotalIssuance (the 1B assert above only
	//    sums the balances pallet, so it would not catch the duplication).
	// -----------------------------------------------------------------
	let evm_accounts: BTreeMap<H160, fp_evm::GenesisAccount> = balances
		.iter()
		.map(|(account, &_bal)| {
			(
				H160::from(*account),
				fp_evm::GenesisAccount {
					balance: U256::zero(),
					code: Default::default(),
					nonce: Default::default(),
					storage: Default::default(),
				},
			)
		})
		.collect();

	// -----------------------------------------------------------------
	// 9. Vesting schedules for team-bucket addresses
	//    SDK pallet_vesting::GenesisConfig tuple: (AccountId, begin, length, liquid)
	//    locked = free_balance(who) - liquid; per_block = locked / length
	//    begin = 5_256_000 (1 yr @ 6s), length = 10_512_000 (2 yr)
	//
	//    Only the TEAM portion of an address is locked. `liquid` is the rest of
	//    that address's genesis balance — so a single wallet can hold multiple
	//    buckets (community/liquidity/reserve liquid, team vested) and only the
	//    team amount is subject to the cliff. We sum team amounts per address
	//    (in case one address appears in several team entries) and set
	//    liquid = total_balance - team_locked.
	// -----------------------------------------------------------------
	let mut team_locked: BTreeMap<AccountId, u128> = BTreeMap::new();
	for e in alloc_entries.iter().filter(|e| e.bucket == "team") {
		*team_locked.entry(AccountId::from(e.address)).or_default() += e.balance;
	}
	let vesting_schedules: Vec<_> = team_locked
		.iter()
		.map(|(account, &locked)| {
			let total = *balances.get(account).unwrap_or(&0);
			// liquid = everything at this address that is NOT the team grant.
			let liquid = total.saturating_sub(locked);
			serde_json::json!([account, 5_256_000u32, 10_512_000u32, liquid])
		})
		.collect();

	// Convert balances map to the Vec<(AccountId, u128)> shape pallet-balances expects.
	let balances_vec: Vec<_> = balances.into_iter().collect();

	serde_json::json!({
		"sudo": { "key": Some(sudo_key) },
		"balances": {
			"balances": balances_vec
		},
		// `babe.authorities` and `grandpa.authorities` are intentionally left
		// empty here -- `pallet-session::GenesisConfig::build` runs after this
		// patch and pumps the genesis authority sets into Babe + GRANDPA via
		// `SessionHandler::on_genesis_session`. Setting either array here
		// triggers the runtime assertion `Authorities are already initialized!`
		// and the node aborts at `BuildGenesisConfig::build` before producing
		// block #0 (Plan 2 R10 lesson).
		"babe": {
			"authorities": [],
			"epochConfig": {
				"c": [1, 4],
				"allowed_slots": "PrimaryAndSecondaryVRFSlots",
			},
		},
		"grandpa": { "authorities": [] },
		"session": { "keys": session_keys },
		"staking": {
			"validatorCount": validator_count,
			// Defaults to the full genesis set; lower it via
			// IMPETUS_MIN_VALIDATOR_COUNT to tolerate offline validators without
			// halting (never below 1, never above the set size).
			"minimumValidatorCount": min_validator_count,
			// No invulnerables: every validator, including the founding set, must
			// be slashable. An unslashable bootstrap set would make the entire
			// offence -> slashing pipeline a no-op at launch (M-2).
			"invulnerables": Vec::<AccountId>::new(),
			"forceEra": "NotForcing",
			"slashRewardFraction": 100_000_000u32,
			"stakers": stakers,
			"minNominatorBond": 10u128 * UNITS,
			"minValidatorBond": 1_000u128 * UNITS,
			"maxValidatorCount": Some::<u32>(32),
			"maxNominatorCount": Some::<u32>(1024),
		},
		"nominationPools": {
			"minJoinBond": 10u128 * UNITS,
			"minCreateBond": 1_000u128 * UNITS,
			"maxPools": Some::<u32>(32),
			"maxMembers": Some::<u32>(1_024),
			"maxMembersPerPool": Some::<u32>(256),
			"globalMaxCommission": Some::<u32>(100_000_000),
		},
		"treasury": {},
		"vesting": { "vesting": vesting_schedules },
		"evmChainId": { "chainId": chain_id },
		"evm": { "accounts": evm_accounts },
		"gaslessRegistry": { "rules": [] },
	})
}

#[cfg(test)]
mod tests {
	use super::*;
	use sc_chain_spec::ChainSpec as _;

	#[test]
	fn impetus_spec_id_is_impetus_dev_npos() {
		let spec = impetus_config();
		assert_eq!(spec.id(), "impetus_dev_npos");
	}

	#[test]
	fn impetus_spec_has_chain_id_388266() {
		let spec = impetus_config();
		let json: serde_json::Value = serde_json::from_str(&spec.as_json(false).unwrap()).unwrap();
		assert_eq!(
			json["genesis"]["runtimeGenesis"]["patch"]["evmChainId"]["chainId"],
			388266
		);
	}

	#[test]
	fn impetus_spec_has_session_keys_for_4_validators() {
		let spec = impetus_config();
		let json: serde_json::Value = serde_json::from_str(&spec.as_json(false).unwrap()).unwrap();
		let keys = &json["genesis"]["runtimeGenesis"]["patch"]["session"]["keys"];
		assert!(
			keys.as_array().map(|a| a.len() == 4).unwrap_or(false),
			"expected 4 session-key entries at genesis"
		);
	}

	#[test]
	fn impetus_spec_has_staker_entries() {
		let spec = impetus_config();
		let json: serde_json::Value = serde_json::from_str(&spec.as_json(false).unwrap()).unwrap();
		let stakers = &json["genesis"]["runtimeGenesis"]["patch"]["staking"]["stakers"];
		assert!(
			stakers.as_array().map(|a| a.len() == 5).unwrap_or(false),
			"expected 4 validators + 1 nominator in genesis stakers"
		);
	}

	#[test]
	fn impulse_spec_has_chain_id_322644() {
		let spec = impulse_config();
		let json: serde_json::Value = serde_json::from_str(&spec.as_json(false).unwrap()).unwrap();
		assert_eq!(
			json["genesis"]["runtimeGenesis"]["patch"]["evmChainId"]["chainId"],
			322644
		);
	}

	#[test]
	fn dev_spec_enables_manual_seal_when_requested() {
		let spec = development_config(true);
		let json: serde_json::Value = serde_json::from_str(&spec.as_json(false).unwrap()).unwrap();
		assert_eq!(
			json["genesis"]["runtimeGenesis"]["patch"]["manualSeal"]["enable"],
			true
		);
	}

	#[test]
	fn dev_spec_leaves_manual_seal_disabled_by_default() {
		let spec = development_config(false);
		let json: serde_json::Value = serde_json::from_str(&spec.as_json(false).unwrap()).unwrap();
		assert_eq!(
			json["genesis"]["runtimeGenesis"]["patch"]["manualSeal"]["enable"],
			false
		);
	}

	#[test]
	fn impetus_production_spec_is_live() {
		// Rehearsal path: allow_placeholder=true so the builder falls back to
		// the placeholder sudo + placeholder stashes without env vars.
		let spec = impetus_production_config_inner(None, None, true);
		assert_eq!(spec.id(), "impetus");
		assert_eq!(spec.chain_type(), ChainType::Live);
	}

	#[test]
	fn production_genesis_validators_are_slashable_and_min_count_matches_set() {
		// M-2 regression guard: the production genesis must NOT mark any
		// validator invulnerable, and (with IMPETUS_MIN_VALIDATOR_COUNT unset)
		// minimumValidatorCount defaults to the full genesis set.
		let spec = impetus_production_config_inner(None, None, true);
		let json: serde_json::Value = serde_json::from_str(&spec.as_json(false).unwrap()).unwrap();
		let staking = &json["genesis"]["runtimeGenesis"]["patch"]["staking"];
		let invulnerables = staking["invulnerables"].as_array().expect("invulnerables array");
		assert!(invulnerables.is_empty(), "production genesis must have no invulnerables");
		let validator_count = staking["validatorCount"].as_u64().expect("validatorCount");
		let min = staking["minimumValidatorCount"].as_u64().expect("minimumValidatorCount");
		assert!(min >= 1, "minimumValidatorCount must be positive");
		assert!(min <= validator_count, "minimumValidatorCount must not exceed the set size");
		// Default (no env override) is the full set.
		if std::env::var("IMPETUS_MIN_VALIDATOR_COUNT").is_err() {
			assert_eq!(min, validator_count, "default minimumValidatorCount is the full set");
		}
	}

	#[test]
	fn impetus_alias_loads_production_not_dev() {
		let prod = impetus_production_config_inner(None, None, true);
		let dev = impetus_config();
		assert_eq!(prod.id(), "impetus");
		assert_eq!(dev.id(), "impetus_dev_npos");
		assert_ne!(prod.chain_type(), dev.chain_type());
	}

	#[test]
	fn production_genesis_total_supply_is_exactly_one_billion() {
		// Hard-cap regression guard: the sum of all genesis balances must be
		// exactly 1,000,000,000 IPT regardless of validator count (the validator
		// bucket is a fixed 100M split evenly). Rehearsal path for determinism.
		let spec = impetus_production_config_inner(None, None, true);
		let json: serde_json::Value = serde_json::from_str(&spec.as_json(false).unwrap()).unwrap();
		let balances = json["genesis"]["runtimeGenesis"]["patch"]["balances"]["balances"]
			.as_array()
			.expect("balances array");
		let total: u128 = balances
			.iter()
			.map(|e| e[1].as_u64().map(u128::from).unwrap_or_else(|| {
				// large values serialize as numbers that may exceed u64; parse the raw token
				e[1].to_string().trim_matches('"').parse::<u128>().expect("balance u128")
			}))
			.sum();
		assert_eq!(
			total,
			1_000_000_000u128 * UNITS,
			"genesis total supply must be exactly 1,000,000,000 IPT (hard cap)"
		);
	}

	#[test]
	fn production_genesis_has_team_vesting_schedule() {
		// Vesting regression guard: the team allocation bucket must produce a
		// vesting genesis entry with the 1-year cliff + 2-year linear encoding.
		let spec = impetus_production_config_inner(None, None, true);
		let json: serde_json::Value = serde_json::from_str(&spec.as_json(false).unwrap()).unwrap();
		let vesting = json["genesis"]["runtimeGenesis"]["patch"]["vesting"]["vesting"]
			.as_array()
			.expect("vesting array");
		assert!(!vesting.is_empty(), "team bucket must yield a vesting schedule");
		let entry = &vesting[0];
		assert_eq!(entry[1].as_u64(), Some(5_256_000), "1-year cliff begin block");
		assert_eq!(entry[2].as_u64(), Some(10_512_000), "2-year linear length");
		// liquid = (total balance at the team address) - (team grant). With the
		// placeholder allocation the team address holds ONLY the 150M team grant,
		// so liquid must be 0 (the whole balance is locked). When an operator
		// reuses one wallet across buckets, liquid would be the non-team portion.
		let liquid = entry[3].to_string().trim_matches('"').parse::<u128>().expect("liquid u128");
		assert_eq!(liquid, 0, "placeholder team address holds only the team grant -> fully locked");
	}

	#[test]
	fn production_genesis_does_not_fund_hardhat_dev_accounts() {
		// C-3 regression guard: the production balances set must NOT include any
		// of the public Hardhat mnemonic addresses (only sudo + validator
		// stashes are endowed). Uses the rehearsal path for determinism.
		let spec = impetus_production_config_inner(None, None, true);
		let json: serde_json::Value = serde_json::from_str(&spec.as_json(false).unwrap()).unwrap();
		let balances = json["genesis"]["runtimeGenesis"]["patch"]["balances"]["balances"]
			.as_array()
			.expect("balances array");
		let funded: Vec<String> = balances
			.iter()
			.filter_map(|entry| entry.get(0).and_then(|a| a.as_str()).map(|s| s.to_lowercase()))
			.collect();
		// Hardhat #0 and #5 (a non-validator dev account) must be absent.
		assert!(
			!funded.iter().any(|a| a.contains("f39fd6e51aad88f6f4ce6ab8827279cfffb92266")),
			"production genesis must not fund Hardhat dev account #0"
		);
		assert!(
			!funded.iter().any(|a| a.contains("9965507d1a55bcc2695c58ba16fb37d819b0a4dc")),
			"production genesis must not fund Hardhat dev account #5"
		);
	}

	#[test]
	#[should_panic(expected = "burned dev")]
	fn production_sudo_rejects_burned_dev_key() {
		// C-2 regression guard: the compromised dev sudo address must be
		// rejected on the real launch path even if supplied explicitly.
		let burned = "0xd2aE0A2139dC83Cb920e3cd7B9F640922D14b872".to_string();
		let _ = resolve_production_sudo(Some(burned), false);
	}

	#[test]
	#[should_panic(expected = "IMPETUS_SUDO_ADDRESS")]
	fn production_sudo_requires_address_on_launch_path() {
		// C-2: the real launch path refuses to fall back to the dev admin key.
		let _ = resolve_production_sudo(None, false);
	}

	#[test]
	fn production_sudo_accepts_fresh_address() {
		let fresh = "0x00000000000000000000000000000000000000aa".to_string();
		let account = resolve_production_sudo(Some(fresh), false);
		assert_eq!(
			H160::from(account),
			H160::from_str("0x00000000000000000000000000000000000000aa").unwrap()
		);
	}

	#[test]
	#[should_panic(expected = "burned dev")]
	fn production_sudo_rejects_burned_dev_key_even_on_rehearsal() {
		// C-2 hardening: the burned dev sudo is rejected on EVERY path, including
		// the placeholder/rehearsal opt-in.
		let burned = "0xd2aE0A2139dC83Cb920e3cd7B9F640922D14b872".to_string();
		let _ = resolve_production_sudo(Some(burned), true);
	}

	#[test]
	fn rehearsal_sudo_is_obviously_fake_not_burned_dev_key() {
		// C-2 hardening: rehearsal fallback is the dEAD..dEAD placeholder, never
		// the burned dev admin key.
		let account = resolve_production_sudo(None, true);
		let h160 = H160::from(account);
		assert_eq!(h160, H160::from(PLACEHOLDER_SUDO));
		assert_ne!(h160, H160::from(BURNED_DEV_SUDO));
	}

	#[test]
	#[should_panic(expected = "IMPETUS_VALIDATOR_KEYS_FILE")]
	fn production_launch_rejects_empty_validator_keys() {
		// C-4 hardening: a non-rehearsal build with no real keys must panic
		// rather than silently fall back to placeholder validators.
		let _ = impetus_production_config_inner(Some(vec![]), None, false);
	}

	#[test]
	fn impetus_production_accepts_operator_supplied_keys() {
		// Real-key path: every stash entry resolves to a SessionKeys value
		// built from operator pubkeys (just sr25519/ed25519 32-byte literals
		// for the test; pallet-session genesis only checks pairwise
		// uniqueness, not signature validity).
		let entries: Vec<ValidatorKeysEntry> = production_validator_accounts()
			.into_iter()
			.enumerate()
			.map(|(i, stash)| {
				let mut babe = [0u8; 32];
				let mut grandpa = [0u8; 32];
				let mut im_online = [0u8; 32];
				let mut authority_discovery = [0u8; 32];
				babe[31] = (i as u8) * 4 + 1;
				grandpa[31] = (i as u8) * 4 + 2;
				im_online[31] = (i as u8) * 4 + 3;
				authority_discovery[31] = (i as u8) * 4 + 4;
				ValidatorKeysEntry {
					stash: stash.into(),
					babe: sp_core::sr25519::Public::from_raw(babe),
					grandpa: sp_core::ed25519::Public::from_raw(grandpa),
					im_online: sp_core::sr25519::Public::from_raw(im_online),
					authority_discovery: sp_core::sr25519::Public::from_raw(authority_discovery),
				}
			})
			.collect();
		// allow_placeholder=true keeps the test free of process-global env vars
		// (the sudo address falls back to the dev admin); we only assert on the
		// session-key set, which is driven by the real entries.
		let spec = impetus_production_config_inner(Some(entries), None, true);
		let json: serde_json::Value = serde_json::from_str(&spec.as_json(false).unwrap()).unwrap();
		// Session keys array length should match validator count.
		let session_keys =
			json["genesis"]["runtimeGenesis"]["patch"]["session"]["keys"]
				.as_array()
				.expect("session.keys array");
		assert_eq!(session_keys.len(), 4);
	}
}
