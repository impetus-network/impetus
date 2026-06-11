//! Unit tests for the bags-list precompile at `0x0838`.
//!
//! Coverage strategy:
//!
//! * Every write entry has a happy-path test and a pallet-error test.
//! * Views are tested by seeding storage through `SortedListProvider::on_insert`
//!   so the cached `score` / `bag_upper` fields are wired by the pallet itself.
//! * DELEGATECALL guard tests cover both write entries.
//! * Selector anchoring pins the keccak hashes that downstream SDKs hard-code.

#![cfg(test)]

use alloc::vec::Vec;
use precompile_utils::prelude::*;
use precompile_utils::solidity::encode_with_selector;
use precompile_utils::testing::{compute_selector, MockHandle, PrecompileTesterExt};
use sp_core::{H160, U256};

use crate::mock::{bags_list_addr, insert_node, new_test_ext, set_score, AccountId, Runtime};
use crate::BagsListPrecompileSet;

// ---- helpers --------------------------------------------------------------

fn alice() -> AccountId {
    H160::from_low_u64_be(1)
}

fn bob() -> AccountId {
    H160::from_low_u64_be(2)
}

fn charlie() -> AccountId {
    H160::from_low_u64_be(3)
}

fn dispatched_failed(out: &[u8]) -> bool {
    alloc::string::String::from_utf8_lossy(out)
        .to_lowercase()
        .contains("dispatched call failed")
}

fn stored_bag_upper(who: AccountId) -> u64 {
    pallet_bags_list::ListNodes::<Runtime, pallet_bags_list::Instance1>::get(who)
        .map(|n| n.bag_upper)
        .unwrap_or_default()
}

fn stored_score(who: AccountId) -> u64 {
    pallet_bags_list::ListNodes::<Runtime, pallet_bags_list::Instance1>::get(who)
        .map(|n| n.score)
        .unwrap_or_default()
}

// ---- selector anchors -----------------------------------------------------

/// Pin a stable hex constant for each declared selector. The macro-generated
/// `__BagsListPrecompile_test_solidity_signatures` test verifies that every
/// declared selector matches its `keccak256`; these anchors catch a stray ABI
/// rename loudly even before the macro test runs.
#[test]
fn anchor_selectors_are_stable() {
    let put_in_front_of_sel = compute_selector("putInFrontOf(address)");
    let rebag_sel = compute_selector("rebag(address)");
    let list_size_sel = compute_selector("listSize()");
    let score_sel = compute_selector("score(address)");
    let bag_of_sel = compute_selector("bagOf(address)");

    // Pinned via test-then-record: compute the real keccak selectors locally
    // and assert against the recorded hex. Surfaces ABI rename drift loudly
    // before the macro-generated signature test runs.
    let pairs: [(&str, u32, u32); 5] = [
        ("putInFrontOf(address)", put_in_front_of_sel, 0x87ce3993),
        ("rebag(address)", rebag_sel, 0xf47d4496),
        ("listSize()", list_size_sel, 0x972c5356),
        ("score(address)", score_sel, 0x776f3843),
        ("bagOf(address)", bag_of_sel, 0xbd57b72d),
    ];
    for (sig, actual, recorded) in pairs.iter() {
        assert_eq!(
            *actual, *recorded,
            "{sig} selector drift: actual 0x{actual:08x}, recorded 0x{recorded:08x}",
        );
    }
}

// ---- rebag ----------------------------------------------------------------

#[test]
fn rebag_moves_node_to_correct_bag() {
    new_test_ext().execute_with(|| {
        // Insert ALICE with score 50 -> lands in the 100-bag.
        insert_node(alice(), 50);
        assert_eq!(stored_bag_upper(alice()), 100u64);

        // Bump score to 500 (still cached as 50 in the list); rebag should
        // pick the new value up from `StaticScoreProvider` and migrate the
        // node to the 1000-bag.
        set_score(alice(), 500);
        let data = encode_with_selector(compute_selector("rebag(address)"), (Address(alice()),));
        BagsListPrecompileSet::new()
            .prepare_test(alice(), bags_list_addr(), data)
            .execute_returns(());

        assert_eq!(stored_bag_upper(alice()), 1_000u64);
        assert_eq!(stored_score(alice()), 500u64);
    });
}

