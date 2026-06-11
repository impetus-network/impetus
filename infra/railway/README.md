# Deploy Impetus on Railway (5 validators + 1 archive)

Centralized first-phase topology: 6 services in ONE Railway project. Each is the
same image (`infra/railway/Dockerfile`) with a persistent volume and per-node
secrets. p2p uses Railway **private networking** between services, so no public
TCP proxy is needed for validator-to-validator gossip.

> This is a CANARY/centralized launch — all 6 nodes are operated by one party.
> It is not yet trustless. Decentralize later by adding independent operators,
> then moving sudo to a multisig/governance.

## 0. Prerequisites (generated locally, kept secret)

From `apps/node/`:
- `launch-keys/validators.json` — 5 validator session pubkeys (public)
- `launch-keys/secrets/validator-N.env` — each validator's 4 session SURIs (SECRET)
- `launch-keys/node-keys/node-N.key` — each node's p2p key hex (SECRET)
- `launch-keys/node-keys/peer-ids.txt` — PeerIDs (public, below)
- `chain-specs/impetus.json` — the byte-identical raw spec (host it somewhere
  every node can fetch, e.g. a Railway volume, a release asset, or a gist).
  Current sha256: see your build output; all nodes MUST use the same file.

## 1. PeerIDs (public)

These come from the committed node keys. They are baked into the bootnode
multiaddrs below.

| Node        | key file    | PeerID |
|-------------|-------------|--------|
| validator-1 | node-1.key  | `12D3KooWPH4Dp9n9csjEgWy3MWsnrRfEy1ewCYkahG59uNt5S3q9` |
| validator-2 | node-2.key  | `12D3KooWDLzWZxgqwtab7mH2YLb7NWEgpNdkumHmJRySnmEgBjEL` |
| validator-3 | node-3.key  | `12D3KooWATCQCifMZqrAabzLsSa5YQEWPDRw3fVpCukKvPR1swZB` |
| validator-4 | node-4.key  | `12D3KooWBnkAs9C9XPkpGARqxQRC2xsbpo8bT8oG6cMsnM4W8dz6` |
| validator-5 | node-5.key  | `12D3KooWSezFEk7GEDba2rdMDCciGFuFZjwrPFT9HwsxVNzhfHys` |
| archive     | node-6.key  | `12D3KooWQMFKDsV7Uzq9e2jQs7R47MoZEdbrgAU1rDVDNy474ssM` |

## 2. Create the project + 6 services

In one Railway project, create 6 services named exactly:
`validator-1 validator-2 validator-3 validator-4 validator-5 archive`

For each service:
- Source: this repo. Set **Dockerfile path** = `infra/railway/Dockerfile`,
  **Root directory** = repo root (the Dockerfile copies `apps/node/` and
  `infra/railway/entrypoint.sh`).
- Add a **Volume** mounted at `/data` (REQUIRED — holds the keystore + chain DB;
  losing it loses the validator's keys and identity).
- Disable **sleep / serverless** (validators must stay up).
- Private networking is on by default; the internal DNS name is
  `<service>.railway.internal`.

## 3. Bootnodes (Railway internal networking)

Use 2 validators as bootnodes (enough; if one is down the other still seeds).
Every service gets the SAME `BOOTNODES` value:

```
/dns4/validator-1.railway.internal/tcp/30333/p2p/12D3KooWPH4Dp9n9csjEgWy3MWsnrRfEy1ewCYkahG59uNt5S3q9 /dns4/validator-2.railway.internal/tcp/30333/p2p/12D3KooWDLzWZxgqwtab7mH2YLb7NWEgpNdkumHmJRySnmEgBjEL
```

(If you later expose nodes publicly via Railway TCP proxy, swap `*.railway.internal`
for the public host:port and regenerate the multiaddrs — the PeerID is unchanged.)

## 4. Environment variables per service

### All services
| Var | Value |
|-----|-------|
| `CHAIN_SPEC_URL` | URL to the byte-identical `impetus.json` (or pre-place it at `/data/impetus.json`) |
| `BOOTNODES` | the two multiaddrs from section 3 |
| `NODE_NAME` | the service name (validator-1, …, archive) |
| `NODE_KEY_HEX` | contents of that node's `node-keys/node-N.key` (SECRET) |

### Validators (validator-1 .. validator-5) — ALSO
| Var | Value (from `secrets/validator-N.env`) |
|-----|------|
| `ROLE` | `validator` |
| `BABE_SURI` | `BABE_SECRET` |
| `IMON_SURI` | `IM_ONLINE_SECRET` |
| `AUDI_SURI` | `AUTHORITY_DISCOVERY_SECRET` |
| `GRAN_SURI` | `GRANDPA_SECRET` |

### Archive — ALSO
| Var | Value |
|-----|-------|
| `ROLE` | `archive` |

> Store all secrets as Railway **service variables** (or via Railway's secret
> integration), never in the repo. The entrypoint writes them to the volume
> keystore at boot; re-inserting the same seed is a no-op, so redeploys are safe.

## 5. Public RPC (archive)

The `archive` service runs `--rpc-external --rpc-methods safe --pruning archive`.
Add a Railway **TCP proxy** (or HTTP domain) to its port `9944` to serve the
explorer / wallets. Validators keep RPC internal (`--rpc-methods safe`, not
externalised).

## 6. Launch order

1. Deploy `validator-1` and `validator-2` first (the bootnodes). They will sit
   waiting for peers — that's fine.
2. Deploy `validator-3..5` and `archive`. They dial the bootnodes and the set
   forms.
3. Watch logs for block production (validator-1 authors block #1 once a quorum
   of session keys is loaded) and GRANDPA finality. Prometheus metrics on
   `:9615` per node.

## 7. Notes / gotchas

- **minimumValidatorCount = 3** (baked into the spec): the chain tolerates up to
  2 validators offline (e.g. a Railway redeploy) without halting. Don't take 3+
  down at once.
- **No staking inflation** (capped 1B): validators earn from tx fees (50% of
  fees → block author). Keep them up to earn.
- **Volume is sacred**: it holds the keystore (session keys) and the p2p node
  key. A wiped volume = lost validator identity; you'd re-insert keys and the
  node key from your offline `launch-keys/` copies.
- **Same spec everywhere**: all nodes must load the byte-identical
  `impetus.json` (same sha256), or they fork.
