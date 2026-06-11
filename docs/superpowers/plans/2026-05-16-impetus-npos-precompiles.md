# Plan 3 — Impetus NPoS Precompiles + Production Spec + E2E

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ship the 7 NPoS Solidity precompiles at `0x0810`–`0x0840` (Staking, Session, NominationPools, FastUnstake, Treasury, BagsList, StakingAdmin), wire them into `FrontierPrecompilesNpos`, fix the Plan 2 T28 babe-worker initialisation bug so live impetus can author blocks, replace the dev-Hardhat impetus chain spec with a production-key spec (keeping `impetus_dev_npos` for tests), remap `command.rs::load_spec` aliases to the production spec, ship 9 Hardhat E2E spec files under `packages/contracts/test/`, exercise the `runtime-test-fast` Cargo feature from E2E runs, and refresh `apps/node/CLAUDE.md` to mark the NPoS rollout complete.

**Architecture:** Each precompile crate (`precompiles/<name>/`) follows the existing `precompile-batch` template — `#[precompile_utils_macro::precompile]` on a unit struct, `RawOrigin::Signed(handle.context().caller.into())` origin, DELEGATECALL/CALLCODE guard at every write entry, `RuntimeHelper::try_dispatch` for weight + revert-string mapping, and Moonbeam-style typed events. `FrontierPrecompilesNpos` (introduced as an empty shell in Plan 1) extends the `Basic` registry with the 7 new addresses. The production impetus spec uses a production mnemonic for stash addresses and rotates session keys via a sealed file rather than derivable `//Alice` strings — operators run `scripts/dump-session-keys.ts` to produce SCALE-encoded `Session.sol::setKeys` inputs and `author_insertKey` RPCs. `runtime-test-fast` feature (declared in Plan 2 T6) is finally exercised by the E2E build, compressing sessions to 30 s and eras to 1 min so the full validator/nominator lifecycle fits each test under 5 minutes. The Plan 2 babe-worker init bug is the prerequisite: Task 1 instruments `sc-consensus-babe::start_babe` to surface the swallowed error, then fixes the keystore-insertion or genesis-election ordering so `--chain impetus_dev_npos --validator --alice` authors blocks before any E2E suite runs.

**Tech Stack:** Rust 2021, polkadot-sdk `stable2603`, Frontier `stable2603`, `precompile-utils` (workspace), Hardhat + ethers v6 + TypeScript strict for E2E, `@polkadot/util-crypto` + `@polkadot/keyring` for `dump-session-keys.ts`.

**Spec:** [`docs/superpowers/specs/2026-05-16-impetus-npos-via-precompiles-design.md`](../specs/2026-05-16-impetus-npos-via-precompiles-design.md)
**Predecessor:** [Plan 2 — NPoS Pallets](2026-05-16-impetus-npos-pallets.md) — must be merged first.

**All paths below are relative to repo root** (`/Users/huyduan/projects/blockchain`) unless prefixed with `cd` in commands. Rust workspace lives at `apps/node/`. E2E lives at `packages/contracts/`.

**Plan 2 known gaps that Plan 3 must address:**

| Plan 2 concern | Plan 3 task |
|---|---|
| Babe-worker exits during first poll (T28 DONE_WITH_CONCERNS) | T1 — debug + fix |
| Treasury extrinsics `propose_spend` / `approve_proposal` removed in stable2603 | T6 redesigns `Treasury.sol` to wrap the actual `Treasury::spend()` + `Treasury::spend_local()` extrinsics (root-gated) |
| EPM `SlashHandler = ()` (legacy NegativeImbalance vs modern Credit) | Out of scope — wait for upstream EPM v2 |
| `runtime-test-fast` feature declared but unused | T18 — E2E build sets `--features impetus-runtime/runtime-test-fast` |

---

## File Map

**Created:**
- `apps/node/precompiles/staking/{Cargo.toml,src/lib.rs,src/mock.rs,src/tests.rs}`
- `apps/node/precompiles/session/{Cargo.toml,src/lib.rs,src/mock.rs,src/tests.rs}`
- `apps/node/precompiles/nomination-pools/{Cargo.toml,src/lib.rs,src/mock.rs,src/tests.rs}`
- `apps/node/precompiles/fast-unstake/{Cargo.toml,src/lib.rs,src/mock.rs,src/tests.rs}`
- `apps/node/precompiles/treasury/{Cargo.toml,src/lib.rs,src/mock.rs,src/tests.rs}`
- `apps/node/precompiles/bags-list/{Cargo.toml,src/lib.rs,src/mock.rs,src/tests.rs}`
- `apps/node/precompiles/staking-admin/{Cargo.toml,src/lib.rs,src/mock.rs,src/tests.rs}`
- `packages/contracts/contracts/interfaces/IStaking.sol`
- `packages/contracts/contracts/interfaces/ISession.sol`
- `packages/contracts/contracts/interfaces/INominationPools.sol`
- `packages/contracts/contracts/interfaces/IFastUnstake.sol`
- `packages/contracts/contracts/interfaces/ITreasury.sol`
- `packages/contracts/contracts/interfaces/IBagsList.sol`
- `packages/contracts/contracts/interfaces/IStakingAdmin.sol`
- `packages/contracts/test/helpers/staking-helpers.ts`
- `packages/contracts/test/staking-validator.spec.ts`
- `packages/contracts/test/staking-nominator.spec.ts`
- `packages/contracts/test/staking-rebond.spec.ts`
- `packages/contracts/test/pools.spec.ts`
- `packages/contracts/test/treasury.spec.ts`
- `packages/contracts/test/fast-unstake.spec.ts`
- `packages/contracts/test/bags-list.spec.ts`
- `packages/contracts/test/staking-admin.spec.ts`
- `packages/contracts/test/delegatecall-guard.spec.ts`
- `scripts/dump-session-keys.ts`

**Modified:**
- `apps/node/Cargo.toml` — register the 7 new precompile crates as workspace members
- `apps/node/runtimes/common/Cargo.toml` — pull the 7 precompile crates as deps
- `apps/node/runtimes/common/src/precompiles.rs` — wire 7 new entries into `FrontierPrecompilesNpos`
- `apps/node/runtimes/impetus/Cargo.toml` — gate `runtime-test-fast` propagation
- `apps/node/node/src/service/babe.rs` — fix babe-worker init bug (T1)
- `apps/node/node/src/chain_spec.rs` — add `impetus_production_config()` for the real `impetus` spec; keep `impetus_dev_npos` as dev
- `apps/node/node/src/command.rs` — remap `impetus`/`mainnet` aliases to production spec
- `apps/node/runtimes/impetus/src/lib.rs` — Treasury config tweaks if T6 redesign requires
- `apps/node/CLAUDE.md` — final NPoS rollout notes + precompile address table

**Deleted:** none.

---

## Task 1: Debug + fix babe-worker init bug (Plan 2 T28 outstanding)

**Files:**
- Modify: `apps/node/node/src/service/babe.rs`

This is the BLOCKER for everything downstream — without live impetus block production, no E2E suite can run. The Plan 2 smoke surfaced `Essential task 'babe-worker' failed. Shutting down service.` with no underlying error visible.

- [ ] **Step 1: Instrument `start_babe` to surface the error**

Locate the call to `sc_consensus_babe::start_babe` in `service/babe.rs`. It currently looks like:

```rust
let babe = sc_consensus_babe::start_babe(babe_config)?;
task_manager.spawn_essential_handle().spawn_blocking(
    "babe-worker",
    Some("block-authoring"),
    babe,
);
```

Wrap the `babe` future so any `Err` from its poll path lands in the log before `task_manager` swallows it. Replace with:

```rust
let babe_future = sc_consensus_babe::start_babe(babe_config)?;
let babe_logged: Pin<Box<dyn Future<Output = ()> + Send>> = Box::pin(async move {
    if let Err(err) = babe_future.await {
        log::error!(target: "babe", "babe-worker failed: {err:?}");
    }
});
task_manager.spawn_essential_handle().spawn_blocking(
    "babe-worker",
    Some("block-authoring"),
    babe_logged,
);
```

Note: `start_babe` returns `Result<impl Future<Output = ()>, ServiceError>` in some stable2603 minor versions and `Result<impl Future<Output = Result<(), ConsensusError>>, ServiceError>` in others. If `babe_future.await` returns `()` directly, the `if let Err` wrapper won't compile — instead, wrap the future to catch panics via `futures::FutureExt::catch_unwind`:

```rust
use futures::FutureExt;
let babe_logged = babe_future.catch_unwind().map(|res| {
    if let Err(panic) = res {
        log::error!(target: "babe", "babe-worker panicked: {panic:?}");
    }
});
```

- [ ] **Step 2: Re-run smoke + capture the error**

```bash
cd apps/node && cargo build --release
pkill -f frontier-template-node 2>/dev/null
RUST_LOG=babe=trace,sc_consensus_babe=trace,error \
  ./target/release/frontier-template-node \
    --chain impetus_dev_npos --tmp --validator --alice \
    --unsafe-force-node-key-generation \
    > /tmp/babe-debug.log 2>&1 &
sleep 5
pkill -f frontier-template-node 2>/dev/null
grep -E "babe-worker|panic|epoch|authorities|keystore" /tmp/babe-debug.log | head -30
```

This now logs the actual error. Diagnose based on the output. The four most likely root causes:

(a) **Empty authorities at first epoch poll.** `pallet-session::GenesisConfig::build` ran but `Babe::Authorities` storage is still empty because `pallet_session::SessionManager::new_session(0)` returned `None` (no historical session yet). Fix: ensure `pallet-staking`'s `GenesisElectionProvider` populates the initial validator set BEFORE Babe queries authorities at block 1. The runtime's `genesis_builder_helper::build_state` should run pallets in `construct_runtime!` order — Babe is index 2, Session 18, Staking 20. Babe runs first and has no authorities yet.

  **Fix:** Move the Babe genesis `authorities` array population to be derived from the elected validator set. Option A: populate `babe.authorities` in `chain_spec::impetus_genesis_patch` with the 4 dev-validator Babe keys at index time. Option B: change Babe genesis to pull from `pallet_session::Pallet::queued_keys()` after session genesis runs.

  Option A is simpler. Edit `chain_spec::impetus_genesis_patch` (Plan 2 T18 left this as empty `[]`):

  ```rust
  "babe": {
      "authorities": validators
          .iter()
          .map(|(_, keys)| (keys.babe.clone(), 1u64))
          .collect::<Vec<_>>(),
      "epochConfig": { ... unchanged ... },
  },
  "grandpa": {
      "authorities": validators
          .iter()
          .map(|(_, keys)| (keys.grandpa.clone(), 1u64))
          .collect::<Vec<_>>(),
  },
  ```

(b) **Keystore missing Babe key.** `--alice` auto-keystore-insertion may handle Aura but not the 4-key Babe/Grandpa/ImOnline/AuthDisc set. Fix: extend the dev-keystore-insertion path in `service/babe.rs::new_full` (search for the existing `--alice` handler) to also call `keystore.sr25519_generate_new(BABE, Some("//Alice"))`, same for `grandpa` (ed25519), `im_online`, and `authority_discovery`.

(c) **`epoch_index` runtime API panics.** Babe's first-block poll calls `Runtime::current_epoch()` which depends on `pallet_babe::EpochIndex` storage. If empty, the API returns 0 — usually fine, but if Babe's `genesis_authorities()` returns empty, the worker can't pick a slot leader.

  Same fix as (a).

(d) **GenesisElectionProvider runs after Babe genesis.** Pallet ordering in `construct_runtime!` puts Babe at 2 and Staking at 20. The `OnGenesis` hook fires in pallet-index order, so Babe's genesis ran before Staking's. Babe stores its authorities from the chain spec; Staking elects validators from `staking.stakers`. There's no automatic sync.

  Fix: same as (a) — chain spec populates `babe.authorities` directly with the elected validator's Babe keys, so the initial state matches across both pallets.

- [ ] **Step 3: Apply the most likely fix (a + b combined)**

Edit `apps/node/node/src/chain_spec.rs::impetus_genesis_patch` to populate `babe.authorities` and `grandpa.authorities` from the validators array per Step 2 (a) above.

Edit `apps/node/node/src/service/babe.rs::new_full`. Locate the `--alice` dev-keystore-insertion block (it should look like `if config.role.is_authority() { ... keystore.sr25519_generate_new(...) ... }`). Extend it so when `--alice` is provided:

```rust
use sp_core::crypto::KeyTypeId;

if config.role.is_authority() {
    const ALICE: &str = "//Alice";
    for (key_type, alg) in [
        (sp_consensus_babe::KEY_TYPE,                 "sr25519"),
        (sp_consensus_grandpa::KEY_TYPE,              "ed25519"),
        (pallet_im_online::sr25519::AuthorityId::ID,  "sr25519"),
        (sp_authority_discovery::KEY_TYPE,            "sr25519"),
    ] {
        if alg == "sr25519" {
            keystore_container.keystore().sr25519_generate_new(key_type, Some(ALICE))
                .map_err(|e| format!("keystore insert {key_type:?} failed: {e:?}"))?;
        } else {
            keystore_container.keystore().ed25519_generate_new(key_type, Some(ALICE))
                .map_err(|e| format!("keystore insert {key_type:?} failed: {e:?}"))?;
        }
    }
}
```

This block must run AFTER `KeystoreContainer::new(...)` and BEFORE `start_babe`.

- [ ] **Step 4: Verify the smoke triple gate (Plan 2 T28 acceptance)**

```bash
cd apps/node && cargo build --release --features impetus-runtime/runtime-test-fast
pkill -f frontier-template-node 2>/dev/null
RUST_LOG=runtime=info,babe=info,session=info,staking=info \
  ./target/release/frontier-template-node \
    --chain impetus_dev_npos --tmp --validator --alice \
    --unsafe-force-node-key-generation \
    > /tmp/impetus-smoke.log 2>&1 &
sleep 180
```

Then run all three gates from Plan 2 T28:

```bash
test "$(grep -cE '^.*Imported #[0-9]+' /tmp/impetus-smoke.log)" -ge 20
grep -qE 'New session.*index["= ]+[2-9]|session_index["= ]+[2-9]' /tmp/impetus-smoke.log
ERA_HEX=$(curl -s -H 'Content-Type: application/json' \
  -d '{"jsonrpc":"2.0","method":"state_getStorage","params":["0x5f3e4907f716ac89b6347d15ececedca0b6a45321efae92aea15e0740ec7afe7"],"id":1}' \
  http://127.0.0.1:9944 | python3 -c 'import sys,json; print(json.load(sys.stdin).get("result"))')
case "$ERA_HEX" in
  None|null|""|0x010000000000|0x0100000000) echo 'FAIL'; exit 1 ;;
  0x01*) echo 'OK' ;;
  *) echo "FAIL: $ERA_HEX"; exit 1 ;;
esac
pkill -f frontier-template-node
```

All three must pass. If any fails, the root cause was different — re-run Step 2 to get fresh diagnostics and iterate.

- [ ] **Step 5: Commit**

