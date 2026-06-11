# Artemis Explorer — Design

EVM block explorer for the Artemis chain, served from a new `packages/explorer` workspace. Reads Subscan's Postgres directly via Prisma, exposes data through tRPC, renders with Next.js 16 server-first, and reuses coss UI primitives shared with `packages/ui`.

- Chain: Artemis | Chain ID: 322 | Token: ART (18 decimals)
- Status: design
- Author: Huy Duan Tran
- Date: 2026-05-07
- Related: `infra/subscan/ui/FUNCTIONAL_SPEC.md` (full Subscan spec), `infra/subscan/README.md` (data source)

---

## 1. Goals and non-goals

### Goals

- A self-contained Next.js 16 explorer for Artemis that reads from the Subscan Postgres already running in `infra/subscan/`.
- EVM-only feature parity with Screens 11-18 of the Subscan functional spec: block, transaction, address, contract — list and detail.
- Type-safe data layer end-to-end via Prisma → tRPC → Server Components.
- Shared coss UI primitives (Base-UI wrappers) extracted into a new `@artemis/coss-ui` package, consumed by both `packages/ui` and `packages/explorer`.
- Local-first development. No remote deploy until the local stack works end-to-end.

### Non-goals (v1)

- Substrate-side screens (Screens 1-10). Out of scope; can be added in v2.
- ERC-20 / ERC-721 token list, holders, transfers (Screens 20-22).
- Contract verification form (Screen 19) — read verified data only, no write path.
- Replacing the existing `packages/ui/app/blockexplorer/` route. The dApp keeps its lightweight in-app explorer; the new package is the full-feature explorer.
- Dark mode. Light mode only in v1; structure must not preclude adding dark.
- Multi-chain. Single chain (Artemis chain id 322).

---

## 2. Architecture

```
                ┌──────────────────────────────┐
                │  Artemis archive node :9946  │
                └──────────────┬───────────────┘
                               │  WS / HTTP RPC
                               ▼
        ┌──────────────────────────────────────────┐
        │  Subscan stack (infra/subscan/)          │
        │  observer + worker + api → Postgres      │
        └──────────────┬───────────────────────────┘
                       │ Postgres (read-only role)
                       ▼
        ┌──────────────────────────────────────────┐
        │  packages/explorer (Next.js 16)          │
        │  ┌────────────────────────────────────┐  │
        │  │ Server Components + tRPC caller    │  │
        │  │  └─ Prisma Client (introspected)   │  │
        │  └────────────────────────────────────┘  │
        │  ┌────────────────────────────────────┐  │
        │  │ Client islands (latest cards,      │  │
        │  │ search bar, copy hash, polling)    │  │
        │  │  └─ @trpc/react-query              │  │
        │  └────────────────────────────────────┘  │
        │  ┌────────────────────────────────────┐  │
        │  │ @artemis/coss-ui (Base-UI primitives)│ │
        │  └────────────────────────────────────┘  │
        └──────────────────────────────────────────┘
```

### 2.1 Boundaries

- **Subscan owns DB schema.** Explorer reads only. No `prisma migrate`, no triggers, no Postgres functions added by us. Subscan is pinned by docker image tag in `infra/subscan/`; CI guards against schema drift (Section 6).
- **tRPC is the only data transport.** UI never calls Prisma directly. Server Components use `appRouter.createCaller(ctx)`; Client islands use `@trpc/react-query` over an HTTP route handler.
- **`@artemis/coss-ui` holds primitives only.** Button, table, tabs, dialog, dropdown, tooltip, input, label, separator, scroll-area, skeleton, toast. Domain-aware components (BlockTable, AddressLink) live in `packages/explorer/src/components/`.

### 2.2 Risks accepted

- Subscan schema is an internal contract with no stability guarantee. Mitigation: pin Subscan version, run `prisma db pull` diff in CI when the pin changes.
- Postgres without indexes for some access patterns (e.g. all transactions for an address with > 10k tx) may be slow. Mitigation: accept slow queries in v1; if real users hit this, propose an upstream Subscan PR to add indexes. Do not add migrations from explorer.

---

## 3. Package layout

