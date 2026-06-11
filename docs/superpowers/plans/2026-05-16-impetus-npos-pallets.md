# Plan 2 — Impetus NPoS Pallets

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Wire the full Polkadot NPoS pallet stack onto `runtime-impetus` on top of Plan 1's Babe scaffold. After this plan, `--chain impetus_dev_npos --validator` produces blocks past the genesis epoch boundary (Plan 1's babe-worker exit dissolves once `pallet-session` provides the `ExternalTrigger` source), the EVM `block.coinbase` resolves to the real validator H160, era progression / slashing / rewards / nomination pools / fast unstake / treasury all work at the runtime layer, and 11 runtime integration tests pin the lifecycle. **No precompiles in this plan** — those are Plan 3.

**Architecture:** Plan 2 turns Plan 1's "Babe authority list pinned at genesis" model into "Babe authorities rotate per era via Session, driven by Staking elections." That means: `pallet-session` (+ `historical`) takes ownership of `SessionKeys`, `pallet-staking` plus `pallet-election-provider-multi-phase` + `pallet-bags-list` produce the elected validator set each era, `pallet-offences` + `pallet-im-online` close the loop on liveness and slashing, `pallet-authority-discovery` powers DHT peer routing, and `pallet-nomination-pools` + `pallet-treasury` + `pallet-fast-unstake` round out staking economics. We also wire `pallet-authorship`'s `FindAuthor` to a Babe-aware source so EVM `COINBASE` returns the real validator instead of `0x0`. The dev-fast timing profile from the spec (10-min sessions, 60-min eras, 4-era bonding) is baked into a new `runtimes/common/src/staking_constants.rs`. A `runtime-test-fast` Cargo feature compresses those further (30s sessions, 1-min eras) for E2E use in Plan 3, but stays gated and unused at runtime in Plan 2.

**Tech Stack:** Rust 2021, polkadot-sdk `stable2603`, Frontier `stable2603`, FRAME pallets `pallet-session`, `pallet-session::historical`, `pallet-staking`, `pallet-staking-reward-curve`, `pallet-offences`, `pallet-bags-list`, `pallet-election-provider-multi-phase`, `frame-election-provider-support::onchain`, `pallet-authority-discovery` (Config wiring; types were added in Plan 1), `pallet-im-online` (Config wiring; types added in Plan 1), `pallet-nomination-pools`, `pallet-treasury`, `pallet-fast-unstake`.

**Spec:** [`docs/superpowers/specs/2026-05-16-impetus-npos-via-precompiles-design.md`](../specs/2026-05-16-impetus-npos-via-precompiles-design.md)
**Predecessor:** [Plan 1 — Babe Migration](2026-05-16-impetus-babe-migration.md) — must be merged first.

**All paths below are relative to repo root** (`/Users/huyduan/projects/blockchain`) unless prefixed with `cd` in commands. The Rust workspace lives at `apps/node/`.

**Out of scope (Plan 3):** all 7 Solidity precompile crates, genesis rewrite for production mainnet, alias cleanup in `command.rs::load_spec` (`mainnet` / `impetus` keep pointing at the dev NPoS spec until Plan 3 ships the production spec), E2E suites under `packages/contracts/test/`, `runtime-test-fast` feature usage (the feature is *declared* in Plan 2 but not exercised until Plan 3).

---

## File Map

**Created:**
- `apps/node/runtimes/common/src/staking_constants.rs` — dev-fast NPoS timing + bond constants
- `apps/node/runtimes/common/src/voter_bags.rs` — generated bags-list thresholds (200 buckets)
- `apps/node/runtimes/common/src/reward_curve.rs` — Polkadot-style piecewise-linear reward curve
- `apps/node/runtimes/common/src/staking_election.rs` — `OnChainSeqPhragmen` solver config + bounds
- `apps/node/runtimes/impetus/tests/common.rs` — `ExtBuilder`, `run_to_block`, `run_to_session`, `run_to_era` helpers + genesis sugar
- `apps/node/runtimes/impetus/tests/genesis_election.rs`
- `apps/node/runtimes/impetus/tests/era_progression.rs`
- `apps/node/runtimes/impetus/tests/session_rotation.rs`
- `apps/node/runtimes/impetus/tests/bond_lifecycle.rs`
- `apps/node/runtimes/impetus/tests/nominate_payout.rs`
- `apps/node/runtimes/impetus/tests/slashing.rs`
- `apps/node/runtimes/impetus/tests/pool_lifecycle.rs`
- `apps/node/runtimes/impetus/tests/fast_unstake.rs`
- `apps/node/runtimes/impetus/tests/treasury_proposal.rs`
- `apps/node/runtimes/impetus/tests/force_eras.rs`
- `apps/node/runtimes/impetus/tests/babe_smoke.rs`

**Modified:**
- `apps/node/Cargo.toml` — add NPoS pallet workspace deps (`pallet-staking`, `pallet-session`, `pallet-offences`, `pallet-bags-list`, `pallet-election-provider-multi-phase`, `frame-election-provider-support`, `pallet-nomination-pools`, `pallet-treasury`, `pallet-fast-unstake`, `pallet-staking-reward-curve`, `sp-staking`)
- `apps/node/runtimes/common/Cargo.toml` — pull in new shared deps
- `apps/node/runtimes/common/src/lib.rs` — re-export new modules
- `apps/node/runtimes/impetus/Cargo.toml` — add the same pallet deps; add `runtime-test-fast` feature
- `apps/node/runtimes/impetus/src/lib.rs` — Config impls for all 10 new pallets, `construct_runtime!` indices 18–28, FindAuthor swap, Babe `EpochChangeTrigger = ExternalTrigger`, `DisabledValidators = Session`, `KeyOwnerProof` via `Historical`, real `EquivocationReportSystem`, Authorship.FindAuthor swap, GRANDPA equivocation reporter swap, `spec_version` 3 → 4
- `apps/node/node/src/chain_spec.rs` — `impetus_genesis_patch` rewritten with `session.keys`, `staking.stakers`, `nominationPools`, `treasury` sections; new session-key helpers; build-time genesis assertions
- `apps/node/node/src/service/babe.rs` — wire authority-discovery DHT worker + register im-online OCW; drop "Plan 1 known limitation" notes
- `apps/node/CLAUDE.md` — replace Plan 1 limitation section with NPoS section; update pallet index map; bump `spec_version` reference

**Deleted:** none.

---

## Task 1: Workspace dependencies for the NPoS stack

**Files:**
- Modify: `apps/node/Cargo.toml` (`[workspace.dependencies]`)

- [ ] **Step 1: Add the new dep block**

Open `apps/node/Cargo.toml` and add the following block inside `[workspace.dependencies]`, immediately after the existing Babe block from Plan 1. All entries pin to `stable2603` to match the workspace.

```toml
# NPoS stack (impetus only — Plan 2)
pallet-staking                    = { git = "https://github.com/paritytech/polkadot-sdk", branch = "stable2603", default-features = false }
pallet-staking-reward-curve       = { git = "https://github.com/paritytech/polkadot-sdk", branch = "stable2603" }
pallet-session                    = { git = "https://github.com/paritytech/polkadot-sdk", branch = "stable2603", default-features = false }
pallet-offences                   = { git = "https://github.com/paritytech/polkadot-sdk", branch = "stable2603", default-features = false }
pallet-bags-list                  = { git = "https://github.com/paritytech/polkadot-sdk", branch = "stable2603", default-features = false }
pallet-election-provider-multi-phase = { git = "https://github.com/paritytech/polkadot-sdk", branch = "stable2603", default-features = false }
frame-election-provider-support   = { git = "https://github.com/paritytech/polkadot-sdk", branch = "stable2603", default-features = false }
pallet-nomination-pools           = { git = "https://github.com/paritytech/polkadot-sdk", branch = "stable2603", default-features = false }
pallet-treasury                   = { git = "https://github.com/paritytech/polkadot-sdk", branch = "stable2603", default-features = false }
pallet-fast-unstake               = { git = "https://github.com/paritytech/polkadot-sdk", branch = "stable2603", default-features = false }
sp-staking                        = { git = "https://github.com/paritytech/polkadot-sdk", branch = "stable2603", default-features = false }
```

`pallet-staking-reward-curve` is a proc-macro crate — it must NOT carry `default-features = false`.

- [ ] **Step 2: Verify workspace resolves**

```bash
cd apps/node && SKIP_WASM_BUILD=1 cargo check --workspace
```

Expected: green. New deps appear in `Cargo.lock`.

- [ ] **Step 3: Commit**

```bash
cd /Users/huyduan/projects/blockchain
git add apps/node/Cargo.toml apps/node/Cargo.lock
git commit -m "chore(node): add NPoS pallet workspace deps"
```

---

## Task 2: Dev-fast NPoS constants in `runtimes/common`

**Files:**
- Create: `apps/node/runtimes/common/src/staking_constants.rs`
- Modify: `apps/node/runtimes/common/src/lib.rs`
- Modify: `apps/node/runtimes/common/Cargo.toml`

- [ ] **Step 1: Create `apps/node/runtimes/common/src/staking_constants.rs`**

```rust
//! Dev-fast NPoS timing + bond constants.
//!
//! Tuned so a full validator/nominator lifecycle fits in a single working
//! session: 10-minute sessions, 60-minute eras, 4-era (~4-hour) bonding.
//! When promoting to production timing, edit this file and bump
//! `spec_version`. The constants are compile-time, not governance-tunable.
//!
//! A `runtime-test-fast` feature further compresses session/era length for
//! E2E test runs (Plan 3 wires it up; gated and unused in Plan 2).

use crate::{Balance, BlockNumber};
use sp_staking::{EraIndex, SessionIndex};

pub const UNIT: Balance = 1_000_000_000_000_000_000;

#[cfg(not(feature = "runtime-test-fast"))]
pub const BLOCKS_PER_SESSION: BlockNumber = 100; // 10 min @ 6s blocks
#[cfg(not(feature = "runtime-test-fast"))]
pub const SESSIONS_PER_ERA: SessionIndex = 6; // 60 min eras
#[cfg(not(feature = "runtime-test-fast"))]
pub const BONDING_DURATION_ERAS: EraIndex = 4;
#[cfg(not(feature = "runtime-test-fast"))]
pub const SLASH_DEFER_DURATION_ERAS: EraIndex = 3;

#[cfg(feature = "runtime-test-fast")]
pub const BLOCKS_PER_SESSION: BlockNumber = 5; // 30s
#[cfg(feature = "runtime-test-fast")]
pub const SESSIONS_PER_ERA: SessionIndex = 2; // 1 min
#[cfg(feature = "runtime-test-fast")]
pub const BONDING_DURATION_ERAS: EraIndex = 2;
#[cfg(feature = "runtime-test-fast")]
pub const SLASH_DEFER_DURATION_ERAS: EraIndex = 1;

pub const MAX_NOMINATIONS: u32 = 16;
pub const MAX_NOMINATORS_PER_VALIDATOR: u32 = 16;
pub const VALIDATOR_COUNT_TARGET: u32 = 8;
pub const MAX_VALIDATOR_COUNT: u32 = 32;

pub const MIN_VALIDATOR_BOND: Balance = 1_000 * UNIT;
pub const MIN_NOMINATOR_BOND: Balance = 10 * UNIT;

pub const REPORT_LONGEVITY: u64 = (BONDING_DURATION_ERAS as u64)
    * (SESSIONS_PER_ERA as u64)
    * (BLOCKS_PER_SESSION as u64);

pub const SESSION_PERIOD: BlockNumber = BLOCKS_PER_SESSION;
pub const SESSION_OFFSET: BlockNumber = 0;
```

- [ ] **Step 2: Add `sp-staking` to `runtimes/common/Cargo.toml`**

Add inside `[dependencies]`:

