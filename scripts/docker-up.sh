#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$SCRIPT_DIR/.."

echo "Starting 2-node Artemis testnet..."
docker compose -f "$REPO_ROOT/docker-compose.yml" up -d

echo ""
echo "  Alice RPC: http://localhost:9944"
echo "  Bob RPC:   http://localhost:9945"
echo ""
echo "Logs: docker compose logs -f"
