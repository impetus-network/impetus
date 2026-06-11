# Batch Precompile Implementation Plan

> **Status (2026-05-16):** This plan is **historical**. The batch precompile
> has shipped on `master` (commits up to `872654c`). Two patterns in the
> task steps below diverged from the final implementation and **must not be
> copy-pasted verbatim** if you re-execute parts of this plan:
>
> 1. **Mock test pattern.** Tasks 7–14 use `mock::deploy_code` +
>    `mock::revert_bytecode` / `mock::sstore_bytecode` to plant target
>    contracts. The stable2603 `precompile-utils::testing::MockHandle` does
>    **not** auto-execute deployed bytecode, so this approach was abandoned
>    in the shipped tests. Look at
>    `apps/node/precompiles/batch/src/lib.rs` (e.g. line 200 onwards) and
>    use the `with_subcall_handle(|s| SubcallOutput::succeed())` /
>    `SubcallOutput::revert()` closure pattern with `Rc<RefCell<...>>`
>    interior mutability for observations.
> 2. **Caller-funded sub-call semantics.** The shipped dispatch in
>    `precompiles/batch/src/mode.rs` uses
>    `Transfer { source: handle.context().caller, ... }` and
>    `Context.caller = handle.context().caller`, asserts
>    `handle.code_address() == handle.context().address` to reject
>    DELEGATECALL/CALLCODE, and bubbles `ExitReason::Fatal` unchanged in
>    every mode. Earlier drafts of this plan documented the
>    precompile-funded pattern; those sections have been updated, but if
>    you spot any leftover `caller: precompile_h160` or
>    `source: precompile_h160`, treat the shipped code as authoritative.
>
> Everything else (architecture, file map, address `0x0808`, ABI selectors,
> event encoding, constants, codec bounds, gas overhead, runtime wiring,
> E2E shape) still reflects what landed.

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add an EVM batch precompile at `0x0808` with Moonbeam-compatible `batchSome` / `batchSomeUntilFailure` / `batchAll` semantics, wired into both `impetus` and `impulse` runtimes.

**Architecture:** New crate `precompile-batch` under `apps/node/precompiles/batch/`. Uses Frontier `precompile-utils` macros for ABI codec + modifier enforcement. Logic lives in two files: `lib.rs` (entry points, constants, types) and `mode.rs` (dispatch loop). Wire into `runtimes/common`'s `FrontierPrecompiles` so both chains pick it up via re-export.

**Tech Stack:** Rust 2021, Substrate `polkadot-sdk` stable2603, Frontier stable2603, `precompile-utils` from `polkadot-evm/frontier` (stable2603), `hex-literal`, `sha3` (for compile-time keccak in tests). E2E: TypeScript, Hardhat, ethers v6.

**Spec:** [`docs/superpowers/specs/2026-05-15-batch-precompile-design.md`](../specs/2026-05-15-batch-precompile-design.md)

**All paths below are relative to repo root** (`/Users/huyduan/projects/blockchain`) unless prefixed with `cd` in commands. The Rust workspace lives at `apps/node/`.

---

## File Map

**Created:**
- `apps/node/precompiles/batch/Cargo.toml`
- `apps/node/precompiles/batch/Batch.sol`
- `apps/node/precompiles/batch/src/lib.rs`
- `apps/node/precompiles/batch/src/mode.rs`
- `apps/node/precompiles/batch/src/mock.rs`
- `apps/node/precompiles/batch/tests/integration.rs`
- `packages/contracts/contracts/Echo.sol`
- `packages/contracts/test/batch.spec.ts`

**Modified:**
- `apps/node/Cargo.toml` (add workspace member + workspace dep)
- `apps/node/runtimes/common/Cargo.toml` (add `precompile-batch` dep)
- `apps/node/runtimes/common/src/precompiles.rs` (add `0x0808` to `used_addresses()` + match arm)

---

## Task 1: Scaffold the `precompile-batch` crate

**Files:**
- Create: `apps/node/precompiles/batch/Cargo.toml`
- Create: `apps/node/precompiles/batch/src/lib.rs`
- Modify: `apps/node/Cargo.toml` (add member + workspace dep)

- [ ] **Step 1: Create `apps/node/precompiles/batch/Cargo.toml`**

```toml
[package]
name = "precompile-batch"
version = "0.0.0"
edition = "2021"
publish = false

[dependencies]
fp-evm = { workspace = true }
frame-support = { workspace = true }
frame-system = { workspace = true }
pallet-evm = { workspace = true }
precompile-utils = { workspace = true }
sp-core = { workspace = true }
sp-std = { workspace = true }

[dev-dependencies]
hex-literal = { workspace = true }
pallet-balances = { workspace = true }
pallet-timestamp = { workspace = true }
precompile-utils = { workspace = true, features = ["testing"] }
sha3 = "0.10"
sp-io = { workspace = true }
sp-runtime = { workspace = true }

[features]
default = ["std"]
std = [
	"fp-evm/std",
	"frame-support/std",
	"frame-system/std",
	"pallet-evm/std",
	"precompile-utils/std",
	"sp-core/std",
	"sp-std/std",
]
```

- [ ] **Step 2: Create `apps/node/precompiles/batch/src/lib.rs` stub**

```rust
#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

/// Precompile address: 0x0808 (2056).
pub const PRECOMPILE_ADDRESS: u64 = 2056;
```

- [ ] **Step 3: Modify `apps/node/Cargo.toml` — add workspace member**

In the `members = [...]` array (currently line 2-9), add `"precompiles/batch",` after `"precompiles/gasless-registry",`. Result:

```toml
[workspace]
members = [
	"node",
	"runtimes/common",
	"runtimes/impetus",
	"runtimes/impulse",
	"pallets/gasless-registry",
	"precompiles/gasless-registry",
	"precompiles/batch",
]
```

- [ ] **Step 4: Modify `apps/node/Cargo.toml` — add workspace dep**

After line `precompile-gasless-registry = { path = "precompiles/gasless-registry", default-features = false }` (around line 182), add:

```toml
precompile-batch = { path = "precompiles/batch", default-features = false }
```

- [ ] **Step 5: Verify the crate compiles**

```bash
cd apps/node && SKIP_WASM_BUILD=1 cargo check -p precompile-batch
```

Expected: `Finished` with no errors. Skeleton crate compiles.

- [ ] **Step 6: Commit**

```bash
cd /Users/huyduan/projects/blockchain
git add apps/node/Cargo.toml apps/node/precompiles/batch/
git commit -m "feat(node): scaffold precompile-batch crate"
```

---

## Task 2: Constants, event-topic computation, and the Solidity interface

**Files:**
- Modify: `apps/node/precompiles/batch/src/lib.rs`
- Create: `apps/node/precompiles/batch/Batch.sol`

- [ ] **Step 1: Write the topic-regression test first (TDD)**

Append to `apps/node/precompiles/batch/src/lib.rs`:

```rust
#[cfg(test)]
mod topic_tests {
	use super::*;
	use sha3::{Digest, Keccak256};

	fn topic(sig: &str) -> [u8; 32] {
		let mut h = Keccak256::new();
		h.update(sig.as_bytes());
		h.finalize().into()
	}

	#[test]
	fn subcall_succeeded_topic_matches_signature() {
		assert_eq!(SUBCALL_SUCCEEDED_TOPIC, topic("SubcallSucceeded(uint256)"));
	}

	#[test]
	fn subcall_failed_topic_matches_signature() {
		assert_eq!(SUBCALL_FAILED_TOPIC, topic("SubcallFailed(uint256)"));
	}
}
```

- [ ] **Step 2: Run the test, expect it to fail (constants undefined)**

```bash
cd apps/node && SKIP_WASM_BUILD=1 cargo test -p precompile-batch
```

Expected: compile error `cannot find value SUBCALL_SUCCEEDED_TOPIC`.

- [ ] **Step 3: Add the constants**

Replace the body of `apps/node/precompiles/batch/src/lib.rs` (keep the topic_tests module at the bottom):

```rust
#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

use sp_core::H256;

/// Precompile address: 0x0808 (2056).
pub const PRECOMPILE_ADDRESS: u64 = 2056;

/// Maximum number of sub-calls per batch. Codec-enforced.
pub const MAX_BATCH_SIZE: u32 = 256;

/// Maximum size of a single sub-call's `callData` in bytes (2 MiB). Codec-enforced.
pub const CALL_DATA_LIMIT: u32 = 2 * 1024 * 1024;

/// Fixed overhead charged once at the top of every batch dispatch.
pub const BASE_OVERHEAD: u64 = 1_000;

/// Per-sub-call decode/dispatch overhead charged after length validation.
pub const PER_SUBCALL_OVERHEAD: u64 = 1_500;

/// `keccak256("SubcallSucceeded(uint256)")`.
pub const SUBCALL_SUCCEEDED_TOPIC: [u8; 32] = [
	0x32, 0xf8, 0xa4, 0x21, 0x4b, 0xee, 0x9b, 0x21, 0x4b, 0xfc, 0xa1, 0x60, 0xa7, 0xee, 0xed, 0x29,
	0x4b, 0xfb, 0x55, 0x42, 0xea, 0x82, 0xc7, 0x65, 0x14, 0xbc, 0xab, 0xae, 0x9a, 0x9d, 0xa8, 0x5d,
];

/// `keccak256("SubcallFailed(uint256)")`.
pub const SUBCALL_FAILED_TOPIC: [u8; 32] = [
	0x73, 0x6c, 0xa5, 0x5d, 0xed, 0xd3, 0x21, 0x6a, 0x95, 0xd6, 0x3f, 0xc7, 0xc4, 0xa4, 0x5d, 0x4c,
	0xae, 0x14, 0x82, 0xa7, 0x0e, 0xc8, 0x7a, 0x97, 0x12, 0x29, 0xd9, 0x4b, 0x4f, 0xb6, 0x96, 0x86,
];

/// Topic constants converted to `H256` for use with `handle.log(...)`.
pub fn subcall_succeeded_topic() -> H256 { H256(SUBCALL_SUCCEEDED_TOPIC) }
pub fn subcall_failed_topic() -> H256    { H256(SUBCALL_FAILED_TOPIC) }
```

