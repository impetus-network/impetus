# Plan 1 — Impetus Babe + Grandpa Migration

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Swap `runtime-impetus` from solo Aura+Grandpa to Babe+Grandpa with hardcoded genesis validators (`SameAuthoritiesForever`), and split the node binary into a dual-path service that runs Babe for impetus while keeping Aura authoring for impulse + dev. End state: a single `frontier-template-node` binary that produces blocks on both `--chain impetus_dev_npos` (Babe) and `--chain impulse` / `--chain dev` (Aura), with no NPoS pallets yet.

**Architecture:** Move `SessionKeys` out of `runtimes/common` so each runtime owns its own opaque key set (impulse keeps `(Aura, Grandpa)`, impetus grows to `(Babe, Grandpa, ImOnline, AuthorityDiscovery)`). Split `FrontierPrecompiles` into a `Basic` variant (current 9 entries) and an empty `Npos` shell that Plan 2/3 will populate. In the node, extract shared `new_partial` scaffolding into `service_common.rs` and put the Aura path in `service_aura.rs` (lift-and-shift of today's code), with a new `service_babe.rs` for the Babe import queue, Babe authoring loop, authority-discovery worker, and im-online placeholder. `command.rs` selects the consensus path by `Network::from_spec_id`. Babe uses `EpochChangeTrigger = SameAuthoritiesForever` so we can ship without `pallet-session` — Plan 2 swaps to `ExternalTrigger` once session+staking lands.

**Tech Stack:** Rust 2021, polkadot-sdk `stable2603`, Frontier `stable2603`, `pallet-babe`, `pallet-grandpa`, `pallet-authorship`, `sc-consensus-babe`, `sc-consensus-babe-rpc`, `sp-authority-discovery`, `sc-authority-discovery`. No new runtime pallets beyond Babe + Authorship.

**Spec:** [`docs/superpowers/specs/2026-05-16-impetus-npos-via-precompiles-design.md`](../specs/2026-05-16-impetus-npos-via-precompiles-design.md)

**All paths below are relative to repo root** (`/Users/huyduan/projects/blockchain`) unless prefixed with `cd` in commands. The Rust workspace lives at `apps/node/`.

---

## File Map

**Created:**
- `apps/node/runtimes/impetus/src/session_keys.rs` — impetus's 4-key `SessionKeys`
- `apps/node/runtimes/impulse/src/session_keys.rs` — impulse's 2-key `SessionKeys` (moved from common)
- `apps/node/node/src/service/mod.rs` — re-export shim
- `apps/node/node/src/service/common.rs` — `new_partial`, `FullClient`, telemetry, Frontier backend, GRANDPA block import (shared)
- `apps/node/node/src/service/aura.rs` — current Aura import queue + authoring (lifted from `service.rs`)
- `apps/node/node/src/service/babe.rs` — Babe import queue, Babe authoring, authority-discovery worker

**Modified:**
- `apps/node/Cargo.toml` — add Babe/im-online/authority-discovery deps
- `apps/node/runtimes/common/Cargo.toml` — drop direct Aura dep
- `apps/node/runtimes/common/src/lib.rs` — remove `opaque::SessionKeys`
- `apps/node/runtimes/common/src/precompiles.rs` — split into `FrontierPrecompilesBasic<R>` + `FrontierPrecompilesNpos<R>` (Npos delegates to Basic until Plan 3 wires NPoS entries)
- `apps/node/runtimes/impetus/Cargo.toml` — add pallet-babe + pallet-authorship + im-online + authority-discovery types; drop pallet-aura
- `apps/node/runtimes/impetus/src/lib.rs` — Babe + Authorship wiring, new SessionKeys path, BabeApi + AuthorityDiscoveryApi runtime APIs, `spec_version` 2 → 3
- `apps/node/runtimes/impulse/src/lib.rs` — point at its own `session_keys.rs`
- `apps/node/node/Cargo.toml` — add sc-consensus-babe, sc-consensus-babe-rpc, sc-authority-discovery
- `apps/node/node/src/lib.rs` (or `main.rs`) — replace `mod service;` with `mod service;` re-export from `service/mod.rs`
- `apps/node/node/src/command.rs` — dispatch by `Network::from_spec_id`
- `apps/node/node/src/chain_spec.rs` — impetus spec id `impetus_dev_npos`, new `impetus_genesis_patch` emitting Babe + Grandpa authorities
- `apps/node/CLAUDE.md` — Babe-on-impetus consensus note, updated pallet index map

**Deleted:**
- `apps/node/node/src/service.rs` — content moves into the new `service/` directory

---

## Task 1: Add workspace deps for Babe + AuthorityDiscovery + ImOnline types

**Files:**
- Modify: `apps/node/Cargo.toml` (`[workspace.dependencies]`)

- [ ] **Step 1: Add Babe + authority-discovery + im-online to workspace deps**

Open `apps/node/Cargo.toml` and add these lines inside `[workspace.dependencies]`, immediately after the existing `sp-consensus-aura` declaration block. Pin every entry to `stable2603` to match the rest of the workspace.

```toml
# Babe consensus (impetus)
pallet-babe          = { git = "https://github.com/paritytech/polkadot-sdk", branch = "stable2603", default-features = false }
pallet-authorship    = { git = "https://github.com/paritytech/polkadot-sdk", branch = "stable2603", default-features = false }
sp-consensus-babe    = { git = "https://github.com/paritytech/polkadot-sdk", branch = "stable2603", default-features = false }
sc-consensus-babe    = { git = "https://github.com/paritytech/polkadot-sdk", branch = "stable2603" }
sc-consensus-babe-rpc = { git = "https://github.com/paritytech/polkadot-sdk", branch = "stable2603" }

# Authority discovery + ImOnline session-key types (no pallet config yet)
pallet-im-online             = { git = "https://github.com/paritytech/polkadot-sdk", branch = "stable2603", default-features = false }
sp-authority-discovery       = { git = "https://github.com/paritytech/polkadot-sdk", branch = "stable2603", default-features = false }
sc-authority-discovery       = { git = "https://github.com/paritytech/polkadot-sdk", branch = "stable2603" }
pallet-authority-discovery   = { git = "https://github.com/paritytech/polkadot-sdk", branch = "stable2603", default-features = false }
```

> `pallet-im-online` and `pallet-authority-discovery` are workspace dependencies now but only used for their `AuthorityId` types in this plan. The Config impls land in Plan 2.

- [ ] **Step 2: Verify workspace resolves**

```bash
cd apps/node && SKIP_WASM_BUILD=1 cargo check --workspace
```

Expected: green (no compile errors). New deps appear in `Cargo.lock`.

- [ ] **Step 3: Commit**

```bash
cd /Users/huyduan/projects/blockchain
git add apps/node/Cargo.toml apps/node/Cargo.lock
git commit -m "chore(node): add Babe + authority-discovery + im-online workspace deps"
```

---

## Task 2: Move impulse SessionKeys out of `runtimes/common`

**Files:**
- Create: `apps/node/runtimes/impulse/src/session_keys.rs`
- Modify: `apps/node/runtimes/impulse/src/lib.rs`
- Modify: `apps/node/runtimes/common/src/lib.rs` (remove `opaque::SessionKeys`)

- [ ] **Step 1: Create `apps/node/runtimes/impulse/src/session_keys.rs`**

```rust
//! Opaque session keys for the impulse (testnet) runtime.
//!
//! Aura + Grandpa only — staking-free, single permissioned authority set.
//! Per-runtime location keeps impetus free to grow a 4-key set without
//! coupling the testnet to Babe types.

use sp_runtime::impl_opaque_keys;
use sp_consensus_aura::sr25519::AuthorityId as AuraId;
use sp_consensus_grandpa::AuthorityId as GrandpaId;

impl_opaque_keys! {
    pub struct SessionKeys {
        pub aura: AuraId,
        pub grandpa: GrandpaId,
    }
}
```

- [ ] **Step 2: Re-export from impulse `lib.rs`**

Add near the top of `apps/node/runtimes/impulse/src/lib.rs`, after the existing `mod` declarations / use statements:

```rust
pub mod session_keys;
pub use session_keys::SessionKeys;
```

If the runtime currently references `runtime_common::opaque::SessionKeys`, replace those references with the local `SessionKeys` path.

- [ ] **Step 3: Drop `opaque::SessionKeys` from `runtimes/common/src/lib.rs`**

Open `apps/node/runtimes/common/src/lib.rs` and locate the `pub mod opaque { ... }` block (around lines 37–54 per the spec read). **Delete** the `impl_opaque_keys! { pub struct SessionKeys { ... } }` invocation inside it. Keep `pub use sp_runtime::OpaqueExtrinsic as UncheckedExtrinsic` and the `Header` / `Block` / `BlockId` aliases — those are still shared.

Result of the `opaque` mod after edit:

```rust
pub mod opaque {
    use alloc::vec::Vec;
    use super::{generic, BlakeTwo256, BlockNumber};

    pub use sp_runtime::OpaqueExtrinsic as UncheckedExtrinsic;

    pub type Header = generic::Header<BlockNumber, BlakeTwo256>;
    pub type Block = generic::Block<Header, UncheckedExtrinsic>;
    pub type BlockId = generic::BlockId<Block>;
}
```

Also remove the now-unused `impl_opaque_keys` import from the top of `lib.rs` — keep `use sp_runtime::{generic, traits::{BlakeTwo256, IdentifyAccount, Verify}, Perbill};` and drop the `impl_opaque_keys` symbol from that import line.

Likewise drop `use sp_consensus_aura::sr25519::AuthorityId as AuraId;` from `common/src/lib.rs` — neither common type aliases nor the (about-to-be-deleted) SessionKeys reference it any more.

- [ ] **Step 4: Drop `sp-consensus-aura` from `runtimes/common/Cargo.toml`**

Open `apps/node/runtimes/common/Cargo.toml`. In `[dependencies]`, remove the `sp-consensus-aura = { workspace = true, default-features = false }` line. Also remove `sp-consensus-aura/std` from the `std` feature in `[features]` if present.

- [ ] **Step 5: Verify impulse still compiles**

```bash
cd apps/node && SKIP_WASM_BUILD=1 cargo check -p runtime-impulse
```

Expected: green.

If you get `cannot find type SessionKeys`, you missed a callsite — grep for it:

```bash
rg "opaque::SessionKeys|common::opaque::SessionKeys|runtime_common::opaque::SessionKeys" apps/node
```

Replace each hit with the local `crate::session_keys::SessionKeys` (or just `SessionKeys` if re-exported at crate root).

- [ ] **Step 6: Commit**

```bash
cd /Users/huyduan/projects/blockchain
git add apps/node/runtimes/common apps/node/runtimes/impulse
git commit -m "refactor(node): move SessionKeys out of runtime-common into impulse"
```

---

## Task 3: Define impetus SessionKeys (4 keys: Babe + Grandpa + ImOnline + AuthorityDiscovery)

**Files:**
- Create: `apps/node/runtimes/impetus/src/session_keys.rs`
- Modify: `apps/node/runtimes/impetus/Cargo.toml`
- Modify: `apps/node/runtimes/impetus/src/lib.rs`

- [ ] **Step 1: Add pallet-im-online + sp-authority-discovery to impetus deps**

Open `apps/node/runtimes/impetus/Cargo.toml`. In `[dependencies]`, add:

```toml
sp-consensus-babe = { workspace = true, default-features = false }
pallet-im-online = { workspace = true, default-features = false }
sp-authority-discovery = { workspace = true, default-features = false }
```

In `[features]`, append to the `std = [ ... ]` list:

```toml
"sp-consensus-babe/std",
"pallet-im-online/std",
"sp-authority-discovery/std",
```

- [ ] **Step 2: Create `apps/node/runtimes/impetus/src/session_keys.rs`**

```rust
//! Opaque session keys for the impetus (mainnet/NPoS) runtime.
//!
//! Four keys, registered atomically per validator via Session::set_keys.
//! Plan 2 will introduce pallet-session and SessionManager that exposes
//! these keys to the consensus and offence layers; Plan 1 declares the
//! struct so the runtime API surface is final from day one.

use sp_runtime::impl_opaque_keys;

use sp_consensus_babe::AuthorityId as BabeId;
use sp_consensus_grandpa::AuthorityId as GrandpaId;
use pallet_im_online::sr25519::AuthorityId as ImOnlineId;
use sp_authority_discovery::AuthorityId as AuthorityDiscoveryId;

impl_opaque_keys! {
    pub struct SessionKeys {
        pub babe: BabeId,
        pub grandpa: GrandpaId,
        pub im_online: ImOnlineId,
        pub authority_discovery: AuthorityDiscoveryId,
    }
}
```

- [ ] **Step 3: Re-export from impetus `lib.rs`**

Add near the top of `apps/node/runtimes/impetus/src/lib.rs`, mirroring the impulse pattern:

```rust
pub mod session_keys;
pub use session_keys::SessionKeys;
```

Remove any reference to `runtime_common::opaque::SessionKeys` in the impetus crate; replace with the local `SessionKeys` path.

- [ ] **Step 4: Verify impetus still compiles (Aura is still wired at this point)**

```bash
cd apps/node && SKIP_WASM_BUILD=1 cargo check -p runtime-impetus
```

Expected: green. The runtime still uses Aura authoring; we have only added the SessionKeys struct on top.

- [ ] **Step 5: Commit**

```bash
cd /Users/huyduan/projects/blockchain
git add apps/node/runtimes/impetus
git commit -m "feat(node): add 4-key SessionKeys for impetus (Babe/Grandpa/ImOnline/AuthDiscovery)"
```

---

## Task 4: Split `FrontierPrecompiles` into `Basic` + `Npos` variants

**Files:**
- Modify: `apps/node/runtimes/common/src/precompiles.rs`
- Modify: `apps/node/runtimes/impulse/src/lib.rs` (use `Basic`)
- Modify: `apps/node/runtimes/impetus/src/lib.rs` (use `Npos`)

- [ ] **Step 1: Rewrite `apps/node/runtimes/common/src/precompiles.rs`**

Replace the entire file:

```rust
use core::marker::PhantomData;
use pallet_evm::{
    IsPrecompileResult, Precompile, PrecompileHandle, PrecompileResult, PrecompileSet,
};
use sp_core::H160;

use pallet_evm_precompile_curve25519 as curve25519_precompile;
use pallet_evm_precompile_modexp::Modexp;
use pallet_evm_precompile_sha3fips::Sha3FIPS256;
use pallet_evm_precompile_simple::{ECRecover, ECRecoverPublicKey, Identity, Ripemd160, Sha256};
use precompile_batch::BatchPrecompile;
use precompile_gasless_registry::GaslessRegistryPrecompile;

/// Precompile set shipped on impulse (testnet) and dev mode.
///
/// Stable surface: Ethereum stdlib (1..=5), curve25519 / Sha3 / ECRecoverPK
/// (1024..=1027), gasless registry (0x0800), batch (0x0808). Adding entries
/// here requires bumping `spec_version` on both runtimes.
pub struct FrontierPrecompilesBasic<R>(PhantomData<R>);

impl<R> Default for FrontierPrecompilesBasic<R>
where
    R: pallet_evm::Config,
{
    fn default() -> Self {
        Self(Default::default())
    }
}

impl<R> FrontierPrecompilesBasic<R>
where
    R: pallet_evm::Config,
{
    pub fn new() -> Self {
        Self::default()
    }

    pub fn used_addresses() -> [H160; 11] {
        [
            hash(1),
            hash(2),
            hash(3),
            hash(4),
            hash(5),
            hash(1024),
            hash(1025),
            hash(1026),
            hash(1027),
            hash(precompile_gasless_registry::PRECOMPILE_ADDRESS),
            hash(precompile_batch::PRECOMPILE_ADDRESS),
        ]
    }
}

impl<R> PrecompileSet for FrontierPrecompilesBasic<R>
where
    R: pallet_evm::Config
        + frame_system::Config
        + pallet_gasless_registry::Config
        + pallet_sudo::Config,
    <R as frame_system::Config>::AccountId: Into<H160>,
    <R as frame_system::Config>::RuntimeOrigin:
        From<frame_support::dispatch::RawOrigin<<R as frame_system::Config>::AccountId>>,
{
    fn execute(&self, handle: &mut impl PrecompileHandle) -> Option<PrecompileResult> {
        match handle.code_address() {
            a if a == hash(1) => Some(ECRecover::execute(handle)),
            a if a == hash(2) => Some(Sha256::execute(handle)),
            a if a == hash(3) => Some(Ripemd160::execute(handle)),
            a if a == hash(4) => Some(Identity::execute(handle)),
            a if a == hash(5) => Some(Modexp::execute(handle)),
            a if a == hash(1024) => Some(Sha3FIPS256::<
                R,
                crate::weights::pallet_evm_precompile_sha3fips::WeightInfo<R>,
            >::execute(handle)),
            a if a == hash(1025) => Some(ECRecoverPublicKey::execute(handle)),
            a if a == hash(1026) => Some(curve25519_precompile::Curve25519Add::<
                R,
                crate::weights::pallet_evm_precompile_curve25519::WeightInfo<R>,
            >::execute(handle)),
            a if a == hash(1027) => Some(curve25519_precompile::Curve25519ScalarMul::<
                R,
                crate::weights::pallet_evm_precompile_curve25519::WeightInfo<R>,
            >::execute(handle)),
            a if a == hash(precompile_gasless_registry::PRECOMPILE_ADDRESS) => {
                Some(GaslessRegistryPrecompile::<R>::execute(handle))
            }
            a if a == hash(precompile_batch::PRECOMPILE_ADDRESS) => {
                Some(BatchPrecompile::<R>::execute(handle))
            }
            _ => None,
        }
    }

    fn is_precompile(&self, address: H160, _gas: u64) -> IsPrecompileResult {
        IsPrecompileResult::Answer {
            is_precompile: Self::used_addresses().contains(&address),
            extra_cost: 0,
        }
    }
}

/// Precompile set shipped on impetus (mainnet, NPoS).
///
/// Plan 1: delegates 1:1 to `FrontierPrecompilesBasic` (no NPoS entries yet).
/// Plan 3 will extend `used_addresses` and the dispatch match with seven
/// NPoS precompiles at 0x0810..=0x0840 and add the required pallet bounds.
pub struct FrontierPrecompilesNpos<R>(PhantomData<R>);

impl<R> Default for FrontierPrecompilesNpos<R>
where
    R: pallet_evm::Config,
{
    fn default() -> Self {
        Self(Default::default())
    }
}

impl<R> FrontierPrecompilesNpos<R>
where
    R: pallet_evm::Config,
{
    pub fn new() -> Self {
        Self::default()
    }

    pub fn used_addresses() -> [H160; 11] {
        FrontierPrecompilesBasic::<R>::used_addresses()
    }
}

impl<R> PrecompileSet for FrontierPrecompilesNpos<R>
where
    R: pallet_evm::Config
        + frame_system::Config
        + pallet_gasless_registry::Config
        + pallet_sudo::Config,
    <R as frame_system::Config>::AccountId: Into<H160>,
    <R as frame_system::Config>::RuntimeOrigin:
        From<frame_support::dispatch::RawOrigin<<R as frame_system::Config>::AccountId>>,
{
    fn execute(&self, handle: &mut impl PrecompileHandle) -> Option<PrecompileResult> {
        FrontierPrecompilesBasic::<R>::default().execute(handle)
    }

    fn is_precompile(&self, address: H160, gas: u64) -> IsPrecompileResult {
        FrontierPrecompilesBasic::<R>::default().is_precompile(address, gas)
    }
}

fn hash(a: u64) -> H160 {
    H160::from_low_u64_be(a)
}
```

- [ ] **Step 2: Point impulse runtime at `FrontierPrecompilesBasic`**

Grep impulse runtime for the existing precompile type alias / `pallet_evm::Config::PrecompilesType`:

```bash
rg "FrontierPrecompiles" apps/node/runtimes/impulse
```

Wherever you find `runtime_common::precompiles::FrontierPrecompiles` (or `FrontierPrecompiles<Self>`), rename to `FrontierPrecompilesBasic`. Keep the `<Self>` generic instantiation.

- [ ] **Step 3: Point impetus runtime at `FrontierPrecompilesNpos`**

```bash
rg "FrontierPrecompiles" apps/node/runtimes/impetus
```

Rename hits to `FrontierPrecompilesNpos<Self>` (same pattern). Plan 3 will add bounds; Plan 1 keeps the bounds aligned with `Basic`.

- [ ] **Step 4: Verify both runtimes compile**

```bash
cd apps/node && SKIP_WASM_BUILD=1 cargo check -p runtime-impulse -p runtime-impetus
```

Expected: green.

- [ ] **Step 5: Commit**

```bash
cd /Users/huyduan/projects/blockchain
git add apps/node/runtimes
git commit -m "refactor(node): split FrontierPrecompiles into Basic + Npos variants"
```

---

## Task 5: Wire `pallet-babe` into the impetus runtime (SameAuthoritiesForever)

**Files:**
- Modify: `apps/node/runtimes/impetus/Cargo.toml`
- Modify: `apps/node/runtimes/impetus/src/lib.rs`

- [ ] **Step 1: Add `pallet-babe` + `pallet-authorship` to impetus deps**

In `apps/node/runtimes/impetus/Cargo.toml`, under `[dependencies]`:

```toml
pallet-babe = { workspace = true, default-features = false }
pallet-authorship = { workspace = true, default-features = false }
```

In `[features].std`, append:

```toml
"pallet-babe/std",
"pallet-authorship/std",
```

- [ ] **Step 2: Add Babe + Authorship Config impls in `runtimes/impetus/src/lib.rs`**

Above the `construct_runtime!` block, after the existing `impl pallet_aura::Config for Runtime` (which we will remove in Task 7 — for now keep it side-by-side), add:

```rust
use sp_consensus_babe::AuthorityId as BabeId;

parameter_types! {
    // 100 blocks ≈ 10 min @ 6s. Plan 2 swaps the trigger and aligns this
    // with pallet-session's Period; Plan 1 holds the authority set fixed.
    pub const EpochDuration: u64 = 100;
    pub const ExpectedBlockTime: u64 = runtime_common::MILLISECS_PER_BLOCK;
    pub const MaxAuthorities: u32 = 32;
    pub const ReportLongevity: u64 = 24 * 100; // 24 epochs @ 100 blocks
}

impl pallet_babe::Config for Runtime {
    type EpochDuration = EpochDuration;
    type ExpectedBlockTime = ExpectedBlockTime;
    // Plan 1: authorities fixed at genesis. Plan 2 swaps to ExternalTrigger
    // once pallet-session is wired.
    type EpochChangeTrigger = pallet_babe::SameAuthoritiesForever;
    type DisabledValidators = ();
    type WeightInfo = ();
    type MaxAuthorities = MaxAuthorities;
    type MaxNominators = ConstU32<0>;
    // No equivocation reporting yet — Plan 2 wires this through pallet-session::historical.
    type KeyOwnerProof = sp_core::Void;
    type EquivocationReportSystem = ();
}

impl pallet_authorship::Config for Runtime {
    // Babe writes the slot author's index into the digest; FindAuthor below
    // (Task 6) reads it. Until pallet-session lands, the index maps onto the
    // genesis BabeAuthorities vector — i.e. our 4 hardcoded validators.
    type FindAuthor = pallet_session::FindAccountFromAuthorIndex<Self, Babe>;
    // EventHandler is `()` in Plan 1. Plan 2 swaps it to `(Staking, ImOnline)`.
    type EventHandler = ();
}
```

> The `FindAccountFromAuthorIndex` import path resolves once pallet-session is wired in Plan 2. Until then, the type doesn't actually need to compile against a fully concrete `pallet_session::Config` — the `pallet_authorship::Config::FindAuthor` bound is on `FindAuthor<...>`, not on Session itself. If `pallet_session` is not yet a dep, Step 5 below provides an alternative.

If your `pallet-session` dep is not yet present (Plan 2 brings it in), substitute:

```rust
impl pallet_authorship::Config for Runtime {
    type FindAuthor = pallet_babe::FindAuthorFromBabe<Self>;
    type EventHandler = ();
}
```

`FindAuthorFromBabe` reads from `pallet_babe::Authorities`, which works without pallet-session.

- [ ] **Step 3: Add `Babe` + `Authorship` to `construct_runtime!`**

In the `#[frame_support::runtime] mod runtime { ... }` block, register `Babe` at index 2 (replacing Aura — Task 7 deletes Aura's macro entry) and `Authorship` at index 17:

```rust
// existing entries unchanged except Aura (index 2):
#[runtime::pallet_index(2)]
pub type Babe = pallet_babe;
// ... other entries up to 16 ...
#[runtime::pallet_index(17)]
pub type Authorship = pallet_authorship;
```

> Index 2 was Aura. We are deliberately taking that slot because Plan 1 also removes Aura (Task 7). For one commit window, both Aura and Babe exist in the runtime — keep Aura at a temporary fresh index (e.g. 30) during this task only so the build is green, and let Task 7 delete it cleanly. Concretely: rename the existing `#[runtime::pallet_index(2)] pub type Aura = pallet_aura;` to `#[runtime::pallet_index(30)] pub type Aura = pallet_aura;` for this commit, then claim 2 for Babe.

- [ ] **Step 4: Verify compile**

```bash
cd apps/node && SKIP_WASM_BUILD=1 cargo check -p runtime-impetus
```

Expected: green. If you get `cannot find type pallet_session in scope` from `FindAccountFromAuthorIndex`, fall back to the `FindAuthorFromBabe` variant in Step 2.

- [ ] **Step 5: Commit**

```bash
cd /Users/huyduan/projects/blockchain
git add apps/node/runtimes/impetus
git commit -m "feat(node): wire pallet-babe (SameAuthoritiesForever) + pallet-authorship on impetus"
```

---

## Task 6: Switch impetus `pallet_evm::Config::FindAuthor` from Aura to Authorship

**Files:**
- Modify: `apps/node/runtimes/impetus/src/lib.rs`

- [ ] **Step 1: Update `FindAuthor` in `impl pallet_evm::Config for Runtime`**

Locate the existing line:

```rust
type FindAuthor = runtime_common::FindAuthorTruncated<Aura, Self>;
```

Replace with the Authorship-backed variant. Frontier ships a `FindAuthorTruncated` over any `FindAuthor` implementer; we point it at `Authorship`:

```rust
type FindAuthor = runtime_common::FindAuthorTruncated<Authorship, Self>;
```

If `runtime_common::FindAuthorTruncated` is generic over the Aura type alias specifically rather than over `FindAuthor`, check `apps/node/runtimes/common/src/lib.rs` for the definition. If needed, generalize the helper:

```rust
// In runtimes/common/src/lib.rs, replace the FindAuthorTruncated definition with:
pub struct FindAuthorTruncated<F, R>(core::marker::PhantomData<(F, R)>);

impl<F, R> frame_support::traits::FindAuthor<sp_core::H160>
    for FindAuthorTruncated<F, R>
where
    F: frame_support::traits::FindAuthor<<R as frame_system::Config>::AccountId>,
    R: frame_system::Config<AccountId = sp_core::H160>,
{
    fn find_author<'a, I>(digests: I) -> Option<sp_core::H160>
    where
        I: 'a + IntoIterator<Item = (sp_runtime::ConsensusEngineId, &'a [u8])>,
    {
        F::find_author(digests)
    }
}
```

That removes the Aura-specific bound and works for both `Aura` (impulse) and `Authorship` (impetus) since both implement `FindAuthor<AccountId20>`.

- [ ] **Step 2: Verify both runtimes still compile**

```bash
cd apps/node && SKIP_WASM_BUILD=1 cargo check -p runtime-impulse -p runtime-impetus
```

Expected: green.

- [ ] **Step 3: Commit**

```bash
cd /Users/huyduan/projects/blockchain
git add apps/node/runtimes
git commit -m "feat(node): point impetus pallet-evm FindAuthor at Authorship"
```

---

## Task 7: Remove `pallet-aura` from impetus

**Files:**
- Modify: `apps/node/runtimes/impetus/Cargo.toml`
- Modify: `apps/node/runtimes/impetus/src/lib.rs`

- [ ] **Step 1: Delete the `pallet-aura` Config impl**

In `apps/node/runtimes/impetus/src/lib.rs`, locate `impl pallet_aura::Config for Runtime { ... }` and delete the entire block.

- [ ] **Step 2: Remove Aura from `construct_runtime!`**

Delete the `#[runtime::pallet_index(30)] pub type Aura = pallet_aura;` line (the temporary index from Task 5). Index 2 is now permanently Babe; index 30 returns to the unused pool.

- [ ] **Step 3: Drop pallet-aura imports**

Remove `use sp_consensus_aura::sr25519::AuthorityId as AuraId;` and any `pallet_aura` symbol from `apps/node/runtimes/impetus/src/lib.rs`.

- [ ] **Step 4: Drop pallet-aura from `runtimes/impetus/Cargo.toml`**

Open `apps/node/runtimes/impetus/Cargo.toml`. Remove from `[dependencies]`:

```toml
pallet-aura = { workspace = true, default-features = false }
sp-consensus-aura = { workspace = true, default-features = false }
```

And from `[features].std`:

```toml
"pallet-aura/std",
"sp-consensus-aura/std",
```

- [ ] **Step 5: Verify compile**

```bash
cd apps/node && SKIP_WASM_BUILD=1 cargo check -p runtime-impetus
```

Expected: green.

- [ ] **Step 6: Commit**

```bash
cd /Users/huyduan/projects/blockchain
git add apps/node/runtimes/impetus
git commit -m "refactor(node): drop pallet-aura from impetus runtime"
```

---

## Task 8: Swap `AuraApi` for `BabeApi` in impetus runtime APIs

**Files:**
- Modify: `apps/node/runtimes/impetus/src/lib.rs`

- [ ] **Step 1: Remove `impl sp_consensus_aura::AuraApi<Block, AuraId> for Runtime`**

Locate the impl block (around lines 605–613 per the spec read) and delete it entirely.

- [ ] **Step 2: Add `impl sp_consensus_babe::BabeApi<Block> for Runtime`**

Inside `impl_runtime_apis! { ... }`, after the existing `Core` / `Metadata` impls, add:

```rust
impl sp_consensus_babe::BabeApi<Block> for Runtime {
    fn configuration() -> sp_consensus_babe::BabeConfiguration {
        let epoch_config = Babe::epoch_config().unwrap_or(BABE_GENESIS_EPOCH_CONFIG);
        sp_consensus_babe::BabeConfiguration {
            slot_duration: Babe::slot_duration(),
            epoch_length: EpochDuration::get(),
            c: epoch_config.c,
            authorities: Babe::authorities().to_vec(),
            randomness: Babe::randomness(),
            allowed_slots: epoch_config.allowed_slots,
        }
    }

    fn current_epoch_start() -> sp_consensus_babe::Slot {
        Babe::current_epoch_start()
    }

    fn current_epoch() -> sp_consensus_babe::Epoch {
        Babe::current_epoch()
    }

    fn next_epoch() -> sp_consensus_babe::Epoch {
        Babe::next_epoch()
    }

    fn generate_key_ownership_proof(
        _slot: sp_consensus_babe::Slot,
        _authority_id: sp_consensus_babe::AuthorityId,
    ) -> Option<sp_consensus_babe::OpaqueKeyOwnershipProof> {
        // Plan 1: no equivocation reporting yet (no pallet-session::historical).
        None
    }

    fn submit_report_equivocation_unsigned_extrinsic(
        _equivocation_proof: sp_consensus_babe::EquivocationProof<<Block as BlockT>::Header>,
        _key_owner_proof: sp_consensus_babe::OpaqueKeyOwnershipProof,
    ) -> Option<()> {
        // Plan 2 wires this through pallet-babe::EquivocationReportSystem.
        None
    }
}
```

- [ ] **Step 3: Add the genesis epoch config constant**

Near the top of `apps/node/runtimes/impetus/src/lib.rs`, after the existing `parameter_types!`, add:

```rust
/// Babe primary VRF slot ratio (1 in 4). Permissive enough that secondary
/// slots cover any epoch where no primary winner emerges.
pub const BABE_GENESIS_EPOCH_CONFIG: sp_consensus_babe::BabeEpochConfiguration =
    sp_consensus_babe::BabeEpochConfiguration {
        c: (1, 4),
        allowed_slots: sp_consensus_babe::AllowedSlots::PrimaryAndSecondaryVRFSlots,
    };
```

- [ ] **Step 4: Verify compile**

```bash
cd apps/node && SKIP_WASM_BUILD=1 cargo check -p runtime-impetus
```

Expected: green.

- [ ] **Step 5: Commit**

```bash
cd /Users/huyduan/projects/blockchain
git add apps/node/runtimes/impetus
git commit -m "feat(node): swap AuraApi for BabeApi in impetus runtime"
```

---

## Task 9: Add `AuthorityDiscoveryApi` runtime API

**Files:**
- Modify: `apps/node/runtimes/impetus/src/lib.rs`

- [ ] **Step 1: Add the impl inside `impl_runtime_apis!`**

```rust
impl sp_authority_discovery::AuthorityDiscoveryApi<Block> for Runtime {
    fn authorities() -> Vec<sp_authority_discovery::AuthorityId> {
        // Plan 1: no pallet-authority-discovery yet; expose Babe's authority
        // set projected onto sp_authority_discovery::AuthorityId. The DHT
        // worker (Task 12) registers but the list is small (4 keys) and
        // static. Plan 2 swaps to pallet-authority-discovery's storage once
        // session keys flow.
        Babe::authorities()
            .iter()
            .map(|(babe_id, _weight)| {
                // BabeId and AuthorityDiscoveryId are both sr25519::Public.
                let bytes: [u8; 32] = (*babe_id).clone().into();
                sp_authority_discovery::AuthorityId::from_slice(&bytes[..])
                    .expect("32-byte sr25519 public; qed")
            })
            .collect()
    }
}
```

- [ ] **Step 2: Verify compile**

```bash
cd apps/node && SKIP_WASM_BUILD=1 cargo check -p runtime-impetus
```

Expected: green.

- [ ] **Step 3: Commit**

```bash
cd /Users/huyduan/projects/blockchain
git add apps/node/runtimes/impetus
git commit -m "feat(node): add AuthorityDiscoveryApi runtime API on impetus"
```

---

## Task 10: Bump impetus `spec_version` 2 → 3

**Files:**
- Modify: `apps/node/runtimes/impetus/src/lib.rs`

- [ ] **Step 1: Update `RUNTIME_VERSION`**

Locate the `RUNTIME_VERSION: RuntimeVersion = ...` block (around line 123 per the spec read). Change `spec_version: 2` to `spec_version: 3`. Leave `impl_version` alone and bump `transaction_version` only if any extrinsic signature changes — Plan 1 does not change extrinsics, so leave it.

- [ ] **Step 2: Verify compile**

```bash
cd apps/node && SKIP_WASM_BUILD=1 cargo check -p runtime-impetus
```

Expected: green.

- [ ] **Step 3: Commit**

```bash
cd /Users/huyduan/projects/blockchain
git add apps/node/runtimes/impetus
git commit -m "chore(runtime): bump impetus spec_version 2 -> 3 for Babe migration"
```

---

## Task 11: Split `apps/node/node/src/service.rs` into `service/{mod,common,aura}.rs`

**Files:**
- Delete: `apps/node/node/src/service.rs`
- Create: `apps/node/node/src/service/mod.rs`
- Create: `apps/node/node/src/service/common.rs`
- Create: `apps/node/node/src/service/aura.rs`
- Modify: `apps/node/node/src/lib.rs` (or `main.rs`) — already declares `mod service;`, which now resolves to the directory

- [ ] **Step 1: Move `service.rs` contents**

Read the current `apps/node/node/src/service.rs`. Carve it into three pieces:

| Stays in | Items |
|---|---|
| `service/common.rs` | `FullClient`, `FullBackend`, `FrontierBackend`, `FullSelectChain`, `GrandpaBlockImport`, `GrandpaLinkHalf` type aliases; the `new_partial` fn; the `GrandpaPruningFilter` registration; everything imported by the import-queue builders |
| `service/aura.rs` | `build_aura_grandpa_import_queue`, the Aura authoring wiring, any Aura-specific imports (`sp_consensus_aura::*`, `sc_consensus_aura::*`, `AuraPair`) |
| `service/mod.rs` | `pub mod common; pub mod aura;` plus any items the rest of the binary expects to import (e.g. `pub use common::new_partial; pub use aura::new_full as new_full_aura;`) |

The exact item names depend on what's currently in `service.rs` — keep public names backwards-compatible where the rest of `node/src/` imports them; otherwise update those callsites in Task 13.

- [ ] **Step 2: Create `service/mod.rs`**

```rust
//! Node service dispatch.
//!
//! `common` holds the parts shared between Aura and Babe (client setup,
//! GRANDPA block import, Frontier backend, telemetry). `aura` powers
//! impulse + dev; `babe` (Task 12) powers impetus. `command.rs` picks
//! the path by chain spec id (Task 13).

pub mod aura;
pub mod common;

#[cfg(feature = "with-babe")]
pub mod babe;
```

> `with-babe` feature gate is optional; we keep `babe` always-on by default in this plan (no feature flag). Drop the `#[cfg(...)]` line if you don't want a feature gate; Task 12 will create `babe.rs` unconditionally.

- [ ] **Step 3: Verify compile**

```bash
cd apps/node && cargo build --release -p frontier-template-node
```

Expected: green. Aura authoring still works; the Babe module doesn't exist yet (Task 12 adds it).

- [ ] **Step 4: Smoke test impulse still produces blocks**

```bash
./target/release/frontier-template-node --chain impulse --tmp --validator &
NODE_PID=$!
sleep 20
curl -s -H 'Content-Type: application/json' \
    -d '{"id":1,"jsonrpc":"2.0","method":"chain_getHeader","params":[]}' \
    http://127.0.0.1:9944 | grep -q '"number":"0x[1-9a-f]'
SMOKE=$?
kill $NODE_PID
test $SMOKE -eq 0 && echo "PASS" || echo "FAIL"
```

Expected: `PASS` — block number ≥ 1.

- [ ] **Step 5: Commit**

```bash
cd /Users/huyduan/projects/blockchain
git add apps/node/node/src/service apps/node/node/src/service.rs
git commit -m "refactor(node): split service.rs into service/{mod,common,aura} modules"
```

(Use `git add -A apps/node/node/src/` if the delete + creates aren't picked up cleanly.)

---

## Task 12: Implement `service/babe.rs` (import queue + authoring + authority-discovery)

**Files:**
- Create: `apps/node/node/src/service/babe.rs`
- Modify: `apps/node/node/src/service/mod.rs` (uncomment `pub mod babe;`)
- Modify: `apps/node/node/Cargo.toml`

- [ ] **Step 1: Add Babe + authority-discovery deps to `apps/node/node/Cargo.toml`**

```toml
sc-consensus-babe = { workspace = true }
sc-consensus-babe-rpc = { workspace = true }
sp-consensus-babe = { workspace = true }
sc-authority-discovery = { workspace = true }
sp-authority-discovery = { workspace = true }
```

- [ ] **Step 2: Create `apps/node/node/src/service/babe.rs`**

This file is long. Implementation strategy:

1. **Lift-and-adapt** — Open `apps/node/node/src/service/aura.rs` (just created in Task 11) and copy the `new_full` function wholesale into `babe.rs`. Rename it freely; we'll restructure as we go.
2. **Swap consensus-specific blocks** — In the copy, replace the Aura-specific parts with the Babe equivalents (see substitution map below). Keep networking, RPC plumbing, GRANDPA spawning, transaction pool, telemetry, prometheus, offchain worker registration **identical** to the Aura path. Only the consensus blocks differ.
3. **Add authority-discovery** — After consensus is wired and only when `config.role.is_authority()`, spawn the DHT worker (block at the bottom of the skeleton below).

**Substitution map (Aura → Babe):**

| Replace this Aura wiring | …with this Babe wiring |
|---|---|
| `sc_consensus_aura::slot_duration(&*client)?` | `sc_consensus_babe::configuration(&*client)?` (returns `BabeConfiguration`; pull `.slot_duration()` from it) |
| `sc_consensus_aura::import_queue::<AuraPair, _, _, _, _, _>(...)` | `sc_consensus_babe::import_queue(sc_consensus_babe::ImportQueueParams { ... })` (see Step 2a below) |
| `sc_consensus_aura::start_aura(...)` | `sc_consensus_babe::start_babe(sc_consensus_babe::BabeParams { ... })` (see Step 2b) |
| `sp_consensus_aura::inherents::InherentDataProvider::from_timestamp_and_slot_duration` | `sp_consensus_babe::inherents::InherentDataProvider::from_timestamp_and_slot_duration` (different module path; same call shape) |
| `BlockImport<B>` instantiated directly | Wrap it in `sc_consensus_babe::block_import(...)` first — returns `(BabeBlockImport, BabeLink)`. The `BabeLink` is required by both the import queue *and* the authoring loop, so store it on the heap or pass by clone. |

**Skeleton for the unique Babe parts** — paste these into the lifted code at the appropriate spots:

```rust
//! Babe + Grandpa service for runtime-impetus.
//!
//! Plan 1: Babe runs `SameAuthoritiesForever`, so authority discovery is
//! seeded by the genesis BabeAuthorities. Plan 2 swaps to pallet-session
//! and lets validators rotate.

use std::{sync::Arc, time::Duration};

use sc_client_api::{Backend, BlockBackend};
use sc_consensus::{BasicQueue, BoxBlockImport};
use sc_consensus_babe::{BabeBlockImport, BabeLink, BabeWorkerHandle};
use sc_consensus_grandpa::{BlockNumberOps, GrandpaBlockImport, GrandpaPruningFilter};
use sc_executor::WasmExecutor;
use sc_service::{Configuration, TaskManager, error::Error as ServiceError};
use sc_telemetry::{Telemetry, TelemetryHandle};
use sc_transaction_pool_api::OffchainTransactionPoolFactory;
use sp_api::ConstructRuntimeApi;
use sp_consensus_babe::AuthorityId as BabeId;
use sp_runtime::traits::Block as BlockT;

use crate::service::common::{
    new_partial as common_new_partial, FullBackend, FullClient, FrontierBackend, FullSelectChain,
};

type BabeBlockImportT<B, C> =
    BabeBlockImport<B, C, GrandpaBlockImport<FullBackend<B>, B, C, FullSelectChain<B>>>;

pub fn build_babe_grandpa_import_queue<B, RA, HF>(
    client: Arc<FullClient<B, RA, HF>>,
    select_chain: FullSelectChain<B>,
    config: &Configuration,
    telemetry: Option<&Telemetry>,
    babe_link: BabeLink<B>,
    block_import: BabeBlockImportT<B, FullClient<B, RA, HF>>,
) -> Result<BasicQueue<B>, ServiceError>
where
    B: BlockT,
    <B as BlockT>::Header: BlockNumberOps,
    RA: ConstructRuntimeApi<B, FullClient<B, RA, HF>> + Send + Sync + 'static,
    RA::RuntimeApi: sp_consensus_babe::BabeApi<B>
        + sp_authority_discovery::AuthorityDiscoveryApi<B>,
    HF: sc_executor::HostFunctions + 'static,
{
    let slot_duration = babe_link.config().slot_duration();
    sc_consensus_babe::import_queue(sc_consensus_babe::ImportQueueParams {
        link: babe_link,
        block_import,
        justification_import: None,
        client,
        select_chain,
        inherent_data_providers: move |_, ()| async move {
            let timestamp = sp_timestamp::InherentDataProvider::from_system_time();
            let slot = sp_consensus_babe::inherents::InherentDataProvider::from_timestamp_and_slot_duration(
                *timestamp, slot_duration,
            );
            Ok((slot, timestamp))
        },
        spawner: &config.task_manager.spawn_essential_handle(),
        registry: config.prometheus_registry(),
        telemetry: telemetry.map(|t| t.handle()),
    })
    .map_err(Into::into)
}

pub fn new_full<RA, HF>(mut config: Configuration) -> Result<TaskManager, ServiceError>
where
    RA: ConstructRuntimeApi<Block, FullClient<Block, RA, HF>> + Send + Sync + 'static,
    RA::RuntimeApi: sp_consensus_babe::BabeApi<Block>
        + sp_authority_discovery::AuthorityDiscoveryApi<Block>
        + sp_consensus_grandpa::GrandpaApi<Block>
        + sp_api::ApiExt<Block>
        + sp_block_builder::BlockBuilder<Block>
        + sp_offchain::OffchainWorkerApi<Block>
        + sp_session::SessionKeys<Block>
        + sp_transaction_pool::runtime_api::TaggedTransactionQueue<Block>,
    HF: sc_executor::HostFunctions + 'static,
{
    // 1. Reuse common `new_partial` for client/backend/Frontier/Grandpa setup.
    let common_new_partial::PartialComponents {
        client, backend, mut task_manager, import_queue: _placeholder, keystore_container,
        select_chain, transaction_pool, other: (block_import_grandpa, grandpa_link,
            frontier_backend, telemetry),
    } = common_new_partial::<Block, RA, HF, _>(&config, |client, select_chain, config, telemetry, grandpa_block_import| {
        // Build Babe block import wrapping Grandpa
        let (babe_block_import, babe_link) = sc_consensus_babe::block_import(
            sc_consensus_babe::configuration(&*client)?,
            grandpa_block_import,
            client.clone(),
        )?;
        // Wrap with Frontier
        let block_import = fc_consensus::FrontierBlockImport::new(babe_block_import.clone(), client.clone());
        let import_queue = build_babe_grandpa_import_queue(
            client.clone(),
            select_chain.clone(),
            config,
            telemetry.as_ref(),
            babe_link.clone(),
            babe_block_import,
        )?;
        Ok((BoxBlockImport::new(block_import), import_queue, babe_link))
    })?;

    // 2. Networking, RPC, etc. — same shape as service_aura.rs::new_full;
    //    refer to the lifted Aura version for the exact build_network /
    //    build_offchain_workers / spawn_rpc_tasks calls. Differences below.

    // 3. Spawn Babe authoring (only when --validator).
    if config.role.is_authority() {
        let proposer = sc_basic_authorship::ProposerFactory::new(
            task_manager.spawn_handle(),
            client.clone(),
            transaction_pool.clone(),
            config.prometheus_registry(),
            telemetry.as_ref().map(|t| t.handle()),
        );
        let babe_config = sc_consensus_babe::BabeParams {
            keystore: keystore_container.keystore(),
            client: client.clone(),
            select_chain: select_chain.clone(),
            env: proposer,
            block_import: /* the babe_block_import returned from new_partial closure */,
            sync_oracle: /* network sync handle */,
            justification_sync_link: /* same */,
            create_inherent_data_providers: move |_parent, ()| async move {
                let timestamp = sp_timestamp::InherentDataProvider::from_system_time();
                let slot = sp_consensus_babe::inherents::InherentDataProvider::from_timestamp_and_slot_duration(
                    *timestamp, /* slot_duration */,
                );
                Ok((slot, timestamp))
            },
            force_authoring: config.force_authoring,
            backoff_authoring_blocks: Option::<()>::None,
            babe_link: /* the babe_link from new_partial */,
            block_proposal_slot_portion: sc_consensus_babe::SlotProportion::new(0.5),
            max_block_proposal_slot_portion: None,
            telemetry: telemetry.as_ref().map(|t| t.handle()),
        };
        let babe_worker = sc_consensus_babe::start_babe(babe_config)?;
        task_manager.spawn_essential_handle().spawn_blocking(
            "babe-proposer",
            Some("block-authoring"),
            babe_worker,
        );
    }

    // 4. Spawn authority-discovery worker (validator role only).
    if config.role.is_authority() {
        let (worker, service) = sc_authority_discovery::new_worker_and_service_with_config(
            sc_authority_discovery::WorkerConfig {
                publish_non_global_ips: config.network.allow_non_globals_in_dht,
                ..Default::default()
            },
            client.clone(),
            /* network handle */,
            Box::pin(/* dht event stream */),
            sc_authority_discovery::Role::PublishAndDiscover(keystore_container.keystore()),
            config.prometheus_registry(),
        );
        task_manager.spawn_handle().spawn(
            "authority-discovery-worker",
            Some("networking"),
            worker.run(),
        );
        // `service` handle is not consumed in Plan 1 beyond construction;
        // Plan 2/3 will wire it into ImOnline / paravalidation when relevant.
        let _ = service;
    }

    // 5. Spawn GRANDPA (lift from service_aura.rs — same call shape).

    Ok(task_manager)
}
```

> The skeleton is intentionally incomplete in places that mirror service_aura.rs exactly (networking handle, RPC, GRANDPA spawning). The implementer should copy those blocks from `service/aura.rs` and adjust only the import-queue and authoring sections. The signature of `common::new_partial` will likely need a generic closure parameter that returns `(BoxBlockImport, BasicQueue, BabeLink)` for Babe vs `(BoxBlockImport, BasicQueue)` for Aura — adjust the common `new_partial` to take the closure result as `Other` payload, OR add a sibling helper `new_partial_babe`. If the latter is cleaner, name it `new_partial_babe` and put it alongside `new_partial` in `service/common.rs` (or in `service/babe.rs` directly).

- [ ] **Step 3: Uncomment `pub mod babe;` in `service/mod.rs`**

If you gated `babe` behind `#[cfg(feature = "with-babe")]` in Task 11, remove the gate so the module is unconditional. The plan does not introduce a feature flag for it.

- [ ] **Step 4: Verify compile**

```bash
cd apps/node && cargo build --release -p frontier-template-node
```

Expected: green.

- [ ] **Step 5: Commit**

```bash
cd /Users/huyduan/projects/blockchain
git add apps/node/node
git commit -m "feat(node): implement service/babe.rs (import queue, authoring, authority-discovery)"
```

---

## Task 13: `command.rs` dispatch by `Network::from_spec_id`

**Files:**
- Modify: `apps/node/node/src/command.rs`
- Modify: `apps/node/node/src/chain_spec.rs` (extend `Network::from_spec_id` mapping)

- [ ] **Step 1: Extend `Network::from_spec_id`**

In `apps/node/node/src/chain_spec.rs`:

```rust
impl Network {
    pub fn from_spec_id(id: &str) -> Self {
        match id {
            "impetus" | "mainnet" | "impetus_dev_npos" => Network::Impetus,
            _ => Network::Impulse,
        }
    }
}
```

- [ ] **Step 2: Find the `run` (or `run_node`) entrypoint in `command.rs`**

Locate where the binary calls `service::new_full(config)` (or similar). Wrap it in a match:

```rust
use crate::chain_spec::Network;

match Network::from_spec_id(config.chain_spec.id()) {
    Network::Impetus => crate::service::babe::new_full::<
        impetus_runtime::RuntimeApi,
        sp_io::SubstrateHostFunctions,
    >(config),
    Network::Impulse => crate::service::aura::new_full::<
        impulse_runtime::RuntimeApi,
        sp_io::SubstrateHostFunctions,
    >(config),
}
```

If the existing call site is `service::new_full(config)` without runtime API generics, factor the generic out — both `babe::new_full` and `aura::new_full` are generic over `RuntimeApi`, but the binary chooses the runtime by chain spec id, so the match arms specify the concrete `RuntimeApi` per spec id.

- [ ] **Step 3: Verify compile**

```bash
cd apps/node && cargo build --release -p frontier-template-node
```

Expected: green.

- [ ] **Step 4: Commit**

```bash
cd /Users/huyduan/projects/blockchain
git add apps/node/node/src/command.rs apps/node/node/src/chain_spec.rs
git commit -m "feat(node): dispatch consensus path by chain spec id (impetus=Babe, impulse=Aura)"
```

---

## Task 14: Add `impetus_genesis_patch` + new spec id `impetus_dev_npos`

**Files:**
- Modify: `apps/node/node/src/chain_spec.rs`

- [ ] **Step 1: Add Babe key derivation helper**

Near the top of `apps/node/node/src/chain_spec.rs`, after `authority_keys_from_seed`, add:

```rust
use sp_consensus_babe::AuthorityId as BabeId;

fn impetus_authority_keys_from_seed(s: &str) -> (BabeId, GrandpaId) {
    (from_seed::<BabeId>(s), from_seed::<GrandpaId>(s))
}
```

- [ ] **Step 2: Update `impetus_profile`**

```rust
fn impetus_profile() -> ChainProfile {
    ChainProfile {
        display_name: "Impetus (Dev NPoS)",
        spec_id: "impetus_dev_npos",
        evm_chain_id: 388266,
        token_symbol: "IPT",
        ss58_prefix: 11434,
        chain_type: ChainType::Live,
        manual_seal: false,
    }
}
```

- [ ] **Step 3: Add `impetus_genesis_patch` function**

```rust
fn impetus_genesis_patch(
    sudo_key: AccountId,
    endowed: Vec<AccountId>,
    initial_authorities: Vec<(BabeId, GrandpaId)>,
    chain_id: u64,
) -> serde_json::Value {
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
            "authorities": initial_authorities.iter()
                .map(|x| (x.0.clone(), 1u64))
                .collect::<Vec<_>>(),
            "epochConfig": {
                "c": [1, 4],
                "allowed_slots": "PrimaryAndSecondaryVRFSlots"
            }
        },
        "grandpa": {
            "authorities": initial_authorities.iter()
                .map(|x| (x.1.clone(), 1u64))
                .collect::<Vec<_>>()
        },
        "evmChainId": { "chainId": chain_id },
        "evm": { "accounts": evm_accounts },
        "gaslessRegistry": { "rules": [] }
        // No manualSeal key — impetus does not use manual seal.
    })
}
```

- [ ] **Step 4: Update `impetus_config` to call `impetus_genesis_patch`**

```rust
pub fn impetus_config() -> ChainSpec {
    let profile = impetus_profile();
    let wasm = impetus_runtime::WASM_BINARY.expect("Impetus WASM not built");
    let authorities = vec![
        impetus_authority_keys_from_seed("Alice"),
        impetus_authority_keys_from_seed("Bob"),
        impetus_authority_keys_from_seed("Charlie"),
        impetus_authority_keys_from_seed("Dave"),
    ];
    ChainSpec::builder(wasm, Default::default())
        .with_name(profile.display_name)
        .with_id(profile.spec_id)
        .with_chain_type(profile.chain_type.clone())
        .with_properties(properties(&profile))
        .with_genesis_config_patch(impetus_genesis_patch(
            admin_account(),
            endowed_accounts(),
            authorities,
            profile.evm_chain_id,
        ))
        .build()
}
```

> `build_spec` stays in place for impulse + dev. impetus_config no longer goes through it; impetus has its own builder so the genesis patch can differ.

- [ ] **Step 5: Update `chain_spec::tests` to match new spec id**

In the test module (lines 172–215), the impetus test currently asserts `evmChainId == 388266` regardless of spec id — that still holds. Update the test name + add an explicit spec-id check:

```rust
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
fn impetus_spec_has_babe_authorities() {
    let spec = impetus_config();
    let json: serde_json::Value = serde_json::from_str(&spec.as_json(false).unwrap()).unwrap();
    let babe_auth = &json["genesis"]["runtimeGenesis"]["patch"]["babe"]["authorities"];
    assert!(babe_auth.as_array().map(|a| a.len() == 4).unwrap_or(false),
            "expected 4 Babe authorities at genesis");
}
```

- [ ] **Step 6: Run chain spec tests**

```bash
cd apps/node && cargo test -p frontier-template-node --lib chain_spec
```

Expected: all PASS.

- [ ] **Step 7: Commit**

```bash
cd /Users/huyduan/projects/blockchain
git add apps/node/node/src/chain_spec.rs
git commit -m "feat(node): impetus chain spec id 'impetus_dev_npos' with Babe genesis authorities"
```

---

## Task 15: Smoke test — `--chain impetus_dev_npos` produces blocks

**Files:**
- (no source changes — verification only)

- [ ] **Step 1: Build the release binary**

```bash
cd apps/node && cargo build --release
```

Expected: green.

- [ ] **Step 2: Start a single-validator impetus node**

In one terminal, from the repo root:

```bash
cd apps/node && ./target/release/frontier-template-node \
    --chain impetus_dev_npos --tmp --alice --validator \
    --rpc-port 9944
```

Leave running. Babe should pick `//Alice` from the keystore (the `--alice` flag pre-loads it) and produce blocks starting from slot leadership.

- [ ] **Step 3: Verify block production after 30 seconds**

In a second terminal:

```bash
sleep 30
curl -s -H 'Content-Type: application/json' \
    -d '{"id":1,"jsonrpc":"2.0","method":"chain_getHeader","params":[]}' \
    http://127.0.0.1:9944 | jq '.result.number'
```

Expected: a hex value `"0x[1-9a-f]..."`, i.e. block number ≥ 1.

- [ ] **Step 4: Verify BabeApi surface**

```bash
curl -s -H 'Content-Type: application/json' \
    -d '{"id":1,"jsonrpc":"2.0","method":"state_call","params":["BabeApi_configuration","0x"]}' \
    http://127.0.0.1:9944
```

Expected: a non-error response with SCALE-encoded `BabeConfiguration` in `result`.

- [ ] **Step 5: Stop the node**

`Ctrl+C` in the node terminal. The chain runs `--tmp`, so DB is discarded.

- [ ] **Step 6: Commit (smoke evidence)**

No code changes — note the smoke test result in a follow-up doc commit or skip the commit step.

---

## Task 16: Non-regression — `--chain impulse` and `--chain dev` still produce blocks

**Files:**
- (no source changes — verification only)

- [ ] **Step 1: Start impulse node**

```bash
cd apps/node && ./target/release/frontier-template-node \
    --chain impulse --tmp --alice --validator \
    --rpc-port 9944
```

- [ ] **Step 2: Verify block production**

```bash
sleep 20
curl -s -H 'Content-Type: application/json' \
    -d '{"id":1,"jsonrpc":"2.0","method":"chain_getHeader","params":[]}' \
    http://127.0.0.1:9944 | jq '.result.number'
```

Expected: block number ≥ 1.

Stop the node.

- [ ] **Step 3: Start `--chain dev` with manual seal**

```bash
./target/release/frontier-template-node --chain dev --tmp --alice --sealing manual \
    --rpc-port 9944
```

- [ ] **Step 4: Drive a block via `engine_createBlock`**

```bash
sleep 5
curl -s -H 'Content-Type: application/json' \
    -d '{"id":1,"jsonrpc":"2.0","method":"engine_createBlock","params":[true,true]}' \
    http://127.0.0.1:9944 | jq
```

Expected: a `result.hash` field — the seal succeeded.

Stop the node.

- [ ] **Step 5: Start `--chain dev` without `--sealing` (default Aura)**

```bash
./target/release/frontier-template-node --chain dev --tmp --alice --validator \
    --rpc-port 9944
```

- [ ] **Step 6: Verify Aura authoring**

```bash
sleep 20
curl -s -H 'Content-Type: application/json' \
    -d '{"id":1,"jsonrpc":"2.0","method":"chain_getHeader","params":[]}' \
    http://127.0.0.1:9944 | jq '.result.number'
```

Expected: block number ≥ 1.

Stop the node.

- [ ] **Step 7: Also run the existing E2E suites against `--chain dev`**

```bash
./target/release/frontier-template-node --chain dev --tmp --alice --sealing manual &
NODE_PID=$!
sleep 5
cd packages/contracts && pnpm test
RESULT=$?
kill $NODE_PID
test $RESULT -eq 0 && echo "PASS" || echo "FAIL"
```

Expected: existing gasless + batch + Echo suites all green.

---

## Task 17: Update `apps/node/CLAUDE.md` for Babe-on-impetus

**Files:**
- Modify: `apps/node/CLAUDE.md`

- [ ] **Step 1: Update the consensus row of the Chains table**

Locate the table (around lines 51–57). Update the impetus row's `spec_name` column to `impetus_dev_npos` and add a new column or footnote indicating Babe authoring. Concretely:

```markdown
| Chain   | Role     | Chain ID | Token | Decimals | SS58  | Consensus      | `spec_name` |
|---------|----------|----------|-------|----------|-------|----------------|-------------|
| Impetus | mainnet  | 388266   | IPT   | 18       | 11434 | Babe + Grandpa | `impetus`   |
| Impulse | testnet  | 322644   | IPL   | 18       | 11348 | Aura + Grandpa | `impulse`   |
| dev     | local    | 322644   | IPL   | 18       | 11348 | Aura (+ optional manual seal) | `impulse` (alias; `--sealing` is optional) |
```

(Add `Consensus` column. Keep the rest of the table as is.)

- [ ] **Step 2: Update the pallet index map**

Find the "Runtime pallets (indexed, identical in both runtimes)" block and rewrite it to acknowledge the divergence:

```markdown
### Runtime pallets

**Impulse (testnet, dev)** — unchanged from before NPoS work:

0-System, 1-Timestamp, 2-Aura, 3-Grandpa, 4-Balances, 5-TransactionPayment,
6-Sudo, 7-Ethereum, 8-EVM, 9-EVMChainId, 10-BaseFee, 11-ManualSeal, 12-Assets,
14-GaslessRegistry. Index 13 is intentionally skipped.

**Impetus (NPoS, Plan 1)** — Babe replaces Aura; pallet-authorship added:

0-System, 1-Timestamp, **2-Babe**, 3-Grandpa, 4-Balances, 5-TransactionPayment,
6-Sudo, 7-Ethereum, 8-EVM, 9-EVMChainId, 10-BaseFee, 11-ManualSeal (idle),
12-Assets, 14-GaslessRegistry, **17-Authorship**.

Plans 2 and 3 will add session, staking, election-provider-multi-phase,
bags-list, offences, im-online, authority-discovery, nomination-pools,
treasury, and fast-unstake.
```

- [ ] **Step 3: Add a note under `### Service (`node/src/service.rs`)`**

Replace that section with:

```markdown
### Service (`node/src/service/`)

`service/` is a directory module split into:

- `common.rs` — `new_partial` skeleton, client + Frontier backend + GRANDPA block import wiring, registered `Arc::new(GrandpaPruningFilter)` (required after the stable2603 bump).
- `aura.rs` — Aura import queue + authoring; powers `--chain impulse` and `--chain dev`.
- `babe.rs` — Babe import queue + authoring + authority-discovery worker; powers `--chain impetus_dev_npos`.

`command.rs::load_spec` resolves the chain spec id, and the run path dispatches via `Network::from_spec_id`:
- `"impetus" | "mainnet" | "impetus_dev_npos"` → `service::babe::new_full`
- everything else → `service::aura::new_full`

A single `frontier-template-node` binary handles both consensus paths.
```

- [ ] **Step 4: Update the Releasing section spec_version note**

Change:

> Current `spec_version` is `2` on both runtimes.

to:

> Current `spec_version` is `3` on impetus, `2` on impulse.

- [ ] **Step 5: Commit**

```bash
cd /Users/huyduan/projects/blockchain
git add apps/node/CLAUDE.md
git commit -m "docs(node): Babe-on-impetus + service split notes in CLAUDE.md"
```

---

## Acceptance Criteria

1. `cargo build --release -p frontier-template-node` succeeds.
2. `cargo test -p frontier-template-node --lib chain_spec` passes (new tests for spec id and Babe authority count green).
3. `cargo check -p runtime-impetus -p runtime-impulse` succeeds.
4. `--chain impetus_dev_npos --validator --alice` produces blocks (number ≥ 1 within 30s).
5. `--chain impulse --validator --alice` produces blocks (number ≥ 1 within 20s) — non-regression.
6. `--chain dev --sealing manual` mines blocks via `engine_createBlock` — non-regression.
7. `--chain dev --validator` (no `--sealing`) produces blocks via Aura authoring — non-regression.
8. Existing E2E suites (`packages/contracts/test/*.spec.ts` — gasless, batch, Echo) all green against `--chain dev --sealing manual`.
9. `runtime-impetus` `spec_version` is `3`; `runtime-impulse` `spec_version` is `2`.
10. `apps/node/CLAUDE.md` reflects the per-runtime consensus split and the new `service/` module layout.

## Self-Review Checklist (run before declaring Plan 1 done)

- [ ] Every task has a corresponding commit on the branch.
- [ ] `apps/node/node/src/service.rs` is deleted; `apps/node/node/src/service/{mod,common,aura,babe}.rs` exist.
- [ ] `runtimes/common/src/lib.rs` does not contain `impl_opaque_keys!`.
- [ ] `runtimes/{impetus,impulse}/src/session_keys.rs` exist.
- [ ] `precompiles.rs` defines both `FrontierPrecompilesBasic` and `FrontierPrecompilesNpos`.
- [ ] `pallet-aura` is absent from `runtimes/impetus/Cargo.toml`.
- [ ] `pallet-babe` + `pallet-authorship` present in `runtimes/impetus/Cargo.toml`.
- [ ] `Network::from_spec_id("impetus_dev_npos") == Network::Impetus`.
- [ ] `chain_spec::impetus_config()` builds without panic.
- [ ] No `cargo clippy -- -D warnings` regressions (run as a final gate).

---

**Next plan:** `2026-05-16-impetus-npos-pallets.md` — wires `pallet-session` + `pallet-staking` + offences + election-provider-multi-phase + bags-list + authority-discovery + im-online + nomination-pools + treasury + fast-unstake into the impetus runtime, flips `EpochChangeTrigger` from `SameAuthoritiesForever` to `ExternalTrigger`, and adds runtime integration tests for the full NPoS lifecycle.
