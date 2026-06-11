# Impetus Launch Checklist

Mainnet readiness audit conducted 2026-05-30 across 8 dimensions (consensus,
genesis, precompiles, economics, runtime/governance, node/RPC, ops) with
adversarial verification, plus direct code ground-truthing of every CRITICAL.

**Verdict: NO-GO for a real-money mainnet on the current configuration.
Canary-first is the correct path.** The consensus engine (BABE/GRANDPA, NPoS,
precompiles, election anti-halt) is wired correctly. The blockers live in
genesis data, committed secrets, economic parameters, and ops — not in the
engine logic.

Severity rubric (mainnet context): CRITICAL = funds loss / chain halt / total
compromise. HIGH = privilege escalation / economic exploit / launch that cannot
be safely recovered without a hard fork. Each item lists the file:line evidence
and a verify step.

---

## Canary Checklist

Goal: a chain that runs without leaking funds or keys. Low value, trusted
validators, clearly labeled "not final", willing to redeploy / hard-fork. Almost
entirely data / config / ops — no new logic required.

### C-1. Rotate all committed secrets (absolute blocker)

- **File:** `infra/.env.example:8,20,24` — three real 24-word mnemonics in plaintext (ADMIN + 2 relayers).
- **Verified:** `cast wallet address --mnemonic "<line 8>"` → `0xd2aE0A2139dC83Cb920e3cd7B9F640922D14b872`, the exact sudo address pinned in genesis. Anyone with repo access owns sudo (`set_code`, treasury, force-era, every precompile admin gate) and can drain sudo's 1M IPT.
- [ ] `cast wallet new-mnemonic --words 24 --accounts 1` → new ADMIN mnemonic (store off-tree, e.g. Infisical).
- [ ] Replace the mnemonics in `infra/.env.example` with empty placeholders (match the root `.env.example` style).
- [ ] Rotate the two relayer mnemonics (`infra/.env.example:20,24`).
- [ ] If the repo was ever public: treat all three keys as permanently burned; scrub git history (`git filter-repo`).

### C-2. Update the sudo address in genesis (pairs with C-1)

- **File:** `apps/node/runtimes/common/src/genesis_helpers.rs:10-12` — `admin_account()` hardcodes `d2aE0A...b872`.
- [ ] Replace the hex with account #0 of the new ADMIN mnemonic.
- **Verify:** `grep -n d2aE apps/node/runtimes/common/src/genesis_helpers.rs` → no match.

### C-3. Remove Hardhat dev-account funding from production genesis (absolute blocker)

- **File:** `apps/node/runtimes/common/src/genesis_helpers.rs:21-39` (`mnemonic_accounts()` / `endowed_accounts()`), consumed at `apps/node/node/src/chain_spec.rs:234`.
- **Problem:** the production path endows all 10 public Hardhat addresses (mnemonic `test test ... junk`) with 1,000,000 IPT each, in both `balances` and `evm.accounts`. Anyone can derive those keys and drain ~10M IPT at block 0.
- [ ] Add a `production_endowed_accounts()` that returns only the new admin + treasury — NOT `mnemonic_accounts()`.
- [ ] Use it in `impetus_production_config_inner` (`chain_spec.rs:234`).
- **Verify:** build the spec, then `grep -c f39Fd6 chain-specs/impetus.json` → `0`.

### C-4. Replace placeholder validator stashes with real ones

- **File:** `apps/node/node/src/chain_spec.rs:133-142` — `PRODUCTION_VALIDATOR_STASHES = [0x1111.., 0x2222.., 0x3333.., 0x4444..]`.
- **Problem:** the only build-time guard checks the stashes are endowed (the inner builder force-endows them, so it always passes). Session keys are guard-gated; stashes are not.
- [ ] Replace the 4 stashes with real operator validator addresses.
- [ ] Collect real session keys into a JSON for `IMPETUS_VALIDATOR_KEYS_FILE` (each entry: `stash, babe, grandpa, im_online, authority_discovery` — operators generate offline via `subkey generate`).