Note: the byte arrays above are placeholders. Step 4 confirms the test computes them — if they don't match, step 5 swaps in the real values from the test output.

- [ ] **Step 4: Run the test — it will print the actual topic bytes**

```bash
cd apps/node && SKIP_WASM_BUILD=1 cargo test -p precompile-batch --lib -- --nocapture
```

If the assertion fails, the panic message shows the real value: `left: [...] right: [...]`. Copy the `right` value (which is `topic(...)`) into the constants in `lib.rs`. Re-run until PASS.

- [ ] **Step 5: Create `apps/node/precompiles/batch/Batch.sol`**

```solidity
// SPDX-License-Identifier: Apache-2.0
pragma solidity >=0.8.3;

/// @dev Batch precompile at 0x0000000000000000000000000000000000000808.
/// Sub-calls execute with msg.sender = the immediate caller of the precompile
/// (EOA or intermediate contract). Native-value transfers debit directly from
/// that caller; the precompile never holds value. DELEGATECALL / CALLCODE
/// into this precompile is rejected. gasLimit[i] = 0 means "forward all
/// remaining gas".
interface Batch {
    /// Executes calls in order, continuing past any sub-call that reverts.
    /// Always returns to caller with success.
    function batchSome(
        address[] memory to,
        uint256[] memory value,
        bytes[]   memory callData,
        uint64[]  memory gasLimit
    ) external;

    /// Executes calls in order, stops on first revert but DOES NOT revert
    /// the outer call. Remaining indices are skipped.
    function batchSomeUntilFailure(
        address[] memory to,
        uint256[] memory value,
        bytes[]   memory callData,
        uint64[]  memory gasLimit
    ) external;

    /// Executes calls in order, reverts the entire batch if any sub-call reverts.
    /// Reverts of sub-calls bubble up with their original revert data.
    function batchAll(
        address[] memory to,
        uint256[] memory value,
        bytes[]   memory callData,
        uint64[]  memory gasLimit
    ) external;

    /// Emitted after each successful sub-call. `index` is the position in the
    /// input arrays (0-based). Non-indexed: lives in event data.
    event SubcallSucceeded(uint256 index);

    /// Emitted after each failed sub-call (only in batchSome / batchSomeUntilFailure).
    /// Non-indexed: lives in event data.
    event SubcallFailed(uint256 index);
}
```

- [ ] **Step 6: Commit**

```bash
cd /Users/huyduan/projects/blockchain
git add apps/node/precompiles/batch/
git commit -m "feat(node): add batch precompile constants, topics, and Batch.sol"
```

---

## Task 3: Define `BatchMode` and the dispatch skeleton

**Files:**
- Create: `apps/node/precompiles/batch/src/mode.rs`
- Modify: `apps/node/precompiles/batch/src/lib.rs`

- [ ] **Step 1: Create `apps/node/precompiles/batch/src/mode.rs`**

```rust
use frame_support::pallet_prelude::ConstU32;
use precompile_utils::prelude::*;
use sp_core::U256;

use crate::{CALL_DATA_LIMIT, MAX_BATCH_SIZE};

/// Behavior on per-sub-call failure.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum BatchMode {
	/// Best-effort: emit `SubcallFailed`, keep going.
	Some,
	/// Stop at first failure but do not revert the outer call.
	SomeUntilFailure,
	/// Atomic: any sub-call failure reverts the outer call with the
	/// sub-call's revert data bubbled verbatim.
	All,
}

pub type GetMaxBatchSize = ConstU32<{ MAX_BATCH_SIZE }>;
pub type GetCallDataLimit = ConstU32<{ CALL_DATA_LIMIT }>;

/// Loop over the four input arrays, dispatch each sub-call, apply mode
/// semantics. See spec section "Execution Flow" for the contract.
pub fn dispatch(
	_handle: &mut impl PrecompileHandle,
	_mode: BatchMode,
	_to: BoundedVec<Address, GetMaxBatchSize>,
	_value: BoundedVec<U256, GetMaxBatchSize>,
	_call_data: BoundedVec<BoundedBytes<GetCallDataLimit>, GetMaxBatchSize>,
	_gas_limit: BoundedVec<u64, GetMaxBatchSize>,
) -> EvmResult {
	Err(revert("dispatch: not yet implemented"))
}
```

- [ ] **Step 2: Wire `mode` into `lib.rs`**

Add to the top of `apps/node/precompiles/batch/src/lib.rs` (above the topic_tests module):

```rust
pub mod mode;
```

- [ ] **Step 3: Verify it compiles**

```bash
cd apps/node && SKIP_WASM_BUILD=1 cargo check -p precompile-batch
```

Expected: clean. The dispatch function is a stub; entry points come next.

- [ ] **Step 4: Commit**

```bash
cd /Users/huyduan/projects/blockchain
git add apps/node/precompiles/batch/
git commit -m "feat(node): add BatchMode enum and dispatch skeleton"
```

---

## Task 4: Mock runtime for unit tests

**Files:**
- Create: `apps/node/precompiles/batch/src/mock.rs`

- [ ] **Step 1: Create `apps/node/precompiles/batch/src/mock.rs`**

```rust
#![cfg(test)]

use frame_support::{
	construct_runtime, parameter_types,
	traits::{ConstU32, ConstU64, FindAuthor},
	weights::Weight,
};
use pallet_evm::{EnsureAddressNever, EnsureAddressRoot, IdentityAddressMapping};
use sp_core::{H160, U256};
use sp_runtime::{
	traits::{BlakeTwo256, IdentityLookup},
	BuildStorage, ConsensusEngineId,
};

use crate::BatchPrecompileSet;

pub type Block = frame_system::mocking::MockBlock<Runtime>;
pub type AccountId = H160;
pub type Balance = u128;

construct_runtime!(
	pub enum Runtime {
		System:    frame_system,
		Balances:  pallet_balances,
		Timestamp: pallet_timestamp,
		EVM:       pallet_evm,
	}
);

parameter_types! {
	pub const BlockHashCount: u64 = 250;
}

impl frame_system::Config for Runtime {
	type AccountId = AccountId;
	type Block = Block;
	type BlockHashCount = BlockHashCount;
	type BlockWeights = ();
	type BlockLength = ();
	type RuntimeOrigin = RuntimeOrigin;
	type RuntimeCall = RuntimeCall;
	type RuntimeTask = ();
	type Nonce = u64;
	type Hash = sp_core::H256;
	type Hashing = BlakeTwo256;
	type Lookup = IdentityLookup<Self::AccountId>;
	type RuntimeEvent = RuntimeEvent;
	type DbWeight = ();
	type Version = ();
	type PalletInfo = PalletInfo;
	type AccountData = pallet_balances::AccountData<Balance>;
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type SystemWeightInfo = ();
	type SS58Prefix = ();
	type OnSetCode = ();
	type MaxConsumers = ConstU32<16>;
	type MultiBlockMigrator = ();
	type SingleBlockMigrations = ();
	type PreInherents = ();
	type PostInherents = ();
	type PostTransactions = ();
	type ExtensionsWeightInfo = ();
}

impl pallet_balances::Config for Runtime {
	type Balance = Balance;
	type DustRemoval = ();
	type RuntimeEvent = RuntimeEvent;
	type ExistentialDeposit = ConstU128<0>;
	type AccountStore = System;
	type WeightInfo = ();
	type MaxLocks = ();
	type MaxReserves = ();
	type ReserveIdentifier = [u8; 8];
	type FreezeIdentifier = ();
	type MaxFreezes = ();
	type RuntimeHoldReason = ();
	type RuntimeFreezeReason = ();
	type DoneSlashHandler = ();
}

parameter_types! {
	pub const MinimumPeriod: u64 = 1;
}

impl pallet_timestamp::Config for Runtime {
	type Moment = u64;
	type OnTimestampSet = ();
	type MinimumPeriod = MinimumPeriod;
	type WeightInfo = ();
}

pub struct FindAuthorAlice;
impl FindAuthor<H160> for FindAuthorAlice {
	fn find_author<'a, I>(_digests: I) -> Option<H160>
	where I: 'a + IntoIterator<Item = (ConsensusEngineId, &'a [u8])> { None }
}

parameter_types! {
	pub BlockGasLimit: U256 = U256::from(75_000_000u64);
	pub WeightPerGas: Weight = Weight::from_parts(20_000, 0);
	pub GasLimitPovSizeRatio: u64 = 0;
	pub GasLimitStorageGrowthRatio: u64 = 0;
	pub SuicideQuickClearLimit: u32 = 0;
	pub PrecompilesValue: BatchPrecompileSet = BatchPrecompileSet::new();
}

use frame_support::traits::ConstU128;

impl pallet_evm::Config for Runtime {
	type FeeCalculator = ();
	type GasWeightMapping = pallet_evm::FixedGasWeightMapping<Self>;
	type WeightPerGas = WeightPerGas;
	type BlockHashMapping = pallet_evm::SubstrateBlockHashMapping<Self>;
	type CallOrigin = EnsureAddressRoot<AccountId>;
	type WithdrawOrigin = EnsureAddressNever<AccountId>;
	type AddressMapping = IdentityAddressMapping;
	type Currency = Balances;
	type RuntimeEvent = RuntimeEvent;
	type PrecompilesType = BatchPrecompileSet;
	type PrecompilesValue = PrecompilesValue;
	type ChainId = ConstU64<322>;
	type BlockGasLimit = BlockGasLimit;
	type Runner = pallet_evm::runner::stack::Runner<Self>;
	type OnChargeTransaction = ();
	type OnCreate = ();
	type FindAuthor = FindAuthorAlice;
	type GasLimitPovSizeRatio = GasLimitPovSizeRatio;
	type GasLimitStorageGrowthRatio = GasLimitStorageGrowthRatio;
	type Timestamp = Timestamp;
	type WeightInfo = ();
	type AccountProvider = pallet_evm::FrameSystemAccountProvider<Self>;
	type SuicideQuickClearLimit = SuicideQuickClearLimit;
	type CreateInnerOrigin = EnsureAddressNever<AccountId>;
}

pub fn new_test_ext() -> sp_io::TestExternalities {
	let t = frame_system::GenesisConfig::<Runtime>::default()
		.build_storage()
		.unwrap();
	let mut ext = sp_io::TestExternalities::new(t);
	ext.execute_with(|| System::set_block_number(1));
	ext
}
```

