# Impetus NPoS via EVM Precompiles — Design

**Date:** 2026-05-16
**Status:** Draft (pending implementation plan)
**Scope:** `apps/node` — `runtime-impetus` only. `runtime-impulse` and
`--chain dev` are explicitly unchanged.
**Spec depends on:** stable2603 polkadot-sdk + frontier toolchain pinning
(already in `Cargo.toml`).

## Summary

Convert `runtime-impetus` from a solo Aura + Grandpa permissioned chain into
a Nominated Proof of Stake (NPoS) chain using `pallet-babe` + `pallet-grandpa`
for consensus and the full Polkadot NPoS stack
(`pallet-staking`, `pallet-session`, `pallet-election-provider-multi-phase`,
`pallet-bags-list`, `pallet-offences`, `pallet-authority-discovery`,
`pallet-im-online`, `pallet-nomination-pools`, `pallet-treasury`,
`pallet-fast-unstake`) for staking economics. Expose the entire user-facing
surface via seven typed Solidity precompiles at `0x0810`–`0x0840` so
EVM wallets can bond, nominate, validate, claim rewards, manage pools,
fast-unstake, propose treasury spends, and run admin ops without any
Substrate-native tooling.

The chain already uses `AccountId20 = H160` via `fp_account::EthereumSignature`,
so EVM wallets are Substrate accounts at the type level; no H160 → AccountId32
mapping pallet is needed. `pallet-staking` is generic over `T::AccountId`
and compiles unchanged against `AccountId20`. Each precompile dispatches its
backing `pallet_*::Call::*` with `RawOrigin::Signed(handle.context().caller)`,
mirroring Moonbeam's `ParachainStaking` and `AuthorMapping` pattern.

`runtime-impulse` (testnet) stays on Aura + Grandpa with manual seal in dev
mode. The same `frontier-template-node` binary handles both chains via a
dual-path service module that dispatches consensus by `chain_spec.id()`.

## Goals

- Full NPoS economics on `runtime-impetus`: era-based validator election,
  nominator reward distribution, slashing into Treasury, fast-unstake queue,
  nomination pools.
- 100% of user-facing staking and operational surface callable from Solidity
  / EVM wallets — no Substrate-native UI dependency for `runtime-impetus`.
- Babe + Grandpa consensus standard so future migrations toward Kusama /
  Polkadot patterns (e.g. parachain conversion, governance v2) are
  incremental, not rewrites.
- Zero impact on `runtime-impulse` and `--chain dev`: same binary, separate
  consensus path, fast iteration on testnet preserved.
- Dev-fast timing profile (10-min sessions, 60-min eras, 4-era bonding)
  so the full validator/nominator lifecycle can be exercised in a single
  working session.

## Non-Goals (v1)

- Governance v2 (referenda, conviction voting, OpenGov).
- Council / Technical Committee. Treasury approval is sudo-gated for v1.
- XCM, HRMP, or parachain migration of `impetus`.
- Bridge integrations.
- Staking proxies, multisig validator operators.
- Try-runtime simulation framework, on-chain storage migrations (greenfield
  reset — user can purge chain at any time during dev).
- Substrate-native UI (still usable via polkadot.js apps; not required for
  the user flows the precompiles cover).
- Off-chain election miner client setup (relying on
  `OnChainExecution<OnChainSeqPhragmen>` fallback for v1).
- Benchmark-generated pallet weights (defaulting to `()` for v1; production
  hardening deferred).
- Migrating `runtime-impulse` to NPoS or Babe.

## Architecture

### Layered stack

```
+----------------------------------------------------------+
| Solidity dApps / wallets (MetaMask)                      |  EVM client
+----------------------------------------------------------+
| Typed precompiles 0x0810–0x0840 (Staking, Session, ...)  |  EVM ↔ Substrate
+----------------------------------------------------------+
| pallet-staking / pallet-session / pallet-babe / ...      |  Runtime FRAME
+----------------------------------------------------------+
| Babe authorship + Grandpa finality                       |  Consensus
+----------------------------------------------------------+
| sc-consensus-babe + sc-consensus-grandpa (node service)  |  Client
+----------------------------------------------------------+
```

### Per-runtime divergence

|                       | `runtime-impetus`                              | `runtime-impulse`                            | `--chain dev`           |
|-----------------------|------------------------------------------------|----------------------------------------------|-------------------------|
| Consensus             | **Babe + Grandpa**                             | Aura + Grandpa (unchanged)                   | Aura + manual seal      |
| NPoS pallets          | Full stack (10 new pallets)                    | None (unchanged)                             | None                    |
| Precompiles           | 1–5, curve, gasless, batch, **+ 7 NPoS**       | 1–5, curve, gasless, batch                   | Same as impulse         |
| Session keys          | Babe + Grandpa + ImOnline + AuthDiscovery (4)  | Aura + Grandpa (2)                           | Same as impulse         |
| `spec_version`        | 2 → **3**                                      | 2 (unchanged)                                | n/a                     |
| Block production      | Babe slot leaders                              | Aura round-robin                             | Manual seal via RPC     |

### Workspace changes

```
apps/node/
├── Cargo.toml                       # ~15 new pallet deps + Babe deps
├── node/
│   └── src/
│       ├── service.rs               # SPLIT into service_aura, service_babe, service_common
│       ├── service_common.rs        # NEW: shared new_partial helpers
│       ├── service_aura.rs          # NEW: extracted Aura import queue + authoring
│       ├── service_babe.rs          # NEW: Babe import queue + authoring + authority-discovery + im-online
│       └── command.rs               # MODIFIED: dispatch by Network::from_spec_id
├── runtimes/
│   ├── common/
│   │   └── src/
│   │       ├── precompiles.rs       # SPLIT FrontierPrecompiles into Basic + Npos variants
│   │       └── staking_constants.rs # NEW: dev-fast NPoS constants
│   ├── impetus/                     # MODIFIED: full NPoS pallet wiring, Babe swap, SessionKeys (4)
│   └── impulse/                     # UNCHANGED structurally; SessionKeys moves out of common
├── pallets/
│   └── gasless-registry/            # UNCHANGED
└── precompiles/
    ├── gasless-registry/            # UNCHANGED
    ├── batch/                       # UNCHANGED
    ├── staking/                     # NEW — Staking.sol @ 0x0810
    ├── session/                     # NEW — Session.sol @ 0x0818
    ├── nomination-pools/            # NEW — NominationPools.sol @ 0x0820
    ├── fast-unstake/                # NEW — FastUnstake.sol @ 0x0828
    ├── treasury/                    # NEW — Treasury.sol @ 0x0830
    ├── bags-list/                   # NEW — BagsList.sol @ 0x0838
    └── staking-admin/               # NEW — StakingAdmin.sol @ 0x0840 (root-gated)
```