```toml
sp-staking = { workspace = true }
```

And inside `[features]` under the existing `std = [` list (and the no-std list if present), add:

```toml
"sp-staking/std",
```

If a `runtime-test-fast` feature does not yet exist in this crate, add:

```toml
runtime-test-fast = []
```

- [ ] **Step 3: Re-export module from `apps/node/runtimes/common/src/lib.rs`**

Add at the bottom of the file (after the existing `mod precompiles;` / `pub use precompiles::*;` block):

```rust
pub mod staking_constants;
pub use staking_constants::{
    BLOCKS_PER_SESSION, BONDING_DURATION_ERAS, MAX_NOMINATIONS, MAX_NOMINATORS_PER_VALIDATOR,
    MAX_VALIDATOR_COUNT, MIN_NOMINATOR_BOND, MIN_VALIDATOR_BOND, REPORT_LONGEVITY,
    SESSIONS_PER_ERA, SESSION_OFFSET, SESSION_PERIOD, SLASH_DEFER_DURATION_ERAS, UNIT,
    VALIDATOR_COUNT_TARGET,
};
```

- [ ] **Step 4: Verify**

```bash
cd apps/node && SKIP_WASM_BUILD=1 cargo check -p runtime-common
```

Expected: green.

- [ ] **Step 5: Commit**

```bash
git add apps/node/runtimes/common/src/staking_constants.rs \
        apps/node/runtimes/common/src/lib.rs \
        apps/node/runtimes/common/Cargo.toml
git commit -m "feat(runtime): add dev-fast NPoS staking constants"
```

---

## Task 3: Polkadot-style reward curve in `runtimes/common`

**Files:**
- Create: `apps/node/runtimes/common/src/reward_curve.rs`
- Modify: `apps/node/runtimes/common/src/lib.rs`
- Modify: `apps/node/runtimes/common/Cargo.toml`

- [ ] **Step 1: Create `apps/node/runtimes/common/src/reward_curve.rs`**

```rust
//! Piecewise-linear inflation curve shared by every NPoS-enabled runtime.
//!
//! Matches Polkadot's defaults: 2.5%–10% inflation, ideal staking ratio 75%,
//! falloff 5%. The curve crate emits a `static REWARD_CURVE` symbol that
//! `pallet_staking::Config::EraPayout = ConvertCurve<RewardCurve>` consumes.

use pallet_staking_reward_curve::build;

build! {
    const REWARD_CURVE: sp_runtime::curve::PiecewiseLinear<'static> = curve!(
        min_inflation: 0_025_000,
        max_inflation: 0_100_000,
        ideal_stake: 0_750_000,
        falloff: 0_050_000,
        max_piece_count: 40,
        test_precision: 0_005_000,
    );
}

pub use REWARD_CURVE;
```

- [ ] **Step 2: Add deps to `runtimes/common/Cargo.toml`**

```toml
pallet-staking-reward-curve = { workspace = true }
pallet-staking              = { workspace = true }
```

Add to `[features].std`:

```toml
"pallet-staking/std",
```

(`pallet-staking-reward-curve` is a proc macro and does not need a `std` entry.)

- [ ] **Step 3: Re-export from `runtimes/common/src/lib.rs`**

```rust
pub mod reward_curve;
pub use reward_curve::REWARD_CURVE;
```

- [ ] **Step 4: Verify**

```bash
cd apps/node && SKIP_WASM_BUILD=1 cargo check -p runtime-common
```

Expected: green.

- [ ] **Step 5: Commit**

```bash
git add apps/node/runtimes/common/src/reward_curve.rs \
        apps/node/runtimes/common/src/lib.rs \
        apps/node/runtimes/common/Cargo.toml
git commit -m "feat(runtime): add Polkadot-style staking reward curve"
```

---

## Task 4: Bags-list voter bag thresholds

**Files:**
- Create: `apps/node/runtimes/common/src/voter_bags.rs`
- Modify: `apps/node/runtimes/common/src/lib.rs`

- [ ] **Step 1: Generate the thresholds offline**

Run from a scratch dir (this clones polkadot-sdk locally for the helper binary; the generated file is what we commit, not the toolchain):

```bash
cd /tmp
git clone --depth 1 --branch stable2603 https://github.com/paritytech/polkadot-sdk pdsdk-bags
cd pdsdk-bags/substrate/utils/frame/generate-bags
cargo run --release -- \
    --total-issuance 100000000000000000000000000 \
    --minimum-balance 10000000000000000000 \
    --output /tmp/impetus_voter_bags.rs \
    --runtime kitchensink-runtime
```

Parameters: `total-issuance` = 100 M IPT in plancks (10^26); `minimum-balance` = `MIN_NOMINATOR_BOND` = 10 IPT in plancks (10^19). The binary emits 200 geometric buckets from min bond up to total issuance with ratio ~1.21, exactly matching Polkadot's tuning.

- [ ] **Step 2: Copy the generated file to `runtimes/common/src/voter_bags.rs`**

```bash
cp /tmp/impetus_voter_bags.rs /Users/huyduan/projects/blockchain/apps/node/runtimes/common/src/voter_bags.rs
```

Edit the top of the copied file to replace the auto-generated header with:

```rust
//! Bags-list bucket thresholds for impetus (Plan 2).
//!
//! Generated by polkadot-sdk `substrate/utils/frame/generate-bags` against
//! stable2603 with `total-issuance = 100 M IPT`, `min-balance = 10 IPT`.
//! 200 geometric buckets, ratio ~1.21. To regenerate, see the comment block
//! at the top of `staking_constants.rs` and re-run the generator with the
//! same params.

pub const CONSTANT_RATIO: f64 = 1.21;
pub const EXISTENTIAL_WEIGHT: u64 = 10_000_000_000_000_000_000;
```

Keep the generator's `pub const THRESHOLDS: [u64; 200] = [ ... ];` literal intact below this header.

- [ ] **Step 3: Re-export from `lib.rs`**

```rust
pub mod voter_bags;
```

- [ ] **Step 4: Verify**

```bash
cd apps/node && SKIP_WASM_BUILD=1 cargo check -p runtime-common
```

Expected: green.

- [ ] **Step 5: Commit**

```bash
git add apps/node/runtimes/common/src/voter_bags.rs \
        apps/node/runtimes/common/src/lib.rs
git commit -m "feat(runtime): embed bags-list thresholds for impetus"
```

---

## Task 5: `OnChainSeqPhragmen` solver scaffolding

**Files:**
- Create: `apps/node/runtimes/common/src/staking_election.rs`
- Modify: `apps/node/runtimes/common/src/lib.rs`
- Modify: `apps/node/runtimes/common/Cargo.toml`

- [ ] **Step 1: Create `apps/node/runtimes/common/src/staking_election.rs`**

```rust
//! Solver wiring shared by impetus's NPoS pallets.
//!
//! `pallet-staking::GenesisElectionProvider` and the fallback path for
//! `pallet-election-provider-multi-phase` both use this on-chain Phragmen
//! configuration. The `MaxWinners` / voter bounds match the dev-fast
//! `VALIDATOR_COUNT_TARGET` budget.

use crate::{
    staking_constants::{MAX_NOMINATIONS, VALIDATOR_COUNT_TARGET},
    AccountId,
};
use frame_election_provider_support::{
    bounds::{ElectionBounds, ElectionBoundsBuilder},
    SequentialPhragmen,
};
use frame_support::parameter_types;
use sp_runtime::Perbill;

parameter_types! {
    pub MaxOnChainElectingVoters: u32 = 1024;
    pub MaxOnChainElectableTargets: u16 = 64;
    pub MaxActiveValidators: u32 = VALIDATOR_COUNT_TARGET;
    pub OffchainSolutionLengthLimit: u32 = 4 * 1024 * 1024;
    pub OffchainSolutionWeightLimit: frame_support::weights::Weight =
        frame_support::weights::Weight::from_parts(u64::MAX, u64::MAX);
    pub ElectionBoundsOnChain: ElectionBounds = ElectionBoundsBuilder::default()
        .voters_count(MaxOnChainElectingVoters::get().into())
        .targets_count(MaxOnChainElectableTargets::get().into())
        .build();
    pub ElectionBoundsMultiPhase: ElectionBounds = ElectionBoundsBuilder::default()
        .voters_count(MaxOnChainElectingVoters::get().into())
        .targets_count(MaxOnChainElectableTargets::get().into())
        .build();
    pub SignedRewardBase: u128 = 1 * crate::UNIT;
    pub SignedDepositBase: u128 = 1 * crate::UNIT;
    pub SignedDepositByte: u128 = 1_000_000;
    pub BetterSignedThreshold: Perbill = Perbill::from_rational(5u32, 10_000u32);
    pub MaxNominations: u32 = MAX_NOMINATIONS;
}

pub type OnChainSeqPhragmenScore = sp_npos_elections::ExtendedBalance;

pub struct OnChainSeqPhragmen<R>(core::marker::PhantomData<R>);

impl<R> frame_election_provider_support::onchain::Config for OnChainSeqPhragmen<R>
where
    R: frame_system::Config<AccountId = AccountId>
        + pallet_staking::Config
        + pallet_bags_list::Config<pallet_bags_list::Instance1>,
{
    type System = R;
    type Solver = SequentialPhragmen<AccountId, Perbill>;
    type DataProvider = pallet_staking::Pallet<R>;
    type WeightInfo = ();
    type MaxWinners = MaxActiveValidators;
    type Bounds = ElectionBoundsOnChain;
}
```

- [ ] **Step 2: Add `frame-election-provider-support` + `sp-npos-elections` to `runtimes/common/Cargo.toml`**

```toml
frame-election-provider-support = { workspace = true }
sp-npos-elections               = { git = "https://github.com/paritytech/polkadot-sdk", branch = "stable2603", default-features = false }
pallet-bags-list                = { workspace = true }
```

Add `sp-npos-elections` to `[workspace.dependencies]` in `apps/node/Cargo.toml` if not already present (it is a transitive of the election provider but we use the type alias directly).

Add to `[features].std`:

```toml
"frame-election-provider-support/std",
"pallet-bags-list/std",
"sp-npos-elections/std",
```

- [ ] **Step 3: Re-export from `lib.rs`**

```rust
pub mod staking_election;
pub use staking_election::{
    ElectionBoundsMultiPhase, ElectionBoundsOnChain, MaxActiveValidators,
    MaxOnChainElectableTargets, MaxOnChainElectingVoters, MaxNominations, OnChainSeqPhragmen,
};
```

- [ ] **Step 4: Verify**

```bash
cd apps/node && SKIP_WASM_BUILD=1 cargo check -p runtime-common
```

Expected: green.

- [ ] **Step 5: Commit**

```bash
git add apps/node/runtimes/common/src/staking_election.rs \
        apps/node/runtimes/common/src/lib.rs \
        apps/node/runtimes/common/Cargo.toml \
        apps/node/Cargo.toml apps/node/Cargo.lock
git commit -m "feat(runtime): add OnChainSeqPhragmen + multi-phase bounds"
```

---

## Task 6: Crate-level dependencies for `runtime-impetus`

**Files:**
- Modify: `apps/node/runtimes/impetus/Cargo.toml`

- [ ] **Step 1: Add the new dep block**

Inside `[dependencies]`, add:

```toml
pallet-staking                        = { workspace = true }
pallet-staking-reward-curve           = { workspace = true }
pallet-session                        = { workspace = true, features = ["historical"] }
pallet-offences                       = { workspace = true }
pallet-bags-list                      = { workspace = true }
pallet-election-provider-multi-phase  = { workspace = true }
frame-election-provider-support       = { workspace = true }
pallet-nomination-pools               = { workspace = true }
pallet-treasury                       = { workspace = true }
pallet-fast-unstake                   = { workspace = true }
pallet-im-online                      = { workspace = true }
pallet-authority-discovery            = { workspace = true }
sp-staking                            = { workspace = true }
sp-npos-elections                     = { workspace = true }
```

