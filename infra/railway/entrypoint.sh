#!/usr/bin/env bash
#
# Railway entrypoint for an Impetus node (validator or archive).
#
# Loads the node's p2p key and (for validators) its 4 session-key seeds from
# environment variables into the keystore on the persistent volume, then starts
# the node. Secrets are passed as Railway service variables — NEVER baked into
# the image or committed.
#
# Required env (all nodes):
#   CHAIN_SPEC_URL     URL or path to the byte-identical impetus.json
#   NODE_NAME          telemetry/display name (e.g. validator-1)
#   NODE_KEY_HEX       32-byte hex p2p node key (from launch-keys/node-keys/node-N.key)
#   BOOTNODES          space-separated bootnode multiaddrs
#   DATA_DIR           base path (default /data — mount a Railway volume here)
#
# Validators ALSO need (the 4 session-key secret phrases from secrets/validator-N.env):
#   BABE_SURI, IMON_SURI, AUDI_SURI, GRAN_SURI
#   ROLE=validator
#
# Archive / RPC node:
#   ROLE=archive
#
set -euo pipefail

DATA_DIR="${DATA_DIR:-/data}"
NODE_BIN="${NODE_BIN:-impetus-node}"
CHAIN="${CHAIN_SPEC_PATH:-$DATA_DIR/impetus.json}"
P2P_PORT="${P2P_PORT:-30333}"
RPC_PORT="${RPC_PORT:-9944}"
PROM_PORT="${PROM_PORT:-9615}"

mkdir -p "$DATA_DIR"

# --- 1. Chain spec ----------------------------------------------------------
# Prefer a baked/mounted file; otherwise fetch from CHAIN_SPEC_URL once.
if [ ! -f "$CHAIN" ]; then
  if [ -n "${CHAIN_SPEC_URL:-}" ]; then
    echo "[entrypoint] fetching chain spec from $CHAIN_SPEC_URL"
    curl -fsSL "$CHAIN_SPEC_URL" -o "$CHAIN"
  else
    echo "[entrypoint] ERROR: no chain spec at $CHAIN and CHAIN_SPEC_URL unset" >&2
    exit 1
  fi
fi

# --- 2. Node (p2p) key ------------------------------------------------------
# Write the stable node key so the PeerID matches the bootnode multiaddrs.
NODE_KEY_FILE="$DATA_DIR/node.key"
if [ -n "${NODE_KEY_HEX:-}" ]; then
  printf '%s' "$NODE_KEY_HEX" > "$NODE_KEY_FILE"
  chmod 600 "$NODE_KEY_FILE"
else
  echo "[entrypoint] ERROR: NODE_KEY_HEX is required (stable PeerID)" >&2
  exit 1
fi

# --- 3. Session keys (validators only) --------------------------------------
# Insert the 4 session-key seeds into the keystore (idempotent: re-inserting the
# same seed is a no-op). KeyTypes: babe/imon/audi/gran.
insert_key() {
  local scheme="$1" keytype="$2" suri="$3"
  [ -n "$suri" ] || { echo "[entrypoint] ERROR: missing $keytype SURI" >&2; exit 1; }
  "$NODE_BIN" key insert --base-path "$DATA_DIR" --chain "$CHAIN" \
    --scheme "$scheme" --key-type "$keytype" --suri "$suri"
}

if [ "${ROLE:-archive}" = "validator" ]; then
  echo "[entrypoint] inserting validator session keys for $NODE_NAME"
  insert_key sr25519 babe "${BABE_SURI:-}"
  insert_key sr25519 imon "${IMON_SURI:-}"
  insert_key sr25519 audi "${AUDI_SURI:-}"
  insert_key ed25519 gran "${GRAN_SURI:-}"
fi

# --- 4. Common flags --------------------------------------------------------
# Everything below is env-driven so node flags can change WITHOUT rebuilding the
# image. Use EXTRA_ARGS as a raw escape hatch for any flag not modelled here
# (e.g. EXTRA_ARGS="--public-addr /dns4/host/tcp/30333 --in-peers 50").
RPC_CORS="${RPC_CORS:-all}"
RPC_METHODS="${RPC_METHODS:-safe}"
RPC_MAX_CONNECTIONS="${RPC_MAX_CONNECTIONS:-2000}"

