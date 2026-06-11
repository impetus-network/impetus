#!/usr/bin/env bash
#
# run-node.sh — run an Impetus/Impulse node, native or in Docker, prompting for
# your validator secret at runtime so it NEVER lands in a file, image, or shell
# history.
#
# The secret you paste can be a BIP39 mnemonic ("word word ... word"), a raw
# 0x/hex seed, or any Substrate SURI (e.g. "<mnemonic>//stash"). It is read with
# echo OFF and used only to insert your 4 session keys (babe/im_online/
# authority_discovery = sr25519, grandpa = ed25519) into the node keystore.
#
# Two key models:
#   - single   one secret derives all 4 session keys (the dev/`--alice` model).
#              Use for a NEW validator you will register via session.setKeys.
#   - separate paste 4 distinct SURIs, or load them from an env file shaped like
#              apps/node/launch-keys/secrets/validator-N.env (BABE_SECRET, ...).
#              Use to re-run an EXISTING validator whose 4 keys are already
#              registered on-chain (e.g. a genesis/launch validator).
#
# Usage (interactive — just answer the prompts):
#   apps/node/scripts/run-node.sh
#
# Non-interactive (set any of these to skip the matching prompt):
#   RUNTIME=native|docker  CHAIN=impulse|dev  ROLE=validator|full
#   SPEC=<path|url to a raw chain spec>   (REQUIRED to join impetus mainnet —
#         the named `impetus` builder rebuilds genesis from env and will NOT
#         match the live chain. Use the byte-identical raw impetus.json.)
#   NODE_NAME=...  IMAGE=...  BASE_PATH=...  DATA_VOLUME=...  EXTRA_ARGS="..."
#   KEY_MODEL=single|separate|envfile  KEY_ENV_FILE=/path/to/validator-N.env
# Secrets may also be pre-exported to skip the secure prompt entirely:
#   VALIDATOR_SURI=...                     (KEY_MODEL=single)
#   BABE_SURI=... IMON_SURI=... AUDI_SURI=... GRAN_SURI=...   (KEY_MODEL=separate)
#   NODE_KEY_HEX=...                       (32-byte hex p2p key; optional)
#
# SECURITY NOTES
#   - The script reads secrets with `read -rs` (no echo) and never writes them
#     to disk except into the node's own keystore (the keystore IS where a
#     running validator needs them).
#   - NATIVE: `impetus-node key insert --suri <secret>` briefly exposes the SURI
#     in the process argv (visible to `ps` by the SAME user). This matches the
#     stock Substrate tooling; it is acceptable on an operator-owned box. The
#     long-running node process carries NO secret in its argv.
#   - DOCKER: secrets are passed to the insert container via `-e VAR` PASSTHROUGH
#     (value taken from this script's env, not placed on the host argv), so they
#     do not leak into host shell history or `ps`. They are written into the
#     keystore on the mounted volume, same as native.
#
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BIN="${BIN:-$ROOT/target/release/impetus-node}"

# ---- tiny prompt helpers ---------------------------------------------------
is_tty() { [ -t 0 ]; }

# ask VAR "Question" "default" "opt1 opt2 ..."  — only prompts if VAR is unset.
ask() {
  local var="$1" q="$2" def="${3:-}" opts="${4:-}" cur ans
  cur="$(eval "printf '%s' \"\${$var:-}\"")"
  [ -n "$cur" ] && return 0
  if ! is_tty; then
    [ -n "$def" ] || { echo "ERROR: $var unset and no TTY to prompt" >&2; exit 1; }
    printf -v "$var" '%s' "$def"; return 0
  fi
  [ -n "$opts" ] && printf '  options: %s\n' "$opts" >&2
  read -r -p "$q${def:+ [$def]}: " ans </dev/tty
  printf -v "$var" '%s' "${ans:-$def}"
}

# ask_secret VAR "Label"  — reads with echo off; skips if already exported.
ask_secret() {
  local var="$1" label="$2" cur s
  cur="$(eval "printf '%s' \"\${$var:-}\"")"
  [ -n "$cur" ] && return 0
  is_tty || { echo "ERROR: $var unset and no TTY to read secret" >&2; exit 1; }
  read -rs -p "Paste $label (input hidden): " s </dev/tty; echo >&2
  [ -n "$s" ] || { echo "ERROR: empty secret for $var" >&2; exit 1; }
  printf -v "$var" '%s' "$s"
}

die() { echo "ERROR: $*" >&2; exit 1; }

# ---- 1. choose runtime / chain / role --------------------------------------
ask RUNTIME "Run natively or in Docker? (native/docker)" native "native docker"
ask ROLE    "Validator or full node? (validator/full)"    full    "validator full"
case "$RUNTIME" in native|docker) ;; *) die "RUNTIME must be native or docker" ;; esac
case "$ROLE"    in validator|full) ;; *) die "ROLE must be validator or full" ;; esac