> Note: `BatchPrecompileSet` is added in Task 5. The mock will not compile until then — that's fine, mock is `#[cfg(test)]` only.

- [ ] **Step 2: Wire `mock` into `lib.rs`**

In `apps/node/precompiles/batch/src/lib.rs`, add (above the topic_tests module):

```rust
#[cfg(test)]
mod mock;
```

- [ ] **Step 3: Commit**

The crate won't compile fully yet because `BatchPrecompileSet` is undefined. Commit anyway so the mock change is isolated:

```bash
cd /Users/huyduan/projects/blockchain
git add apps/node/precompiles/batch/
git commit -m "feat(node): add mock runtime for batch precompile tests"
```

---

## Task 5: `BatchPrecompile` entry points (3 modes, non-payable, bounded codecs)

**Files:**
- Modify: `apps/node/precompiles/batch/src/lib.rs`

- [ ] **Step 1: Write the selector regression test (TDD)**

Add to `apps/node/precompiles/batch/src/lib.rs` (above topic_tests):

```rust
#[cfg(test)]
mod selector_tests {
	use precompile_utils::testing::solidity;

	#[test]
	fn selectors_match_canonical_signatures() {
		assert_eq!(
			solidity::compute_selector("batchSome(address[],uint256[],bytes[],uint64[])"),
			0x79df4b9c
		);
		assert_eq!(
			solidity::compute_selector(
				"batchSomeUntilFailure(address[],uint256[],bytes[],uint64[])"
			),
			0xcf0491c7
		);
		assert_eq!(
			solidity::compute_selector("batchAll(address[],uint256[],bytes[],uint64[])"),
			0x96e292b8
		);
	}
}
```

- [ ] **Step 2: Run; expect compile error (`solidity::compute_selector` exists, but no `BatchPrecompile` yet)**

```bash
cd apps/node && SKIP_WASM_BUILD=1 cargo test -p precompile-batch --lib selector_tests
```

If the symbol path is different, search the `precompile-utils` source for `compute_selector` and fix the import. Expected outcome: test exists, will pass once Step 3 doesn't break anything.

- [ ] **Step 3: Implement `BatchPrecompile` entry points**

Add to `apps/node/precompiles/batch/src/lib.rs` (above `#[cfg(test)] mod mock`):

```rust
use core::marker::PhantomData;

use precompile_utils::prelude::*;
use sp_core::{H160, U256};

use crate::mode::{dispatch, BatchMode, GetCallDataLimit, GetMaxBatchSize};

pub struct BatchPrecompile<Runtime>(PhantomData<Runtime>);

#[precompile_utils_macro::precompile]
impl<Runtime> BatchPrecompile<Runtime>
where
	Runtime: pallet_evm::Config,
{
	#[precompile::public("batchSome(address[],uint256[],bytes[],uint64[])")]
	fn batch_some(
		handle: &mut impl PrecompileHandle,
		to:        BoundedVec<Address, GetMaxBatchSize>,
		value:     BoundedVec<U256,    GetMaxBatchSize>,
		call_data: BoundedVec<BoundedBytes<GetCallDataLimit>, GetMaxBatchSize>,
		gas_limit: BoundedVec<u64,     GetMaxBatchSize>,
	) -> EvmResult {
		dispatch(handle, BatchMode::Some, to, value, call_data, gas_limit)
	}

	#[precompile::public("batchSomeUntilFailure(address[],uint256[],bytes[],uint64[])")]
	fn batch_some_until_failure(
		handle: &mut impl PrecompileHandle,
		to:        BoundedVec<Address, GetMaxBatchSize>,
		value:     BoundedVec<U256,    GetMaxBatchSize>,
		call_data: BoundedVec<BoundedBytes<GetCallDataLimit>, GetMaxBatchSize>,
		gas_limit: BoundedVec<u64,     GetMaxBatchSize>,
	) -> EvmResult {
		dispatch(handle, BatchMode::SomeUntilFailure, to, value, call_data, gas_limit)
	}

	#[precompile::public("batchAll(address[],uint256[],bytes[],uint64[])")]
	fn batch_all(
		handle: &mut impl PrecompileHandle,
		to:        BoundedVec<Address, GetMaxBatchSize>,
		value:     BoundedVec<U256,    GetMaxBatchSize>,
		call_data: BoundedVec<BoundedBytes<GetCallDataLimit>, GetMaxBatchSize>,
		gas_limit: BoundedVec<u64,     GetMaxBatchSize>,
	) -> EvmResult {
		dispatch(handle, BatchMode::All, to, value, call_data, gas_limit)
	}
}

/// `PrecompileSet` adapter used by the mock runtime only. The production
/// `runtimes/common::FrontierPrecompiles` integration is added in Task 14.
#[cfg(test)]
pub struct BatchPrecompileSet(PhantomData<crate::mock::Runtime>);

#[cfg(test)]
impl BatchPrecompileSet {
	pub fn new() -> Self { Self(PhantomData) }
}

#[cfg(test)]
impl fp_evm::PrecompileSet for BatchPrecompileSet {
	fn execute(&self, handle: &mut impl fp_evm::PrecompileHandle) -> Option<fp_evm::PrecompileResult> {
		if handle.code_address() == sp_core::H160::from_low_u64_be(PRECOMPILE_ADDRESS) {
			let r: fp_evm::PrecompileResult = <BatchPrecompile<crate::mock::Runtime> as fp_evm::Precompile>::execute(handle);
			Some(r)
		} else {
			None
		}
	}
	fn is_precompile(&self, address: sp_core::H160, _gas: u64) -> fp_evm::IsPrecompileResult {
		fp_evm::IsPrecompileResult::Answer {
			is_precompile: address == sp_core::H160::from_low_u64_be(PRECOMPILE_ADDRESS),
			extra_cost: 0,
		}
	}
}
```

> The macro `#[precompile_utils_macro::precompile]` reads from `precompile_utils::prelude::*` — `Address`, `BoundedVec`, `BoundedBytes`, `EvmResult`, `PrecompileHandle`, `revert` should all resolve. If a name is missing, check `precompile-utils/src/prelude.rs` in the cargo cache.

- [ ] **Step 4: Run selector test**

```bash
cd apps/node && SKIP_WASM_BUILD=1 cargo test -p precompile-batch --lib selector_tests
```

Expected: PASS. If a selector mismatch appears, double-check the signature string for typos.

- [ ] **Step 5: Confirm the mock + macro compile together**

```bash
cd apps/node && SKIP_WASM_BUILD=1 cargo check -p precompile-batch --tests
```

Expected: clean. Calling `batch_*` would currently return the `"dispatch: not yet implemented"` revert — that's wired up in Task 6.

- [ ] **Step 6: Commit**

```bash
cd /Users/huyduan/projects/blockchain
git add apps/node/precompiles/batch/
git commit -m "feat(node): wire BatchPrecompile entry points with bounded codecs"
```

---

## Task 6: `batchAll` happy path

**Files:**
- Modify: `apps/node/precompiles/batch/src/mode.rs`
- Modify: `apps/node/precompiles/batch/src/lib.rs` (add unit test module)

- [ ] **Step 1: Write the failing test**

In `apps/node/precompiles/batch/src/lib.rs`, add (above topic_tests):