```
packages/
  coss-ui/                            (NEW — extracted from packages/ui/components/clay/)
    src/
      button.tsx
      table.tsx
      tabs.tsx
      dialog.tsx
      tooltip.tsx
      dropdown.tsx
      input.tsx
      label.tsx
      separator.tsx
      scroll-area.tsx
      skeleton.tsx
      toast.tsx
      index.ts
    package.json                      (name: @artemis/coss-ui, type: module)
    tsconfig.json

  explorer/                           (NEW)
    prisma/
      schema.prisma                   (introspected from Subscan DB, committed)
    src/
      app/
        layout.tsx                    (Server)
        page.tsx                      (home)
        loading.tsx
        error.tsx
        not-found.tsx
        block/
          page.tsx                    (Screen 11)
          [num]/page.tsx              (Screen 12)
          loading.tsx
        tx/
          page.tsx                    (Screen 13)
          [hash]/page.tsx             (Screen 14)
        address/
          page.tsx                    (Screen 15)
          [addr]/page.tsx             (Screen 16)
        contract/
          page.tsx                    (Screen 17)
          [addr]/page.tsx             (Screen 18)
        api/
          trpc/[trpc]/route.ts        (HTTP handler for client islands)
          health/route.ts             (Dokploy health check)
        search/
          actions.ts                  (server action)
      server/
        trpc.ts                       (init t, context, middleware)
        prisma.ts                     (PrismaClient singleton)
        caller.ts                     (createCaller for Server Components)
        routers/
          _app.ts
          metadata.ts
          block.ts
          tx.ts
          address.ts
          contract.ts
          search.ts
        lib/
          encode.ts                   (Prisma → wire boundary helpers)
          cursor.ts                   (base64 encode/decode opaque cursor)
      lib/
        regex.ts                      (search input classifier)
      components/
        layout/
          header.tsx
          footer.tsx
          nav.tsx
        search/
          search-bar.tsx              ('use client')
        home/
          summary-cards.tsx
          latest-blocks.tsx           ('use client', polling)
          latest-txs.tsx              ('use client', polling)
        block/
          block-table.tsx
          block-info-panel.tsx
          block-tx-tab.tsx
        tx/
          tx-table.tsx
          tx-info-panel.tsx
          tx-input-data.tsx           ('use client')
        address/
          address-table.tsx
          address-info-panel.tsx
          address-tx-tab.tsx
        contract/
          contract-table.tsx
          contract-info-panel.tsx
          contract-source-tab.tsx
          contract-tx-tab.tsx
        shared/
          pagination.tsx
          copy-button.tsx              ('use client')
          hash-link.tsx
          address-link.tsx
          timestamp.tsx                ('use client')
          json-viewer.tsx              ('use client')
      trpc/
        client.tsx                     (provider for client islands)
      env.ts
    next.config.ts
    package.json                      (name: @artemis/explorer, dev port 3002)
    tsconfig.json
    .env.example

  ui/                                 (EXISTING — modified)
    components/
      clay/                           (REMOVED — moved to @artemis/coss-ui)
      ...
    package.json                      (add "@artemis/coss-ui": "workspace:*")

  shared/                             (EXISTING — modified)
    src/
      format/                         (NEW module)
        balance.ts
        hex.ts
        time.ts
        number.ts
        index.ts
      abis/                           (unchanged)
      constants/                      (unchanged)
      index.ts                        (re-export format/*)
    package.json                      (add "date-fns": "^4")

  shared/, indexer/, contracts/, node/  (otherwise unchanged)

infra/
  explorer/                           (NEW — created only after local works)
    Dockerfile
    docker-compose.yml
  subscan/                            (unchanged)
```

### 3.1 Migration: clay → coss-ui

In a single commit:

1. Create `packages/coss-ui/` with new `package.json`, `tsconfig.json`.
2. Move `packages/ui/components/clay/*` to `packages/coss-ui/src/`.
3. Update imports inside `packages/ui/` from `@/components/clay/...` to `@artemis/coss-ui`.
4. Add `"@artemis/coss-ui": "workspace:*"` to `packages/ui/package.json`.
5. Run `pnpm install`, `pnpm turbo build`, verify `packages/ui` still builds.
6. No build step for `coss-ui` itself: ship raw `.tsx` source; consumer Next.js compiles. Set `"main": "./src/index.ts"`, `"types": "./src/index.ts"` in `package.json`. Both `packages/explorer/next.config.ts` and `packages/ui/next.config.ts` must include `transpilePackages: ['@artemis/coss-ui']` so Next.js compiles the raw source.

---

## 4. Data layer

### 4.1 Schema acquisition

1. Run Subscan locally: `cd infra/subscan && docker compose up -d`.
2. Wait for observer to index genesis block (~30 seconds).
3. From `packages/explorer/`: `pnpm prisma db pull` to introspect.
4. Commit the resulting `schema.prisma`.
5. Pin the Subscan image tag in `infra/subscan/docker-compose.yml`. CI re-runs `db pull --print` and diffs against the committed schema when the pin changes.

`schema.prisma` config:
- `output = "../node_modules/.prisma/client"`
- No `migrations` directive — explorer never owns migrations.
- `previewFeatures = []`.

### 4.2 Prisma Client singleton

```ts
// server/prisma.ts
import { PrismaClient } from '@prisma/client';
const globalForPrisma = globalThis as unknown as { prisma?: PrismaClient };
export const prisma = globalForPrisma.prisma ?? new PrismaClient({
  log: process.env.NODE_ENV === 'development' ? ['query', 'warn', 'error'] : ['warn', 'error'],
});
if (process.env.NODE_ENV !== 'production') globalForPrisma.prisma = prisma;
```

### 4.3 Type boundary (Prisma → wire)

tRPC procedures convert all DB types to JSON-friendly primitives before returning. No superjson, no BigInt over the wire, no Buffer, no Decimal, no Date.

| Postgres column         | Prisma type | tRPC output       | UI consume                           |
|-------------------------|-------------|-------------------|--------------------------------------|
| `bigint block_num`      | `BigInt`    | `number`          | `number` (safe < 2^53 for block num) |
| `bigint nonce`          | `BigInt`    | `number`          | `number`                             |
| `numeric value`         | `Decimal`   | `string` (wei)    | `formatBalance(str, 18)`             |
| `numeric balance`       | `Decimal`   | `string`          | `formatBalance(str, 18)`             |
| `bytea hash` (32 bytes) | `Buffer`    | `0x{64}` string   | `formatHex(s)` for display           |
| `bytea address` (20 b)  | `Buffer`    | `0x{40}` checksum | `formatHex(s)`                       |
| `timestamp ts`          | `Date`      | `number` (unix s) | `formatRelativeTime` / `formatTimestamp` |
| `text status`           | `string`    | `string`          | `string`                             |

Helpers in `server/lib/encode.ts` (server-only, depends on Buffer/Decimal/Date):

```ts
import { getAddress } from 'viem';
import type { Decimal } from '@prisma/client/runtime/library';

export const hex = (b: Buffer): `0x${string}` => `0x${b.toString('hex')}` as const;
export const addr = (b: Buffer): `0x${string}` => getAddress(hex(b));
export const unix = (d: Date): number => Math.floor(d.getTime() / 1000);

export const safeNum = (n: bigint): number => {
  if (n > BigInt(Number.MAX_SAFE_INTEGER)) {
    throw new Error(`safeNum overflow: ${n}`);
  }
  return Number(n);
};

export const bigStr = (n: bigint | Decimal): string =>
  typeof n === 'bigint' ? n.toString() : n.toFixed(0);

export const nullable = <T, U>(x: T | null, fn: (t: T) => U): U | null =>
  x === null ? null : fn(x);
```

### 4.4 UI display helpers

Live in `packages/shared/src/format/` so both `packages/ui` and `packages/explorer` consume them. Relies on viem and `date-fns` for primitives we do not reinvent.

