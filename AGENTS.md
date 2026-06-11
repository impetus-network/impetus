# Impetus / Impulse - Blockchain Monorepo

EVM-compatible Substrate solochain (Frontier) with precompile extensibility, indexer, and Next.js UI.

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