```rust
#[cfg(test)]
mod batch_all_tests {
	use crate::mock::{new_test_ext, AccountId, Runtime};
	use crate::{subcall_succeeded_topic, PRECOMPILE_ADDRESS};
	use precompile_utils::testing::*;
	use sp_core::{H160, H256, U256};

	fn batch_addr() -> H160 { H160::from_low_u64_be(PRECOMPILE_ADDRESS) }

	#[test]
	fn batch_all_three_successful_subcalls_emit_three_succeeded_events() {
		new_test_ext().execute_with(|| {
			let caller = H160::from_low_u64_be(0xAA);
			let t1 = H160::from_low_u64_be(0x01);
			let t2 = H160::from_low_u64_be(0x02);
			let t3 = H160::from_low_u64_be(0x03);

			// Empty calldata, no value, default gas budget.
			PrecompileTesterExt::new::<crate::BatchPrecompileSet>(
				crate::BatchPrecompileSet::new(),
			)
			.prepare_test(
				caller,
				batch_addr(),
				BatchPrecompileCall::batch_all {
					to:        vec![t1.into(), t2.into(), t3.into()].into(),
					value:     vec![U256::zero(); 3].into(),
					call_data: vec![vec![].into(); 3].into(),
					gas_limit: vec![100_000u64; 3].into(),
				},
			)
			.expect_log(log_subcall_succeeded(0))
			.expect_log(log_subcall_succeeded(1))
			.expect_log(log_subcall_succeeded(2))
			.execute_returns(());
		});
	}

	fn log_subcall_succeeded(i: u64) -> Log {
		// primitive-types in stable2603: to_big_endian() returns [u8; 32].
		let data = U256::from(i).to_big_endian().to_vec();
		Log {
			address: batch_addr(),
			topics: vec![H256(crate::SUBCALL_SUCCEEDED_TOPIC)],
			data,
		}
	}
}
```

> `BatchPrecompileCall::batch_all` and `PrecompileTesterExt` are generated by `#[precompile_utils_macro::precompile]` plus the `testing` feature. If exact names differ, check `precompile-utils/src/testing/handle.rs` and `solidity_call::generate_call_enum`.

- [ ] **Step 2: Run; expect failure**

```bash
cd apps/node && SKIP_WASM_BUILD=1 cargo test -p precompile-batch --lib batch_all_three_successful_subcalls_emit_three_succeeded_events
```

Expected: panic with `dispatch: not yet implemented`.

- [ ] **Step 3: Implement the happy-path dispatch in `mode.rs`**

Replace the body of `dispatch` in `apps/node/precompiles/batch/src/mode.rs`:

```rust
pub fn dispatch(
	handle: &mut impl PrecompileHandle,
	mode: BatchMode,
	to: BoundedVec<Address, GetMaxBatchSize>,
	value: BoundedVec<U256, GetMaxBatchSize>,
	call_data: BoundedVec<BoundedBytes<GetCallDataLimit>, GetMaxBatchSize>,
	gas_limit: BoundedVec<u64, GetMaxBatchSize>,
) -> EvmResult {
	// Reject DELEGATECALL / CALLCODE. Both keep `handle.context().caller`
	// pointing at the original EOA while running the precompile's code from
	// the delegatecaller's address; using that caller as `Transfer.source`
	// would let any contract reached via delegatecall drain native value
	// from the EOA without explicit msg.value authorization.
	if handle.code_address() != handle.context().address {
		return Err(revert("DELEGATECALL/CALLCODE forbidden"));
	}

	handle.record_cost(BASE_OVERHEAD)?;

	let to: Vec<Address> = to.into();
	let value: Vec<U256> = value.into();
	let call_data: Vec<BoundedBytes<GetCallDataLimit>> = call_data.into();
	let gas_limit: Vec<u64> = gas_limit.into();

	let n = to.len();
	if value.len() != n || call_data.len() != n || gas_limit.len() != n {
		return Err(revert("length mismatch"));
	}
	handle.record_cost((n as u64).saturating_mul(PER_SUBCALL_OVERHEAD))?;

	let precompile_h160 = H160::from_low_u64_be(PRECOMPILE_ADDRESS);
	let is_static = handle.is_static();
	// Caller-funded transfers (Moonbeam pattern): each sub-call's native-value
	// transfer is debited from the EOA / contract that invoked the precompile,
	// and `msg.sender` propagated into the sub-call is that same address. The
	// precompile itself never holds value.
	let outer_caller = handle.context().caller;

	for i in 0..n {
		let target: H160 = to[i].into();
		if target == precompile_h160 {
			return Err(revert("self-call forbidden"));
		}

		let remaining = handle.remaining_gas();
		let sub_gas = if gas_limit[i] == 0 {
			remaining
		} else {
			core::cmp::min(gas_limit[i], remaining)
		};

		let input: Vec<u8> = call_data[i].clone().into();

		let context = fp_evm::Context {
			address: target,
			caller: outer_caller,
			apparent_value: value[i],
		};

		let transfer = if value[i].is_zero() {
			None
		} else {
			Some(fp_evm::Transfer {
				source: outer_caller,
				target,
				value: value[i],
			})
		};

		let (reason, output) =
			handle.call(target, transfer, input, Some(sub_gas), is_static, &context);

		match (mode, &reason) {
			(_, ExitReason::Succeed(_)) => {
				emit_event(handle, subcall_succeeded_topic(), i as u64)?;
			}
			(BatchMode::All, ExitReason::Revert(_)) => {
				return Err(PrecompileFailure::Revert {
					exit_status: ExitRevert::Reverted,
					output,
				});
			}
			(BatchMode::All, _) => {
				return Err(revert(alloc::format!(
					"sub-call {} failed",
					i
				)));
			}
			(BatchMode::Some, _) => {
				emit_event(handle, subcall_failed_topic(), i as u64)?;
			}
			(BatchMode::SomeUntilFailure, _) => {
				emit_event(handle, subcall_failed_topic(), i as u64)?;
				break;
			}
		}
	}

	Ok(())
}

fn emit_event(
	handle: &mut impl PrecompileHandle,
	topic: sp_core::H256,
	index: u64,
) -> Result<(), PrecompileFailure> {
	// primitive-types in stable2603: to_big_endian() returns [u8; 32].
	let data = U256::from(index).to_big_endian().to_vec();
	handle
		.log(
			H160::from_low_u64_be(PRECOMPILE_ADDRESS),
			alloc::vec![topic],
			data,
		)
		.map_err(|e| PrecompileFailure::Error { exit_status: e })?;
	Ok(())
}
```

- [ ] **Step 4: Run the test**

```bash
cd apps/node && SKIP_WASM_BUILD=1 cargo test -p precompile-batch --lib batch_all_three_successful_subcalls_emit_three_succeeded_events
```

Expected: PASS. Mock EVM treats targets without deployed code as immediate success (returning empty output) so all three sub-calls succeed naturally.

- [ ] **Step 5: Commit**

```bash
cd /Users/huyduan/projects/blockchain
git add apps/node/precompiles/batch/
git commit -m "feat(node): implement batchAll happy-path dispatch"
```

---

## Task 7: `batchAll` revert bubbling

**Files:**
- Modify: `apps/node/precompiles/batch/src/mock.rs` (helper to deploy a reverting target)
- Modify: `apps/node/precompiles/batch/src/lib.rs` (new tests)

- [ ] **Step 1: Add a helper that deploys minimal EVM bytecode**

Append to `apps/node/precompiles/batch/src/mock.rs`:

```rust
/// Deploy raw bytecode at `addr` so the EVM treats it as a contract.
/// Used to plant always-revert and always-succeed targets in tests.
pub fn deploy_code(addr: H160, code: Vec<u8>) {
	pallet_evm::AccountCodes::<Runtime>::insert(addr, code);
}

/// Bytecode that always reverts with `Error(string)` carrying "boom".
/// 0x60 0x84 0x60 0x00 0x52 0x60 0x20 0x60 0x04 0xfd
/// Simpler: PUSH1 0x00 PUSH1 0x00 REVERT — empty revert data.
pub fn revert_bytecode() -> Vec<u8> {
	vec![0x60, 0x00, 0x60, 0x00, 0xfd]
}
```

- [ ] **Step 2: Write the failing test**

Append to the `batch_all_tests` module in `lib.rs`:

```rust
#[test]
fn batch_all_revert_in_middle_bubbles_outer_revert_and_emits_no_events() {
	new_test_ext().execute_with(|| {
		let caller = H160::from_low_u64_be(0xAA);
		let t1 = H160::from_low_u64_be(0x01);
		let t2 = H160::from_low_u64_be(0x02);
		let t3 = H160::from_low_u64_be(0x03);

		// Plant a contract at t2 that always reverts.
		crate::mock::deploy_code(t2, crate::mock::revert_bytecode());

		PrecompileTesterExt::new::<crate::BatchPrecompileSet>(
			crate::BatchPrecompileSet::new(),
		)
		.prepare_test(
			caller,
			batch_addr(),
			BatchPrecompileCall::batch_all {
				to:        vec![t1.into(), t2.into(), t3.into()].into(),
				value:     vec![U256::zero(); 3].into(),
				call_data: vec![vec![].into(); 3].into(),
				gas_limit: vec![100_000u64; 3].into(),
			},
		)
		.expect_no_logs()
		.execute_reverts(|output| output.is_empty());
	});
}
```

- [ ] **Step 3: Run; expect FAIL until step 4 verifies the bubble**

```bash
cd apps/node && SKIP_WASM_BUILD=1 cargo test -p precompile-batch --lib batch_all_revert_in_middle_bubbles_outer_revert_and_emits_no_events
```

Expected: PASS already, because Task 6 already implemented the `BatchMode::All + Revert` bubble. If it fails, the bubble logic in `mode.rs:dispatch` is wrong — fix that match arm.

- [ ] **Step 4: Commit**

```bash
cd /Users/huyduan/projects/blockchain
git add apps/node/precompiles/batch/
git commit -m "test(node): cover batchAll revert bubbling"
```

