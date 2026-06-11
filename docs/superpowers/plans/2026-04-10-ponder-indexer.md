# Ponder Indexer Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a Ponder-based analytics indexer for the Artemis betting precompile, including a precompile bug fix and full lifecycle E2E test.

**Architecture:** Ponder package inside the Turborepo monorepo at `packages/indexer/`. Indexes 4 EVM events from the betting precompile into 6 Postgres tables. ABI and constants imported from `@betting/shared`.

**Tech Stack:** Ponder, TypeScript, Postgres, viem, vitest

---

## File Map

| Action | Path | Responsibility |
|--------|------|----------------|
| Modify | `packages/node/precompiles/betting/src/lib.rs` | Fix PoolClaimed amount=0 bug |
| Create | `packages/indexer/package.json` | Package definition, dependencies |
| Create | `packages/indexer/tsconfig.json` | TypeScript config |
| Create | `packages/indexer/ponder.config.ts` | Chain + contract config |
| Create | `packages/indexer/ponder.schema.ts` | 6 database tables |
| Create | `packages/indexer/src/index.ts` | 4 event handlers |
| Create | `packages/indexer/.env.local` | Local env vars |
| Create | `packages/indexer/.gitignore` | Ignore .env.local, ponder generated |
| Modify | `pnpm-workspace.yaml` | Add packages/indexer |
| Modify | `turbo.json` | Add indexer tasks |
| Create | `packages/indexer/test/indexer.e2e.test.ts` | Full lifecycle E2E test |

---

### Task 1: Fix PoolClaimed Precompile Bug

**Files:**
- Modify: `packages/node/precompiles/betting/src/lib.rs:353-385`

The precompile emits `PoolClaimed` with `amount = 0`. Fix: read the pallet account's native balance before calling `admin_claim_pool`, then after, compute `claimed = before - after`.

- [ ] **Step 1: Add Currency trait import**

In `packages/node/precompiles/betting/src/lib.rs`, add `Currency` to imports:

```rust
use frame_support::traits::{Currency, UnixTime};
```

Replace the existing line 16:
```rust
use frame_support::traits::UnixTime;
```

- [ ] **Step 2: Replace the adminClaimPool handler block**

Replace lines 353-385 (the `selector_admin_claim_pool` branch) with:

```rust
		} else if sel == selector_admin_claim_pool() {
			handle.record_cost(GAS_COST_WRITE)?;

			let round_id = decode_u32_from_u256(&data, 0)? as RoundId;

			let caller_h160 = handle.context().caller;
			let caller = <R as pallet_evm::Config>::AddressMapping::into_account_id(
				caller_h160,
			);

			// Read pallet account balance BEFORE claim to compute actual amount
			let pallet_account = pallet_betting::Pallet::<R>::account_id();
			let balance_before: u128 = <R as pallet_betting::Config>::Currency::free_balance(&pallet_account).into();

			let origin: <R as frame_system::Config>::RuntimeOrigin =
				RawOrigin::Signed(caller).into();

			pallet_betting::Pallet::<R>::admin_claim_pool(origin, round_id)
				.map_err(dispatch_error_to_precompile_failure)?;

			// Read balance AFTER claim — difference is the actual claimed amount
			let balance_after: u128 = <R as pallet_betting::Config>::Currency::free_balance(&pallet_account).into();
			let claimed_amount = balance_before.saturating_sub(balance_after);

			// Emit PoolClaimed(uint256 indexed roundId, address indexed admin, address token, uint256 amount)
			let mut log_data = Vec::with_capacity(64);
			log_data.extend_from_slice(&encode_address(H160::zero())); // native token
			log_data.extend_from_slice(&encode_u128(claimed_amount));
			handle.log(
				handle.code_address(),
				vec![
					event_topic("PoolClaimed(uint256,address,address,uint256)"),
					u256_to_topic(U256::from(round_id)),
					address_to_topic(caller_h160),
				],
				log_data,
			).map_err(|_| PrecompileFailure::Error {
				exit_status: ExitError::Other(Cow::Borrowed("Failed to emit log")),
			})?;

			ok_empty()
```

- [ ] **Step 3: Verify Rust compiles**

Run:
```bash
cd packages/node && cargo check -p precompile-betting
```

Expected: compiles with no errors. If `Currency` trait bound is missing on `R`, add it to the `where` clause on the `impl` block (line 216-221). The existing bound `R: pallet_betting::Config` already requires `Currency` via `Config::Currency`, so this should work.

- [ ] **Step 4: Run pallet unit tests**

Run:
```bash
cd packages/node && cargo test -p pallet-betting
```