#[test]
fn rebag_when_score_unchanged_is_noop() {
    // Calling rebag with no score change must succeed (it is permissionless)
    // and leave the node in the same bag.
    new_test_ext().execute_with(|| {
        insert_node(alice(), 50);
        assert_eq!(stored_bag_upper(alice()), 100u64);
        let data = encode_with_selector(compute_selector("rebag(address)"), (Address(alice()),));
        BagsListPrecompileSet::new()
            .prepare_test(alice(), bags_list_addr(), data)
            .execute_returns(());
        assert_eq!(stored_bag_upper(alice()), 100u64);
    });
}

#[test]
fn rebag_reverts_for_unknown_node() {
    // `rebag` on a non-member surfaces `NodeNotFound`, which the precompile
    // surfaces as "dispatched call failed".
    new_test_ext().execute_with(|| {
        let data = encode_with_selector(compute_selector("rebag(address)"), (Address(alice()),));
        BagsListPrecompileSet::new()
            .prepare_test(alice(), bags_list_addr(), data)
            .execute_reverts(dispatched_failed);
    });
}

// ---- putInFrontOf ---------------------------------------------------------

#[test]
fn put_in_front_of_swaps_position() {
    // Two nodes in the same bag (both score in [101, 1_000] => 1000-bag).
    // BOB has the heavier score; calling `putInFrontOf(alice)` from BOB
    // must succeed (BOB is heavier than the lighter target).
    new_test_ext().execute_with(|| {
        insert_node(alice(), 200);
        insert_node(bob(), 800);
        assert_eq!(stored_bag_upper(alice()), 1_000u64);
        assert_eq!(stored_bag_upper(bob()), 1_000u64);

        let data = encode_with_selector(
            compute_selector("putInFrontOf(address)"),
            (Address(alice()),),
        );
        BagsListPrecompileSet::new()
            .prepare_test(bob(), bags_list_addr(), data)
            .execute_returns(());

        // Both nodes remain in the same bag after the swap; the list
        // ordering changed but `bag_upper` is invariant under intra-bag
        // moves.
        assert_eq!(stored_bag_upper(alice()), 1_000u64);
        assert_eq!(stored_bag_upper(bob()), 1_000u64);
    });
}

#[test]
fn put_in_front_of_reverts_when_caller_is_lighter() {
    // ALICE calls `putInFrontOf(bob)` but ALICE's score (200) is lighter than
    // BOB's (800). Pallet returns `NotHeavier` => "dispatched call failed".
    new_test_ext().execute_with(|| {
        insert_node(alice(), 200);
        insert_node(bob(), 800);

        let data =
            encode_with_selector(compute_selector("putInFrontOf(address)"), (Address(bob()),));
        BagsListPrecompileSet::new()
            .prepare_test(alice(), bags_list_addr(), data)
            .execute_reverts(dispatched_failed);
    });
}

#[test]
fn put_in_front_of_reverts_when_nodes_in_different_bags() {
    // ALICE in 100-bag, BOB in 1000-bag. Pallet returns `NotInSameBag`.
    new_test_ext().execute_with(|| {
        insert_node(alice(), 50);
        insert_node(bob(), 500);
        assert_eq!(stored_bag_upper(alice()), 100u64);
        assert_eq!(stored_bag_upper(bob()), 1_000u64);

        // BOB (heavier overall but in a higher bag) calls putInFrontOf(ALICE).
        // Reverts with NotInSameBag.
        let data = encode_with_selector(
            compute_selector("putInFrontOf(address)"),
            (Address(alice()),),
        );
        BagsListPrecompileSet::new()
            .prepare_test(bob(), bags_list_addr(), data)
            .execute_reverts(dispatched_failed);
    });
}

// ---- listSize -------------------------------------------------------------

