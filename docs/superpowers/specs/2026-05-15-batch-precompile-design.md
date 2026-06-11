# Batch Precompile Design

**Date:** 2026-05-15
**Status:** Approved (pending implementation plan)
**Scope:** `apps/node`

## Summary

Add an EVM precompile at `0x0808` that lets a single transaction dispatch
multiple sub-calls with Moonbeam-compatible semantics. Three entry points
expose best-effort (`batchSome`), fail-fast-but-soft (`batchSomeUntilFailure`),
and atomic (`batchAll`) execution. All three entries are **non-payable**;
the precompile never holds value. Sub-calls execute with
`msg.sender = the immediate caller of the precompile` and `Transfer.source`
debited from that same caller (Moonbeam caller-funded pattern). The
precompile rejects `DELEGATECALL` / `CALLCODE` to prevent caller-spoofing.
Gasless registry integration is deferred to a follow-up plan.

## Goals

- One transaction, many EVM sub-calls, three well-defined failure modes.
- ABI-compatible with Moonbeam `Batch.sol` so existing tooling, wallets, and
  documentation transfer without translation.
- Safe by construction: bounded batch size, no recursion into the precompile,
  no caller spoofing.
- Observable: emit `SubcallSucceeded(uint256)` / `SubcallFailed(uint256)` per
  iteration so the Ponder indexer can reconstruct batch outcomes.

## Non-Goals

- Integration with `pallet-gasless-registry` / `GaslessEvmRunner` (deferred —
  separate brainstorming + spec).
- UI surfacing in `apps/ui`.
- Indexer schema migrations in `apps/indexer` (separate task once event topics
  are finalized in code).
- Support for `delegatecall` / `staticcall` modes (v1 only does `call`).

## Architecture

### Crate layout

```
apps/node/
├── Cargo.toml                              # + workspace member, + dep
├── precompiles/
│   └── batch/                              # NEW crate: precompile-batch
│       ├── Cargo.toml
│       ├── Batch.sol                       # public Solidity interface
│       └── src/
│           ├── lib.rs                      # BatchPrecompile<R>, 3 entries
│           ├── mode.rs                     # BatchMode enum + dispatch loop
│           └── mock.rs                     # #[cfg(test)] mock runtime
└── runtimes/common/
    └── src/precompiles.rs                  # + arm for 0x0808
```

### Integration boundary

- New crate `precompile-batch` exporting `BatchPrecompile<R>`.
- `runtimes/common/src/precompiles.rs`: append `hash(2056)` to
  `used_addresses()` and add a match arm in `execute()`.
- `runtimes/common/Cargo.toml`: add direct dep on `precompile-batch`
  (mirroring how `precompile-gasless-registry` is already wired here, since
  `FrontierPrecompiles` lives in this crate and must `use` the precompile
  type directly).
- `runtimes/impetus/Cargo.toml` and `runtimes/impulse/Cargo.toml`: no direct
  dep needed — they consume `FrontierPrecompiles` re-exported from
  `runtime-common`.
- Workspace `Cargo.toml` at `apps/node/`: add member `precompiles/batch` and a
  `precompile-batch = { path = "precompiles/batch", default-features = false }`
  entry.

No pallets are modified. `GaslessEvmRunner` and fee logic are untouched.

### Approach

Use the Frontier `precompile-utils` macros already pinned in the workspace
(`precompile-utils = { git = "https://github.com/polkadot-evm/frontier", branch = "stable2603" }`).
The macros handle ABI decode/encode for compound types and enforce
function-modifier semantics (payable / view / non-payable) before our logic
runs.

**Bounded codec types.** Use `BoundedVec<T, ConstU32<N>>` and
`BoundedBytes<ConstU32<N>>` rather than `Vec<T>` / `UnboundedBytes` in the
Rust signatures. Per `precompile-utils` source
(`solidity/codec/bytes.rs:95-108`, `solidity/codec/native.rs:283-315`), the
codec validates the declared length prefix against the bound **before** it
copies the payload. Using unbounded types would let an attacker send an ABI
header claiming a 1 GiB call-data blob, forcing memory allocation before
our `MAX_BATCH_SIZE` / `CALL_DATA_LIMIT` checks run.

