# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build

Requires Rust with the `wasm32-unknown-unknown` target installed (`rustup target add wasm32-unknown-unknown`).

```bash
cargo build --release                        # full release build (binary: target/release/impetus-node)
cargo check                                  # fast type-check without linking
SKIP_WASM_BUILD=1 cargo check                # skip WASM compilation for faster iteration
cargo test                                   # run all unit tests
cargo test -p pallet-gasless-registry        # run tests for a single crate
cargo test -p precompile-batch -- --nocapture # run a specific crate with stdout
cargo test -p precompile-batch test_name     # filter by test name
cargo clippy -- -D warnings                  # lint (treat warnings as errors)
cargo fmt --check                            # check formatting
```

The WASM build is slow; use `SKIP_WASM_BUILD=1` when you only need to verify Rust compilation.
Each runtime crate (`runtime-impetus`, `runtime-impulse`) builds its own WASM blob, so a full
release build compiles WASM twice.

## Run node

```bash
# Dev mode: alias of impulse with pre-funded Hardhat dev users.
# By default it runs Aura authoring; add `--sealing manual` (or bare
# `--sealing`) to switch to RPC-driven block creation via engine_createBlock.
./target/release/impetus-node --chain dev --tmp --alice --sealing manual

# Impetus mainnet (validator locally)
./target/release/impetus-node --chain impetus --tmp --validator

# Impulse testnet (validator locally)
./target/release/impetus-node --chain impulse --tmp --validator
```

- RPC: `http://127.0.0.1:9944`
- `--sealing` (dev only, optional) enables consensus-bypass sealing. Without
  the flag, `--chain dev` runs normal Aura slot-based authoring. Pass
  `--sealing manual` (or bare `--sealing`, which defaults to manual) to mine
  blocks on demand via the `engine_createBlock` RPC. Pass `--sealing instant`
  to mine a block as soon as any transaction enters the pool.
- An empty `--chain` argument resolves to impulse (testnet)

## Architecture

Substrate solochain with EVM compatibility via Frontier. Pinned to polkadot-sdk
**`stable2603`** and frontier **`stable2603`** (workspace `Cargo.toml`).

### Chains

| Chain                | Role     | Chain ID | Token | Decimals | SS58  | Consensus               | `spec_id`            |
|----------------------|----------|----------|-------|----------|-------|-------------------------|----------------------|
| Impetus (Dev NPoS)   | dev/mainnet target | 388266 | IPT | 18 | 11434 | **Babe + Grandpa**     | `impetus_dev_npos` (also `impetus` / `mainnet` aliases) |
| Impulse              | testnet  | 322644   | IPL   | 18       | 11348 | Aura + Grandpa          | `impulse`            |
| dev                  | local    | 322644   | IPL   | 18       | 11348 | Aura (+ optional manual seal) | `dev` (impulse alias; `--sealing` is optional) |

Both runtimes share configuration via `runtimes/common` but compile to separate
WASM blobs with different chain IDs, tokens, and SS58 prefixes.

### Workspace crates

| Crate | Path | Purpose |
|-------|------|---------|
| `impetus-node` | `node/` | Node binary: CLI, service, RPC, chain spec selection |
| `runtime-impetus` | `runtimes/impetus/` | Mainnet runtime WASM |
| `runtime-impulse` | `runtimes/impulse/` | Testnet/dev runtime WASM |
| `runtime-common` | `runtimes/common/` | Shared pallet config, weights, `FrontierPrecompiles`, `gasless` runner, helpers |
| `pallet-gasless-registry` | `pallets/gasless-registry/` | Admin-managed gasless tx rules |
| `precompile-gasless-registry` | `precompiles/gasless-registry/` | EVM precompile at `0x0800` (2048) wrapping the registry |
| `precompile-batch` | `precompiles/batch/` | EVM precompile at `0x0808` (2056) for atomic / best-effort batched calls |

The node selects a runtime via `node/src/command.rs`; chain spec id determines
which `Network` variant (`Impetus` / `Impulse`) is plumbed through CLI subcommands
(benchmarks, key-pair generation, etc.).

### Runtime pallets

**Impulse (testnet, dev) — unchanged from Plan 0:**

0-System, 1-Timestamp, 2-Aura, 3-Grandpa, 4-Balances, 5-TransactionPayment,
6-Sudo, 7-Ethereum, 8-EVM, 9-EVMChainId, 10-BaseFee, 11-ManualSeal, 12-Assets,
14-GaslessRegistry. Index 13 is intentionally skipped.

**Impetus (NPoS, Plan 2 — full NPoS pallet stack):**

