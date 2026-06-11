//! Unit tests for the session precompile at `0x0818`.
//!
//! Each write entry has at least one happy-path test that calls through the
//! precompile and asserts the resulting `pallet-session` storage state. The
//! DELEGATECALL guard and codec failure modes are exercised via dedicated
//! tests. View entries are tested by seeding storage and comparing the
//! decoded return tuple.

#![cfg(test)]

use alloc::vec;
use alloc::vec::Vec;
use precompile_utils::prelude::*;
use precompile_utils::solidity::encode_with_selector;
use precompile_utils::testing::{compute_selector, MockHandle, PrecompileTesterExt};
use scale_codec::{Decode, Encode};
use sp_core::H160;
use sp_runtime::testing::UintAuthorityId;

use crate::mock::{new_test_ext, session_addr, Runtime};
use crate::SessionPrecompileSet;

// ---- helpers --------------------------------------------------------------

fn caller() -> H160 {
    H160::from_low_u64_be(1)
}

/// Encode a `UintAuthorityId` (= `Runtime::Keys` in the mock) into the SCALE
/// byte string that the precompile expects on the wire.
fn encode_keys(id: u64) -> Vec<u8> {
    UintAuthorityId(id).encode()
}

// ---- selector sanity ------------------------------------------------------
//
// The macro-generated `__SessionPrecompile_test_solidity_signatures` test
// already verifies every declared selector matches its `keccak256` signature.
// We additionally pin the five selectors as hex constants so an accidental
// ABI rename (e.g. `setKeys(bytes,bytes)` → `registerKeys(bytes,bytes)`)
// trips this test loudly, and downstream SDKs that hard-code these selectors
// keep working across refactors.
#[test]
fn anchor_selectors_are_stable() {
    // keccak256("setKeys(bytes,bytes)")[..4]  = 0x250e0e9f
    assert_eq!(compute_selector("setKeys(bytes,bytes)"), 0x250e0e9f);
    // keccak256("purgeKeys()")[..4]           = 0xc8587297
    assert_eq!(compute_selector("purgeKeys()"), 0xc8587297);
    // keccak256("currentIndex()")[..4]        = 0x26987b60
    assert_eq!(compute_selector("currentIndex()"), 0x26987b60);
    // keccak256("nextKeys(address)")[..4]     = 0xc20454fc
    assert_eq!(compute_selector("nextKeys(address)"), 0xc20454fc);
    // keccak256("queuedKeys()")[..4]          = 0x89488230
    assert_eq!(compute_selector("queuedKeys()"), 0x89488230);
}

// ---- write happy paths ----------------------------------------------------

#[test]
fn set_keys_happy_path() {
    new_test_ext().execute_with(|| {
        let me = caller();
        let keys_bytes: Vec<u8> = encode_keys(42);
        let proof: Vec<u8> = Vec::new();
        let data = encode_with_selector(
            compute_selector("setKeys(bytes,bytes)"),
            (
                UnboundedBytes::from(keys_bytes.clone()),
                UnboundedBytes::from(proof),
            ),
        );
        SessionPrecompileSet::new()
            .prepare_test(me, session_addr(), data)
            .execute_returns(());

        // ValidatorId == AccountId in the mock (ConvertInto identity), so the
        // map is keyed by the caller's H160.
        let stored = pallet_session::NextKeys::<Runtime>::get(me).expect("keys stored");
        assert_eq!(stored, UintAuthorityId(42));
    });
}

#[test]
fn set_keys_invalid_scale_reverts() {
    new_test_ext().execute_with(|| {
        let me = caller();
        // `UintAuthorityId` is a u64 — 8 bytes of SCALE input is required.
        // 3 bytes can never decode cleanly.
        let bad_keys: Vec<u8> = vec![0xFFu8; 3];
        let data = encode_with_selector(
            compute_selector("setKeys(bytes,bytes)"),
            (
                UnboundedBytes::from(bad_keys),
                UnboundedBytes::from(Vec::<u8>::new()),
            ),
        );
        SessionPrecompileSet::new()
            .prepare_test(me, session_addr(), data)
            .execute_reverts(|out| {
                alloc::string::String::from_utf8_lossy(out)
                    .contains("InvalidKey: SCALE decode failed")
            });
    });
}

#[test]
fn set_keys_rejects_trailing_bytes() {
    new_test_ext().execute_with(|| {
        let me = caller();
        // 8 bytes decode to a `UintAuthorityId`, plus one stray byte that
        // would silently be dropped without the cursor-empty check.
        let mut keys_bytes = encode_keys(7);
        keys_bytes.push(0xAA);
        let data = encode_with_selector(
            compute_selector("setKeys(bytes,bytes)"),
            (
                UnboundedBytes::from(keys_bytes),
                UnboundedBytes::from(Vec::<u8>::new()),
            ),
        );
        SessionPrecompileSet::new()
            .prepare_test(me, session_addr(), data)
            .execute_reverts(|out| {
                alloc::string::String::from_utf8_lossy(out)
                    .contains("InvalidKey: trailing bytes after SCALE decode")
            });
    });
}

#[test]
fn purge_keys_happy_path() {
    new_test_ext().execute_with(|| {
        let me = caller();
        // First: register keys via the precompile.
        let data = encode_with_selector(
            compute_selector("setKeys(bytes,bytes)"),
            (
                UnboundedBytes::from(encode_keys(99)),
                UnboundedBytes::from(Vec::<u8>::new()),
            ),
        );
        SessionPrecompileSet::new()
            .prepare_test(me, session_addr(), data)
            .execute_returns(());
        assert!(pallet_session::NextKeys::<Runtime>::contains_key(me));

        // Then: purge.
        let data = encode_with_selector(compute_selector("purgeKeys()"), ());
        SessionPrecompileSet::new()
            .prepare_test(me, session_addr(), data)
            .execute_returns(());
        assert!(!pallet_session::NextKeys::<Runtime>::contains_key(me));
    });
}

