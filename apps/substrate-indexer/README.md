# Impetus substrate indexer (Subsquid / Squid SDK)

Indexes NPoS activity from the **Substrate events/calls** that `pallet-staking`
and `pallet-nomination-pools` already emit — no runtime change, and it captures
**all** activity (EVM precompile, native polkadot.js, genesis), unlike EVM logs.

Design rationale and the event/call/storage map: see
[`docs/substrate-indexer-plan.md`](../../docs/substrate-indexer-plan.md).

- **Tool:** classic Squid SDK (`@subsquid/substrate-processor`), **RPC-only**
  (`setRpcEndpoint`, no `setGateway`) because Impetus is not an SQD Portal
  dataset. (The Pipes SDK / Portal cannot index it — wrong tool.)
- **Standalone:** npm-managed, excluded from the pnpm workspace. Run all commands
  from this directory.
- **Addresses:** Impetus uses `AccountId20`, so accounts are `0x`-hex (= H160) —
  no SS58 conversion.
- **Node:** use LTS **20 or 22**.

## Prerequisite

The Impetus **`archive`** Railway service is the endpoint — it already runs
`--rpc-external --rpc-methods safe --pruning archive`, so it serves full history
over its native Substrate JSON-RPC on port 9944 (`state_*` / `chain_*`, and also
`eth_*` — same port). `safe` methods are sufficient for the processor.

Point `RPC_ENDPOINT` at it:
- **Recommended:** deploy this squid as a service in the **same Railway project**
  and use `http://archive.railway.internal:9944` (private networking).
- Outside Railway: add a Railway TCP proxy / HTTP domain to `archive:9944`.

**Do NOT** use the Caddy `rpc-proxy` → it targets the **pruned** `rpc-singapore`
node (no history) and is eth-only + Origin-gated.

## Finish the scaffold (one-time)

```bash
cd apps/substrate-indexer
cp .env.example .env          # set RPC_ENDPOINT + DB_*
npm install

# 1. Capture runtime metadata history from the chain (needs the RPC).
npm run metadata              # -> versions.jsonl

# 2. Generate typed pallet decoders from that metadata.
npm run typegen               # -> src/types
#    Then open src/mapping.ts and confirm the `.v9` version keys match what
#    typegen emitted (search for ".v9"); adjust if the chain uses other keys.

# 3. Generate TypeORM entities from schema.graphql.
npm run codegen               # -> src/model

# 4. Bring up Postgres and generate the initial migration.
docker compose up -d db
npm run build
npx squid-typeorm-migration generate

# 5. Run it.
npm run build
npx squid-typeorm-migration apply
npm run processor             # in one shell
npm run serve                 # GraphQL on :4350 in another
```

Or, once `src/types` + `db/migrations` exist, run everything in Docker:

```bash
docker compose up --build
```

## EVM block explorer (block / tx / address, Etherscan-style)

Alongside the staking/holders entities, the processor indexes two EVM entities
served by the same GraphQL API (`indexer.impetus.network/graphql`), which the
dApp's block explorer (`apps/ui`) queries. Address balances reuse `Holder`
(now incl. `frozen` -> the explorer's "locked").

| Entity | Feeds | Source |
|--------|-------|--------|
| `Block` | block list + detail, search-by-number/hash | `eth_getBlockByNumber` |
| `EvmTransaction` | tx detail, per-block + per-address tx lists | `eth_getBlockByNumber` + `eth_getBlockReceipts` |
| `Holder` | address balance (free/reserved/frozen/nonce) | `System.Account` storage |

How it works (see `src/evm-rpc.ts`, `src/evm-build.ts`):

- EVM block/tx/receipt data is sourced via the node's **`eth_*` RPC on the same
  `RPC_ENDPOINT`** (Frontier serves it on the substrate RPC port) -- no
  pallet-ethereum SCALE decoding, no extra typegen. Substrate height == EVM
  block number 1:1.
- The entities are written through the Squid **TypeORM store** in the same batch
  as the staking entities, so `supportHotBlocks` rolls them back automatically on
  a reorg. A null block or missing receipt **throws** (Squid retries the batch)
  -- never a silent gap.
- Query patterns: `blocks(orderBy: height_DESC)`, `blockById`,
  `evmTransactions(where: {OR:[{from_eq},{to_eq}]}, orderBy: [block_DESC, txIndex_DESC])`,
  `evmTransactionById(id: <hash>)`.

> **Go-live / backfill — IMPORTANT.** The processor only writes these entities for
> blocks it processes *after* this code ships, and the consolidated migration only
> applies on an empty DB. So backfill from genesis by **re-syncing on a fresh DB
> volume** (bump the `pgdata*` volume name in `docker-compose.yml`). The indexer
> DB is fully re-derivable from chain, so wiping it is safe; the chain is young so
> a full re-sync is cheap.

## Wire the dApp (follow-up)

Add `NEXT_PUBLIC_SQUID_URL=http://<host>:4350/graphql` and switch `/validators`
and `/pools` to read this GraphQL (today they read the precompiles directly via
wagmi + the `KNOWN_VALIDATORS` config — keep that as a fallback until the squid
is live and backfilled).

## Indexer division

- **Ponder** (`apps/indexer`): EVM logs → gasless registry (mainnet 388266).
- **This squid**: Substrate events/calls/storage → validators, nominators,
  pools, eras, payouts, stake events.

## Known follow-ups (marked in code)

- `Validator.elected` / `selfBonded`: needs a storage read on
  `Staking.StakersElected` (`Session.Validators`, `Staking.Ledger`). The
  `main.ts` handler has a `TODO` where this goes; `typegen.json` already
  requests those storage items.
- `src/mapping.ts` version keys (`.v9`) and the `signerOf` origin shape must be
  confirmed against the generated types after `npm run typegen`.