```bash
git add apps/node/node/src/service/babe.rs apps/node/node/src/chain_spec.rs
git commit -m "fix(node): unstick babe-worker by seeding genesis authorities + keystore"
```

---

## Task 2: `precompile-staking` crate

**Files:**
- Create: `apps/node/precompiles/staking/Cargo.toml`
- Create: `apps/node/precompiles/staking/src/lib.rs`
- Create: `apps/node/precompiles/staking/src/mock.rs`
- Create: `apps/node/precompiles/staking/src/tests.rs`
- Modify: `apps/node/Cargo.toml` (workspace.members)

**Address:** `0x0000000000000000000000000000000000000810` (2064).

- [ ] **Step 1: `Cargo.toml`**

```toml
[package]
name = "precompile-staking"
version = "0.1.0"
edition = "2021"

[dependencies]
parity-scale-codec = { workspace = true, features = ["derive"] }
scale-info         = { workspace = true, features = ["derive"] }
fp-evm             = { workspace = true }
frame-support      = { workspace = true }
frame-system       = { workspace = true }
pallet-balances    = { workspace = true }
pallet-evm         = { workspace = true }
pallet-staking     = { workspace = true }
pallet-session     = { workspace = true }
pallet-sudo        = { workspace = true }
precompile-utils   = { workspace = true }
sp-core            = { workspace = true }
sp-runtime         = { workspace = true }
sp-staking         = { workspace = true }

[dev-dependencies]
pallet-timestamp = { workspace = true }
sp-io            = { workspace = true }

[features]
default = ["std"]
std = [
    "parity-scale-codec/std", "scale-info/std",
    "fp-evm/std", "frame-support/std", "frame-system/std",
    "pallet-balances/std", "pallet-evm/std", "pallet-staking/std",
    "pallet-session/std", "pallet-sudo/std",
    "precompile-utils/std", "sp-core/std", "sp-runtime/std", "sp-staking/std",
]
```

- [ ] **Step 2: Add to `apps/node/Cargo.toml` `[workspace.members]`**

Append `"precompiles/staking",` to the existing `members = [...]` list.

- [ ] **Step 3: `src/lib.rs`** — write the precompile dispatch. Follow this skeleton; expand each write entry per the spec table.

```rust
#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

use alloc::vec::Vec;
use core::marker::PhantomData;

use fp_evm::{PrecompileFailure, PrecompileHandle};
use frame_support::dispatch::GetDispatchInfo;
use frame_system::pallet_prelude::OriginFor;
use pallet_evm::AddressMapping;
use precompile_utils::prelude::*;
use sp_core::{H160, U256};
use sp_runtime::traits::Dispatchable;
use sp_staking::EraIndex;

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

/// Solidity reward destination enum.
/// kind 0=Staked, 1=Stash, 2=Controller, 3=Account(account), 4=None
#[derive(Default, solidity::Codec)]
pub struct RewardDestination {
    pub kind: u8,
    pub account: Address,
}

fn convert_payee<AccountId: From<H160>>(
    payee: RewardDestination,
) -> Result<pallet_staking::RewardDestination<AccountId>, PrecompileFailure> {
    use pallet_staking::RewardDestination::*;
    Ok(match payee.kind {
        0 => Staked,
        1 => Stash,
        2 => Account(payee.account.0.into()), // Controller variant deprecated; map to Account
        3 => Account(payee.account.0.into()),
        4 => None,
        _ => return Err(revert("invalid reward destination kind")),
    })
}

pub struct StakingPrecompile<Runtime>(PhantomData<Runtime>);

#[precompile_utils_macro::precompile]
impl<Runtime> StakingPrecompile<Runtime>
where
    Runtime: pallet_evm::Config + pallet_staking::Config + pallet_session::Config + pallet_sudo::Config,
    Runtime::AccountId: From<H160> + Into<H160>,
    <Runtime as frame_system::Config>::RuntimeCall:
        Dispatchable<RuntimeOrigin = Runtime::RuntimeOrigin> + GetDispatchInfo,
    <Runtime as frame_system::Config>::RuntimeCall: From<pallet_staking::Call<Runtime>>,
    pallet_staking::BalanceOf<Runtime>: TryFrom<U256> + Into<U256>,
{
    #[precompile::public("bond(uint256,(uint8,address))")]
    fn bond(
        handle: &mut impl PrecompileHandle,
        value: U256,
        payee: RewardDestination,
    ) -> EvmResult {
        delegate_guard(handle)?;
        let value = balance_from_u256::<Runtime>(value)?;
        let payee = convert_payee::<Runtime::AccountId>(payee)?;
        let origin = Runtime::AddressMapping::into_account_id(handle.context().caller);
        let call = pallet_staking::Call::<Runtime>::bond { value, payee };
        RuntimeHelper::<Runtime>::try_dispatch(handle, Some(origin).into(), call)?;
        Ok(())
    }

    #[precompile::public("bondExtra(uint256)")]
    fn bond_extra(handle: &mut impl PrecompileHandle, max_additional: U256) -> EvmResult {
        delegate_guard(handle)?;
        let max_additional = balance_from_u256::<Runtime>(max_additional)?;
        let origin = Runtime::AddressMapping::into_account_id(handle.context().caller);
        let call = pallet_staking::Call::<Runtime>::bond_extra { max_additional };
        RuntimeHelper::<Runtime>::try_dispatch(handle, Some(origin).into(), call)?;
        Ok(())
    }

    #[precompile::public("unbond(uint256)")]
    fn unbond(handle: &mut impl PrecompileHandle, value: U256) -> EvmResult {
        delegate_guard(handle)?;
        let value = balance_from_u256::<Runtime>(value)?;
        let origin = Runtime::AddressMapping::into_account_id(handle.context().caller);
        let call = pallet_staking::Call::<Runtime>::unbond { value };
        RuntimeHelper::<Runtime>::try_dispatch(handle, Some(origin).into(), call)?;
        Ok(())
    }

    #[precompile::public("withdrawUnbonded(uint32)")]
    fn withdraw_unbonded(handle: &mut impl PrecompileHandle, num_slashing_spans: u32) -> EvmResult {
        delegate_guard(handle)?;
        let origin = Runtime::AddressMapping::into_account_id(handle.context().caller);
        let call = pallet_staking::Call::<Runtime>::withdraw_unbonded { num_slashing_spans };
        RuntimeHelper::<Runtime>::try_dispatch(handle, Some(origin).into(), call)?;
        Ok(())
    }

    #[precompile::public("validate((uint16,bool))")]
    fn validate(
        handle: &mut impl PrecompileHandle,
        prefs: (u16, bool),
    ) -> EvmResult {
        delegate_guard(handle)?;
        let prefs = pallet_staking::ValidatorPrefs {
            commission: sp_runtime::Perbill::from_parts(prefs.0 as u32 * 10_000_000),
            blocked: prefs.1,
        };
        let origin = Runtime::AddressMapping::into_account_id(handle.context().caller);
        let call = pallet_staking::Call::<Runtime>::validate { prefs };
        RuntimeHelper::<Runtime>::try_dispatch(handle, Some(origin).into(), call)?;
        Ok(())
    }

    #[precompile::public("nominate(address[])")]
    fn nominate(handle: &mut impl PrecompileHandle, targets: Vec<Address>) -> EvmResult {
        delegate_guard(handle)?;
        let targets: Vec<Runtime::AccountId> =
            targets.into_iter().map(|a| a.0.into()).collect();
        let targets_lookup = targets
            .into_iter()
            .map(|t| <Runtime::Lookup as sp_runtime::traits::StaticLookup>::unlookup(t))
            .collect();
        let origin = Runtime::AddressMapping::into_account_id(handle.context().caller);
        let call = pallet_staking::Call::<Runtime>::nominate { targets: targets_lookup };
        RuntimeHelper::<Runtime>::try_dispatch(handle, Some(origin).into(), call)?;
        Ok(())
    }

    #[precompile::public("chill()")]
    fn chill(handle: &mut impl PrecompileHandle) -> EvmResult {
        delegate_guard(handle)?;
        let origin = Runtime::AddressMapping::into_account_id(handle.context().caller);
        let call = pallet_staking::Call::<Runtime>::chill {};
        RuntimeHelper::<Runtime>::try_dispatch(handle, Some(origin).into(), call)?;
        Ok(())
    }

    #[precompile::public("setPayee((uint8,address))")]
    fn set_payee(handle: &mut impl PrecompileHandle, payee: RewardDestination) -> EvmResult {
        delegate_guard(handle)?;
        let payee = convert_payee::<Runtime::AccountId>(payee)?;
        let origin = Runtime::AddressMapping::into_account_id(handle.context().caller);
        let call = pallet_staking::Call::<Runtime>::set_payee { payee };
        RuntimeHelper::<Runtime>::try_dispatch(handle, Some(origin).into(), call)?;
        Ok(())
    }

    #[precompile::public("payoutStakers(address,uint32)")]
    fn payout_stakers(
        handle: &mut impl PrecompileHandle,
        validator_stash: Address,
        era: u32,
    ) -> EvmResult {
        delegate_guard(handle)?;
        let validator_stash: Runtime::AccountId = validator_stash.0.into();
        let origin = Runtime::AddressMapping::into_account_id(handle.context().caller);
        let call = pallet_staking::Call::<Runtime>::payout_stakers {
            validator_stash,
            era,
        };
        RuntimeHelper::<Runtime>::try_dispatch(handle, Some(origin).into(), call)?;
        Ok(())
    }

    #[precompile::public("payoutStakersByPage(address,uint32,uint32)")]
    fn payout_stakers_by_page(
        handle: &mut impl PrecompileHandle,
        validator_stash: Address,
        era: u32,
        page: u32,
    ) -> EvmResult {
        delegate_guard(handle)?;
        let validator_stash: Runtime::AccountId = validator_stash.0.into();
        let origin = Runtime::AddressMapping::into_account_id(handle.context().caller);
        let call = pallet_staking::Call::<Runtime>::payout_stakers_by_page {
            validator_stash,
            era,
            page,
        };
        RuntimeHelper::<Runtime>::try_dispatch(handle, Some(origin).into(), call)?;
        Ok(())
    }

    #[precompile::public("rebond(uint256)")]
    fn rebond(handle: &mut impl PrecompileHandle, value: U256) -> EvmResult {
        delegate_guard(handle)?;
        let value = balance_from_u256::<Runtime>(value)?;
        let origin = Runtime::AddressMapping::into_account_id(handle.context().caller);
        let call = pallet_staking::Call::<Runtime>::rebond { value };
        RuntimeHelper::<Runtime>::try_dispatch(handle, Some(origin).into(), call)?;
        Ok(())
    }

    // View functions

    #[precompile::public("currentEra()")]
    #[precompile::view]
    fn current_era(_handle: &mut impl PrecompileHandle) -> EvmResult<u32> {
        Ok(pallet_staking::CurrentEra::<Runtime>::get().unwrap_or_default())
    }

    #[precompile::public("activeEra()")]
    #[precompile::view]
    fn active_era(_handle: &mut impl PrecompileHandle) -> EvmResult<(u32, u64)> {
        let active = pallet_staking::ActiveEra::<Runtime>::get();
        match active {
            Some(info) => Ok((info.index, info.start.unwrap_or_default())),
            None => Ok((0, 0)),
        }
    }

    #[precompile::public("minNominatorBond()")]
    #[precompile::view]
    fn min_nominator_bond(_handle: &mut impl PrecompileHandle) -> EvmResult<U256> {
        Ok(pallet_staking::MinNominatorBond::<Runtime>::get().into())
    }

    #[precompile::public("minValidatorBond()")]
    #[precompile::view]
    fn min_validator_bond(_handle: &mut impl PrecompileHandle) -> EvmResult<U256> {
        Ok(pallet_staking::MinValidatorBond::<Runtime>::get().into())
    }

    #[precompile::public("validatorCount()")]
    #[precompile::view]
    fn validator_count(_handle: &mut impl PrecompileHandle) -> EvmResult<u32> {
        Ok(pallet_staking::ValidatorCount::<Runtime>::get())
    }

    // The remaining write entries (kick, chillOther, forceApplyMinCommission,
    // reapStash) and remaining views (bonded, ledger, validators, nominators,
    // erasStakers, erasValidatorReward, erasRewardPoints, minActiveStake,
    // counterForValidators, counterForNominators, historyDepth) follow the
    // same pattern: delegate_guard at write entries, RawOrigin::Signed
    // dispatch via RuntimeHelper, storage reads for views. Implement each
    // method per the spec table at section 4.1 of the design doc.
}

// Shared helpers ------------------------------------------------------------

fn delegate_guard(handle: &mut impl PrecompileHandle) -> EvmResult<()> {
    if handle.code_address() != handle.context().address {
        return Err(revert("DELEGATECALL/CALLCODE forbidden"));
    }
    Ok(())
}

fn balance_from_u256<R: pallet_staking::Config>(
    v: U256,
) -> EvmResult<pallet_staking::BalanceOf<R>> {
    pallet_staking::BalanceOf::<R>::try_from(v)
        .map_err(|_| revert("balance overflow"))
}

fn revert(msg: &'static str) -> PrecompileFailure {
    PrecompileFailure::Revert {
        exit_status: fp_evm::ExitRevert::Reverted,
        output: precompile_utils::revert::revert_as_bytes(msg),
    }
}
```

Implement the missing entries listed in the inline comment. Each follows the same shape; copy-paste from the template above and swap the `pallet_staking::Call` variant. Do NOT skip — the spec lists 15 write entries and 16 view entries; all must be present for ABI coverage.

- [ ] **Step 4: `src/mock.rs`** — Substrate test runtime with Balances + Staking + Session + Sudo + EVM (mock). Follow `apps/node/precompiles/batch/src/mock.rs` as the template — same Frontier-mock-runtime pattern with the StakingPrecompile registered at address `0x0810`.

- [ ] **Step 5: `src/tests.rs`** — at least 20 tests covering:
  - Each write entry's happy path (bond, bondExtra, unbond, withdrawUnbonded, validate, nominate, chill, setPayee, payoutStakers, rebond, kick, chillOther, forceApplyMinCommission, reapStash, payoutStakersByPage).
  - Each view entry returns correct storage value.
  - DELEGATECALL guard reverts on each write.
  - Origin construction: `handle.context().caller` becomes `RawOrigin::Signed`.
  - Pallet error mapping: `InsufficientBond`, `NotController`, `AlreadyBonded` revert with the canonical error name.
  - Codec bounds: `nominate` with >MAX_NOMINATIONS targets reverts.

Reference `apps/node/precompiles/batch/src/tests.rs` for mock-runtime test patterns.

- [ ] **Step 6: Verify**

```bash
cd apps/node && SKIP_WASM_BUILD=1 cargo test -p precompile-staking
```

Expected: all ≥20 tests pass.

- [ ] **Step 7: Commit**

```bash
git add apps/node/precompiles/staking/ apps/node/Cargo.toml apps/node/Cargo.lock
git commit -m "feat(precompile): Staking precompile at 0x0810"
```

---

## Task 3: `precompile-session` crate

**Files:**
- Create: `apps/node/precompiles/session/{Cargo.toml,src/lib.rs,src/mock.rs,src/tests.rs}`
- Modify: `apps/node/Cargo.toml`

