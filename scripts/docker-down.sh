#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$SCRIPT_DIR/.."

PURGE=false
for arg in "$@"; do
  case $arg in
    --purge) PURGE=true ;;
  esac
done

echo "Stopping Artemis testnet..."
docker compose -f "$REPO_ROOT/docker-compose.yml" down

if [ "$PURGE" = true ]; then
  echo "Purging chain data volumes..."
  docker compose -f "$REPO_ROOT/docker-compose.yml" down -v
fi

echo "Done."