# Chain: a NAMED chain (impulse/dev) OR a raw spec via SPEC=<path|url>. The
# mainnet genesis CANNOT be reached by the named `impetus` builder (it rebuilds
# genesis from env -> a different hash); join mainnet with the byte-identical raw
# impetus.json via SPEC=... (the v0.3.0 image bakes it at /opt/impetus.json).
SPEC="${SPEC:-}"
SPEC_LOCAL=""   # host path to the resolved spec file (set when SPEC is used)
if [ -n "$SPEC" ]; then
  case "$SPEC" in
    http://*|https://*)
      SPEC_LOCAL="$(mktemp -t impetus-spec.XXXXXX)"
      echo "Downloading chain spec from $SPEC ..."
      curl -fsSL "$SPEC" -o "$SPEC_LOCAL" || die "failed to download spec: $SPEC" ;;
    *)
      [ -f "$SPEC" ] || die "spec file not found: $SPEC"
      SPEC_LOCAL="$(cd "$(dirname "$SPEC")" && pwd)/$(basename "$SPEC")" ;;
  esac
  CHAIN="${CHAIN:-custom}"
else
  ask CHAIN "Which named chain? (impulse/dev) — for impetus mainnet set SPEC=<raw impetus.json>" dev "impulse dev"
  case "$CHAIN" in
    impulse|dev) ;;
    impetus) echo "WARNING: named 'impetus' rebuilds genesis from env and will NOT match live mainnet. Set SPEC=<raw impetus.json> to join mainnet." >&2 ;;
    *) die "CHAIN must be impulse or dev (or set SPEC=<path|url> for a raw spec)" ;;
  esac
fi

NODE_NAME="${NODE_NAME:-my-${CHAIN}-${ROLE}}"
IMAGE="${IMAGE:-tranhuyduan/impetus-railway:v0.3.0}"
EXTRA_ARGS="${EXTRA_ARGS:-}"

# ---- 2. collect validator secrets (validator role only) --------------------
# Populates BABE_SURI / IMON_SURI / AUDI_SURI / GRAN_SURI in this script's env.
collect_session_keys() {
  ask KEY_MODEL "Key model? (single/separate/envfile)" single "single separate envfile"
  case "$KEY_MODEL" in
    single)
      ask_secret VALIDATOR_SURI "your validator mnemonic / seed / SURI"
      BABE_SURI="$VALIDATOR_SURI"; IMON_SURI="$VALIDATOR_SURI"
      AUDI_SURI="$VALIDATOR_SURI"; GRAN_SURI="$VALIDATOR_SURI"
      ;;
    separate)
      ask_secret BABE_SURI "BABE (sr25519) SURI"
      ask_secret IMON_SURI "IM_ONLINE (sr25519) SURI"
      ask_secret AUDI_SURI "AUTHORITY_DISCOVERY (sr25519) SURI"
      ask_secret GRAN_SURI "GRANDPA (ed25519) SURI"
      ;;
    envfile)
      ask KEY_ENV_FILE "Path to validator-N.env" "" ""
      [ -f "$KEY_ENV_FILE" ] || die "env file not found: $KEY_ENV_FILE"
      # shellcheck disable=SC1090
      ( set -a; . "$KEY_ENV_FILE" ) >/dev/null 2>&1 || true
      # Re-source into THIS shell to read the values (the subshell above only
      # validated it parses). Accept either *_SURI or *_SECRET key names.
      # shellcheck disable=SC1090
      set -a; . "$KEY_ENV_FILE"; set +a
      BABE_SURI="${BABE_SURI:-${BABE_SECRET:-}}"
      IMON_SURI="${IMON_SURI:-${IM_ONLINE_SECRET:-}}"
      AUDI_SURI="${AUDI_SURI:-${AUTHORITY_DISCOVERY_SECRET:-}}"
      GRAN_SURI="${GRAN_SURI:-${GRANDPA_SECRET:-}}"
      ;;
    *) die "KEY_MODEL must be single, separate or envfile" ;;
  esac
  for v in BABE_SURI IMON_SURI AUDI_SURI GRAN_SURI; do
    [ -n "$(eval "printf '%s' \"\${$v:-}\"")" ] || die "missing $v"
  done
  export BABE_SURI IMON_SURI AUDI_SURI GRAN_SURI
}

[ "$ROLE" = "validator" ] && collect_session_keys

