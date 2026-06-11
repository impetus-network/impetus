# Betting Blockchain Monorepo — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a Substrate solochain with Frontier EVM, a custom gasless betting pallet, and a precompile bridge so EVM clients (viem/Wagmi) can interact with on-chain betting.

**Architecture:** Frontier solochain template provides the EVM-compatible Substrate node. A custom `pallet-betting` implements predict-number game logic with `Pays::No` for gasless user transactions. A Rust precompile exposes pallet functions at a fixed EVM address. Hardhat tests verify the precompile from the EVM side.

**Tech Stack:** Rust (Substrate/FRAME, Frontier), Solidity (interface only), TypeScript (Hardhat, shared types), pnpm workspaces, Turborepo

**Spec:** `docs/superpowers/specs/2026-04-09-betting-blockchain-monorepo-design.md`

---

## Dependency Graph

```
Phase 1: Monorepo scaffold
    ↓
Phase 2: Substrate node (Frontier template)
    ↓
Phase 3: Betting pallet
    ↓
Phase 4: Betting precompile
    ↓
Phase 5: Hardhat integration tests
    ↓
Phase 6: Shared package + final integration
```

Phases are sequential — each depends on the previous. Within each phase, tasks are ordered but some steps can be parallelized where noted.

---

## Phase 1: Monorepo Scaffold

**Goal:** Set up the root monorepo structure with pnpm workspaces, Turborepo, and placeholder packages.

### Task 1.1: Initialize root monorepo

**Files to create:**
- `package.json` — root workspace config
- `pnpm-workspace.yaml` — workspace package list
- `turbo.json` — Turborepo pipeline config
- `.gitignore` — Node, Rust, IDE ignores
- `.nvmrc` — Node version pin

**Steps:**
- [ ] Initialize git repo in `blockchain/`
- [ ] Create root `package.json` with `"private": true`, pnpm workspace config
- [ ] Create `pnpm-workspace.yaml` listing `packages/contracts`, `packages/shared`, `packages/webapp`
- [ ] Note: `packages/node` is NOT in pnpm workspace (Rust/Cargo project)
- [ ] Create `turbo.json` with pipelines: `build`, `test`, `lint`
- [ ] Include a `node:build` pipeline entry that shells out to `cargo build` in `packages/node/`
- [ ] Create `.gitignore` covering: `node_modules/`, `target/`, `artifacts/`, `cache/`, `.turbo/`
- [ ] Create `.nvmrc` with Node LTS version
- [ ] Run `pnpm install` to verify workspace resolves
- [ ] Commit: `chore: initialize monorepo with pnpm workspaces and turborepo`

### Task 1.2: Scaffold TypeScript packages

**Files to create:**
- `packages/contracts/package.json`
- `packages/contracts/tsconfig.json`
- `packages/contracts/hardhat.config.ts`
- `packages/shared/package.json`
- `packages/shared/tsconfig.json`
- `packages/shared/src/index.ts`

**Steps:**
- [ ] Create `packages/contracts/` with Hardhat TypeScript project (`package.json`, `hardhat.config.ts`, `tsconfig.json`)
- [ ] Install Hardhat dependencies: `hardhat`, `@nomicfoundation/hardhat-toolbox`, `viem`
- [ ] Configure `hardhat.config.ts` with a `substrate` network pointing to `http://127.0.0.1:9944`
- [ ] Create `packages/shared/` as a plain TypeScript package
- [ ] Add `packages/shared` as a workspace dependency in `packages/contracts`
- [ ] Create placeholder `packages/shared/src/index.ts`
- [ ] Run `pnpm install` and `pnpm turbo build` to verify
- [ ] Commit: `chore: scaffold contracts and shared typescript packages`

---

## Phase 2: Substrate Node (Frontier Template)

> **RESEARCH GATE:** Before starting this phase, you MUST:
> 1. Query Polkadot MCP docs: `mcp__polkadot-docs__search` for "frontier template setup", "add EVM to substrate", "pallet-evm configuration"
> 2. Read wiki pages: `polkadot-sdk.md`, `smart-contracts-on-polkadot-hub.md`, `precompiles.md` at `/Users/huyduan/Library/Mobile Documents/iCloud~md~obsidian/Documents/Vault/wiki/`
> 3. Check the Frontier repo README and template README: `https://github.com/polkadot-evm/frontier/tree/master/template`
> 4. Verify compatible versions between polkadot-sdk and frontier crates
>
> Use findings to inform exact dependency versions, runtime configuration, and any breaking changes.