### C-5. Rebuild and commit a clean chain spec

- **File:** `chain-specs/impetus.json` — currently stale (2026-05-10, pre-NPoS), embeds public `//Alice` BABE/GRANDPA keys, `bootNodes=[]`, `protocolId=None`, no telemetry.
- [ ] Generate a bootnode key: `impetus-node key generate-node-key`.
- [ ] Build: `IMPETUS_VALIDATOR_KEYS_FILE=validators.json impetus-node build-spec --chain impetus --raw > chain-specs/impetus.json`
- [ ] Add real `bootNodes`, `telemetryEndpoints`, and a unique `protocolId` (e.g. `"impetus"`) to the spec.
- [ ] Distribute the byte-identical `impetus.json` to every operator.
- **Verify:** `python3 -c "import json;d=json.load(open('chain-specs/impetus.json'));print(len(d['bootNodes']), d['protocolId'])"` → bootnode count `>0`, protocolId not `None`.

### C-6. Lock down RPC (CLI flags — no code change)

- **Related:** `apps/node/node/src/rpc/mod.rs`, `apps/node/node/src/command.rs`.
- [ ] Run validators with `--rpc-methods safe`; do not expose `--rpc-external` publicly; set an explicit `--rpc-cors`.
- [ ] Audit `infra/docker-compose.yml` and root `docker-compose.yml` — ensure they do not expose unsafe RPC externally and do not reuse the well-known dev node key (OPS-4).
- Note: `debug_*` / `txpool_*` namespaces ship enabled; gate or firewall them on public endpoints.

### C-7. Minimal CI, bootnode, and monitoring

- **File:** `.github/workflows/` (currently only `docker-publish.yml`, `runtime-benchmarks.yml`).
- [ ] Add a workflow running `cargo build`, `cargo test`, `cargo clippy -- -D warnings`, `cargo fmt --check`.
- [ ] Deploy at least one managed bootnode.
- [ ] Basic monitoring: block production, finality lag, peer count (`--prometheus-external` + Grafana). Note: `infra/oz-*` is an Artemis<->Base bridge, unrelated to this chain.

### C-8. Clean up brand leakage and stale artifacts

- [ ] `infra/.env.example`, `apps/node/Dockerfile` (default `CMD ["--chain", "impulse"]`), `infra/oz-relayer/config/config.json` still say "Artemis"/"ART" — fix or confirm intentional.
- [ ] Remove committed build artifacts: `infra/subscan/ui/.next/`.

### Canary order

C-1 -> C-2 -> C-3 -> C-4 -> C-5 form a single chain (secrets -> genesis data ->
rebuild spec). Then C-6 / C-7 / C-8 in parallel.

---

## Mainnet Checklist (real money)

Do this AFTER the entire canary checklist passes. This section includes
recompiles AND new code — not solvable by configuration alone.

> Includes everything in the Canary Checklist, plus:

### M-1. Production staking timing (compile-time; bump spec_version) — DONE

- **File:** `apps/node/runtimes/common/src/staking_constants.rs`.
- [x] Polkadot-style timing: 4h sessions, 24h eras, **28-era (~28-day) bonding**, 27-era slash-defer. Invariant holds: bonding(28) <= HistoryDepth(84), slash-defer(27) < bonding(28).
- [x] Also widened `grandpa::MaxSetIdSessionEntries` to `BondingSessionEntries` (bonding x sessions) so GRANDPA equivocations stay reportable across the full unbonding window.
- [x] `spec_version` bumped 5 -> 6 (`apps/node/runtimes/impetus/src/lib.rs`).
- **Verify:** clippy `-D warnings` clean; 18/18 chain_spec tests pass (full WASM build).

### M-2. Slashable and larger validator set — DONE (genesis policy)