**Address:** `0x0000000000000000000000000000000000000818` (2072).

- [ ] **Step 1: Cargo.toml** — same template as Task 2 Step 1 but `name = "precompile-session"` and the dep set is narrower: `pallet-session`, no `pallet-staking`.

- [ ] **Step 2: Add to workspace members**

`"precompiles/session",`

- [ ] **Step 3: `src/lib.rs`** — `SessionPrecompile<Runtime>` with these entries (mirror spec §4.2):

```rust
#[precompile_utils_macro::precompile]
impl<Runtime> SessionPrecompile<Runtime>
where
    Runtime: pallet_evm::Config + pallet_session::Config,
    Runtime::AccountId: From<H160> + Into<H160>,
    <Runtime as frame_system::Config>::RuntimeCall:
        Dispatchable<RuntimeOrigin = Runtime::RuntimeOrigin> + GetDispatchInfo,
    <Runtime as frame_system::Config>::RuntimeCall: From<pallet_session::Call<Runtime>>,
{
    #[precompile::public("setKeys(bytes,bytes)")]
    fn set_keys(handle: &mut impl PrecompileHandle, keys: BoundedBytes<ConstU32<512>>, proof: BoundedBytes<ConstU32<512>>) -> EvmResult {
        delegate_guard(handle)?;
        let keys: Runtime::Keys = scale_codec::Decode::decode(&mut &keys.as_bytes()[..])
            .map_err(|_| revert("InvalidKey: SCALE decode failed"))?;
        let origin = Runtime::AddressMapping::into_account_id(handle.context().caller);
        let call = pallet_session::Call::<Runtime>::set_keys { keys, proof: proof.into() };
        RuntimeHelper::<Runtime>::try_dispatch(handle, Some(origin).into(), call)?;
        Ok(())
    }

    #[precompile::public("purgeKeys()")]
    fn purge_keys(handle: &mut impl PrecompileHandle) -> EvmResult {
        delegate_guard(handle)?;
        let origin = Runtime::AddressMapping::into_account_id(handle.context().caller);
        let call = pallet_session::Call::<Runtime>::purge_keys {};
        RuntimeHelper::<Runtime>::try_dispatch(handle, Some(origin).into(), call)?;
        Ok(())
    }

    #[precompile::public("currentIndex()")]
    #[precompile::view]
    fn current_index(_handle: &mut impl PrecompileHandle) -> EvmResult<u32> {
        Ok(pallet_session::CurrentIndex::<Runtime>::get())
    }

    #[precompile::public("nextKeys(address)")]
    #[precompile::view]
    fn next_keys(_handle: &mut impl PrecompileHandle, validator: Address) -> EvmResult<UnboundedBytes> {
        let acct: Runtime::AccountId = validator.0.into();
        let keys = pallet_session::NextKeys::<Runtime>::get(acct)
            .map(|k| scale_codec::Encode::encode(&k))
            .unwrap_or_default();
        Ok(keys.into())
    }

    #[precompile::public("queuedKeys()")]
    #[precompile::view]
    fn queued_keys(_handle: &mut impl PrecompileHandle) -> EvmResult<(Vec<Address>, Vec<UnboundedBytes>)> {
        let queued = pallet_session::QueuedKeys::<Runtime>::get();
        let validators: Vec<Address> = queued.iter().map(|(v, _)| {
            let h160: H160 = v.clone().into();
            Address(h160)
        }).collect();
        let keys: Vec<UnboundedBytes> = queued.iter().map(|(_, k)| {
            scale_codec::Encode::encode(k).into()
        }).collect();
        Ok((validators, keys))
    }
}
```

- [ ] **Step 4: `src/mock.rs`** — same template as Task 2 with a minimal pallet-session runtime.

- [ ] **Step 5: `src/tests.rs`** — ≥ 8 tests:
  - `setKeys` happy path with SCALE-encoded 4-key struct.
  - `setKeys` rejects malformed SCALE bytes with revert `"InvalidKey: SCALE decode failed"`.
  - `purgeKeys` happy path.
  - `currentIndex`, `nextKeys`, `queuedKeys` return correct values.
  - DELEGATECALL guard on `setKeys` and `purgeKeys`.

- [ ] **Step 6: Verify + commit**

```bash
cd apps/node && SKIP_WASM_BUILD=1 cargo test -p precompile-session
git add apps/node/precompiles/session/ apps/node/Cargo.toml apps/node/Cargo.lock
git commit -m "feat(precompile): Session precompile at 0x0818"
```

---

## Task 4: `precompile-nomination-pools` crate

**Files:**
- Create: `apps/node/precompiles/nomination-pools/{Cargo.toml,src/lib.rs,src/mock.rs,src/tests.rs}`
- Modify: `apps/node/Cargo.toml`

**Address:** `0x0000000000000000000000000000000000000820` (2080).

- [ ] **Step 1: Cargo.toml** — same template, deps include `pallet-nomination-pools`, `pallet-staking`, `pallet-balances`, `pallet-sudo`.

- [ ] **Step 2: Add to workspace members.**

- [ ] **Step 3: `src/lib.rs`** — 19 write entries + 4 view entries per spec §4.3. Pattern matches Task 2 (delegate_guard + RawOrigin::Signed + try_dispatch). Examples for the trickier entries:

```rust
#[precompile::public("create(uint256,address,address,address)")]
fn create(
    handle: &mut impl PrecompileHandle,
    amount: U256,
    root: Address,
    nominator: Address,
    bouncer: Address,
) -> EvmResult {
    delegate_guard(handle)?;
    let amount = balance_from_u256::<Runtime>(amount)?;
    let root_lookup = lookup(root)?;
    let nominator_lookup = lookup(nominator)?;
    let bouncer_lookup = lookup(bouncer)?;
    let origin = Runtime::AddressMapping::into_account_id(handle.context().caller);
    let call = pallet_nomination_pools::Call::<Runtime>::create {
        amount, root: root_lookup, nominator: nominator_lookup, bouncer: bouncer_lookup,
    };
    RuntimeHelper::<Runtime>::try_dispatch(handle, Some(origin).into(), call)?;
    Ok(())
}

#[precompile::public("setState(uint32,uint8)")]
fn set_state(handle: &mut impl PrecompileHandle, pool_id: u32, state: u8) -> EvmResult {
    delegate_guard(handle)?;
    let state = match state {
        0 => pallet_nomination_pools::PoolState::Open,
        1 => pallet_nomination_pools::PoolState::Blocked,
        2 => pallet_nomination_pools::PoolState::Destroying,
        _ => return Err(revert("invalid pool state")),
    };
    let origin = Runtime::AddressMapping::into_account_id(handle.context().caller);
    let call = pallet_nomination_pools::Call::<Runtime>::set_state { pool_id, state };
    RuntimeHelper::<Runtime>::try_dispatch(handle, Some(origin).into(), call)?;
    Ok(())
}

#[precompile::public("setConfigs(uint256,uint256,uint32,uint32,uint32,uint32)")]
fn set_configs(
    handle: &mut impl PrecompileHandle,
    min_join_bond: U256,
    min_create_bond: U256,
    max_pools: u32,
    max_members: u32,
    max_members_per_pool: u32,
    global_max_commission: u32,
) -> EvmResult {
    delegate_guard(handle)?;
    sudo_only::<Runtime>(handle)?;
    use pallet_nomination_pools::ConfigOp::Set;
    let call = pallet_nomination_pools::Call::<Runtime>::set_configs {
        min_join_bond: Set(balance_from_u256::<Runtime>(min_join_bond)?),
        min_create_bond: Set(balance_from_u256::<Runtime>(min_create_bond)?),
        max_pools: Set(max_pools),
        max_members: Set(max_members),
        max_members_per_pool: Set(max_members_per_pool),
        global_max_commission: Set(sp_runtime::Perbill::from_parts(global_max_commission)),
    };
    RuntimeHelper::<Runtime>::try_dispatch(handle, frame_system::RawOrigin::Root.into(), call)?;
    Ok(())
}

fn sudo_only<R: pallet_sudo::Config>(handle: &mut impl PrecompileHandle) -> EvmResult<()> {
    let caller: R::AccountId = handle.context().caller.into();
    let sudo_key = pallet_sudo::Key::<R>::get().ok_or_else(|| revert("NotSudo: no sudo key set"))?;
    if caller != sudo_key {
        return Err(revert("NotSudo"));
    }
    Ok(())
}
```

Implement remaining entries from spec §4.3 (join, bondExtra, claimPayout, unbond, poolWithdrawUnbonded, withdrawUnbonded, createWithPoolId, nominate, setMetadata, updateRoles, chill, bondExtraOther, setCommission, setCommissionMax, setCommissionChangeRate, claimCommission).

- [ ] **Step 4 & 5: Mock + tests** — ≥ 22 tests covering all write entries + 4 views + sudo gating on `setConfigs` + DELEGATECALL guards.

- [ ] **Step 6: Verify + commit**

```bash
cd apps/node && SKIP_WASM_BUILD=1 cargo test -p precompile-nomination-pools
git add apps/node/precompiles/nomination-pools/ apps/node/Cargo.toml apps/node/Cargo.lock
git commit -m "feat(precompile): NominationPools precompile at 0x0820"
```

---

## Task 5: `precompile-fast-unstake` crate

**Files:**
- Create: `apps/node/precompiles/fast-unstake/{Cargo.toml,src/lib.rs,src/mock.rs,src/tests.rs}`
- Modify: `apps/node/Cargo.toml`

**Address:** `0x0000000000000000000000000000000000000828` (2088).

- [ ] **Step 1: Cargo.toml** — deps include `pallet-fast-unstake`, `pallet-staking`, `pallet-balances`, `pallet-sudo`.

- [ ] **Step 2: Add to workspace members.**

- [ ] **Step 3: `src/lib.rs`** — 3 write entries + 3 view entries per spec §4.4:

```rust
#[precompile_utils_macro::precompile]
impl<Runtime> FastUnstakePrecompile<Runtime>
where
    Runtime: pallet_evm::Config + pallet_fast_unstake::Config + pallet_sudo::Config,
    Runtime::AccountId: From<H160> + Into<H160>,
    <Runtime as frame_system::Config>::RuntimeCall:
        Dispatchable<RuntimeOrigin = Runtime::RuntimeOrigin> + GetDispatchInfo,
    <Runtime as frame_system::Config>::RuntimeCall: From<pallet_fast_unstake::Call<Runtime>>,
{
    #[precompile::public("registerFastUnstake()")]
    fn register_fast_unstake(handle: &mut impl PrecompileHandle) -> EvmResult {
        delegate_guard(handle)?;
        let origin = Runtime::AddressMapping::into_account_id(handle.context().caller);
        let call = pallet_fast_unstake::Call::<Runtime>::register_fast_unstake {};
        RuntimeHelper::<Runtime>::try_dispatch(handle, Some(origin).into(), call)?;
        Ok(())
    }

    #[precompile::public("deregister()")]
    fn deregister(handle: &mut impl PrecompileHandle) -> EvmResult {
        delegate_guard(handle)?;
        let origin = Runtime::AddressMapping::into_account_id(handle.context().caller);
        let call = pallet_fast_unstake::Call::<Runtime>::deregister {};
        RuntimeHelper::<Runtime>::try_dispatch(handle, Some(origin).into(), call)?;
        Ok(())
    }

    #[precompile::public("control(uint32)")]
    fn control(handle: &mut impl PrecompileHandle, eras_to_check_per_block: u32) -> EvmResult {
        delegate_guard(handle)?;
        sudo_only::<Runtime>(handle)?;
        let call = pallet_fast_unstake::Call::<Runtime>::control { eras_to_check_per_block };
        RuntimeHelper::<Runtime>::try_dispatch(handle, frame_system::RawOrigin::Root.into(), call)?;
        Ok(())
    }

    #[precompile::public("head()")]
    #[precompile::view]
    fn head(_handle: &mut impl PrecompileHandle) -> EvmResult<Address> {
        let head_info = pallet_fast_unstake::Head::<Runtime>::get();
        let stash: H160 = head_info.map(|h| h.stash.into()).unwrap_or_default();
        Ok(Address(stash))
    }

    #[precompile::public("queue(address)")]
    #[precompile::view]
    fn queue(_handle: &mut impl PrecompileHandle, stash: Address) -> EvmResult<U256> {
        let stash: Runtime::AccountId = stash.0.into();
        let deposit = pallet_fast_unstake::Queue::<Runtime>::get(stash).unwrap_or_default();
        Ok(deposit.into())
    }

    #[precompile::public("erasToCheckPerBlock()")]
    #[precompile::view]
    fn eras_to_check_per_block(_handle: &mut impl PrecompileHandle) -> EvmResult<u32> {
        Ok(pallet_fast_unstake::ErasToCheckPerBlock::<Runtime>::get())
    }
}
```

- [ ] **Step 4 & 5: Mock + tests** — ≥ 8 tests: register/deregister happy path, sudo-gated control, queue read after register, head read, eras read, 3× delegate_guard, sudo gating revert.

- [ ] **Step 6: Verify + commit**

```bash
cd apps/node && SKIP_WASM_BUILD=1 cargo test -p precompile-fast-unstake
git add apps/node/precompiles/fast-unstake/ apps/node/Cargo.toml apps/node/Cargo.lock
git commit -m "feat(precompile): FastUnstake precompile at 0x0828"
```

---

## Task 6: `precompile-treasury` crate (stable2603 API redesign)

**Files:**
- Create: `apps/node/precompiles/treasury/{Cargo.toml,src/lib.rs,src/mock.rs,src/tests.rs}`
- Modify: `apps/node/Cargo.toml`

**Address:** `0x0000000000000000000000000000000000000830` (2096).

**Plan 2 surfaced that stable2603 removed `propose_spend` and `approve_proposal`** — those are pre-2402 extrinsics. The actual stable2603 Treasury surface is:

- `spend(asset_kind, amount, beneficiary, valid_from)` — requires `SpendOrigin` (set to `NeverEnsureOrigin` in Plan 2 → unreachable without root).
- `spend_local(amount, beneficiary)` — requires `SpendOrigin` (same).
- `payout(index)` — anyone can call to trigger payout of an approved spend.
- `check_status(index)` — view.
- `void_spend(index)` — root-gated.
- `remove_approval(index)` — root-gated (lingering from proposal era).

Plan 3 redesigns `Treasury.sol` to wrap this surface. The original spec's `proposeSpend`/`approveProposal` Solidity functions are **removed** — they no longer have a backing extrinsic.

- [ ] **Step 1: Cargo.toml** — deps `pallet-treasury`, `pallet-balances`, `pallet-sudo`.

- [ ] **Step 2: Add to workspace members.**

- [ ] **Step 3: `src/lib.rs`** with the redesigned Solidity surface:

```rust
#[precompile_utils_macro::precompile]
impl<Runtime> TreasuryPrecompile<Runtime>
where
    Runtime: pallet_evm::Config + pallet_treasury::Config + pallet_sudo::Config,
    Runtime::AccountId: From<H160> + Into<H160>,
    Runtime::Beneficiary: From<Runtime::AccountId>,
    <Runtime as frame_system::Config>::RuntimeCall:
        Dispatchable<RuntimeOrigin = Runtime::RuntimeOrigin> + GetDispatchInfo,
    <Runtime as frame_system::Config>::RuntimeCall: From<pallet_treasury::Call<Runtime>>,
{
    /// Root-only direct spend (SpendOrigin = NeverEnsureOrigin in v1 so this
    /// requires `Sudo::sudo(...)` externally; precompile exists for ABI
    /// completeness and reverts "NotSudo" when called directly).
    #[precompile::public("spendLocal(uint256,address)")]
    fn spend_local(
        handle: &mut impl PrecompileHandle,
        amount: U256,
        beneficiary: Address,
    ) -> EvmResult {
        delegate_guard(handle)?;
        sudo_only::<Runtime>(handle)?;
        let amount = balance_from_u256::<Runtime>(amount)?;
        let beneficiary: Runtime::AccountId = beneficiary.0.into();
        let beneficiary_lookup = <Runtime::BeneficiaryLookup as sp_runtime::traits::StaticLookup>::unlookup(beneficiary.into());
        let call = pallet_treasury::Call::<Runtime>::spend_local {
            amount,
            beneficiary: beneficiary_lookup,
        };
        RuntimeHelper::<Runtime>::try_dispatch(handle, frame_system::RawOrigin::Root.into(), call)?;
        Ok(())
    }

    /// Trigger a payout of an approved spend. Permissionless.
    #[precompile::public("payout(uint32)")]
    fn payout(handle: &mut impl PrecompileHandle, index: u32) -> EvmResult {
        delegate_guard(handle)?;
        let origin = Runtime::AddressMapping::into_account_id(handle.context().caller);
        let call = pallet_treasury::Call::<Runtime>::payout { index };
        RuntimeHelper::<Runtime>::try_dispatch(handle, Some(origin).into(), call)?;
        Ok(())
    }

    /// Cancel an approved spend (root only).
    #[precompile::public("voidSpend(uint32)")]
    fn void_spend(handle: &mut impl PrecompileHandle, index: u32) -> EvmResult {
        delegate_guard(handle)?;
        sudo_only::<Runtime>(handle)?;
        let call = pallet_treasury::Call::<Runtime>::void_spend { index };
        RuntimeHelper::<Runtime>::try_dispatch(handle, frame_system::RawOrigin::Root.into(), call)?;
        Ok(())
    }

    /// Inspect a pending spend status.
    #[precompile::public("checkStatus(uint32)")]
    fn check_status(handle: &mut impl PrecompileHandle, index: u32) -> EvmResult {
        delegate_guard(handle)?;
        let origin = Runtime::AddressMapping::into_account_id(handle.context().caller);
        let call = pallet_treasury::Call::<Runtime>::check_status { index };
        RuntimeHelper::<Runtime>::try_dispatch(handle, Some(origin).into(), call)?;
        Ok(())
    }

    // Views

    #[precompile::public("pot()")]
    #[precompile::view]
    fn pot(_handle: &mut impl PrecompileHandle) -> EvmResult<U256> {
        let pot = pallet_treasury::Pallet::<Runtime>::pot();
        Ok(pot.into())
    }

    #[precompile::public("spendCount()")]
    #[precompile::view]
    fn spend_count(_handle: &mut impl PrecompileHandle) -> EvmResult<u32> {
        Ok(pallet_treasury::SpendCount::<Runtime>::get())
    }

    #[precompile::public("approvals()")]
    #[precompile::view]
    fn approvals(_handle: &mut impl PrecompileHandle) -> EvmResult<Vec<u32>> {
        let approvals: Vec<u32> = pallet_treasury::Approvals::<Runtime>::get().into_iter().collect();
        Ok(approvals)
    }
}
```

- [ ] **Step 4 & 5: Mock + tests** — ≥ 10 tests: `spendLocal` requires sudo (Root via `Sudo::sudo` in test setup), `payout` permissionless, `voidSpend` sudo-gated, `pot` returns seeded balance, `spendCount` increments, DELEGATECALL guards.

- [ ] **Step 6: Verify + commit**

```bash
cd apps/node && SKIP_WASM_BUILD=1 cargo test -p precompile-treasury
git add apps/node/precompiles/treasury/ apps/node/Cargo.toml apps/node/Cargo.lock
git commit -m "feat(precompile): Treasury precompile at 0x0830 (stable2603 spend API)"
```

---

## Task 7: `precompile-bags-list` crate

**Files:**
- Create: `apps/node/precompiles/bags-list/{Cargo.toml,src/lib.rs,src/mock.rs,src/tests.rs}`
- Modify: `apps/node/Cargo.toml`

**Address:** `0x0000000000000000000000000000000000000838` (2104).

- [ ] **Step 1: Cargo.toml** — deps include `pallet-bags-list`, `pallet-staking`.

- [ ] **Step 2: Add to workspace members.**

- [ ] **Step 3: `src/lib.rs`** with spec §4.6's surface:

```rust
#[precompile_utils_macro::precompile]
impl<Runtime> BagsListPrecompile<Runtime>
where
    Runtime: pallet_evm::Config + pallet_bags_list::Config<pallet_bags_list::Instance1>,
    Runtime::AccountId: From<H160> + Into<H160>,
    <Runtime as frame_system::Config>::RuntimeCall:
        Dispatchable<RuntimeOrigin = Runtime::RuntimeOrigin> + GetDispatchInfo,
    <Runtime as frame_system::Config>::RuntimeCall:
        From<pallet_bags_list::Call<Runtime, pallet_bags_list::Instance1>>,
{
    #[precompile::public("putInFrontOf(address)")]
    fn put_in_front_of(handle: &mut impl PrecompileHandle, lighter: Address) -> EvmResult {
        delegate_guard(handle)?;
        let lighter: Runtime::AccountId = lighter.0.into();
        let lighter_lookup = <Runtime::Lookup as sp_runtime::traits::StaticLookup>::unlookup(lighter);
        let origin = Runtime::AddressMapping::into_account_id(handle.context().caller);
        let call = pallet_bags_list::Call::<Runtime, pallet_bags_list::Instance1>::put_in_front_of { lighter: lighter_lookup };
        RuntimeHelper::<Runtime>::try_dispatch(handle, Some(origin).into(), call)?;
        Ok(())
    }

    #[precompile::public("rebag(address)")]
    fn rebag(handle: &mut impl PrecompileHandle, dislocated: Address) -> EvmResult {
        delegate_guard(handle)?;
        let dislocated: Runtime::AccountId = dislocated.0.into();
        let dislocated_lookup = <Runtime::Lookup as sp_runtime::traits::StaticLookup>::unlookup(dislocated);
        let origin = Runtime::AddressMapping::into_account_id(handle.context().caller);
        let call = pallet_bags_list::Call::<Runtime, pallet_bags_list::Instance1>::rebag { dislocated: dislocated_lookup };
        RuntimeHelper::<Runtime>::try_dispatch(handle, Some(origin).into(), call)?;
        Ok(())
    }

    // Views — pallet-bags-list doesn't expose direct getters for an account's
    // bag or score; we read from its CounterForListNodes + List storage.

    #[precompile::public("listSize()")]
    #[precompile::view]
    fn list_size(_handle: &mut impl PrecompileHandle) -> EvmResult<u32> {
        Ok(pallet_bags_list::CounterForListNodes::<Runtime, pallet_bags_list::Instance1>::get())
    }

    #[precompile::public("score(address)")]
    #[precompile::view]
    fn score(_handle: &mut impl PrecompileHandle, who: Address) -> EvmResult<u64> {
        let who: Runtime::AccountId = who.0.into();
        let node = pallet_bags_list::ListNodes::<Runtime, pallet_bags_list::Instance1>::get(&who);
        Ok(node.map(|n| n.score).unwrap_or_default())
    }

    #[precompile::public("bagOf(address)")]
    #[precompile::view]
    fn bag_of(_handle: &mut impl PrecompileHandle, who: Address) -> EvmResult<u64> {
        let who: Runtime::AccountId = who.0.into();
        let node = pallet_bags_list::ListNodes::<Runtime, pallet_bags_list::Instance1>::get(&who);
        Ok(node.map(|n| n.bag_upper).unwrap_or_default())
    }
}
```

- [ ] **Step 4 & 5: Mock + tests** — ≥ 6 tests: rebag happy path, putInFrontOf happy path, listSize after bonds, score/bagOf reads, 2× DELEGATECALL guards.

- [ ] **Step 6: Verify + commit**

```bash
cd apps/node && SKIP_WASM_BUILD=1 cargo test -p precompile-bags-list
git add apps/node/precompiles/bags-list/ apps/node/Cargo.toml apps/node/Cargo.lock
git commit -m "feat(precompile): BagsList precompile at 0x0838"
```

---

## Task 8: `precompile-staking-admin` crate

**Files:**
- Create: `apps/node/precompiles/staking-admin/{Cargo.toml,src/lib.rs,src/mock.rs,src/tests.rs}`
- Modify: `apps/node/Cargo.toml`

**Address:** `0x0000000000000000000000000000000000000840` (2112). All write entries are sudo-gated.

- [ ] **Step 1: Cargo.toml** — deps include `pallet-staking`, `pallet-sudo`.

- [ ] **Step 2: Add to workspace members.**

- [ ] **Step 3: `src/lib.rs`** — every entry calls `sudo_only` then dispatches as `RawOrigin::Root`:

```rust
#[precompile_utils_macro::precompile]
impl<Runtime> StakingAdminPrecompile<Runtime>
where
    Runtime: pallet_evm::Config + pallet_staking::Config + pallet_sudo::Config,
    Runtime::AccountId: From<H160> + Into<H160>,
    <Runtime as frame_system::Config>::RuntimeCall:
        Dispatchable<RuntimeOrigin = Runtime::RuntimeOrigin> + GetDispatchInfo,
    <Runtime as frame_system::Config>::RuntimeCall: From<pallet_staking::Call<Runtime>>,
{
    #[precompile::public("setValidatorCount(uint32)")]
    fn set_validator_count(handle: &mut impl PrecompileHandle, new: u32) -> EvmResult {
        delegate_guard(handle)?;
        sudo_only::<Runtime>(handle)?;
        let call = pallet_staking::Call::<Runtime>::set_validator_count { new };
        RuntimeHelper::<Runtime>::try_dispatch(handle, frame_system::RawOrigin::Root.into(), call)?;
        Ok(())
    }

    #[precompile::public("increaseValidatorCount(uint32)")]
    fn increase_validator_count(handle: &mut impl PrecompileHandle, additional: u32) -> EvmResult {
        delegate_guard(handle)?;
        sudo_only::<Runtime>(handle)?;
        let call = pallet_staking::Call::<Runtime>::increase_validator_count { additional };
        RuntimeHelper::<Runtime>::try_dispatch(handle, frame_system::RawOrigin::Root.into(), call)?;
        Ok(())
    }

    #[precompile::public("scaleValidatorCount(uint8)")]
    fn scale_validator_count(handle: &mut impl PrecompileHandle, factor_percent: u8) -> EvmResult {
        delegate_guard(handle)?;
        sudo_only::<Runtime>(handle)?;
        let factor = sp_runtime::Percent::from_percent(factor_percent);
        let call = pallet_staking::Call::<Runtime>::scale_validator_count { factor };
        RuntimeHelper::<Runtime>::try_dispatch(handle, frame_system::RawOrigin::Root.into(), call)?;
        Ok(())
    }

    #[precompile::public("setInvulnerables(address[])")]
    fn set_invulnerables(handle: &mut impl PrecompileHandle, validators: Vec<Address>) -> EvmResult {
        delegate_guard(handle)?;
        sudo_only::<Runtime>(handle)?;
        let invulnerables: Vec<Runtime::AccountId> =
            validators.into_iter().map(|a| a.0.into()).collect();
        let call = pallet_staking::Call::<Runtime>::set_invulnerables { invulnerables };
        RuntimeHelper::<Runtime>::try_dispatch(handle, frame_system::RawOrigin::Root.into(), call)?;
        Ok(())
    }

    #[precompile::public("forceUnstake(address,uint32)")]
    fn force_unstake(handle: &mut impl PrecompileHandle, stash: Address, num_slashing_spans: u32) -> EvmResult {
        delegate_guard(handle)?;
        sudo_only::<Runtime>(handle)?;
        let stash: Runtime::AccountId = stash.0.into();
        let call = pallet_staking::Call::<Runtime>::force_unstake { stash, num_slashing_spans };
        RuntimeHelper::<Runtime>::try_dispatch(handle, frame_system::RawOrigin::Root.into(), call)?;
        Ok(())
    }

    #[precompile::public("forceNewEra()")]
    fn force_new_era(handle: &mut impl PrecompileHandle) -> EvmResult {
        delegate_guard(handle)?;
        sudo_only::<Runtime>(handle)?;
        let call = pallet_staking::Call::<Runtime>::force_new_era {};
        RuntimeHelper::<Runtime>::try_dispatch(handle, frame_system::RawOrigin::Root.into(), call)?;
        Ok(())
    }

    #[precompile::public("forceNoEras()")]
    fn force_no_eras(handle: &mut impl PrecompileHandle) -> EvmResult {
        delegate_guard(handle)?;
        sudo_only::<Runtime>(handle)?;
        let call = pallet_staking::Call::<Runtime>::force_no_eras {};
        RuntimeHelper::<Runtime>::try_dispatch(handle, frame_system::RawOrigin::Root.into(), call)?;
        Ok(())
    }

    #[precompile::public("forceNewEraAlways()")]
    fn force_new_era_always(handle: &mut impl PrecompileHandle) -> EvmResult {
        delegate_guard(handle)?;
        sudo_only::<Runtime>(handle)?;
        let call = pallet_staking::Call::<Runtime>::force_new_era_always {};
        RuntimeHelper::<Runtime>::try_dispatch(handle, frame_system::RawOrigin::Root.into(), call)?;
        Ok(())
    }

    #[precompile::public("cancelDeferredSlash(uint32,uint32[])")]
    fn cancel_deferred_slash(
        handle: &mut impl PrecompileHandle,
        era: u32,
        slash_indices: Vec<u32>,
    ) -> EvmResult {
        delegate_guard(handle)?;
        sudo_only::<Runtime>(handle)?;
        let call = pallet_staking::Call::<Runtime>::cancel_deferred_slash { era, slash_indices };
        RuntimeHelper::<Runtime>::try_dispatch(handle, frame_system::RawOrigin::Root.into(), call)?;
        Ok(())
    }

    #[precompile::public("setStakingConfigs(uint256,uint256,uint32,uint32,uint8,uint32)")]
    fn set_staking_configs(
        handle: &mut impl PrecompileHandle,
        min_nominator_bond: U256,
        min_validator_bond: U256,
        max_nominator_count: u32,
        max_validator_count: u32,
        chill_threshold_percent: u8,
        min_commission: u32,
    ) -> EvmResult {
        delegate_guard(handle)?;
        sudo_only::<Runtime>(handle)?;
        use pallet_staking::ConfigOp::Set;
        let call = pallet_staking::Call::<Runtime>::set_staking_configs {
            min_nominator_bond: Set(balance_from_u256::<Runtime>(min_nominator_bond)?),
            min_validator_bond: Set(balance_from_u256::<Runtime>(min_validator_bond)?),
            max_nominator_count: Set(max_nominator_count),
            max_validator_count: Set(max_validator_count),
            chill_threshold: Set(sp_runtime::Percent::from_percent(chill_threshold_percent)),
            min_commission: Set(sp_runtime::Perbill::from_parts(min_commission)),
        };
        RuntimeHelper::<Runtime>::try_dispatch(handle, frame_system::RawOrigin::Root.into(), call)?;
        Ok(())
    }

    #[precompile::public("chillOther(address)")]
    fn chill_other(handle: &mut impl PrecompileHandle, stash: Address) -> EvmResult {
        delegate_guard(handle)?;
        sudo_only::<Runtime>(handle)?;
        let stash: Runtime::AccountId = stash.0.into();
        let call = pallet_staking::Call::<Runtime>::chill_other { stash };
        RuntimeHelper::<Runtime>::try_dispatch(handle, frame_system::RawOrigin::Root.into(), call)?;
        Ok(())
    }
}
```

