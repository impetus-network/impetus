# Artemis Next Steps Design Spec

## Overview

Six phases of work to bring Artemis from current gasless-registry MVP to a
testable, deployable chain with admin UI, expanded tests, and multi-node
testnet infrastructure.

## Phase 1: Cleanup + Foundation

### Remove betting placeholders

The betting routes and placeholder pages were already removed in a previous
refactor. Verify no leftover references remain:

- Confirm no `/rounds` or `/rounds/:id` routes in
  `packages/webapp/src/App.tsx`
- Confirm no nav links to rounds in `AppLayout`
- Confirm `Rounds.tsx` and `RoundDetail.tsx` do not exist
- If any remnants are found, remove them

### Shared ABI exports

The gasless registry ABI currently lives only in
`packages/indexer/abis/GaslessRegistry.ts`. Both the webapp and indexer need
it. Move the canonical ABI to `@artemis/shared` and re-export.

Changes:

- Add `packages/shared/src/abis/GaslessRegistry.ts` with the full ABI
  constant (same content as the indexer copy)
- Export from `packages/shared/src/index.ts`
- Add TypeScript types for rule data:
  ```
  GaslessRule { contract: Address, selector: Hex, enabled: boolean, minValue: bigint }
  ```
- Update `packages/indexer/abis/GaslessRegistry.ts` to re-export from
  `@artemis/shared` (or import directly in ponder config)
- Webapp will import from `@artemis/shared` in Phase 3

## Phase 2: Indexer GraphQL API

Ponder already generates a GraphQL endpoint from the `onchainTable` schema.
The current `src/api/index.ts` mounts it at `/graphql`.

Changes:

- Verify the existing GraphQL endpoint serves `gaslessRules` queries with
  filtering by `contract`, `enabled`, and pagination
- Add `.env.example` to `packages/indexer/` documenting required env vars:
  `PONDER_RPC_URL_322` (default `http://127.0.0.1:9944`),
  `DATABASE_URL` (default `postgresql://localhost:5432/artemis_indexer`)
- If existing `.env.local` references old `betting_indexer` database name,
  update to `artemis_indexer`
- Ponder dev server runs on port 42069 by default
- No additional API routes needed; Ponder's auto-generated GraphQL is
  sufficient for the admin panel

The webapp admin panel (Phase 3) will query this endpoint directly using
`fetch` with GraphQL queries. No additional client library is required.

## Phase 3: Admin Panel

### Access control

