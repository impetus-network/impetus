# Reproducible runtime builds (M-8)

A mainnet runtime must be byte-reproducible so independent parties can verify
that the on-chain WASM matches the audited source. Two layers:

## 1. Pinned dependencies

The workspace pins `polkadot-sdk`, `frontier`, and `evm` to fixed git revisions
(not moving branches) in `apps/node/Cargo.toml`, and `Cargo.lock` is committed.
A moving `branch = "stable2603"` would let an upstream force-push silently
change the build; a `rev = "<40-hex>"` cannot.

Locked revisions (matching the committed `Cargo.lock` at the time of pinning):

| Dependency       | Revision                                   |
|------------------|--------------------------------------------|
| polkadot-sdk     | `0af459f3b9fe176b285f8431b8b9e19f427ec6cb` |
| frontier         | `baf505d8feeaaa0ba636a003faa49e4ad897b592` |
| evm (rust-ethereum) | `a656db9050c65170b050360c3fa66c0fd8bf226a` |

To bump: change the `rev` in `apps/node/Cargo.toml`, run `cargo update -p <crate>`,
review the `Cargo.lock` diff, and rebuild. Never revert to a `branch` pin.

Builds use `--locked` (see `Dockerfile`) so CI/release builds fail rather than
silently mutating the lockfile.

## 2. Deterministic WASM with srtool

`cargo build` is not guaranteed bit-identical across machines. Use
[srtool](https://github.com/paritytech/srtool) (a pinned Docker toolchain) to
produce the canonical runtime blob and publish its hash:

```bash
# Impetus mainnet runtime
srtool build --package impetus-runtime --runtime-dir apps/node/runtimes/impetus

# srtool prints the proposal hash + blake2-256 of the compressed WASM.
# Publish that hash in the release notes; every operator can re-run srtool and
# confirm they get the same bytes before trusting a runtime upgrade.
```

The GitHub `runtime-benchmarks.yml` workflow is smoke-only and NOT a source of
truth for weights or for the release blob. The release runtime + its srtool
hash should be produced on a clean CI runner and attached to the tagged release.

## Release checklist

- [ ] Dependencies pinned by `rev`, `Cargo.lock` committed.
- [ ] `cargo build --release --locked` succeeds (no lockfile drift).
- [ ] Runtime built with srtool; blake2-256 hash recorded in release notes.
- [ ] Docker image built from the pinned toolchain digest, runs as non-root.
- [ ] Weights regenerated on reference hardware (see `runtimes/impetus/src/weights/mod.rs`).