### Task 2.1: Fork and integrate Frontier solochain template

**Target directory:** `packages/node/`

**Steps:**
- [ ] Clone or copy the Frontier solochain template into `packages/node/`
- [ ] Verify the Cargo workspace structure: `node/` (binary), `runtime/` (runtime crate)
- [ ] Create `pallets/` and `precompiles/` directories (empty for now)
- [ ] Update root `Cargo.toml` workspace members to include future `pallets/*` and `precompiles/*`
- [ ] Run `cargo check` to verify the template compiles
- [ ] Run `cargo build --release` (this will take a while — ~15-30 min first build)
- [ ] Commit: `feat(node): add frontier solochain template`

### Task 2.2: Verify local dev node

**Steps:**
- [ ] Start the node: `./target/release/frontier-template-node --dev`
- [ ] Verify Substrate RPC: `curl -X POST http://localhost:9944` with a `system_health` JSON-RPC call
- [ ] Verify Ethereum RPC: send `eth_blockNumber` JSON-RPC call to `http://localhost:9944`
- [ ] Verify EVM works: use Hardhat or a script to deploy a simple test contract (e.g., a Storage contract)
- [ ] Document dev accounts and their private keys (Frontier template pre-funds dev accounts)
- [ ] Stop the node
- [ ] Commit: `docs(node): verify frontier template dev node works`

### Task 2.3: Add pallet-assets to runtime

> **RESEARCH GATE:** Query `mcp__polkadot-docs__search` for "pallet-assets configuration" and "asset management runtime". Check the polkadot-sdk-pallets wiki page for the exact crate name.

**Steps:**
- [ ] Add `pallet-assets` dependency to `runtime/Cargo.toml`
- [ ] Configure `pallet-assets` in `runtime/src/lib.rs`: implement `pallet_assets::Config` for the runtime
- [ ] Set reasonable defaults: `AssetDeposit`, `MetadataDepositBase`, `StringLimit`, etc.
- [ ] Add `Assets` to the `construct_runtime!` macro
- [ ] Run `cargo check` in `packages/node/`
- [ ] Run `cargo test` in `packages/node/` to verify nothing broke
- [ ] Commit: `feat(runtime): add pallet-assets for multi-token support`

---

## Testing Requirements (All Phases)

> **Coverage target: >95% for both unit tests and e2e tests.**
>
> - **Unit tests (Rust):** Every pallet function, every error path, every storage mutation must be tested. Use `cargo tarpaulin` or `cargo llvm-cov` to measure coverage. Do not proceed to the next task if coverage drops below 95%.
> - **Unit tests (Precompile):** Every selector, every input decode/encode path, every error mapping.
> - **E2E tests (Hardhat):** Full round lifecycle, all dispatchables via precompile, all error cases, multi-token flows, edge cases (boundary timestamps, zero amounts, max values). Run against a live local Substrate node.
> - **Coverage enforcement:** Add a CI-compatible script at `packages/node/scripts/check-coverage.sh` that runs `cargo tarpaulin` and fails if coverage < 95%.

---

## Phase 3: Betting Pallet

> **RESEARCH GATE:** Before starting this phase, you MUST:
> 1. Query Polkadot MCP docs: `mcp__polkadot-docs__search` for "custom pallet development", "pallet testing", "Pays::No", "pallet weight", "StorageMap", "StorageDoubleMap"
> 2. Read wiki: `polkadot-sdk.md`, `polkadot-sdk-pallets.md`
> 3. Search for "mock runtime" in Polkadot docs — needed for pallet unit tests
> 4. Search for "pallet-assets integration" — needed for multi-token transfer logic
>
> Pay special attention to how `Pays::No` works and any anti-spam considerations.

### Task 3.1: Scaffold betting pallet crate

**Files to create:**
- `packages/node/pallets/betting/Cargo.toml`
- `packages/node/pallets/betting/src/lib.rs`

