# Ponder Indexer for Artemis Betting Chain

## Overview

Analytics indexer for the Artemis betting precompile using [Ponder](https://ponder.sh). Indexes all 4 EVM events emitted by the betting precompile (`0x...0801`) into Postgres, serving analytics via Ponder's built-in GraphQL API.

Lives at `packages/indexer/` inside the existing Turborepo monorepo.

## Scope

1. Fix `PoolClaimed` precompile bug (emits `amount = 0`)
2. New `packages/indexer/` Ponder package
3. 6 database tables for analytics
4. 4 event handlers
5. Turborepo pipeline integration
6. Multi-environment support (dev / testnet / production)
7. E2E test simulating full betting lifecycle

## Precompile Fix

### Problem

`adminClaimPool` in `packages/node/precompiles/betting/src/lib.rs` (line 370-372) emits `PoolClaimed` with `amount = 0` because it does not read the actual transferred amount.

### Solution

Read the pallet betting account's balance (or round pool) before calling `admin_claim_pool`, then emit the difference as the actual claimed amount. Follows the same pattern as `claimWinnings` which reads bet info before the pallet call.

If pallet does not expose a direct pool balance query, calculate from storage: sum of all bet amounts for the round minus already-paid winnings.

## Package Structure

```
packages/indexer/
├── ponder.config.ts          # Chain + contract config
├── ponder.schema.ts          # 6 onchain tables
├── src/
│   └── index.ts              # 4 event handlers
├── package.json              # depends on @betting/shared
├── tsconfig.json
├── .env.local                # DATABASE_URL, RPC URL (gitignored)
└── test/
    └── indexer.e2e.test.ts   # Full lifecycle E2E test
```

## Ponder Configuration

```typescript
// ponder.config.ts
import { createConfig } from "ponder";
import { IBettingPrecompileAbi } from "@betting/shared";

export default createConfig({
  chains: {
    artemis: {
      id: 322,
      rpc: process.env.PONDER_RPC_URL_322,
      disableCache: process.env.NODE_ENV !== "production",
    },
  },
  contracts: {
    BettingPrecompile: {
      abi: IBettingPrecompileAbi,
      chain: "artemis",
      address: "0x0000000000000000000000000000000000000801",
      startBlock: 1,
    },
  },
});
```

### Environment Variables

| Variable | Dev | Testnet/Prod |
|----------|-----|-------------|
| `PONDER_RPC_URL_322` | `http://127.0.0.1:9944` | remote RPC URL |
| `DATABASE_URL` | local Postgres | managed Postgres |

## Schema

6 tables defined with `onchainTable`:

### `round`

| Column | Type | Notes |
|--------|------|-------|
| `id` | bigint | PK, round ID (day number in GMT+7) |
| `status` | integer | 0=Open, 1=Closed, 2=Resolved, 3=Settled |
| `winningNumber` | integer | nullable, 0-99 |
| `totalBets` | integer | count of bets placed |
| `totalVolume` | bigint | sum of all bet amounts |
| `totalPayout` | bigint | sum of all winner payouts |
| `poolClaimed` | bigint | amount admin claimed |
| `resolvedAt` | bigint | nullable, block timestamp |

### `bet`

| Column | Type | Notes |
|--------|------|-------|
| `id` | text | PK, `{roundId}-{userAddress}` |
| `roundId` | bigint | indexed |
| `user` | hex | indexed |
| `number` | integer | 0-99 |
| `token` | hex | token address |
| `amount` | bigint | wager amount |
| `claimed` | boolean | default false |
| `payout` | bigint | 0 if lost or unclaimed |
| `timestamp` | bigint | block timestamp |

### `userStats`

| Column | Type | Notes |
|--------|------|-------|
| `address` | hex | PK |
| `totalBets` | integer | |
| `totalWagered` | bigint | |
| `totalWon` | bigint | total payout received |
| `totalClaimed` | bigint | same as totalWon (claim is binary) |
| `winCount` | integer | |

### `dailyStats`

| Column | Type | Notes |
|--------|------|-------|
| `id` | text | PK, round ID as string |
| `totalBets` | integer | |
| `totalVolume` | bigint | |
| `totalPayout` | bigint | |
| `uniqueUsers` | integer | distinct bettors |

### `numberStats`

| Column | Type | Notes |
|--------|------|-------|
| `number` | integer | PK, 0-99 |
| `timesPicked` | integer | |
| `timesWon` | integer | |

### `protocolStats`

| Column | Type | Notes |
|--------|------|-------|
| `id` | text | PK, "global" singleton |
| `totalRounds` | integer | |
| `totalBets` | integer | |
| `totalVolume` | bigint | |
| `totalPayout` | bigint | |
| `totalPoolClaimed` | bigint | |

## Event Handlers

### `BetPlaced`

```
args: { roundId, user, number, token, amount }
```

1. Upsert `round` — create if new, increment `totalBets`, add to `totalVolume`
2. Insert `bet` — id = `{roundId}-{user}`
3. Upsert `userStats` — increment `totalBets`, add `totalWagered`
4. Upsert `dailyStats` — increment counters; check if user already has bet in this round to decide whether to increment `uniqueUsers`
5. Upsert `numberStats` — increment `timesPicked` for the chosen number
6. Upsert `protocolStats` — increment global `totalBets` and `totalVolume`

### `ResultSubmitted`

```
args: { roundId, number }
```

1. Update `round` — set `winningNumber = number`, `status = 2` (Resolved), `resolvedAt = block.timestamp`
2. Upsert `numberStats` — increment `timesWon` for the winning number
3. Upsert `protocolStats` — increment `totalRounds`

### `WinningsClaimed`

```
args: { roundId, user, token, amount }
```

1. Update `bet` — set `claimed = true`, `payout = amount`
2. Update `round` — add `amount` to `totalPayout`
3. Upsert `userStats` — increment `winCount`, add to `totalWon` and `totalClaimed`
4. Upsert `protocolStats` — add to `totalPayout`

### `PoolClaimed`

```
args: { roundId, admin, token, amount }
```

1. Update `round` — set `poolClaimed = amount`, `status = 3` (Settled)
2. Upsert `protocolStats` — add to `totalPoolClaimed`

## Unique Users Tracking

For `dailyStats.uniqueUsers`: before incrementing, query the `bet` table to check if a bet already exists for `{roundId}-{user}`. If no existing bet, increment `uniqueUsers`. This works because Ponder processes events sequentially and the insert happens in the same handler.

## Turborepo Integration

Add to `turbo.json`:

```json
{
  "indexer:dev": {
    "cache": false,
    "persistent": true
  },
  "indexer:build": {
    "dependsOn": ["^build"]
  }
}
```

The indexer `build` task depends on `@betting/shared` being built first (for ABI and types).

## E2E Test

### Prerequisites

- Substrate dev node running (`--dev` mode)
- Ponder indexer running and synced
- Local Postgres running

### Test Scenario: Full Betting Lifecycle

Uses dev accounts from the chain genesis:

| Account | Address | Role in test |
|---------|---------|-------------|
| #0 | `0xf39F...2266` | admin |
| #1 | `0x7099...79C8` | bettor (winner) |
| #2 | `0x3C44...93BC` | bettor (loser) |
| #3 | `0x90F7...3906` | bettor (loser) |
| #4 | `0x15d3...6A65` | bettor (loser) |

### Steps

1. **Place bets** — 4 users each place a bet on different numbers via precompile
   - User #1 bets on number X (will be the winning number)
   - Users #2-#4 bet on other numbers
   - Various amounts to test volume tracking

2. **Force close round** — admin calls `forceCloseRound` (testing utility)

3. **Submit result** — admin submits winning number = X

4. **Winner claims** — user #1 calls `claimWinnings`

5. **Admin claims pool** — admin calls `adminClaimPool`

6. **Wait for indexer** — poll Ponder GraphQL API until data is indexed

7. **Assert indexed data:**
   - `round`: totalBets=4, totalVolume=sum of all bets, winningNumber=X, status=Settled, totalPayout matches winner payout, poolClaimed > 0
   - `bet`: 4 records, winner has claimed=true and payout > 0, losers have claimed=false and payout=0
   - `userStats`: winner has winCount=1, all users have totalBets=1
   - `dailyStats`: uniqueUsers=4, totalBets=4
   - `numberStats`: winning number has timesWon=1
   - `protocolStats`: totalRounds=1, totalBets=4, totals match

### Test Implementation

Test file at `packages/indexer/test/indexer.e2e.test.ts`. Uses viem to send transactions to the precompile and fetch/assert from Ponder GraphQL API. Reuses wallet setup pattern from `packages/contracts/test/helpers/setup.ts`.

## Dependencies

### `packages/indexer/package.json`

```json
{
  "name": "@betting/indexer",
  "private": true,
  "dependencies": {
    "@betting/shared": "workspace:*",
    "ponder": "latest"
  },
  "devDependencies": {
    "typescript": "^5.5.0",
    "viem": "^2.0.0",
    "vitest": "^3.0.0"
  }
}
```

## Out of Scope

- Frontend dashboard UI
- Real-time WebSocket subscriptions
- Historical backfill for rounds before indexer deployment
- Multi-token analytics (only native ART tracked initially)
- Rate limiting or auth on the GraphQL API
