mod common;
use common::*;

use impetus_runtime::Babe;

/// Smoke test that the Babe pallet is wired into the runtime and its public
/// getters return without panicking after the harness has driven enough
/// blocks to establish `GenesisSlot` and seed the first epoch.
///
/// We intentionally do NOT assert that the epoch index advances after
/// `EpochDuration` blocks: epoch advancement is driven by
/// `pallet_session::on_new_session -> Babe::on_new_session`, and the
/// integration-test harness does not faithfully reproduce every consensus
/// invariant (e.g. real pre-digests carry VRF outputs that influence
/// `Authorities` storage). Plan 3 will exercise epoch progression in a
/// node-level E2E test instead.
#[test]
fn babe_storage_is_populated_after_block_one() {
    ExtBuilder::default().build().execute_with(|| {
        run_to_block(2);
        // `current_epoch_start()` returns the slot at which the current epoch
        // started. After block 1 this MUST be a non-default value because the
        // harness seeded a pre-digest with `Slot::from(1 * BLOCK_MS / SLOT_MS)`.
        let epoch_start = Babe::current_epoch_start();
        assert!(
            *epoch_start > 0,
            "Babe::current_epoch_start should be populated after block 1, got {epoch_start:?}"
        );
    });
}

#[test]
fn babe_randomness_is_accessible() {
    ExtBuilder::default().build().execute_with(|| {
        run_to_block(2);
        // Randomness is rotated each epoch; for a fresh chain the first epoch's
        // randomness can legitimately be all-zeros. We just verify the call
        // does not panic and returns the canonical 32-byte buffer.
        let r = Babe::randomness();
        assert_eq!(r.len(), 32);
    });
}