// ---- views ----------------------------------------------------------------

#[test]
fn current_index_returns_storage() {
    new_test_ext().execute_with(|| {
        pallet_session::CurrentIndex::<Runtime>::put(17u32);
        let data = encode_with_selector(compute_selector("currentIndex()"), ());
        SessionPrecompileSet::new()
            .prepare_test(caller(), session_addr(), data)
            .execute_returns(17u32);
    });
}

#[test]
fn next_keys_returns_encoded_keys() {
    new_test_ext().execute_with(|| {
        let v = H160::from_low_u64_be(2);
        let keys = UintAuthorityId(123);
        pallet_session::NextKeys::<Runtime>::insert(v, keys);

        let data = encode_with_selector(compute_selector("nextKeys(address)"), (Address(v),));
        let expected: UnboundedBytes = encode_keys(123).into();
        SessionPrecompileSet::new()
            .prepare_test(caller(), session_addr(), data)
            .execute_returns(expected);

        // Decoded round-trip sanity check.
        let bytes = encode_keys(123);
        let decoded = UintAuthorityId::decode(&mut &bytes[..]).unwrap();
        assert_eq!(decoded, UintAuthorityId(123));
    });
}

#[test]
fn next_keys_returns_empty_when_unset() {
    new_test_ext().execute_with(|| {
        let v = H160::from_low_u64_be(0xDEAD);
        let data = encode_with_selector(compute_selector("nextKeys(address)"), (Address(v),));
        let expected: UnboundedBytes = Vec::<u8>::new().into();
        SessionPrecompileSet::new()
            .prepare_test(caller(), session_addr(), data)
            .execute_returns(expected);
    });
}

#[test]
fn queued_keys_returns_validators_and_keys() {
    new_test_ext().execute_with(|| {
        let v1 = H160::from_low_u64_be(2);
        let v2 = H160::from_low_u64_be(3);
        let k1 = UintAuthorityId(10);
        let k2 = UintAuthorityId(20);
        pallet_session::QueuedKeys::<Runtime>::put(vec![(v1, k1.clone()), (v2, k2.clone())]);

        let data = encode_with_selector(compute_selector("queuedKeys()"), ());
        let expected_validators: Vec<Address> = vec![Address(v1), Address(v2)];
        let expected_keys: Vec<UnboundedBytes> = vec![k1.encode().into(), k2.encode().into()];
        SessionPrecompileSet::new()
            .prepare_test(caller(), session_addr(), data)
            .execute_returns((expected_validators, expected_keys));
    });
}

// ---- delegatecall guard ---------------------------------------------------

#[test]
fn delegate_guard_blocks_set_keys() {
    new_test_ext().execute_with(|| {
        let original_caller = H160::from_low_u64_be(0xAA);
        let delegate_caller = H160::from_low_u64_be(0xBB);
        let mut handle = MockHandle::new(
            session_addr(),
            fp_evm::Context {
                address: delegate_caller,
                caller: original_caller,
                apparent_value: sp_core::U256::zero(),
            },
        );
        let data = encode_with_selector(
            compute_selector("setKeys(bytes,bytes)"),
            (
                UnboundedBytes::from(encode_keys(1)),
                UnboundedBytes::from(Vec::<u8>::new()),
            ),
        );
        handle.input = data;
        let r = <crate::SessionPrecompile<Runtime> as fp_evm::Precompile>::execute(&mut handle);
        match r.expect_err("setKeys must reject delegatecall") {
            fp_evm::PrecompileFailure::Revert { output, .. } => {
                let needle = b"DELEGATECALL/CALLCODE forbidden";
                assert!(output.windows(needle.len()).any(|w| w == needle));
            }
            other => panic!("expected Revert, got {other:?}"),
        }
    });
}

#[test]
fn delegate_guard_blocks_purge_keys() {
    new_test_ext().execute_with(|| {
        let original_caller = H160::from_low_u64_be(0xAA);
        let delegate_caller = H160::from_low_u64_be(0xBB);
        let mut handle = MockHandle::new(
            session_addr(),
            fp_evm::Context {
                address: delegate_caller,
                caller: original_caller,
                apparent_value: sp_core::U256::zero(),
            },
        );
        let data = encode_with_selector(compute_selector("purgeKeys()"), ());
        handle.input = data;
        let r = <crate::SessionPrecompile<Runtime> as fp_evm::Precompile>::execute(&mut handle);
        match r.expect_err("purgeKeys must reject delegatecall") {
            fp_evm::PrecompileFailure::Revert { output, .. } => {
                let needle = b"DELEGATECALL/CALLCODE forbidden";
                assert!(output.windows(needle.len()).any(|w| w == needle));
            }
            other => panic!("expected Revert, got {other:?}"),
        }
    });
}

// ---- pallet-error mapping ------------------------------------------------

#[test]
fn purge_keys_with_no_keys_reverts() {
    new_test_ext().execute_with(|| {
        let me = caller();
        // No set_keys has been called → `pallet_session::Error::NoKeys`.
        let data = encode_with_selector(compute_selector("purgeKeys()"), ());
        SessionPrecompileSet::new()
            .prepare_test(me, session_addr(), data)
            .execute_reverts(|out| {
                alloc::string::String::from_utf8_lossy(out)
                    .to_lowercase()
                    .contains("dispatched call failed")
            });
    });
}