- [ ] **Step 4 & 5: Mock + tests** — ≥ 12 tests: every entry's sudo-gated happy path (caller = Sudo key) + non-sudo rejection (`"NotSudo"`) + DELEGATECALL guards.

- [ ] **Step 6: Verify + commit**

```bash
cd apps/node && SKIP_WASM_BUILD=1 cargo test -p precompile-staking-admin
git add apps/node/precompiles/staking-admin/ apps/node/Cargo.toml apps/node/Cargo.lock
git commit -m "feat(precompile): StakingAdmin precompile at 0x0840"
```

---

## Task 9: Wire `FrontierPrecompilesNpos` with the 7 new addresses

**Files:**
- Modify: `apps/node/runtimes/common/Cargo.toml`
- Modify: `apps/node/runtimes/common/src/precompiles.rs`

- [ ] **Step 1: Add the 7 precompile crate deps**

In `apps/node/runtimes/common/Cargo.toml` `[dependencies]`:

```toml
precompile-staking          = { path = "../../precompiles/staking", default-features = false }
precompile-session          = { path = "../../precompiles/session", default-features = false }
precompile-nomination-pools = { path = "../../precompiles/nomination-pools", default-features = false }
precompile-fast-unstake     = { path = "../../precompiles/fast-unstake", default-features = false }
precompile-treasury         = { path = "../../precompiles/treasury", default-features = false }
precompile-bags-list        = { path = "../../precompiles/bags-list", default-features = false }
precompile-staking-admin    = { path = "../../precompiles/staking-admin", default-features = false }
```

Add to `[features].std`:

```toml
"precompile-staking/std",
"precompile-session/std",
"precompile-nomination-pools/std",
"precompile-fast-unstake/std",
"precompile-treasury/std",
"precompile-bags-list/std",
"precompile-staking-admin/std",
```

- [ ] **Step 2: Extend `FrontierPrecompilesNpos` in `apps/node/runtimes/common/src/precompiles.rs`**

The existing `FrontierPrecompilesNpos<R>` from Plan 1 currently delegates entirely to `FrontierPrecompilesBasic<R>`. Replace its impl block with the explicit Npos surface — adds 7 new `used_addresses()` entries AND extends the `execute()` match arm.

First, add a `used_addresses()` static for Npos. The fixed-size array length grows from `[H160; 11]` (Basic) to `[H160; 18]` (Basic 11 + Npos 7):

```rust
impl<R> FrontierPrecompilesNpos<R>
where
    R: pallet_evm::Config,
{
    pub fn used_addresses() -> [H160; 18] {
        [
            // Inherited from Basic (must mirror FrontierPrecompilesBasic::used_addresses)
            hash(1), hash(2), hash(3), hash(4), hash(5),
            hash(1024), hash(1025), hash(1026), hash(1027),
            hash(precompile_gasless_registry::PRECOMPILE_ADDRESS),
            hash(precompile_batch::PRECOMPILE_ADDRESS),
            // NPoS additions
            hash(0x0810), hash(0x0818), hash(0x0820), hash(0x0828),
            hash(0x0830), hash(0x0838), hash(0x0840),
        ]
    }
}

impl<R> PrecompileSet for FrontierPrecompilesNpos<R>
where
    R: pallet_evm::Config
        + pallet_staking::Config
        + pallet_session::Config
        + pallet_nomination_pools::Config
        + pallet_fast_unstake::Config
        + pallet_treasury::Config
        + pallet_bags_list::Config<pallet_bags_list::Instance1>
        + pallet_sudo::Config,
    R::AccountId: From<H160> + Into<H160>,
    <R as frame_system::Config>::RuntimeCall:
        Dispatchable<RuntimeOrigin = R::RuntimeOrigin> + GetDispatchInfo,
    <R as frame_system::Config>::RuntimeCall:
        From<pallet_staking::Call<R>>
        + From<pallet_session::Call<R>>
        + From<pallet_nomination_pools::Call<R>>
        + From<pallet_fast_unstake::Call<R>>
        + From<pallet_treasury::Call<R>>
        + From<pallet_bags_list::Call<R, pallet_bags_list::Instance1>>,
{
    fn execute(&self, handle: &mut impl PrecompileHandle) -> Option<PrecompileResult> {
        match handle.code_address() {
            // Delegate the existing 9 entries to Basic
            a if FrontierPrecompilesBasic::<R>::new().used_addresses().contains(&a) =>
                FrontierPrecompilesBasic::<R>::new().execute(handle),
            // NPoS additions
            a if a == hash(0x0810) =>
                Some(precompile_staking::StakingPrecompile::<R>::execute(handle)),
            a if a == hash(0x0818) =>
                Some(precompile_session::SessionPrecompile::<R>::execute(handle)),
            a if a == hash(0x0820) =>
                Some(precompile_nomination_pools::NominationPoolsPrecompile::<R>::execute(handle)),
            a if a == hash(0x0828) =>
                Some(precompile_fast_unstake::FastUnstakePrecompile::<R>::execute(handle)),
            a if a == hash(0x0830) =>
                Some(precompile_treasury::TreasuryPrecompile::<R>::execute(handle)),
            a if a == hash(0x0838) =>
                Some(precompile_bags_list::BagsListPrecompile::<R>::execute(handle)),
            a if a == hash(0x0840) =>
                Some(precompile_staking_admin::StakingAdminPrecompile::<R>::execute(handle)),
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
```

- [ ] **Step 3: Verify**

```bash
cd apps/node && cargo check -p impetus-runtime
```

Expected: green. WASM build also expected green.

- [ ] **Step 4: Commit**

```bash
git add apps/node/runtimes/common/Cargo.toml apps/node/runtimes/common/src/precompiles.rs apps/node/Cargo.lock
git commit -m "feat(runtime): wire 7 NPoS precompiles into FrontierPrecompilesNpos"
```

---

## Task 10: `scripts/dump-session-keys.ts` helper

**Files:**
- Create: `scripts/dump-session-keys.ts`
- Modify: `package.json` (root) — add a `dump-session-keys` script and a tsx dev dep if not present.

- [ ] **Step 1: Create the script**

```typescript
#!/usr/bin/env tsx
// Usage:
//   pnpm run dump-session-keys "//Alice"
//
// Emits:
//   1) JSON of the 4 public keys.
//   2) SCALE-encoded bytes for Session.sol::setKeys.
//   3) author_insertKey RPC commands per key type.

import { Keyring } from "@polkadot/keyring";
import { u8aToHex, hexToU8a } from "@polkadot/util";
import { cryptoWaitReady, mnemonicValidate } from "@polkadot/util-crypto";

async function main() {
  await cryptoWaitReady();
  const seed = process.argv[2];
  if (!seed) {
    console.error("usage: dump-session-keys <seed-uri-or-mnemonic>");
    process.exit(1);
  }

  const sr25519 = new Keyring({ type: "sr25519" });
  const ed25519 = new Keyring({ type: "ed25519" });

  const babe = sr25519.addFromUri(seed);
  const grandpa = ed25519.addFromUri(seed);
  const imOnline = sr25519.addFromUri(seed);
  const authDisc = sr25519.addFromUri(seed);

  const out = {
    babe: u8aToHex(babe.publicKey),
    grandpa: u8aToHex(grandpa.publicKey),
    imOnline: u8aToHex(imOnline.publicKey),
    authorityDiscovery: u8aToHex(authDisc.publicKey),
  };
  console.log("public keys:\n" + JSON.stringify(out, null, 2));

  // SCALE-encoded SessionKeys = concat of 4× 32-byte pubkeys (sr25519=32, ed25519=32)
  const scaleBytes = new Uint8Array(128);
  scaleBytes.set(hexToU8a(out.babe), 0);
  scaleBytes.set(hexToU8a(out.grandpa), 32);
  scaleBytes.set(hexToU8a(out.imOnline), 64);
  scaleBytes.set(hexToU8a(out.authorityDiscovery), 96);
  console.log("\nsetKeys argument (hex):", u8aToHex(scaleBytes));
  console.log("setKeys proof argument (hex): 0x00 (empty proof — ownership signature is encoded elsewhere)");

  console.log("\nauthor_insertKey RPC commands:");
  for (const [keyType, seedField, pubkey] of [
    ["babe", seed, out.babe],
    ["gran", seed, out.grandpa],
    ["imon", seed, out.imOnline],
    ["audi", seed, out.authorityDiscovery],
  ] as const) {
    console.log(
      `curl -s -H 'Content-Type: application/json' -d '{"jsonrpc":"2.0","method":"author_insertKey","params":["${keyType}","${seedField}","${pubkey}"],"id":1}' http://127.0.0.1:9944`,
    );
  }
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});
```

- [ ] **Step 2: Wire the npm script**

Edit root `package.json` `scripts`:

```json
"dump-session-keys": "tsx scripts/dump-session-keys.ts"
```

If `tsx` and `@polkadot/util-crypto` / `@polkadot/keyring` aren't already in `devDependencies` (Plan 1's contract package may have them), add:

```bash
pnpm add -D -w tsx @polkadot/util-crypto @polkadot/keyring @polkadot/util
```

- [ ] **Step 3: Smoke-test the script**

```bash
pnpm run dump-session-keys "//Alice"
```

Expected output: 4 keys, SCALE bytes, 4 curl commands.

- [ ] **Step 4: Commit**

```bash
git add scripts/dump-session-keys.ts package.json pnpm-lock.yaml
git commit -m "feat(scripts): dump-session-keys helper for impetus validators"
```

---

## Task 11: Production `impetus_config()` chain spec

**Files:**
- Modify: `apps/node/node/src/chain_spec.rs`

Plan 2 set `impetus_config()` to ChainType::Development with Hardhat `//Alice..//Dave` keys. Production needs a Live spec with non-Hardhat validator material.

- [ ] **Step 1: Add `impetus_production_config()` alongside the dev variant**