---

## Task 8: `batchSome` semantics

**Files:**
- Modify: `apps/node/precompiles/batch/src/lib.rs` (new tests)

- [ ] **Step 1: Write the failing test**

Add a new test module to `apps/node/precompiles/batch/src/lib.rs`:

```rust
#[cfg(test)]
mod batch_some_tests {
	use crate::batch_all_tests::*; // re-use helpers if you exposed them via `pub(super)`
	use crate::mock::{deploy_code, new_test_ext, revert_bytecode};
	use crate::PRECOMPILE_ADDRESS;
	use precompile_utils::testing::*;
	use sp_core::{H160, H256, U256};

	fn batch_addr() -> H160 { H160::from_low_u64_be(PRECOMPILE_ADDRESS) }

	#[test]
	fn batch_some_middle_failure_continues_and_emits_mixed_events() {
		new_test_ext().execute_with(|| {
			let caller = H160::from_low_u64_be(0xAA);
			let t1 = H160::from_low_u64_be(0x01);
			let t2 = H160::from_low_u64_be(0x02);
			let t3 = H160::from_low_u64_be(0x03);

			deploy_code(t2, revert_bytecode());

			PrecompileTesterExt::new::<crate::BatchPrecompileSet>(
				crate::BatchPrecompileSet::new(),
			)
			.prepare_test(
				caller,
				batch_addr(),
				BatchPrecompileCall::batch_some {
					to:        vec![t1.into(), t2.into(), t3.into()].into(),
					value:     vec![U256::zero(); 3].into(),
					call_data: vec![vec![].into(); 3].into(),
					gas_limit: vec![100_000u64; 3].into(),
				},
			)
			.expect_log(Log {
				address: batch_addr(),
				topics: vec![H256(crate::SUBCALL_SUCCEEDED_TOPIC)],
				data: u256_be(0),
			})
			.expect_log(Log {
				address: batch_addr(),
				topics: vec![H256(crate::SUBCALL_FAILED_TOPIC)],
				data: u256_be(1),
			})
			.expect_log(Log {
				address: batch_addr(),
				topics: vec![H256(crate::SUBCALL_SUCCEEDED_TOPIC)],
				data: u256_be(2),
			})
			.execute_returns(());
		});
	}

	fn u256_be(i: u64) -> Vec<u8> {
		// primitive-types in stable2603: to_big_endian() returns [u8; 32].
		U256::from(i).to_big_endian().to_vec()
	}
}
```

- [ ] **Step 2: Run; expect PASS (logic already there from Task 6)**

```bash
cd apps/node && SKIP_WASM_BUILD=1 cargo test -p precompile-batch --lib batch_some_middle_failure_continues_and_emits_mixed_events
```

Expected: PASS.

- [ ] **Step 3: Commit**

```bash
cd /Users/huyduan/projects/blockchain
git add apps/node/precompiles/batch/
git commit -m "test(node): cover batchSome best-effort semantics"
```

---

## Task 9: `batchSomeUntilFailure` semantics

**Files:**
- Modify: `apps/node/precompiles/batch/src/lib.rs` (new tests)

- [ ] **Step 1: Write the failing test**

Add to `apps/node/precompiles/batch/src/lib.rs`:

```rust
#[cfg(test)]
mod batch_some_until_failure_tests {
	use crate::mock::{deploy_code, new_test_ext, revert_bytecode};
	use crate::PRECOMPILE_ADDRESS;
	use precompile_utils::testing::*;
	use sp_core::{H160, H256, U256};

	fn batch_addr() -> H160 { H160::from_low_u64_be(PRECOMPILE_ADDRESS) }

	#[test]
	fn batch_some_until_failure_stops_after_first_revert() {
		new_test_ext().execute_with(|| {
			let caller = H160::from_low_u64_be(0xAA);
			let t1 = H160::from_low_u64_be(0x01);
			let t2 = H160::from_low_u64_be(0x02);
			let t3 = H160::from_low_u64_be(0x03);

			deploy_code(t2, revert_bytecode());
			// Track t3 execution: if hit, it would be a successful subcall.
			// We rely on the absence of a 3rd event to prove the loop stopped.

			let succeeded_data = U256::from(0u64).to_big_endian().to_vec();
			let failed_data    = U256::from(1u64).to_big_endian().to_vec();

			PrecompileTesterExt::new::<crate::BatchPrecompileSet>(
				crate::BatchPrecompileSet::new(),
			)
			.prepare_test(
				caller,
				batch_addr(),
				BatchPrecompileCall::batch_some_until_failure {
					to:        vec![t1.into(), t2.into(), t3.into()].into(),
					value:     vec![U256::zero(); 3].into(),
					call_data: vec![vec![].into(); 3].into(),
					gas_limit: vec![100_000u64; 3].into(),
				},
			)
			.expect_log(Log {
				address: batch_addr(),
				topics: vec![H256(crate::SUBCALL_SUCCEEDED_TOPIC)],
				data: succeeded_data.to_vec(),
			})
			.expect_log(Log {
				address: batch_addr(),
				topics: vec![H256(crate::SUBCALL_FAILED_TOPIC)],
				data: failed_data.to_vec(),
			})
			// No third event — break stopped the loop.
			.execute_returns(());
		});
	}
}
```

- [ ] **Step 2: Run; expect PASS**

```bash
cd apps/node && SKIP_WASM_BUILD=1 cargo test -p precompile-batch --lib batch_some_until_failure_stops_after_first_revert
```

- [ ] **Step 3: Commit**

```bash
cd /Users/huyduan/projects/blockchain
git add apps/node/precompiles/batch/
git commit -m "test(node): cover batchSomeUntilFailure break semantics"
```

---

## Task 10: Self-call guard

**Files:**
- Modify: `apps/node/precompiles/batch/src/lib.rs`

- [ ] **Step 1: Write the failing test**

```rust
#[cfg(test)]
mod self_call_tests {
	use crate::PRECOMPILE_ADDRESS;
	use crate::mock::new_test_ext;
	use precompile_utils::testing::*;
	use sp_core::{H160, U256};

	fn batch_addr() -> H160 { H160::from_low_u64_be(PRECOMPILE_ADDRESS) }

	#[test]
	fn batch_all_rejects_self_call() {
		new_test_ext().execute_with(|| {
			let caller = H160::from_low_u64_be(0xAA);
			PrecompileTesterExt::new::<crate::BatchPrecompileSet>(
				crate::BatchPrecompileSet::new(),
			)
			.prepare_test(
				caller,
				batch_addr(),
				BatchPrecompileCall::batch_all {
					to:        vec![batch_addr().into()].into(),
					value:     vec![U256::zero()].into(),
					call_data: vec![vec![].into()].into(),
					gas_limit: vec![100_000u64].into(),
				},
			)
			.execute_reverts(|out| String::from_utf8_lossy(out).contains("self-call forbidden"));
		});
	}

	#[test]
	fn batch_some_rejects_self_call() {
		new_test_ext().execute_with(|| {
			let caller = H160::from_low_u64_be(0xAA);
			PrecompileTesterExt::new::<crate::BatchPrecompileSet>(
				crate::BatchPrecompileSet::new(),
			)
			.prepare_test(
				caller,
				batch_addr(),
				BatchPrecompileCall::batch_some {
					to:        vec![batch_addr().into()].into(),
					value:     vec![U256::zero()].into(),
					call_data: vec![vec![].into()].into(),
					gas_limit: vec![100_000u64].into(),
				},
			)
			.execute_reverts(|out| String::from_utf8_lossy(out).contains("self-call forbidden"));
		});
	}
}
```

- [ ] **Step 2: Run — should already PASS (Task 6 implemented the check)**

```bash
cd apps/node && SKIP_WASM_BUILD=1 cargo test -p precompile-batch --lib self_call_tests
```

- [ ] **Step 3: Commit**

```bash
cd /Users/huyduan/projects/blockchain
git add apps/node/precompiles/batch/
git commit -m "test(node): cover self-call rejection in all modes"
```

---

## Task 11: Value forwarding

**Files:**
- Modify: `apps/node/precompiles/batch/src/lib.rs`

- [ ] **Step 1: Write the failing test**

```rust
#[cfg(test)]
mod value_forwarding_tests {
	use crate::PRECOMPILE_ADDRESS;
	use crate::mock::{new_test_ext, AccountId};
	use frame_support::traits::Currency;
	use precompile_utils::testing::*;
	use sp_core::{H160, U256};

	fn batch_addr() -> H160 { H160::from_low_u64_be(PRECOMPILE_ADDRESS) }

	#[test]
	fn batch_all_forwards_value_to_each_target_and_leaves_excess_stuck() {
		new_test_ext().execute_with(|| {
			let caller = H160::from_low_u64_be(0xAA);
			let t1 = H160::from_low_u64_be(0x01);
			let t2 = H160::from_low_u64_be(0x02);

			// Credit precompile with caller's msg.value = 10 (mocked direct credit).
			crate::mock::Balances::deposit_creating(&batch_addr(), 10);

			PrecompileTesterExt::new::<crate::BatchPrecompileSet>(
				crate::BatchPrecompileSet::new(),
			)
			.prepare_test(
				caller,
				batch_addr(),
				BatchPrecompileCall::batch_all {
					to:        vec![t1.into(), t2.into()].into(),
					value:     vec![U256::from(3u64), U256::from(5u64)].into(),
					call_data: vec![vec![].into(); 2].into(),
					gas_limit: vec![100_000u64; 2].into(),
				},
			)
			.execute_returns(());

			assert_eq!(crate::mock::Balances::free_balance(&t1), 3);
			assert_eq!(crate::mock::Balances::free_balance(&t2), 5);
			assert_eq!(crate::mock::Balances::free_balance(&batch_addr()), 2); // 10 - 3 - 5
		});
	}
}
```