```ts
// packages/shared/src/format/balance.ts
import { formatUnits } from 'viem';

export function formatBalance(
  wei: bigint | string,
  decimals: number,
  displayDecimals = 4,
): string {
  const raw = formatUnits(typeof wei === 'string' ? BigInt(wei) : wei, decimals);
  const [int, frac = ''] = raw.split('.');
  const intGrouped = new Intl.NumberFormat('en-US').format(BigInt(int));
  const fracTrimmed = frac.slice(0, displayDecimals).replace(/0+$/, '');
  return fracTrimmed ? `${intGrouped}.${fracTrimmed}` : intGrouped;
}

// packages/shared/src/format/hex.ts
export function formatHex(hex: `0x${string}`, head = 6, tail = 4): string {
  return `${hex.slice(0, 2 + head)}…${hex.slice(-tail)}`;
}

// packages/shared/src/format/time.ts
import { formatDistanceToNow } from 'date-fns';

export function formatRelativeTime(unixSec: number): string {
  return formatDistanceToNow(new Date(unixSec * 1000), { addSuffix: true });
}

export function formatTimestamp(unixSec: number): string {
  const fmt = new Intl.DateTimeFormat('en-US', {
    dateStyle: 'medium',
    timeStyle: 'medium',
    timeZone: 'UTC',
  });
  return `${fmt.format(new Date(unixSec * 1000))} UTC`;
}

// packages/shared/src/format/number.ts
export function formatNumber(n: number | bigint | string): string {
  const v = typeof n === 'string' ? BigInt(n) : n;
  return new Intl.NumberFormat('en-US').format(v);
}
```

Viem helpers used directly (no wrapper): `formatUnits`, `formatEther`, `parseUnits`, `parseEther`, `getAddress`, `isAddress`, `isHex`, `slice`, `size`.

### 4.5 Connection & access control

- Dev: `DATABASE_URL=postgresql://postgres:postgres@localhost:5432/subscan`. Uses superuser for convenience.
- Prod: dedicated read-only role.
  ```sql
  CREATE ROLE explorer_readonly LOGIN PASSWORD '...';
  GRANT CONNECT ON DATABASE subscan TO explorer_readonly;
  GRANT USAGE ON SCHEMA public TO explorer_readonly;
  GRANT SELECT ON ALL TABLES IN SCHEMA public TO explorer_readonly;
  ALTER DEFAULT PRIVILEGES IN SCHEMA public
    GRANT SELECT ON TABLES TO explorer_readonly;
  ```
- Pool: `?connection_limit=10&pool_timeout=20`. Subscan API also pools against the same Postgres; total must stay below `max_connections` (default 100).

### 4.6 Schema unknowns (resolved at first introspect)

- Whether Subscan prefixes tables with the network name (e.g. `artemis_evm_block` vs `evm_block`). Verify after `db pull`; adjust router code accordingly.
- Index coverage on `evm_transaction.from`, `evm_transaction.to`, `evm_transaction.block_num`. If missing, accept slow queries in v1.
- Exact column names for tx fee components (gas_used, effective_gas_price, gas_price). Compute fee at boundary using whichever columns Subscan exposes.

---

## 5. tRPC routers

### 5.1 Router tree

```ts
appRouter = router({
  metadata: metadataRouter,
  block:    blockRouter,
  tx:       txRouter,
  address:  addressRouter,
  contract: contractRouter,
  search:   searchRouter,
});
```

### 5.2 Procedure shape

All list procedures share one shape:

```ts
list: publicProcedure
  .input(z.object({
    cursor: z.string().optional(),
    limit:  z.number().int().min(1).max(50).default(10),
  }))
  .query(async ({ ctx, input }) => ({
    items:      Array<TItem>,
    nextCursor: string | null,
    prevCursor: string | null,
  }));
```

Detail procedures return `T | null` (never throw NOT_FOUND). Server Components call `notFound()` from `next/navigation` when null.

### 5.3 Per-router procedures