- [ ] **Step 2: Extend `[features].std`**

Append these to the existing `std = [` list:

```toml
"pallet-staking/std",
"pallet-session/std",
"pallet-offences/std",
"pallet-bags-list/std",
"pallet-election-provider-multi-phase/std",
"frame-election-provider-support/std",
"pallet-nomination-pools/std",
"pallet-treasury/std",
"pallet-fast-unstake/std",
"pallet-im-online/std",
"pallet-authority-discovery/std",
"sp-staking/std",
"sp-npos-elections/std",
```

- [ ] **Step 3: Add `runtime-test-fast` feature**

```toml
[features]
runtime-test-fast = [
    "runtime-common/runtime-test-fast",
]
```

Leave the feature declared but un-exercised; Plan 3 will use it from E2E builds.

- [ ] **Step 4: Verify**

```bash
cd apps/node && SKIP_WASM_BUILD=1 cargo check -p impetus-runtime
```

Expected: green (no new Config impls yet — just deps).

- [ ] **Step 5: Commit**

```bash
git add apps/node/runtimes/impetus/Cargo.toml apps/node/Cargo.lock
git commit -m "chore(impetus): pull NPoS pallet deps into runtime crate"
```

---

## Task 7: `pallet-session` + `historical` Config

**Files:**
- Modify: `apps/node/runtimes/impetus/src/lib.rs`

- [ ] **Step 1: Add session imports + constants**

Near the existing `use` block at the top of the file, add:

```rust
use frame_support::traits::KeyOwnerProofSystem;
use runtime_common::{
    BONDING_DURATION_ERAS, MAX_NOMINATIONS, MAX_NOMINATORS_PER_VALIDATOR, MAX_VALIDATOR_COUNT,
    MIN_NOMINATOR_BOND, MIN_VALIDATOR_BOND, REPORT_LONGEVITY, SESSIONS_PER_ERA, SESSION_OFFSET,
    SESSION_PERIOD, SLASH_DEFER_DURATION_ERAS, UNIT, VALIDATOR_COUNT_TARGET,
};
use sp_staking::{EraIndex, SessionIndex};
```

Near the existing `parameter_types!` block, add:

```rust
parameter_types! {
    pub const Period: BlockNumber = SESSION_PERIOD;
    pub const Offset: BlockNumber = SESSION_OFFSET;
    pub const SessionsPerEra: SessionIndex = SESSIONS_PER_ERA;
    pub const BondingDuration: EraIndex = BONDING_DURATION_ERAS;
    pub const SlashDeferDuration: EraIndex = SLASH_DEFER_DURATION_ERAS;
}
```

- [ ] **Step 2: Add the `pallet-session` Config impl**

Below the existing `impl pallet_grandpa::Config for Runtime` block:

```rust
impl pallet_session::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type ValidatorId = AccountId;
    type ValidatorIdOf = pallet_staking::StashOf<Self>;
    type ShouldEndSession = pallet_session::PeriodicSessions<Period, Offset>;
    type NextSessionRotation = pallet_session::PeriodicSessions<Period, Offset>;
    type SessionManager =
        pallet_session::historical::NoteHistoricalRoot<Self, Staking>;
    type SessionHandler =
        <SessionKeys as sp_runtime::traits::OpaqueKeys>::KeyTypeIdProviders;
    type Keys = SessionKeys;
    type WeightInfo = ();
}

impl pallet_session::historical::Config for Runtime {
    type FullIdentification = sp_staking::Exposure<AccountId, Balance>;
    type FullIdentificationOf = pallet_staking::ExposureOf<Self>;
}
```

- [ ] **Step 3: Verify (will fail until Staking lands)**

```bash
cd apps/node && SKIP_WASM_BUILD=1 cargo check -p impetus-runtime 2>&1 | tail -40
```

Expected: errors referencing `Staking` (not yet in scope). That's fine — Task 10 wires `pallet-staking` and these errors clear. Do not commit yet.

---

## Task 8: `pallet-staking` Config (placeholder ElectionProvider)

**Files:**
- Modify: `apps/node/runtimes/impetus/src/lib.rs`

- [ ] **Step 1: Add `pallet-staking` parameter types and Config**

Below the session block:

```rust
parameter_types! {
    pub const MaxExposurePageSize: u32 = MAX_NOMINATORS_PER_VALIDATOR;
    pub const MaxUnlockingChunks: u32 = 32;
    pub const HistoryDepth: u32 = 84;
    pub HistoryDepthGet: u32 = HistoryDepth::get();
    pub const MinValidatorBondConst: Balance = MIN_VALIDATOR_BOND;
    pub const MinNominatorBondConst: Balance = MIN_NOMINATOR_BOND;
    pub const SlashRewardFraction: Perbill = Perbill::from_percent(10);
}

pub struct StakingBenchmarkingConfig;
impl pallet_staking::BenchmarkingConfig for StakingBenchmarkingConfig {
    type MaxNominators = frame_support::traits::ConstU32<1000>;
    type MaxValidators = frame_support::traits::ConstU32<1000>;
}

impl pallet_staking::Config for Runtime {
    type Currency = Balances;
    type CurrencyBalance = Balance;
    type UnixTime = Timestamp;
    type CurrencyToVote = sp_staking::currency_to_vote::SaturatingCurrencyToVote;
    type RewardRemainder = Treasury;
    type RuntimeEvent = RuntimeEvent;
    type Slash = Treasury;
    type Reward = ();
    type SessionsPerEra = SessionsPerEra;
    type BondingDuration = BondingDuration;
    type SlashDeferDuration = SlashDeferDuration;
    type AdminOrigin = frame_system::EnsureRoot<AccountId>;
    type SessionInterface = Self;
    type EraPayout = pallet_staking::ConvertCurve<runtime_common::reward_curve::REWARD_CURVE>;
    type NextNewSession = Session;
    type MaxExposurePageSize = MaxExposurePageSize;
    type ElectionProvider = ElectionProviderMultiPhase;
    type GenesisElectionProvider =
        frame_election_provider_support::onchain::OnChainExecution<
            runtime_common::OnChainSeqPhragmen<Self>,
        >;
    type VoterList = VoterList;
    type TargetList = pallet_staking::UseValidatorsMap<Self>;
    type NominationsQuota =
        pallet_staking::FixedNominationsQuota<{ MAX_NOMINATIONS }>;
    type MaxUnlockingChunks = MaxUnlockingChunks;
    type HistoryDepth = HistoryDepthGet;
    type EventListeners = NominationPools;
    type BenchmarkingConfig = StakingBenchmarkingConfig;
    type WeightInfo = ();
    type DisablingStrategy = pallet_staking::UpToLimitDisablingStrategy;
}
```

- [ ] **Step 2: Verify (still expected to fail until 9–11 land)**

```bash
cd apps/node && SKIP_WASM_BUILD=1 cargo check -p impetus-runtime 2>&1 | tail -40
```

Expected: errors for `Treasury`, `ElectionProviderMultiPhase`, `VoterList`, `NominationPools` — all wired in subsequent tasks. Do not commit yet.

---

## Task 9: `pallet-offences` + `pallet-bags-list` Config

**Files:**
- Modify: `apps/node/runtimes/impetus/src/lib.rs`

- [ ] **Step 1: Add offences + bags-list configs**

```rust
impl pallet_offences::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type IdentificationTuple = pallet_session::historical::IdentificationTuple<Self>;
    type OnOffenceHandler = Staking;
}

parameter_types! {
    pub const BagThresholds: &'static [u64] = &runtime_common::voter_bags::THRESHOLDS;
}

impl pallet_bags_list::Config<pallet_bags_list::Instance1> for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type ScoreProvider = Staking;
    type BagThresholds = BagThresholds;
    type Score = sp_npos_elections::VoteWeight;
    type WeightInfo = ();
}
```

- [ ] **Step 2: Verify (still expected to fail)**

```bash
cd apps/node && SKIP_WASM_BUILD=1 cargo check -p impetus-runtime 2>&1 | tail -20
```

Expected: errors for ElectionProviderMultiPhase, Treasury, NominationPools — cleared in next tasks.

---

## Task 10: `pallet-election-provider-multi-phase` Config

**Files:**
- Modify: `apps/node/runtimes/impetus/src/lib.rs`

- [ ] **Step 1: Add the EPM Config**

```rust
parameter_types! {
    pub const SignedPhase: u32 = 25;
    pub const UnsignedPhase: u32 = 25;
    pub const SignedMaxSubmissions: u32 = 10;
    pub const SignedMaxRefunds: u32 = 5;
    pub const SignedMaxWeight: Weight =
        Weight::from_parts(WEIGHT_MILLISECS_PER_BLOCK * 200, u64::MAX);
    pub const MinerMaxLength: u32 = 4 * 1024 * 1024;
    pub const MinerMaxWeight: Weight =
        Weight::from_parts(WEIGHT_MILLISECS_PER_BLOCK * 200, u64::MAX);
    pub const MinerTxPriority: u64 = 5_000_000_000;
}

frame_election_provider_support::generate_solution_type!(
    #[compact]
    pub struct NposCompactSolution16::<
        VoterIndex = u32,
        TargetIndex = u16,
        Accuracy = sp_runtime::PerU16,
        MaxVoters = runtime_common::MaxOnChainElectingVoters,
    >(16)
);

pub struct OnChainFallback;
impl frame_election_provider_support::ElectionProvider for OnChainFallback {
    type AccountId = AccountId;
    type BlockNumber = BlockNumber;
    type Error = &'static str;
    type DataProvider = Staking;
    type Pages = frame_support::traits::ConstU32<1>;
    type MaxWinnersPerPage = runtime_common::MaxActiveValidators;
    type MaxBackersPerWinner = frame_support::traits::ConstU32<MAX_NOMINATORS_PER_VALIDATOR>;

    fn elect(_page: u32) -> Result<
        frame_election_provider_support::BoundedSupportsOf<Self>,
        Self::Error,
    > {
        frame_election_provider_support::onchain::OnChainExecution::<
            runtime_common::OnChainSeqPhragmen<Runtime>,
        >::elect(0)
        .map_err(|_| "onchain fallback failed")
    }
}

impl pallet_election_provider_multi_phase::MinerConfig for Runtime {
    type AccountId = AccountId;
    type MaxLength = MinerMaxLength;
    type MaxWeight = MinerMaxWeight;
    type Solution = NposCompactSolution16;
    type MaxVotesPerVoter =
        <<Self as pallet_election_provider_multi_phase::Config>::DataProvider as
            frame_election_provider_support::ElectionDataProvider>::MaxVotesPerVoter;
    type MaxWinners = runtime_common::MaxActiveValidators;

    fn solution_weight(_v: u32, _t: u32, _a: u32, _d: u32) -> Weight {
        MinerMaxWeight::get()
    }
}

impl pallet_election_provider_multi_phase::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type Currency = Balances;
    type EstimateCallFee = TransactionPayment;
    type SignedPhase = SignedPhase;
    type UnsignedPhase = UnsignedPhase;
    type SignedMaxSubmissions = SignedMaxSubmissions;
    type SignedMaxRefunds = SignedMaxRefunds;
    type SignedRewardBase = runtime_common::staking_election::SignedRewardBase;
    type SignedDepositBase =
        pallet_election_provider_multi_phase::GeometricDepositBase<
            Balance,
            runtime_common::staking_election::SignedDepositBase,
            frame_support::traits::ConstU128<1_000_000>,
        >;
    type SignedDepositByte = runtime_common::staking_election::SignedDepositByte;
    type SignedDepositWeight = ();
    type SignedMaxWeight = SignedMaxWeight;
    type SlashHandler = Treasury;
    type RewardHandler = ();
    type SolutionImprovementThreshold = runtime_common::staking_election::BetterSignedThreshold;
    type MinerConfig = Self;
    type MinerTxPriority = MinerTxPriority;
    type DataProvider = Staking;
    type Fallback = OnChainFallback;
    type GovernanceFallback =
        frame_election_provider_support::onchain::OnChainExecution<
            runtime_common::OnChainSeqPhragmen<Self>,
        >;
    type Solver = frame_election_provider_support::SequentialPhragmen<
        AccountId,
        runtime_common::staking_election::BetterSignedThreshold,
        runtime_common::OnChainSeqPhragmen<Self>,
    >;
    type BenchmarkingConfig =
        pallet_election_provider_multi_phase::NoopElectionProviderBenchmarkingConfig;
    type ForceOrigin = frame_system::EnsureRoot<AccountId>;
    type ElectionBounds = runtime_common::ElectionBoundsMultiPhase;
    type WeightInfo = ();
}
```