# ---- 3a. native ------------------------------------------------------------
run_native() {
  [ -x "$BIN" ] || die "node binary not found at $BIN (build: cd apps/node && cargo build --release)"
  local chain_arg="${SPEC_LOCAL:-$CHAIN}"
  BASE_PATH="${BASE_PATH:-$HOME/.impetus/$CHAIN}"
  mkdir -p "$BASE_PATH"

  if [ "$ROLE" = "validator" ]; then
    echo "Inserting session keys into keystore at $BASE_PATH ..."
    "$BIN" key insert --base-path "$BASE_PATH" --chain "$chain_arg" --scheme sr25519 --key-type babe --suri "$BABE_SURI"
    "$BIN" key insert --base-path "$BASE_PATH" --chain "$chain_arg" --scheme sr25519 --key-type imon --suri "$IMON_SURI"
    "$BIN" key insert --base-path "$BASE_PATH" --chain "$chain_arg" --scheme sr25519 --key-type audi --suri "$AUDI_SURI"
    "$BIN" key insert --base-path "$BASE_PATH" --chain "$chain_arg" --scheme ed25519 --key-type gran --suri "$GRAN_SURI"
    # Drop the secrets from this process's env once they are in the keystore.
    unset BABE_SURI IMON_SURI AUDI_SURI GRAN_SURI VALIDATOR_SURI
  fi

  ARGS=(--base-path "$BASE_PATH" --chain "$chain_arg" --name "$NODE_NAME" --rpc-cors all)
  [ "$ROLE" = "validator" ] && ARGS+=(--validator)
  [ -n "${NODE_KEY_HEX:-}" ] && ARGS+=(--node-key "$NODE_KEY_HEX")
  # shellcheck disable=SC2206
  [ -n "$EXTRA_ARGS" ] && ARGS+=($EXTRA_ARGS)

  echo "Starting native node: $BIN ${ARGS[*]}"
  exec "$BIN" "${ARGS[@]}"
}

# ---- 3b. docker ------------------------------------------------------------
# Overrides the image ENTRYPOINT so we can pass a NAMED chain (impulse/dev) or a
# mounted raw spec (SPEC=...) instead of the Railway baked-spec entrypoint flow.
run_docker() {
  command -v docker >/dev/null || die "docker not found"
  DATA_VOLUME="${DATA_VOLUME:-impetus-${CHAIN}-data}"
  CONTAINER="${CONTAINER:-impetus-${CHAIN}-${ROLE}}"
  RPC_PORT="${RPC_PORT:-9944}"
  P2P_PORT="${P2P_PORT:-30333}"
  docker volume create "$DATA_VOLUME" >/dev/null

  # When SPEC is used, mount it read-only at /spec.json and point --chain there;
  # otherwise pass the named chain. Both the insert and run containers must use
  # the SAME chain value (the keystore path is keyed by the chain id).
  local chain_arg; local spec_mount=()
  if [ -n "$SPEC_LOCAL" ]; then chain_arg="/spec.json"; spec_mount=(-v "$SPEC_LOCAL:/spec.json:ro"); else chain_arg="$CHAIN"; fi

  if [ "$ROLE" = "validator" ]; then
    echo "Inserting session keys into volume $DATA_VOLUME ..."
    # `-e VAR` passes the value from THIS script's env (not the host argv), so
    # secrets stay out of shell history / ps. The insert runs as one container.
    docker run --rm \
      -e BABE_SURI -e IMON_SURI -e AUDI_SURI -e GRAN_SURI \
      -v "$DATA_VOLUME:/data" ${spec_mount[@]+"${spec_mount[@]}"} --entrypoint sh "$IMAGE" -c '
        set -e
        impetus-node key insert --base-path /data --chain '"$chain_arg"' --scheme sr25519 --key-type babe --suri "$BABE_SURI"
        impetus-node key insert --base-path /data --chain '"$chain_arg"' --scheme sr25519 --key-type imon --suri "$IMON_SURI"
        impetus-node key insert --base-path /data --chain '"$chain_arg"' --scheme sr25519 --key-type audi --suri "$AUDI_SURI"
        impetus-node key insert --base-path /data --chain '"$chain_arg"' --scheme ed25519 --key-type gran --suri "$GRAN_SURI"
      '
    unset BABE_SURI IMON_SURI AUDI_SURI GRAN_SURI VALIDATOR_SURI
  fi

  ARGS=(--base-path /data --chain "$chain_arg" --name "$NODE_NAME" --rpc-cors all --rpc-external --rpc-port "$RPC_PORT" --port "$P2P_PORT")
  [ "$ROLE" = "validator" ] && ARGS+=(--validator)
  [ -n "${NODE_KEY_HEX:-}" ] && ARGS+=(--node-key "$NODE_KEY_HEX")
  # shellcheck disable=SC2206
  [ -n "$EXTRA_ARGS" ] && ARGS+=($EXTRA_ARGS)

  echo "Starting container $CONTAINER from $IMAGE ..."
  docker rm -f "$CONTAINER" >/dev/null 2>&1 || true
  docker run -d --name "$CONTAINER" --restart unless-stopped \
    -p "$RPC_PORT:$RPC_PORT" -p "$P2P_PORT:$P2P_PORT" \
    -v "$DATA_VOLUME:/data" ${spec_mount[@]+"${spec_mount[@]}"} --entrypoint impetus-node "$IMAGE" "${ARGS[@]}"
  echo "Started. Logs:  docker logs -f $CONTAINER"
}

case "$RUNTIME" in
  native) run_native ;;
  docker) run_docker ;;
esac