The admin panel is sudo-only. Only the sudo account
(`0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266`, account #0) can access
`/admin`. Any other connected wallet sees an "Unauthorized" message.

The check compares the connected wagmi account address against a hardcoded
sudo address constant (or one read from `@artemis/shared`). This is a UI
guard only; the precompile enforces the real access control on-chain.

### Route

`/admin` replaces the existing placeholder page.

### Features

**List rules:**
- Fetch current gasless rules from Ponder GraphQL endpoint
- Display as a table: contract address, function selector (hex), enabled
  status, minValue (formatted as ART)
- Auto-refresh after mutations

**Add rule:**
- Form fields: contract address (input), function selector (bytes4 hex
  input), minValue (number input in ART), enabled (toggle, default true)
- Submit calls precompile `setRule(address,bytes4,uint256,bool)` via wagmi
  `writeContract` at the gasless registry precompile address
- Show transaction status (pending, success, error)

**Remove rule:**
- Button on each row
- Calls precompile `removeRule(address,bytes4)` via wagmi `writeContract`
- Confirm dialog before submitting

**Toggle enable/disable:**
- Toggle switch on each row
- Calls precompile `setRule` with the existing contract, selector, and
  minValue but toggled `enabled` flag

### Components

- `AdminGuard` — Checks connected wallet is sudo, renders children or
  "Unauthorized" message
- `RuleList` — Table component, fetches from Ponder GraphQL
- `AddRuleForm` — Form with validation (address format, bytes4 hex format,
  non-negative minValue)
- Uses existing coss `Button` component

### Precompile interaction

All write operations go through the gasless registry precompile at
`0x0000000000000000000000000000000000000800`. The ABI is imported from
`@artemis/shared`. wagmi's `writeContract` handles MetaMask signing.

Read operations go through Ponder GraphQL (not direct precompile calls) for
better UX (no RPC call per rule, batch query, filtering).

## Phase 4: Contract Test Expansion

### New test contract

`TestNFT.sol` — Simple ERC721 with a public `mint(address to, uint256
tokenId)` function. Uses OpenZeppelin ERC721. No access control on mint
(test-only contract).

### New test cases

**ERC721 gasless flow** (`GaslessERC721.test.ts`):
- Deploy TestNFT
- Register `transferFrom(address,address,uint256)` selector as gasless
- Mint an NFT to user, transfer via gasless call
- Verify sender balance unchanged (no gas fee)

**Gasless edge cases** (extend `GaslessERC20.test.ts` or new file):
- Set rule then disable via `setRule(..., enabled=false)` — verify fallback
  to paid
- Set rule then remove via `removeRule` — verify fallback to paid
- Multiple rules for same contract, different selectors — verify independent
  evaluation
- Rule with `minValue > 0` — call with value below minimum, verify paid
  fallback; call with sufficient value, verify gasless

**Precompile direct calls** (`GaslessPrecompile.test.ts`):
- Call `getRule` via ethers/viem `readContract`, verify returned
  `(enabled, minValue)` matches what was set
- Call `isGasless` with sample calldata, verify boolean return
- Non-admin calls `setRule` — expect revert with "caller is not admin"
- Non-admin calls `removeRule` — expect revert with "caller is not admin"

### Test infrastructure

All tests use the existing Hardhat setup against a live dev node. Tests use
`@polkadot/api` for sudo operations (registering rules via substrate
extrinsic) or can use the precompile directly from the admin account.

## Phase 5: Webapp E2E Tests

### Framework

Playwright with dappwright for MetaMask automation. Existing fixtures in
`packages/webapp/e2e/fixtures.ts` bootstrap MetaMask with the dev mnemonic.

### Prerequisites

- Dev node running on `localhost:9944`
- Ponder indexer running (for admin panel tests)
- MetaMask configured with Artemis chain (dappwright handles this)

### Test cases

**`e2e/wallet.spec.ts`:**
- Connect MetaMask wallet, verify address appears on Home page
- Verify ART balance displays correctly (non-zero for dev accounts)
- Disconnect wallet, verify "Connect your wallet" prompt

**`e2e/admin.spec.ts`:**
- Connect sudo account (#0), navigate to `/admin`, verify rule list loads
- Connect non-sudo account (#1), navigate to `/admin`, verify
  "Unauthorized" message
- Add a gasless rule via form: fill contract address, selector, minValue,
  submit, confirm in MetaMask, verify rule appears in table
- Remove a rule: click remove button, confirm dialog, confirm in MetaMask,
  verify rule disappears from table

### Setup documentation

Add to webapp README: instructions for running E2E tests (start dev node,
start Ponder, run `pnpm test:e2e`).

## Phase 6: Testnet Deployment

### Docker image

Multi-stage Dockerfile at `packages/node/Dockerfile`:

```
Stage 1: Build
  FROM rust:<version>
  Install wasm32-unknown-unknown target
  Copy source, cargo build --release

Stage 2: Runtime
  FROM debian:bookworm-slim
  Copy binary from build stage
  ENTRYPOINT for node binary
```

### Docker Compose

`docker-compose.yml` at repo root with two services:

**alice:**
- Validator node with `--alice` flag
- Exposes RPC on port `9944`
- Exposes P2P on port `30333`
- Volume for persistent chain data

**bob:**
- Validator node with `--bob` flag
- Peers with alice via `--bootnodes`
- Exposes RPC on port `9945`
- Exposes P2P on port `30334`
- Volume for persistent chain data

Both use the same custom chain spec with:
- Same dev accounts pre-funded with 1M ART each
- Aura + GRANDPA with Alice and Bob as authorities
- Chain ID 322, token ART

### Scripts

- `scripts/docker-build.sh` — Build the node Docker image
- `scripts/docker-up.sh` — Start the 2-node testnet
  (`docker compose up -d`)
- `scripts/docker-down.sh` — Stop and optionally clean up chain data

### Not included

- Cloud deployment automation (AWS, GCP, etc.)
- Indexer or webapp Docker images (node only in this phase)
- SSL, domain names, reverse proxy
- Production chain spec (this is a dev/test setup)

## Testing Strategy

### Per-phase verification

| Phase | Verification |
|-------|-------------|
| 1 | `pnpm turbo build` passes, webapp loads without rounds routes |
| 2 | `curl localhost:42069/graphql` returns schema with gaslessRules |
| 3 | Manual: connect sudo wallet, add/remove/toggle rules via UI |
| 4 | `pnpm test` in contracts package, all new tests pass |
| 5 | `pnpm test:e2e` in webapp package, all E2E tests pass |
| 6 | `docker compose up`, both nodes produce blocks, RPC responds |

### Integration test

After all phases: deploy TestToken via admin account, register transfer
selector as gasless via admin panel, transfer tokens from another account,
verify no gas fee charged — end-to-end across all components.

## Events

No new Substrate pallet events. No new EVM events. All features use existing
gasless registry events (RuleSet, RuleRemoved) emitted by the precompile.

## Error Handling

- Admin panel: show toast/inline error for failed transactions (reverts,
  rejected in MetaMask, network errors)
- Contract tests: assert specific revert reasons ("caller is not admin",
  "remove_rule dispatch failed")
- Docker: health checks on RPC endpoints, restart policy

## Open Risks

- Dappwright MetaMask automation may be fragile across browser/extension
  updates. Mitigate by pinning versions in `playwright.config.ts`.
- Docker multi-stage Rust build is slow (30+ minutes). Mitigate with Docker
  layer caching and cargo-chef for dependency caching.
- Ponder GraphQL schema is auto-generated; if Ponder version changes, query
  format may change. Pin Ponder version in `package.json`.