| Router    | Procedure        | Input                               | Output                                                                 |
|-----------|------------------|-------------------------------------|------------------------------------------------------------------------|
| metadata  | `get`            | none                                | `{ chainId, chainName, tokenSymbol, tokenDecimals, blockHeight, txCount, accountCount, contractCount, finalizedHeight }` |
| block     | `latest`         | `{ limit }`                         | `EvmBlockSummary[]` (for home polling)                                 |
| block     | `list`           | `{ cursor?, limit }`                | `{ items: EvmBlockSummary[], nextCursor, prevCursor }`                 |
| block     | `byNumber`       | `{ num: number }`                   | `EvmBlockDetail \| null`                                               |
| block     | `txList`         | `{ blockNum, cursor?, limit }`      | `{ items: EvmTxSummary[], nextCursor, prevCursor }`                    |
| tx        | `latest`         | `{ limit }`                         | `EvmTxSummary[]`                                                       |
| tx        | `list`           | `{ cursor?, limit }`                | `{ items: EvmTxSummary[], nextCursor, prevCursor }`                    |
| tx        | `byHash`         | `{ hash: 0x{64} }`                  | `EvmTxDetail \| null`                                                  |
| address   | `list`           | `{ cursor?, limit }`                | `{ items: EvmAccountSummary[], nextCursor, prevCursor }`               |
| address   | `byAddress`      | `{ addr: 0x{40} }`                  | `EvmAccountDetail \| null`                                             |
| address   | `txList`         | `{ addr, cursor?, limit }`          | `{ items: EvmTxSummary[], nextCursor, prevCursor }`                    |
| contract  | `list`           | `{ cursor?, limit }`                | `{ items: EvmContractSummary[], nextCursor, prevCursor }`              |
| contract  | `byAddress`      | `{ addr: 0x{40} }`                  | `EvmContractDetail \| null` (includes verified flag, source, abi, bytecode) |
| contract  | `txList`         | `{ addr, cursor?, limit }`          | `{ items: EvmTxSummary[], nextCursor, prevCursor }`                    |
| search    | `resolve`        | `{ query: string }`                 | discriminated union (Section 5.6)                                      |

### 5.4 Server-side caller

```ts
// server/caller.ts
import 'server-only';
import { appRouter } from './routers/_app';
import { createContext } from './trpc';

export async function getServerCaller() {
  return appRouter.createCaller(await createContext());
}

// usage in app/block/page.tsx
const trpc = await getServerCaller();
const { items, nextCursor } = await trpc.block.list({
  cursor: searchParams.cursor,
  limit: 25,
});
```

### 5.5 Client provider

```ts
// trpc/client.tsx
'use client';
import { createTRPCReact } from '@trpc/react-query';
import type { AppRouter } from '@/server/routers/_app';

export const trpc = createTRPCReact<AppRouter>();

// HTTP handler at app/api/trpc/[trpc]/route.ts uses fetchRequestHandler
```

### 5.6 Search resolution

```ts
type SearchResult =
  | { kind: 'block';    num: number }
  | { kind: 'tx';       hash: `0x${string}` }
  | { kind: 'address';  addr: `0x${string}` }
  | { kind: 'contract'; addr: `0x${string}` }
  | { kind: 'not_found' };

// classifier in lib/regex.ts
function classify(q: string): 'block_num' | 'tx_hash' | 'address' | 'unknown' {
  const trimmed = q.trim();
  if (/^\d+$/.test(trimmed)) return 'block_num';
  if (/^0x[0-9a-fA-F]{64}$/.test(trimmed)) return 'tx_hash';
  if (/^0x[0-9a-fA-F]{40}$/.test(trimmed)) return 'address';
  return 'unknown';
}
```

For `address`, run one Prisma query: `evm_account.findUnique({ where: { address }, select: { code } })`. If `code` is non-empty (`!= '\\x'`), result is `{ kind: 'contract', addr }`; otherwise `{ kind: 'address', addr }`.

### 5.7 Cursor encoding

Opaque base64 of JSON. Implementation in `server/lib/cursor.ts`:

```ts
export function encodeCursor(payload: object): string {
  return Buffer.from(JSON.stringify(payload)).toString('base64url');
}
export function decodeCursor<T>(s: string): T {
  return JSON.parse(Buffer.from(s, 'base64url').toString('utf8')) as T;
}
```

Block list cursor: `{ block_num: number }`. Tx list cursor: `{ block_num: number, transaction_index: number }`.