**Steps:**
- [ ] Create `pallets/betting/` directory with `Cargo.toml`
- [ ] Add dependencies: `frame-support`, `frame-system`, `sp-runtime`, `pallet-balances`, `pallet-assets`
- [ ] Create `src/lib.rs` with the pallet skeleton: `#[frame_support::pallet]` macro, empty `Config` trait, empty `Pallet` struct
- [ ] Add `pallet-betting` to the Cargo workspace members in root `Cargo.toml`
- [ ] Run `cargo check -p pallet-betting`
- [ ] Commit: `feat(pallet): scaffold betting pallet crate`

### Task 3.2: Define types and storage

**Files to modify:**
- `packages/node/pallets/betting/src/lib.rs`

**Files to create:**
- `packages/node/pallets/betting/src/types.rs`

**Steps:**
- [ ] Create `types.rs` with: `RoundInfo`, `BetInfo`, `RoundStatus` structs/enums as defined in spec
- [ ] Add `PayoutMultiplier` storage value (configurable N for `1 x N` payout rate)
- [ ] Define `Config` trait with associated types: `RuntimeEvent`, `Currency`, `Assets`, `AdminOrigin`, `MaxBetsPerRound`
- [ ] Define storage items in `lib.rs`:
  - `Rounds: StorageMap<RoundId, RoundInfo>`
  - `Bets: StorageDoubleMap<RoundId, AccountId, BetInfo>`
  - `SupportedTokens: StorageMap<TokenId, bool>`
  - `Admin: StorageValue<AccountId>`
  - `PayoutMultiplier: StorageValue<u128>`
- [ ] Run `cargo check -p pallet-betting`
- [ ] Commit: `feat(pallet): define betting types and storage`

### Task 3.3: Implement round logic (timestamp-based)

**Steps:**
- [ ] Implement helper function `current_round_id(now: u64) -> RoundId` — calculates round ID from timestamp using 18:00 GMT+7 cutoff
- [ ] Implement helper `is_round_open(now: u64, round_id: RoundId) -> bool`
- [ ] Implement helper `round_close_timestamp(round_id: RoundId) -> u64`
- [ ] Write unit tests for round ID calculation:
  - Before 18:00 GMT+7 → current day round
  - After 18:00 GMT+7 → next day round
  - Exactly at 18:00 → closed (next day)
  - Midnight boundary (00:00 GMT+7)
  - Timestamp = 0 (genesis edge case)
  - Very large timestamp (year 2100+ overflow check)
  - `is_round_open` returns true/false correctly at boundary
  - `round_close_timestamp` returns correct value for consecutive days
- [ ] Run `cargo test -p pallet-betting`
- [ ] Commit: `feat(pallet): implement timestamp-based round logic`

### Task 3.4: Implement place_bet dispatchable

**Steps:**
- [ ] Implement `place_bet(origin, number: u8, token: TokenId, amount: Balance)` dispatchable
- [ ] Add `#[pallet::weight(...)]` with `Pays::No`
- [ ] Validations: number 0-99, token supported, amount > 0, round is open, user has sufficient balance, 1 bet per user per round
- [ ] Transfer tokens from user to pallet account (native via `Currency`, assets via `pallet-assets`)
- [ ] Insert into `Bets` storage, update `Rounds` total_pool
- [ ] Emit `BetPlaced` event
- [ ] Write unit tests:
  - Successful bet placement (native token)
  - Successful bet placement (asset token)
  - Reject: number > 99
  - Reject: number = 100 (boundary)
  - Reject: unsupported token
  - Reject: insufficient balance (native)
  - Reject: insufficient balance (asset)
  - Reject: amount = 0
  - Reject: duplicate bet same round
  - Reject: round is closed (after 18:00 GMT+7)
  - Bet goes to next day round after 18:00
  - Verify storage: Bets entry created correctly
  - Verify storage: Rounds total_pool updated correctly
  - Verify event: BetPlaced emitted with correct fields
  - Verify token transfer: user balance decreased, pallet balance increased
- [ ] Run `cargo test -p pallet-betting`
- [ ] Commit: `feat(pallet): implement place_bet with Pays::No`

### Task 3.5: Implement submit_result dispatchable

**Steps:**
- [ ] Implement `submit_result(origin, round_id: RoundId, number: u8)` dispatchable
- [ ] Admin-only check via `Admin` storage value
- [ ] Validations: round exists, round is closed (past 18:00 GMT+7), not already resolved, number 0-99
- [ ] Update round status to `Resolved`, set `result`
- [ ] Emit `ResultSubmitted` event
- [ ] Write unit tests:
  - Successful result submission
  - Reject: non-admin caller
  - Reject: round still open
  - Reject: already resolved
  - Reject: round does not exist
  - Reject: number > 99
  - Verify storage: round status changed to Resolved
  - Verify storage: round result set correctly
  - Verify event: ResultSubmitted emitted with correct fields
