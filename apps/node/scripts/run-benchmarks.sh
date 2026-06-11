#!/usr/bin/env bash
# Generate per-pallet WeightInfo files for the impetus runtime.
#
# Substrate's reference hardware is a recent x86_64 Linux box with NVMe
# storage. Running this script on macOS, in a VM, or on a shared host
# yields unreliable timings that should NOT be committed as production
# weights. CI invokes this on ubuntu-latest; locally it is useful only to
# validate the pipeline.
#
# Usage:
#   apps/node/scripts/run-benchmarks.sh                      # all NPoS pallets
#   apps/node/scripts/run-benchmarks.sh pallet_staking       # single pallet
#   STEPS=2 REPEAT=1 apps/node/scripts/run-benchmarks.sh     # smoke run
#
# Env overrides:
#   STEPS      Number of steps per benchmark (default: 50, production).
#   REPEAT     Repetitions per step (default: 20, production).
#   CHAIN      Chain spec to benchmark against (default: impetus).
#   OUT_DIR    Output dir for generated *.rs files
#              (default: runtimes/impetus/src/weights).
#   BIN        Path to a pre-built node binary
#              (default: target/release/impetus-node).
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

STEPS="${STEPS:-50}"
REPEAT="${REPEAT:-20}"
CHAIN="${CHAIN:-impetus}"
OUT_DIR="${OUT_DIR:-runtimes/impetus/src/weights}"
BIN="${BIN:-target/release/impetus-node}"

# Full NPoS pallet set wired into `define_benchmarks!`. Order matches the
# macro so the failure log lines up with the source. Skipping a pallet here
# only skips generating its weight file — it does not disable the runtime
# benchmark entry, which means CI catches a regression if you forget to add
# a newly registered pallet.
PALLETS=(
    pallet_babe
    pallet_grandpa
    pallet_session
    pallet_staking
    pallet_offences
    pallet_bags_list
    pallet_election_provider_multi_phase
    pallet_im_online
    pallet_nomination_pools
    pallet_treasury
    pallet_fast_unstake
    pallet_election_provider_support_benchmarking
    # Non-NPoS pallets already wired into the runtime — included so a single
    # CI run regenerates the entire weight bundle in one pass.
    pallet_balances
    pallet_timestamp
    pallet_sudo
    pallet_evm
    pallet_gasless_registry
)

if [ $# -ge 1 ]; then
    PALLETS=("$@")
fi

# The benchmark subcommand only exists when the binary is compiled with
# `--features runtime-benchmarks`. A regular release build still produces a
# `impetus-node` binary, so a `[ -x "$BIN" ]` check alone would
# silently skip the rebuild and then fail later with "unexpected argument
# 'pallet'". Probe the subcommand directly.
if ! "$BIN" benchmark pallet --help >/dev/null 2>&1; then
    echo "[run-benchmarks] rebuilding $BIN with runtime-benchmarks feature..." >&2
    cargo build --release --features runtime-benchmarks --bin impetus-node
fi

mkdir -p "$OUT_DIR"

for pallet in "${PALLETS[@]}"; do
    out="$OUT_DIR/${pallet}.rs"
    echo "[run-benchmarks] $pallet -> $out (steps=$STEPS repeat=$REPEAT)" >&2
    # Use frame-benchmarking-cli's built-in template (no `--template`).
    # Adding a custom Handlebars template later is a follow-up if the
    # generated header needs to embed a project license banner.
    "$BIN" benchmark pallet \
        --chain "$CHAIN" \
        --wasm-execution compiled \
        --pallet "$pallet" \
        --extrinsic "*" \
        --steps "$STEPS" \
        --repeat "$REPEAT" \
        --heap-pages 4096 \
        --output "$out"
done

echo "[run-benchmarks] done. Generated files:" >&2
ls -1 "$OUT_DIR"/*.rs 2>/dev/null || true
