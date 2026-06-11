#!/usr/bin/env bash
#
# Deploy / reconcile Impetus to Railway from infra/railway/config/.
#
# Non-secret config is the committed snapshot in config/*.env (the single source
# of truth: _common.env + one <service>.env per service). Secrets come from
# apps/node/launch-keys/ (gitignored). Re-running reconciles every service back
# to the snapshot — no image rebuild needed (the entrypoint is env-driven).
#
# Services: 5 validators + archive + rpc-singapore (the Impetus node image) plus
# rpc-proxy (the Caddy reverse proxy image). Validators spread across all 4
# Railway regions (validator-5 doubles up); p2p meshes over private networking.
#
# Prereqs (interactive, once):  brew install railway && railway login
# Then, from the repo root:      infra/railway/deploy.sh
#
set -euo pipefail

# --- config -----------------------------------------------------------------
PROJECT_NAME="${PROJECT_NAME:-impetus}"
IMAGE="${IMAGE:-tranhuyduan/impetus-railway:v0.3.0}"
PROXY_IMAGE="${PROXY_IMAGE:-tranhuyduan/impetus-rpc-proxy:v0.1.0}"
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
CONFIG_DIR="$REPO_ROOT/infra/railway/config"
KEYS_DIR="$REPO_ROOT/apps/node/launch-keys"
NODE_KEYS_DIR="$KEYS_DIR/node-keys"
MOUNT_PATH="/data"

# Node services — name -> node key file -> region (index-aligned).
SERVICES=(validator-1 validator-2 validator-3 validator-4 validator-5 archive rpc-singapore)
NODE_KEY_FILES=(node-1.key node-2.key node-3.key node-4.key node-5.key node-6.key node-7-rpc.key)
REGIONS=(us-west us-east eu-west southeast-asia us-east eu-west southeast-asia)

# --- helpers ----------------------------------------------------------------
log() { printf '\033[1;34m[deploy]\033[0m %s\n' "$*"; }
die() { printf '\033[1;31m[deploy] ERROR:\033[0m %s\n' "$*" >&2; exit 1; }

command -v railway >/dev/null || die "railway CLI not found (brew install railway)"
railway whoami >/dev/null 2>&1 || die "not logged in — run: railway login"
[ -d "$CONFIG_DIR" ] || die "config dir missing: $CONFIG_DIR"
[ -d "$NODE_KEYS_DIR" ] || die "node keys missing: $NODE_KEYS_DIR"

set_var()       { railway variable set "$2=$3" --service "$1" --skip-deploys >/dev/null; }
service_exists() { railway variable list --service "$1" >/dev/null 2>&1; }
service_has_volume() {
  railway volume list --json 2>/dev/null \
    | python3 -c "import sys,json;d=json.load(sys.stdin);v=d.get('volumes',d) if isinstance(d,dict) else d;exit(0 if any(s.get('serviceName')==\"$1\" for s in v) else 1)" 2>/dev/null
}

# Push every non-secret KEY=VALUE from a config env file onto a service.
# Comments (#...) and blank lines are skipped; secrets are never in these files.
push_config_file() {
  local svc="$1" file="$2" line key val
  [ -f "$file" ] || die "config file missing: $file"
  while IFS= read -r line || [ -n "$line" ]; do
    case "$line" in ''|\#*) continue ;; esac
    key="${line%%=*}"; val="${line#*=}"
    [ -n "$key" ] || continue
    set_var "$svc" "$key" "$val"
  done < "$file"
}

# Set a validator's 4 session SURIs from its gitignored secret env file.
set_session_suris() {
  local svc="$1" envfile="$KEYS_DIR/secrets/${svc}.env"
  [ -f "$envfile" ] || die "secret env missing: $envfile"
  # shellcheck disable=SC1090
  ( set -a; . "$envfile"; set +a
    printf '%s' "$BABE_SECRET"                | railway variable set BABE_SURI --stdin --service "$svc" --skip-deploys >/dev/null
    printf '%s' "$IM_ONLINE_SECRET"           | railway variable set IMON_SURI --stdin --service "$svc" --skip-deploys >/dev/null
    printf '%s' "$AUTHORITY_DISCOVERY_SECRET" | railway variable set AUDI_SURI --stdin --service "$svc" --skip-deploys >/dev/null
    printf '%s' "$GRANDPA_SECRET"             | railway variable set GRAN_SURI --stdin --service "$svc" --skip-deploys >/dev/null
  )
}