> **Note:** The signed-phase miner client is intentionally not registered; the
> on-chain fallback handles election in dev (validator count ≤ 32). Risk register
> R4 covers this trade-off.

- [ ] **Step 2: Verify (still failing — Treasury / pools / fast-unstake pending)**

```bash
cd apps/node && SKIP_WASM_BUILD=1 cargo check -p impetus-runtime 2>&1 | tail -20
```

---

## Task 11: `pallet-im-online` + `pallet-authority-discovery` Config

**Files:**
- Modify: `apps/node/runtimes/impetus/src/lib.rs`

- [ ] **Step 1: Add im-online + authority-discovery configs**

```rust
parameter_types! {
    pub const ImOnlineUnsignedPriority: u64 = u64::MAX / 2;
    pub const MaxKeys: u32 = 1024;
    pub const MaxPeerInHeartbeats: u32 = 10_000;
}

impl pallet_im_online::Config for Runtime {
    type AuthorityId = pallet_im_online::sr25519::AuthorityId;
    type RuntimeEvent = RuntimeEvent;
    type NextSessionRotation = pallet_session::PeriodicSessions<Period, Offset>;
    type ValidatorSet = Historical;
    type ReportUnresponsiveness = Offences;
    type UnsignedPriority = ImOnlineUnsignedPriority;
    type WeightInfo = ();
    type MaxKeys = MaxKeys;
    type MaxPeerInHeartbeats = MaxPeerInHeartbeats;
}

parameter_types! {
    pub const MaxAuthorities: u32 = MAX_VALIDATOR_COUNT;
}

impl pallet_authority_discovery::Config for Runtime {
    type MaxAuthorities = MaxAuthorities;
}
```

- [ ] **Step 2: Verify (still failing)**

```bash
cd apps/node && SKIP_WASM_BUILD=1 cargo check -p impetus-runtime 2>&1 | tail -20
```

---

## Task 12: `pallet-treasury` Config

**Files:**
- Modify: `apps/node/runtimes/impetus/src/lib.rs`

- [ ] **Step 1: Add treasury config**

```rust
use frame_support::PalletId;
use sp_runtime::traits::IdentityLookup;

parameter_types! {
    pub const ProposalBond: Permill = Permill::from_percent(5);
    pub const ProposalBondMinimum: Balance = 100 * UNIT;
    pub const ProposalBondMaximum: Balance = 1_000_000 * UNIT;
    pub const SpendPeriod: BlockNumber = HOURS;
    pub const Burn: Permill = Permill::from_percent(0);
    pub const TreasuryPalletId: PalletId = PalletId(*b"py/trsry");
    pub TreasuryAccount: AccountId =
        <TreasuryPalletId as sp_runtime::traits::AccountIdConversion<AccountId>>::into_account_truncating(
            &TreasuryPalletId::get()
        );
    pub const MaxApprovals: u32 = 100;
    pub const PayoutPeriod: BlockNumber = 30 * DAYS;
}

impl pallet_treasury::Config for Runtime {
    type PalletId = TreasuryPalletId;
    type Currency = Balances;
    type RejectOrigin = frame_system::EnsureRoot<AccountId>;
    type RuntimeEvent = RuntimeEvent;
    type SpendPeriod = SpendPeriod;
    type Burn = Burn;
    type BurnDestination = ();
    type SpendFunds = ();
    type MaxApprovals = MaxApprovals;
    type WeightInfo = ();
    type SpendOrigin = frame_support::traits::NeverEnsureOrigin<Balance>;
    type AssetKind = ();
    type Beneficiary = AccountId;
    type BeneficiaryLookup = IdentityLookup<AccountId>;
    type Paymaster = frame_support::traits::tokens::PayFromAccount<Balances, TreasuryAccount>;
    type BalanceConverter = frame_support::traits::tokens::UnityAssetBalanceConversion;
    type PayoutPeriod = PayoutPeriod;
    #[cfg(feature = "runtime-benchmarks")]
    type BenchmarkHelper = ();
}
```

- [ ] **Step 2: Verify (still failing — pools/fast-unstake pending)**

```bash
cd apps/node && SKIP_WASM_BUILD=1 cargo check -p impetus-runtime 2>&1 | tail -20
```

---

## Task 13: `pallet-nomination-pools` + `pallet-fast-unstake` Config

**Files:**
- Modify: `apps/node/runtimes/impetus/src/lib.rs`

- [ ] **Step 1: Add pools + fast-unstake**

```rust
parameter_types! {
    pub const PoolsPalletId: PalletId = PalletId(*b"py/nopls");
    pub const MaxPointsToBalance: u8 = 10;
    pub const PoolMinJoinBond: Balance = 10 * UNIT;
    pub const PoolMinCreateBond: Balance = 1_000 * UNIT;
    pub const PoolMaxPools: u32 = 32;
    pub const PoolMaxMembers: u32 = 1_024;
    pub const PoolMaxMembersPerPool: u32 = 256;
    pub const PoolGlobalMaxCommission: Perbill = Perbill::from_percent(10);
}

impl pallet_nomination_pools::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type WeightInfo = ();
    type Currency = Balances;
    type RuntimeFreezeReason = RuntimeFreezeReason;
    type RewardCounter = sp_runtime::FixedU128;
    type BalanceToU256 = runtime_common::staking_election::BalanceToU256;
    type U256ToBalance = runtime_common::staking_election::U256ToBalance;
    type StakeAdapter =
        pallet_nomination_pools::adapter::TransferStake<Self, Staking>;
    type PostUnbondingPoolsWindow = frame_support::traits::ConstU32<4>;
    type MaxMetadataLen = frame_support::traits::ConstU32<256>;
    type MaxUnbonding = frame_support::traits::ConstU32<8>;
    type PalletId = PoolsPalletId;
    type MaxPointsToBalance = MaxPointsToBalance;
    type AdminOrigin = frame_system::EnsureRoot<AccountId>;
    type BlockNumberProvider = System;
    type Filter = frame_support::traits::Nothing;
}

parameter_types! {
    pub const FastUnstakeDeposit: Balance = UNIT;
    pub const FastUnstakeBatchSize: u32 = 4;
}

impl pallet_fast_unstake::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type ControlOrigin = frame_system::EnsureRoot<AccountId>;
    type BatchSize = FastUnstakeBatchSize;
    type Deposit = FastUnstakeDeposit;
    type Currency = Balances;
    type Staking = Staking;
    type MaxErasToCheckPerBlock = frame_support::traits::ConstU32<1>;
    type WeightInfo = ();
}
```

- [ ] **Step 2: Add `BalanceToU256` / `U256ToBalance` helpers**

The pools pallet expects helpers that aren't in stable2603 directly. Append to `apps/node/runtimes/common/src/staking_election.rs`:

```rust
use sp_core::U256;

pub struct BalanceToU256;
impl sp_runtime::traits::Convert<crate::Balance, U256> for BalanceToU256 {
    fn convert(n: crate::Balance) -> U256 {
        n.into()
    }
}

pub struct U256ToBalance;
impl sp_runtime::traits::Convert<U256, crate::Balance> for U256ToBalance {
    fn convert(n: U256) -> crate::Balance {
        use sp_runtime::traits::UniqueSaturatedInto;
        n.unique_saturated_into()
    }
}
```

- [ ] **Step 3: Verify**

```bash
cd apps/node && SKIP_WASM_BUILD=1 cargo check -p impetus-runtime 2>&1 | tail -20
```