- [ ] Run `cargo test -p pallet-betting`
- [ ] Commit: `feat(pallet): implement submit_result admin dispatchable`

### Task 3.6: Implement claim_winnings dispatchable

**Steps:**
- [ ] Implement `claim_winnings(origin, round_id: RoundId)` dispatchable with `Pays::No`
- [ ] Check: round is resolved, caller has a winning bet (bet.number == round.result)
- [ ] Calculate payout: `bet.amount * PayoutMultiplier`
- [ ] If pool has enough: pay full payout. If not: pay proportional share of remaining pool
- [ ] Transfer tokens from pallet account to winner
- [ ] Mark bet as claimed (prevent double-claim)
- [ ] Emit `WinningsClaimed` event
- [ ] Write unit tests:
  - Single winner, full payout (native token)
  - Single winner, full payout (asset token)
  - Multiple winners same number, full payout
  - Multiple winners, pool insufficient — proportional split
  - Payout multiplier: bet 10 tokens with N=90 → win 900
  - Payout multiplier: verify house edge (pool - total payouts > 0)
  - Reject: not a winner (wrong number)
  - Reject: already claimed (double-claim attempt)
  - Reject: round not resolved (still open or closed)
  - Reject: round does not exist
  - Verify balance: winner receives correct amount
  - Verify balance: pallet account decreased correctly
  - Verify storage: bet marked as claimed
  - Verify event: WinningsClaimed emitted with correct fields
- [ ] Run `cargo test -p pallet-betting`
- [ ] Commit: `feat(pallet): implement claim_winnings with payout multiplier`

### Task 3.7: Implement admin claim (no winners)

**Steps:**
- [ ] Implement `admin_claim_pool(origin, round_id: RoundId)` dispatchable
- [ ] Admin-only, round must be resolved, must have zero winners (or all winners already claimed and surplus remains)
- [ ] Transfer remaining pool balance to admin
- [ ] Update round status to `Settled`
- [ ] Emit `PoolClaimed` event
- [ ] Write unit tests:
  - No winners — admin takes entire pool (native token)
  - No winners — admin takes entire pool (asset token)
  - After winners claimed — admin takes surplus (house edge)
  - Multiple tokens in same round — admin claims each token pool
  - Reject: non-admin caller
  - Reject: round not resolved
  - Reject: round does not exist
  - Reject: pool already empty (double-claim)
  - Verify balance: admin receives correct amount
  - Verify storage: round status changed to Settled
  - Verify event: PoolClaimed emitted with correct fields
- [ ] Run `cargo test -p pallet-betting`
- [ ] Commit: `feat(pallet): implement admin_claim_pool for house edge`

### Task 3.8: Implement token management dispatchables

**Steps:**
- [ ] Implement `add_supported_token(origin, token_id: TokenId)` — admin only
- [ ] Implement `remove_supported_token(origin, token_id: TokenId)` — admin only
- [ ] Implement `set_payout_multiplier(origin, multiplier: u128)` — admin only
- [ ] Implement `set_admin(origin, new_admin: AccountId)` — sudo or current admin only
- [ ] Write unit tests:
  - add_supported_token: success, reject non-admin, reject duplicate
  - remove_supported_token: success, reject non-admin, reject non-existent, reject if active bets use it
  - set_payout_multiplier: success, reject non-admin, reject zero multiplier
  - set_admin: success via sudo, success via current admin, reject unauthorized
- [ ] Run `cargo test -p pallet-betting`
- [ ] Commit: `feat(pallet): implement admin token management and config`

### Task 3.9: Coverage check for betting pallet

**Files to create:**
- `packages/node/scripts/check-coverage.sh`

**Steps:**
- [ ] Install `cargo-tarpaulin` or `cargo-llvm-cov`
- [ ] Run coverage: `cargo tarpaulin -p pallet-betting --out Html`
- [ ] Verify coverage >95%. If not, identify uncovered lines and add missing tests
- [ ] Create `scripts/check-coverage.sh` that runs coverage and fails if <95%
- [ ] Run all tests one final time: `cargo test -p pallet-betting`
- [ ] Commit: `test(pallet): achieve >95% unit test coverage for betting pallet`

