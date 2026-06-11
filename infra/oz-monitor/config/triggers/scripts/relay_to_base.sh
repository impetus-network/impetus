#!/usr/bin/env bash
# Trigger script invoked by OZ Monitor when an Artemis ResultRequested event
# fires. Encodes `requestResult(uint256)` calldata and asks the OZ Relayer to
# submit the transaction on Base Sepolia using the base_signer relayer.
#
# Stdin: { "monitor_match": { ... }, "args": [...] }
# Required env: RELAYER_API_URL, RELAYER_API_KEY, VRF_RESULT_SOURCE_ADDRESS
# Exit: 0 on success, 1 on failure (Monitor will log + retry per its policy)

set -euo pipefail

PAYLOAD=$(cat)

require_env() {
  if [ -z "${!1:-}" ]; then
    echo "[relay_to_base] missing required env: $1" >&2
    exit 1
  fi
}

require_env RELAYER_API_URL
require_env RELAYER_API_KEY
require_env VRF_RESULT_SOURCE_ADDRESS

ROUND_ID=$(echo "$PAYLOAD" | jq -r '.. | .roundId? // empty' | head -n 1)
if [ -z "$ROUND_ID" ] || [ "$ROUND_ID" = "null" ]; then
  echo "[relay_to_base] could not extract roundId from monitor_match" >&2
  echo "$PAYLOAD" | jq . >&2
  exit 1
fi

# requestResult(uint256) selector + 32-byte big-endian roundId
SELECTOR="d2fb1e1a"
ROUND_HEX=$(printf "%064x" "$ROUND_ID")
DATA="0x${SELECTOR}${ROUND_HEX}"

BODY=$(jq -nc \
  --arg to "$VRF_RESULT_SOURCE_ADDRESS" \
  --arg data "$DATA" \
  '{to:$to, data:$data, speed:"fast"}')

RESPONSE=$(curl -fsS -X POST \
  "${RELAYER_API_URL%/}/api/v1/relayers/base_signer/transactions" \
  -H "Authorization: Bearer ${RELAYER_API_KEY}" \
  -H "Content-Type: application/json" \
  -d "$BODY") || {
    echo "[relay_to_base] relayer POST failed for roundId=$ROUND_ID" >&2
    exit 1
  }

echo "[relay_to_base] roundId=$ROUND_ID submitted: $(echo "$RESPONSE" | jq -c .)"
