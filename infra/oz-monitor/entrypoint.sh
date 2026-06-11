#!/usr/bin/env bash
# Render config templates with values from the container environment, then
# exec the openzeppelin-monitor binary. Re-runs on every container start, so
# updating .env + restarting the container is enough to pick up new addresses.
set -euo pipefail

SRC_DIR="/app/config-template"
DST_DIR="/app/config"
VARS=(VRF_RESULT_SOURCE_ADDRESS CROSS_CHAIN_RECEIVER_ADDRESS)

mkdir -p "$DST_DIR"
# Wipe stale rendered files first so removed templates don't linger.
find "$DST_DIR" -mindepth 1 -delete
cp -r "$SRC_DIR"/. "$DST_DIR"/

while IFS= read -r -d '' file; do
  for var in "${VARS[@]}"; do
    val="${!var:-}"
    if [ -z "$val" ]; then
      echo "[entrypoint] WARNING: ${var} is empty; placeholder will remain in $file" >&2
      continue
    fi
    # Use | as sed delimiter so 0x-prefixed addresses don't conflict with /
    sed -i "s|\${${var}}|${val}|g" "$file"
  done
done < <(find "$DST_DIR" -type f -name '*.json' -print0)

exec /app/openzeppelin-monitor "$@"