### Task 3.10: Integrate pallet into runtime

**Files to modify:**
- `packages/node/runtime/src/lib.rs`

**Steps:**
- [ ] Add `pallet-betting` dependency to `runtime/Cargo.toml` as a path dependency
- [ ] Implement `pallet_betting::Config` for the runtime
- [ ] Wire `Currency` to `pallet-balances`, `Assets` to `pallet-assets`
- [ ] Add `Betting` to `construct_runtime!` macro
- [ ] Set genesis config: initial admin account, default payout multiplier, initial supported tokens
- [ ] Run `cargo check` on the full workspace
- [ ] Run `cargo test` on the full workspace
- [ ] Build the node: `cargo build --release`
- [ ] Start dev node and verify pallet is loaded (check metadata via RPC)
- [ ] Commit: `feat(runtime): integrate pallet-betting into runtime`

---

## Phase 4: Betting Precompile

> **RESEARCH GATE:** Before starting this phase, you MUST:
> 1. Query Polkadot MCP docs: `mcp__polkadot-docs__search` for "custom precompile", "frontier precompile", "precompile implementation"
> 2. Read wiki: `precompiles.md` at `/Users/huyduan/Library/Mobile Documents/iCloud~md~obsidian/Documents/Vault/wiki/`
> 3. Study existing Frontier precompile examples in the Frontier repo: `https://github.com/polkadot-evm/frontier`
> 4. Search for "EVM address to AccountId conversion" — critical for mapping EVM callers to Substrate accounts
> 5. Search for "precompile framework" or "PrecompileSet" in Frontier docs/code
>
> Understanding the Frontier precompile framework is critical. The implementation pattern differs significantly from writing Solidity contracts.

### Task 4.1: Scaffold precompile crate

**Files to create:**
- `packages/node/precompiles/betting/Cargo.toml`
- `packages/node/precompiles/betting/src/lib.rs`

**Steps:**
- [ ] Create `precompiles/betting/` directory with `Cargo.toml`
- [ ] Add dependencies: `fp-evm`, `pallet-evm`, `precompile-utils` (from Frontier), `pallet-betting` (path dep)
- [ ] Create skeleton `src/lib.rs` with precompile struct
- [ ] Add to Cargo workspace members
- [ ] Run `cargo check -p precompile-betting`
- [ ] Commit: `feat(precompile): scaffold betting precompile crate`

### Task 4.2: Implement precompile functions

**Steps:**
- [ ] Implement `Precompile` trait for `BettingPrecompile`
- [ ] Parse Solidity function selectors for: `placeBet`, `submitResult`, `claimWinnings`, `getCurrentRound`, `getBet`
- [ ] For each write function: decode EVM input → call pallet dispatchable → encode EVM output
- [ ] For each read function: decode EVM input → read pallet storage → encode EVM output
- [ ] Handle EVM address ↔ Substrate AccountId conversion
- [ ] Handle gas metering (even though pallet is `Pays::No`, EVM side needs gas accounting)
- [ ] Emit EVM logs corresponding to pallet events: `BetPlaced`, `ResultSubmitted`, `WinningsClaimed`
- [ ] Write Rust unit tests:
  - Selector parsing: each function selector matches expected 4-byte hash
  - Input decoding: valid ABI-encoded inputs for each function
  - Input decoding: malformed input → revert
  - Input decoding: wrong number of args → revert
  - Output encoding: verify return data matches Solidity ABI spec
  - EVM address → AccountId mapping: known address produces expected AccountId
  - EVM log emission: verify log topics and data for each event
  - Error mapping: pallet errors map to EVM revert with reason string
  - Gas metering: verify gas cost is reasonable for each function
  - Unknown selector → revert
- [ ] Run `cargo test -p precompile-betting`
- [ ] Run coverage: verify >95% for precompile crate
- [ ] Commit: `feat(precompile): implement betting precompile functions`

### Task 4.3: Register precompile in runtime

**Files to modify:**
- `packages/node/runtime/src/lib.rs`