- **File:** `apps/node/node/src/chain_spec.rs` `impetus_production_genesis_patch`.
- [x] `invulnerables: []` — every genesis validator is slashable.
- [x] `minimumValidatorCount = validator_count` (no single-validator fallback).
- [x] Regression test `production_genesis_validators_are_slashable_and_min_count_matches_set`.
- [ ] Operator action (not code): size the genesis validator set >= 4 (GRANDPA tolerates only 1 Byzantine at 4) via `IMPETUS_VALIDATOR_KEYS_FILE`; review `MAX_VALIDATOR_COUNT = 32`.

### M-3. Real benchmarked weights (replace placeholders) — DONE (reference weights)

- [x] Wired `pallet_x::weights::SubstrateWeight<Runtime>` (real Substrate reference-hardware weights) for the 9 pallets that re-export a public `weights` module: session, staking, bags-list, election-provider-multi-phase, im-online, treasury, nomination-pools, fast-unstake, timestamp.
- [x] babe / grandpa / cumulus-weight-reclaim keep `()` — these pallets do NOT re-export `weights`, and their `WeightInfo for ()` impls already carry real reference-hardware weights (~78-88ms for equivocation proofs), so `()` is the correct production value, not a zero placeholder. gasless-registry (custom) likewise: its `()` impl returns real RocksDb-based weights.
- **Verify:** `grep "WeightInfo = ()"` → only gasless-registry (intentional, commented). No NPoS pallet uses zero weights.
- [ ] Follow-up (recommended): regenerate chain-specific weights on bare-metal Linux via `scripts/run-benchmarks.sh` and swap the `SubstrateWeight` defaults for `weights::pallet_xxx::WeightInfo<Runtime>`; the `runtime-benchmarks.yml` job is smoke-only.

### M-4. Sudo exit path -> governance (new code; Plan 4 not shipped)

- **File:** `apps/node/runtimes/impetus/src/lib.rs` — `pallet_sudo` (index 6); every `ForceOrigin` / `AdminOrigin` / `SpendOrigin` is `EnsureRoot`, reachable only via the single sudo EOA. No on-chain governance, no timelock.
- [ ] Add a multisig or on-chain governance (collective / referenda).
- [ ] Add a timelock on `set_code` (Root currently upgrades the runtime instantly).
- [ ] Replace `Treasury::SpendOrigin = EnsureRootWithSuccess<.., u128::MAX>` (`lib.rs` treasury config, ~line 716) with a bounded governance origin.

### M-5. Rate-limit the gasless system (new code)

- **File:** `apps/node/runtimes/common/src/gasless.rs`, `apps/node/pallets/gasless-registry/src/lib.rs`.
- [ ] Add per-account / per-rule caps, rate limiting, or an exhaustible sponsor budget. Currently an enabled rule grants unlimited fee-free transactions (free-compute / state-growth DoS).
- [ ] Bound the `Rules` map entry count (REGISTRY-1).

### M-6. Fee model — DONE (commit 73ab03f, spec_version 7)

- [x] Substrate + EVM base fees split **80% Treasury / 20% block author** via `DealWithFees` (`amount.ration(80,20)` -> ResolveTo treasury / author; `AuthorOrTreasury` falls back to treasury pre-author). Tips stay 100% author. Was: burned.
- [x] `WeightToFee = ConstantMultiplier<Balance, 50_000>` so a native transfer ~0.00002 IPT, matching a 1-gwei EVM transfer (BSC-like). `DefaultBaseFeePerGas` stays 1 gwei.
- [x] Treasury `Burn` kept at 0% (fees now flow to treasury, no extra burn needed).
- [x] impulse unchanged (keeps burning — it has no treasury); `GaslessEvmFee` gained an `Inner` type param defaulting to `()`.
- Note: routing the EVM base fee to treasury/author (vs Ethereum/BSC which BURN the EIP-1559 base fee) is a deliberate Moonbeam-style choice approved for treasury funding.

### M-7. Complete EVM parity and version discipline — DONE

