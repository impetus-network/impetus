//! Unit tests for the fast-unstake precompile at `0x0828`.
//!
//! Coverage strategy:
//!
//! * Every write entry has both a happy-path and an error / revert test.
//! * Views are tested by seeding storage directly via `pallet_fast_unstake::*`
//!   so we never need to drive `on_idle` to land a `Head` batch.
//! * Sudo gating, DELEGATECALL guard, and selector anchoring round out the
//!   suite to mirror the staking + session + nomination-pools conventions.

#![cfg(test)]

use alloc::vec;
use alloc::vec::Vec;
use precompile_utils::prelude::*;
use precompile_utils::solidity::encode_with_selector;
use precompile_utils::testing::{compute_selector, MockHandle, PrecompileTesterExt};
use sp_core::{H160, U256};

use crate::mock::{
    enable_fast_unstake, fast_unstake_addr, new_test_ext, pre_bond, Runtime, SUDO_KEY,
};
use crate::FastUnstakePrecompileSet;

// ---- helpers --------------------------------------------------------------

fn caller() -> H160 {
    H160::from_low_u64_be(1)
}

fn dispatched_failed(out: &[u8]) -> bool {
    alloc::string::String::from_utf8_lossy(out)
        .to_lowercase()
        .contains("dispatched call failed")
}

// ---- selector anchors -----------------------------------------------------

/// The macro-generated `__FastUnstakePrecompile_test_solidity_signatures`
/// test already verifies every declared selector matches its `keccak256`. We
/// additionally pin a few selectors as hex constants so a stray ABI rename
/// trips this test loudly even before the macro test runs, and downstream
/// SDKs hard-coding these selectors keep working across refactors.
#[test]
fn anchor_selectors_are_stable() {
    let register_sel = compute_selector("registerFastUnstake()");
    let deregister_sel = compute_selector("deregister()");
    let control_sel = compute_selector("control(uint32)");
    let queue_sel = compute_selector("queue(address)");
    assert_eq!(
        register_sel, 0xa47a6feb,
        "registerFastUnstake selector drift: 0x{register_sel:08x}"
    );
    assert_eq!(
        deregister_sel, 0xaff5edb1,
        "deregister selector drift: 0x{deregister_sel:08x}"
    );
    assert_eq!(
        control_sel, 0x6bce648c,
        "control selector drift: 0x{control_sel:08x}"
    );
    assert_eq!(
        queue_sel, 0x038ee1cb,
        "queue selector drift: 0x{queue_sel:08x}"
    );
}

// ---- registerFastUnstake --------------------------------------------------

#[test]
fn register_fast_unstake_happy_path() {
    new_test_ext().execute_with(|| {
        let me = caller();
        // Pallet refuses to register unless `ErasToCheckPerBlock != 0`. In
        // production this is set via `control` (sudo); here we seed it
        // directly so we exercise the happy-path through the register entry.
        enable_fast_unstake(1);
        // Bond the controller in the StakingMock so `stash_by_ctrl` resolves.
        pre_bond(me, 100_000);
        let data = encode_with_selector(compute_selector("registerFastUnstake()"), ());
        FastUnstakePrecompileSet::new()
            .prepare_test(me, fast_unstake_addr(), data)
            .execute_returns(());
        // Queue must now record the deposit for the stash.
        let deposit = pallet_fast_unstake::Queue::<Runtime>::get(me).expect("queued");
        assert_eq!(deposit, 10u128);
    });
}

#[test]
fn register_fast_unstake_requires_bonded() {
    new_test_ext().execute_with(|| {
        let me = caller();
        enable_fast_unstake(1);
        // No pre_bond: `stash_by_ctrl` returns `NotController` which the
        // pallet maps to `Error::NotController` and the precompile to
        // "dispatched call failed".
        let data = encode_with_selector(compute_selector("registerFastUnstake()"), ());
        FastUnstakePrecompileSet::new()
            .prepare_test(me, fast_unstake_addr(), data)
            .execute_reverts(dispatched_failed);
    });
}

#[test]
fn register_fast_unstake_when_pallet_halted_reverts() {
    // `ErasToCheckPerBlock == 0` halts the pallet. The pallet then surfaces
    // `Error::CallNotAllowed` for both `register_fast_unstake` and
    // `deregister`. Exercises the `CallNotAllowed` revert path.
    new_test_ext().execute_with(|| {
        let me = caller();
        pre_bond(me, 100_000);
        // No `enable_fast_unstake` -- default is 0.
        let data = encode_with_selector(compute_selector("registerFastUnstake()"), ());
        FastUnstakePrecompileSet::new()
            .prepare_test(me, fast_unstake_addr(), data)
            .execute_reverts(dispatched_failed);
    });
}