**Steps:**
- [ ] Add `precompile-betting` dependency to `runtime/Cargo.toml`
- [ ] Register `BettingPrecompile` at address `0x0000000000000000000000000000000000000801` in the runtime's `PrecompileSet`
- [ ] Run `cargo check` on full workspace
- [ ] Run `cargo test` on full workspace
- [ ] Build: `cargo build --release`
- [ ] Start dev node, verify precompile is reachable by calling `eth_call` to the precompile address
- [ ] Commit: `feat(runtime): register betting precompile at 0x801`

---

## Phase 5: Hardhat Integration Tests

### Task 5.1: Create Solidity interface

**Files to create:**
- `packages/contracts/contracts/interfaces/IBettingPrecompile.sol`

**Steps:**
- [ ] Write `IBettingPrecompile.sol` matching the interface defined in the spec (functions + events)
- [ ] Compile with Hardhat: `pnpm hardhat compile`
- [ ] Commit: `feat(contracts): add IBettingPrecompile solidity interface`

### Task 5.2: Write e2e tests — core betting flow

**Files to create:**
- `packages/contracts/test/BettingPrecompile.test.ts`
- `packages/contracts/test/helpers/setup.ts` — shared test utilities (deploy, accounts, time helpers)

**Prerequisites:** Local Substrate node running with `--dev`

**Steps:**
- [ ] Configure Hardhat to connect to local Substrate node (`http://127.0.0.1:9944`)
- [ ] Configure dev accounts (use Frontier template pre-funded accounts)
- [ ] Create `test/helpers/setup.ts` with:
  - Account setup (admin, users)
  - Precompile contract instance at `0x801`
  - Time manipulation helpers (advance blocks to simulate 18:00 cutoff)
  - Balance query helpers
- [ ] Write test suite: **placeBet**
  - Place bet with native token — verify event `BetPlaced`
  - Place bet with asset token — verify event
  - Verify user balance decreased
  - Reject: number > 99 (expect revert)
  - Reject: number = 100 boundary (expect revert)
  - Reject: amount = 0 (expect revert)
  - Reject: unsupported token (expect revert)
  - Reject: insufficient balance (expect revert)
  - Reject: duplicate bet same round (expect revert)
- [ ] Write test suite: **submitResult**
  - Admin submits result — verify event `ResultSubmitted`
  - Reject: non-admin caller (expect revert)
  - Reject: round still open (expect revert)
  - Reject: already resolved (expect revert)
  - Reject: invalid number (expect revert)
- [ ] Write test suite: **claimWinnings**
  - Winner claims — verify balance increased by `amount * multiplier`
  - Reject: non-winner (expect revert)
  - Reject: double claim (expect revert)
  - Reject: round not resolved (expect revert)
- [ ] Run tests: `pnpm hardhat test --network substrate`
- [ ] Commit: `test(contracts): add e2e tests for core betting flow`

### Task 5.3: Write e2e tests — advanced scenarios

**Files to create:**
- `packages/contracts/test/BettingAdvanced.test.ts`

**Steps:**
- [ ] Write test suite: **Full round lifecycle (e2e)**
  - Multiple users place bets → time passes 18:00 → admin submits → winners claim → admin claims surplus
  - Verify all balances correct at each step
  - Verify round status transitions via `getCurrentRound`
- [ ] Write test suite: **Multiple winners**
  - 3 users bet same winning number, different amounts
  - Verify each receives `amount * multiplier`
  - Verify admin gets surplus
- [ ] Write test suite: **Pool insufficient**
  - Set multiplier high, few bets
  - Winners get proportional share of available pool
- [ ] Write test suite: **No winner**
  - All users bet wrong number
  - Admin claims entire pool
  - Verify admin balance increased by full pool amount
- [ ] Write test suite: **Multi-token in same round**
  - User A bets native token, User B bets asset token
  - Both win — each gets payout in their respective token
- [ ] Write test suite: **Cross-round**
  - Place bet in round N, time advances, place bet in round N+1
  - Submit results for both rounds
  - Claim for both rounds independently
- [ ] Write test suite: **Admin functions via precompile**
  - `getCurrentRound` returns correct data
  - `getBet` returns correct user bet info
  - `getBet` for non-existent bet returns zeros
- [ ] Write test suite: **Edge cases**
  - Bet at exactly 18:00 GMT+7 boundary
  - Bet at 17:59:59 vs 18:00:00
  - Maximum bet amount (near max uint256)
  - Many users (10+) betting in same round