### Key type invariants

- `Signature = fp_account::EthereumSignature` (unchanged) ⇒
  `AccountId = AccountId20 = H160`.
- All NPoS pallets are `T::AccountId`-generic and compile unchanged with
  AccountId20. `pallet-staking`'s offence/slash machinery, election provider
  solution types, and bags-list scoring are all generic.
- `PalletId::into_sub_account_truncating` works with `AccountId20`
  (`fp_account` provides the `AccountIdConversion` impl). Treasury pot and
  per-pool bonded accounts derive correctly.

### FrontierPrecompiles split

```rust
// runtimes/common/src/precompiles.rs
pub struct FrontierPrecompilesBasic<R>(PhantomData<R>);   // used by impulse
pub struct FrontierPrecompilesNpos<R>(PhantomData<R>);    // used by impetus
```

`Basic` registers the 9 existing entries (Ethereum 1–5, curve25519,
gasless `0x0800`, batch `0x0808`). `Npos` re-uses `Basic`'s dispatch arms
and adds 7 more for NPoS, gated by extra trait bounds:

```rust
R: pallet_evm::Config
    + pallet_staking::Config
    + pallet_session::Config
    + pallet_nomination_pools::Config
    + pallet_treasury::Config
    + pallet_fast_unstake::Config
    + pallet_bags_list::Config<pallet_bags_list::Instance1>
    + pallet_sudo::Config,
```

## Consensus migration (Aura → Babe + Grandpa) — impetus only

### Runtime-side changes (`runtimes/impetus/src/lib.rs`)

**Remove:** `pallet_aura`, `sp_consensus_aura` imports, `Aura` from
`construct_runtime`, `AuraApi` runtime API impl.

**Add (pallet indexes — Babe at 2, NPoS stack 17–28):**

```
 0 System              17 Authorship          24 AuthorityDiscovery
 1 Timestamp           18 Session             25 ImOnline
 2 Babe                19 Historical          26 NominationPools
 3 Grandpa             20 Staking             27 Treasury
 4 Balances            21 Offences            28 FastUnstake
 5 TransactionPayment  22 ElectionProviderMultiPhase
 6 Sudo                23 VoterList (bags-list, Instance1)
 7 Ethereum
 8 EVM
 9 EVMChainId
10 BaseFee
11 ManualSeal          (kept for RPC compat; idle under Babe authoring;
                       removal deferred to a later cleanup commit)
12 Assets
14 GaslessRegistry
```

**Babe config (key bits):**

```rust
impl pallet_babe::Config for Runtime {
    type EpochDuration = ConstU64<{ BLOCKS_PER_SESSION as u64 }>;
    type ExpectedBlockTime = ConstU64<MILLISECS_PER_BLOCK>;
    type EpochChangeTrigger = pallet_babe::ExternalTrigger;          // pallet-session triggers
    type DisabledValidators = Session;
    type KeyOwnerProof =
        <Historical as KeyOwnerProofSystem<(KeyTypeId, AuthorityId)>>::Proof;
    type EquivocationReportSystem =
        pallet_babe::EquivocationReportSystem<Self, Offences, Historical, ReportLongevity>;
    type MaxAuthorities = ConstU32<MAX_VALIDATOR_COUNT>;
    type MaxNominators = ConstU32<{ MAX_VALIDATOR_COUNT * MAX_NOMINATORS_PER_VALIDATOR }>;
    type WeightInfo = ();
}
```

Babe randomness comes from its own `RandomnessFromOneEpochAgo`; no separate
`pallet-randomness-collective-flip` needed. Primary slot ratio `c = (1, 4)`
(genesis epoch config) — ~25% primary VRF wins, 75% secondary round-robin
fallback (Aura-like) so block production never stalls when no primary winner
exists for a slot.

**FindAuthor swap** (`pallet_evm::Config::FindAuthor`):
- Before: `FindAuthorTruncated<Aura, Self>`
- After: `pallet_session::FindAccountFromAuthorIndex<Self, Babe>` —
  `pallet-authorship` already maps slot author → `T::AccountId = H160`.

**Runtime APIs swap:**
- Remove `impl sp_consensus_aura::AuraApi for Runtime`.
- Add `impl sp_consensus_babe::BabeApi for Runtime` (epoch, slot,
  authorities, generate/submit equivocation proof).
- Add `impl sp_authority_discovery::AuthorityDiscoveryApi for Runtime`
  for DHT peer lookup.
- Keep `sp_consensus_grandpa::GrandpaApi for Runtime` but swap the stub
  equivocation reporter for the real one:
  ```rust
  fn submit_report_equivocation_unsigned_extrinsic(
      equivocation_proof: ..., key_owner_proof: ...,
  ) -> Option<()> {
      Grandpa::submit_unsigned_equivocation_report(
          equivocation_proof, key_owner_proof,
      )
  }
  ```

**Session keys** (now in `runtimes/impetus/src/lib.rs`, no longer in common):

```rust
impl_opaque_keys! {
    pub struct SessionKeys {
        pub babe: BabeId,                               // sr25519
        pub grandpa: GrandpaId,                         // ed25519
        pub im_online: ImOnlineId,                      // sr25519
        pub authority_discovery: AuthorityDiscoveryId,  // sr25519
    }
}
```

Validators register all four atomically via `Session.sol::setKeys(bytes keys,
bytes proof)`. `keys` is the SCALE-encoded `SessionKeys` struct; `proof` is
the ownership proof. The `scripts/dump-session-keys.ts` helper produces both
from a seed phrase.

### Node-side changes (`apps/node/node/src/`)

`service.rs` splits into three modules with shared scaffolding:

| Module                | Responsibility                                                                 |
|-----------------------|--------------------------------------------------------------------------------|
| `service_common.rs`   | `new_partial` skeleton, `FrontierBackend`, `FullClient` aliases, telemetry     |
| `service_aura.rs`     | Existing `build_aura_grandpa_import_queue` + `start_aura_authoring` (unchanged) |
| `service_babe.rs`     | NEW `build_babe_grandpa_import_queue` + `start_babe_authoring` + authority-discovery worker + im-online registration |

`command.rs` dispatches by `Network::from_spec_id(chain_spec.id())`:

