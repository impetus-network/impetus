# Chain specs

The committed `impetus.json` was **removed** (C-5 in `docs/launch-checklist.md`):
the old file was stale (pre-NPoS, dated 2026-05-10), embedded the public
`//Alice..//Dave` BABE/GRANDPA dev keys, funded the public Hardhat dev
accounts, and shipped empty `bootNodes` / no `protocolId` / no telemetry.
Launching from it would have produced a chain anyone could take over.

A production raw spec must be regenerated, per-deployment, from fresh keys.
**Do not commit a spec built with `IMPETUS_ALLOW_PLACEHOLDER_KEYS=1`.**

## Generate the production raw spec

```bash
# 1. Each operator generates session keys offline:
#      subkey generate --scheme sr25519   # babe, im_online, authority_discovery
#      subkey generate --scheme ed25519   # grandpa
#    and submits {stash, babe, grandpa, im_online, authority_discovery} pubkeys.
#
# 2. Coordinator assembles validators.json (one entry per validator):
#      [{"stash":"0x..","babe":"0x..","grandpa":"0x..","im_online":"0x..","authority_discovery":"0x.."}, ...]
#
# 3. Generate a fresh sudo key and build the spec:
cast wallet new-mnemonic --words 24 --accounts 1   # -> IMPETUS_SUDO_ADDRESS

IMPETUS_SUDO_ADDRESS=0x<fresh-sudo-h160> \
IMPETUS_VALIDATOR_KEYS_FILE=validators.json \
IMPETUS_MIN_VALIDATOR_COUNT=3 \
  ./target/release/impetus-node build-spec \
    --chain impetus --raw --disable-default-bootnode \
    > chain-specs/impetus.json

# 4. Edit chain-specs/impetus.json to add real bootNodes, telemetryEndpoints,
#    and a unique protocolId, then distribute the byte-identical file to every
#    operator.
```

`IMPETUS_MIN_VALIDATOR_COUNT` sets `staking.minimumValidatorCount` — the floor
below which the chain refuses to elect a validator group and halts. It defaults
to the full validator set; lower it (clamped to `1..=set-size`) so the network
tolerates some validators being offline (e.g. a hosting-provider redeploy)
without halting. For a 5-validator genesis, `3` tolerates 2 validators down.
Keep the active set above 2/3 honest for BFT finality.

The builder refuses to run without `IMPETUS_SUDO_ADDRESS` + a keys file (or the
explicit `IMPETUS_ALLOW_PLACEHOLDER_KEYS=1` rehearsal opt-in), and rejects the
burned dev sudo address `0xd2aE0A2139dC83Cb920e3cd7B9F640922D14b872`.
See `node/src/chain_spec.rs` and `docs/launch-checklist.md`.
