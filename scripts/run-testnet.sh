#!/usr/bin/env bash
set -euo pipefail

# Run Impulse local testnet with 2 validators (Alice + Bob)
# Chain ID: 322644 | Uses persistent data directories
#
# Usage:
#   ./scripts/run-testnet.sh              # Start validator Alice (default)
#   ./scripts/run-testnet.sh --bob        # Start validator Bob
#   ./scripts/run-testnet.sh --purge      # Purge chain data and restart

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
NODE_DIR="$SCRIPT_DIR/../apps/node"
NODE_BIN="$NODE_DIR/target/release/impetus-node"
DATA_DIR="$SCRIPT_DIR/../.chain-data"

export PATH="$HOME/.cargo/bin:$PATH"

if [ ! -f "$NODE_BIN" ]; then
  echo "Node binary not found. Building..."
  cd "$NODE_DIR" && cargo build --release
fi

VALIDATOR="alice"
PURGE=false
RPC_PORT=9944
P2P_PORT=30333

for arg in "$@"; do
  case $arg in
    --bob)
      VALIDATOR="bob"
      RPC_PORT=9945
      P2P_PORT=30334
      ;;
    --purge)
      PURGE=true
      ;;
  esac
done

CHAIN_DATA="$DATA_DIR/$VALIDATOR"

if [ "$PURGE" = true ] && [ -d "$CHAIN_DATA" ]; then
  echo "Purging chain data for $VALIDATOR..."
  rm -rf "$CHAIN_DATA"
fi

mkdir -p "$CHAIN_DATA"

echo "Starting Impulse testnet - Validator: $VALIDATOR"
echo "  Chain ID:  322644"
echo "  RPC:       http://127.0.0.1:$RPC_PORT"
echo "  P2P:       $P2P_PORT"
echo "  Data:      $CHAIN_DATA"
echo ""

COMMON_ARGS=(
  --chain=impulse
  --base-path="$CHAIN_DATA"
  --rpc-port="$RPC_PORT"
  --port="$P2P_PORT"
  --rpc-cors=all
  --rpc-methods=unsafe
  --validator
)

if [ "$VALIDATOR" = "alice" ]; then
  exec "$NODE_BIN" "${COMMON_ARGS[@]}" --alice "$@"
elif [ "$VALIDATOR" = "bob" ]; then
  BOOTNODE=$("$NODE_BIN" key inspect-node-key --file "$DATA_DIR/alice/chains/impulse/network/secret_ed25519" 2>/dev/null || echo "")
  EXTRA_ARGS=()
  if [ -n "$BOOTNODE" ]; then
    EXTRA_ARGS+=(--bootnodes "/ip4/127.0.0.1/tcp/30333/p2p/$BOOTNODE")
  fi
  exec "$NODE_BIN" "${COMMON_ARGS[@]}" --bob "${EXTRA_ARGS[@]}"
fi
