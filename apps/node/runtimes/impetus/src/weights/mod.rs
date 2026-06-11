//! Per-pallet benchmark-generated weight modules for the impetus runtime.
//!
//! Files in this directory are produced by `scripts/run-benchmarks.sh`
//! (which shells out to `impetus-node benchmark pallet`).
//! Each generated file exposes a `WeightInfo<T>` struct that implements the
//! upstream pallet's `WeightInfo` trait.
//!
//! Wiring workflow:
//!
//! 1. Run `scripts/run-benchmarks.sh` on a Linux reference host (Substrate
//!    reference hardware: AMD Ryzen 7 7700 / equivalent, NVMe SSD, native
//!    Linux — running on macOS or in a VM yields unreliable timings).
//! 2. Commit the generated `pallet_<name>.rs` files into this directory.
//! 3. Re-export each file below (`pub mod pallet_staking;`).
//! 4. Swap `type WeightInfo = ();` for
//!    `type WeightInfo = weights::pallet_staking::WeightInfo<Runtime>;`
//!    in the corresponding pallet config in `lib.rs`.
//!
//! Until step 2 completes, every NPoS pallet still uses `WeightInfo = ()`,
//! which yields unsafe (free) weights — acceptable for dev/test chains, but
//! a launch blocker for mainnet impetus.