0-System, 1-Timestamp, **2-Babe**, 3-Grandpa, 4-Balances, 5-TransactionPayment,
6-Sudo, 7-Ethereum, 8-EVM, 9-EVMChainId, 10-BaseFee, 11-ManualSeal (idle),
12-Assets, 14-GaslessRegistry, **17-Authorship**, **18-Session**,
**19-Historical**, **20-Staking**, **21-Offences**,
**22-ElectionProviderMultiPhase**, **23-VoterList** (bags-list Instance1),
**24-AuthorityDiscovery**, **25-ImOnline**, **26-NominationPools**,
**27-Treasury**, **28-FastUnstake**.

Plan 2 swaps Babe `EpochChangeTrigger` to `ExternalTrigger` (driven by
`pallet-session`), routes `Authorship::FindAuthor` through
`pallet_session::FindAccountFromAuthorIndex<Self, Babe>` (so EVM
`block.coinbase` reflects the elected validator), wires the full Polkadot
staking economics with `Slash = Treasury`, and ships hardcoded
4-validator + 1-nominator genesis (Hardhat #0–#3 stash, //Alice..//Dave
session keys; Hardhat #4 nominator targeting Alice/Bob/Charlie). 15
runtime integration tests in `runtimes/impetus/tests/` pin era progression,
session rotation, slashing, nomination pools, fast-unstake, and Babe
epoch advancement. `spec_version: 4`.

Plan 3 ships the 7 NPoS precompiles at `0x0810`..`0x0840`, the production
mainnet spec (replacing dev Hardhat authority material), the Treasury
SpendOrigin unlock (Plan 2 NeverEnsureOrigin → EnsureRootWithSuccess), 
and E2E suites. `spec_version: 5` on impetus, `2` on impulse (unchanged).

### Gasless transaction system

The chain's distinguishing feature. Three components work together:

1. **`pallet-gasless-registry`** (`pallets/gasless-registry/src/lib.rs`) -- stores rules keyed by `(contract_address, selector)`. Each rule has `enabled` flag and `min_value`. The `evaluate()` function checks if a call matches a rule and is under `MaxGaslessGasLimit` (5M gas).
2. **`runtimes/common/src/gasless.rs`** -- `GaslessEvmRunner` wraps `pallet_evm::runner::stack::Runner`, injecting call context via `environmental!` so the fee handler can inspect target/input/value/gas_limit. `GaslessEvmFee` implements `OnChargeEVMTransaction` and skips fee withdrawal when the call matches a gasless rule.
3. **`precompiles/gasless-registry/src/lib.rs`** -- EVM precompile at `0x0800` with `getRule`, `isGasless`, `setRule`, `removeRule`. Write functions are sudo-gated.

### Batch precompile (`0x0808`)

Atomic / best-effort multi-call dispatcher modeled on Moonbeam's batch pattern:

- **Solidity entries** (non-payable): `batchSome`, `batchSomeUntilFailure`,
  `batchAll(address[], uint256[], bytes[], uint64[])`.
- **Caller-funded transfers**: sub-call `Transfer.source` is the address that
  called the precompile (EOA or intermediate contract — i.e.
  `handle.context().caller`), **not** the precompile itself. Sub-call
  `Context.caller` is set to that same address, so sub-calls see
  `msg.sender = the immediate caller of the precompile`. Matches Moonbeam
  semantics; the original `tx.from` is **not** propagated through
  intermediate contracts.
- **DELEGATECALL / CALLCODE rejected**: dispatch reverts if
  `handle.code_address() != handle.context().address`, preventing a
  delegatecaller from draining native value from the EOA without explicit
  `msg.value` authorization.
- **Fatal exits** (`ExitReason::Fatal`) bubble up unchanged in every mode;
  reverts only stop execution per mode contract.
- **Events**: `SubcallSucceeded(uint256 index)`,
  `SubcallFailed(uint256 index)` emitted from the precompile address.
- Codec bounds: `MAX_BATCH_SIZE = 256` sub-calls, `CALL_DATA_LIMIT = 2 MiB`
  per sub-call.
- Self-call (target == precompile) is rejected to avoid reentrant gas blowups.

`PrecompileSet` wiring lives in `runtimes/common/src/precompiles.rs`. Updating
`used_addresses()` and the `match` arm together is required when adding a new
precompile.

### Precompile address map

| Address       | Precompile        | Crate                         |
|---------------|-------------------|-------------------------------|
| `0x01–0x05`   | Ethereum stdlib   | upstream                      |
| `0x0400–0x0403` | curve25519 + Sha3FIPS + ECRecoverPK | upstream    |
| `0x0800` (2048) | GaslessRegistry | precompile-gasless-registry   |
| `0x0808` (2056) | Batch           | precompile-batch              |
| `0x0810` (2064) | Staking         | precompile-staking            |
| `0x0818` (2072) | Session         | precompile-session            |
| `0x0820` (2080) | NominationPools | precompile-nomination-pools   |
| `0x0828` (2088) | FastUnstake     | precompile-fast-unstake       |
| `0x0830` (2096) | Treasury        | precompile-treasury           |
| `0x0838` (2104) | BagsList        | precompile-bags-list          |
| `0x0840` (2112) | StakingAdmin    | precompile-staking-admin      |

### Service (`node/src/service/`)

`service/` is a directory module split into:

- `common.rs` — `new_partial` skeleton, client + Frontier backend + GRANDPA block import wiring, registered `Arc::new(GrandpaPruningFilter)` (required after the stable2603 bump).
- `aura.rs` — Aura import queue + authoring; powers `--chain impulse` and `--chain dev`.
- `babe.rs` — Babe import queue + authoring + authority-discovery worker; powers `--chain impetus_dev_npos` (and aliases `impetus`, `mainnet`).

`command.rs::load_spec` resolves the chain spec id to a `ChainSpec`, and `service::build_full` dispatches via `Network::from_spec_id`:
- `"impetus" | "mainnet" | "impetus_dev_npos"` → `service::babe::new_full`
- everything else → `service::aura::new_full`

A single `impetus-node` binary handles both consensus paths.

**Plan 3 status:** All 7 NPoS precompiles (0x0810-0x0840) are live and
wired into FrontierPrecompilesNpos. Production chain spec
(`impetus_production_config`) routes the `impetus` / `mainnet` aliases; the
dev variant (`impetus_dev_npos`) remains for local testing. The Plan 2
babe-worker init bug is fixed (commit 6301fcc — worker handle kept alive
via task_manager, dev keystore injection for 4-key NPoS set).

**Production launch requires real operator session keys.** The previous
placeholder approach (deterministic per-stash bytes) could not author
block #1 because no validator held the matching private key. The chain
spec now refuses to build with placeholders unless
`IMPETUS_ALLOW_PLACEHOLDER_KEYS=1` (test/internal-rehearsal only). Real
launch workflow:

1. Each validator operator generates session keys offline, e.g.
   `subkey generate --scheme sr25519` for babe / im_online /
   authority_discovery and `subkey generate --scheme ed25519` for
   grandpa. They keep the seed private and submit the four pubkeys plus
   their stash H160 to the coordinator.
2. The coordinator assembles a JSON array, one entry per validator:
   `[{"stash":"0x...","babe":"0x...","grandpa":"0x...","im_online":"0x...","authority_discovery":"0x..."}, ...]`
3. Coordinator builds the live spec with the keys baked in:
   `IMPETUS_VALIDATOR_KEYS_FILE=validators.json impetus-node build-spec --chain impetus --raw > impetus.json`
4. `impetus.json` is distributed to every operator (same bytes — chain
   spec must be byte-identical across the network).
5. Each operator starts `impetus-node --chain impetus.json
   --validator ...` and uses `author_insertKey` to load their private
   seeds into the keystore before block #1.

The `runtime-impetus::sp_authority_discovery::AuthorityDiscoveryApi`
returns `AuthorityDiscovery::authorities()` so the DHT worker
publishes/discovers under the operator's actual authority_discovery key
(not the babe key projected onto the same type). Operators may rotate
authority_discovery to a key distinct from babe without going dark on
the DHT.

Single-node dev authoring is now gated by
`IMPETUS_INJECT_ALL_DEV_KEYS=1` (set this env var to bulk-inject all four
`//Alice..//Dave` session keys into one keystore and auto-enable
`force_authoring`; without it, only the seed selected by `--alice` /
`--bob` / ... is injected so multi-node setups avoid equivocation). Live
`impetus_dev_npos` single-node authoring verified: 29 blocks / era 3
within 180s under runtime-test-fast. Treasury SpendOrigin:
`NeverEnsureOrigin` → `EnsureRootWithSuccess` (T18 unlock —
`Sudo::sudo(...)` is the only externally-reachable spend path until
Plan 4 governance lands).

**Plan 3 E2E status: 13/19 specs pass, 1 pending, 6 fail.** The 12-13
read-only specs (treasury views, fast-unstake views, bags-list,
staking-admin sudo-reverts, delegatecall staticCall) pass deterministically.
The 6-7 tx-submitting specs (validator/nominator/rebond lifecycle, pools
lifecycle, delegatecall-guard proxy deployment) intermittently fail
because the proposer occasionally drifts past a slot boundary under
runtime-test-fast (30s sessions) on a multitasking macOS host running
hardhat + node in the same process group. The runtime then panics on
`pallet_babe::on_finalize`'s `Timestamp slot must match CurrentSlot`
assertion, rolling back the block and evicting the test's tx from the
pool, leaving `tx.wait()` hanging until mocha timeout. This is **not a
production concern** — production has 10-min sessions, dedicated
validator hardware, release-optimized binaries, and `slot_proportion=0.5`
caps proposal time at 3s. The 4-validator runtime integration tests in
`runtimes/impetus/tests/` already cover the tx-heavy lifecycle paths at
production timing. To run the full E2E suite reliably, use Linux native
(GitHub Actions ubuntu-latest works; Docker Desktop on macOS does not
help because it is a VM sharing CPU with the host scheduler). The
`apps/node/scripts/run-impetus-4node.sh` script reproduces production
NPoS topology locally for diagnostic work but does not change the
underlying drift problem. A future plan to wire `sc_consensus_manual_seal`
+ `BabeConsensusDataProvider` into the Babe service path would let
`--sealing instant` bypass the slot timer for dev/E2E and lift the
ceiling toward 18/19 without affecting production authoring.

### Genesis and accounts

- Sudo/admin: `0xd2aE0A2139dC83Cb920e3cd7B9F640922D14b872` (project admin mnemonic, not Hardhat)
- Dev users: Hardhat mnemonic accounts #0–#9, pre-funded with 1M IPT / IPL each
- Existential deposit: 0

## Releasing a runtime upgrade

Bump `spec_version` in both `runtimes/impetus/src/lib.rs` and
`runtimes/impulse/src/lib.rs` whenever runtime logic, pallet config, or
precompile registration changes. Current `spec_version` is `5` on impetus
(Plan 3 Treasury SpendOrigin unlock — bumped from Plan 2's `4`), `2` on
impulse (unchanged).

## Fuzzing

```bash
SKIP_WASM_BUILD=1 cargo ziggy fuzz -j$(nproc) -t5
```

Uses ziggy (AFL++ / honggfuzz) with structure-aware fuzzing via `arbitrary`. See `fuzz/README.md`.

## Key constants

| Constant | Value | Location |
|----------|-------|----------|
| Block gas limit | 75,000,000 | `runtimes/common/src/lib.rs` (`BLOCK_GAS_LIMIT`) |
| Max gasless gas limit | 5,000,000 | `runtimes/{impetus,impulse}/src/lib.rs` (`MaxGaslessGasLimit`) |
| Block time | 6 s | `runtimes/common/src/lib.rs` (`MILLISECS_PER_BLOCK`) |
| Existential deposit | 0 | `runtimes/common/src/lib.rs` (`EXISTENTIAL_DEPOSIT`) |
| Max batch size | 256 sub-calls | `precompiles/batch/src/lib.rs` (`MAX_BATCH_SIZE`) |
| Batch call-data limit | 2 MiB per sub-call | `precompiles/batch/src/lib.rs` (`CALL_DATA_LIMIT`) |
| Batch base / per-subcall gas overhead | 1,000 / 1,500 | `precompiles/batch/src/lib.rs` |
| Precompile address — gasless registry | `0x0800` (2048) | `precompiles/gasless-registry/src/lib.rs` |
| Precompile address — batch | `0x0808` (2056) | `precompiles/batch/src/lib.rs` |

### NPoS timings (impetus only)

Defined in `runtimes/common/src/staking_constants.rs` (dev-fast profile).
A `runtime-test-fast` Cargo feature compresses these for runtime
integration tests and Plan 3 E2E runs (30 s sessions, 1 min eras).

| Constant              | Default (Plan 2)    | runtime-test-fast |
|-----------------------|---------------------|-------------------|
| BLOCKS_PER_SESSION    | 100 (10 min)        | 5 (30 s)          |
| SESSIONS_PER_ERA      | 6 (60 min)          | 2 (1 min)         |
| BONDING_DURATION_ERAS | 4 (~4 h)            | 2 (~2 min)        |
| SLASH_DEFER_DURATION  | 3                   | 1                 |
| MIN_VALIDATOR_BOND    | 1,000 IPT           | same              |
| MIN_NOMINATOR_BOND    | 10 IPT              | same              |
| MAX_VALIDATOR_COUNT   | 32                  | same              |