Expected: all existing tests pass (precompile change doesn't affect pallet logic).

- [ ] **Step 5: Commit**

```bash
git add packages/node/precompiles/betting/src/lib.rs
git commit -m "fix(precompile): emit actual amount in PoolClaimed event

Read pallet account balance before/after admin_claim_pool to compute
the real claimed amount instead of hardcoding 0."
```

---

### Task 2: Scaffold Indexer Package

**Files:**
- Create: `packages/indexer/package.json`
- Create: `packages/indexer/tsconfig.json`
- Create: `packages/indexer/.env.local`
- Create: `packages/indexer/.gitignore`
- Modify: `pnpm-workspace.yaml`
- Modify: `turbo.json`

- [ ] **Step 1: Create package.json**

Create `packages/indexer/package.json`:

```json
{
  "name": "@betting/indexer",
  "version": "0.0.0",
  "private": true,
  "type": "module",
  "scripts": {
    "dev": "ponder dev",
    "start": "ponder start",
    "build": "ponder build"
  },
  "dependencies": {
    "@betting/shared": "workspace:*",
    "ponder": "^0.9"
  },
  "devDependencies": {
    "@types/node": "^22.0.0",
    "typescript": "^5.5.0",
    "viem": "^2.0.0",
    "vitest": "^3.0.0"
  }
}
```

Note: Ponder version `^0.9` — check latest on npm during install. Adjust if needed.

- [ ] **Step 2: Create tsconfig.json**

Create `packages/indexer/tsconfig.json`:

```json
{
  "compilerOptions": {
    "target": "ES2022",
    "module": "ES2022",
    "moduleResolution": "bundler",
    "strict": true,
    "esModuleInterop": true,
    "skipLibCheck": true,
    "forceConsistentCasingInFileNames": true,
    "resolveJsonModule": true,
    "sourceMap": true,
    "outDir": "./dist",
    "rootDir": "."
  },
  "include": ["ponder.config.ts", "ponder.schema.ts", "src/**/*.ts"]
}
```

- [ ] **Step 3: Create .env.local**

Create `packages/indexer/.env.local`:

```
DATABASE_URL=postgresql://localhost:5432/betting_indexer
PONDER_RPC_URL_322=http://127.0.0.1:9944
```

- [ ] **Step 4: Create .gitignore**

Create `packages/indexer/.gitignore`:

```
.env.local
.ponder/
generated/
node_modules/
dist/
```

- [ ] **Step 5: Add to pnpm workspace**

Modify `pnpm-workspace.yaml` — add `"packages/indexer"` to the packages list:

```yaml
packages:
  - "packages/contracts"
  - "packages/shared"
  - "packages/webapp"
  - "packages/indexer"
```

- [ ] **Step 6: Add Turborepo tasks**

Modify `turbo.json` — add two new tasks to the `tasks` object:

```json
{
  "$schema": "https://turbo.build/schema.json",
  "tasks": {
    "build": {
      "dependsOn": ["^build"],
      "outputs": ["dist/**", "artifacts/**", "typechain-types/**"]
    },
    "test": {
      "dependsOn": ["build"]
    },
    "copy-abi": {
      "dependsOn": ["^build"],
      "outputs": ["abis/**"]
    },
    "lint": {},
    "node:build": {
      "cache": false
    },
    "dev": {
      "cache": false,
      "persistent": true
    }
  }
}
```

The `dev` task is generic (not `indexer:dev`) so it can be filtered: `turbo run dev --filter=@betting/indexer`.

- [ ] **Step 7: Install dependencies**

Run:
```bash
cd /Users/huyduan/projects/blockchain && pnpm install
```

Expected: lockfile updates, ponder and deps installed in `packages/indexer/node_modules`.

- [ ] **Step 8: Commit**

```bash
git add packages/indexer/package.json packages/indexer/tsconfig.json packages/indexer/.gitignore pnpm-workspace.yaml turbo.json pnpm-lock.yaml
git commit -m "chore(indexer): scaffold ponder package

Add @betting/indexer to monorepo with ponder, typescript, viem, vitest.
Register in pnpm workspace and turbo pipeline."
```

---

### Task 3: Ponder Config

**Files:**
- Create: `packages/indexer/ponder.config.ts`

- [ ] **Step 1: Create ponder.config.ts**

Create `packages/indexer/ponder.config.ts`:

```typescript
import { createConfig } from "ponder";
import { http } from "viem";
import { BETTING_PRECOMPILE_ADDRESS } from "@betting/shared";
import { IBettingPrecompileAbi } from "@betting/shared";

export default createConfig({
  chains: {
    artemis: {
      id: 322,
      transport: http(process.env.PONDER_RPC_URL_322),
    },
  },
  contracts: {
    BettingPrecompile: {
      abi: IBettingPrecompileAbi,
      network: "artemis",
      address: BETTING_PRECOMPILE_ADDRESS,
      startBlock: 0,
    },
  },
});
```

Note: Ponder API may use `chains`/`chain` or `networks`/`network` depending on version. The above uses the 0.9.x API with `networks`/`network` and `transport`. If using newer API, adjust to `chains`/`chain` with `rpc` string. Verify against installed Ponder version after `pnpm install`.

- [ ] **Step 2: Verify Ponder can parse config**

Run:
```bash
cd packages/indexer && npx ponder build
```

Expected: Ponder reads config without errors. If there are import resolution issues with `@betting/shared`, ensure `packages/shared` is built first:

```bash
cd /Users/huyduan/projects/blockchain && pnpm turbo build --filter=@betting/shared
```

Then retry.

- [ ] **Step 3: Commit**

```bash
git add packages/indexer/ponder.config.ts
git commit -m "feat(indexer): add ponder config for Artemis chain"
```

---

### Task 4: Schema Definition

**Files:**
- Create: `packages/indexer/ponder.schema.ts`

- [ ] **Step 1: Create ponder.schema.ts with all 6 tables**

Create `packages/indexer/ponder.schema.ts`:

```typescript
import { index, onchainTable } from "ponder";

export const round = onchainTable("round", (t) => ({
  id: t.bigint().primaryKey(),
  status: t.integer().notNull().default(0),
  winningNumber: t.integer(),
  totalBets: t.integer().notNull().default(0),
  totalVolume: t.bigint().notNull().default(0n),
  totalPayout: t.bigint().notNull().default(0n),
  poolClaimed: t.bigint().notNull().default(0n),
  resolvedAt: t.bigint(),
}));

export const bet = onchainTable(
  "bet",
  (t) => ({
    id: t.text().primaryKey(),
    roundId: t.bigint().notNull(),
    user: t.hex().notNull(),
    number: t.integer().notNull(),
    token: t.hex().notNull(),
    amount: t.bigint().notNull(),
    claimed: t.boolean().notNull().default(false),
    payout: t.bigint().notNull().default(0n),
    timestamp: t.bigint().notNull(),
  }),
  (table) => ({
    roundIdx: index().on(table.roundId),
    userIdx: index().on(table.user),
  }),
);

export const userStats = onchainTable("user_stats", (t) => ({
  address: t.hex().primaryKey(),
  totalBets: t.integer().notNull().default(0),
  totalWagered: t.bigint().notNull().default(0n),
  totalWon: t.bigint().notNull().default(0n),
  totalClaimed: t.bigint().notNull().default(0n),
  winCount: t.integer().notNull().default(0),
}));

export const dailyStats = onchainTable("daily_stats", (t) => ({
  id: t.text().primaryKey(),
  totalBets: t.integer().notNull().default(0),
  totalVolume: t.bigint().notNull().default(0n),
  totalPayout: t.bigint().notNull().default(0n),
  uniqueUsers: t.integer().notNull().default(0),
}));

export const numberStats = onchainTable("number_stats", (t) => ({
  number: t.integer().primaryKey(),
  timesPicked: t.integer().notNull().default(0),
  timesWon: t.integer().notNull().default(0),
}));

export const protocolStats = onchainTable("protocol_stats", (t) => ({
  id: t.text().primaryKey(),
  totalRounds: t.integer().notNull().default(0),
  totalBets: t.integer().notNull().default(0),
  totalVolume: t.bigint().notNull().default(0n),
  totalPayout: t.bigint().notNull().default(0n),
  totalPoolClaimed: t.bigint().notNull().default(0n),
}));
```

Note: Ponder's `onchainTable` API may not support `.default()`. If it doesn't, remove the `.default()` calls and set defaults explicitly in handlers. Verify against Ponder docs after install.

- [ ] **Step 2: Verify schema parses**

Run:
```bash
cd packages/indexer && npx ponder build
```

Expected: no schema errors. If `onchainTable` API differs, adjust per Ponder docs (e.g., older versions use `createSchema` with `p.createTable`).

- [ ] **Step 3: Commit**

```bash
git add packages/indexer/ponder.schema.ts
git commit -m "feat(indexer): define 6 analytics tables in ponder schema"
```

---

### Task 5: Event Handlers

**Files:**
- Create: `packages/indexer/src/index.ts`

- [ ] **Step 1: Create src/index.ts with all 4 handlers**

Create `packages/indexer/src/index.ts`:

```typescript
import { ponder } from "ponder:registry";
import {
  round,
  bet,
  userStats,
  dailyStats,
  numberStats,
  protocolStats,
} from "ponder:schema";

ponder.on("BettingPrecompile:BetPlaced", async ({ event, context }) => {
  const { db } = context;
  const roundId = event.args.roundId;
  const user = event.args.user;
  const number = event.args.number;
  const token = event.args.token;
  const amount = event.args.amount;
  const betId = `${roundId}-${user}`;
  const dayId = roundId.toString();

  // Check if user already has a bet in this round (for uniqueUsers tracking)
  const existingBet = await db.find(bet, { id: betId });

  // Upsert round
  await db
    .insert(round)
    .values({
      id: roundId,
      status: 0,
      totalBets: 1,
      totalVolume: amount,
      totalPayout: 0n,
      poolClaimed: 0n,
    })
    .onConflictDoUpdate((row) => ({
      totalBets: row.totalBets + 1,
      totalVolume: row.totalVolume + amount,
    }));

  // Insert bet
  await db.insert(bet).values({
    id: betId,
    roundId,
    user,
    number,
    token,
    amount,
    claimed: false,
    payout: 0n,
    timestamp: event.block.timestamp,
  });

  // Upsert userStats
  await db
    .insert(userStats)
    .values({
      address: user,
      totalBets: 1,
      totalWagered: amount,
      totalWon: 0n,
      totalClaimed: 0n,
      winCount: 0,
    })
    .onConflictDoUpdate((row) => ({
      totalBets: row.totalBets + 1,
      totalWagered: row.totalWagered + amount,
    }));

  // Upsert dailyStats
  const uniqueIncrement = existingBet ? 0 : 1;
  await db
    .insert(dailyStats)
    .values({
      id: dayId,
      totalBets: 1,
      totalVolume: amount,
      totalPayout: 0n,
      uniqueUsers: 1,
    })
    .onConflictDoUpdate((row) => ({
      totalBets: row.totalBets + 1,
      totalVolume: row.totalVolume + amount,
      uniqueUsers: row.uniqueUsers + uniqueIncrement,
    }));

  // Upsert numberStats
  await db
    .insert(numberStats)
    .values({
      number,
      timesPicked: 1,
      timesWon: 0,
    })
    .onConflictDoUpdate((row) => ({
      timesPicked: row.timesPicked + 1,
    }));

  // Upsert protocolStats
  await db
    .insert(protocolStats)
    .values({
      id: "global",
      totalRounds: 0,
      totalBets: 1,
      totalVolume: amount,
      totalPayout: 0n,
      totalPoolClaimed: 0n,
    })
    .onConflictDoUpdate((row) => ({
      totalBets: row.totalBets + 1,
      totalVolume: row.totalVolume + amount,
    }));
});

ponder.on("BettingPrecompile:ResultSubmitted", async ({ event, context }) => {
  const { db } = context;
  const roundId = event.args.roundId;
  const number = event.args.number;

  // Update round
  await db.update(round, { id: roundId }).set({
    status: 2,
    winningNumber: number,
    resolvedAt: event.block.timestamp,
  });

  // Upsert numberStats
  await db
    .insert(numberStats)
    .values({
      number,
      timesPicked: 0,
      timesWon: 1,
    })
    .onConflictDoUpdate((row) => ({
      timesWon: row.timesWon + 1,
    }));

  // Upsert protocolStats
  await db
    .insert(protocolStats)
    .values({
      id: "global",
      totalRounds: 1,
      totalBets: 0,
      totalVolume: 0n,
      totalPayout: 0n,
      totalPoolClaimed: 0n,
    })
    .onConflictDoUpdate((row) => ({
      totalRounds: row.totalRounds + 1,
    }));
});

ponder.on("BettingPrecompile:WinningsClaimed", async ({ event, context }) => {
  const { db } = context;
  const roundId = event.args.roundId;
  const user = event.args.user;
  const amount = event.args.amount;
  const betId = `${roundId}-${user}`;
  const dayId = roundId.toString();

  // Update bet
  await db.update(bet, { id: betId }).set({
    claimed: true,
    payout: amount,
  });

  // Update round
  const currentRound = await db.find(round, { id: roundId });
  if (currentRound) {
    await db.update(round, { id: roundId }).set({
      totalPayout: currentRound.totalPayout + amount,
    });
  }

  // Upsert userStats
  await db
    .insert(userStats)
    .values({
      address: user,
      totalBets: 0,
      totalWagered: 0n,
      totalWon: amount,
      totalClaimed: amount,
      winCount: 1,
    })
    .onConflictDoUpdate((row) => ({
      totalWon: row.totalWon + amount,
      totalClaimed: row.totalClaimed + amount,
      winCount: row.winCount + 1,
    }));

  // Update dailyStats
  const currentDaily = await db.find(dailyStats, { id: dayId });
  if (currentDaily) {
    await db.update(dailyStats, { id: dayId }).set({
      totalPayout: currentDaily.totalPayout + amount,
    });
  }

  // Update protocolStats
  const currentProtocol = await db.find(protocolStats, { id: "global" });
  if (currentProtocol) {
    await db.update(protocolStats, { id: "global" }).set({
      totalPayout: currentProtocol.totalPayout + amount,
    });
  }
});

ponder.on("BettingPrecompile:PoolClaimed", async ({ event, context }) => {
  const { db } = context;
  const roundId = event.args.roundId;
  const amount = event.args.amount;

  // Update round
  await db.update(round, { id: roundId }).set({
    poolClaimed: amount,
    status: 3,
  });

  // Update protocolStats
  const currentProtocol = await db.find(protocolStats, { id: "global" });
  if (currentProtocol) {
    await db.update(protocolStats, { id: "global" }).set({
      totalPoolClaimed: currentProtocol.totalPoolClaimed + amount,
    });
  }
});
```

Note: Ponder's `db` API may differ from the above. Key variations:
- `db.find(table, { id })` might be `db.find(table, id)` for single PK
- `onConflictDoUpdate((row) => ...)` callback style may differ
- If API doesn't support row callbacks, use read-then-write pattern

Verify against installed Ponder version and adjust.

- [ ] **Step 2: Verify Ponder build**

Run:
```bash
cd packages/indexer && npx ponder build
```

Expected: builds without errors. Fix any API mismatches.

- [ ] **Step 3: Commit**

```bash
git add packages/indexer/src/index.ts
git commit -m "feat(indexer): implement 4 event handlers for betting analytics"
```

---

### Task 6: Verify Indexer Against Live Dev Node

**Prerequisites:** Substrate dev node running, Postgres running, database `betting_indexer` created.

- [ ] **Step 1: Create Postgres database**

Run:
```bash
createdb betting_indexer
```

If Postgres is not running, start it first. If `createdb` is not available, use:
```bash
psql -c "CREATE DATABASE betting_indexer;"
```

- [ ] **Step 2: Build shared package**

Run:
```bash
cd /Users/huyduan/projects/blockchain && pnpm turbo build --filter=@betting/shared
```

Expected: shared package builds, `dist/` and `abis/` available.

- [ ] **Step 3: Start Ponder dev server**

Run:
```bash
cd packages/indexer && pnpm dev
```

Expected: Ponder starts, connects to RPC at `http://127.0.0.1:9944`, creates tables in Postgres, begins syncing from block 0. If the dev node is not running, Ponder will retry connections — that's fine.

Watch for errors related to:
- ABI import issues (fix imports in config)
- Schema validation errors (fix schema)
- RPC connection failures (ensure node is running)

- [ ] **Step 4: Verify tables exist**

In another terminal:
```bash
psql betting_indexer -c "\dt"
```

Expected: 6 tables listed (round, bet, user_stats, daily_stats, number_stats, protocol_stats).

- [ ] **Step 5: Stop dev server and commit if any fixes were needed**

If you made adjustments to config/schema/handlers, commit:
```bash
git add packages/indexer/
git commit -m "fix(indexer): adjust ponder config for runtime compatibility"
```

---

### Task 7: E2E Test

**Files:**
- Create: `packages/indexer/test/indexer.e2e.test.ts`

**Prerequisites:** Substrate dev node running, Ponder dev server running and synced, Postgres with `betting_indexer` database.

- [ ] **Step 1: Create E2E test file**

Create `packages/indexer/test/indexer.e2e.test.ts`:

```typescript
import { describe, it, expect, beforeAll } from "vitest";
import {
  createWalletClient,
  createPublicClient,
  http,
  parseEther,
  getContract,
  type WalletClient,
  type PublicClient,
  type GetContractReturnType,
} from "viem";
import { privateKeyToAccount } from "viem/accounts";
import {
  BETTING_PRECOMPILE_ADDRESS,
  NATIVE_TOKEN_ADDRESS,
  IBettingPrecompileAbi,
} from "@betting/shared";

const ARTEMIS_CHAIN = {
  id: 322,
  name: "Artemis",
  nativeCurrency: { name: "ART", symbol: "ART", decimals: 18 },
  rpcUrls: {
    default: { http: ["http://127.0.0.1:9944"] },
  },
} as const;

const RPC_URL = "http://127.0.0.1:9944";
const PONDER_GRAPHQL_URL = "http://localhost:42069/graphql";

// Dev accounts from genesis
const ACCOUNTS = {
  admin: privateKeyToAccount("0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80"),
  user1: privateKeyToAccount("0x59c6995e998f97a5a0044966f0945389dc9e86dae88c7a8412f4603b6b78690d"),
  user2: privateKeyToAccount("0x5de4111afa1a4b94908f83103eb1f1706367c2e68ca870fc3fb9a804cdab365a"),
  user3: privateKeyToAccount("0x7c852118294e51e653712a81e05800f419141751be58f605c371e15141b007a6"),
  user4: privateKeyToAccount("0x47e179ec197488593b187f80a00eb0da91f1b9d0b13f8733639f19c30a34926a"),
} as const;

const WINNING_NUMBER = 42;
const BET_AMOUNTS = {
  user1: parseEther("10"),   // winner
  user2: parseEther("5"),    // loser
  user3: parseEther("8"),    // loser
  user4: parseEther("3"),    // loser
} as const;

function createClient(account: ReturnType<typeof privateKeyToAccount>): WalletClient {
  return createWalletClient({
    account,
    chain: ARTEMIS_CHAIN,
    transport: http(RPC_URL),
  });
}

function getPrecompile(client: WalletClient) {
  return getContract({
    address: BETTING_PRECOMPILE_ADDRESS,
    abi: IBettingPrecompileAbi,
    client,
  });
}

const publicClient: PublicClient = createPublicClient({
  chain: ARTEMIS_CHAIN,
  transport: http(RPC_URL),
});

async function waitForBlock(txHash: `0x${string}`): Promise<void> {
  await publicClient.waitForTransactionReceipt({ hash: txHash });
}

async function graphqlQuery(query: string): Promise<unknown> {
  const response = await fetch(PONDER_GRAPHQL_URL, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ query }),
  });
  const json = await response.json() as { data: unknown };
  return json.data;
}

async function waitForIndexer(maxRetries = 30, delayMs = 2000): Promise<void> {
  for (let i = 0; i < maxRetries; i++) {
    try {
      const data = await graphqlQuery(`{ protocolStatss { items { totalBets } } }`) as {
        protocolStatss: { items: Array<{ totalBets: number }> };
      };
      if (data?.protocolStatss?.items?.[0]?.totalBets === 4) {
        return;
      }
    } catch {
      // Ponder not ready yet
    }
    await new Promise((resolve) => setTimeout(resolve, delayMs));
  }
  throw new Error("Indexer did not sync in time");
}

async function waitForIndexerSettled(maxRetries = 30, delayMs = 2000): Promise<void> {
  for (let i = 0; i < maxRetries; i++) {
    try {
      const data = await graphqlQuery(`{ rounds { items { status } } }`) as {
        rounds: { items: Array<{ status: number }> };
      };
      if (data?.rounds?.items?.[0]?.status === 3) {
        return;
      }
    } catch {
      // Ponder not ready yet
    }
    await new Promise((resolve) => setTimeout(resolve, delayMs));
  }
  throw new Error("Indexer did not reach Settled state in time");
}

describe("Indexer E2E: Full Betting Lifecycle", () => {
  let roundId: bigint;

  beforeAll(async () => {
    // Get current round ID from the chain
    const adminClient = createClient(ACCOUNTS.admin);
    const precompile = getPrecompile(adminClient);
    const [currentRoundId] = await publicClient.readContract({
      address: BETTING_PRECOMPILE_ADDRESS,
      abi: IBettingPrecompileAbi,
      functionName: "getCurrentRound",
    });
    roundId = currentRoundId;
  }, 30_000);

  it("should place 4 bets", async () => {
    const bets: Array<{ account: ReturnType<typeof privateKeyToAccount>; number: number; amount: bigint }> = [
      { account: ACCOUNTS.user1, number: WINNING_NUMBER, amount: BET_AMOUNTS.user1 },
      { account: ACCOUNTS.user2, number: 10, amount: BET_AMOUNTS.user2 },
      { account: ACCOUNTS.user3, number: 77, amount: BET_AMOUNTS.user3 },
      { account: ACCOUNTS.user4, number: 3, amount: BET_AMOUNTS.user4 },
    ];

    for (const { account, number, amount } of bets) {
      const client = createClient(account);
      const hash = await client.writeContract({
        address: BETTING_PRECOMPILE_ADDRESS,
        abi: IBettingPrecompileAbi,
        functionName: "placeBet",
        args: [number, NATIVE_TOKEN_ADDRESS, amount],
      });
      await waitForBlock(hash);
    }
  }, 60_000);

  it("should force close the round", async () => {
    const adminClient = createClient(ACCOUNTS.admin);
    const hash = await adminClient.writeContract({
      address: BETTING_PRECOMPILE_ADDRESS,
      abi: IBettingPrecompileAbi,
      functionName: "forceCloseRound",
      args: [roundId],
    });
    await waitForBlock(hash);
  }, 30_000);

  it("should submit result", async () => {
    const adminClient = createClient(ACCOUNTS.admin);
    const hash = await adminClient.writeContract({
      address: BETTING_PRECOMPILE_ADDRESS,
      abi: IBettingPrecompileAbi,
      functionName: "submitResult",
      args: [roundId, WINNING_NUMBER],
    });
    await waitForBlock(hash);
  }, 30_000);

  it("should claim winnings for winner", async () => {
    const winnerClient = createClient(ACCOUNTS.user1);
    const hash = await winnerClient.writeContract({
      address: BETTING_PRECOMPILE_ADDRESS,
      abi: IBettingPrecompileAbi,
      functionName: "claimWinnings",
      args: [roundId],
    });
    await waitForBlock(hash);
  }, 30_000);

  it("should admin claim pool", async () => {
    const adminClient = createClient(ACCOUNTS.admin);
    const hash = await adminClient.writeContract({
      address: BETTING_PRECOMPILE_ADDRESS,
      abi: IBettingPrecompileAbi,
      functionName: "adminClaimPool",
      args: [roundId],
    });
    await waitForBlock(hash);
  }, 30_000);

  it("should have correct round data in indexer", async () => {
    await waitForIndexerSettled();

    const totalVolume = BET_AMOUNTS.user1 + BET_AMOUNTS.user2 + BET_AMOUNTS.user3 + BET_AMOUNTS.user4;

    const data = await graphqlQuery(`{
      round(id: "${roundId}") {
        id
        status
        winningNumber
        totalBets
        totalVolume
        totalPayout
        poolClaimed
      }
    }`) as {
      round: {
        id: string;
        status: number;
        winningNumber: number;
        totalBets: number;
        totalVolume: string;
        totalPayout: string;
        poolClaimed: string;
      };
    };

    expect(data.round).toBeTruthy();
    expect(data.round.status).toBe(3); // Settled
    expect(data.round.winningNumber).toBe(WINNING_NUMBER);
    expect(data.round.totalBets).toBe(4);
    expect(BigInt(data.round.totalVolume)).toBe(totalVolume);
    expect(BigInt(data.round.totalPayout)).toBeGreaterThan(0n);
    expect(BigInt(data.round.poolClaimed)).toBeGreaterThan(0n);
  }, 120_000);

  it("should have correct bet data in indexer", async () => {
    // Winner bet
    const winnerData = await graphqlQuery(`{
      bet(id: "${roundId}-${ACCOUNTS.user1.address}") {
        number
        amount
        claimed
        payout
      }
    }`) as {
      bet: { number: number; amount: string; claimed: boolean; payout: string };
    };

    expect(winnerData.bet.number).toBe(WINNING_NUMBER);
    expect(BigInt(winnerData.bet.amount)).toBe(BET_AMOUNTS.user1);
    expect(winnerData.bet.claimed).toBe(true);
    expect(BigInt(winnerData.bet.payout)).toBeGreaterThan(0n);

    // Loser bet
    const loserData = await graphqlQuery(`{
      bet(id: "${roundId}-${ACCOUNTS.user2.address}") {
        number
        claimed
        payout
      }
    }`) as {
      bet: { number: number; claimed: boolean; payout: string };
    };

    expect(loserData.bet.number).toBe(10);
    expect(loserData.bet.claimed).toBe(false);
    expect(BigInt(loserData.bet.payout)).toBe(0n);
  }, 30_000);

  it("should have correct userStats in indexer", async () => {
    // Winner stats
    const winnerStats = await graphqlQuery(`{
      userStats(id: "${ACCOUNTS.user1.address}") {
        totalBets
        totalWagered
        totalWon
        winCount
      }
    }`) as {
      userStats: { totalBets: number; totalWagered: string; totalWon: string; winCount: number };
    };

    expect(winnerStats.userStats.totalBets).toBe(1);
    expect(BigInt(winnerStats.userStats.totalWagered)).toBe(BET_AMOUNTS.user1);
    expect(winnerStats.userStats.winCount).toBe(1);
    expect(BigInt(winnerStats.userStats.totalWon)).toBeGreaterThan(0n);

    // Loser stats
    const loserStats = await graphqlQuery(`{
      userStats(id: "${ACCOUNTS.user3.address}") {
        totalBets
        winCount
      }
    }`) as {
      userStats: { totalBets: number; winCount: number };
    };

    expect(loserStats.userStats.totalBets).toBe(1);
    expect(loserStats.userStats.winCount).toBe(0);
  }, 30_000);

  it("should have correct dailyStats in indexer", async () => {
    const data = await graphqlQuery(`{
      dailyStats(id: "${roundId}") {
        totalBets
        uniqueUsers
        totalVolume
      }
    }`) as {
      dailyStats: { totalBets: number; uniqueUsers: number; totalVolume: string };
    };

    const totalVolume = BET_AMOUNTS.user1 + BET_AMOUNTS.user2 + BET_AMOUNTS.user3 + BET_AMOUNTS.user4;
    expect(data.dailyStats.totalBets).toBe(4);
    expect(data.dailyStats.uniqueUsers).toBe(4);
    expect(BigInt(data.dailyStats.totalVolume)).toBe(totalVolume);
  }, 30_000);

  it("should have correct numberStats in indexer", async () => {
    const data = await graphqlQuery(`{
      numberStats(id: "${WINNING_NUMBER}") {
        timesPicked
        timesWon
      }
    }`) as {
      numberStats: { timesPicked: number; timesWon: number };
    };

    expect(data.numberStats.timesPicked).toBe(1);
    expect(data.numberStats.timesWon).toBe(1);
  }, 30_000);

  it("should have correct protocolStats in indexer", async () => {
    const data = await graphqlQuery(`{
      protocolStats(id: "global") {
        totalRounds
        totalBets
        totalVolume
        totalPayout
        totalPoolClaimed
      }
    }`) as {
      protocolStats: {
        totalRounds: number;
        totalBets: number;
        totalVolume: string;
        totalPayout: string;
        totalPoolClaimed: string;
      };
    };

    const totalVolume = BET_AMOUNTS.user1 + BET_AMOUNTS.user2 + BET_AMOUNTS.user3 + BET_AMOUNTS.user4;
    expect(data.protocolStats.totalRounds).toBe(1);
    expect(data.protocolStats.totalBets).toBe(4);
    expect(BigInt(data.protocolStats.totalVolume)).toBe(totalVolume);
    expect(BigInt(data.protocolStats.totalPayout)).toBeGreaterThan(0n);
    expect(BigInt(data.protocolStats.totalPoolClaimed)).toBeGreaterThan(0n);
  }, 30_000);
});
```

Note on GraphQL query names: Ponder auto-generates plural names for list queries (e.g., `rounds`, `bets`) and singular for by-ID queries (e.g., `round(id: ...)`, `bet(id: ...)`). The plurals use `{ items { ... } }` nesting. Singular queries return the object directly. Verify against Ponder's actual GraphQL schema by visiting `http://localhost:42069/graphql` in a browser when Ponder dev is running.

The `id` field in singular queries uses the PK value. For `userStats`, the PK is `address` (hex), so the GraphQL ID is the lowercased address. Ponder may lowercase hex values — verify and adjust if needed.

- [ ] **Step 2: Run E2E test**

Ensure the following are running:
1. Substrate dev node (`./target/release/frontier-template-node --dev`)
2. Ponder dev server (`cd packages/indexer && pnpm dev`)
3. Postgres with `betting_indexer` database

Then run:
```bash
cd packages/indexer && npx vitest run test/indexer.e2e.test.ts --timeout 300000
```

Expected: all 8 tests pass. If tests fail:
- Check Ponder logs for indexing errors
- Check GraphQL field names match (visit `http://localhost:42069/graphql`)
- Adjust query syntax for the installed Ponder version
- Address ID may need to be lowercased

- [ ] **Step 3: Commit**

```bash
git add packages/indexer/test/indexer.e2e.test.ts
git commit -m "test(indexer): add full lifecycle e2e test

Simulates 4 bets, result submission, winner claim, admin pool claim.
Verifies all 6 indexed tables against expected analytics data."
```

---

### Task 8: Final Verification and Cleanup

- [ ] **Step 1: Run full monorepo build**

```bash
cd /Users/huyduan/projects/blockchain && pnpm turbo build
```

Expected: all packages build successfully, including `@betting/shared` and `@betting/indexer`.

- [ ] **Step 2: Verify .gitignore covers sensitive files**

Check that `.env.local` is not tracked:
```bash
git status packages/indexer/.env.local
```

Expected: file not shown (gitignored).

- [ ] **Step 3: Final commit if any cleanup needed**

```bash
git add -A
git commit -m "chore(indexer): cleanup and final verification"
```