# Place a service in exactly one region (1 replica). `railway scale X=1` ADDS a
# replica on top of the default sfo one (two nodes, identical keys -> equivocation
# risk), so zero sfo unless the target IS us-west (which maps to sfo).
place_region() {
  local svc="$1" region="$2"
  if [ "$region" = "us-west" ]; then
    railway scale --service "$svc" us-west=1 >/dev/null 2>&1 || log "  WARN: scale $svc -> $region failed (set in dashboard)"
  else
    railway scale --service "$svc" sfo=0 "$region=1" >/dev/null 2>&1 || log "  WARN: scale $svc -> $region failed (set in dashboard)"
  fi
}

# --- 0. project -------------------------------------------------------------
if railway status >/dev/null 2>&1; then
  log "using linked project"
else
  log "creating project '$PROJECT_NAME'"
  railway init -n "$PROJECT_NAME" >/dev/null
fi

# --- 1. node services -------------------------------------------------------
for i in "${!SERVICES[@]}"; do
  svc="${SERVICES[$i]}"
  region="${REGIONS[$i]}"
  keyfile="$NODE_KEYS_DIR/${NODE_KEY_FILES[$i]}"
  [ -f "$keyfile" ] || die "node key file missing: $keyfile"

  log "=== $svc (region: $region) ==="

  # 1a. service from the prebuilt image
  if service_exists "$svc"; then
    log "  service exists, reusing"
  else
    log "  creating service from $IMAGE"
    railway add --service "$svc" --image "$IMAGE" >/dev/null
  fi

  # 1b. volume at /data (operates on the LINKED service)
  railway service link "$svc" >/dev/null 2>&1 || true
  if service_has_volume "$svc"; then
    log "  volume exists"
  else
    log "  adding volume at $MOUNT_PATH"
    railway volume add -m "$MOUNT_PATH" >/dev/null
  fi

  # 1c. non-secret config: shared + per-service deltas
  log "  pushing config (_common.env + ${svc}.env)"
  push_config_file "$svc" "$CONFIG_DIR/_common.env"
  push_config_file "$svc" "$CONFIG_DIR/${svc}.env"

  # 1d. secrets: p2p node key (all) + session SURIs (validators only)
  printf '%s' "$(tr -d '\n' < "$keyfile")" \
    | railway variable set NODE_KEY_HEX --stdin --service "$svc" --skip-deploys >/dev/null
  if [ "${svc#validator-}" != "$svc" ]; then
    set_session_suris "$svc"
    log "  session SURIs set"
  fi

  # 1e. region + deploy
  log "  placing in region $region (single replica)"
  place_region "$svc" "$region"
  log "  redeploying"
  railway redeploy --service "$svc" --yes >/dev/null 2>&1 || true
done

# --- 2. rpc-proxy (Caddy) ---------------------------------------------------
# Public RPC gateway: enforces *.impetus.network Origin/Referer + rate limit,
# proxies to rpc-singapore. Different image, no volume, no node/session keys.
log "=== rpc-proxy ==="
if service_exists rpc-proxy; then
  log "  service exists, reusing"
else
  log "  creating service from $PROXY_IMAGE"
  railway add --service rpc-proxy --image "$PROXY_IMAGE" >/dev/null
fi
push_config_file rpc-proxy "$CONFIG_DIR/rpc-proxy.env"
railway domain --service rpc-proxy >/dev/null 2>&1 || true   # idempotent: keeps existing domain
log "  redeploying"
railway redeploy --service rpc-proxy --yes >/dev/null 2>&1 || true

log "done. Watch:  railway logs --service validator-1"
log "Block production starts once >=4 validators load session keys and peer up."
log "Public RPC: rpc-proxy public domain -> rpc-singapore (see infra/railway/config/rpc-proxy.env)."
