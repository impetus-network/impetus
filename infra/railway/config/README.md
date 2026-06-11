# Railway service configuration snapshot

Authoritative, **non-secret** snapshot of the live Railway env for each service
in the `impetus` project (project id `72b16d53-77fc-450a-96eb-b80ac40aeb90`,
environment `production`). Captured 2026-06-10 from `railway variable list`.

This is config-as-code: it makes the deployment reproducible and reviewable.
The live source of truth is still Railway; re-run `infra/railway/deploy.sh`
(or set vars by hand) to reconcile a service back to this snapshot.

## Files

- `_common.env` — vars identical across every NODE service (validators +
  archive + rpc-singapore). Each per-service file lists only its deltas.
- `<service>.env` — per-service config (`ROLE`, `NODE_NAME`, role-specific
  tuning) plus a header with region / service id / volume.
- `rpc-proxy.env` — the Caddy reverse-proxy service (no node config).

## Secrets are NOT here

These never enter git; they live off-tree in `apps/node/launch-keys/`
(gitignored) and are pushed to Railway as service variables by `deploy.sh`:

| Var | Source |
|-----|--------|
| `NODE_KEY_HEX` | `apps/node/launch-keys/node-keys/node-N.key` |
| `BABE_SURI` `IMON_SURI` `AUDI_SURI` `GRAN_SURI` | `apps/node/launch-keys/secrets/validator-N.env` |

Railway also auto-injects `RAILWAY_*` vars (service id, private domain, etc.);
those are platform-managed and intentionally omitted here.

## Image

All node services run `tranhuyduan/impetus-railway:v0.3.0` (chain spec baked at
`/opt/impetus.json`, so `CHAIN_SPEC_PATH` points there and `CHAIN_SPEC_URL` is a
cold-start fallback only). `rpc-proxy` runs `tranhuyduan/impetus-rpc-proxy`.
The entrypoint is fully env-driven — these vars change node flags with no rebuild.