- [x] Registered standard precompiles `0x06-0x09` (bn128 add/mul/pairing, blake2f) in both `FrontierPrecompilesBasic` and `FrontierPrecompilesNpos` (`apps/node/runtimes/common/src/precompiles.rs`): added deps `pallet-evm-precompile-bn128` + `-blake2`, updated `used_addresses()` (11->15, 18->22), `execute()` match, and the NPoS delegation guard. Full Ethereum stdlib 0x01-0x09 now present.
- [x] `transaction_version`: deliberately kept at 1, with a comment documenting the rule — bump ONLY on extrinsic-encoding changes (call index add/remove/reorder, signed-extension change). Precompile registration / weights / staking-config do NOT change extrinsic encoding, so bumping would force wallets to update signing logic for nothing.
- [ ] `ExistentialDeposit = 0` + `DustRemoval = ()`: still open — confirm the EVM-parity intent and accept the state-bloat risk, or add a reaping mechanism (tracked under ECON-3 / COMP-1).

### M-8. Reproducible build and dependency pinning — DONE (lock-based)

- **Finding:** branch->rev pinning in `Cargo.toml` is NOT viable here — `frontier` itself references `polkadot-sdk` via `branch = "stable2603"`, so a workspace `rev=` pin makes cargo treat `?rev=` and `?branch=` as two sources, double-compiling `sp_io` (duplicate lang item). The `stable2603` HEAD has also drifted to a commit that fails to build (yanked `multihash`), which is exactly the risk this item exists for.
- [x] Reproducibility is enforced the Substrate-standard way instead: `Cargo.lock` is committed and pins exact revs (polkadot-sdk `0af459f3`, frontier `baf505d8`, evm `a656db90`); the Dockerfile builds with `cargo build --release --locked` so any lockfile drift fails the build rather than silently pulling a new upstream commit. Adding the bn128/blake2 crates was done while collapsing the lock back to the single pinned rev (no drift; lock diff adds only the 3 new packages).
- [x] `docs/reproducible-build.md` documents the pinned revs, the srtool deterministic-WASM workflow, and a release checklist.
- [x] `Dockerfile` hardened: non-root `impetus` user, `--locked` builds, `HEALTHCHECK` (system_health JSON-RPC probe), `VOLUME /data`, metrics port 9615 exposed, base image tag + digest note.
- [ ] Operator follow-up: produce the canonical release WASM via srtool and publish its blake2-256 hash; pin the Docker base image by digest in CI.

### M-9. Full production infrastructure

- [ ] Monitoring + alerting (block stall, finality lag, equivocation, peer count, disk).
- [ ] Runbooks: session-key rotation (`author_rotateKeys` over safe RPC), backups, disaster recovery, incident response.
- [ ] E2E green on Linux (CLAUDE.md notes 13/19 on macOS due to slot drift; re-run on ubuntu).

### Mainnet order

Finish canary -> M-1 / M-2 / M-3 (recompile + spec bump in one cycle) -> M-4 /
M-5 / M-6 (new code) -> M-7 / M-8 / M-9.

---

## Verified positives (the engine is sound)

- Batch precompile hardened: DELEGATECALL/CALLCODE rejected, sub-calls bound to the outer caller, `ExitReason::Fatal` propagated, non-payable, codec bounds, gas cap, self-call rejection — all tested.
- Staking-admin precompile correctly sudo-gated (all 11 write entries run `sudo_only` after a `delegate_guard`).
- BABE + GRANDPA equivocation reporting fully wired (KeyOwnerProof via session::historical -> Offences -> Staking) with a real slashing integration test.
- On-chain SequentialPhragmen wired as GenesisElectionProvider + EPM Fallback/GovernanceFallback (anti-chain-halt guard).
- Chain spec refuses to build without real session keys unless `IMPETUS_ALLOW_PLACEHOLDER_KEYS=1`.
- H160<->AccountId mapping correct (AccountId20 + IdentityAddressMapping). Cargo.lock committed.
- The EIP-1559 base-fee "no floor" concern was investigated and ruled a FALSE POSITIVE — the base fee recovery mechanism is correct.