### 5.8 Caching strategy

Default: no caching. Server Components call the tRPC server caller directly; each request hits Postgres. Indexed queries on Subscan tables return < 5 ms; explorer is internal/dev tooling, not high-traffic.

One exception: `metadata.get` is always cached with `'use cache'` + `cacheLife({ revalidate: 30 })` because count queries (total blocks, total tx, total accounts) can be slow full-table scans and the values change slowly.

If we later measure Postgres load too high on other procedures, add `'use cache'` per-procedure. Cost is two lines, no architectural change:

```ts
import { cacheLife } from 'next/cache';
async function getCachedBlockList(input: ListInput) {
  'use cache';
  cacheLife({ revalidate: 6, stale: 0 });
  const trpc = await getServerCaller();
  return trpc.block.list(input);
}
```

Client islands (latest cards) use react-query `staleTime: 6000, refetchInterval: 6000`.

### 5.9 Error model

- Procedures throw `TRPCError({ code: 'BAD_REQUEST' })` for malformed input (rejected by zod first).
- Detail procedures return `null` instead of `NOT_FOUND` so Server Components can call `notFound()` cleanly.
- Prisma connection failures surface as `INTERNAL_SERVER_ERROR`. Caught by `error.tsx` boundaries.
- All errors logged with structured JSON (request id, route, error code).

---

## 6. Routes and components

### 6.1 Route map

| Route              | Screen | Render | Notes                                                  |
|--------------------|--------|--------|--------------------------------------------------------|
| `/`                | Home   | SSR + client islands | Summary cards (server) + Latest Blocks/Tx (polling) |
| `/block`           | 11     | SSR    | Block list, cursor pagination                          |
| `/block/[num]`     | 12     | SSR    | Block info + Transactions tab                          |
| `/tx`              | 13     | SSR    | Tx list, cursor pagination                             |
| `/tx/[hash]`       | 14     | SSR    | Tx detail (info, status, value, fee, raw input)        |
| `/address`         | 15     | SSR    | Address list                                           |
| `/address/[addr]`  | 16     | SSR    | Account info + Transactions tab                        |
| `/contract`        | 17     | SSR    | Contract list                                          |
| `/contract/[addr]` | 18     | SSR    | Contract info + Contract tab + Transactions tab; no verify form |
| `/api/trpc/[trpc]` | —      | Route  | tRPC fetch handler for client islands                  |
| `/api/health`      | —      | Route  | `{ ok, db_ok, latest_block }` for Dokploy health check |

### 6.2 Server vs Client split rule

Default: Server Component. `'use client'` only when:
- Clipboard API (copy hash button)
- Polling (react-query refetch interval)
- Local UI state (expand/collapse JSON viewer, tab switch with hash routing)
- Time tickers (`Timestamp` re-renders relative time every 60s)

### 6.3 Pagination

URL-driven, no JS required:

```
/block                      first page
/block?cursor=eyJpZCI6MTIzfQ next page
```

`Pagination` component renders `<Link>` with `?cursor=...`; Server Component re-renders. Cursor is opaque base64.

### 6.4 Search flow

1. User types into `SearchBar` Client Component, presses Enter.
2. Form action posts to server action `searchAction(formData)`.
3. Server action calls `trpc.search.resolve({ query })`.
4. On match, `redirect()` to the appropriate route. On `not_found`, redirect to `/?error=not-found` with toast.
5. No client-side JS required to navigate.

### 6.5 Loading and error states

- `loading.tsx` co-located with each segment renders `Skeleton` (10 rows for tables, info-panel placeholder for detail).
- `error.tsx` at root + segment level. Shows friendly message + Retry button (calls `reset()`).
- `not-found.tsx` at root. Detail pages call `notFound()` when tRPC returns `null`.

### 6.6 Themes

Light only in v1. Tailwind v4 `@variant dark` ready for v2 — no refactor needed.

### 6.7 Coexistence with `packages/ui/app/blockexplorer/`

The dApp keeps its lightweight in-app block explorer (transfer history, recent tx for current wallet). The full-feature explorer at `packages/explorer` is a separate Next.js app on port 3002 with no auth. Users navigate between them via cross-app links (configurable via `NEXT_PUBLIC_EXPLORER_URL`).