- [ ] **Step 2: Run**

```bash
cd apps/node && SKIP_WASM_BUILD=1 cargo test -p precompile-batch --lib value_forwarding_tests
```

Expected: PASS. Task 6 already wires `transfer` when `value[i] > 0`.

- [ ] **Step 3: Commit**

```bash
cd /Users/huyduan/projects/blockchain
git add apps/node/precompiles/batch/
git commit -m "test(node): cover native value forwarding through batch"
```

---

## Task 12: Gas-limit cap and forward-all behavior

**Files:**
- Modify: `apps/node/precompiles/batch/src/lib.rs`

- [ ] **Step 1: Write the failing test**

```rust
#[cfg(test)]
mod gas_limit_tests {
	use crate::PRECOMPILE_ADDRESS;
	use crate::mock::{deploy_code, new_test_ext, revert_bytecode};
	use precompile_utils::testing::*;
	use sp_core::{H160, U256};

	fn batch_addr() -> H160 { H160::from_low_u64_be(PRECOMPILE_ADDRESS) }

	#[test]
	fn batch_all_forwards_all_gas_when_limit_is_zero() {
		new_test_ext().execute_with(|| {
			let caller = H160::from_low_u64_be(0xAA);
			let t1 = H160::from_low_u64_be(0x01);

			PrecompileTesterExt::new::<crate::BatchPrecompileSet>(
				crate::BatchPrecompileSet::new(),
			)
			.with_target_gas(Some(500_000u64))
			.prepare_test(
				caller,
				batch_addr(),
				BatchPrecompileCall::batch_all {
					to:        vec![t1.into()].into(),
					value:     vec![U256::zero()].into(),
					call_data: vec![vec![].into()].into(),
					gas_limit: vec![0u64].into(), // 0 = forward all
				},
			)
			.execute_returns(());
		});
	}

	#[test]
	fn batch_all_caps_subcall_gas_to_remaining() {
		new_test_ext().execute_with(|| {
			let caller = H160::from_low_u64_be(0xAA);
			let t1 = H160::from_low_u64_be(0x01);

			PrecompileTesterExt::new::<crate::BatchPrecompileSet>(
				crate::BatchPrecompileSet::new(),
			)
			.with_target_gas(Some(50_000u64))
			.prepare_test(
				caller,
				batch_addr(),
				BatchPrecompileCall::batch_all {
					to:        vec![t1.into()].into(),
					value:     vec![U256::zero()].into(),
					call_data: vec![vec![].into()].into(),
					gas_limit: vec![10_000_000u64].into(), // > remaining → cap
				},
			)
			.execute_returns(());
		});
	}
}
```

- [ ] **Step 2: Run**

```bash
cd apps/node && SKIP_WASM_BUILD=1 cargo test -p precompile-batch --lib gas_limit_tests
```

Expected: PASS. Already implemented (`if gas_limit[i] == 0 { remaining } else { min(...) }`).

- [ ] **Step 3: Commit**

```bash
cd /Users/huyduan/projects/blockchain
git add apps/node/precompiles/batch/
git commit -m "test(node): cover gas-limit forward-all and cap behavior"
```

---

## Task 13: Static-call propagation

**Files:**
- Modify: `apps/node/precompiles/batch/src/lib.rs`

- [ ] **Step 1: Write the failing test**

Bytecode for "SSTORE 1 to slot 0" (3 PUSH ops + SSTORE + STOP): `0x60 0x01 0x60 0x00 0x55 0x00`.

Add to `mock.rs`:

```rust
pub fn sstore_bytecode() -> Vec<u8> {
	vec![0x60, 0x01, 0x60, 0x00, 0x55, 0x00]
}
```

Add to `lib.rs`:

```rust
#[cfg(test)]
mod static_call_tests {
	use crate::PRECOMPILE_ADDRESS;
	use crate::mock::{deploy_code, new_test_ext, sstore_bytecode};
	use precompile_utils::testing::*;
	use sp_core::{H160, U256};

	fn batch_addr() -> H160 { H160::from_low_u64_be(PRECOMPILE_ADDRESS) }

	#[test]
	fn batch_all_under_staticcall_reverts_when_subcall_mutates() {
		new_test_ext().execute_with(|| {
			let caller = H160::from_low_u64_be(0xAA);
			let t1 = H160::from_low_u64_be(0x01);

			deploy_code(t1, sstore_bytecode());

			PrecompileTesterExt::new::<crate::BatchPrecompileSet>(
				crate::BatchPrecompileSet::new(),
			)
			.with_static_call(true) // outer call is STATICCALL
			.prepare_test(
				caller,
				batch_addr(),
				BatchPrecompileCall::batch_all {
					to:        vec![t1.into()].into(),
					value:     vec![U256::zero()].into(),
					call_data: vec![vec![].into()].into(),
					gas_limit: vec![100_000u64].into(),
				},
			)
			.execute_reverts(|_| true);
		});
	}
}
```

> If `with_static_call` is not a method on `PrecompileTesterExt`, search `precompile-utils/src/testing/handle.rs` for the equivalent (likely `is_static_call(true)` or a builder field). Adjust accordingly.

- [ ] **Step 2: Run**

```bash
cd apps/node && SKIP_WASM_BUILD=1 cargo test -p precompile-batch --lib static_call_tests
```

Expected: PASS (the dispatch passes `handle.is_static()` to the sub-call; the SSTORE under STATICCALL reverts; `batchAll` bubbles the revert).

- [ ] **Step 3: Commit**

```bash
cd /Users/huyduan/projects/blockchain
git add apps/node/precompiles/batch/
git commit -m "test(node): cover static-call propagation in batch dispatch"
```

---

## Task 14: Codec-enforced bounds (oversized batch + oversized callData)

**Files:**
- Modify: `apps/node/precompiles/batch/src/lib.rs`

- [ ] **Step 1: Write the failing tests**

```rust
#[cfg(test)]
mod codec_bound_tests {
	use crate::{CALL_DATA_LIMIT, MAX_BATCH_SIZE, PRECOMPILE_ADDRESS};
	use crate::mock::new_test_ext;
	use precompile_utils::testing::*;
	use sp_core::{H160, U256};

	fn batch_addr() -> H160 { H160::from_low_u64_be(PRECOMPILE_ADDRESS) }

	#[test]
	fn rejects_batch_with_more_than_max_subcalls_via_codec() {
		new_test_ext().execute_with(|| {
			let caller = H160::from_low_u64_be(0xAA);
			let n = (MAX_BATCH_SIZE as usize) + 1;
			let target: Address = H160::from_low_u64_be(0x01).into();

			PrecompileTesterExt::new::<crate::BatchPrecompileSet>(
				crate::BatchPrecompileSet::new(),
			)
			.prepare_test(
				caller,
				batch_addr(),
				BatchPrecompileCall::batch_all {
					to:        vec![target; n].into(),
					value:     vec![U256::zero(); n].into(),
					call_data: vec![vec![].into(); n].into(),
					gas_limit: vec![100_000u64; n].into(),
				},
			)
			.execute_reverts(|out| String::from_utf8_lossy(out).contains("length"));
		});
	}

	#[test]
	fn rejects_oversized_call_data_via_codec() {
		new_test_ext().execute_with(|| {
			let caller = H160::from_low_u64_be(0xAA);
			let target: Address = H160::from_low_u64_be(0x01).into();
			let too_big = vec![0u8; (CALL_DATA_LIMIT as usize) + 1];

			PrecompileTesterExt::new::<crate::BatchPrecompileSet>(
				crate::BatchPrecompileSet::new(),
			)
			.prepare_test(
				caller,
				batch_addr(),
				BatchPrecompileCall::batch_all {
					to:        vec![target].into(),
					value:     vec![U256::zero()].into(),
					call_data: vec![too_big.into()].into(),
					gas_limit: vec![100_000u64].into(),
				},
			)
			.execute_reverts(|out| String::from_utf8_lossy(out).contains("length"));
		});
	}
}
```

- [ ] **Step 2: Run**

```bash
cd apps/node && SKIP_WASM_BUILD=1 cargo test -p precompile-batch --lib codec_bound_tests
```

Expected: PASS. `BoundedVec` / `BoundedBytes` codec rejects oversized inputs during ABI decode.

- [ ] **Step 3: Commit**

```bash
cd /Users/huyduan/projects/blockchain
git add apps/node/precompiles/batch/
git commit -m "test(node): cover codec-enforced batch-size and call-data bounds"
```

---

## Task 15: Length-mismatch + empty-batch + non-payable regression

**Files:**
- Modify: `apps/node/precompiles/batch/src/lib.rs`

- [ ] **Step 1: Write the tests**