// ---- deregister -----------------------------------------------------------

#[test]
fn deregister_happy_path() {
    new_test_ext().execute_with(|| {
        let me = caller();
        enable_fast_unstake(1);
        pre_bond(me, 100_000);
        // Register first so deregister has state to operate on.
        let register_data = encode_with_selector(compute_selector("registerFastUnstake()"), ());
        FastUnstakePrecompileSet::new()
            .prepare_test(me, fast_unstake_addr(), register_data)
            .execute_returns(());
        assert!(pallet_fast_unstake::Queue::<Runtime>::contains_key(me));
        let data = encode_with_selector(compute_selector("deregister()"), ());
        FastUnstakePrecompileSet::new()
            .prepare_test(me, fast_unstake_addr(), data)
            .execute_returns(());
        assert!(!pallet_fast_unstake::Queue::<Runtime>::contains_key(me));
    });
}

#[test]
fn deregister_when_not_registered_reverts() {
    new_test_ext().execute_with(|| {
        let me = caller();
        enable_fast_unstake(1);
        pre_bond(me, 100_000);
        let data = encode_with_selector(compute_selector("deregister()"), ());
        FastUnstakePrecompileSet::new()
            .prepare_test(me, fast_unstake_addr(), data)
            .execute_reverts(dispatched_failed);
    });
}

// ---- control --------------------------------------------------------------

#[test]
fn control_sudo_path_updates_eras() {
    new_test_ext().execute_with(|| {
        let data = encode_with_selector(compute_selector("control(uint32)"), (5u32,));
        FastUnstakePrecompileSet::new()
            .prepare_test(SUDO_KEY, fast_unstake_addr(), data)
            .execute_returns(());
        assert_eq!(
            pallet_fast_unstake::ErasToCheckPerBlock::<Runtime>::get(),
            5u32
        );
    });
}

#[test]
fn control_reverts_for_non_sudo_caller() {
    new_test_ext().execute_with(|| {
        let data = encode_with_selector(compute_selector("control(uint32)"), (3u32,));
        FastUnstakePrecompileSet::new()
            .prepare_test(caller(), fast_unstake_addr(), data)
            .execute_reverts(|out| alloc::string::String::from_utf8_lossy(out).contains("NotSudo"));
    });
}

#[test]
fn control_rejects_value_exceeding_max() {
    // `MaxErasToCheckPerBlock = 16` in the mock. Pallet refuses anything
    // greater than that with `Error::CallNotAllowed`.
    new_test_ext().execute_with(|| {
        let data = encode_with_selector(compute_selector("control(uint32)"), (100u32,));
        FastUnstakePrecompileSet::new()
            .prepare_test(SUDO_KEY, fast_unstake_addr(), data)
            .execute_reverts(dispatched_failed);
    });
}

// ---- views ----------------------------------------------------------------

#[test]
fn head_returns_empty_when_no_active_unstake() {
    new_test_ext().execute_with(|| {
        let data = encode_with_selector(compute_selector("head()"), ());
        let empty: Vec<Address> = Vec::new();
        FastUnstakePrecompileSet::new()
            .prepare_test(caller(), fast_unstake_addr(), data)
            .execute_returns(empty);
    });
}

#[test]
fn head_returns_stashes_when_head_seeded() {
    // Seed `Head` directly: the on_idle path is not exercised here, so we
    // construct an `UnstakeRequest` and write it. This drives both the
    // `head()` and `headDeposits()` views from a known shape.
    use frame_support::BoundedVec;
    use pallet_fast_unstake::types::UnstakeRequest;
    new_test_ext().execute_with(|| {
        let a = H160::from_low_u64_be(0xAA);
        let b = H160::from_low_u64_be(0xBB);
        let stashes: BoundedVec<_, <Runtime as pallet_fast_unstake::Config>::BatchSize> =
            vec![(a, 10u128), (b, 20u128)]
                .try_into()
                .expect("fits in BatchSize=16");
        let req = UnstakeRequest::<Runtime> {
            stashes,
            checked: Default::default(),
        };
        pallet_fast_unstake::Head::<Runtime>::put(req);
        let data = encode_with_selector(compute_selector("head()"), ());
        let expected: Vec<Address> = vec![Address(a), Address(b)];
        FastUnstakePrecompileSet::new()
            .prepare_test(caller(), fast_unstake_addr(), data)
            .execute_returns(expected);
        let data = encode_with_selector(compute_selector("headDeposits()"), ());
        let expected: Vec<U256> = vec![U256::from(10u128), U256::from(20u128)];
        FastUnstakePrecompileSet::new()
            .prepare_test(caller(), fast_unstake_addr(), data)
            .execute_returns(expected);
    });
}

