#!/usr/bin/env bash
set -euo pipefail

# Run Impulse dev node (single validator, instant seal)
# Chain ID: 322644 | RPC: http://127.0.0.1:9944
#
# Sudo / admin: 0xd2aE0A2139dC83Cb920e3cd7B9F640922D14b872 (account #0 of ADMIN_MNEMONIC)
# Dev users:    derived from Hardhat mnemonic
#               "test test test test test test test test test test test junk"
# Admin and dev users are each pre-funded with 1,000,000 IPL at genesis.

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
NODE_DIR="$SCRIPT_DIR/../apps/node"
NODE_BIN="$NODE_DIR/target/release/impetus-node"

# Ensure rustup toolchain is used
export PATH="$HOME/.cargo/bin:$PATH"

# Build if binary not found
if [ ! -f "$NODE_BIN" ]; then
  echo "Node binary not found. Building..."
  cd "$NODE_DIR" && cargo build --release
fi

echo "Starting Impulse dev node..."
echo "  Chain ID:  322644"
echo "  RPC:       http://127.0.0.1:9944"
echo "  WS:        ws://127.0.0.1:9944"
echo "  Admin:     0xd2aE0A2139dC83Cb920e3cd7B9F640922D14b872"
echo ""

exec "$NODE_BIN" \
  --dev \
  --sealing=instant \
  --rpc-cors=all \
  --rpc-external \
  --rpc-methods=unsafe \
  --tmp \
  "$@"