```rust
#[cfg(test)]
mod misc_validation_tests {
	use crate::PRECOMPILE_ADDRESS;
	use crate::mock::new_test_ext;
	use precompile_utils::testing::*;
	use sp_core::{H160, U256};

	fn batch_addr() -> H160 { H160::from_low_u64_be(PRECOMPILE_ADDRESS) }

	#[test]
	fn rejects_length_mismatch() {
		new_test_ext().execute_with(|| {
			let caller = H160::from_low_u64_be(0xAA);
			let t1 = H160::from_low_u64_be(0x01);

			PrecompileTesterExt::new::<crate::BatchPrecompileSet>(
				crate::BatchPrecompileSet::new(),
			)
			.prepare_test(
				caller,
				batch_addr(),
				BatchPrecompileCall::batch_all {
					to:        vec![t1.into(), t1.into()].into(),
					value:     vec![U256::zero()].into(), // length 1 vs to length 2
					call_data: vec![vec![].into(); 2].into(),
					gas_limit: vec![100_000u64; 2].into(),
				},
			)
			.execute_reverts(|out| String::from_utf8_lossy(out).contains("length mismatch"));
		});
	}

	#[test]
	fn empty_batch_succeeds_with_no_events() {
		new_test_ext().execute_with(|| {
			let caller = H160::from_low_u64_be(0xAA);

			PrecompileTesterExt::new::<crate::BatchPrecompileSet>(
				crate::BatchPrecompileSet::new(),
			)
			.prepare_test(
				caller,
				batch_addr(),
				BatchPrecompileCall::batch_all {
					to:        vec![].into(),
					value:     vec![].into(),
					call_data: vec![].into(),
					gas_limit: vec![].into(),
				},
			)
			.expect_no_logs()
			.execute_returns(());
		});
	}
}
```

- [ ] **Step 2: Add a non-payable modifier regression via `precompile-utils::testing::PrecompilesModifierTester`**

If `precompile-utils` provides `PrecompilesModifierTester` (it does — see `precompiles/src/testing/modifier.rs`), assert all three entries reject non-zero `msg.value` (default modifier = non-payable, non-view):

```rust
#[cfg(test)]
mod modifier_tests {
	use crate::PRECOMPILE_ADDRESS;
	use crate::mock::new_test_ext;
	use precompile_utils::testing::*;
	use sp_core::H160;

	#[test]
	fn all_three_entries_reject_msg_value() {
		new_test_ext().execute_with(|| {
			let mut tester = PrecompilesModifierTester::new(
				crate::BatchPrecompileSet::new(),
				H160::from_low_u64_be(0xAA),
				H160::from_low_u64_be(PRECOMPILE_ADDRESS),
			);
			// `test_default_modifier` asserts that the selectors revert when
			// `msg.value > 0` (i.e. they are not annotated `#[precompile::payable]`).
			tester.test_default_modifier(&[0x79df4b9c, 0xcf0491c7, 0x96e292b8]);
		});
	}
}
```

- [ ] **Step 3: Run**

```bash
cd apps/node && SKIP_WASM_BUILD=1 cargo test -p precompile-batch --lib misc_validation_tests modifier_tests
```

Expected: all PASS.

- [ ] **Step 4: Commit**

```bash
cd /Users/huyduan/projects/blockchain
git add apps/node/precompiles/batch/
git commit -m "test(node): cover length mismatch, empty batch, non-payable modifier"
```

---

## Task 16: Wire `BatchPrecompile` into `FrontierPrecompiles`

**Files:**
- Modify: `apps/node/runtimes/common/Cargo.toml`
- Modify: `apps/node/runtimes/common/src/precompiles.rs`

- [ ] **Step 1: Add the dep to `runtimes/common/Cargo.toml`**

After line `precompile-gasless-registry = { workspace = true }` (around line 42), insert:

```toml
precompile-batch = { workspace = true }
```

In the `[features]` `std` array (around line 80), after `"precompile-gasless-registry/std",` insert:

```toml
	"precompile-batch/std",
```

- [ ] **Step 2: Modify `runtimes/common/src/precompiles.rs`**

Add the import after `use precompile_gasless_registry::GaslessRegistryPrecompile;`:

```rust
use precompile_batch::BatchPrecompile;
```

Update `used_addresses()` (currently 10 entries) to 11:

```rust
pub fn used_addresses() -> [H160; 11] {
	[
		hash(1), hash(2), hash(3), hash(4), hash(5),
		hash(1024), hash(1025), hash(1026), hash(1027),
		hash(precompile_gasless_registry::PRECOMPILE_ADDRESS),
		hash(precompile_batch::PRECOMPILE_ADDRESS),
	]
}
```

In `execute`, add a new arm just before the wildcard `_ => None`:

```rust
		a if a == hash(precompile_batch::PRECOMPILE_ADDRESS) => {
			Some(BatchPrecompile::<R>::execute(handle))
		}
```

- [ ] **Step 3: Verify both runtimes compile**

```bash
cd apps/node && SKIP_WASM_BUILD=1 cargo check -p impetus-runtime -p impulse-runtime
```

Expected: clean.

- [ ] **Step 4: Commit**

```bash
cd /Users/huyduan/projects/blockchain
git add apps/node/runtimes/common/Cargo.toml apps/node/runtimes/common/src/precompiles.rs
git commit -m "feat(node): register batch precompile at 0x0808 in FrontierPrecompiles"
```

---

## Task 17: Full build verification + lint + coverage

**Files:** (none — verification only)

- [ ] **Step 1: Full release build**

```bash
cd apps/node && cargo build --release
```

Expected: clean. Both runtimes build to WASM. Binary at `target/release/frontier-template-node`.

- [ ] **Step 2: Run all unit tests**

```bash
cd apps/node && SKIP_WASM_BUILD=1 cargo test -p precompile-batch
```

Expected: every test passes. Confirm no `ignored` or `failed` lines.

- [ ] **Step 3: Clippy with warnings as errors**

```bash
cd apps/node && SKIP_WASM_BUILD=1 cargo clippy -p precompile-batch -- -D warnings
cd apps/node && SKIP_WASM_BUILD=1 cargo clippy -p runtime-common -- -D warnings
```

Expected: clean.

- [ ] **Step 4: Coverage gate**

```bash
cd apps/node && SKIP_WASM_BUILD=1 cargo llvm-cov -p precompile-batch --fail-under-lines 80
```

If `cargo-llvm-cov` is missing, install with `cargo install cargo-llvm-cov`. Expected: ≥ 80 % line coverage.

- [ ] **Step 5: Format**

```bash
cd apps/node && cargo fmt
```

Commit any formatting changes:

```bash
cd /Users/huyduan/projects/blockchain
git add -u apps/node
git diff --cached --stat
# if non-empty:
git commit -m "style(node): cargo fmt after batch precompile work"
```

---

## Task 18: E2E fixture — `Echo.sol`

**Files:**
- Create: `packages/contracts/contracts/Echo.sol`

- [ ] **Step 1: Create `packages/contracts/contracts/Echo.sol`**

```solidity
// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.20;

contract Echo {
    uint256 public lastValue;
    address public lastSender;

    event Stored(uint256 value, address from);
    error Boom(uint256 reason);

    /// Reverts iff `x == 0`. Otherwise stores `x` and records `msg.sender`.
    function succeed(uint256 x) external {
        require(x != 0, "Echo: zero");
        lastValue = x;
        lastSender = msg.sender;
        emit Stored(x, msg.sender);
    }

    /// Reverts unconditionally with custom error `Boom(reason)`.
    function fail(uint256 reason) external pure {
        revert Boom(reason);
    }

    /// Plain native-value sink so batch transfers land.
    receive() external payable {}
}
```

- [ ] **Step 2: Compile**

```bash
cd packages/contracts && pnpm hardhat compile
```

Expected: artifact at `artifacts/contracts/Echo.sol/Echo.json`.

- [ ] **Step 3: Commit**

```bash
cd /Users/huyduan/projects/blockchain
git add packages/contracts/contracts/Echo.sol
git commit -m "test(contracts): add Echo fixture for batch precompile E2E"
```

---

## Task 19: E2E — `batchAll` happy and revert paths

**Files:**
- Create: `packages/contracts/test/batch.spec.ts`

- [ ] **Step 1: Start a fresh dev node**

In one terminal, from repo root:

```bash
cd apps/node && cargo build --release
./target/release/frontier-template-node --chain dev --tmp --alice
```

Leave it running. (Memory: E2E requires a fresh dev node before every run.)

- [ ] **Step 2: Create `packages/contracts/test/batch.spec.ts`**

```ts
import { expect } from "chai";
import { ethers } from "hardhat";

const BATCH = "0x0000000000000000000000000000000000000808";

const BATCH_ABI = [
  "function batchSome(address[],uint256[],bytes[],uint64[]) external",
  "function batchSomeUntilFailure(address[],uint256[],bytes[],uint64[]) external",
  "function batchAll(address[],uint256[],bytes[],uint64[]) external",
  "event SubcallSucceeded(uint256 index)",
  "event SubcallFailed(uint256 index)",
];