```rust
match Network::from_spec_id(config.chain_spec.id()) {
    Network::Impetus => service_babe::new_full(config),
    Network::Impulse => service_aura::new_full(config),
}
```

Manual-seal mode under `--chain dev` continues to use `service_aura` (since
`development_config` is an impulse alias). Manual seal coexists with the Babe
pallet only at the *runtime* level (block construction still drives Babe's
inherent), but block authorship is driven by `sc_consensus_manual_seal::run_manual_seal`
in that path. `--chain dev` does **not** exercise Babe authoring; that
behavior is `--chain impetus` only.

**New node-side deps** (workspace `Cargo.toml`):
- Add: `sc-consensus-babe`, `sc-consensus-babe-rpc`, `sp-consensus-babe`,
  `sp-authority-discovery`, `sc-authority-discovery`.
- Keep: `sc-consensus-aura`, `sp-consensus-aura` (still used by impulse path).

**Binary size impact**: ~10–15 MB increase in release build (two import
queues + Babe runtime API client + im-online worker bytecode). Accept.

## NPoS pallet wiring (impetus only)

### `runtimes/common/src/staking_constants.rs` (NEW, dev-fast profile)

```rust
pub const MILLISECS_PER_BLOCK: u64 = 6_000;
pub const BLOCKS_PER_SESSION: BlockNumber = 100;       // 10 min
pub const SESSIONS_PER_ERA: SessionIndex = 6;          // era = 60 min
pub const BONDING_DURATION_ERAS: EraIndex = 4;         // ~4 hours
pub const SLASH_DEFER_DURATION_ERAS: EraIndex = 3;
pub const MAX_NOMINATIONS: u32 = 16;
pub const MAX_NOMINATORS_PER_VALIDATOR: u32 = 16;
pub const VALIDATOR_COUNT_TARGET: u32 = 8;
pub const MAX_VALIDATOR_COUNT: u32 = 32;
pub const MIN_VALIDATOR_BOND: Balance = 1_000 * UNIT;
pub const MIN_NOMINATOR_BOND: Balance = 10 * UNIT;
pub const REPORT_LONGEVITY: u64 =
    (BONDING_DURATION_ERAS as u64) * (SESSIONS_PER_ERA as u64) * (BLOCKS_PER_SESSION as u64);
```

These constants are compile-time. When promoting to production timing, edit
this file and bump `spec_version`. They are **not** governance-tunable in v1.

### Reward curve

```rust
pallet_staking_reward_curve::build! {
    const REWARD_CURVE: PiecewiseLinear<'static> = curve!(
        min_inflation: 0_025_000,    // 2.5%
        max_inflation: 0_100_000,    // 10%
        ideal_stake: 0_750_000,      // 75%
        falloff: 0_050_000,
        max_piece_count: 40,
        test_precision: 0_005_000,
    );
}
```

Matches Polkadot defaults. Inflation accrues to elected validators each era
(`pallet_staking::EraPayout = pallet_staking::ConvertCurve<RewardCurve>`);
`RewardRemainder` (unclaimed slice when stake < ideal) routes to Treasury.

### Critical pallet configs (highlights)

```rust
// pallet-staking
impl pallet_staking::Config for Runtime {
    type Currency = Balances;
    type UnixTime = Timestamp;
    type CurrencyToVote = sp_staking::currency_to_vote::SaturatingCurrencyToVote;
    type RewardRemainder = Treasury;
    type Slash = Treasury;                              // slashed funds → Treasury pot
    type Reward = ();                                   // mint via inflation
    type SessionsPerEra = SessionsPerEra;
    type BondingDuration = BondingDuration;
    type SlashDeferDuration = SlashDeferDuration;
    type AdminOrigin = EnsureRoot<AccountId>;
    type SessionInterface = Self;
    type EraPayout = pallet_staking::ConvertCurve<RewardCurve>;
    type NextNewSession = Session;
    type MaxExposurePageSize = ConstU32<MAX_NOMINATORS_PER_VALIDATOR>;
    type ElectionProvider = ElectionProviderMultiPhase;
    type GenesisElectionProvider = onchain::OnChainExecution<OnChainSeqPhragmen>;
    type VoterList = VoterList;                         // pallet-bags-list, Instance1
    type TargetList = pallet_staking::UseValidatorsMap<Self>;
    type NominationsQuota = pallet_staking::FixedNominationsQuota<MAX_NOMINATIONS>;
    type MaxUnlockingChunks = ConstU32<32>;
    type HistoryDepth = ConstU32<84>;
    type EventListeners = NominationPools;
    type WeightInfo = ();
}

// pallet-session
impl pallet_session::Config for Runtime {
    type ValidatorId = AccountId;                       // = H160
    type ValidatorIdOf = pallet_staking::StashOf<Self>;
    type ShouldEndSession = pallet_session::PeriodicSessions<Period, Offset>;
    type NextSessionRotation = pallet_session::PeriodicSessions<Period, Offset>;
    type SessionManager = pallet_session::historical::NoteHistoricalRoot<Self, Staking>;
    type SessionHandler = <SessionKeys as OpaqueKeys>::KeyTypeIdProviders;
    type Keys = SessionKeys;
}

// pallet-election-provider-multi-phase
parameter_types! {
    pub const SignedPhase: u32 = 25;     // ~2.5 min — short for dev
    pub const UnsignedPhase: u32 = 25;
    pub const SignedMaxSubmissions: u32 = 10;
    pub MaxElectingVoters: u32 = 1024;
    pub MaxElectableTargets: u16 = 64;
}
// Falls back to OnChainExecution<OnChainSeqPhragmen> if signed+unsigned phases
// produce no valid solution — sufficient for dev validator-set sizes ≤ 32.
```

### Bags-list thresholds

200 geometric buckets from `MIN_NOMINATOR_BOND` to `1_000_000_000 * UNIT`
(ratio ≈ 1.21). Generated offline via the `pallet-bags-list/voter-bags`
binary; pasted into `runtimes/common/src/voter_bags.rs`. Implementation plan
provides the exact command.

### Inherent ordering at block construction

1. Timestamp
2. Babe slot
3. Uncles (via `pallet-authorship`)
4. Staking off-chain election (signed/unsigned phase tx, when in election window)
5. ImOnline heartbeats (off-chain worker)

## Precompile design

### Shared invariants

Every precompile crate (`precompiles/<name>/`) follows the `precompile-batch`
template:

- `#[precompile_utils_macro::precompile]` on a unit struct
  `XxxPrecompile<Runtime>(PhantomData<Runtime>)`.
- Trait bounds on `Runtime` matching the backing pallet's Config requirements
  plus `pallet_evm::Config` and `Runtime::AccountId: From<H160>`.
- **DELEGATECALL / CALLCODE guard** at the top of every write entry —
  reverts `"DELEGATECALL/CALLCODE forbidden"` when
  `handle.code_address() != handle.context().address`.
- **Origin construction**: `RawOrigin::Signed(handle.context().caller.into())`
  — sub-pallet sees `msg.sender = the immediate caller of the precompile`.
  Matches the batch precompile's caller-funded semantics.
- **Dispatch helper**: `RuntimeHelper::<Runtime>::try_dispatch(handle, origin, call)`
  records dispatch weight as EVM gas and maps `DispatchError` to revert
  strings. Pallet errors surface as their canonical names (e.g.
  `"InsufficientBond"`, `"NotController"`, `"AlreadyBonded"`).
- **Reward destination encoding** (Solidity ↔ Rust):
  ```solidity
  struct RewardDestination { uint8 kind; address account; }
  // kind 0=Staked, 1=Stash, 2=Controller, 3=Account(account), 4=None
  ```
  `account` is ignored unless `kind == 3`.
- **Events**: mirror `pallet_*::Event` 1:1 with `keccak256(signature)` topics
  hardcoded as 32-byte constants in each crate's `lib.rs`. Param indexing
  matches Moonbeam where applicable (e.g. `address`-typed identifiers
  indexed; balance amounts non-indexed).
- **Sudo gating** (where applicable): `StakingAdmin`, `Treasury::approveProposal`,
  `Treasury::rejectProposal`, `Treasury::spendLocal`, `FastUnstake::control`,
  `NominationPools::setConfigs`, `Staking::chillOther` (when called as admin)
  revert with `"NotSudo"` if `caller != Sudo::key()`.

### Precompile address map (extended)

| Address       | Precompile        | Crate                              |
|---------------|-------------------|------------------------------------|
| `0x01–0x05`   | Ethereum stdlib   | upstream                           |
| `0x0400–0x0403` | curve25519 + Sha3FIPS + ECRecoverPK | upstream                |
| `0x0800` (2048) | GaslessRegistry | `precompile-gasless-registry`      |
| `0x0808` (2056) | Batch           | `precompile-batch`                 |
| `0x0810` (2064) | Staking         | `precompile-staking`               |
| `0x0818` (2072) | Session         | `precompile-session`               |
| `0x0820` (2080) | NominationPools | `precompile-nomination-pools`      |
| `0x0828` (2088) | FastUnstake     | `precompile-fast-unstake`          |
| `0x0830` (2096) | Treasury        | `precompile-treasury`              |
| `0x0838` (2104) | BagsList        | `precompile-bags-list`             |
| `0x0840` (2112) | StakingAdmin    | `precompile-staking-admin`         |

### 4.1 `Staking.sol` @ `0x0810`

**Write functions** (map to `pallet_staking::Call`):

| Solidity | Substrate call |
|---|---|
| `bond(uint256 value, RewardDestination payee)` | `bond { value, payee }` |
| `bondExtra(uint256 maxAdditional)` | `bond_extra { max_additional }` |
| `unbond(uint256 value)` | `unbond { value }` |
| `withdrawUnbonded(uint32 numSlashingSpans)` | `withdraw_unbonded { num_slashing_spans }` |
| `validate((uint16 commission, bool blocked))` | `validate { prefs }` |
| `nominate(address[] targets)` | `nominate { targets }` (codec-bounded at MAX_NOMINATIONS) |
| `chill()` | `chill {}` |
| `setPayee(RewardDestination payee)` | `set_payee { payee }` |
| `payoutStakers(address validatorStash, uint32 era)` | `payout_stakers { validator_stash, era }` |
| `payoutStakersByPage(address validatorStash, uint32 era, uint32 page)` | `payout_stakers_by_page` |
| `rebond(uint256 value)` | `rebond { value }` |
| `kick(address[] who)` | `kick { who }` (validator-only) |
| `chillOther(address controller)` | `chill_other { stash }` |
| `forceApplyMinCommission(address validator)` | `force_apply_min_commission` |
| `reapStash(address stash, uint32 numSlashingSpans)` | `reap_stash` |

**View functions** (read storage / staking runtime API):

| Solidity | Returns |
|---|---|
| `bonded(address stash)` | `address controller` (= stash post-stable2603) |
| `ledger(address controller)` | `StakingLedger { stash, total, active, unlocking[], legacyClaimedRewards[] }` |
| `validators(address stash)` | `ValidatorPrefs { commission (Perbill as uint32), blocked }` |
| `nominators(address stash)` | `Nominations { targets[], submittedIn, suppressed }` |
| `currentEra()` | `uint32` |
| `activeEra()` | `(uint32 index, uint64 startMs)` |
| `erasStakers(uint32 era, address validator)` | `Exposure { total, own, others[] }` |
| `erasValidatorReward(uint32 era)` | `uint256` |
| `erasRewardPoints(uint32 era)` | `(uint32 total, (address, uint32)[] individual)` |
| `minNominatorBond()` | `uint256` |
| `minValidatorBond()` | `uint256` |
| `minActiveStake()` | `uint256` |
| `validatorCount()` | `uint32` |
| `counterForValidators()` | `uint32` |
| `counterForNominators()` | `uint32` |
| `historyDepth()` | `uint32` |

**Events** (mirror `pallet_staking::Event`):

`Bonded(address stash, uint256 amount)`,
`Unbonded(address stash, uint256 amount)`,
`Withdrawn(address stash, uint256 amount)`,
`Slashed(address staker, uint256 amount)`,
`SlashReported(address validator, uint256 fraction, uint32 slashEra)`,
`OldSlashingReportDiscarded(uint32 sessionIndex)`,
`StakersElected()`,
`StakingElectionFailed()`,
`Chilled(address stash)`,
`Kicked(address nominator, address stash)`,
`PayoutStarted(uint32 eraIndex, address validatorStash)`,
`Rewarded(address stash, uint8 dest, uint256 amount)`,
`ValidatorPrefsSet(address stash, uint32 commission, bool blocked)`,
`ForceEra(uint8 mode)` // 0=NotForcing, 1=ForceNew, 2=ForceNone, 3=ForceAlways

### 4.2 `Session.sol` @ `0x0818`

