# Impetus / Impulse - Blockchain Monorepo

EVM-compatible Substrate solochain (Frontier) with precompile extensibility, indexer, and Next.js UI.

## Second Brain Mapping

Use `/Users/huyduan/knowledge` for prior decisions, project context, and cross-project knowledge.

Relevant pages:

- `/Users/huyduan/knowledge/wiki/projects/impetus-blockchain.md`
- `/Users/huyduan/knowledge/wiki/projects/impetus-infrastructure.md`
- `/Users/huyduan/knowledge/wiki/concepts/evm-compatible-substrate-solochain.md`
- `/Users/huyduan/knowledge/wiki/decisions/impetus-chain-launch-ops-model.md`
- `/Users/huyduan/knowledge/wiki/syntheses/impetus-infrastructure-operating-model.md`
- `/Users/huyduan/knowledge/wiki/syntheses/token-platform-architecture-options.md`

Repo source is truth for current implementation. The second brain is truth for prior decisions, rationale, boundaries, and reusable context. If they conflict, prefer repo source and update or flag the wiki.

- **Mainnet:** Impetus (chain id `388266`, token `IPT`, SS58 prefix `11434`, runtime `spec_name="impetus"`)
- **Testnet:** Impulse (chain id `322644`, token `IPL`, SS58 prefix `11348`, runtime `spec_name="impulse"`)
- **Dev mode:** alias of Impulse with manual seal enabled (`--chain dev`)
- **Decimals:** 18

## Architecture

```
apps/
  node/       Substrate node (Rust) -- Frontier solochain template
  ui/         Next.js frontend (wagmi + RainbowKit + Tailwind v4)
  indexer/    Ponder indexer (TypeScript)
packages/
  contracts/  Solidity interface + Hardhat E2E tests (TypeScript)
  shared/     Shared constants, types, and ABI (TypeScript)
```

- **Rust** for the node runtime and custom pallets/precompiles
- **TypeScript** for E2E tests, shared package, UI, and indexer
- Turborepo handles build orchestration and caching

## Build

```bash
# Install dependencies
pnpm install

# Build all TypeScript packages
pnpm turbo build

# Build the Substrate node
cd apps/node && cargo build --release
```

## Run tests

```bash
# Start dev node first (manual seal, pre-funded Hardhat dev users)
cd apps/node && ./target/release/impetus-node --chain dev --tmp --alice

# Run mainnet/testnet locally
./target/release/impetus-node --chain impetus --tmp --validator
./target/release/impetus-node --chain impulse --tmp --validator

# Run E2E tests against the live node
cd packages/contracts && pnpm test
```

## Run on Railway (canary mainnet)

Everything for the cloud deploy lives in `infra/railway/`
(`Dockerfile`, `entrypoint.sh`, `deploy.sh`, `rpc-proxy/`); `README.md` there is
the full runbook. Topology: ONE Railway project, 6 node services
(`validator-1..5` + `archive`) + `rpc-singapore` (pruned RPC node) behind
`rpc-proxy` (Caddy). p2p meshes over Railway **private networking**
(`<service>.railway.internal`), spread across all 4 Railway regions.

```bash
# Deploy / reconcile all 6 nodes (idempotent). Run after `railway login`.
# CHAIN_SPEC_URL must serve the byte-identical raw impetus.json.
CHAIN_SPEC_URL="https://.../impetus.json" infra/railway/deploy.sh

# Logs / metrics
railway logs --service validator-1
railway metrics --service archive --memory --json
```

Operational invariants — violating these halts the chain or slashes validators:

- **The image is prebuilt and pushed to Docker Hub; `entrypoint.sh` is fully
  env-driven.** Node flags change via Railway service variables with NO rebuild.
  Only rebuild for runtime/binary changes. `ROLE` selects the profile:
  `validator | rpc | archive`. Tuning knobs (all optional, per service):
  `STATE_PRUNING`, `BLOCKS_PRUNING`, `DB_CACHE`, `TRIE_CACHE_SIZE`,
  `MAX_RUNTIME_INSTANCES` (never <8 on a validator — too low → "Ran out of free
  WASM instances" + missed slots), and `EXTRA_ARGS` (raw flag passthrough).
- **`RAILWAY_RUN_UID=0` is required on every service** — Railway volumes mount
  root-owned but the image runs as uid 10000; without it the spec fetch fails
  and the container crash-loops.
- **`BOOTNODES` lists ALL 6 nodes (full mesh).** Fewer bootnodes → star topology
  (2 peers each) because authority-discovery can't advertise on private
  networking without `--public-addr`.
- **NEVER restart more than one validator at a time.** Rapid/parallel validator
  redeploys cause GRANDPA/BABE **equivocation** → the validator is disabled for
  the era + deferred slash. Wait for a full rejoin before touching the next.
  Region moves use `railway scale --service X sfo=0 <region>=1` (MOVE, not add —
  two replicas with the same keys = guaranteed equivocation).
- **The `/data` volume is sacred** (keystore + p2p node key + GRANDPA
  last-voted state). A wipe loses validator identity and drops the anti-double-vote
  guard. Re-seed keys from the offline `apps/node/launch-keys/` copies.
- **Same byte-identical `impetus.json` (same sha256) on every node**, or they fork.

## Key conventions

- Rust for node, pallets, and precompile logic; TypeScript strict mode for everything else
- No `any` in TypeScript; named imports; 2-space indent
- Immutable data structures; no `console.log` in production
- Commit messages: `<type>(<scope>): <subject>` (feat, fix, chore, docs, etc.)
- Atomic commits split by concern
- Organize by feature/domain, not by type
- Turborepo handles build orchestration and caching
- **BABE consensus invariant:** the runtime drives sessions from BABE slots —
  `pallet_session::Config` MUST use `ShouldEndSession = Babe` /
  `NextSessionRotation = Babe` (with `EpochChangeTrigger = ExternalTrigger`),
  NEVER block-based `PeriodicSessions`. Block-based sessions on a BABE chain
  deadlock permanently at the first epoch boundary under missed slots.

## Sudo / admin account

Sudo is account #0 of a project-specific mnemonic, kept off-tree in
`ADMIN_MNEMONIC` (see `.env.example`). The address is pinned in genesis:

| Index | Address                                      | Role          |
|-------|----------------------------------------------|---------------|
| #0    | `0xd2aE0A2139dC83Cb920e3cd7B9F640922D14b872` | sudo / admin  |

Generate a fresh mnemonic with `cast wallet new-mnemonic --words 24 --accounts 1`
and populate `ADMIN_MNEMONIC` / `ADMIN_ADDRESS` / `ADMIN_PRIVATE_KEY` in `.env`.

## Dev users

Pre-funded helper accounts derived from the Hardhat mnemonic. No privileged
role — used only by the E2E suites and wallets seeded from the canonical
Hardhat mnemonic.

Mnemonic: `test test test test test test test test test test test junk`

| Index | Address                                      | Role          |
|-------|----------------------------------------------|---------------|
| #0    | `0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266` | dev user      |
| #1    | `0x70997970C51812dc3A010C7d01b50e0d17dc79C8` | dev user      |
| #2    | `0x3C44CdDdB6a900fa2b585dd299e03d12FA4293BC` | dev user      |
| #3    | `0x90F79bf6EB2c4f870365E785982E1f101E93b906` | dev user      |
| #4    | `0x15d34AAf54267DB7D7c367839AAf71A00a2C6A65` | dev user      |

Sudo and each dev user are pre-funded with 1,000,000 IPT (mainnet) / IPL
(testnet/dev) at genesis.

## TypeScript / frontend

The full workspace is `apps/{node, ui, explorer, indexer}` +
`packages/{contracts, shared, coss-ui}`. Package manager is pinned
`pnpm@10.8.0` (corepack); Turborepo tasks: `build`, `test`, `lint`,
`copy-abi`, `node:build`, `dev`.

- **The npm package scope is `@artemis/*`** (legacy — the repo/org is now
  `impetus-network/impetus`). Every filter uses it: `pnpm --filter @artemis/ui
  …`. Renaming the scope is a wide, high-risk refactor (every import +
  package.json + tsconfig path + Dockerfile) — leave it unless explicitly asked.
- **`packages/coss-ui` (`@artemis/coss-ui`)** is a Base-UI component-primitive
  library, **source-only, consumed via Next `transpilePackages`** (no build
  step). Its `./clay` subpath export is the UI's design-system layer. Any
  Dockerfile that builds `apps/ui` MUST `COPY` *both* `packages/shared` and
  `packages/coss-ui` (and their package.json in the deps stage) — omitting
  coss-ui makes `next build` fail with a webpack module-not-found on the first
  page importing `@artemis/coss-ui/clay`.

```bash
pnpm turbo build | test | lint           # whole monorepo
pnpm --filter @artemis/ui dev            # Next.js dApp dev server
pnpm --filter @artemis/shared test       # vitest;   single: -t "<name>"
pnpm --filter @artemis/contracts test    # Hardhat E2E against a RUNNING node; single: --grep "<name>"
pnpm dump-session-keys                   # tsx scripts/dump-session-keys.ts
```

- **Chain metadata lives in two places that must stay in sync:**
  `apps/ui/config/chains.ts` (the wagmi/viem chain the dApp connects to —
  id/name/symbol/rpcUrls) and `packages/shared/src/constants/chain.ts`
  (`CHAIN_CONFIG`). Keep `chainId`/`tokenSymbol` aligned. `CHAIN_CONFIG.rpcUrl`
  is NOT read at runtime — the UI's RPC comes from the wagmi chain's `rpcUrls`.
- The block explorer reads blocks **directly from the RPC via viem/wagmi**
  (`useBlockNumber`/`usePublicClient`), NOT from the Ponder indexer. A UI stuck
  at "block 0" means the RPC call is failing, not an indexer problem.
- Public RPC is the Caddy `rpc-proxy` (`infra/railway/rpc-proxy/`), which only
  admits browser `Origin`/`Referer` on `*.impetus.network` (else `403`). The
  dApp's in-browser RPC calls therefore only work when served from
  `*.impetus.network`; `localhost`/preview origins get 403.
- The `apps/ui` Docker build injects build-time env via **Infisical**
  (`secret-manager.impetus.network`) — do not bake those creds into a `RUN`
  command (they leak into build logs); prefer BuildKit secret mounts.

## Node ops — additional invariants

(complements the Railway runbook above)

- **The node image BAKES the chain spec at `/opt/impetus.json`**
  (`CHAIN_SPEC_PATH`); `CHAIN_SPEC_URL` (the gist) is only a cold-start fallback
  fetched when that file is absent. The canonical Impetus genesis hash is
  `0x1e79…f885`. If the gist drifts from the baked spec, a freshly-initialised
  node forks — and because every node shares `protocolId=impetus`, the genesis
  mismatch is masked (peers connect, then reject each other's blocks / `Genesis
  mismatch`) instead of failing loudly.
- **A fresh pruned `rpc` node cannot full-sync from genesis** — the validators
  are pruned and can't serve historical block bodies, so it stalls at `#0`
  (`Not requested block data` / 0 bps). Give it `EXTRA_ARGS=--sync warp` to warp
  to the finalised head via the GRANDPA proof (env-only, no rebuild). On an
  already-synced DB warp is a no-op.
- **Verify persistence with `df /data` inside the container** — it must show a
  real device (e.g. `/dev/zd*`), NOT `overlay`. If a service's Railway volume is
  missing, `/data` silently falls back to the ephemeral container overlay and
  ALL chain state is lost on the next redeploy (the chain only survives while the
  container runs uninterrupted). Recreate with `railway volume -s <id> add -m /data`.
- **Nodes default to the fork-aware transaction pool** (`--pool-type
  fork-aware`). Its essential `txpool-background` task has crashed every node in
  the fleet simultaneously (around reorgs) — `Essential task 'txpool-background'
  failed. Shutting down service.`. The env-only mitigation is
  `EXTRA_ARGS=--pool-type single-state` (legacy stable pool, no rebuild).