**Non-payable entries (caller-funded sub-call transfers).** None of the three
entries are annotated `#[precompile::payable]`. The precompile never receives
`msg.value`; instead, each sub-call's native-value transfer is debited
directly from `handle.context().caller` via
`fp_evm::Transfer { source: handle.context().caller, target, value }`. This
mirrors the Moonbeam `Batch` precompile and avoids the dangerous pattern of
holding value at the precompile address between sub-calls (where a partial
batch could leave dust trapped at `0x0808`).

**DELEGATECALL / CALLCODE guard.** Because sub-call authorization derives from
`handle.context().caller`, a delegatecaller could otherwise spoof an EOA and
drain its native balance without `msg.value` consent. The dispatch entry
asserts `handle.code_address() == handle.context().address` and reverts
`"DELEGATECALL/CALLCODE forbidden"` otherwise.

Logic is re-implemented from the Moonbeam reference rather than vendored, to
avoid GPL-3.0 license clash and to keep the crate aligned with the existing
`precompile-gasless-registry` style.

### Rust signature shape

```rust
#[precompile_utils::precompile]
impl<R: ...> BatchPrecompile<R> {
    #[precompile::public("batchSome(address[],uint256[],bytes[],uint64[])")]
    fn batch_some(
        handle: &mut impl PrecompileHandle,
        to:        BoundedVec<Address, ConstU32<{ MAX_BATCH_SIZE }>>,
        value:     BoundedVec<U256,    ConstU32<{ MAX_BATCH_SIZE }>>,
        call_data: BoundedVec<BoundedBytes<ConstU32<{ CALL_DATA_LIMIT }>>,
                              ConstU32<{ MAX_BATCH_SIZE }>>,
        gas_limit: BoundedVec<u64,     ConstU32<{ MAX_BATCH_SIZE }>>,
    ) -> EvmResult { dispatch(handle, BatchMode::Some, to, value, call_data, gas_limit) }

    // batch_some_until_failure and batch_all follow the same signature shape.
}
```

## Solidity Interface

