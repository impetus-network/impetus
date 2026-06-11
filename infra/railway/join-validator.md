# Join Impetus mainnet as an external Docker validator

How a new operator (your friend) runs a Docker validator and joins the live
**Impetus** canary, and what **you** (the chain operator, holder of the sudo
key) must do to seat them. This is the first decentralization step — until now
all nodes are run by one party.

Everything on-chain is done from an **EVM account via precompiles** (MetaMask /
ethers) — this is an `AccountId20` (H160) chain, no polkadot.js required.

| Thing | Value |
|-------|-------|
| Chain id | `388266` · token `IPT` · 18 decimals |
| Genesis hash | `0x1e79733152721d2fdf888a06cd25f01cd1f04b316215a96b1083fb2f7e63f885` |
| Raw spec sha256 | `08e9163d102c62a1ba3ceb563e7885aa6a36f36342c232da2ee878af7a420377` |
| Node image | `tranhuyduan/impetus-railway:v0.3.0` (spec baked at `/opt/impetus.json`) |
| Staking precompile | `0x0000000000000000000000000000000000000810` |
| Session precompile | `0x0000000000000000000000000000000000000818` |
| StakingAdmin (sudo) | `0x0000000000000000000000000000000000000840` |

Public p2p bootnodes (validator-1 + validator-2, via Railway TCP proxy):

```
/dns4/interchange.proxy.rlwy.net/tcp/46484/p2p/12D3KooWPH4Dp9n9csjEgWy3MWsnrRfEy1ewCYkahG59uNt5S3q9
/dns4/acela.proxy.rlwy.net/tcp/28949/p2p/12D3KooWDLzWZxgqwtab7mH2YLb7NWEgpNdkumHmJRySnmEgBjEL
```

---

## OPERATOR (you) — prerequisites

1. **Share** with your friend: the two bootnodes above and the genesis hash /
   spec sha256 so they can verify they joined the right chain. The image bakes
   the byte-identical spec, so they do not need the file itself.
2. **Fund their stash** (their EVM address) with enough IPT to bond above
   `IStaking.minValidatorBond()` plus gas. The genesis launch validators bonded
   2,000 IPT; send comfortably above the minimum.
3. Be ready to **seat them** (Step D) — a new validator only enters the active
   set if `validatorCount` has a free slot and their stake qualifies.

> Trust note: a new validator gains finality weight. BFT finality needs >2/3 of
> the active set honest. One external validator among five is safe; keep the set
> >2/3 honest as you add more.

---

## FRIEND — Part A: run the Docker validator

Generate a 24-word mnemonic offline (`cast wallet new-mnemonic --words 24` or
`impetus-node key generate`). Used both as the session-key seed and to derive the
on-chain keys in Part B. Keep it secret — it controls the validator.

```bash
MN="your twenty four word mnemonic ......"
NODE_KEY=$(openssl rand -hex 32)        # stable p2p identity

docker volume create impetus-data
docker run -d --name impetus-validator --restart unless-stopped \
  -v impetus-data:/data \
  -p 30333:30333 \
  -p 127.0.0.1:9944:9944 \
  -e ROLE=validator \
  -e CHAIN_SPEC_PATH=/opt/impetus.json \
  -e NODE_NAME=friend-validator \
  -e NODE_KEY_HEX="$NODE_KEY" \
  -e BOOTNODES="/dns4/interchange.proxy.rlwy.net/tcp/46484/p2p/12D3KooWPH4Dp9n9csjEgWy3MWsnrRfEy1ewCYkahG59uNt5S3q9 /dns4/acela.proxy.rlwy.net/tcp/28949/p2p/12D3KooWDLzWZxgqwtab7mH2YLb7NWEgpNdkumHmJRySnmEgBjEL" \
  -e BABE_SURI="$MN" -e IMON_SURI="$MN" -e AUDI_SURI="$MN" -e GRAN_SURI="$MN" \
  tranhuyduan/impetus-railway:v0.3.0

docker logs -f impetus-validator   # watch it sync to the chain tip
```

The entrypoint inserts the 4 session keys (babe/im_online/authority_discovery =
sr25519, grandpa = ed25519, all from `$MN`) into the keystore on the volume, then
runs `--validator`. The node syncs but will NOT author until Part B + C + D.

