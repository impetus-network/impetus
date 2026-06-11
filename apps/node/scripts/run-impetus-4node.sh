#!/usr/bin/env bash
# Launch a 4-validator impetus_dev_npos network on localhost.
#
# Node #0 (Alice) exposes RPC on 9944 for Hardhat / E2E.
# Bob/Charlie/Dave connect via libp2p and participate in Babe slot lottery
# + GRANDPA finalization. Used to reproduce production NPoS topology
# locally and avoid the single-node `force_authoring` slot drift that
# triggers `Unexpected epoch change` rollbacks.
#
# Usage:
#   apps/node/scripts/run-impetus-4node.sh        # foreground (Ctrl+C stops all)
#   apps/node/scripts/run-impetus-4node.sh start  # daemonize, write PIDs
#   apps/node/scripts/run-impetus-4node.sh stop   # kill all 4 nodes
#   apps/node/scripts/run-impetus-4node.sh status # show peers / blocks
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BIN="$ROOT/target/release/impetus-node"
CHAIN=impetus_dev_npos
BASE_DIR=/tmp/impetus-4node
PID_FILE=$BASE_DIR/pids
NODE_KEY_ALICE=0000000000000000000000000000000000000000000000000000000000000001
BOOT_PEER_ID=12D3KooWEyoppNCUx8Yx66oV9fJnriXwCcXwDDUA2kj6vnc6iDEp

stop_all() {
  if [[ -f $PID_FILE ]]; then
    while read -r pid; do
      [[ -n "$pid" ]] && kill "$pid" 2>/dev/null || true
    done < "$PID_FILE"
    rm -f "$PID_FILE"
  fi
  pkill -f "impetus-node.*$CHAIN" 2>/dev/null || true
  echo "Stopped 4-node network."
}

status() {
  for port in 9944 9945 9946 9947; do
    local who=""
    case $port in 9944) who=alice ;; 9945) who=bob ;; 9946) who=charlie ;; 9947) who=dave ;; esac
    local h=$(curl -s -m 2 -X POST -H "Content-Type: application/json" \
      --data '{"jsonrpc":"2.0","method":"chain_getHeader","params":[],"id":1}' \
      "http://127.0.0.1:$port" 2>/dev/null \
      | python3 -c "import json,sys;r=json.load(sys.stdin);print(int(r['result']['number'],16))" 2>/dev/null || echo "DOWN")
    local p=$(curl -s -m 2 -X POST -H "Content-Type: application/json" \
      --data '{"jsonrpc":"2.0","method":"system_health","params":[],"id":1}' \
      "http://127.0.0.1:$port" 2>/dev/null \
      | python3 -c "import json,sys;r=json.load(sys.stdin);print(r['result']['peers'])" 2>/dev/null || echo "?")
    echo "$who  port=$port  block=$h  peers=$p"
  done
}

case "${1:-foreground}" in
  stop) stop_all; exit 0 ;;
  status) status; exit 0 ;;
esac

stop_all
rm -rf "$BASE_DIR"
mkdir -p "$BASE_DIR"

if [[ ! -x "$BIN" ]]; then
  echo "Binary not found at $BIN. Run: cargo build --release" >&2
  exit 1
fi

start_node() {
  local idx=$1 name=$2 port=$3 rpc=$4 extra=$5
  local logfile=$BASE_DIR/n$idx.log
  # shellcheck disable=SC2086
  nohup "$BIN" --chain $CHAIN \
    --base-path $BASE_DIR/n$idx \
    --port $port --rpc-port $rpc \
    --validator $name \
    --no-mdns --no-telemetry \
    --rpc-cors all \
    $extra > "$logfile" 2>&1 &
  echo $! >> "$PID_FILE"
  echo "  $name  port=$port  rpc=$rpc  pid=$!"
}

echo "Launching 4-node impetus NPoS network..."
> "$PID_FILE"

# Alice = bootnode with pinned node-key so others can dial deterministically.
start_node 0 --alice 30333 9944 "--node-key $NODE_KEY_ALICE"

# Wait until Alice's RPC is up and resolve actual peer id (in case key->peer-id
# mapping shifts across Substrate versions).
echo "Waiting for Alice RPC..."
for _ in $(seq 1 30); do
  PEER=$(curl -s -m 2 -X POST -H "Content-Type: application/json" \
    --data '{"jsonrpc":"2.0","method":"system_localPeerId","params":[],"id":1}' \
    http://127.0.0.1:9944 2>/dev/null \
    | python3 -c "import json,sys;print(json.load(sys.stdin)['result'])" 2>/dev/null || true)
  if [[ -n "$PEER" ]]; then
    break
  fi
  sleep 1
done
if [[ -z "${PEER:-}" ]]; then
  echo "Alice RPC never came up; aborting." >&2
  stop_all
  exit 1
fi
BOOT_ADDR="/ip4/127.0.0.1/tcp/30333/p2p/$PEER"
echo "Alice peer-id: $PEER"

NODE_KEY_BOB=0000000000000000000000000000000000000000000000000000000000000002
NODE_KEY_CHARLIE=0000000000000000000000000000000000000000000000000000000000000003
NODE_KEY_DAVE=0000000000000000000000000000000000000000000000000000000000000004
start_node 1 --bob     30334 9945 "--bootnodes $BOOT_ADDR --node-key $NODE_KEY_BOB"
start_node 2 --charlie 30335 9946 "--bootnodes $BOOT_ADDR --node-key $NODE_KEY_CHARLIE"
start_node 3 --dave    30336 9947 "--bootnodes $BOOT_ADDR --node-key $NODE_KEY_DAVE"

echo "All 4 nodes launched. Logs: $BASE_DIR/n{0,1,2,3}.log"
echo
echo "Sleeping 15s for peer discovery..."
sleep 15
status

if [[ "${1:-foreground}" == "start" ]]; then
  echo
  echo "Daemonized. PIDs: $(tr '\n' ' ' < "$PID_FILE")"
  exit 0
fi

echo
echo "Press Ctrl+C to stop all nodes."
trap stop_all EXIT INT TERM
wait