`precompiles/batch/Batch.sol`:

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
    function batchSome(
        address[] memory to,
        uint256[] memory value,
        bytes[]   memory callData,
        uint64[]  memory gasLimit
    ) external;

    function batchSomeUntilFailure(
        address[] memory to,
        uint256[] memory value,
        bytes[]   memory callData,
        uint64[]  memory gasLimit
    ) external;

    function batchAll(
        address[] memory to,
        uint256[] memory value,
        bytes[]   memory callData,
        uint64[]  memory gasLimit
    ) external;

    event SubcallSucceeded(uint256 index);
    event SubcallFailed(uint256 index);
}
```

### Selectors

Selectors match Moonbeam mainnet so existing tooling decodes the function:

| Function | Signature | Selector |
|---|---|---|
| `batchSome` | `batchSome(address[],uint256[],bytes[],uint64[])` | `0x79df4b9c` |
| `batchSomeUntilFailure` | `batchSomeUntilFailure(address[],uint256[],bytes[],uint64[])` | `0xcf0491c7` |
| `batchAll` | `batchAll(address[],uint256[],bytes[],uint64[])` | `0x96e292b8` |

> Selectors verified with `cast sig` against the canonical signatures.

A regression test computes selectors via
`precompile_utils::testing::solidity::compute_selector(...)` and asserts they
match the hardcoded constants. If a value diverges, the test value wins and
the constant is updated — macro-generated ABI is authoritative.

### Event topics and encoding

Both events declare `index` as a **non-indexed** `uint256` parameter
(matching Moonbeam's `Batch.sol`). Per Solidity ABI rules, only `indexed`
parameters go into log topics; non-indexed ones go into the data section.
Emission therefore uses:

```rust
// primitive-types in stable2603 returns [u8; 32] from to_big_endian().
handle.log(
    PRECOMPILE_ADDRESS,
    vec![SUBCALL_SUCCEEDED_TOPIC],                 // topics = [topic0]
    U256::from(i).to_big_endian().to_vec(),        // data   = 32-byte BE index
)
```

The dangerous refactor is the inverse: marking `index` as `indexed` on
**one** side without syncing the other. Two failure modes to guard against:

- If the Solidity declaration is changed to `uint256 indexed index` but Rust
  still emits `data = <32-byte index>` with no extra topic, then
  `ethers.Interface.parseLog` reads `topics[1]` (empty) → `args.index`
  becomes `undefined` (or the call throws on length mismatch).
- If Rust is changed to emit `topics = [topic0, index_bytes32], data = []`
  but the Solidity declaration stays non-indexed, `parseLog` reads `data`
  (empty) → `args.index` silently decodes to `0n` for every event.

Keep both sides synced: param stays non-indexed in `Batch.sol`, Rust emits
`topics = [topic0]` and `data = <32-byte BE index>`.

Topic constants are computed as `keccak256(signature)` and hardcoded as
32-byte values in the same style as
`precompiles/gasless-registry/src/lib.rs:23-28`. A unit test recomputes
them and asserts equality, so renaming the event surfaces as a test
failure rather than a silent ABI break.

## Execution Flow

### Constants

```rust
pub const PRECOMPILE_ADDRESS: u64 = 2056;          // 0x0808
const MAX_BATCH_SIZE: u32 = 256;
const PER_SUBCALL_OVERHEAD: u64 = 1_500;
const BASE_OVERHEAD: u64 = 1_000;
const CALL_DATA_LIMIT: u32 = 2 * 1024 * 1024;      // 2 MiB per sub-call
```

### Dispatch pseudocode (shared by all three modes)

```
fn dispatch(handle, mode, to[], value[], call_data[], gas_limit[]):
    # DELEGATECALL / CALLCODE guard. Without this, a delegatecaller would let
    # us spoof `handle.context().caller` (which is the original EOA), letting
    # any contract drain native value from that EOA via Transfer.source below.
    require(handle.code_address() == handle.context().address,
            revert "DELEGATECALL/CALLCODE forbidden")

    # Codec invariants (already enforced at ABI decode by BoundedVec/BoundedBytes):
    #   - to.len() <= MAX_BATCH_SIZE
    #   - value.len() <= MAX_BATCH_SIZE
    #   - call_data.len() <= MAX_BATCH_SIZE
    #   - gas_limit.len() <= MAX_BATCH_SIZE
    #   - every call_data[i].len() <= CALL_DATA_LIMIT
    # Codec reverts with "length too large" if any of the above is violated.

    handle.record_cost(BASE_OVERHEAD)?

    n = to.len()
    require(n == value.len() == call_data.len() == gas_limit.len(),
            revert "length mismatch")
    handle.record_cost(n * PER_SUBCALL_OVERHEAD)?

    # Snapshot caller once; reused for every sub-call's Transfer.source and
    # Context.caller. The precompile itself never holds value and never
    # appears as `msg.sender` to sub-targets.
    outer_caller = handle.context().caller

    for i in 0..n:
        target = to[i]
        require(target != PRECOMPILE_ADDRESS, revert "self-call forbidden")

        remaining = handle.remaining_gas()
        sub_gas   = if gas_limit[i] == 0 { remaining }
                    else { min(gas_limit[i], remaining) }

        (reason, output) = handle.call(
            target,
            # Caller-funded transfer (Moonbeam pattern). Sub-call native value
            # debits directly from the outer caller, not the precompile.
            if value[i] == 0 { None } else {
                Some(Transfer {
                    source: outer_caller,
                    target,
                    value:  value[i],
                })
            },
            call_data[i],
            Some(sub_gas),
            is_static = handle.is_static(),   # propagate outer STATICCALL
            context = Context {
                address:        target,
                caller:         outer_caller,
                apparent_value: value[i],
            },
        )

        match (mode, reason):
            # Fatal exits unwind the entire execution stack — never swallow,
            # regardless of mode.
            (_,             Fatal(s))   => return PrecompileFailure::Fatal { s }
            (_,             Succeed(_)) => emit SubcallSucceeded(i)
            (BatchAll,      Revert(_))  => return Revert { output }   # bubble raw
            (BatchAll,      _)          => return revert with diagnostic
            (BatchSome,     _)          => emit SubcallFailed(i); continue
            (BatchSomeUF,   _)          => emit SubcallFailed(i); break

    return Succeed::Returned { output: empty }