---

## 7. Deployment and ops

### 7.1 Local dev

Hard prerequisite: local stack must work end-to-end before any remote deploy.

```bash
# 1. Start Subscan locally (indexes against archive node remote)
cd infra/subscan
cp .env.example .env
docker compose up -d
# wait ~30s for observer to index genesis

# 2. Verify Subscan tables exist
psql -h localhost -U postgres -d subscan -c "\dt"

# 3. Start explorer
cd packages/explorer
cp .env.example .env
pnpm prisma db pull
pnpm prisma generate
pnpm dev   # http://localhost:3002
```

### 7.2 Environment variables

`packages/explorer/src/env.ts` (zod-validated at boot):

```ts
export const env = z.object({
  DATABASE_URL: z.string().url(),
  NEXT_PUBLIC_CHAIN_ID: z.coerce.number().default(322),
  NEXT_PUBLIC_CHAIN_NAME: z.string().default('Artemis'),
  NEXT_PUBLIC_TOKEN_SYMBOL: z.string().default('ART'),
  NEXT_PUBLIC_TOKEN_DECIMALS: z.coerce.number().default(18),
  NEXT_PUBLIC_EXPLORER_URL: z.string().url().optional(),
}).parse(process.env);
```

`.env.example` checked in. `.env` gitignored.

### 7.3 Production (created only after local works)

`infra/explorer/Dockerfile` — multi-stage:
1. `deps` — pnpm fetch + install
2. `builder` — `pnpm turbo build --filter=@artemis/explorer` with `output: 'standalone'` in `next.config.ts`
3. `runner` — Node 22 alpine, copy `.next/standalone`, `.next/static`, `public`. Run as non-root user.

`infra/explorer/docker-compose.yml`:

```yaml
services:
  explorer:
    build: ./
    environment:
      DATABASE_URL: ${DATABASE_URL}
      NEXT_PUBLIC_CHAIN_ID: 322
      NEXT_PUBLIC_CHAIN_NAME: Artemis
      NEXT_PUBLIC_TOKEN_SYMBOL: ART
      NEXT_PUBLIC_TOKEN_DECIMALS: 18
      NEXT_PUBLIC_EXPLORER_URL: ${EXPLORER_URL}
    ports:
      - "${EXPLORER_PORT:-3002}:3000"
    restart: unless-stopped
    healthcheck:
      test: ["CMD", "wget", "-qO-", "http://localhost:3000/api/health"]
      interval: 30s
      timeout: 5s
      retries: 3
```

Dokploy on sg-storage. Traefik label routes `explorer.<domain>` with Let's Encrypt + 100 req/min/IP rate-limit middleware.

### 7.4 Logging

- Next.js: `logging.fetches.fullUrl: true` in dev; off in prod.
- Prisma: query log in dev only (set via `log` array in client constructor based on `NODE_ENV`).
- Application: structured JSON with request id, route, latency, error code. Stdout only — Dokploy aggregates.

### 7.5 Health check

`GET /api/health`:
```ts
const latest = await prisma.evm_block.findFirst({
  orderBy: { block_num: 'desc' }, select: { block_num: true },
});
return Response.json({
  ok: true,
  db_ok: latest !== null,
  latest_block: latest ? Number(latest.block_num) : null,
});
```

### 7.6 Schema drift CI guard

Script `pnpm explorer:schema:check`:
1. Spin Postgres test container.
2. Boot Subscan observer at pinned commit.
3. Wait for tables.
4. Run `prisma db pull --print` against the test DB.
5. Diff against committed `schema.prisma`. Fail if drift.

Triggered on PRs that change `infra/subscan/` pin.

---

## 8. Testing

Coverage target 80% lines/branches per project AGENTS.md, gated on `lib/`, `server/routers/`, `server/lib/`. UI components rely on E2E signal.

### 8.1 Unit (Vitest)