# Memory budget (env-overridable). Defaults are tuned LOW for small Railway
# instances; raise per service if a node needs more headroom.
#   DB_CACHE              RocksDB/ParityDB cache, MiB (substrate default ~1024)
#   TRIE_CACHE_SIZE       trie cache, bytes (0 disables; default 67108864)
#   MAX_RUNTIME_INSTANCES parallel wasm instances (default 8)
#   RUNTIME_CACHE_SIZE    cached runtimes (default 2)
DB_CACHE="${DB_CACHE:-256}"
TRIE_CACHE_SIZE="${TRIE_CACHE_SIZE:-33554432}"
MAX_RUNTIME_INSTANCES="${MAX_RUNTIME_INSTANCES:-2}"
RUNTIME_CACHE_SIZE="${RUNTIME_CACHE_SIZE:-2}"

COMMON=(
  --base-path "$DATA_DIR"
  --chain "$CHAIN"
  --name "${NODE_NAME:-impetus-node}"
  --node-key-file "$NODE_KEY_FILE"
  --port "$P2P_PORT"
  --rpc-port "$RPC_PORT"
  --prometheus-port "$PROM_PORT"
  --prometheus-external
  --rpc-cors "$RPC_CORS"
  --db-cache "$DB_CACHE"
  --trie-cache-size "$TRIE_CACHE_SIZE"
  --max-runtime-instances "$MAX_RUNTIME_INSTANCES"
  --runtime-cache-size "$RUNTIME_CACHE_SIZE"
)
# Bootnodes (space-separated multiaddrs).
if [ -n "${BOOTNODES:-}" ]; then
  # shellcheck disable=SC2206
  COMMON+=(--bootnodes ${BOOTNODES})
fi
# Raw extra flags from env (space-separated), appended verbatim. No rebuild needed.
if [ -n "${EXTRA_ARGS:-}" ]; then
  # shellcheck disable=SC2206
  COMMON+=(${EXTRA_ARGS})
fi

# Pruning is env-driven (STATE_PRUNING / BLOCKS_PRUNING). Per-role defaults below
# (archive -> keep everything; validator/rpc -> keep a small recent window).
build_prune_flags() {
  local default_state="$1" default_blocks="$2"
  local sp="${STATE_PRUNING:-$default_state}"
  local bp="${BLOCKS_PRUNING:-$default_blocks}"
  PRUNE_FLAGS=()
  [ -n "$sp" ] && PRUNE_FLAGS+=(--state-pruning "$sp")
  [ -n "$bp" ] && PRUNE_FLAGS+=(--blocks-pruning "$bp")
  # Explicit success: a trailing failed test (e.g. empty $bp) must not abort the
  # script under `set -e`.
  return 0
}

# --- 5. Start ---------------------------------------------------------------
case "${ROLE:-archive}" in
  validator)
    # Validators don't serve historical RPC, so keep only a small recent window.
    # Default state window ~256 blocks; override with STATE_PRUNING (e.g. 128).
    build_prune_flags 256 ""
    echo "[entrypoint] starting VALIDATOR $NODE_NAME (state-pruning ${STATE_PRUNING:-256})"
    exec "$NODE_BIN" "${COMMON[@]}" "${PRUNE_FLAGS[@]}" \
      --validator \
      --rpc-methods "$RPC_METHODS"
    ;;
  rpc)
    # Pruned full node serving external RPC, behind the Caddy proxy that enforces
    # the caller allow-list (the node trusts the proxy via --rpc-cors).
    build_prune_flags 256 ""
    echo "[entrypoint] starting RPC (pruned) $NODE_NAME (state-pruning ${STATE_PRUNING:-256})"
    exec "$NODE_BIN" "${COMMON[@]}" "${PRUNE_FLAGS[@]}" \
      --rpc-external \
      --rpc-methods "$RPC_METHODS" \
      --rpc-max-connections "$RPC_MAX_CONNECTIONS"
    ;;
  archive|*)
    # Archive node: full history + external RPC for explorer/historical queries.
    build_prune_flags archive archive
    echo "[entrypoint] starting ARCHIVE/RPC $NODE_NAME"
    exec "$NODE_BIN" "${COMMON[@]}" "${PRUNE_FLAGS[@]}" \
      --rpc-external \
      --rpc-methods "$RPC_METHODS" \
      --rpc-max-connections "$RPC_MAX_CONNECTIONS"
    ;;
esac
