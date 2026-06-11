//! Production NPoS timing + bond constants (Polkadot-style economics).
//!
//! Default profile (mainnet impetus): 4-hour sessions, 24-hour eras, 28-era
//! (~28-day) bonding, 27-era slash-defer — matching Polkadot so a misbehaving
//! validator stays bonded (and slashable) for the full unbonding window. The
//! constants are compile-time, not governance-tunable; changing them requires a
//! WASM runtime upgrade + `spec_version` bump.
//!
//! Invariant: `BONDING_DURATION_ERAS <= HistoryDepth` (84 in the runtime), and
//! `SLASH_DEFER_DURATION_ERAS < BONDING_DURATION_ERAS` so deferred slashes
//! apply before the offender can fully unbond.
//!
//! A `runtime-test-fast` feature compresses session/era length for E2E test
//! runs (30s sessions / 1-min eras) so a full lifecycle fits in one test run.

use crate::{Balance, BlockNumber};
use sp_staking::{EraIndex, SessionIndex};

pub const UNIT: Balance = 1_000_000_000_000_000_000;

#[cfg(not(feature = "runtime-test-fast"))]
pub const BLOCKS_PER_SESSION: BlockNumber = 2_400; // 4 h @ 6s blocks
#[cfg(not(feature = "runtime-test-fast"))]
pub const SESSIONS_PER_ERA: SessionIndex = 6; // 24 h eras
#[cfg(not(feature = "runtime-test-fast"))]
pub const BONDING_DURATION_ERAS: EraIndex = 28; // ~28 days
#[cfg(not(feature = "runtime-test-fast"))]
pub const SLASH_DEFER_DURATION_ERAS: EraIndex = 27; // ~27 days (< bonding)

#[cfg(feature = "runtime-test-fast")]
pub const BLOCKS_PER_SESSION: BlockNumber = 5; // 30s
#[cfg(feature = "runtime-test-fast")]
pub const SESSIONS_PER_ERA: SessionIndex = 2; // 1 min
#[cfg(feature = "runtime-test-fast")]
pub const BONDING_DURATION_ERAS: EraIndex = 2;
#[cfg(feature = "runtime-test-fast")]
pub const SLASH_DEFER_DURATION_ERAS: EraIndex = 1;

pub const MAX_NOMINATIONS: u32 = 16;
pub const MAX_NOMINATORS_PER_VALIDATOR: u32 = 16;
pub const VALIDATOR_COUNT_TARGET: u32 = 8;
pub const MAX_VALIDATOR_COUNT: u32 = 32;

pub const MIN_VALIDATOR_BOND: Balance = 1_000 * UNIT;
pub const MIN_NOMINATOR_BOND: Balance = 10 * UNIT;

pub const REPORT_LONGEVITY: u64 = (BONDING_DURATION_ERAS as u64)
    * (SESSIONS_PER_ERA as u64)
    * (BLOCKS_PER_SESSION as u64);

pub const SESSION_PERIOD: BlockNumber = BLOCKS_PER_SESSION;
pub const SESSION_OFFSET: BlockNumber = 0;
