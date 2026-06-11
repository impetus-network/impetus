#!/usr/bin/env bash
#
# gen-launch-keys.sh — generate ALL keys for an Impetus launch on ONE machine.
#
# RUN THIS YOURSELF, OFFLINE. It writes secrets to ./launch-keys/secrets/ (chmod
# 600). NEVER commit that directory, and move it into a password manager /
# Infisical afterwards. Anyone who reads a secret phrase controls that key.
#
# Produces, under ./launch-keys/ :
#   sudo.txt                 sudo (admin) mnemonic + address + private key
#   validators.json          PUBLIC keys only — safe to share, feeds build-spec
#   secrets/validator-N.env  per-validator secret phrases (chmod 600)
#   insert-validator-N.sh    per-validator keystore-insert script (run on its node)
#   summary.txt              addresses / pubkeys overview (no secrets)
#
# Usage:
#   ./scripts/gen-launch-keys.sh [VALIDATOR_COUNT]   # default 5
#
set -euo pipefail

# ---- config ----------------------------------------------------------------
COUNT="${1:-5}"
NODE="${NODE:-./target/release/impetus-node}"
OUT="${OUT:-./launch-keys}"
CHAIN_SPEC="${CHAIN_SPEC:-./chain-specs/impetus.json}"  # used only by insert scripts
BASE_PATH="${BASE_PATH:-/data}"                         # node --base-path on each validator

# ---- preflight -------------------------------------------------------------
command -v cast >/dev/null || { echo "ERROR: 'cast' (foundry) not found"; exit 1; }
command -v python3 >/dev/null || { echo "ERROR: python3 not found"; exit 1; }
[ -x "$NODE" ] || { echo "ERROR: node binary not found/executable at $NODE (build it first)"; exit 1; }

if [ -e "$OUT" ]; then
  echo "ERROR: $OUT already exists — refusing to overwrite keys. Move it aside first."
  exit 1
fi
mkdir -p "$OUT/secrets"
chmod 700 "$OUT" "$OUT/secrets"
umask 077

echo "Generating launch keys for $COUNT validators into $OUT/ ..."

# ---- helpers ---------------------------------------------------------------
# Extract a JSON field from a node key-generate blob via python (no jq dependency).
jqf() { python3 -c "import json,sys; print(json.load(sys.stdin)['$1'])"; }

# Generate one session key of a given scheme; echoes "PUBLIC<TAB>SECRETPHRASE".
gen_key() {
  local scheme="$1" blob pub sec
  blob="$($NODE key generate --scheme "$scheme" --output-type json)"
  # The genesis builder deserializes session keys as `sp_core::*::Public`, whose
  # serde impl expects an SS58 string — NOT a 0x-hex public key. Use the SS58
  # form here (the stash stays 0x; it is an H160 parsed from hex).
  pub="$(printf '%s' "$blob" | jqf ss58PublicKey)"
  sec="$(printf '%s' "$blob" | jqf secretPhrase)"
  # Trailing newline is required: `read` returns non-zero at EOF without it,
  # which trips `set -e`.
  printf '%s\t%s\n' "$pub" "$sec"
}

# ---- sudo / admin key ------------------------------------------------------
echo "  - sudo / admin key"
SUDO_JSON="$(cast wallet new-mnemonic --json --words 24 --accounts 1)"
SUDO_ADDR="$(printf '%s' "$SUDO_JSON" | python3 -c "import json,sys; print(json.load(sys.stdin)['accounts'][0]['address'])")"
SUDO_PK="$(printf '%s'   "$SUDO_JSON" | python3 -c "import json,sys; print(json.load(sys.stdin)['accounts'][0]['private_key'])")"
SUDO_MN="$(printf '%s'   "$SUDO_JSON" | python3 -c "import json,sys; print(json.load(sys.stdin)['mnemonic'])")"
{
  echo "# Impetus sudo / admin account #0 — KEEP SECRET. Store in Infisical, then delete this file."
  echo "ADMIN_MNEMONIC=\"$SUDO_MN\""
  echo "ADMIN_ADDRESS=$SUDO_ADDR"
  echo "ADMIN_PRIVATE_KEY=$SUDO_PK"
  echo "# Use this when building the chain spec:"
  echo "# IMPETUS_SUDO_ADDRESS=$SUDO_ADDR"
} > "$OUT/sudo.txt"
chmod 600 "$OUT/sudo.txt"

# ---- per-validator keys ----------------------------------------------------
ENTRIES=()       # JSON objects for validators.json
SUMMARY=()       # human-readable, no secrets

