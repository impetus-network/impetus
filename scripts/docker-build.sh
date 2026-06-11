#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$SCRIPT_DIR/.."

echo "Building Artemis node Docker image..."
docker compose -f "$REPO_ROOT/docker-compose.yml" build

echo "Done."