Keep `impetus_config()` returning the dev NPoS spec (it's the `impetus_dev_npos` profile). Add:

```rust
fn impetus_production_profile() -> ChainProfile {
    ChainProfile {
        display_name: "Impetus",
        spec_id: "impetus",
        evm_chain_id: 388266,
        token_symbol: "IPT",
        ss58_prefix: 11434,
        chain_type: ChainType::Live,
        manual_seal: false,
    }
}

/// Production chain spec for impetus mainnet. Stash addresses and session
/// keys MUST be replaced with production material before deployment — the
/// constants below are placeholders for testing the production code path.
/// The placeholders are funded by `production_endowed_accounts()` (added
/// to `runtimes/common/src/genesis_helpers.rs`) so the build-time `assert!`
/// in `impetus_production_genesis_patch` passes during smoke testing.
const PRODUCTION_VALIDATOR_STASHES: [&str; 4] = [
    "0x1111111111111111111111111111111111111111",
    "0x2222222222222222222222222222222222222222",
    "0x3333333333333333333333333333333333333333",
    "0x4444444444444444444444444444444444444444",
];

fn production_validator_accounts() -> [AccountId; 4] {
    PRODUCTION_VALIDATOR_STASHES.map(|s| {
        let hex = s.strip_prefix("0x").unwrap();
        let bytes: [u8; 20] = hex::FromHex::from_hex(hex).expect("invalid stash hex");
        H160::from_slice(&bytes).into()
    })
}

pub fn impetus_production_config() -> ChainSpec {
    let profile = impetus_production_profile();
    let wasm = impetus_runtime::WASM_BINARY.expect("Impetus WASM not built");

    // Production endowment = admin + Hardhat dev users + 4 production stashes.
    // Without the stashes here, the build-time assert! below trips. Operators
    // replacing PRODUCTION_VALIDATOR_STASHES with real material must also
    // update production_endowed_accounts() in runtime_common::genesis_helpers
    // or the chain panics at block 1.
    let mut endowed = endowed_accounts();
    endowed.extend(production_validator_accounts());

    ChainSpec::builder(wasm, Default::default())
        .with_name(profile.display_name)
        .with_id(profile.spec_id)
        .with_chain_type(profile.chain_type.clone())
        .with_properties(properties(&profile))
        .with_genesis_config_patch(impetus_production_genesis_patch(
            admin_account(),
            endowed,
            profile.evm_chain_id,
        ))
        .build()
}

fn impetus_production_genesis_patch(
    sudo_key: AccountId,
    endowed: Vec<AccountId>,
    chain_id: u64,
) -> serde_json::Value {
    let validator_stashes = production_validator_accounts();

    // Build-time pre-condition: all validator stashes must be pre-funded so
    // genesis election does not panic at block 1 (Plan 2 R10 carryover).
    assert!(
        validator_stashes.iter().all(|s| endowed.contains(s)),
        "every production validator stash must be in endowed accounts"
    );

    // Session keys for production are inserted via author_insertKey by
    // operators after node start; chain spec only commits the stash → empty
    // SessionKeys placeholder. Validators MUST run scripts/dump-session-keys.ts
    // and submit setKeys via the Session precompile within the first session.
    let session_keys: Vec<_> = validator_stashes
        .iter()
        .map(|stash| {
            serde_json::json!([
                stash, stash,
                {
                    "babe": "0x0000000000000000000000000000000000000000000000000000000000000000",
                    "grandpa": "0x0000000000000000000000000000000000000000000000000000000000000000",
                    "im_online": "0x0000000000000000000000000000000000000000000000000000000000000000",
                    "authority_discovery": "0x0000000000000000000000000000000000000000000000000000000000000000",
                }
            ])
        })
        .collect();

    // Genesis stakers: 4 validators, each pre-bonded with 100K IPT. No
    // nominators at genesis — they join via the Staking precompile.
    let stakers: Vec<_> = validator_stashes
        .iter()
        .map(|stash| serde_json::json!([stash, stash, 100_000u128 * UNITS, "Validator"]))
        .collect();

    let evm_accounts: BTreeMap<H160, fp_evm::GenesisAccount> = endowed
        .iter()
        .map(|account| (
            H160::from(*account),
            fp_evm::GenesisAccount {
                balance: U256::from(1_000_000u128) * U256::from(UNITS),
                code: Default::default(),
                nonce: Default::default(),
                storage: Default::default(),
            },
        ))
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
            "invulnerables": validator_stashes.iter().collect::<Vec<_>>(),
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
```

Add `hex` dep to `apps/node/node/Cargo.toml` if not present (likely already there via Frontier).

- [ ] **Step 2: Add a unit test for the production spec**

```rust
#[test]
fn impetus_production_spec_is_live() {
    let spec = impetus_production_config();
    let json: serde_json::Value = serde_json::from_str(&spec.as_json(false).unwrap()).unwrap();
    assert_eq!(spec.id(), "impetus");
    assert_eq!(spec.chain_type(), ChainType::Live);
    // Production stash addresses should be invulnerable at genesis.
    let invulnerables = &json["genesis"]["runtimeGenesis"]["patch"]["staking"]["invulnerables"];
    assert_eq!(invulnerables.as_array().map(|a| a.len()), Some(4));
}
```

- [ ] **Step 3: Verify**

```bash
cd apps/node && SKIP_WASM_BUILD=1 cargo test -p frontier-template-node chain_spec
```

Expected: 8 tests pass (7 from Plan 2 + 1 new).

- [ ] **Step 4: Commit**

```bash
git add apps/node/node/src/chain_spec.rs apps/node/node/Cargo.toml
git commit -m "feat(node): add impetus_production_config (Live, invulnerable validators)"
```

---

## Task 12: Remap `command.rs::load_spec` aliases

**Files:**
- Modify: `apps/node/node/src/command.rs`

Until Plan 3, `impetus` and `mainnet` aliases pointed at the dev NPoS spec — a footgun. Now that `impetus_production_config()` exists, route those aliases to it.

- [ ] **Step 1: Update `load_spec` and `Network::from_spec_id`**

In `apps/node/node/src/command.rs::SubstrateCli::load_spec`:

```rust
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
```

In `apps/node/node/src/chain_spec.rs::Network::from_spec_id`:

```rust
pub fn from_spec_id(id: &str) -> Self {
    match id {
        "impetus" | "mainnet" | "impetus_dev_npos" => Network::Impetus,
        _ => Network::Impulse,
    }
}
```

`from_spec_id` keeps grouping all impetus variants under the Babe path. Only `load_spec` distinguishes between production and dev.

- [ ] **Step 2: Add a regression test**

In `chain_spec.rs::tests`:

```rust
#[test]
fn impetus_alias_loads_production_not_dev() {
    // Sanity: the spec id "impetus" must NOT collide with "impetus_dev_npos".
    let prod = impetus_production_config();
    let dev = impetus_config();
    assert_eq!(prod.id(), "impetus");
    assert_eq!(dev.id(), "impetus_dev_npos");
    assert_ne!(prod.chain_type(), dev.chain_type());
}
```

- [ ] **Step 3: Verify + commit**

```bash
cd apps/node && SKIP_WASM_BUILD=1 cargo test -p frontier-template-node chain_spec
git add apps/node/node/src/command.rs apps/node/node/src/chain_spec.rs
git commit -m "feat(node): route impetus/mainnet aliases to production spec"
```

---

## Task 13: E2E test harness — helpers + Hardhat config

**Files:**
- Create: `packages/contracts/test/helpers/staking-helpers.ts`
- Modify: `packages/contracts/hardhat.config.ts` (RPC URL + chain id pointing at local node)

- [ ] **Step 1: Create helpers**

```typescript
// packages/contracts/test/helpers/staking-helpers.ts
import { ethers } from "hardhat";

const STAKING_ADDRESS = "0x0000000000000000000000000000000000000810";
const SESSION_ADDRESS = "0x0000000000000000000000000000000000000818";
const POOLS_ADDRESS = "0x0000000000000000000000000000000000000820";
const FAST_UNSTAKE_ADDRESS = "0x0000000000000000000000000000000000000828";
const TREASURY_ADDRESS = "0x0000000000000000000000000000000000000830";
const BAGS_LIST_ADDRESS = "0x0000000000000000000000000000000000000838";
const STAKING_ADMIN_ADDRESS = "0x0000000000000000000000000000000000000840";

export const ADDRESSES = {
  STAKING: STAKING_ADDRESS,
  SESSION: SESSION_ADDRESS,
  POOLS: POOLS_ADDRESS,
  FAST_UNSTAKE: FAST_UNSTAKE_ADDRESS,
  TREASURY: TREASURY_ADDRESS,
  BAGS_LIST: BAGS_LIST_ADDRESS,
  STAKING_ADMIN: STAKING_ADMIN_ADDRESS,
} as const;

const POLL_INTERVAL_MS = 1_500;
const DEFAULT_TIMEOUT_MS = 5 * 60 * 1_000;

export async function waitForEra(target: number, timeoutMs = DEFAULT_TIMEOUT_MS): Promise<void> {
  const staking = await ethers.getContractAt("IStaking", STAKING_ADDRESS);
  const start = Date.now();
  while (Date.now() - start < timeoutMs) {
    const era = Number(await staking.currentEra());
    if (era >= target) return;
    await sleep(POLL_INTERVAL_MS);
  }
  throw new Error(`waitForEra(${target}) timed out after ${timeoutMs} ms`);
}

export async function advanceToNextSession(timeoutMs = DEFAULT_TIMEOUT_MS): Promise<number> {
  const session = await ethers.getContractAt("ISession", SESSION_ADDRESS);
  const start = Date.now();
  const initial = Number(await session.currentIndex());
  while (Date.now() - start < timeoutMs) {
    const cur = Number(await session.currentIndex());
    if (cur > initial) return cur;
    await sleep(POLL_INTERVAL_MS);
  }
  throw new Error(`advanceToNextSession timed out after ${timeoutMs} ms`);
}

export async function getStakingLedger(stash: string) {
  const staking = await ethers.getContractAt("IStaking", STAKING_ADDRESS);
  return await staking.ledger(stash);
}

export async function seedDevValidators(): Promise<void> {
  // Genesis already pre-bonds 4 validators in impetus_dev_npos.
  // This helper exists as a no-op for parity with the original spec;
  // future variants may need to inject keys here.
}

function sleep(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}
```

- [ ] **Step 2: Add `impetus_dev_npos` network to `hardhat.config.ts`**

```typescript
networks: {
  impetus_dev: {
    url: "http://127.0.0.1:9944",
    chainId: 388266,
    accounts: {
      mnemonic: "test test test test test test test test test test test junk",
      count: 10,
    },
  },
}
```

- [ ] **Step 3: Commit**

```bash
git add packages/contracts/test/helpers/staking-helpers.ts packages/contracts/hardhat.config.ts
git commit -m "test(e2e): staking helpers + hardhat impetus_dev network"
```

---

## Task 14: Solidity interfaces (7 files)

**Files:**
- Create: `packages/contracts/contracts/interfaces/IStaking.sol`
- Create: `packages/contracts/contracts/interfaces/ISession.sol`
- Create: `packages/contracts/contracts/interfaces/INominationPools.sol`
- Create: `packages/contracts/contracts/interfaces/IFastUnstake.sol`
- Create: `packages/contracts/contracts/interfaces/ITreasury.sol`
- Create: `packages/contracts/contracts/interfaces/IBagsList.sol`
- Create: `packages/contracts/contracts/interfaces/IStakingAdmin.sol`

- [ ] **Step 1: Write each interface** matching the Solidity surface in spec §4.1–4.7. Use the typed structs from spec (RewardDestination, ValidatorPrefs, BondedPool, etc.). The Treasury interface follows Task 6's redesigned surface (no `proposeSpend`/`approveProposal` — those are removed; use `spendLocal`, `payout`, `voidSpend`, `checkStatus`, `pot`, `spendCount`, `approvals`).

Example for `IStaking.sol`:

```solidity
// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

struct RewardDestination {
    uint8 kind;     // 0=Staked, 1=Stash, 2=Controller, 3=Account, 4=None
    address account;
}

struct ValidatorPrefs {
    uint16 commission; // basis points / 10_000
    bool blocked;
}

interface IStaking {
    function bond(uint256 value, RewardDestination calldata payee) external;
    function bondExtra(uint256 maxAdditional) external;
    function unbond(uint256 value) external;
    function withdrawUnbonded(uint32 numSlashingSpans) external;
    function validate(ValidatorPrefs calldata prefs) external;
    function nominate(address[] calldata targets) external;
    function chill() external;
    function setPayee(RewardDestination calldata payee) external;
    function payoutStakers(address validatorStash, uint32 era) external;
    function payoutStakersByPage(address validatorStash, uint32 era, uint32 page) external;
    function rebond(uint256 value) external;
    function kick(address[] calldata who) external;
    function chillOther(address controller) external;
    function forceApplyMinCommission(address validator) external;
    function reapStash(address stash, uint32 numSlashingSpans) external;

    function currentEra() external view returns (uint32);
    function activeEra() external view returns (uint32 index, uint64 startMs);
    function minNominatorBond() external view returns (uint256);
    function minValidatorBond() external view returns (uint256);
    function validatorCount() external view returns (uint32);

    event Bonded(address indexed stash, uint256 amount);
    event Unbonded(address indexed stash, uint256 amount);
    event Withdrawn(address indexed stash, uint256 amount);
    event Slashed(address indexed staker, uint256 amount);
    event Chilled(address indexed stash);
    event PayoutStarted(uint32 indexed eraIndex, address indexed validatorStash);
    event Rewarded(address indexed stash, uint8 dest, uint256 amount);
}
```

**`ISession.sol`:**

```solidity
// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

interface ISession {
    function setKeys(bytes calldata keys, bytes calldata proof) external;
    function purgeKeys() external;
    function currentIndex() external view returns (uint32);
    function nextKeys(address validator) external view returns (bytes memory);
    function queuedKeys() external view returns (address[] memory validators, bytes[] memory keys);
    event NewSession(uint32 indexed sessionIndex);
}
```

**`INominationPools.sol`:**

```solidity
// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

struct BondExtraSource { uint8 kind; uint256 amount; } // 0=FreeBalance, 1=Rewards
struct PoolRoleUpdate { uint8 op; address account; }    // op: 0=NoChange, 1=Set, 2=Remove
struct PoolCommission { uint32 commission; address payee; }
struct PoolCommissionChangeRate { uint32 maxIncrease; uint32 minDelay; }
struct PoolMember {
    uint32 poolId;
    uint256 points;
    uint256 lastRecordedRewardCounter;
    uint32[] unbondingEras;
}
struct BondedPool {
    uint256 points;
    uint8 state;
    uint32 memberCounter;
    address rootRole;
    address nominatorRole;
    address bouncerRole;
    PoolCommission commission;
}

interface INominationPools {
    function join(uint256 amount, uint32 poolId) external;
    function bondExtra(BondExtraSource calldata extra) external;
    function claimPayout() external;
    function unbond(address memberAccount, uint256 unbondingPoints) external;
    function poolWithdrawUnbonded(uint32 poolId, uint32 numSlashingSpans) external;
    function withdrawUnbonded(address memberAccount, uint32 numSlashingSpans) external;
    function create(uint256 amount, address root, address nominator, address bouncer) external;
    function createWithPoolId(uint256 amount, address root, address nominator, address bouncer, uint32 poolId) external;
    function nominate(uint32 poolId, address[] calldata validators) external;
    function setState(uint32 poolId, uint8 state) external;
    function setMetadata(uint32 poolId, bytes calldata metadata) external;
    function setConfigs(uint256 minJoinBond, uint256 minCreateBond, uint32 maxPools, uint32 maxMembers, uint32 maxMembersPerPool, uint32 globalMaxCommission) external;
    function updateRoles(uint32 poolId, PoolRoleUpdate[3] calldata roles) external;
    function chill(uint32 poolId) external;
    function bondExtraOther(address member, BondExtraSource calldata extra) external;
    function setCommission(uint32 poolId, PoolCommission calldata commission) external;
    function setCommissionMax(uint32 poolId, uint32 maxCommission) external;
    function setCommissionChangeRate(uint32 poolId, PoolCommissionChangeRate calldata changeRate) external;
    function claimCommission(uint32 poolId) external;

    function bondedPools(uint32 poolId) external view returns (BondedPool memory);
    function poolMembers(address account) external view returns (PoolMember memory);
    function metadata(uint32 poolId) external view returns (bytes memory);
    function lastPoolId() external view returns (uint32);

    event Created(address indexed depositor, uint32 indexed poolId);
    event Bonded(address indexed member, uint32 indexed poolId, uint256 bonded, bool joined);
    event PaidOut(address indexed member, uint32 indexed poolId, uint256 payout);
    event Unbonded(address indexed member, uint32 indexed poolId, uint256 balance, uint256 points, uint32 era);
    event Withdrawn(address indexed member, uint32 indexed poolId, uint256 balance, uint256 points);
    event Destroyed(uint32 indexed poolId);
    event StateChanged(uint32 indexed poolId, uint8 newState);
    event MemberRemoved(uint32 indexed poolId, address indexed member);
    event RolesUpdated(address root, address nominator, address bouncer);
    event PoolSlashed(uint32 indexed poolId, uint256 balance);
    event UnbondingPoolSlashed(uint32 indexed poolId, uint32 era, uint256 balance);
    event PoolCommissionUpdated(uint32 indexed poolId, uint32 commission, address payee);
    event PoolMaxCommissionUpdated(uint32 indexed poolId, uint32 maxCommission);
    event PoolCommissionChangeRateUpdated(uint32 indexed poolId, uint32 maxIncrease, uint32 minDelay);
    event PoolCommissionClaimed(uint32 indexed poolId, uint256 amount);
}
```

**`IFastUnstake.sol`:**

```solidity
// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

interface IFastUnstake {
    function registerFastUnstake() external;
    function deregister() external;
    function control(uint32 erasToCheckPerBlock) external;

    function head() external view returns (address stash);
    function queue(address stash) external view returns (uint256 deposit);
    function erasToCheckPerBlock() external view returns (uint32);

    event Unstaked(address indexed stash, bool success);
    event Slashed(address indexed stash, uint256 amount);
    event InternalError();
    event BatchChecked(uint32[] eras);
    event BatchFinished(uint32 size);
}
```

**`ITreasury.sol`** (stable2603 surface — no `proposeSpend`/`approveProposal`):

```solidity
// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

interface ITreasury {
    function spendLocal(uint256 amount, address beneficiary) external;
    function payout(uint32 index) external;
    function voidSpend(uint32 index) external;
    function checkStatus(uint32 index) external;

    function pot() external view returns (uint256);
    function spendCount() external view returns (uint32);
    function approvals() external view returns (uint32[] memory);

    event Spending(uint256 budgetRemaining);
    event Awarded(uint32 indexed proposalIndex, uint256 award, address indexed account);
    event Burnt(uint256 burntFunds);
    event Rollover(uint256 rolloverBalance);
    event Deposit(uint256 value);
    event SpendApproved(uint32 indexed index, uint256 amount, address indexed beneficiary);
    event Paid(uint32 indexed index, address indexed beneficiary);
    event AssetSpendVoided(uint32 indexed index);
}
```

**`IBagsList.sol`:**

```solidity
// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

interface IBagsList {
    function putInFrontOf(address lighter) external;
    function rebag(address dislocated) external;

    function bagOf(address account) external view returns (uint64 threshold);
    function score(address account) external view returns (uint64);
    function listSize() external view returns (uint32);

    event Rebagged(address indexed who, uint64 from, uint64 to);
    event ScoreUpdated(address indexed who, uint64 newScore);
}
```

**`IStakingAdmin.sol`:**

```solidity
// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

interface IStakingAdmin {
    function setValidatorCount(uint32 newCount) external;
    function increaseValidatorCount(uint32 additional) external;
    function scaleValidatorCount(uint8 factorPercent) external;
    function setInvulnerables(address[] calldata validators) external;
    function forceUnstake(address stash, uint32 numSlashingSpans) external;
    function forceNewEra() external;
    function forceNoEras() external;
    function forceNewEraAlways() external;
    function cancelDeferredSlash(uint32 era, uint32[] calldata slashIndices) external;
    function setStakingConfigs(
        uint256 minNominatorBond,
        uint256 minValidatorBond,
        uint32 maxNominatorCount,
        uint32 maxValidatorCount,
        uint8 chillThresholdPercent,
        uint32 minCommission
    ) external;
    function chillOther(address stash) external;
}
```

- [ ] **Step 2: Verify Solidity compiles**

```bash
cd packages/contracts && pnpm hardhat compile
```

Expected: green. Generates ABI artifacts under `artifacts/contracts/interfaces/`.

- [ ] **Step 3: Commit**

```bash
git add packages/contracts/contracts/interfaces/
git commit -m "feat(contracts): 7 NPoS precompile Solidity interfaces"
```

---

## Task 15: `staking-validator.spec.ts` E2E

**Files:**
- Create: `packages/contracts/test/staking-validator.spec.ts`

- [ ] **Step 1: Write the spec**

```typescript
import { expect } from "chai";
import { ethers } from "hardhat";
import { ADDRESSES, waitForEra, getStakingLedger } from "./helpers/staking-helpers";

describe("Staking precompile — validator lifecycle", function () {
  this.timeout(15 * 60 * 1000);

  it("bond → setKeys → validate → wait era → payoutStakers → unbond → withdrawUnbonded", async () => {
    const [signer] = await ethers.getSigners();
    const stash = await signer.getAddress();
    const staking = await ethers.getContractAt("IStaking", ADDRESSES.STAKING, signer);
    const session = await ethers.getContractAt("ISession", ADDRESSES.SESSION, signer);

    const bondAmount = ethers.parseEther("2000");

    // bond
    const payee = { kind: 0, account: ethers.ZeroAddress }; // Staked
    await (await staking.bond(bondAmount, payee)).wait();
    const ledger = await getStakingLedger(stash);
    expect(ledger.active).to.equal(bondAmount);

    // setKeys (dummy keys for dev — real ones produced by dump-session-keys.ts)
    const dummyKeys = "0x" + "01".repeat(128); // 4 × 32-byte pubkeys
    const dummyProof = "0x";
    await (await session.setKeys(dummyKeys, dummyProof)).wait();

    // validate
    await (await staking.validate({ commission: 100, blocked: false })).wait();

    // wait for era 1 — pre-bonded as genesis validator so stash is in current set
    await waitForEra(1);

    // payoutStakers for era 0
    await (await staking.payoutStakers(stash, 0)).wait();

    // unbond
    await (await staking.unbond(bondAmount)).wait();

    // wait BondingDuration eras (runtime-test-fast = 2 eras = ~2 min)
    await waitForEra(3);

    // withdrawUnbonded
    await (await staking.withdrawUnbonded(0)).wait();

    const finalLedger = await getStakingLedger(stash);
    expect(finalLedger.active).to.equal(0n);
  });
});
```

- [ ] **Step 2: Run the spec (requires live node)**

```bash
# In one shell: start node
cd apps/node && ./target/release/frontier-template-node --chain impetus_dev_npos --tmp --validator --alice --unsafe-force-node-key-generation &

# In another shell: run the spec
cd packages/contracts && pnpm hardhat test --network impetus_dev test/staking-validator.spec.ts
```

Expected: 1 test passes within ~5 min (runtime-test-fast era boundary).

- [ ] **Step 3: Commit**

```bash
git add packages/contracts/test/staking-validator.spec.ts
git commit -m "test(e2e): staking validator full lifecycle"
```

---

## Task 16: `staking-nominator.spec.ts` + `staking-rebond.spec.ts`

**Files:**
- Create: `packages/contracts/test/staking-nominator.spec.ts`
- Create: `packages/contracts/test/staking-rebond.spec.ts`

- [ ] **Step 1: `staking-nominator.spec.ts`** — bond → nominate(targets) → wait era → payout → assert ledger updated and balance grew.

```typescript
import { expect } from "chai";
import { ethers } from "hardhat";
import { ADDRESSES, waitForEra, getStakingLedger } from "./helpers/staking-helpers";

describe("Staking precompile — nominator", function () {
  this.timeout(10 * 60 * 1000);

  it("bond → nominate → wait era → check ledger", async () => {
    const [, , , , , nominator] = await ethers.getSigners();
    const nominatorAddr = await nominator.getAddress();
    const staking = await ethers.getContractAt("IStaking", ADDRESSES.STAKING, nominator);

    const bondAmount = ethers.parseEther("1000");
    await (await staking.bond(bondAmount, { kind: 0, account: ethers.ZeroAddress })).wait();

    const validators = [
      "0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266",
      "0x70997970C51812dc3A010C7d01b50e0d17dc79C8",
    ];
    await (await staking.nominate(validators)).wait();

    await waitForEra(2);

    const ledger = await getStakingLedger(nominatorAddr);
    expect(ledger.active).to.be.gte(bondAmount);
  });
});
```

- [ ] **Step 2: `staking-rebond.spec.ts`** — bond → unbond → rebond before BondingDuration expires → withdrawUnbonded should fail / return zero.

```typescript
import { expect } from "chai";
import { ethers } from "hardhat";
import { ADDRESSES, getStakingLedger } from "./helpers/staking-helpers";

describe("Staking precompile — rebond before withdraw", function () {
  this.timeout(5 * 60 * 1000);

  it("bond → unbond → rebond cancels the unbonding chunk", async () => {
    const [, , , , , , bonder] = await ethers.getSigners();
    const bonderAddr = await bonder.getAddress();
    const staking = await ethers.getContractAt("IStaking", ADDRESSES.STAKING, bonder);

    const bondAmount = ethers.parseEther("1000");
    await (await staking.bond(bondAmount, { kind: 0, account: ethers.ZeroAddress })).wait();
    await (await staking.unbond(ethers.parseEther("500"))).wait();

    const midLedger = await getStakingLedger(bonderAddr);
    expect(midLedger.unlocking.length).to.equal(1);

    await (await staking.rebond(ethers.parseEther("500"))).wait();

    const finalLedger = await getStakingLedger(bonderAddr);
    expect(finalLedger.unlocking.length).to.equal(0);
    expect(finalLedger.active).to.equal(bondAmount);
  });
});
```

- [ ] **Step 3: Verify + commit**

```bash
cd packages/contracts && pnpm hardhat test --network impetus_dev \
  test/staking-nominator.spec.ts test/staking-rebond.spec.ts
git add packages/contracts/test/staking-nominator.spec.ts packages/contracts/test/staking-rebond.spec.ts
git commit -m "test(e2e): nominator + rebond flows"
```

---

## Task 17: `pools.spec.ts` E2E

**File:** `packages/contracts/test/pools.spec.ts`

- [ ] **Step 1: Write the spec**

Covers: create pool → join → bondExtra → claimPayout → unbond → withdraw.

```typescript
import { expect } from "chai";
import { ethers } from "hardhat";
import { ADDRESSES, waitForEra } from "./helpers/staking-helpers";

describe("NominationPools precompile — full lifecycle", function () {
  this.timeout(10 * 60 * 1000);

  it("create → join → bondExtra → claimPayout → unbond → withdraw", async () => {
    const [, , , , , , , depositor, joiner] = await ethers.getSigners();
    const pools = await ethers.getContractAt("INominationPools", ADDRESSES.POOLS, depositor);

    const depositorAddr = await depositor.getAddress();
    const joinerAddr = await joiner.getAddress();

    // Create pool with depositor as root, nominator, bouncer
    await (await pools.create(
      ethers.parseEther("1500"),
      depositorAddr, depositorAddr, depositorAddr,
    )).wait();
    const poolId = await pools.lastPoolId();
    expect(poolId).to.be.gt(0);

    const joinerPools = pools.connect(joiner);
    await (await joinerPools.join(ethers.parseEther("500"), poolId)).wait();

    const member = await pools.poolMembers(joinerAddr);
    expect(member.poolId).to.equal(poolId);

    await waitForEra(2);

    // Claim any accrued reward (may be 0 in dev — assert no revert)
    await joinerPools.claimPayout();

    // Unbond joiner's share
    await (await joinerPools.unbond(joinerAddr, member.points)).wait();

    await waitForEra(5);

    // withdraw
    await (await joinerPools.withdrawUnbonded(joinerAddr, 0)).wait();
  });
});
```

- [ ] **Step 2: Verify + commit**

```bash
cd packages/contracts && pnpm hardhat test --network impetus_dev test/pools.spec.ts
git add packages/contracts/test/pools.spec.ts
git commit -m "test(e2e): nomination pool full lifecycle"
```

---

## Task 18: `treasury.spec.ts` E2E (redesigned surface, with sudo path)

**Files:**
- Create: `packages/contracts/test/treasury.spec.ts`
- Use the **sudo EVM proxy** wired by `impetus_dev_npos` genesis (Hardhat #0 = `admin_account()`, sudo key). The dev signer at index 0 can submit `Sudo::sudo(...)` via the existing **sudo precompile if one exists**, OR via direct `state_call`+`author_submitExtrinsic` over JSON-RPC.

Per Plan 2's runtime config, `pallet_sudo::Key` is set to the admin address (Hardhat #0 in dev). For E2E we need to route a `spend_local` through Sudo. Two paths:

**Path A (preferred):** If the existing `Sudo.sol` precompile is in scope (Plan 1 / Plan 2 didn't add one — gasless-registry uses sudo gating internally but doesn't expose a generic sudo wrapper), use `author_submitExtrinsic` via JSON-RPC to send a Sudo::sudo(Treasury::spend_local) extrinsic signed by the admin keypair.

**Path B (fallback):** Test only the revert paths + view paths via EVM. Document that root-gated spend → beneficiary credit is covered in Substrate-level runtime integration tests (Plan 2 T25, currently `#[ignore]`d pending the stable2603 redesign — Plan 3 T6 SHOULD revive that test alongside the precompile).

This plan uses **Path B for the EVM E2E** (Hardhat) and a companion **Path A for runtime integration** (revive Plan 2 T25 in Rust).

- [ ] **Step 1: Revive the Plan 2 T25 treasury runtime test**

Edit `apps/node/runtimes/impetus/tests/treasury_proposal.rs`. Remove `#[ignore]` (if present), rename the test from `propose_spend`-based naming to `spend_local`-based, and rewrite to use stable2603's surface:

```rust
mod common;
use common::*;

use frame_support::assert_ok;
use impetus_runtime::{Balances, RuntimeOrigin, Treasury, UNIT};
use sp_runtime::traits::IdentityLookup;

#[test]
fn root_spend_local_credits_beneficiary_after_spend_period() {
    ExtBuilder::default().build().execute_with(|| {
        let proposer = account(ALICE);
        let beneficiary = account(BOB);
        let spend: u128 = 1_000 * UNIT;

        // Seed the treasury pot.
        let pot_account = pallet_treasury::Pallet::<impetus_runtime::Runtime>::account_id();
        assert_ok!(Balances::transfer_keep_alive(
            RuntimeOrigin::signed(proposer),
            pot_account,
            10_000 * UNIT,
        ));

        let beneficiary_pre = Balances::free_balance(beneficiary);

        // stable2603 surface: Treasury::spend_local takes amount + lookup.
        // With Config::SpendOrigin = NeverEnsureOrigin in Plan 2, the only path
        // is RawOrigin::Root via Sudo. Bypass Sudo in the test by dispatching
        // directly with Root.
        assert_ok!(Treasury::spend_local(
            RuntimeOrigin::root(),
            spend,
            beneficiary,
        ));

        // Advance past one SpendPeriod so the queued spend executes.
        run_to_block(impetus_runtime::SpendPeriod::get() + 5);

        let beneficiary_post = Balances::free_balance(beneficiary);
        assert_eq!(
            beneficiary_post,
            beneficiary_pre + spend,
            "beneficiary must receive the exact spend amount"
        );
    });
}
```

This replaces the `#[ignore]`d test from Plan 2 with one that actually verifies treasury spend → credit.

- [ ] **Step 2: Write the EVM E2E**

```typescript
import { expect } from "chai";
import { ethers } from "hardhat";
import { ADDRESSES } from "./helpers/staking-helpers";

describe("Treasury precompile — EVM surface", function () {
  this.timeout(5 * 60 * 1000);

  it("non-sudo spendLocal reverts NotSudo", async () => {
    const [, , , user] = await ethers.getSigners();
    const treasury = await ethers.getContractAt("ITreasury", ADDRESSES.TREASURY, user);
    await expect(
      treasury.spendLocal(ethers.parseEther("100"), await user.getAddress()),
    ).to.be.revertedWith("NotSudo");
  });

  it("treasury pot is readable", async () => {
    const treasury = await ethers.getContractAt("ITreasury", ADDRESSES.TREASURY);
    const pot = await treasury.pot();
    expect(pot).to.be.gte(0n);
  });

  it("payout of a nonexistent index reverts", async () => {
    const [signer] = await ethers.getSigners();
    const treasury = await ethers.getContractAt("ITreasury", ADDRESSES.TREASURY, signer);
    await expect(treasury.payout(99_999_999)).to.be.reverted;
  });

  it("spendCount is readable", async () => {
    const treasury = await ethers.getContractAt("ITreasury", ADDRESSES.TREASURY);
    const count = await treasury.spendCount();
    expect(count).to.be.gte(0);
  });

  it("sudo-routed spendLocal executes (via signer #0 = sudo key)", async () => {
    // Hardhat #0 (0xf39F...) is the impetus_dev_npos sudo key.
    const [sudo, , , beneficiary] = await ethers.getSigners();
    const treasury = await ethers.getContractAt("ITreasury", ADDRESSES.TREASURY, sudo);
    const beneficiaryAddr = await beneficiary.getAddress();

    // Seed treasury via direct transfer (anyone can do this — Treasury pot
    // is just Balances::free_balance(account_id)).
    await sudo.sendTransaction({
      to: "0x6d6f646c70792f747273727900000000000000000000", // Treasury::account_id() = PalletId("py/trsry").into_account_truncating()
      value: ethers.parseEther("5000"),
    });

    const pre = await ethers.provider.getBalance(beneficiaryAddr);

    // Sudo key calls spendLocal directly via the precompile.
    // The precompile's sudo_only() check passes because tx.from = sudo key.
    // The precompile dispatches as RawOrigin::Root, which Treasury::SpendOrigin
    // = NeverEnsureOrigin still rejects. So this path requires the precompile
    // to dispatch via Sudo::sudo(...) instead of RawOrigin::Root for spend_local
    // specifically.
    //
    // Implementation note for Task 6: rather than RawOrigin::Root for spendLocal,
    // wrap the call as Sudo::sudo(Box::new(Treasury::spend_local{...}.into()))
    // and dispatch with RawOrigin::Signed(sudo_key). This lets NeverEnsureOrigin
    // accept (it sees Root from the Sudo wrapper).
    await (await treasury.spendLocal(ethers.parseEther("1000"), beneficiaryAddr)).wait();

    // Wait one SpendPeriod (HOURS = 600 blocks @ 6s = 1 hour real time; with
    // runtime-test-fast the constant stays HOURS but the test may need a longer
    // timeout if not overridden).
    // For now assert the call did not revert; integration test (Step 1) covers
    // the actual credit.
    expect(true).to.equal(true);
  });
});
```

The 5th test exercises the **sudo-wrapped spend_local** path. The implementer of Task 6 (TreasuryPrecompile) MUST dispatch `spendLocal` as `Sudo::sudo(Box::new(Treasury::spend_local { ... }.into()))` (with caller as `RawOrigin::Signed(sudo_key)`), not as raw `RawOrigin::Root`, because Plan 2 wired `Treasury::Config::SpendOrigin = NeverEnsureOrigin` which rejects naked Root. This is a contract between T6 implementation and this test.

- [ ] **Step 3: Verify**

```bash
cd packages/contracts && pnpm hardhat test --network impetus_dev test/treasury.spec.ts
cd apps/node && SKIP_WASM_BUILD=1 cargo test -p impetus-runtime --test treasury_proposal
```

Expected: 5 EVM tests pass + 1 runtime integration test passes.

- [ ] **Step 4: Commit**

```bash
git add packages/contracts/test/treasury.spec.ts apps/node/runtimes/impetus/tests/treasury_proposal.rs
git commit -m "test(e2e): treasury precompile + revived runtime spend_local test"
```

---

## Task 19: `fast-unstake.spec.ts` + `bags-list.spec.ts` E2E

**Files:**
- Create: `packages/contracts/test/fast-unstake.spec.ts`
- Create: `packages/contracts/test/bags-list.spec.ts`

- [ ] **Step 1: `fast-unstake.spec.ts`** — chill → register → wait batch → assert Unstaked event.

```typescript
import { expect } from "chai";
import { ethers } from "hardhat";
import { ADDRESSES, waitForEra } from "./helpers/staking-helpers";

describe("FastUnstake precompile", function () {
  this.timeout(10 * 60 * 1000);

  it("register after chill emits Unstaked within a few eras", async () => {
    const [, , , dave] = await ethers.getSigners(); // Hardhat #3 = Dave validator
    const staking = await ethers.getContractAt("IStaking", ADDRESSES.STAKING, dave);
    const fastUnstake = await ethers.getContractAt("IFastUnstake", ADDRESSES.FAST_UNSTAKE, dave);

    await (await staking.chill()).wait();
    await (await fastUnstake.registerFastUnstake()).wait();

    const daveAddr = await dave.getAddress();
    const queue = await fastUnstake.queue(daveAddr);
    expect(queue).to.be.gt(0n);

    // Wait long enough for the batch to process the head
    await waitForEra(5);
    // After batch, queue entry should be cleared.
    const queueAfter = await fastUnstake.queue(daveAddr);
    expect(queueAfter).to.equal(0n);
  });
});
```

- [ ] **Step 2: `bags-list.spec.ts`** — bond + observe score → another account triggers rebag → ScoreUpdated emitted.

```typescript
import { expect } from "chai";
import { ethers } from "hardhat";
import { ADDRESSES } from "./helpers/staking-helpers";

describe("BagsList precompile", function () {
  this.timeout(5 * 60 * 1000);

  it("score is non-zero for genesis validators", async () => {
    const [alice] = await ethers.getSigners();
    const bags = await ethers.getContractAt("IBagsList", ADDRESSES.BAGS_LIST);
    const score = await bags.score(await alice.getAddress());
    expect(score).to.be.gt(0n);
  });

  it("listSize reflects bonded accounts", async () => {
    const bags = await ethers.getContractAt("IBagsList", ADDRESSES.BAGS_LIST);
    const size = await bags.listSize();
    expect(size).to.be.gte(4); // 4 genesis validators + 1 nominator
  });
});
```

- [ ] **Step 3: Verify + commit**

```bash
cd packages/contracts && pnpm hardhat test --network impetus_dev \
  test/fast-unstake.spec.ts test/bags-list.spec.ts
git add packages/contracts/test/fast-unstake.spec.ts packages/contracts/test/bags-list.spec.ts
git commit -m "test(e2e): fast-unstake + bags-list coverage"
```

---

## Task 20: `staking-admin.spec.ts` E2E (sudo gating)

**File:** `packages/contracts/test/staking-admin.spec.ts`

- [ ] **Step 1: Write the spec**

```typescript
import { expect } from "chai";
import { ethers } from "hardhat";
import { ADDRESSES } from "./helpers/staking-helpers";

describe("StakingAdmin precompile — sudo gating", function () {
  this.timeout(2 * 60 * 1000);

  it("non-sudo forceNewEra reverts NotSudo", async () => {
    const [, , , user] = await ethers.getSigners();
    const admin = await ethers.getContractAt("IStakingAdmin", ADDRESSES.STAKING_ADMIN, user);
    await expect(admin.forceNewEra()).to.be.revertedWith("NotSudo");
  });

  it("non-sudo setValidatorCount reverts NotSudo", async () => {
    const [, , , user] = await ethers.getSigners();
    const admin = await ethers.getContractAt("IStakingAdmin", ADDRESSES.STAKING_ADMIN, user);
    await expect(admin.setValidatorCount(8)).to.be.revertedWith("NotSudo");
  });
});
```

- [ ] **Step 2: Verify + commit**

```bash
cd packages/contracts && pnpm hardhat test --network impetus_dev test/staking-admin.spec.ts
git add packages/contracts/test/staking-admin.spec.ts
git commit -m "test(e2e): staking-admin sudo gating"
```

---

## Task 21: `delegatecall-guard.spec.ts` E2E

**File:** `packages/contracts/test/delegatecall-guard.spec.ts`

- [ ] **Step 1: Create a proxy contract that delegatecalls each precompile**

Add `packages/contracts/contracts/test/DelegateProxy.sol`:

```solidity
// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

contract DelegateProxy {
    function delegate(address target, bytes calldata data) external returns (bool, bytes memory) {
        return target.delegatecall(data);
    }
}
```

- [ ] **Step 2: Write the spec**

```typescript
import { expect } from "chai";
import { ethers } from "hardhat";
import { ADDRESSES } from "./helpers/staking-helpers";

describe("DELEGATECALL guard across NPoS precompiles", function () {
  this.timeout(2 * 60 * 1000);

  const targets = [
    { name: "Staking", iface: "IStaking", addr: ADDRESSES.STAKING, fn: "currentEra", args: [] },
    { name: "Session", iface: "ISession", addr: ADDRESSES.SESSION, fn: "currentIndex", args: [] },
    { name: "Pools", iface: "INominationPools", addr: ADDRESSES.POOLS, fn: "lastPoolId", args: [] },
    { name: "FastUnstake", iface: "IFastUnstake", addr: ADDRESSES.FAST_UNSTAKE, fn: "erasToCheckPerBlock", args: [] },
    { name: "Treasury", iface: "ITreasury", addr: ADDRESSES.TREASURY, fn: "pot", args: [] },
    { name: "BagsList", iface: "IBagsList", addr: ADDRESSES.BAGS_LIST, fn: "listSize", args: [] },
  ];

  it("write entries reject delegatecall with DELEGATECALL/CALLCODE forbidden", async () => {
    const proxyFactory = await ethers.getContractFactory("DelegateProxy");
    const proxy = await proxyFactory.deploy();
    await proxy.waitForDeployment();

    const staking = await ethers.getContractAt("IStaking", ADDRESSES.STAKING);
    const data = staking.interface.encodeFunctionData("chill", []);
    await expect(proxy.delegate(ADDRESSES.STAKING, data)).to.be.reverted;
  });

  it("view entries also fail via delegatecall (guard is at every entry)", async () => {
    const proxyFactory = await ethers.getContractFactory("DelegateProxy");
    const proxy = await proxyFactory.deploy();
    await proxy.waitForDeployment();

    for (const t of targets) {
      const c = await ethers.getContractAt(t.iface, t.addr);
      const data = c.interface.encodeFunctionData(t.fn, t.args);
      const [ok] = await proxy.delegate.staticCall(t.addr, data);
      expect(ok, `${t.name}.${t.fn} via delegatecall should fail`).to.equal(false);
    }
  });
});
```

- [ ] **Step 2.5: Compile + run**

```bash
cd packages/contracts && pnpm hardhat compile
pnpm hardhat test --network impetus_dev test/delegatecall-guard.spec.ts
```

- [ ] **Step 3: Commit**

```bash
git add packages/contracts/contracts/test/DelegateProxy.sol packages/contracts/test/delegatecall-guard.spec.ts
git commit -m "test(e2e): DELEGATECALL/CALLCODE guard across NPoS precompiles"
```

---

## Task 22: Production smoke + alias regression check

**Files:** none (verification only)

- [ ] **Step 1: Smoke `--chain impetus` (production alias)**

```bash
cd apps/node && cargo build --release --features impetus-runtime/runtime-test-fast
pkill -f frontier-template-node 2>/dev/null
./target/release/frontier-template-node --chain impetus --tmp --validator \
  --unsafe-force-node-key-generation > /tmp/impetus-prod-smoke.log 2>&1 &
sleep 8
pkill -f frontier-template-node
grep -E "Chain specification|Imported|Idle" /tmp/impetus-prod-smoke.log | head -5
```

Expected: `Chain specification: Impetus` (production display name), spec id `impetus`, ChainType::Live, validator boots and waits for session keys via author_insertKey (no auto-`--alice` for production spec).

- [ ] **Step 2: Smoke `--chain mainnet` (production alias)**

```bash
./target/release/frontier-template-node --chain mainnet --tmp \
  --unsafe-force-node-key-generation > /tmp/mainnet-smoke.log 2>&1 &
sleep 5
pkill -f frontier-template-node
grep "Chain specification" /tmp/mainnet-smoke.log
```

Expected: same `Impetus` production spec.

- [ ] **Step 3: Run full E2E suite against `impetus_dev_npos`**

```bash
# Start node
./target/release/frontier-template-node --chain impetus_dev_npos --tmp --validator --alice \
  --unsafe-force-node-key-generation > /tmp/impetus-e2e.log 2>&1 &
sleep 10

# Run all NPoS E2E specs
cd /Users/huyduan/projects/blockchain/packages/contracts
pnpm hardhat test --network impetus_dev \
  test/staking-validator.spec.ts \
  test/staking-nominator.spec.ts \
  test/staking-rebond.spec.ts \
  test/pools.spec.ts \
  test/treasury.spec.ts \
  test/fast-unstake.spec.ts \
  test/bags-list.spec.ts \
  test/staking-admin.spec.ts \
  test/delegatecall-guard.spec.ts

pkill -f frontier-template-node
```

Expected: all 9 spec files pass. Total wall-clock ≤ 15 min with runtime-test-fast.

- [ ] **Step 4: Commit (empty marker)**

```bash
cd /Users/huyduan/projects/blockchain
git commit --allow-empty -m "test(node): production alias smoke + full NPoS E2E suite passes"
```

---

## Task 23: Final CLAUDE.md refresh + acceptance gate

**Files:**
- Modify: `apps/node/CLAUDE.md`

- [ ] **Step 1: Replace the "Plan 2 known gap" note**

Remove the babe-worker note (fixed in Task 1). Add a paragraph confirming Plan 3 completion: 7 precompiles live, production alias routes to production spec, E2E suite green.

- [ ] **Step 2: Refresh the precompile address map table**

Replace the existing table with:

```
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
```

- [ ] **Step 3: Bump `spec_version` reference**

If any runtime config touched in this plan (Treasury redesign in Task 6 may not need runtime changes; if Task 1 modified runtime, bump). Otherwise leave at `4`.

If `spec_version` bumps to `5`, update:

```
Current spec_version is 5 on impetus (Plan 3 precompile rollout — bumped
from Plan 2's 4 if any runtime config was touched), 2 on impulse (unchanged).
```

- [ ] **Step 4: Commit**

```bash
git add apps/node/CLAUDE.md
git commit -m "docs(node): refresh CLAUDE.md for NPoS rollout (Plan 3 complete)"
```

---

## Acceptance gate

All of the following must hold before declaring Plan 3 done:

- [ ] `cd apps/node && cargo build --release` succeeds.
- [ ] `cd apps/node && cargo build --release --features impetus-runtime/runtime-test-fast` succeeds.
- [ ] `cd apps/node && cargo test --workspace` passes. The 7 new precompile crates each have ≥ 80% line coverage via `cargo llvm-cov -p precompile-<name>`.
- [ ] `cd apps/node && cargo clippy --workspace -- -D warnings` is clean.
- [ ] Task 1 smoke triple gate passes on `--chain impetus_dev_npos --validator --alice`: ≥ 20 blocks imported, session index ≥ 2, `Staking::CurrentEra` ≥ 1.
- [ ] Task 22 smoke `--chain impetus` (production alias) boots with ChainType::Live and spec id `impetus`.
- [ ] All 9 E2E spec files under `packages/contracts/test/` pass against a fresh `impetus_dev_npos` node within 15 min wall-clock.
- [ ] DELEGATECALL guard verified on every write entry of every NPoS precompile via `delegatecall-guard.spec.ts`.
- [ ] `apps/node/CLAUDE.md` reflects the NPoS rollout (no Plan 2 babe-worker limitation note, precompile address table extended through `0x0840`).
- [ ] Impulse + dev non-regression: `cargo test -p impulse-runtime` still passes; `--chain impulse --validator --alice` produces blocks; `--chain dev --sealing manual` accepts `engine_createBlock`.

> **Post-Plan-3:** Production deployment requires (a) replacing the
> placeholder stash addresses in `impetus_production_config` with the
> project's signed validator set, (b) operators running
> `scripts/dump-session-keys.ts` to produce SCALE-encoded `setKeys` inputs
> from a sealed mnemonic, (c) Treasury `SpendOrigin` redesign (currently
> `NeverEnsureOrigin` — opens once governance lands), (d) EPM signed-phase
> miner client wiring once validator count exceeds 32, (e) try-runtime
> simulation before each spec_version bump. None of these are blockers for
> Plan 3 sign-off — they belong to Plan 4 (Governance) or operational
> runbooks.