- `packages/shared/src/format/*`: `formatBalance`, `formatHex`, `formatRelativeTime`, `formatTimestamp`, `formatNumber`.
- `packages/explorer/src/server/lib/encode.ts`: `hex`, `addr`, `unix`, `safeNum`, `bigStr`, `nullable`.
- `packages/explorer/src/server/lib/cursor.ts`: round-trip encode/decode.
- `packages/explorer/src/lib/regex.ts`: search classifier.

### 8.2 Integration (Vitest + testcontainers)

- Spin a Postgres container per test suite.
- Apply a `test/fixtures/subscan-schema.sql` snapshot (committed; updated when Subscan pin changes).
- Seed sample rows.
- Call `appRouter.createCaller(ctx).<router>.<procedure>(input)`.
- Assert output shape matches wire contract (string hashes, number block nums, etc.).
- Cover: list pagination forward/backward, detail by id, search resolution (block/tx/eoa/contract).

### 8.3 E2E (Playwright)

Four critical flows on desktop ≥ 1024 only:

1. Home page → click latest block card → land on `/block/[num]` with correct number.
2. Search bar → paste tx hash → redirect to `/tx/[hash]`.
3. `/address/[addr]` → click Transactions tab → click "Next" → URL changes, second page rendered.
4. `/contract/[addr]` for a verified contract → Source Code tab shows non-empty code block.

### 8.4 Out of scope (v1)

- Visual regression screenshots.
- Mobile breakpoints (320, 375, 768) — explorer responsive but not guaranteed.
- Load testing.
- Accessibility audit.
- Security review (no auth, no write paths, public data only).

---

## 9. Open questions and risks

1. **Subscan table naming.** Whether tables are network-prefixed (`artemis_evm_block`) or plain (`evm_block`) — resolved at first `prisma db pull`. Adjust router code accordingly. No design change either way.
2. **Tx fee column shape.** Subscan may expose `effective_gas_price` (EIP-1559) or `gas_price` (legacy) or both. Compute fee at boundary using whatever exists; add unit test fixture covering both.
3. **`evm_account.code` representation.** Whether empty code is `null`, `\\x`, or `\\x00`. Determined at first introspect.
4. **Index coverage.** If `evm_transaction.from`/`to` lack indexes, address tx tab will be slow for hot addresses. v1 accepts this; document the workaround (paginate aggressively, top up indexes via upstream Subscan PR for v2).
5. **Postgres connection sharing.** Subscan API + observer + worker + explorer all hit one Postgres. Pool size 10 each. If contention shows up under load, bump Postgres `max_connections`.

---

## 10. Implementation order (suggested for the plan)

1. Extract `clay/` → `@artemis/coss-ui` and migrate `packages/ui` imports. Verify build green.
2. Add `format/` module to `packages/shared`. Verify build.
3. Scaffold `packages/explorer` with Next.js 16, Tailwind v4, tRPC v11, Prisma.
4. `prisma db pull` against local Subscan, commit schema.
5. Implement `server/lib/encode.ts`, `cursor.ts`, `prisma.ts`, `trpc.ts`, `caller.ts`.
6. Implement routers in order: `metadata` → `block` → `tx` → `address` → `contract` → `search`. Integration test each as it lands.
7. Implement Server Components for routes in same order. Layout + nav + search bar last.
8. Add Client islands (Latest Blocks, Latest Tx, Search Bar, Copy Button, JSON Viewer, Timestamp).
9. Loading/error/not-found segments.
10. E2E suite for the four critical flows.
11. `infra/explorer/` Dockerfile + docker-compose. Dokploy deploy.

---

## 11. References

- `infra/subscan/ui/FUNCTIONAL_SPEC.md` — full Subscan functional spec (22 screens). v1 implements Screens 11-18.
- `infra/subscan/README.md` — Subscan stack, API endpoints, deploy recipe.
- `packages/ui/components/clay/` — current location of coss UI primitives, migrating to `@artemis/coss-ui`.
- `packages/shared/src/abis/` — chain ABIs, unchanged.
- Next.js 16 caching: `'use cache'` + `cacheLife` + `cacheTag` (stable). `unstable_cache` deprecated path.
- tRPC v11 server-side caller: `appRouter.createCaller(ctx)` for Server Components.