> RPC is bound to `127.0.0.1:9944` only (not public). Point MetaMask / ethers at
> `http://127.0.0.1:9944` (chain id `388266`) for the on-chain steps — the public
> RPC proxy rejects non-`*.impetus.network` origins, so use your own node.

The repo's `apps/node/scripts/run-node.sh` automates Part A; for mainnet pass the
raw spec: `RUNTIME=docker ROLE=validator SPEC=/path/to/impetus.json ... run-node.sh`.

## FRIEND — Part B: register session keys (`setKeys`)

Derive the public half of the session keys from `$MN` (the `keys` blob is the
SCALE-encoded `SessionKeys` tuple, in order **babe, grandpa, im_online,
authority_discovery**):

```bash
img=tranhuyduan/impetus-railway:v0.3.0
SR=$(docker run --rm -e MN="$MN" --entrypoint sh $img -c \
  'impetus-node key inspect --scheme sr25519 --output-type json "$MN"' | python3 -c 'import json,sys;print(json.load(sys.stdin)["publicKey"])')
ED=$(docker run --rm -e MN="$MN" --entrypoint sh $img -c \
  'impetus-node key inspect --scheme ed25519 --output-type json "$MN"' | python3 -c 'import json,sys;print(json.load(sys.stdin)["publicKey"])')
# babe || grandpa || im_online || authority_discovery  (single mnemonic -> sr key repeats)
KEYS="0x${SR#0x}${ED#0x}${SR#0x}${SR#0x}"
echo "$KEYS"   # 0x + 128 bytes (4 x 32)
```

From your stash account, call the Session precompile (proof is empty for
pallet-session):

```js
// ethers, signer = your funded stash, provider = http://127.0.0.1:9944
const session = new ethers.Contract("0x0000000000000000000000000000000000000818", ISessionAbi, signer);
await (await session.setKeys(KEYS, "0x")).wait();
```

## FRIEND — Part C: bond + declare intent to validate

```js
const staking = new ethers.Contract("0x0000000000000000000000000000000000000810", IStakingAbi, signer);
const min = await staking.minValidatorBond();
const value = min * 2n;                                  // bond comfortably above the floor
await (await staking.bond(value, { kind: 0, account: ethers.ZeroAddress })).wait();   // kind 0 = Staked
await (await staking.validate({ commissionPercent: 1, blocked: false })).wait();
```

`IStaking.sol` / `ISession.sol` ABIs live in `packages/contracts/contracts/interfaces/`.
Order matters: `bond` before `validate`; `setKeys` any time before the era turns.

## OPERATOR (you) — Part D: seat the validator (sudo)

A new validator only becomes active if there is a free slot and the next era
elects them. From the **sudo** EOA (the only caller the StakingAdmin precompile
accepts), via MetaMask / ethers at your own node:

```js
const admin = new ethers.Contract("0x0000000000000000000000000000000000000840", IStakingAdminAbi, sudoSigner);
await (await admin.increaseValidatorCount(1)).wait();   // 5 -> 6 (or setValidatorCount(6))
await (await admin.forceNewEra()).wait();               // elect now instead of waiting for the era to turn
```

## Verify

- `docker logs impetus-validator` shows it syncing then `🎁 Prepared block for
  proposing` / authoring once active.
- `IStaking.validators(stash)` returns their prefs; after `forceNewEra`,
  `session_validators` (or the next era) includes the stash.
- The node's `system_health` peers ≥ 2 (it dialed both bootnodes).

## Safety notes

- **Never run two copies of the same keys** (e.g. the old container plus a new
  one) — double-signing is equivocation → disable + slash. One validator process
  per key, always.
- Exposing validator-1/-2 p2p publicly adds DoS surface; acceptable for a canary.
  To make them advertise their public address to the DHT, add
  `EXTRA_ARGS="--public-addr /dns4/<proxy-host>/tcp/<proxy-port>"` to their
  config and redeploy — but that is a validator restart (equivocation risk), so
  do it deliberately, one at a time, never both at once.
- The friend's `/data` volume holds the keystore + p2p key; a wipe loses the
  validator identity. Back up the mnemonic offline.
