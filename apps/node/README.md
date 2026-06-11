# Impetus / Impulse Node (Frontier Solochain)

Substrate-based blockchain node with EVM compatibility via Frontier and an admin-managed gasless registry for selected EVM function selectors. The same node binary embeds two runtime WASM blobs and dispatches by chain spec id.

## Build

Requires Rust with the `wasm32-unknown-unknown` target.

```bash
cd apps/node
cargo build --release
```

The binary is produced at `target/release/impetus-node`.

## Run

| `--chain` argument | Network | Token | Chain id | spec_name | Notes |
|---|---|---|---|---|---|
| `impetus`, `mainnet` | Impetus | IPT | 388266 | `impetus` | Production mainnet |
| `impulse`, `testnet`, `""` | Impulse | IPL | 322644 | `impulse` | Testnet (default) |
| `dev` | Impulse Dev | IPL | 322644 | `impulse` | Pre-funded Hardhat dev users; manual seal when `--sealing` is set |
| `<path-to-spec.json>` | from file | — | from file | from file | Load raw chain-spec |

The mainnet raw chain-spec is intentionally NOT committed; generate it
per-deployment from fresh keys (see `chain-specs/README.md`).

```bash
# Dev (instant blocks via --sealing)
./target/release/impetus-node --chain dev --sealing manual --tmp --alice

# Testnet (Aura + Grandpa)
./target/release/impetus-node --chain impulse --tmp --validator

# Mainnet from a freshly generated raw spec (see chain-specs/README.md)
./target/release/impetus-node \
    --chain ./chain-specs/impetus.json --validator
```

RPC defaults to `http://127.0.0.1:9944`. Decimals: 18 across all networks.

## Runtime pallets

- `pallet-gasless-registry` -- admin-managed gasless EVM selector registry
- `pallet-evm` + `pallet-ethereum` -- Frontier EVM compatibility
- `pallet-assets` -- fungible token support

## Crate layout

```
apps/node/
├── node/                Substrate node binary (1 binary, 2 WASM blobs)
├── runtimes/
│   ├── common/          Shared type aliases, helpers, generic gasless module, precompiles, genesis helpers
│   ├── impetus/         Mainnet runtime (spec_name=impetus, SS58=11434)
│   └── impulse/         Testnet runtime (spec_name=impulse, SS58=11348)
├── pallets/             Custom FRAME pallets
├── precompiles/         EVM precompiles
└── chain-specs/         Committed raw chain specs (mainnet only)
```

## Generating a fresh raw chain-spec

The production builder requires `IMPETUS_SUDO_ADDRESS` (a freshly generated
sudo H160) and `IMPETUS_VALIDATOR_KEYS_FILE` (operator session keys); it rejects
the burned dev sudo and refuses placeholder keys unless
`IMPETUS_ALLOW_PLACEHOLDER_KEYS=1` is set (rehearsal only). Full steps:
`chain-specs/README.md`.

```bash
IMPETUS_SUDO_ADDRESS=0x<fresh-sudo-h160> \
IMPETUS_VALIDATOR_KEYS_FILE=validators.json \
  ./target/release/impetus-node build-spec \
    --chain impetus --raw --disable-default-bootnode > chain-specs/impetus.json
```