```solidity
function setKeys(bytes keys, bytes proof) external;
// keys = SCALE-encoded SessionKeys (babe||grandpa||im_online||authority_discovery)
// proof = SCALE-encoded ownership proof per pallet-session::Pallet::set_keys
function purgeKeys() external;
function nextKeys(address validator) external view returns (bytes);
function queuedKeys() external view returns (address[] validators, bytes[] keys);
function currentIndex() external view returns (uint32);
```

Event: `NewSession(uint32 sessionIndex)`.

**Helper script** `scripts/dump-session-keys.ts`: from a seed phrase produces
(a) JSON of 4 public keys, (b) SCALE-encoded bytes ready for `setKeys`,
(c) `author_insertKey` RPC commands per key for keystore population.

### 4.3 `NominationPools.sol` @ `0x0820`

**Write:**

| Solidity | Substrate call |
|---|---|
| `join(uint256 amount, uint32 poolId)` | `join` |
| `bondExtra((uint8 kind, uint256 amount))` | `bond_extra` (kind: 0=FreeBalance, 1=Rewards) |
| `claimPayout()` | `claim_payout` |
| `unbond(address memberAccount, uint256 unbondingPoints)` | `unbond` |
| `poolWithdrawUnbonded(uint32 poolId, uint32 numSlashingSpans)` | `pool_withdraw_unbonded` |
| `withdrawUnbonded(address memberAccount, uint32 numSlashingSpans)` | `withdraw_unbonded` |
| `create(uint256 amount, address root, address nominator, address bouncer)` | `create` |
| `createWithPoolId(...)` | `create_with_pool_id` |
| `nominate(uint32 poolId, address[] validators)` | `nominate` |
| `setState(uint32 poolId, uint8 state)` | `set_state` (0=Open, 1=Blocked, 2=Destroying) |
| `setMetadata(uint32 poolId, bytes metadata)` | `set_metadata` (≤ 256 bytes) |
| `setConfigs(uint256 minJoinBond, uint256 minCreateBond, uint32 maxPools, uint32 maxMembers, uint32 maxMembersPerPool, uint32 globalMaxCommission)` | `set_configs` (sudo) |
| `updateRoles(uint32 poolId, (uint8 op, address account)[3] roles)` | `update_roles` |
| `chill(uint32 poolId)` | `chill` |
| `bondExtraOther(address member, (uint8,uint256) extra)` | `bond_extra_other` |
| `setCommission(uint32 poolId, (uint32 commission, address payee))` | `set_commission` |
| `setCommissionMax(uint32 poolId, uint32 maxCommission)` | `set_commission_max` |
| `setCommissionChangeRate(uint32 poolId, (uint32 maxIncrease, uint32 minDelay))` | `set_commission_change_rate` |
| `claimCommission(uint32 poolId)` | `claim_commission` |

**View:**

- `bondedPools(uint32 poolId)` → `BondedPool { points, state, memberCounter, roles, commission }`
- `poolMembers(address account)` → `PoolMember { poolId, points, lastRecordedRewardCounter, unbondingEras }`
- `metadata(uint32 poolId)` → `bytes`
- `lastPoolId()` → `uint32`

**Events:** `Created`, `Bonded`, `PaidOut`, `Unbonded`, `Withdrawn`,
`Destroyed`, `StateChanged`, `MemberRemoved`, `RolesUpdated`, `PoolSlashed`,
`UnbondingPoolSlashed`, `PoolCommissionUpdated`, `PoolMaxCommissionUpdated`,
`PoolCommissionChangeRateUpdated`, `PoolCommissionClaimed`.

### 4.4 `FastUnstake.sol` @ `0x0828`

```solidity
function registerFastUnstake() external;
function deregister() external;
function control(uint32 erasToCheckPerBlock) external;     // sudo

// View
function head() external view returns (address stash);
function queue(address stash) external view returns (uint256 deposit);
function erasToCheckPerBlock() external view returns (uint32);
```

Events: `Unstaked(address stash, bool success)`,
`Slashed(address stash, uint256 amount)`,
`InternalError()`, `BatchChecked(uint32[] eras)`,
`BatchFinished(uint32 size)`.

### 4.5 `Treasury.sol` @ `0x0830`

```solidity
// Proposal flow: anyone proposes (bonds 5% of value, min 100 IPT), sudo approves
function proposeSpend(uint256 value, address beneficiary) external returns (uint32 proposalId);
function rejectProposal(uint32 proposalId) external;        // sudo
function approveProposal(uint32 proposalId) external;       // sudo
function removeApproval(uint32 proposalId) external;        // sudo

// Direct spend — SpendOrigin = NeverEnsureOrigin in v1, so this only works
// if routed through Sudo::sudo(...) externally; the precompile entry exists
// for ABI completeness and reverts "BadOrigin" otherwise.
function spendLocal(uint256 amount, address beneficiary) external;

// View
function pot() external view returns (uint256);
function proposalCount() external view returns (uint32);
function proposals(uint32 id) external view returns (address proposer, uint256 value, address beneficiary, uint256 bond);
function approvals() external view returns (uint32[]);
```

Events: `Proposed`, `Spending`, `Awarded`, `Rejected`, `Burnt`, `Rollover`,
`Deposit`.

### 4.6 `BagsList.sol` @ `0x0838`

```solidity
function putInFrontOf(address lighter) external;
function rebag(address dislocated) external;

// View
function bagOf(address account) external view returns (uint64 threshold);
function score(address account) external view returns (uint64);
function listSize() external view returns (uint32);
```

Events: `Rebagged(address who, uint64 from, uint64 to)`,
`ScoreUpdated(address who, uint64 newScore)`.

### 4.7 `StakingAdmin.sol` @ `0x0840` (sudo-gated)

```solidity
function setValidatorCount(uint32 new_) external;
function increaseValidatorCount(uint32 additional) external;
function scaleValidatorCount(uint8 factorPercent) external;
function setInvulnerables(address[] validators) external;
function forceUnstake(address stash, uint32 numSlashingSpans) external;
function forceNewEra() external;
function forceNoEras() external;
function forceNewEraAlways() external;
function cancelDeferredSlash(uint32 era, uint32[] slashIndices) external;
function setStakingConfigs(uint256 minNominatorBond, uint256 minValidatorBond, uint32 maxNominatorCount, uint32 maxValidatorCount, uint8 chillThresholdPercent, uint32 minCommission) external;
function chillOther(address controller) external;
```

