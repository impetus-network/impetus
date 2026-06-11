#!/usr/bin/env bash
#
# gen-node-keys.sh — generate libp2p node keys (one per node) for an Impetus
# network. A node key is the node's p2p identity; it derives the PeerID used in
# bootnode multiaddrs. This is SEPARATE from validator session keys.
#
# RUN THIS YOURSELF. Secret key files go under ./launch-keys/node-keys/ (chmod
# 600, gitignored). PeerIDs are public and safe to share / put in bootnode
# multiaddrs.
#
# Produces, under ./launch-keys/node-keys/ :
#   node-N.key            secret node key (hex, 32 bytes) — load on node N
#   peer-ids.txt          PeerID per node (public)
#   bootnodes.template    multiaddr template — fill in host:port after deploy
#
# Usage:
#   ./scripts/gen-node-keys.sh [NODE_COUNT]        # default 6 (5 validators + 1 archive)
#   LABELS="validator-1 validator-2 ... archive" ./scripts/gen-node-keys.sh
#
set -euo pipefail

COUNT="${1:-6}"
NODE="${NODE:-./target/release/impetus-node}"
OUT="${OUT:-./launch-keys/node-keys}"
P2P_PORT="${P2P_PORT:-30333}"

# Optional human labels (space-separated). Defaults to validator-1..N-1 + archive.
if [ -n "${LABELS:-}" ]; then
  # shellcheck disable=SC2206
  LABEL_ARR=($LABELS)
else
  LABEL_ARR=()
  vcount=$((COUNT - 1))
  for i in $(seq 1 "$vcount"); do LABEL_ARR+=("validator-$i"); done
  LABEL_ARR+=("archive")
fi

[ -x "$NODE" ] || { echo "ERROR: node binary not found/executable at $NODE"; exit 1; }
if [ "${#LABEL_ARR[@]}" -ne "$COUNT" ]; then
  echo "ERROR: LABELS has ${#LABEL_ARR[@]} entries but NODE_COUNT=$COUNT"; exit 1
fi
if [ -e "$OUT" ]; then
  echo "ERROR: $OUT already exists — refusing to overwrite node keys. Move it aside first."
  exit 1
fi
mkdir -p "$OUT"; chmod 700 "$OUT"
umask 077

echo "Generating $COUNT node keys into $OUT/ ..."

PEERS_FILE="$OUT/peer-ids.txt"
: > "$PEERS_FILE"
declare -a PEERIDS=()

for idx in $(seq 0 $((COUNT - 1))); do
  label="${LABEL_ARR[$idx]}"
  keyfile="$OUT/node-$((idx + 1)).key"
  # generate-node-key writes the secret to --file and the PeerID to stderr.
  peerid="$("$NODE" key generate-node-key --file "$keyfile" 2>&1 >/dev/null)"
  chmod 600 "$keyfile"
  # sanity: re-derive PeerID from the file and confirm it matches.
  rederived="$("$NODE" key inspect-node-key --file "$keyfile" 2>/dev/null)"
  if [ "$peerid" != "$rederived" ]; then
    echo "ERROR: PeerID mismatch for $label ($peerid vs $rederived)"; exit 1
  fi
  PEERIDS+=("$peerid")
  printf '%-14s node-%s.key  %s\n' "$label" "$((idx + 1))" "$peerid" >> "$PEERS_FILE"
  echo "  - $label -> $peerid"
done

# Bootnode multiaddr template — operator fills in the real host per node after
# deploy. Two forms: public DNS (TCP proxy) and Railway internal networking.
{
  echo "# Bootnode multiaddrs. Fill in <host> with each node's reachable address,"
  echo "# then pass 2-3 of these to every node via --bootnodes (space-separated),"
  echo "# OR paste them into the chain spec's bootNodes array."
  echo "#"
  echo "# Public form (Railway TCP proxy / public host):"
  for idx in $(seq 0 $((COUNT - 1))); do
    echo "#   /dns4/<host-${LABEL_ARR[$idx]}>/tcp/$P2P_PORT/p2p/${PEERIDS[$idx]}"
  done
  echo "#"
  echo "# Railway internal form (same project, private networking):"
  for idx in $(seq 0 $((COUNT - 1))); do
    echo "#   /dns4/<service>.railway.internal/tcp/$P2P_PORT/p2p/${PEERIDS[$idx]}"
  done
} > "$OUT/bootnodes.template"
chmod 600 "$OUT/bootnodes.template"

echo
echo "Done. Wrote $OUT/"
echo "  node-N.key       -> secret; load on the matching node:"
echo "                      impetus-node ... --node-key-file node-N.key"
echo "  peer-ids.txt     -> public PeerIDs"
echo "  bootnodes.template -> fill in hosts after deploy, use 2-3 as --bootnodes"
echo
echo "SECURITY: node-N.key files are secret (a stolen node key lets someone"
echo "          impersonate that node's p2p identity). Keep them on a persistent"
echo "          volume; never commit. launch-keys/ is gitignored."