Expected: only `construct_runtime!` errors remain (pallets aren't in the macro yet).

---

## Task 14: Swap Babe `EpochChangeTrigger` + wire equivocation reporting

**Files:**
- Modify: `apps/node/runtimes/impetus/src/lib.rs`

- [ ] **Step 1: Replace the Plan 1 Babe Config block**

Locate the existing `impl pallet_babe::Config for Runtime { ... }` and replace it with:

```rust
impl pallet_babe::Config for Runtime {
    type EpochDuration = EpochDuration;
    type ExpectedBlockTime = ExpectedBlockTime;
    type EpochChangeTrigger = pallet_babe::ExternalTrigger;
    type DisabledValidators = Session;
    type WeightInfo = ();
    type MaxAuthorities = MaxAuthorities;
    type MaxNominators = ConstU32<MAX_NOMINATORS_PER_VALIDATOR>;
    type KeyOwnerProof = <Historical as KeyOwnerProofSystem<(
        sp_core::crypto::KeyTypeId,
        sp_consensus_babe::AuthorityId,
    )>>::Proof;
    type EquivocationReportSystem = pallet_babe::EquivocationReportSystem<
        Self,
        Offences,
        Historical,
        ConstU64<REPORT_LONGEVITY>,
    >;
}
```

- [ ] **Step 2: Replace the GRANDPA Config block**

```rust
impl pallet_grandpa::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type WeightInfo = ();
    type MaxAuthorities = MaxAuthorities;
    type MaxNominators = ConstU32<MAX_NOMINATORS_PER_VALIDATOR>;
    type MaxSetIdSessionEntries = SessionsPerEra;
    type KeyOwnerProof = <Historical as KeyOwnerProofSystem<(
        sp_core::crypto::KeyTypeId,
        sp_consensus_grandpa::AuthorityId,
    )>>::Proof;
    type EquivocationReportSystem = pallet_grandpa::EquivocationReportSystem<
        Self,
        Offences,
        Historical,
        ConstU64<REPORT_LONGEVITY>,
    >;
}
```

---

## Task 15: Swap `pallet-authorship::FindAuthor` to Babe-aware source

**Files:**
- Modify: `apps/node/runtimes/impetus/src/lib.rs`

- [ ] **Step 1: Replace the Plan 1 Authorship Config**

Locate `impl pallet_authorship::Config for Runtime` and replace with:

```rust
impl pallet_authorship::Config for Runtime {
    type FindAuthor =
        pallet_session::FindAccountFromAuthorIndex<Self, Babe>;
    type EventHandler = (Staking, ImOnline);
}
```

This single change fixes Codex finding #2 (`block.coinbase = 0x0`): `Babe` now provides the slot author index, `pallet-session` maps it back to the registered `AccountId = H160`, and `pallet-evm::Config::FindAuthor = FindAuthorFromAuthorship<Self>` (unchanged from Plan 1) returns the real validator.

- [ ] **Step 2: Defer build verification until `construct_runtime!` adds the new pallets — next task**

---

## Task 16: Wire all new pallets into `construct_runtime!` + bump `spec_version`

**Files:**
- Modify: `apps/node/runtimes/impetus/src/lib.rs`

- [ ] **Step 1: Replace the `construct_runtime!` invocation**

Update the existing macro call to match the spec's index layout (indices 0–14 keep their Plan 1 positions; new pallets fill 17–28):

```rust
construct_runtime!(
    pub enum Runtime {
        System:               frame_system = 0,
        Timestamp:            pallet_timestamp = 1,
        Babe:                 pallet_babe = 2,
        Grandpa:              pallet_grandpa = 3,
        Balances:             pallet_balances = 4,
        TransactionPayment:   pallet_transaction_payment = 5,
        Sudo:                 pallet_sudo = 6,
        Ethereum:             pallet_ethereum = 7,
        EVM:                  pallet_evm = 8,
        EVMChainId:           pallet_evm_chain_id = 9,
        BaseFee:              pallet_base_fee = 10,
        ManualSeal:           pallet_manual_seal = 11,
        Assets:               pallet_assets = 12,
        GaslessRegistry:      pallet_gasless_registry = 14,
        Authorship:           pallet_authorship = 17,
        Session:              pallet_session = 18,
        Historical:           pallet_session::historical = 19,
        Staking:              pallet_staking = 20,
        Offences:             pallet_offences = 21,
        ElectionProviderMultiPhase: pallet_election_provider_multi_phase = 22,
        VoterList:            pallet_bags_list::<Instance1> = 23,
        AuthorityDiscovery:   pallet_authority_discovery = 24,
        ImOnline:             pallet_im_online = 25,
        NominationPools:      pallet_nomination_pools = 26,
        Treasury:             pallet_treasury = 27,
        FastUnstake:          pallet_fast_unstake = 28,
    }
);
```

- [ ] **Step 2: Bump `spec_version`**

In `RuntimeVersion`, change `spec_version: 3` to `spec_version: 4` (Plan 2 introduces 10 new pallets + storage layouts — a major runtime change).

- [ ] **Step 3: Verify native build**

```bash
cd apps/node && SKIP_WASM_BUILD=1 cargo check -p impetus-runtime 2>&1 | tail -40
```

Expected: green.

- [ ] **Step 4: Verify WASM build**

```bash
cd apps/node && cargo check -p impetus-runtime
```

Expected: green (this is the real WASM compile and is slow — first run may take 5+ minutes).

- [ ] **Step 5: Commit (squash of Tasks 7–16)**

```bash
git add apps/node/runtimes/impetus/src/lib.rs apps/node/runtimes/common/src/staking_election.rs
git commit -m "feat(impetus): wire full NPoS pallet stack + Babe ExternalTrigger"
```

---

## Task 17: Add NPoS-aware runtime APIs (Session, NominationPools, FastUnstake helpers)

**Files:**
- Modify: `apps/node/runtimes/impetus/src/lib.rs`

- [ ] **Step 1: Add the missing API impls inside `impl_runtime_apis!`**

Inside the existing `impl_runtime_apis! { ... }` block, after the `BabeApi` impl from Plan 1, add:

```rust
impl pallet_nomination_pools_runtime_api::NominationPoolsApi<Block, AccountId, Balance> for Runtime {
    fn pending_rewards(member_account: AccountId) -> Balance {
        NominationPools::api_pending_rewards(member_account).unwrap_or_default()
    }

    fn points_to_balance(pool_id: u32, points: Balance) -> Balance {
        NominationPools::api_points_to_balance(pool_id, points)
    }

    fn balance_to_points(pool_id: u32, new_funds: Balance) -> Balance {
        NominationPools::api_balance_to_points(pool_id, new_funds)
    }

    fn pool_pending_slash(pool_id: u32) -> Balance {
        NominationPools::api_pool_pending_slash(pool_id)
    }

    fn member_pending_slash(member: AccountId) -> Balance {
        NominationPools::api_member_pending_slash(member).unwrap_or_default()
    }

    fn pool_needs_delegate_migration(_pool_id: u32) -> bool { false }
    fn member_needs_delegate_migration(_member: AccountId) -> bool { false }
    fn member_total_balance(member: AccountId) -> Balance {
        NominationPools::api_member_total_balance(member).unwrap_or_default()
    }
    fn pool_balance(pool_id: u32) -> Balance {
        NominationPools::api_pool_balance(pool_id)
    }
    fn pool_accounts(pool_id: u32) -> (AccountId, AccountId) {
        NominationPools::api_pool_accounts(pool_id)
    }
}

impl pallet_staking_runtime_api::StakingApi<Block, Balance, AccountId> for Runtime {
    fn nominations_quota(balance: Balance) -> u32 {
        Staking::api_nominations_quota(balance)
    }
    fn eras_stakers_page_count(era: sp_staking::EraIndex, account: AccountId) -> sp_staking::Page {
        Staking::api_eras_stakers_page_count(era, account)
    }
    fn pending_rewards(era: sp_staking::EraIndex, account: AccountId) -> bool {
        Staking::api_pending_rewards(era, account)
    }
}
```

First add to `apps/node/Cargo.toml` `[workspace.dependencies]`:

```toml
pallet-nomination-pools-runtime-api = { git = "https://github.com/paritytech/polkadot-sdk", branch = "stable2603", default-features = false }
pallet-staking-runtime-api          = { git = "https://github.com/paritytech/polkadot-sdk", branch = "stable2603", default-features = false }
```

Then pull into `runtimes/impetus/Cargo.toml` `[dependencies]`:

```toml
pallet-nomination-pools-runtime-api = { workspace = true }
pallet-staking-runtime-api          = { workspace = true }
```

Add to `[features].std`:

```toml
"pallet-nomination-pools-runtime-api/std",
"pallet-staking-runtime-api/std",
```

- [ ] **Step 2: Verify**

```bash
cd apps/node && SKIP_WASM_BUILD=1 cargo check -p impetus-runtime
```

Expected: green.

- [ ] **Step 3: Commit**

```bash
git add apps/node/runtimes/impetus/src/lib.rs apps/node/runtimes/impetus/Cargo.toml apps/node/Cargo.lock
git commit -m "feat(impetus): expose NominationPools + Staking runtime APIs"
```

---

## Task 18: Genesis — session key helpers + impetus profile rewrite

**Files:**
- Modify: `apps/node/node/src/chain_spec.rs`

- [ ] **Step 1: Add session-key derivation helpers**

Near the top of the file (after the existing `from_seed` helper):

```rust
use impetus_runtime::SessionKeys as ImpetusSessionKeys;
use pallet_im_online::sr25519::AuthorityId as ImOnlineId;
use sp_authority_discovery::AuthorityId as AuthorityDiscoveryId;

fn impetus_session_keys(seed: &str) -> (AccountId, ImpetusSessionKeys) {
    let stash = match seed {
        "Alice" => H160::from_slice(
            &hex_literal::hex!("f39Fd6e51aad88F6F4ce6aB8827279cffFb92266"),
        ),
        "Bob" => H160::from_slice(
            &hex_literal::hex!("70997970C51812dc3A010C7d01b50e0d17dc79C8"),
        ),
        "Charlie" => H160::from_slice(
            &hex_literal::hex!("3C44CdDdB6a900fa2b585dd299e03d12FA4293BC"),
        ),
        "Dave" => H160::from_slice(
            &hex_literal::hex!("90F79bf6EB2c4f870365E785982E1f101E93b906"),
        ),
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

const NOMINATOR_STASH_HEX: [u8; 20] =
    hex_literal::hex!("15d34AAf54267DB7D7c367839AAf71A00a2C6A65");
```

Add `hex-literal = "0.4"` to `apps/node/node/Cargo.toml` `[dependencies]` if not already there.

- [ ] **Step 2: Rewrite `impetus_genesis_patch`**

Replace the entire function with:

```rust
fn impetus_genesis_patch(
    sudo_key: AccountId,
    endowed: Vec<AccountId>,
    chain_id: u64,
) -> serde_json::Value {
    use impetus_runtime as imp;

    let validators: [(AccountId, ImpetusSessionKeys); 4] = [
        impetus_session_keys("Alice"),
        impetus_session_keys("Bob"),
        impetus_session_keys("Charlie"),
        impetus_session_keys("Dave"),
    ];
    let nominator: AccountId = H160::from_slice(&NOMINATOR_STASH_HEX).into();

    // Build-time pre-conditions (R10 in the spec): without these, genesis
    // election fails and the chain panics at block 1.
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
                    balance: U256::from(1_000_000u128) * U256::from(UNITS),
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
            serde_json::json!([
                stash, stash,
                2_000u128 * UNITS,
                "Validator"
            ])
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
            "slashRewardFraction": 100_000_000u32, // 10% as Perbill ppm
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
```

- [ ] **Step 3: Update `impetus_config()` to drop the `authorities` arg**

Locate the caller of `impetus_genesis_patch` in `impetus_config()` and replace the body with:

```rust
pub fn impetus_config() -> ChainSpec {
    let profile = impetus_profile();
    let wasm = impetus_runtime::WASM_BINARY.expect("Impetus WASM not built");
    ChainSpec::builder(wasm, Default::default())
        .with_name(profile.display_name)
        .with_id(profile.spec_id)
        .with_chain_type(profile.chain_type.clone())
        .with_properties(properties(&profile))
        .with_genesis_config_patch(impetus_genesis_patch(
            admin_account(),
            endowed_accounts(),
            profile.evm_chain_id,
        ))
        .build()
}
```

- [ ] **Step 4: Confirm the nominator stash is in `endowed_accounts()`**

Check `apps/node/runtimes/common/src/lib.rs` — `endowed_accounts()` already includes Hardhat #0–#9 per Plan 1; account #4 (`0x15d34A...`) is among them. If not, add it explicitly and re-run the build-time `assert!`.

- [ ] **Step 5: Update existing chain_spec tests**

Update `impetus_spec_has_babe_authorities` (it asserts a non-empty `babe.authorities` array, but Plan 2 leaves that empty — `pallet-session` populates it at block 1). Replace with:

```rust
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
```

- [ ] **Step 6: Verify**

```bash
cd apps/node && SKIP_WASM_BUILD=1 cargo test -p frontier-template-node chain_spec
```

Expected: 7 tests pass (5 existing + 2 new; the old `impetus_spec_has_babe_authorities` was replaced).

- [ ] **Step 7: Commit**

```bash
git add apps/node/node/src/chain_spec.rs apps/node/node/Cargo.toml apps/node/Cargo.lock
git commit -m "feat(node): genesis with session keys + 4 validators + 1 nominator"
```

---

## Task 19: Test harness — `ExtBuilder` + `run_to_*` helpers

**Files:**
- Create: `apps/node/runtimes/impetus/tests/common.rs`

- [ ] **Step 1: Create the harness**

```rust
//! Shared test scaffolding for impetus runtime integration tests.
//!
//! Each integration test crate (`tests/<scenario>.rs`) is its own binary
//! compiled by cargo, so this `common.rs` is included as a regular module
//! via `mod common;` at the top of each test file.

#![allow(dead_code)]

use frame_support::{
    assert_ok, parameter_types,
    traits::{OnFinalize, OnInitialize},
};
use impetus_runtime::{
    AccountId, Babe, Balance, Balances, BlockNumber, NominationPools, Runtime, RuntimeEvent,
    RuntimeOrigin, Session, SessionKeys, Staking, System, Timestamp, Treasury, UNIT,
};
use sp_core::{H160, U256};
use sp_runtime::BuildStorage;

pub const ALICE: H160 = H160(hex_literal::hex!("f39Fd6e51aad88F6F4ce6aB8827279cffFb92266"));
pub const BOB: H160 = H160(hex_literal::hex!("70997970C51812dc3A010C7d01b50e0d17dc79C8"));
pub const CHARLIE: H160 = H160(hex_literal::hex!("3C44CdDdB6a900fa2b585dd299e03d12FA4293BC"));
pub const DAVE: H160 = H160(hex_literal::hex!("90F79bf6EB2c4f870365E785982E1f101E93b906"));
pub const NOMINATOR: H160 = H160(hex_literal::hex!("15d34AAf54267DB7D7c367839AAf71A00a2C6A65"));

pub fn account(h160: H160) -> AccountId { h160.into() }

pub struct ExtBuilder {
    pub balances: Vec<(AccountId, Balance)>,
    pub initial_validators: Vec<AccountId>,
    pub initial_nominator: Option<(AccountId, Vec<AccountId>)>,
}

impl Default for ExtBuilder {
    fn default() -> Self {
        let validators = vec![ALICE, BOB, CHARLIE, DAVE].into_iter().map(account).collect();
        let nominator = account(NOMINATOR);
        let balances = vec![
            (account(ALICE), 10_000 * UNIT),
            (account(BOB), 10_000 * UNIT),
            (account(CHARLIE), 10_000 * UNIT),
            (account(DAVE), 10_000 * UNIT),
            (nominator, 10_000 * UNIT),
        ];
        ExtBuilder {
            balances,
            initial_validators: validators,
            initial_nominator: Some((nominator, vec![account(ALICE), account(BOB), account(CHARLIE)])),
        }
    }
}

impl ExtBuilder {
    pub fn build(self) -> sp_io::TestExternalities {
        let mut storage = frame_system::GenesisConfig::<Runtime>::default()
            .build_storage()
            .unwrap();

        pallet_balances::GenesisConfig::<Runtime> {
            balances: self.balances.clone(),
        }
        .assimilate_storage(&mut storage)
        .unwrap();

        let session_keys: Vec<(AccountId, AccountId, SessionKeys)> = self
            .initial_validators
            .iter()
            .map(|v| {
                let dummy = sp_core::sr25519::Public::from_raw([0u8; 32]);
                let ed = sp_core::ed25519::Public::from_raw([0u8; 32]);
                let keys = SessionKeys {
                    babe: dummy.into(),
                    grandpa: ed.into(),
                    im_online: dummy.into(),
                    authority_discovery: dummy.into(),
                };
                (*v, *v, keys)
            })
            .collect();
        pallet_session::GenesisConfig::<Runtime> { keys: session_keys }
            .assimilate_storage(&mut storage)
            .unwrap();

        let mut stakers: Vec<_> = self
            .initial_validators
            .iter()
            .map(|v| (*v, *v, 2_000 * UNIT, pallet_staking::StakerStatus::<AccountId>::Validator))
            .collect();
        if let Some((nom, targets)) = self.initial_nominator.clone() {
            stakers.push((nom, nom, 5_000 * UNIT, pallet_staking::StakerStatus::Nominator(targets)));
        }
        pallet_staking::GenesisConfig::<Runtime> {
            validator_count: 4,
            minimum_validator_count: 1,
            invulnerables: vec![],
            slash_reward_fraction: sp_runtime::Perbill::from_percent(10),
            stakers,
            ..Default::default()
        }
        .assimilate_storage(&mut storage)
        .unwrap();

        let mut ext: sp_io::TestExternalities = storage.into();
        ext.execute_with(|| System::set_block_number(1));
        ext
    }
}

pub fn run_to_block(target: BlockNumber) {
    while System::block_number() < target {
        let n = System::block_number();
        <Babe as OnFinalize<_>>::on_finalize(n);
        <Session as OnFinalize<_>>::on_finalize(n);
        <Staking as OnFinalize<_>>::on_finalize(n);
        System::set_block_number(n + 1);
        Timestamp::set_timestamp((n as u64 + 1) * 6_000);
        <Babe as OnInitialize<_>>::on_initialize(n + 1);
        <Session as OnInitialize<_>>::on_initialize(n + 1);
        <Staking as OnInitialize<_>>::on_initialize(n + 1);
    }
}

pub fn run_to_session(idx: u32) {
    let target_block: BlockNumber =
        (idx as BlockNumber) * impetus_runtime::Period::get();
    if System::block_number() < target_block {
        run_to_block(target_block);
    }
}

pub fn run_to_era(era: u32) {
    let blocks_per_era: BlockNumber =
        impetus_runtime::Period::get() * impetus_runtime::SessionsPerEra::get();
    let target = blocks_per_era * (era + 1);
    if System::block_number() < target {
        run_to_block(target);
    }
}
```

- [ ] **Step 2: Verify the harness compiles standalone**

We need at least one test crate to drive the compile; do that in the next task.

---

## Task 20: `tests/genesis_election.rs` + `tests/era_progression.rs`

**Files:**
- Create: `apps/node/runtimes/impetus/tests/genesis_election.rs`
- Create: `apps/node/runtimes/impetus/tests/era_progression.rs`

- [ ] **Step 1: `genesis_election.rs`**

```rust
mod common;
use common::*;

use impetus_runtime::{Session, Staking};

#[test]
fn genesis_elects_four_validators() {
    ExtBuilder::default().build().execute_with(|| {
        run_to_block(2);
        let validators = Session::validators();
        assert_eq!(validators.len(), 4, "expected 4 validators elected at genesis");
        assert!(validators.contains(&account(ALICE)));
        assert!(validators.contains(&account(BOB)));
        assert!(validators.contains(&account(CHARLIE)));
        assert!(validators.contains(&account(DAVE)));
    });
}

#[test]
fn nominator_visible_in_staking_bonded() {
    ExtBuilder::default().build().execute_with(|| {
        let ledger = pallet_staking::Ledger::<impetus_runtime::Runtime>::get(account(NOMINATOR));
        assert!(ledger.is_some(), "nominator must be bonded at genesis");
        let l = ledger.unwrap();
        assert_eq!(l.active, 5_000 * impetus_runtime::UNIT);
    });
}
```

- [ ] **Step 2: `era_progression.rs`**

```rust
mod common;
use common::*;

use impetus_runtime::{SessionsPerEra, Staking};

#[test]
fn current_era_advances_after_sessions_per_era_sessions() {
    ExtBuilder::default().build().execute_with(|| {
        assert_eq!(Staking::current_era(), Some(0));
        run_to_session(SessionsPerEra::get() as u32 + 1);
        assert_eq!(Staking::current_era(), Some(1));
    });
}

#[test]
fn active_era_index_matches_current_minus_one_initially_until_first_session() {
    ExtBuilder::default().build().execute_with(|| {
        let active = Staking::active_era();
        assert!(active.is_some());
        assert_eq!(active.unwrap().index, 0);
    });
}
```

- [ ] **Step 3: Verify both compile + pass**

```bash
cd apps/node && SKIP_WASM_BUILD=1 cargo test -p impetus-runtime --test genesis_election --test era_progression
```

Expected: 4 tests pass.

- [ ] **Step 4: Commit**

```bash
git add apps/node/runtimes/impetus/tests/common.rs \
        apps/node/runtimes/impetus/tests/genesis_election.rs \
        apps/node/runtimes/impetus/tests/era_progression.rs
git commit -m "test(impetus): cover genesis election + era progression"
```

---

## Task 21: `tests/session_rotation.rs`

**Files:**
- Create: `apps/node/runtimes/impetus/tests/session_rotation.rs`

- [ ] **Step 1: Add the test**

```rust
mod common;
use common::*;

use impetus_runtime::{Period, Session};

#[test]
fn session_index_increments_each_period() {
    ExtBuilder::default().build().execute_with(|| {
        assert_eq!(Session::current_index(), 0);
        run_to_block(Period::get() + 1);
        assert_eq!(Session::current_index(), 1);
        run_to_block(Period::get() * 2 + 1);
        assert_eq!(Session::current_index(), 2);
    });
}

#[test]
fn session_validators_pulled_from_staking_on_rotation() {
    ExtBuilder::default().build().execute_with(|| {
        let pre = Session::validators();
        run_to_session(2);
        let post = Session::validators();
        // Same validator set in dev (no stake changes between sessions).
        assert_eq!(pre, post);
    });
}
```

- [ ] **Step 2: Verify**

```bash
cd apps/node && SKIP_WASM_BUILD=1 cargo test -p impetus-runtime --test session_rotation
```

Expected: 2 tests pass.

- [ ] **Step 3: Commit**

```bash
git add apps/node/runtimes/impetus/tests/session_rotation.rs
git commit -m "test(impetus): cover session rotation"
```

---

## Task 22: `tests/bond_lifecycle.rs` + `tests/nominate_payout.rs`

**Files:**
- Create: `apps/node/runtimes/impetus/tests/bond_lifecycle.rs`
- Create: `apps/node/runtimes/impetus/tests/nominate_payout.rs`

- [ ] **Step 1: `bond_lifecycle.rs`**

```rust
mod common;
use common::*;

use frame_support::assert_ok;
use impetus_runtime::{BondingDuration, Runtime, RuntimeOrigin, Staking};

#[test]
fn bond_extra_increases_ledger_active() {
    ExtBuilder::default().build().execute_with(|| {
        let stash = account(ALICE);
        let pre = pallet_staking::Ledger::<Runtime>::get(stash).unwrap().active;
        assert_ok!(Staking::bond_extra(RuntimeOrigin::signed(stash), 500 * impetus_runtime::UNIT));
        let post = pallet_staking::Ledger::<Runtime>::get(stash).unwrap().active;
        assert_eq!(post, pre + 500 * impetus_runtime::UNIT);
    });
}

#[test]
fn unbond_then_withdraw_after_bonding_duration() {
    ExtBuilder::default().build().execute_with(|| {
        let stash = account(ALICE);
        assert_ok!(Staking::unbond(RuntimeOrigin::signed(stash), 500 * impetus_runtime::UNIT));
        // Withdrawing too early returns Ok but does not credit funds (chunks remain).
        run_to_era(BondingDuration::get() as u32 + 1);
        assert_ok!(Staking::withdraw_unbonded(RuntimeOrigin::signed(stash), 0));
        let ledger = pallet_staking::Ledger::<Runtime>::get(stash).unwrap();
        assert!(ledger.unlocking.is_empty(), "unbonded chunks should be withdrawn");
    });
}
```

- [ ] **Step 2: `nominate_payout.rs`**

```rust
mod common;
use common::*;

use frame_support::assert_ok;
use impetus_runtime::{Balances, Runtime, RuntimeOrigin, Staking};

#[test]
fn nominator_balance_grows_after_payout_stakers() {
    ExtBuilder::default().build().execute_with(|| {
        let nom = account(NOMINATOR);
        let pre = Balances::free_balance(nom);
        run_to_era(2);
        // Payout era 1 (already finalized).
        let _ = Staking::payout_stakers(RuntimeOrigin::signed(nom), account(ALICE), 1);
        let post = Balances::free_balance(nom);
        assert!(post >= pre, "nominator balance should not decrease after payout call");
    });
}
```

- [ ] **Step 3: Verify**

```bash
cd apps/node && SKIP_WASM_BUILD=1 cargo test -p impetus-runtime --test bond_lifecycle --test nominate_payout
```

Expected: 3 tests pass.

- [ ] **Step 4: Commit**

```bash
git add apps/node/runtimes/impetus/tests/bond_lifecycle.rs \
        apps/node/runtimes/impetus/tests/nominate_payout.rs
git commit -m "test(impetus): cover bond + nominator payout lifecycle"
```

---

## Task 23: `tests/slashing.rs`

**Files:**
- Create: `apps/node/runtimes/impetus/tests/slashing.rs`

- [ ] **Step 1: Add the test**

The earlier draft of this test discarded the `report_offence` result, used a zero
exposure, and asserted `pot >= 0` (always true for `u128`). Strengthen it so the
test actually fails if (a) offences are silently rejected, (b) slash application
is unwired, or (c) `Slash = Treasury` routing is broken.

```rust
mod common;
use common::*;

use frame_support::assert_ok;
use impetus_runtime::{Offences, Runtime, SlashDeferDuration, Staking, Treasury, UNIT};
use sp_runtime::Perbill;
use sp_staking::{
    offence::{DisableStrategy, Kind, Offence, ReportOffence},
    SessionIndex,
};

type IdTuple = pallet_session::historical::IdentificationTuple<Runtime>;

/// Concrete offence with a fixed 10% slash fraction. The built-in
/// `UnresponsivenessOffence` always returns 0% when offender count < 7/3 of the
/// validator set, which masks a working slashing pipeline in a 4-validator
/// test runtime — hence the local type.
struct TestSlashOffence {
    session_index: SessionIndex,
    validator_set_count: u32,
    offenders: Vec<IdTuple>,
}

impl Offence<IdTuple> for TestSlashOffence {
    // `sp_staking::offence::Kind` is `[u8; 16]`. Literal MUST be exactly 16 bytes.
    const ID: Kind = *b"test_slash_10pct";
    type TimeSlot = SessionIndex;
    fn offenders(&self) -> Vec<IdTuple> { self.offenders.clone() }
    fn session_index(&self) -> SessionIndex { self.session_index }
    fn validator_set_count(&self) -> u32 { self.validator_set_count }
    fn time_slot(&self) -> Self::TimeSlot { self.session_index }
    fn slash_fraction(&self, _offenders_count: u32) -> Perbill {
        Perbill::from_percent(10)
    }
    fn disable_strategy(&self) -> DisableStrategy { DisableStrategy::WhenSlashed }
}

#[test]
fn offence_debits_offender_and_credits_treasury_after_slash_defer() {
    ExtBuilder::default().build().execute_with(|| {
        let offender = account(ALICE);

        // Advance to era 1 so eras_stakers has real exposure for ALICE.
        run_to_era(1);
        let active_era = Staking::active_era().expect("active era after era 1").index;
        let exposure = Staking::eras_stakers(active_era, &offender);
        assert!(
            exposure.total >= 2_000 * UNIT,
            "ALICE must hold the 2000 IPT genesis validator bond, got {}",
            exposure.total
        );

        let bond_before = pallet_staking::Ledger::<Runtime>::get(offender)
            .expect("ALICE bonded at era 1")
            .active;
        assert!(bond_before > 0);
        let pot_before = Treasury::pot();
        let session_idx = impetus_runtime::Session::current_index();

        // The Result MUST be Ok — discarding it was the bug in the prior draft.
        assert_ok!(
            <Offences as ReportOffence<_, IdTuple, TestSlashOffence>>::report_offence(
                vec![],
                TestSlashOffence {
                    session_index: session_idx,
                    validator_set_count: 4,
                    offenders: vec![(offender, exposure.clone())],
                },
            )
        );

        // Advance past the slash-defer window so pallet-staking applies the slash.
        run_to_era(active_era + SlashDeferDuration::get() + 1);

        let bond_after = pallet_staking::Ledger::<Runtime>::get(offender)
            .expect("ALICE remains bonded after slash")
            .active;
        let pot_after = Treasury::pot();

        assert!(
            bond_after < bond_before,
            "active bond must decrease after slash applies: before={bond_before}, after={bond_after}"
        );
        assert!(
            pot_after > pot_before,
            "treasury pot must grow when Slash = Treasury: before={pot_before}, after={pot_after}"
        );
    });
}
```

> The local `TestSlashOffence` exists because Substrate's bundled offence
> types (Babe equivocation, GRANDPA equivocation, ImOnline unresponsiveness)
> all carry slash curves that effectively return 0% in a 4-validator dev
> runtime — useless for verifying that `Slash = Treasury` actually moves
> funds. Defining the offence locally keeps the test deterministic.

- [ ] **Step 2: Verify**

```bash
cd apps/node && SKIP_WASM_BUILD=1 cargo test -p impetus-runtime --test slashing
```

Expected: 1 test passes. Pre-check: if this test PASSED with `Exposure { total: 0, .. }`, the test harness is broken — the assertion ladder above requires non-zero exposure to reach the slash apply branch.

- [ ] **Step 3: Commit**

```bash
git add apps/node/runtimes/impetus/tests/slashing.rs
git commit -m "test(impetus): cover offence reporting pipeline + slash defer"
```

---

## Task 24: `tests/pool_lifecycle.rs` + `tests/fast_unstake.rs`

**Files:**
- Create: `apps/node/runtimes/impetus/tests/pool_lifecycle.rs`
- Create: `apps/node/runtimes/impetus/tests/fast_unstake.rs`

- [ ] **Step 1: `pool_lifecycle.rs`**

```rust
mod common;
use common::*;

use frame_support::assert_ok;
use impetus_runtime::{NominationPools, RuntimeOrigin, UNIT};

#[test]
fn create_pool_then_join() {
    ExtBuilder::default().build().execute_with(|| {
        let depositor = account(ALICE);
        assert_ok!(NominationPools::create(
            RuntimeOrigin::signed(depositor),
            1_500 * UNIT,
            depositor,
            depositor,
            depositor,
        ));
        let pool_id = pallet_nomination_pools::LastPoolId::<impetus_runtime::Runtime>::get();
        assert!(pool_id > 0);

        let joiner = account(BOB);
        assert_ok!(NominationPools::join(
            RuntimeOrigin::signed(joiner),
            500 * UNIT,
            pool_id,
        ));
        let member = pallet_nomination_pools::PoolMembers::<impetus_runtime::Runtime>::get(joiner);
        assert!(member.is_some());
    });
}
```

- [ ] **Step 2: `fast_unstake.rs`**

```rust
mod common;
use common::*;

use frame_support::assert_ok;
use impetus_runtime::{FastUnstake, RuntimeOrigin, Staking};

#[test]
fn register_fast_unstake_appends_to_queue() {
    ExtBuilder::default().build().execute_with(|| {
        let stash = account(DAVE);
        // Validator must chill first to be eligible.
        assert_ok!(Staking::chill(RuntimeOrigin::signed(stash)));
        assert_ok!(FastUnstake::register_fast_unstake(RuntimeOrigin::signed(stash)));
        assert!(pallet_fast_unstake::Queue::<impetus_runtime::Runtime>::contains_key(stash));
    });
}
```

- [ ] **Step 3: Verify**

```bash
cd apps/node && SKIP_WASM_BUILD=1 cargo test -p impetus-runtime --test pool_lifecycle --test fast_unstake
```

Expected: 2 tests pass.

- [ ] **Step 4: Commit**

```bash
git add apps/node/runtimes/impetus/tests/pool_lifecycle.rs \
        apps/node/runtimes/impetus/tests/fast_unstake.rs
git commit -m "test(impetus): cover nomination pool + fast-unstake registration"
```

---

## Task 25: `tests/treasury_proposal.rs` + `tests/force_eras.rs`

**Files:**
- Create: `apps/node/runtimes/impetus/tests/treasury_proposal.rs`
- Create: `apps/node/runtimes/impetus/tests/force_eras.rs`

- [ ] **Step 1: `treasury_proposal.rs`**

```rust
mod common;
use common::*;

use frame_support::assert_ok;
use impetus_runtime::{Balances, RuntimeOrigin, Treasury, UNIT};

#[test]
fn approved_proposal_credits_beneficiary_by_spend_amount() {
    ExtBuilder::default().build().execute_with(|| {
        let proposer = account(ALICE);
        let beneficiary = account(BOB);
        let spend: u128 = 1_000 * UNIT;

        // Seed the treasury pot so the spend can actually execute.
        // Without this, the test would only verify that propose+approve do not
        // panic — a payout path could be silently broken and still pass.
        let pot_account = pallet_treasury::Pallet::<impetus_runtime::Runtime>::account_id();
        assert_ok!(Balances::transfer_keep_alive(
            RuntimeOrigin::signed(proposer),
            pot_account,
            10_000 * UNIT,
        ));
        let pot_seeded = Treasury::pot();
        assert!(
            pot_seeded >= spend,
            "treasury pot {pot_seeded} must cover the proposed spend {spend}"
        );

        assert_ok!(Treasury::propose_spend(
            RuntimeOrigin::signed(proposer),
            spend,
            beneficiary,
        ));
        let id = pallet_treasury::ProposalCount::<impetus_runtime::Runtime>::get() - 1;
        assert_ok!(Treasury::approve_proposal(RuntimeOrigin::root(), id));

        let beneficiary_pre = Balances::free_balance(beneficiary);
        let pot_pre = Treasury::pot();

        run_to_block(impetus_runtime::SpendPeriod::get() + 5);

        let beneficiary_post = Balances::free_balance(beneficiary);
        let pot_post = Treasury::pot();

        assert_eq!(
            beneficiary_post,
            beneficiary_pre + spend,
            "beneficiary must receive the exact spend amount after SpendPeriod"
        );
        assert!(
            pot_post <= pot_pre.saturating_sub(spend),
            "treasury pot must decrease by at least the spend amount: pre={pot_pre}, post={pot_post}"
        );
    });
}
```

- [ ] **Step 2: `force_eras.rs`**

```rust
mod common;
use common::*;

use frame_support::assert_ok;
use impetus_runtime::{RuntimeOrigin, Staking};
use pallet_staking::Forcing;

#[test]
fn force_new_era_flag_is_set() {
    ExtBuilder::default().build().execute_with(|| {
        assert_ok!(Staking::force_new_era(RuntimeOrigin::root()));
        assert_eq!(pallet_staking::ForceEra::<impetus_runtime::Runtime>::get(), Forcing::ForceNew);
    });
}

#[test]
fn force_no_eras_disables_election() {
    ExtBuilder::default().build().execute_with(|| {
        assert_ok!(Staking::force_no_eras(RuntimeOrigin::root()));
        assert_eq!(pallet_staking::ForceEra::<impetus_runtime::Runtime>::get(), Forcing::ForceNone);
    });
}
```

- [ ] **Step 3: Verify**

```bash
cd apps/node && SKIP_WASM_BUILD=1 cargo test -p impetus-runtime --test treasury_proposal --test force_eras
```

Expected: 3 tests pass.

- [ ] **Step 4: Commit**

```bash
git add apps/node/runtimes/impetus/tests/treasury_proposal.rs \
        apps/node/runtimes/impetus/tests/force_eras.rs
git commit -m "test(impetus): cover treasury propose+approve + force_era flags"
```

---

## Task 26: `tests/babe_smoke.rs`

**Files:**
- Create: `apps/node/runtimes/impetus/tests/babe_smoke.rs`

- [ ] **Step 1: Add the test**

```rust
mod common;
use common::*;

use impetus_runtime::{Babe, EpochDuration};

#[test]
fn babe_epoch_advances_after_epoch_duration_blocks() {
    ExtBuilder::default().build().execute_with(|| {
        let epoch_before = Babe::current_epoch_start();
        run_to_block(EpochDuration::get() as u32 + 5);
        let epoch_after = Babe::current_epoch_start();
        assert!(
            epoch_after > epoch_before,
            "Babe epoch should advance after EpochDuration blocks"
        );
    });
}

#[test]
fn babe_randomness_is_set_each_block() {
    ExtBuilder::default().build().execute_with(|| {
        run_to_block(2);
        let r = Babe::randomness();
        assert_ne!(r, [0u8; 32], "Babe randomness should not be all zeros after block 2");
    });
}
```

- [ ] **Step 2: Verify**

```bash
cd apps/node && SKIP_WASM_BUILD=1 cargo test -p impetus-runtime --test babe_smoke
```

Expected: 2 tests pass.

- [ ] **Step 3: Commit**

```bash
git add apps/node/runtimes/impetus/tests/babe_smoke.rs
git commit -m "test(impetus): cover Babe epoch advancement + randomness"
```

---

## Task 27: Wire authority-discovery worker + im-online OCW in `service/babe.rs`

**Files:**
- Modify: `apps/node/node/src/service/babe.rs`

- [ ] **Step 1: Add authority-discovery worker**

Locate the `new_full` function in `service/babe.rs`. After the Babe authoring task spawn block, add:

```rust
// Authority-discovery worker — populates the DHT with validator addresses
// so peers can establish direct connections. Single-node dev runs leave the
// DHT empty but the worker still runs (R6 in the spec).
{
    use sc_authority_discovery::WorkerConfig;
    let authority_discovery_role = sc_authority_discovery::Role::PublishAndDiscover(
        keystore_container.keystore(),
    );
    let dht_event_stream = network
        .event_stream("authority-discovery")
        .filter_map(|e| async move {
            match e {
                sc_network::event::Event::Dht(e) => Some(e),
                _ => None,
            }
        });
    let (worker, _service) = sc_authority_discovery::new_worker_and_service_with_config(
        WorkerConfig {
            publish_non_global_ips: false,
            ..Default::default()
        },
        client.clone(),
        Arc::new(network.clone()),
        Box::pin(dht_event_stream),
        authority_discovery_role,
        prometheus_registry.clone(),
    );
    task_manager
        .spawn_handle()
        .spawn("authority-discovery-worker", Some("networking"), worker.run());
}
```

- [ ] **Step 2: Remove the "Plan 1 known limitation" comment block**

Search for the comment that mentions `babe-worker exits during first poll` (added in Plan 1) and delete it — Plan 2's `ExternalTrigger` + `pallet-session` integration resolves the issue.

- [ ] **Step 3: Verify the node builds**

```bash
cd apps/node && cargo check -p frontier-template-node
```

Expected: green.

- [ ] **Step 4: Commit**

```bash
git add apps/node/node/src/service/babe.rs
git commit -m "feat(node): wire authority-discovery worker into Babe service"
```

---

## Task 28: End-to-end smoke run — impetus authors past a session and era boundary

**Files:** none (verification only)

The earlier draft of this gate slept 30 s and accepted any `Imported #N`
with N ≥ 1. With Plan 2's dev-fast constants (`BLOCKS_PER_SESSION = 100`,
`SESSIONS_PER_ERA = 6`) one session is ~10 min and one era is ~60 min, so
that gate never reaches the Plan 1 babe-worker failure boundary. The gate
below builds with `runtime-test-fast` (session ≈ 30 s, era ≈ 1 min) and
requires three orthogonal signals to pass.