#[test]
fn head_deposits_returns_empty_when_no_active_unstake() {
    new_test_ext().execute_with(|| {
        let data = encode_with_selector(compute_selector("headDeposits()"), ());
        let empty: Vec<U256> = Vec::new();
        FastUnstakePrecompileSet::new()
            .prepare_test(caller(), fast_unstake_addr(), data)
            .execute_returns(empty);
    });
}

#[test]
fn queue_returns_deposit_after_register() {
    new_test_ext().execute_with(|| {
        let me = caller();
        enable_fast_unstake(1);
        pre_bond(me, 100_000);
        let register_data = encode_with_selector(compute_selector("registerFastUnstake()"), ());
        FastUnstakePrecompileSet::new()
            .prepare_test(me, fast_unstake_addr(), register_data)
            .execute_returns(());
        let data = encode_with_selector(compute_selector("queue(address)"), (Address(me),));
        FastUnstakePrecompileSet::new()
            .prepare_test(caller(), fast_unstake_addr(), data)
            .execute_returns(U256::from(10u128));
    });
}

#[test]
fn queue_returns_zero_when_not_registered() {
    new_test_ext().execute_with(|| {
        let stranger = H160::from_low_u64_be(0xDEAD);
        let data = encode_with_selector(compute_selector("queue(address)"), (Address(stranger),));
        FastUnstakePrecompileSet::new()
            .prepare_test(caller(), fast_unstake_addr(), data)
            .execute_returns(U256::zero());
    });
}

#[test]
fn eras_to_check_per_block_returns_storage() {
    new_test_ext().execute_with(|| {
        // Default is 0.
        let data = encode_with_selector(compute_selector("erasToCheckPerBlock()"), ());
        FastUnstakePrecompileSet::new()
            .prepare_test(caller(), fast_unstake_addr(), data.clone())
            .execute_returns(0u32);
        // Flip via sudo and re-read.
        let control_data = encode_with_selector(compute_selector("control(uint32)"), (7u32,));
        FastUnstakePrecompileSet::new()
            .prepare_test(SUDO_KEY, fast_unstake_addr(), control_data)
            .execute_returns(());
        FastUnstakePrecompileSet::new()
            .prepare_test(caller(), fast_unstake_addr(), data)
            .execute_returns(7u32);
    });
}

// ---- delegatecall guard ---------------------------------------------------

fn revert_message_contains(needle: &[u8]) -> impl Fn(&fp_evm::PrecompileFailure) + '_ {
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
        fast_unstake_addr(),
        fp_evm::Context {
            address: delegate_caller,
            caller: original_caller,
            apparent_value: U256::zero(),
        },
    );
    handle.input = input;
    let r = <crate::FastUnstakePrecompile<Runtime> as fp_evm::Precompile>::execute(&mut handle);
    r.expect_err("delegatecall must be rejected")
}

#[test]
fn register_rejects_delegatecall() {
    new_test_ext().execute_with(|| {
        let data = encode_with_selector(compute_selector("registerFastUnstake()"), ());
        let err = delegatecall_attack(data);
        revert_message_contains(b"DELEGATECALL/CALLCODE forbidden")(&err);
    });
}

#[test]
fn deregister_rejects_delegatecall() {
    new_test_ext().execute_with(|| {
        let data = encode_with_selector(compute_selector("deregister()"), ());
        let err = delegatecall_attack(data);
        revert_message_contains(b"DELEGATECALL/CALLCODE forbidden")(&err);
    });
}

#[test]
fn control_rejects_delegatecall_before_sudo_check() {
    // The DELEGATECALL guard runs before the sudo check; even calling from
    // the sudo key via DELEGATECALL must revert with the guard reason.
    new_test_ext().execute_with(|| {
        let data = encode_with_selector(compute_selector("control(uint32)"), (1u32,));
        let err = delegatecall_attack(data);
        revert_message_contains(b"DELEGATECALL/CALLCODE forbidden")(&err);
    });
}