for i in $(seq 1 "$COUNT"); do
  echo "  - validator $i / $COUNT"

  # Stash = EVM account (receives reward + bond).
  V_JSON="$(cast wallet new-mnemonic --json --words 24 --accounts 1)"
  STASH="$(printf '%s' "$V_JSON" | python3 -c "import json,sys; print(json.load(sys.stdin)['accounts'][0]['address'])")"
  STASH_PK="$(printf '%s' "$V_JSON" | python3 -c "import json,sys; print(json.load(sys.stdin)['accounts'][0]['private_key'])")"
  STASH_MN="$(printf '%s' "$V_JSON" | python3 -c "import json,sys; print(json.load(sys.stdin)['mnemonic'])")"

  # 4 session keys: babe, im_online, authority_discovery (sr25519); grandpa (ed25519).
  IFS=$'\t' read -r BABE_PUB   BABE_SEC   < <(gen_key sr25519)
  IFS=$'\t' read -r IMON_PUB   IMON_SEC   < <(gen_key sr25519)
  IFS=$'\t' read -r AUDI_PUB   AUDI_SEC   < <(gen_key sr25519)
  IFS=$'\t' read -r GRAN_PUB   GRAN_SEC   < <(gen_key ed25519)

  # validators.json entry — PUBLIC keys only. Pass values via argv (never
  # interpolate them into the python source) so odd characters can't break it.
  ENTRIES+=("$(python3 -c "import json,sys; print(json.dumps(dict(zip(['stash','babe','grandpa','im_online','authority_discovery'], sys.argv[1:]))))" "$STASH" "$BABE_PUB" "$GRAN_PUB" "$IMON_PUB" "$AUDI_PUB")")

  # Per-validator secret bundle (chmod 600).
  SECF="$OUT/secrets/validator-$i.env"
  {
    echo "# Validator $i — SECRET. Move to that validator's host securely, then delete here."
    echo "STASH_ADDRESS=$STASH"
    echo "STASH_PRIVATE_KEY=$STASH_PK"
    echo "STASH_MNEMONIC=\"$STASH_MN\""
    echo "BABE_SECRET=\"$BABE_SEC\""
    echo "IM_ONLINE_SECRET=\"$IMON_SEC\""
    echo "AUTHORITY_DISCOVERY_SECRET=\"$AUDI_SEC\""
    echo "GRANDPA_SECRET=\"$GRAN_SEC\""
  } > "$SECF"
  chmod 600 "$SECF"

  # Per-validator keystore-insert script — run on that validator's node BEFORE block #1.
  INSF="$OUT/insert-validator-$i.sh"
  cat > "$INSF" <<EOS
#!/usr/bin/env bash
# Insert validator $i session keys into the node keystore. Run ON validator $i's host.
# Requires the secret bundle (secrets/validator-$i.env) next to it.
set -euo pipefail
NODE="\${NODE:-./impetus-node}"
BASE="\${BASE_PATH:-$BASE_PATH}"
CHAIN="\${CHAIN_SPEC:-$CHAIN_SPEC}"
source "\$(dirname "\$0")/secrets/validator-$i.env"
"\$NODE" key insert --base-path "\$BASE" --chain "\$CHAIN" --scheme sr25519 --key-type babe --suri "\$BABE_SECRET"
"\$NODE" key insert --base-path "\$BASE" --chain "\$CHAIN" --scheme sr25519 --key-type imon --suri "\$IM_ONLINE_SECRET"
"\$NODE" key insert --base-path "\$BASE" --chain "\$CHAIN" --scheme sr25519 --key-type audi --suri "\$AUTHORITY_DISCOVERY_SECRET"
"\$NODE" key insert --base-path "\$BASE" --chain "\$CHAIN" --scheme ed25519 --key-type gran --suri "\$GRANDPA_SECRET"
echo "validator $i keys inserted into \$BASE keystore"
EOS
  chmod 700 "$INSF"

  SUMMARY+=("validator $i: stash=$STASH babe=$BABE_PUB grandpa=$GRAN_PUB im_online=$IMON_PUB authority_discovery=$AUDI_PUB")
done

# ---- assemble validators.json (public, safe to share) ----------------------
python3 - "$OUT/validators.json" "${ENTRIES[@]}" <<'PY'
import json, sys
out_path = sys.argv[1]
entries = [json.loads(e) for e in sys.argv[2:]]
with open(out_path, "w") as f:
    json.dump(entries, f, indent=2)
PY

# ---- summary (no secrets) --------------------------------------------------
{
  echo "Impetus launch keys — generated $(date -u +%Y-%m-%dT%H:%M:%SZ)"
  echo "validators: $COUNT"
  echo "sudo address: $SUDO_ADDR   (secret in sudo.txt)"
  echo
  printf '%s\n' "${SUMMARY[@]}"
} > "$OUT/summary.txt"
chmod 600 "$OUT/summary.txt"

echo
echo "Done. Wrote $OUT/"
echo "  validators.json        -> feed to IMPETUS_VALIDATOR_KEYS_FILE (public, safe)"
echo "  sudo.txt               -> IMPETUS_SUDO_ADDRESS=$SUDO_ADDR (secret inside)"
echo "  secrets/validator-N.env + insert-validator-N.sh -> move to each validator host"
echo
echo "Next:"
echo "  IMPETUS_SUDO_ADDRESS=$SUDO_ADDR \\"
echo "  IMPETUS_VALIDATOR_KEYS_FILE=$OUT/validators.json \\"
echo "  IMPETUS_MIN_VALIDATOR_COUNT=3 \\"
echo "    $NODE build-spec --chain impetus --raw --disable-default-bootnode > chain-specs/impetus.json"
echo
echo "SECURITY: move launch-keys/ secrets into Infisical/a vault, then shred the local copies."
echo "          launch-keys/ is gitignored by this script's sibling .gitignore entry — verify it."
