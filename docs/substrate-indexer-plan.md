# Substrate indexer plan (Subsquid) — NPoS staking & pools

Standard approach (matches Moonbeam/Astar): index the **Substrate events** that
`pallet-staking` and `pallet-nomination-pools` already emit, instead of emitting
duplicate EVM logs from the precompiles. No runtime change, and it captures
**all** activity — whether triggered via the EVM precompiles, native extrinsics
(polkadot.js), or genesis — which the EVM-log approach cannot.

Ponder stays EVM-only (gasless registry). This is a **separate** indexer with
its own processor, Postgres DB, and GraphQL API, running alongside Ponder.

## Why not Ponder
Ponder reads EVM logs via `eth_getLogs`. The staking/pools precompiles emit no
EVM logs, and NPoS election/era state lives in Substrate storage. Only a
Substrate-aware processor (Subsquid `@subsquid/substrate-processor`) can read
pallet events, calls, and storage.

## Chain specifics that simplify this
- **AccountId20**: the runtime uses `EnsureAccountId20` + `IdentityAddressMapping`,
  so the native `AccountId` **is** the 20-byte H160. Substrate events carry the
  EVM address directly — **no SS58 ↔ H160 conversion**, just hex-encode the
  20 bytes. Account ids in the squid match the dApp's `0x…` addresses 1:1.
- **No public Subsquid archive** for Impetus → run the processor in **RPC-only**
  mode (`setRpcEndpoint(...)`, no gateway). Point at an **archive** node RPC
  (full history); the pruned rpc node can't serve old blocks.

## Data sources (events / calls / storage)

### pallet-staking
| Signal | Type | Use |
|---|---|---|
| `Staking.Bonded { stash, amount }` | event | stake increase |
| `Staking.Unbonded { stash, amount }` | event | unbonding |
| `Staking.Withdrawn { stash, amount }` | event | withdraw unbonded |
| `Staking.ValidatorPrefsSet { stash, prefs }` | event | validator intent + commission |
| `Staking.Chilled { stash }` | event | stops validating/nominating |
| `Staking.Kicked { nominator, stash }` | event | removed from a validator |
| `Staking.PayoutStarted { era_index, validator_stash }` | event | reward payout |
| `Staking.Rewarded { stash, dest, amount }` | event | reward credited |
| `Staking.Slashed { staker, amount }` | event | slash |
| `Staking.StakersElected` | event | new active set elected (era boundary) |
| **`Staking.nominate { targets }`** | **call** | nominators — pallet-staking emits **NO `Nominated` event**; index the extrinsic call (signer = nominator) to capture targets |
| `Session.Validators` / `Staking.ErasStakersOverview` | storage | the **active validator set** per era (read on `StakersElected` / `NewSession`) |

> Gotcha: there is no `Nominated` event. Track nominators via the `nominate`
> **call** (or by snapshotting `Staking.Nominators` storage on era change).

### pallet-nomination-pools
| Signal | Type | Use |
|---|---|---|
| `NominationPools.Created { depositor, pool_id }` | event | new pool |
| `NominationPools.Bonded { member, pool_id, bonded, joined }` | event | join / bond extra |
| `NominationPools.PaidOut { member, pool_id, payout }` | event | reward claimed |
| `NominationPools.Unbonded { member, pool_id, balance, points, era }` | event | unbond |
| `NominationPools.Withdrawn { member, pool_id, balance, points }` | event | withdraw |
| `NominationPools.StateChanged { pool_id, new_state }` | event | open/blocked/destroying |
| `NominationPools.MemberRemoved { pool_id, member }` | event | exit |
| `NominationPools.PoolCommissionUpdated` | event | commission |

## Entities (schema.graphql)

```graphql
type Validator @entity {
  id: ID!                 # stash address (0x…)
  commission: Int!        # percent
  blocked: Boolean!
  active: Boolean!        # in current era's elected set
  selfBonded: BigInt!
  updatedAt: Int!         # block height
}

type Nominator @entity {
  id: ID!                 # stash address
  targets: [String!]!     # validator stash addresses
  active: Boolean!
  updatedAt: Int!
}

type Era @entity {
  id: ID!                 # era index
  validatorReward: BigInt
  start: Int!             # block height of election
}

type Payout @entity {
  id: ID!                 # `${validator}-${era}`
  validator: Validator!
  era: Int!
  block: Int!
}

type Pool @entity {
  id: ID!                 # pool id
  creator: String!
  state: String!          # Open | Blocked | Destroying
  totalBonded: BigInt!
  memberCount: Int!
  commission: Int!
  createdAt: Int!
}

type PoolMember @entity {
  id: ID!                 # member address
  pool: Pool!
  points: BigInt!
  lastClaimBlock: Int
  updatedAt: Int!
}
```

## Processor sketch (`src/processor.ts`)

```ts
import { SubstrateBatchProcessor } from "@subsquid/substrate-processor";

export const processor = new SubstrateBatchProcessor()
  // RPC-only — no public archive for Impetus. Point at an ARCHIVE node.
  .setRpcEndpoint({ url: process.env.RPC_ENDPOINT!, rateLimit: 10 })
  .setBlockRange({ from: 0 })
  .addEvent({
    name: [
      "Staking.Bonded", "Staking.Unbonded", "Staking.Withdrawn",
      "Staking.ValidatorPrefsSet", "Staking.Chilled", "Staking.Kicked",
      "Staking.PayoutStarted", "Staking.Rewarded", "Staking.Slashed",
      "Staking.StakersElected",
      "NominationPools.Created", "NominationPools.Bonded",
      "NominationPools.PaidOut", "NominationPools.Unbonded",
      "NominationPools.Withdrawn", "NominationPools.StateChanged",
      "NominationPools.MemberRemoved", "NominationPools.PoolCommissionUpdated",
    ],
    extrinsic: true,
  })
  .addCall({ name: ["Staking.nominate"], extrinsic: true })
  .setFields({ event: { args: true }, extrinsic: { signature: true } });
```

`src/main.ts` runs `processor.run(db, async (ctx) => { ... })`, switching on
`item.name`, upserting the entities above. On `Staking.StakersElected`, read
`Session.Validators` storage to flip `Validator.active`. Hex-encode AccountId
bytes directly (they are already 20-byte H160).

## Steps to build
1. `npx @subsquid/cli@latest init impetus-squid -t substrate` → new project.
2. Put the chain metadata in place and run **`sqd typegen`** (needs the runtime
   metadata for spec 9) to generate typed `events.staking.bonded` decoders.
3. Paste the schema above into `schema.graphql`; `sqd codegen`.
4. Implement `src/main.ts` handlers (upserts).
5. `sqd up` (Postgres) → `sqd process` → `sqd serve` (GraphQL on :4350).
6. Deploy: a service + Postgres (Dockerfile from the squid template); set
   `RPC_ENDPOINT` to an **archive** node, `DB_*` to the Postgres.
7. dApp: add `NEXT_PUBLIC_SQUID_URL`; switch `/validators`, `/pools` to read the
   squid GraphQL (today they read the precompile directly via wagmi + the
   `KNOWN_VALIDATORS` config — keep that as fallback until the squid is live).

## Division of indexers
- **Ponder** (`apps/indexer`): EVM logs → gasless registry. Mainnet 388266.
- **Subsquid** (this plan): Substrate events/calls/storage → staking, pools,
  validators, eras, payouts. Captures native + EVM + genesis activity.
```