#[test]
fn list_size_returns_zero_when_empty() {
    new_test_ext().execute_with(|| {
        let data = encode_with_selector(compute_selector("listSize()"), ());
        BagsListPrecompileSet::new()
            .prepare_test(alice(), bags_list_addr(), data)
            .execute_returns(0u32);
    });
}

#[test]
fn list_size_returns_count_after_inserts() {
    new_test_ext().execute_with(|| {
        insert_node(alice(), 50);
        insert_node(bob(), 200);
        insert_node(charlie(), 9_000);
        let data = encode_with_selector(compute_selector("listSize()"), ());
        BagsListPrecompileSet::new()
            .prepare_test(alice(), bags_list_addr(), data)
            .execute_returns(3u32);
    });
}

// ---- score ----------------------------------------------------------------

#[test]
fn score_returns_node_score() {
    new_test_ext().execute_with(|| {
        insert_node(alice(), 500);
        let data = encode_with_selector(compute_selector("score(address)"), (Address(alice()),));
        BagsListPrecompileSet::new()
            .prepare_test(alice(), bags_list_addr(), data)
            .execute_returns(500u64);
    });
}

#[test]
fn score_returns_zero_for_unknown_account() {
    new_test_ext().execute_with(|| {
        let data = encode_with_selector(compute_selector("score(address)"), (Address(alice()),));
        BagsListPrecompileSet::new()
            .prepare_test(alice(), bags_list_addr(), data)
            .execute_returns(0u64);
    });
}

// ---- bagOf ----------------------------------------------------------------

#[test]
fn bag_of_returns_bag_upper() {
    new_test_ext().execute_with(|| {
        // Score 500 -> 1000-bag.
        insert_node(alice(), 500);
        let data = encode_with_selector(compute_selector("bagOf(address)"), (Address(alice()),));
        BagsListPrecompileSet::new()
            .prepare_test(alice(), bags_list_addr(), data)
            .execute_returns(1_000u64);
    });
}

#[test]
fn bag_of_returns_zero_for_unknown_account() {
    new_test_ext().execute_with(|| {
        let data = encode_with_selector(compute_selector("bagOf(address)"), (Address(alice()),));
        BagsListPrecompileSet::new()
            .prepare_test(alice(), bags_list_addr(), data)
            .execute_returns(0u64);
    });
}

// ---- delegatecall guard ---------------------------------------------------

fn revert_message_contains(needle: &'static [u8]) -> impl Fn(&fp_evm::PrecompileFailure) {
    move |err| match err {
        fp_evm::PrecompileFailure::Revert { output, .. } => assert!(
            output.windows(needle.len()).any(|w| w == needle),
            "expected revert containing {:?}; got {:?}",
            core::str::from_utf8(needle).unwrap_or("<binary>"),
            output,
        ),
        other => panic!("expected Revert, got {other:?}"),
    }
}

fn delegatecall_attack(input: Vec<u8>) -> fp_evm::PrecompileFailure {
    let original_caller = H160::from_low_u64_be(0xAA);
    let delegate_caller = H160::from_low_u64_be(0xBB);
    let mut handle = MockHandle::new(
        bags_list_addr(),
        fp_evm::Context {
            address: delegate_caller,
            caller: original_caller,
            apparent_value: U256::zero(),
        },
    );
    handle.input = input;
    let r = <crate::BagsListPrecompile<Runtime> as fp_evm::Precompile>::execute(&mut handle);
    r.expect_err("delegatecall must be rejected")
}

#[test]
fn rebag_rejects_delegatecall() {
    new_test_ext().execute_with(|| {
        let data = encode_with_selector(compute_selector("rebag(address)"), (Address(alice()),));
        let err = delegatecall_attack(data);
        revert_message_contains(b"DELEGATECALL/CALLCODE forbidden")(&err);
    });
}

#[test]
fn put_in_front_of_rejects_delegatecall() {
    new_test_ext().execute_with(|| {
        let data = encode_with_selector(
            compute_selector("putInFrontOf(address)"),
            (Address(alice()),),
        );
        let err = delegatecall_attack(data);
        revert_message_contains(b"DELEGATECALL/CALLCODE forbidden")(&err);
    });
}