```

### Mode behavior matrix

| Mode | On `Succeed` | On `Revert` | On `Fatal` / `Error(OutOfGas)` etc. | Outer return |
|---|---|---|---|---|
| `batchSome` | emit `SubcallSucceeded(i)`, continue | emit `SubcallFailed(i)`, continue | propagate `PrecompileFailure::Fatal` for `Fatal`; otherwise emit `SubcallFailed(i)`, continue | `Succeed::Returned`, empty output (unless Fatal bubbled) |
| `batchSomeUntilFailure` | emit `SubcallSucceeded(i)`, continue | emit `SubcallFailed(i)`, **break** | propagate Fatal; otherwise emit `SubcallFailed(i)`, **break** | `Succeed::Returned`, empty output (unless Fatal bubbled) |
| `batchAll` | emit `SubcallSucceeded(i)`, continue | **revert outer** with sub-call's revert data verbatim | propagate Fatal; otherwise revert outer with diagnostic | `PrecompileFailure::Revert` (or `Fatal`) |

`ExitReason::Fatal` always unwinds the entire batch in every mode — the
precompile never silences it into a `SubcallFailed` event. This is
non-negotiable; a fatal exit means the EVM cannot reliably continue.

### Value forwarding (caller-funded)

The precompile is **non-payable** and never receives `msg.value`. Sending
`msg.value > 0` to any of the three entries reverts at the modifier check
(`"Function is not payable"`) before dispatch runs.

For each sub-call `i`, when `value[i] != 0`, the dispatch builds:

```
Transfer { source: handle.context().caller, target: to[i], value: value[i] }
```

and forwards it to `handle.call(...)`. The Frontier EVM runner debits
`value[i]` directly from the **caller's** balance and credits `to[i]` — the
precompile address (`0x0808`) is **never** the value source or sink and its
balance stays at `0`.

This mirrors Moonbeam's `Batch` precompile. If the caller cannot cover
`sum(value[i])`, the offending sub-call fails with `InsufficientBalance`
and the mode decides next steps (per the matrix above). There is no
"excess `msg.value`" concept and no precompile-pot stuck balance to refund.

`handle.context().caller` is the **immediate** caller of the precompile
(EOA when invoked directly, or the intermediate contract when invoked
through one). The DELEGATECALL/CALLCODE guard at the top of `dispatch`
prevents a delegatecaller from spoofing this address.

### Gas accounting

- `BASE_OVERHEAD = 1000` charged unconditionally at the top.
- `PER_SUBCALL_OVERHEAD = 1500` charged per `n` after length validation.
- Each `handle.call(...)` deducts its own sub-gas from the pool.
- `handle.log(...)` charges roughly 750 gas per event (topic0 only +
  one 32-byte data word, see event encoding below). Frontier's handle
  accounts for this automatically.
- `gas_limit[i] = 0` forwards `handle.remaining_gas()` at that point (i.e.
  after the overheads above).
- `gas_limit[i] > remaining_gas()` is silently capped — never overdraws.

### Re-entrancy and static calls

- Self-call check (`target == PRECOMPILE_ADDRESS`) blocks re-entrant batch.
- EVM call-stack depth (1024, Frontier default) protects against indirect
  recursion through other contracts.
- If the top-level call is `STATICCALL`, the precompile **must** propagate
  `handle.is_static()` into each `handle.call(...)`'s `is_static` argument.
  Frontier's `PrecompileHandle::call` uses that flag as the sub-call's static
  context, so a hardcoded `false` would let sub-targets mutate state under an
  outer static call — breaking EVM static guarantees. Reference:
  `evm::executor::stack::PrecompileHandle::call` and `::is_static`.

## Error Handling

| Situation | Where | Response |
|---|---|---|
| Input < 4 bytes | framework | `PrecompileFailure::Error("input too short")` |
| Unknown selector | macro fallthrough | `PrecompileFailure::Error("unknown selector")` |
| ABI decode failure | `precompile-utils` macro | `Revert` with generic decode error |
| Caller sends `msg.value > 0` to any batch entry | `precompile-utils` modifier check (entries are non-payable) | `revert("Function is not payable")` |
| Precompile invoked via `DELEGATECALL` / `CALLCODE` | dispatch entry | `revert("DELEGATECALL/CALLCODE forbidden")` (asserts `handle.code_address() == handle.context().address`) |
| `n > MAX_BATCH_SIZE` for any of the four arrays | `BoundedVec` codec | `revert("length too large")` at decode, before our logic runs |
| `call_data[i].len() > CALL_DATA_LIMIT` (2 MiB) | `BoundedBytes` codec | `revert("length too large")` at decode, before our logic runs |
| Array length mismatch (the four arrays disagree) | validate step | `revert("length mismatch")` |
| `to[i] == 0x0808` (self-call) | per-iter | `revert("self-call forbidden")` — **hard revert, all modes** |
| OutOfGas on overhead charge | step 1/2 | `PrecompileFailure::Error(OutOfGas)` |
| Sub-call `Revert(data)` | per-iter | mode decides; `batchAll` bubbles `data` verbatim |
| Sub-call `Error(OutOfGas)` | per-iter | treated as failure per mode; `batchAll` returns generic revert with diagnostic |
| Sub-call `ExitReason::Fatal` | per-iter | **unconditional** — return `PrecompileFailure::Fatal { exit_status }` in every mode (never silenced into an event) |
| Sub-call hits depth limit | EVM runner | sub-call reverts; mode decides |

### Self-call is an invariant, not a mode-dependent behavior

A self-call indicates malformed input from the caller, regardless of mode.
`batchSome` does not get to "skip" a self-call — the entire batch reverts so
the caller learns about the bug rather than burying it in an event.

### Revert data propagation for `batchAll`

When the failing sub-call's `ExitReason` is `Revert`, the precompile returns
`PrecompileFailure::Revert { output: raw_subcall_revert_data }`. Solidity
`try/catch` on the outer call sees the original revert payload verbatim
(custom error selector, `Error(string)`, whatever the sub-target emitted).

For sub-call `Error(OutOfGas)` (which carries no revert data of its own),
`batchAll` constructs a Solidity-standard `Error(string)` payload via
`revert(alloc::format!("sub-call {} failed", i))` so callers see a
human-readable reason rather than empty bytes.

For sub-call `ExitReason::Fatal`, the precompile **does not** wrap into an
`Error(string)` — it returns `PrecompileFailure::Fatal { exit_status }`
directly, regardless of mode. A `Fatal` exit signals that the EVM cannot
reliably continue (e.g. call stack overflow); the outer transaction should
unwind too, and the surrounding runtime distinguishes Fatal from Revert in
its result handling.

### No silent failure

Every branch either reverts or emits an event. Indexers can reconstruct
batch outcomes purely from logs:

- `batchSome`: outer succeeds; per-index `Succeeded`/`Failed` tells the
  story.
- `batchSomeUntilFailure`: outer succeeds; trailing absence of events past
  the failure index reveals where execution stopped.
- `batchAll`: outer reverts → no events emitted (state rollback).

## Testing Strategy

### Unit tests — `precompiles/batch/src/lib.rs` + `mock.rs`

Mock runtime: System + Balances + Timestamp + EVM. No `pallet-sudo` or
`pallet-gasless-registry` needed. Use
`precompile_utils::testing::PrecompileTesterExt` for ergonomics.

| Mode | Case | Expected |
|---|---|---|
| `batchAll` | 3 sub-calls succeed | outer success, 3× `SubcallSucceeded` events |
| `batchAll` | sub-call #2 reverts with data | outer revert, raw data bubbled, **no** events (state rollback) |
| `batchAll` | OutOfGas on sub-call #1 | outer revert with diagnostic |
| `batchSome` | sub-call #2 reverts | outer success, events: `Succeeded(0), Failed(1), Succeeded(2)` |
| `batchSomeUntilFailure` | sub-call #2 reverts | outer success, events: `Succeeded(0), Failed(1)`, sub-call #3 NOT executed |
| any | length mismatch between the four arrays | outer revert "length mismatch" |
| any | `to.len() == 257` (over `MAX_BATCH_SIZE`) | outer revert "length too large" (raised by `BoundedVec` codec) |
| any | `n = 0` | outer success, zero events (no-op) |
| any | `to[i] == 0x0808` | outer revert, all modes |
| any | `call_data[i].len() == CALL_DATA_LIMIT` (boundary, 2 MiB exactly) | accepted; sub-call dispatched normally |
| any | `call_data[i].len() == CALL_DATA_LIMIT + 1` (over boundary) | outer revert "length too large" (raised by `BoundedBytes` codec), all modes |
| any | caller sends `msg.value > 0` to a batch entry | outer revert "Function is not payable" (entries are non-payable; caller funds sub-call transfers directly) |
| any | `value=[3,5]`, both targets payable, caller has ≥ 8 native | sub-calls see `apparent_value` correctly; balance of `0x0808` stays at 0 (transfers debited from `handle.context().caller`, not the precompile) |
| any | precompile invoked via `DELEGATECALL` / `CALLCODE` (e.g. proxy contract delegatecalls 0x0808) | outer revert "DELEGATECALL/CALLCODE forbidden" |
| any | sub-call returns `ExitReason::Fatal` (e.g. EVM call stack overflow) | outer revert with the fatal exit_status bubbled in all modes (never swallowed by `BatchSome` / `SomeUntilFailure`) |
| any | `gas_limit[i] = 0` | sub-call receives current `remaining_gas()` |
| any | `gas_limit[i] > remaining` | sub-call receives `remaining` (capped) |
| `batchAll` | outer call is `STATICCALL`, sub-call attempts `SSTORE` | sub-call reverts (static enforced); outer reverts |

Plus three regression tests:

- **Selector regression**: assert hardcoded selectors equal
  `solidity::compute_selector(signature)`.
- **Event topic regression**: assert hardcoded topic constants equal
  `keccak256(event_signature)`.
- **Event ABI decode**: take an emitted `SubcallSucceeded` log, assert
  `log.topics.len() == 1`, then ABI-decode `log.data` as a single `uint256`
  and assert it equals the expected sub-call index. Guards against the
  Moonbeam-incompatible failure mode where a future refactor accidentally
  marks the param `indexed` (or vice versa).

### Runtime integration tests — `runtimes/common`

- `FrontierPrecompiles::used_addresses()` contains `H160::from_low_u64_be(2056)`.
- `is_precompile(0x0808)` returns `true`.
- Dispatch to `0x0808` routes into `BatchPrecompile::execute`.

### E2E tests — `packages/contracts/test/batch.spec.ts`

Run against a fresh `--chain dev` node (per memory:
`project_e2e_fresh_node.md`).

**Fixture:** Deploy `Echo.sol` with three methods:
- `succeed(uint256 x)` — reverts if `x == 0`, otherwise stores `x`.
- `setValue(uint256 x)` — always succeeds, stores `x`.
- `receive() external payable {}` — accepts native value.

**Cases:**

- `batchAll` with 3 successful sub-calls → tx succeeds, on-chain state of
  `Echo` reflects all three updates.
- `batchAll` with middle revert → tx reverts with Echo's raw revert reason;
  Echo state unchanged from before the batch.
- `batchSome` with middle revert → tx succeeds; `SubcallFailed(1)` event
  parseable; state from non-failing sub-calls committed.
- `batchSomeUntilFailure` with early revert → tx succeeds; sub-calls past
  the failure point did not execute (state unchanged for those targets).
- Native transfer (caller-funded, **no** outer `msg.value` — entries are
  non-payable): caller has ≥ 3e18 native; `batchAll` to `[user1, user2]`
  with `value=[1e18, 2e18]` → balances of `user1`/`user2` increase by
  exactly those amounts; caller's balance drops by `3e18 + gas`; balance of
  `0x0808` stays at `0`.
- Non-payable enforcement: caller invokes `batchAll` with `msg.value = 1` →
  revert "Function is not payable" (no sub-calls executed, no events emitted).
- Self-call protection: `to = [0x0808]` → tx reverts in every mode.

**Coverage target:** ≥ 80 % line coverage on crate `precompile-batch` via
`cargo llvm-cov -p precompile-batch --fail-under-lines 80`. E2E is acceptance,
not part of the coverage gate.

### Tooling

```bash
cargo test -p precompile-batch                                # unit
cargo llvm-cov -p precompile-batch --fail-under-lines 80      # coverage gate
pnpm --filter contracts test --grep batch                     # E2E (dev node up)
```

## Risk and Mitigation

| Risk | Mitigation |
|---|---|
| Selector or topic drift vs. Moonbeam | Regression tests compute from canonical signature |
| Caller confuses `tx.origin` with sub-call `msg.sender` (e.g. routes a token transfer through an intermediate contract that calls `0x0808`, then approves the intermediate contract instead of `tx.origin`) | Document in `Batch.sol` natspec that sub-call `msg.sender = handle.context().caller`, i.e. the **immediate** caller of the precompile (not `tx.origin`). ERC-20 approvals and access-controlled writes must target that address. |
| Delegatecaller spoofs `handle.context().caller` to drain native value | Dispatch asserts `handle.code_address() == handle.context().address` and reverts `"DELEGATECALL/CALLCODE forbidden"` (covered by E2E + unit test) |
| Indexer misses partial-failure semantics | Events match Moonbeam exactly; Ponder indexer can copy existing handlers |
| `precompile-utils` upstream changes | Workspace dep is pinned via `Cargo.lock`; bump deliberately |

## Open Questions for Implementation Plan

- Exact event topic constants — compute and pin during implementation phase.
- Whether to introduce a `weights/` module entry for batch (other precompiles
  in `runtimes/common/src/weights/` use `WeightInfo`; batch's per-sub-call
  cost is currently flat 1500 gas, so a weights file may not be needed).
- Final placement of the regression test for selectors — inside the crate or
  in `runtimes/common/tests/`.

## Acceptance Criteria

1. `cargo build --release` succeeds on `apps/node`.
2. `cargo test -p precompile-batch` passes.
3. `cargo llvm-cov -p precompile-batch --fail-under-lines 80` passes.
4. `cargo clippy --workspace -- -D warnings` passes.
5. `pnpm --filter contracts test --grep batch` passes against a freshly
   started dev node.
6. `runtimes/common::FrontierPrecompiles::used_addresses()` includes
   `0x0808`, verified by integration test.
7. Solidity interface `Batch.sol` is committed and the three function
   selectors match Moonbeam.