- [ ] Run all tests: `pnpm hardhat test --network substrate`
- [ ] Verify all tests pass
- [ ] Commit: `test(contracts): add e2e tests for advanced betting scenarios`

### Task 5.4: E2E test coverage report

**Steps:**
- [ ] Add `solidity-coverage` or equivalent Hardhat coverage plugin
- [ ] Run coverage report for all test files
- [ ] Verify >95% branch and line coverage on `IBettingPrecompile` interface paths
- [ ] Document any uncoverable paths (e.g., node-level errors) with justification
- [ ] Commit: `test(contracts): verify >95% e2e test coverage`

### Task 5.5: Generate and export ABIs

**Steps:**
- [ ] After Hardhat compile, copy `IBettingPrecompile` ABI to `packages/shared/abis/`
- [ ] Create a Turborepo pipeline task to auto-generate ABIs on build
- [ ] Verify `packages/shared` exports the ABI correctly
- [ ] Commit: `chore(shared): export betting precompile ABI`

---

## Phase 6: Shared Package + Final Integration

### Task 6.1: Populate shared package

**Files to create/modify:**
- `packages/shared/src/constants/addresses.ts`
- `packages/shared/src/constants/chain.ts`
- `packages/shared/src/types/betting.ts`
- `packages/shared/src/index.ts`

**Steps:**
- [ ] Add precompile address constant: `0x0000000000000000000000000000000000000801`
- [ ] Add chain config: RPC URL, chain ID, block time
- [ ] Add TypeScript types matching the Solidity interface (round status, bet info, etc.)
- [ ] Export everything from `src/index.ts`
- [ ] Verify `packages/contracts` can import from `@betting/shared` (or chosen package name)
- [ ] Commit: `feat(shared): add betting constants types and ABI exports`

### Task 6.2: Turborepo pipeline verification

**Steps:**
- [ ] Run `pnpm turbo build` — verify all packages build in correct order
- [ ] Run `pnpm turbo test` — verify Rust tests and Hardhat tests (with node running) pass
- [ ] Verify caching works: run `pnpm turbo build` again, should hit cache
- [ ] Commit: `chore: verify turborepo pipeline and caching`

### Task 6.3: Documentation and dev setup

**Files to create:**
- `packages/node/README.md`
- `CLAUDE.md` (project-level instructions)

**Steps:**
- [ ] Write `packages/node/README.md`: build instructions, dev node startup, pre-funded accounts
- [ ] Write root `CLAUDE.md` with: project structure, how to build, how to test, key conventions
- [ ] Commit: `docs: add node readme and project claude.md`

---

## Summary of Commits per Phase

| Phase | Commits | Approx. effort |
|-------|---------|-----------------|
| 1. Monorepo scaffold | 2 | Small |
| 2. Substrate node | 3 | Medium (Frontier setup + build time) |
| 3. Betting pallet | 10 | Large (core logic + unit tests + coverage) |
| 4. Precompile | 3 | Medium-Large (Frontier precompile framework learning curve) |
| 5. Hardhat e2e tests | 5 | Large (comprehensive e2e + coverage) |
| 6. Shared + integration | 3 | Small |
| **Total** | **26** | |

## Coverage Targets

| Component | Tool | Target |
|-----------|------|--------|
| `pallet-betting` (Rust) | `cargo-tarpaulin` or `cargo-llvm-cov` | >95% line coverage |
| `precompile-betting` (Rust) | `cargo-tarpaulin` or `cargo-llvm-cov` | >95% line coverage |
| E2E via precompile (Hardhat) | `solidity-coverage` or manual branch audit | >95% branch coverage on all precompile paths |

## Risk Notes

- **Frontier version compatibility**: Frontier crate versions must match the polkadot-sdk version used. Version mismatch is the most common build failure. Always check Frontier's `polkadot-v*` branches.
- **Precompile complexity**: Writing custom precompiles requires deep Frontier framework knowledge. Budget extra time for Phase 4. Study existing precompiles (e.g., `pallet-evm-precompile-simple`) before writing custom ones.
- **`Pays::No` abuse**: Without gas fees, anyone can spam `place_bet`. The 1-bet-per-user-per-round limit mitigates this, but monitor for Sybil attacks (many accounts, 1 bet each).
- **EVM ↔ Substrate address mapping**: Frontier uses a specific H160 ↔ AccountId32 mapping. Test this thoroughly in Phase 4.