Every entry: dispatch with `RawOrigin::Root`. Revert `"NotSudo"` when
`caller != Sudo::key()`.

### Size estimate

- 7 precompile crates × ~500 lines (lib + mode + mock + tests) ≈ **3,500 lines Rust**
- 7 Solidity interface files × ~150 lines ≈ **1,000 lines Solidity** (documentation; not compiled into the chain)
- `runtimes/common/src/precompiles.rs` (+ ~70 lines for `Npos` variant)
- `runtimes/impetus/src/lib.rs` (+ ~400 lines for pallet configs + Babe wiring)

## Genesis spec — impetus only

`node/src/chain_spec.rs::impetus_config()` rewritten. `impulse_config()` and
`development_config()` untouched.

### Initial stakers

| Role | Stash (Hardhat) | Bond | Session-key derivation |
|---|---|---|---|
| Validator A | `0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266` (#0) | 2,000 IPT | `//Alice` |
| Validator B | `0x70997970C51812dc3A010C7d01b50e0d17dc79C8` (#1) | 2,000 IPT | `//Bob` |
| Validator C | `0x3C44CdDdB6a900fa2b585dd299e03d12FA4293BC` (#2) | 2,000 IPT | `//Charlie` |
| Validator D | `0x90F79bf6EB2c4f870365E785982E1f101E93b906` (#3) | 2,000 IPT | `//Dave` |
| Nominator   | `0x15d34AAf54267DB7D7c367839AAf71A00a2C6A65` (#4) | 5,000 IPT, targets A+B+C | (none) |

Stash = secp256k1 (EVM-style). Session keys = sr25519 / ed25519
(Substrate-native crypto). Same validator, two distinct keypairs — standard
Polkadot convention. The `//Alice` … `//Dave` derivations are deterministic
so genesis bytes are reproducible.

### Genesis builder skeleton

```rust
fn impetus_genesis() -> serde_json::Value {
    let validators: [(AccountId, SessionKeys); 4] = [
        (HARDHAT_0, alice_session_keys()),
        (HARDHAT_1, bob_session_keys()),
        (HARDHAT_2, charlie_session_keys()),
        (HARDHAT_3, dave_session_keys()),
    ];

    serde_json::json!({
        "balances": { "balances": pre_funded_balances() },
        "evmChainId": { "chainId": 388266 },
        "sudo": { "key": SUDO_ADMIN },
        "babe": {
            "authorities": [],                          // populated by pallet-session at block 1
            "epochConfig": {
                "c": [1, 4],                            // 25% primary slot ratio
                "allowed_slots": "PrimaryAndSecondaryVRFSlots",
            },
        },
        "session": {
            "keys": validators.iter()
                .map(|(stash, keys)| (stash, stash, keys))
                .collect::<Vec<_>>()
        },
        "staking": {
            "validatorCount": 8,
            "minimumValidatorCount": 1,                 // single-node dev OK
            "invulnerables": [],
            "forceEra": "NotForcing",
            "slashRewardFraction": Perbill::from_percent(10),
            "stakers": validators.iter()
                .map(|(s, _)| (s, s, 2_000 * UNIT, StakerStatus::Validator))
                .chain([(HARDHAT_4, HARDHAT_4, 5_000 * UNIT,
                         StakerStatus::Nominator(vec![HARDHAT_0, HARDHAT_1, HARDHAT_2]))])
                .collect::<Vec<_>>(),
            "minNominatorBond": MIN_NOMINATOR_BOND,
            "minValidatorBond": MIN_VALIDATOR_BOND,
            "maxValidatorCount": Some(MAX_VALIDATOR_COUNT),
            "maxNominatorCount": Some(1024),
        },
        "treasury": {},
        "nominationPools": {
            "minJoinBond": 10 * UNIT,
            "minCreateBond": 1_000 * UNIT,
            "maxPools": Some(32),
            "maxMembers": Some(1024),
            "maxMembersPerPool": Some(256),
            "globalMaxCommission": Some(100_000),       // 10% as Perbill ppm
        },
        "grandpa": { "authorities": [] },               // populated via session
    })
}
```

### Genesis election bootstrap (automatic)

1. Read `staking.stakers` (4 validators + 1 nominator).
2. `GenesisElectionProvider = OnChainExecution<OnChainSeqPhragmen>` runs
   at block 0, elects top 4 by stake-weighted score.
3. `pallet-session::genesis()` reads `session.keys`, registers them under
   the elected validators.
4. `pallet-babe` + `pallet-grandpa` pull authorities from session at block 1.
5. Era 0 starts. Block 1 produced by the Babe slot leader.

### Pre-conditions asserted at genesis build time

- Total genesis stake = 4 × 2000 + 5000 = 13,000 IPT > `1 × MIN_VALIDATOR_BOND`.
- All 4 validator session keys present.
- All stashes pre-funded with ≥ bond amount via `balances.balances`.

A build-time `assert!` in `impetus_genesis()` aborts node startup if any of
these is violated, preventing the chain from panicking at block 1.

### Spec metadata

- `id: "impetus_dev_npos"`
- `name: "Impetus Dev (NPoS)"`
- `protocol_id: "impetus-npos"`
- `spec_version: 3`

Distinct from prior `impetus` spec id so a node running old chain state must
purge DB before booting the new chain.

### Validator startup workflow

1. Operator runs `frontier-template-node --chain impetus_dev_npos --validator`.
2. Inject session keys into the local keystore via RPC:
   ```
   author_insertKey("babe", "//Alice", babe_pubkey)
   author_insertKey("gran", "//Alice", grandpa_pubkey)
   author_insertKey("imon", "//Alice", im_online_pubkey)
   author_insertKey("audi", "//Alice", authority_discovery_pubkey)
   ```
   The `scripts/dump-session-keys.ts` helper prints these commands.
3. Block authoring starts automatically when the slot leader matches the
   keystore-resident Babe key.

## Testing strategy

### Per-precompile unit tests (`precompiles/<name>/src/tests.rs`)

Pattern: reuse `precompile-batch`'s mock + `with_subcall_handle` approach.
Each crate ≥ 20 tests covering:

- Happy-path dispatch for each function.
- Origin enforcement (`handle.context().caller` becomes `RawOrigin::Signed`).
- DELEGATECALL/CALLCODE guard reverts.
- Sudo gating where applicable.
- Pallet error → revert string mapping (`InsufficientBond`, `NotController`,
  `EraNotEnded`, etc.).
- Codec bounds (`BoundedVec` length limits).
- View function correctness post state setup.
- Weight accounting via `RuntimeHelper::try_dispatch`.

Per-precompile mock runtime includes the pallet under test plus its direct
deps (e.g. Staking mock pulls in Session + Timestamp + Authorship + Balances).

Total ≈ **160 unit tests across 7 crates**.

### Runtime integration tests (`runtimes/impetus/tests/`)

`ExternalitiesBuilder` produces genesis identical to production impetus,
then drives timelines via `run_to_block` / `run_to_era` helpers.

| File | Scenario |
|---|---|
| `era_progression.rs` | N blocks → `current_era` increments; `active_era.start` set on era boundary |
| `genesis_election.rs` | Genesis triggers `OnChainSeqPhragmen`; top 4 validators elected |
| `session_rotation.rs` | Each 100 blocks → session +1, authorities pulled from staking |
| `bond_lifecycle.rs` | bond → bond_extra → unbond → withdraw_unbonded across `BondingDuration` eras |
| `nominate_payout.rs` | Nominator bond + nominate → era boundary → `payout_stakers` → reward credit per Perbill split |
| `slashing.rs` | Mock offence via `pallet_offences::report_offence` → `SlashDeferDuration` eras pass → balance debited; slashed funds → Treasury |
| `pool_lifecycle.rs` | pool create → join → bond_extra → claim_payout → unbond → withdraw |
| `fast_unstake.rs` | register → manual block advance → `Unstaked` event |
| `treasury_proposal.rs` | proposeSpend → sudo approveProposal → SpendPeriod elapses → beneficiary credited |
| `force_eras.rs` | StakingAdmin forceNewEra / forceNoEras semantics |
| `babe_smoke.rs` | Genesis: epoch_start = 0; 100 blocks → epoch advances; mock equivocation reports surface in `pallet-offences::Event::Offence` |

### E2E tests (`packages/contracts/test/`)

Hardhat + ethers v6 (same pattern as `batch.spec.ts`). For Babe-driven
impetus, ship a **Cargo feature `runtime-test-fast`** that further compresses
constants:

```rust
#[cfg(feature = "runtime-test-fast")]
pub const BLOCKS_PER_SESSION: BlockNumber = 5;       // 30s
#[cfg(feature = "runtime-test-fast")]
pub const SESSIONS_PER_ERA: SessionIndex = 2;        // era = 1 min
#[cfg(feature = "runtime-test-fast")]
pub const BONDING_DURATION_ERAS: EraIndex = 2;       // 2 min
```

E2E build: `cargo build --release --features=runtime-test-fast`. The
**same** chain spec id `impetus_dev_npos` is used; the compile-time feature
swaps in faster runtime constants. The `runtime-test-fast` feature is a
**build-time switch only**, not a separate chain — never mix WASM blobs
built with and without the feature on the same database.

**Spec files:**

| File | Coverage |
|---|---|
| `staking-validator.spec.ts` | bond → setKeys → validate → wait era → payoutStakers → unbond → withdrawUnbonded |
| `staking-nominator.spec.ts` | bond → nominate → wait era → payout → check ledger |
| `staking-rebond.spec.ts` | bond → unbond → rebond before bonding expires → withdraw fails |
| `pools.spec.ts` | NominationPools full lifecycle |
| `treasury.spec.ts` | proposeSpend → sudo approve → wait SpendPeriod → beneficiary balance |
| `fast-unstake.spec.ts` | register → wait batch processing → `Unstaked` event |
| `bags-list.spec.ts` | bond + score change → rebag from another EOA → `ScoreUpdated` |
| `staking-admin.spec.ts` | sudo-gated forceNewEra, setValidatorCount; non-sudo reverts `"NotSudo"` |
| `delegatecall-guard.spec.ts` | Proxy contract delegatecalls each precompile → all revert `"DELEGATECALL/CALLCODE forbidden"` |

**Helpers** (`packages/contracts/test/helpers/`):
- `waitForEra(n)` — poll `Staking.currentEra()` until match (timeout-bounded).
- `advanceToNextSession()` — same pattern at session boundary.
- `getStakingLedger(stash)` — typed wrapper.
- `seedDevValidators()` — ensure 4 dev validators have set session keys and
  are bonded before each suite.

### Non-regression for impulse

`runtimes/impulse/tests/` existing tests (gasless, batch) must still pass —
ensures the dual-path service swap does not regress testnet behavior. Run via
`cargo test -p runtime-impulse`.

### Coverage target

- Unit (per crate): ≥ 80% line coverage via `cargo llvm-cov -p precompile-staking` (etc.).
- Integration: all 11 runtime test scenarios green.
- E2E: 9 spec files green; full impetus E2E suite < 15 min wall-clock with `runtime-test-fast`.
- CI: parallel jobs — unit + integration + impetus E2E + impulse non-regression E2E.

## Risk register

| # | Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|---|
| R1 | `pallet-nomination-pools` derives bonded account via `PalletId::into_sub_account_truncating` — pattern is well-tested with AccountId32, less so with AccountId20 | M | H | Verify `fp_account::AccountId20: AccountIdConversion<PalletId>` compiles + runtime test `pool_account_derives_non_zero_h160` |
| R2 | `pallet-treasury` derives pot account with same pattern as R1 | M | H | Mirror R1: runtime test `treasury_pot_address_non_zero` |
| R3 | Babe equivocation proof generation needs `Historical::prove((KeyTypeId::BABE, authority_id))` data, which is empty before session 1 | L | H | Accept equivocation reports only from session ≥ 1; `babe_smoke.rs` integration test verifies proof generation from a non-zero session |
| R4 | Election multi-phase signed phase (25 blocks ≈ 2.5 min) may be too short for an off-chain miner to submit a solution with 1024 voters | M | M | Dev: rely on `OnChainExecution<OnChainSeqPhragmen>` fallback; signed-phase miner deferred until validator count > 32 |
| R5 | ImOnline unsigned tx priority MAX may starve fee-paying tx within the same block at session boundary | M | M | Cap `MaxKeys = 1024`; set `ImOnlineUnsignedPriority` just below `TransactionPriority::MAX` so fee tx keeps slot priority |
| R6 | Authority-discovery DHT needs ≥ 2 peers to bootstrap; single-node dev chain cannot form a DHT | H | L | Document: single-node mode runs authority-discovery worker but DHT stays empty; does not affect Babe block production. E2E suites use single-node `--alice` mode and are unaffected. |
| R7 | Release binary size grows ~10–15 MB (two import queues + Babe runtime API client + im-online OCW code) | H | L | Accept. Existing `strip = true` release profile already trims debug symbols. |
| R8 | Solidity `setKeys` requires SCALE-encoded `SessionKeys` + ownership proof bytes; wrong encoding silently reverts `InvalidKey` | H | M | Helper script `scripts/dump-session-keys.ts` outputs ready-to-paste bytes; documented in README; `staking-validator.spec.ts` covers happy path |
| R9 | `pallet-fast-unstake` scans all `HistoryDepth` eras each batch; with `BatchSize = 8` it can dominate block weight on a slow chain | M | M | Initial `BatchSize = 4`; admin can tune via `StakingAdmin::control` |
| R10 | Genesis election fails if cumulative stake < `MinimumValidatorCount * MinValidatorBond` → chain panics block 1 | L | C | Build-time `assert!` in `impetus_genesis()` (see Genesis section). 4 × 2000 = 8000 IPT > 1 × 1000. |
| R11 | Future polkadot-sdk minor bumps (stable2603 → stable2606+) may ship `pallet-staking` storage migrations | L | M | Documented: current pallet-staking version pinned via stable2603; any future upgrade must touch `pallet_staking::migrations::*` |
| R12 | Hard kill of validator node mid-session may leave keystore inconsistent | L | H | Document graceful shutdown via SIGTERM; never recommend SIGKILL during E2E |
| R13 | Single binary path-selection by chain spec id is brittle — typo in spec id silently routes to wrong service | L | M | Add a runtime startup assertion that the runtime version reported matches the chain spec id family (impetus ⇒ Babe API exposed; impulse ⇒ Aura API exposed) |

## Rollout — greenfield (user resets chain between phases)

Phases are dependency-ordered; each must `cargo check + cargo test --lib`
green before the next. Estimates assume one senior engineer; multi-agent
parallelism shortens phases 4 and 5 substantially.

| Phase | Scope | Estimate |
|---|---|---|
| 1. Foundation | `runtimes/common` refactor (split `FrontierPrecompiles`, move `SessionKeys` out), workspace dep pinning for Babe + NPoS pallets (not yet wired) | ~1 day |
| 2. Consensus swap (impetus runtime) | Remove pallet-aura from impetus; add pallet-babe + pallet-authorship; update FindAuthor, runtime APIs (BabeApi, AuthorityDiscoveryApi); 4-key SessionKeys; `cargo check -p runtime-impetus` green | ~2 days |
| 3. Node-side dual path | Split `service.rs` into common/aura/babe; `command.rs` dispatch by chain spec id; authority-discovery + im-online worker wiring; smoke test both `--chain impetus` and `--chain impulse` produce blocks | ~1 day |
| 4. NPoS pallets (impetus) | session+historical, staking + reward curve, offences, bags-list, election-provider-multi-phase, authority-discovery, im-online, nomination-pools, treasury, fast-unstake; 11 runtime integration tests | ~3 days |
| 5. Precompiles | One crate per day: staking → session → nomination-pools → fast-unstake → treasury → bags-list → staking-admin; ≥ 20 unit tests each; register into `FrontierPrecompilesNpos` | ~7 days |
| 6. Genesis + E2E | Rewrite `impetus_config()`; ship `scripts/dump-session-keys.ts`; add `runtime-test-fast` feature; 9 E2E spec files | ~2 days |
| 7. Documentation + smoke | Update `apps/node/CLAUDE.md` with NPoS section; operator README; final cross-suite smoke run | ~1 day |

**Total**: ~17 working days solo, or 5–7 days with multi-agent parallelism on
phases 4–5.

## Open questions — to resolve during plan-writing

1. **Bags-list thresholds** — generate offline via `pallet-bags-list/voter-bags`
   binary with economic params from the staking constants module. Plan
   embeds the exact command + output paste.
2. **WASM runtime size limit** — after Phase 4, verify with `wasm-size` that
   impetus runtime stays below the default WASM size limit. If exceeded,
   raise it in `runtime-impetus/build.rs` or split heavy pallets behind
   compile-time features.
3. **ManualSeal pallet inside impetus runtime** — keep at index 11 for
   uniformity with impulse, or remove entirely? Tentative decision: keep
   for now (idle under Babe); cleanup deferred to a follow-up commit.
4. **Election miner client** — confirm `OnChainExecution<OnChainSeqPhragmen>`
   fallback suffices for v1 with `MaxElectingVoters = 1024`. If election
   exceeds block weight, switch to an unsigned miner before mainnet
   promotion.
5. **Benchmark-generated weights** — pallet weights default to `()`
   (Substrate-generic). Production hardening: run
   `cargo run --release --features=runtime-benchmarks -- benchmark pallet
   --pallet pallet_staking --extrinsic '*'` and paste outputs into
   `runtimes/common/src/weights/`. v1 ships defaults.

## Acceptance criteria

1. `cargo build --release` succeeds for both `runtime-impetus` and
   `runtime-impulse`. Single binary `frontier-template-node` boots either.
2. `cargo test --workspace` passes; 7 new precompile crates total ≥ 80% line
   coverage via `cargo llvm-cov`.
3. `cargo clippy --workspace -- -D warnings` passes.
4. `--chain impetus_dev_npos` produces blocks; era progresses through at
   least one full cycle (genesis → era 0 → era 1) within the runtime-test-fast
   timing window.
5. `--chain impulse` and `--chain dev` continue to produce blocks; existing
   E2E suites (gasless, batch) still green.
6. All 9 E2E spec files in `packages/contracts/test/` pass against a fresh
   `impetus_dev_npos` node.
7. Genesis builder build-time assertions catch bad inputs (test by
   temporarily breaking one assertion).
8. DELEGATECALL/CALLCODE guard covers every write-path precompile entry
   (verified by `delegatecall-guard.spec.ts`).
9. `apps/node/CLAUDE.md` updated with the NPoS section and the new
   precompile address map.
10. `spec_version` on impetus bumped from 2 to 3.

## Out-of-scope (deferred, may become future specs)

- Governance v2 (referenda, conviction voting, OpenGov).
- Council / Technical Committee + Democracy pallet → Treasury approval.
- XCM, HRMP, parachain migration of impetus.
- Bridge integrations.
- Proxy accounts for staking operator.
- Multisig validator operator.
- Try-runtime simulation framework.
- Substrate-native UI (polkadot.js apps suffices via secp256k1 signing).
- Validator dashboard + Prometheus metrics surface.
- Slash refund via governance vote.