describe("Batch precompile (0x0808)", () => {
  let owner: any;
  let echoA: any;
  let echoB: any;
  let echoC: any;
  let batch: any;

  before(async () => {
    [owner] = await ethers.getSigners();
    const Echo = await ethers.getContractFactory("Echo");
    echoA = await Echo.deploy();
    echoB = await Echo.deploy();
    echoC = await Echo.deploy();
    await Promise.all([echoA.waitForDeployment(), echoB.waitForDeployment(), echoC.waitForDeployment()]);
    batch = new ethers.Contract(BATCH, BATCH_ABI, owner);
  });

  it("batchAll: three succeeds → state of all three contracts updated", async () => {
    const iface = new ethers.Interface(["function succeed(uint256)"]);
    const data = (n: number) => iface.encodeFunctionData("succeed", [n]);

    const tx = await batch.batchAll(
      [await echoA.getAddress(), await echoB.getAddress(), await echoC.getAddress()],
      [0, 0, 0],
      [data(11), data(22), data(33)],
      [0, 0, 0],
    );
    const receipt = await tx.wait();
    expect(receipt.status).to.eq(1);

    expect(await echoA.lastValue()).to.eq(11n);
    expect(await echoB.lastValue()).to.eq(22n);
    expect(await echoC.lastValue()).to.eq(33n);
  });

  it("batchAll: middle revert → tx reverts with Echo's custom error and state unchanged", async () => {
    const ifaceOk = new ethers.Interface(["function succeed(uint256)"]);
    const ifaceFail = new ethers.Interface(["function fail(uint256)"]);

    const before = await echoC.lastValue();

    await expect(
      batch.batchAll(
        [await echoA.getAddress(), await echoB.getAddress(), await echoC.getAddress()],
        [0, 0, 0],
        [
          ifaceOk.encodeFunctionData("succeed", [99]),
          ifaceFail.encodeFunctionData("fail", [42]),
          ifaceOk.encodeFunctionData("succeed", [88]),
        ],
        [0, 0, 0],
      ),
    ).to.be.revertedWithCustomError(echoB, "Boom").withArgs(42n);

    // State must NOT have advanced for any contract.
    expect(await echoC.lastValue()).to.eq(before);
  });
});
```

- [ ] **Step 3: Run only the batchAll tests**

```bash
cd packages/contracts && pnpm test --grep "batchAll"
```

Expected: both tests pass. If `revertedWithCustomError` does not propagate, double-check Task 6's `BatchMode::All + Revert` arm: it must return `PrecompileFailure::Revert` with `output` = raw sub-call revert data.

- [ ] **Step 4: Commit**

```bash
cd /Users/huyduan/projects/blockchain
git add packages/contracts/test/batch.spec.ts
git commit -m "test(contracts): E2E batchAll happy + revert bubble"
```

---

## Task 20: E2E — `batchSome`, `batchSomeUntilFailure`, native value, self-call

**Files:**
- Modify: `packages/contracts/test/batch.spec.ts`

- [ ] **Step 1: Append the new cases**

Add inside the `describe("Batch precompile (0x0808)", ...)` block:

```ts
  it("batchSome: middle revert → outer succeeds; events show 0 succeed, 1 fail, 2 succeed", async () => {
    const ifaceOk = new ethers.Interface(["function succeed(uint256)"]);
    const ifaceFail = new ethers.Interface(["function fail(uint256)"]);

    const tx = await batch.batchSome(
      [await echoA.getAddress(), await echoB.getAddress(), await echoC.getAddress()],
      [0, 0, 0],
      [
        ifaceOk.encodeFunctionData("succeed", [1]),
        ifaceFail.encodeFunctionData("fail", [9]),
        ifaceOk.encodeFunctionData("succeed", [3]),
      ],
      [0, 0, 0],
    );
    const receipt = await tx.wait();
    expect(receipt.status).to.eq(1);

    const iface = new ethers.Interface(BATCH_ABI);
    const events = receipt.logs
      .filter((l: any) => l.address.toLowerCase() === BATCH.toLowerCase())
      .map((l: any) => iface.parseLog({ topics: l.topics, data: l.data }));

    expect(events.map((e: any) => `${e.name}(${e.args.index})`)).to.deep.eq([
      "SubcallSucceeded(0)",
      "SubcallFailed(1)",
      "SubcallSucceeded(2)",
    ]);

    expect(await echoA.lastValue()).to.eq(1n);
    expect(await echoC.lastValue()).to.eq(3n);
  });

  it("batchSomeUntilFailure: middle revert → outer succeeds; index 2 NOT executed", async () => {
    const ifaceOk = new ethers.Interface(["function succeed(uint256)"]);
    const ifaceFail = new ethers.Interface(["function fail(uint256)"]);

    const cBefore = await echoC.lastValue();

    const tx = await batch.batchSomeUntilFailure(
      [await echoA.getAddress(), await echoB.getAddress(), await echoC.getAddress()],
      [0, 0, 0],
      [
        ifaceOk.encodeFunctionData("succeed", [7]),
        ifaceFail.encodeFunctionData("fail", [0]),
        ifaceOk.encodeFunctionData("succeed", [999]),
      ],
      [0, 0, 0],
    );
    await tx.wait();

    expect(await echoA.lastValue()).to.eq(7n);
    expect(await echoC.lastValue()).to.eq(cBefore); // not advanced
  });

  it("batchAll: caller funds each sub-call transfer; precompile holds no value", async () => {
    const provider = ethers.provider;
    const a = await echoA.getAddress();
    const b = await echoB.getAddress();

    const aBefore = await provider.getBalance(a);
    const bBefore = await provider.getBalance(b);
    const pBefore = await provider.getBalance(BATCH);

    // Entries are non-payable — do NOT pass `value:` on the outer call.
    // Native value debits directly from the caller via the precompile's
    // `Transfer { source: outer_caller, ... }` per sub-call.
    const tx = await batch.batchAll(
      [a, b],
      [ethers.parseEther("1"), ethers.parseEther("2")],
      ["0x", "0x"],
      [0, 0],
    );
    await tx.wait();

    expect(await provider.getBalance(a)).to.eq(aBefore + ethers.parseEther("1"));
    expect(await provider.getBalance(b)).to.eq(bBefore + ethers.parseEther("2"));
    // Precompile is never the value source / sink.
    expect(await provider.getBalance(BATCH)).to.eq(pBefore);
  });

  it("batchAll: passing msg.value > 0 to non-payable entry reverts", async () => {
    const a = await echoA.getAddress();
    await expect(
      batch.batchAll([a], [0], ["0x"], [0], { value: 1n }),
    ).to.be.reverted;
  });

  it("self-call → tx reverts in every mode", async () => {
    for (const fn of ["batchSome", "batchSomeUntilFailure", "batchAll"] as const) {
      await expect(
        (batch as any)[fn]([BATCH], [0], ["0x"], [0]),
      ).to.be.reverted;
    }
  });
```

- [ ] **Step 2: Run the full E2E suite**

```bash
cd packages/contracts && pnpm test --grep "Batch precompile"
```

Expected: all seven tests pass against the dev node started in Task 19 step 1.

- [ ] **Step 3: Commit**

```bash
cd /Users/huyduan/projects/blockchain
git add packages/contracts/test/batch.spec.ts
git commit -m "test(contracts): E2E batchSome, batchSomeUntilFailure, value, self-call"
```

---

## Task 21: Acceptance criteria verification + final commit hygiene

**Files:** (none — verification)

Verify every line from the spec's "Acceptance Criteria":

- [ ] **Step 1:** `cd apps/node && cargo build --release` succeeds.
- [ ] **Step 2:** `cd apps/node && cargo test -p precompile-batch` all green.
- [ ] **Step 3:** `cd apps/node && cargo llvm-cov -p precompile-batch --fail-under-lines 80` passes.
- [ ] **Step 4:** `cd apps/node && cargo clippy --workspace -- -D warnings` passes.
- [ ] **Step 5:** Restart dev node, run `cd packages/contracts && pnpm test --grep "Batch precompile"` — all pass.
- [ ] **Step 6:** `FrontierPrecompiles::used_addresses()` contains `0x0808` — confirmed by the integration test in `runtimes/common/src/precompiles.rs` (existing `used_addresses()` is now 11 entries; assert in a runtime test if not already).
- [ ] **Step 7:** Verify selectors via `cast`:

```bash
cast sig 'batchSome(address[],uint256[],bytes[],uint64[])'             # 0x79df4b9c
cast sig 'batchSomeUntilFailure(address[],uint256[],bytes[],uint64[])' # 0xcf0491c7
cast sig 'batchAll(address[],uint256[],bytes[],uint64[])'              # 0x96e292b8
```

If any of the three mismatches the regression test in Task 5, update the hardcoded constant and re-run tests.

- [ ] **Step 8: Final commit, if any cleanup**

```bash
cd /Users/huyduan/projects/blockchain
git status
# If anything is dirty after verification:
git add -u
git commit -m "chore(node): finalize batch precompile acceptance pass"
```

---

## Self-Review Notes

- **Spec coverage check:** every requirement in spec section "Acceptance Criteria" has a verification step in Task 21. Every section in spec "Execution Flow", "Error Handling", "Testing Strategy" has at least one task implementing it (Tasks 6-15 cover unit, Task 16 covers integration, Tasks 18-20 cover E2E).
- **No placeholders:** every code step shows complete code. Search this document for `TODO`, `TBD`, `implement later` → none present.
- **Type consistency:** `BatchPrecompile<Runtime>` declared in Task 5 is used in `BatchPrecompileSet` (Task 5) and registered in `FrontierPrecompiles` (Task 16) — names align. `BatchMode { Some, SomeUntilFailure, All }` declared in Task 3 is matched on by `dispatch` in Task 6 — same variants.
- **One known soft spot:** `precompile-utils::testing` API shape (`PrecompileTesterExt::new`, `prepare_test`, `with_target_gas`, `with_static_call`) is inferred from existing Frontier test helpers. The implementer should consult `~/.cargo/git/checkouts/frontier-*/precompiles/src/testing/` for exact method names and adjust calls if the constructor signature differs. The test *intent* and assertions are correct regardless.