- [ ] **Step 1: Build the release binary with `runtime-test-fast`**

```bash
cd apps/node && cargo build --release --features impetus-runtime/runtime-test-fast
```

Expected: green. The feature was declared in Task 6; this is its first use.

- [ ] **Step 2: Start a dev impetus node**

```bash
RUST_LOG=runtime=info,babe=info,session=info,staking=info \
  ./target/release/frontier-template-node \
    --chain impetus_dev_npos \
    --tmp \
    --validator \
    --alice \
    --unsafe-force-node-key-generation \
    > /tmp/impetus-smoke.log 2>&1 &
```

- [ ] **Step 3: Wait for ≥ 3 eras of headroom**

At runtime-test-fast (5 blocks × 6 s = 30 s session, 2 sessions/era = 60 s era),
180 s covers three era boundaries:

```bash
sleep 180
```

- [ ] **Step 4: Triple gate — blocks AND session rotation AND era ≥ 1**

All three commands below must exit 0. Any failure means Plan 2's claim
"babe-worker exit dissolves with pallet-session" did not actually
materialize.

(a) At least 20 blocks imported (proves continuous authoring, not just block #1):

```bash
test "$(grep -cE '^.*Imported #[0-9]+' /tmp/impetus-smoke.log)" -ge 20 \
    || { echo 'FAIL: fewer than 20 blocks imported'; tail -40 /tmp/impetus-smoke.log; exit 1; }
```

(b) At least one session rotation past genesis (proves `pallet-session::PeriodicSessions` fired and `ExternalTrigger` worked):

```bash
grep -qE 'New session.*index["= ]+[2-9]|session_index["= ]+[2-9]' /tmp/impetus-smoke.log \
    || { echo 'FAIL: no session rotation observed'; tail -40 /tmp/impetus-smoke.log; exit 1; }
```

(c) `Staking::CurrentEra` is `Some(n)` with `n >= 1` — reject `null`, `0x010000000000`, and `0x0100000000`:

```bash
ERA_HEX=$(curl -s -H 'Content-Type: application/json' \
  -d '{"jsonrpc":"2.0","method":"state_getStorage","params":["0x5f3e4907f716ac89b6347d15ececedca0b6a45321efae92aea15e0740ec7afe7"],"id":1}' \
  http://127.0.0.1:9944 | python3 -c 'import sys,json; print(json.load(sys.stdin).get("result"))')
echo "CurrentEra storage = $ERA_HEX"
case "$ERA_HEX" in
  None|null|""|0x010000000000|0x0100000000)
    echo 'FAIL: era did not advance past 0'; exit 1 ;;
  0x01*) echo 'OK: CurrentEra > 0' ;;
  *) echo "FAIL: unexpected encoding $ERA_HEX"; exit 1 ;;
esac
```

- [ ] **Step 5: Stop the node + capture log tail**

```bash
pkill -f frontier-template-node
tail -40 /tmp/impetus-smoke.log
```

- [ ] **Step 6: Record smoke result in commit message**

```bash
git commit --allow-empty -m "test(node): impetus smoke passes triple-gate (blocks + session + era)"
```

> If any gate fails, do NOT proceed to Plan 3. Likely culprits per spec
> risk register:
> - **R3**: equivocation proof needs session ≥ 1 (should be a no-op for a
>   single-validator smoke).
> - **R6**: authority-discovery needs ≥ 2 peers; single-node mode should
>   not block Babe authoring but may emit warnings — those are not failures.
> - **R10**: genesis election needs cumulative stake ≥
>   `MinimumValidatorCount × MinValidatorBond`; the build-time assertion in
>   Task 18 should catch this earlier.

---

## Task 29: Non-regression — impulse + dev still produce blocks

**Files:** none (verification only)

- [ ] **Step 1: Smoke impulse**

```bash
./target/release/frontier-template-node --chain impulse --tmp --validator \
  > /tmp/impulse-smoke.log 2>&1 &
sleep 20
grep "Imported #" /tmp/impulse-smoke.log | head -5
pkill -f frontier-template-node
```

Expected: at least one `Imported #N` line.

- [ ] **Step 2: Smoke dev (manual seal)**

```bash
./target/release/frontier-template-node --chain dev --tmp --alice --sealing manual \
  > /tmp/dev-smoke.log 2>&1 &
sleep 5
curl -s -H 'Content-Type: application/json' \
  -d '{"jsonrpc":"2.0","method":"engine_createBlock","params":[true,true,null],"id":1}' \
  http://127.0.0.1:9944
sleep 2
grep "Sealed block" /tmp/dev-smoke.log
pkill -f frontier-template-node
```

Expected: one `Sealed block at finalized #1` line.

- [ ] **Step 3: Run impulse runtime tests**

```bash
cd apps/node && SKIP_WASM_BUILD=1 cargo test -p impulse-runtime
```

Expected: all existing tests pass (no regression from common-crate refactors).

- [ ] **Step 4: Commit (empty marker)**

```bash
git commit --allow-empty -m "test(node): impulse + dev non-regression smoke passes"
```

---

## Task 30: Update `apps/node/CLAUDE.md` for NPoS

**Files:**
- Modify: `apps/node/CLAUDE.md`

- [ ] **Step 1: Remove the Plan 1 limitation note**

Delete the entire block:

```
**Plan 1 known limitation:** `--chain impetus_dev_npos --validator --alice` ...
```

- [ ] **Step 2: Replace the impetus pallet index map**

Update the impetus pallet list to match the spec layout:

```
**Impetus (NPoS, Plan 2 — full NPoS stack):**

0-System, 1-Timestamp, 2-Babe, 3-Grandpa, 4-Balances, 5-TransactionPayment,
6-Sudo, 7-Ethereum, 8-EVM, 9-EVMChainId, 10-BaseFee, 11-ManualSeal (idle),
12-Assets, 14-GaslessRegistry, 17-Authorship, 18-Session, 19-Historical,
20-Staking, 21-Offences, 22-ElectionProviderMultiPhase, 23-VoterList,
24-AuthorityDiscovery, 25-ImOnline, 26-NominationPools, 27-Treasury,
28-FastUnstake.
```

- [ ] **Step 3: Update spec_version row**

In the "Releasing a runtime upgrade" section, change `spec_version` references:

```
Current spec_version is 4 on impetus (Plan 2 NPoS pallets), 2 on impulse (unchanged).
```

- [ ] **Step 4: Add a brief "NPoS pallet timings" reference**

Below the "Key constants" table, add:

```
### NPoS timings (impetus only)

Defined in runtimes/common/src/staking_constants.rs (dev-fast profile).
A runtime-test-fast Cargo feature further compresses these for E2E (Plan 3).

| Constant              | Value                | Wall-clock |
|-----------------------|----------------------|------------|
| BLOCKS_PER_SESSION    | 100                  | 10 min     |
| SESSIONS_PER_ERA      | 6                    | 60 min     |
| BONDING_DURATION_ERAS | 4                    | ~4 hours   |
| SLASH_DEFER_ERAS      | 3                    | ~3 hours   |
| MIN_VALIDATOR_BOND    | 1,000 IPT            | n/a        |
| MIN_NOMINATOR_BOND    | 10 IPT               | n/a        |
| MAX_VALIDATOR_COUNT   | 32                   | n/a        |
```

- [ ] **Step 5: Commit**

```bash
git add apps/node/CLAUDE.md
git commit -m "docs(node): refresh CLAUDE.md for NPoS pallets (Plan 2)"
```

---

## Acceptance gate

Before declaring Plan 2 done, all of the following must hold:

- [ ] `cd apps/node && cargo build --release` succeeds.
- [ ] `cd apps/node && cargo build --release --features impetus-runtime/runtime-test-fast` succeeds.
- [ ] `cd apps/node && SKIP_WASM_BUILD=1 cargo test -p impetus-runtime` runs all 11 integration test scenarios green.
- [ ] `cd apps/node && SKIP_WASM_BUILD=1 cargo test -p impulse-runtime` still passes (no regression).
- [ ] `cd apps/node && cargo clippy --workspace -- -D warnings` is clean.
- [ ] **Task 28 full triple gate** — built with `impetus-runtime/runtime-test-fast` and 180 s wait, all three of:
  - (a) `grep -cE '^.*Imported #[0-9]+' /tmp/impetus-smoke.log` ≥ 20
  - (b) log contains a `New session ... index 2-9` or `session_index 2-9` line
  - (c) `Staking::CurrentEra` storage read returns hex starting `0x01` and is **not** `null`, `0x010000000000`, or `0x0100000000`
  All three must pass — a smoke that only sees block #1 reproduces the Plan 1 babe-worker failure mode and must be treated as FAIL even if no panic is logged.
- [ ] Task 29 smoke shows impulse + dev still produce blocks.
- [ ] `apps/node/CLAUDE.md` no longer references the Plan 1 babe-worker limitation.

> **Plan 3 hand-off:** Plan 3 introduces the 7 NPoS Solidity precompiles
> (`Staking`, `Session`, `NominationPools`, `FastUnstake`, `Treasury`,
> `BagsList`, `StakingAdmin`), wires `FrontierPrecompilesNpos`, ships the
> `scripts/dump-session-keys.ts` helper, swaps `command.rs::load_spec` so
> `impetus` / `mainnet` aliases point at a real production spec (with
> production validator material instead of Hardhat seeds), and adds the 9
> E2E spec files under `packages/contracts/test/`. The `runtime-test-fast`
> Cargo feature added in Task 6 is exercised by Plan 3's E2E build.
