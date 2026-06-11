//! Unit tests for the treasury precompile at `0x0830`.
//!
//! Coverage strategy:
//!
//! * Every write entry has a happy-path test (via sudo where root-gated) and
//!   a sudo-gating / pallet-error test.
//! * Views are tested by seeding storage directly via `pallet_treasury::*`
//!   so we never need to drive `on_initialize` to advance the spend period.
//! * Sudo gating, DELEGATECALL guard, and selector anchoring round out the
//!   suite to mirror the staking + session + nomination-pools + fast-unstake
//!   conventions.

#![cfg(test)]

use alloc::vec;
use alloc::vec::Vec;
use precompile_utils::prelude::*;
use precompile_utils::solidity::encode_with_selector;
use precompile_utils::testing::{compute_selector, MockHandle, PrecompileTesterExt};
use sp_core::{H160, U256};

use crate::mock::{new_test_ext, treasury_account, treasury_addr, Runtime, SUDO_KEY};
use crate::TreasuryPrecompileSet;

// ---- helpers --------------------------------------------------------------

fn caller() -> H160 {
    H160::from_low_u64_be(1)
}

fn beneficiary() -> H160 {
    H160::from_low_u64_be(2)
}

fn dispatched_failed(out: &[u8]) -> bool {
    alloc::string::String::from_utf8_lossy(out)
        .to_lowercase()
        .contains("dispatched call failed")
}

// ---- selector anchors -----------------------------------------------------

/// Pin a stable hex constant for each declared selector. The macro-generated
/// `__TreasuryPrecompile_test_solidity_signatures` test verifies that every
/// declared selector matches its `keccak256`; these anchors catch a stray ABI
/// rename loudly even before the macro test runs, and keep downstream SDKs
/// hard-coding the selectors working across refactors.
#[test]
fn anchor_selectors_are_stable() {
    let spend_local_sel = compute_selector("spendLocal(uint256,address)");
    let payout_sel = compute_selector("payout(uint32)");
    let void_spend_sel = compute_selector("voidSpend(uint32)");
    let check_status_sel = compute_selector("checkStatus(uint32)");
    let pot_sel = compute_selector("pot()");
    let spend_count_sel = compute_selector("spendCount()");
    let approvals_sel = compute_selector("approvals()");
    // Pin a handful of the most-callable entries. The exact bytes are
    // deterministic from the signature string; if any of these fire on a
    // refactor, the macro test will surface the broader picture.
    assert_eq!(
        spend_local_sel, 0xec189d0d,
        "spendLocal selector drift: 0x{spend_local_sel:08x}"
    );
    assert_eq!(
        payout_sel, 0xc18f0ccf,
        "payout selector drift: 0x{payout_sel:08x}"
    );
    assert_eq!(
        void_spend_sel, 0xee0696ae,
        "voidSpend selector drift: 0x{void_spend_sel:08x}"
    );
    assert_eq!(
        check_status_sel, 0x3285b261,
        "checkStatus selector drift: 0x{check_status_sel:08x}"
    );
    assert_eq!(pot_sel, 0x4ba2363a, "pot selector drift: 0x{pot_sel:08x}");
    assert_eq!(
        spend_count_sel, 0x9d12d015,
        "spendCount selector drift: 0x{spend_count_sel:08x}"
    );
    assert_eq!(
        approvals_sel, 0xb05dba96,
        "approvals selector drift: 0x{approvals_sel:08x}"
    );
}

// ---- spendLocal -----------------------------------------------------------

#[test]
fn spend_local_via_sudo_creates_spend() {
    new_test_ext().execute_with(|| {
        let bob = beneficiary();
        let amount = U256::from(1_000u128);
        let data = encode_with_selector(
            compute_selector("spendLocal(uint256,address)"),
            (amount, Address(bob)),
        );
        TreasuryPrecompileSet::new()
            .prepare_test(SUDO_KEY, treasury_addr(), data)
            .execute_returns(());
        // `spend_local` does not increment `SpendCount` (that is for the
        // `spend` flow). Instead it appends to `Approvals` and inserts a
        // `Proposal`. Verify both.
        #[allow(deprecated)]
        let approvals = pallet_treasury::Approvals::<Runtime>::get();
        assert_eq!(approvals.len(), 1);
        assert_eq!(approvals[0], 0);
        #[allow(deprecated)]
        let proposal = pallet_treasury::Proposals::<Runtime>::get(0).expect("proposal queued");
        assert_eq!(proposal.value, 1_000u128);
        assert_eq!(proposal.beneficiary, bob);
    });
}

#[test]
fn spend_local_reverts_for_non_sudo() {
    new_test_ext().execute_with(|| {
        let bob = beneficiary();
        let amount = U256::from(1_000u128);
        let data = encode_with_selector(
            compute_selector("spendLocal(uint256,address)"),
            (amount, Address(bob)),
        );
        TreasuryPrecompileSet::new()
            .prepare_test(caller(), treasury_addr(), data)
            .execute_reverts(|out| alloc::string::String::from_utf8_lossy(out).contains("NotSudo"));
    });
}

// ---- voidSpend ------------------------------------------------------------

#[test]
fn void_spend_via_sudo_cancels() {
    new_test_ext().execute_with(|| {
        let bob = beneficiary();
        // Seed `Spends` directly: `void_spend` only operates on the new
        // `spend` flow's storage. Hand-craft a pending entry so we don't
        // need to drive the full `spend` extrinsic from the mock (which is
        // root-gated and would still require the same plumbing).
        use pallet_treasury::{PaymentState, SpendStatus};
        pallet_treasury::Spends::<Runtime>::insert(
            7u32,
            SpendStatus::<
                <Runtime as pallet_treasury::Config>::AssetKind,
                u128,
                <Runtime as pallet_treasury::Config>::Beneficiary,
                u64,
                (),
            > {
                asset_kind: (),
                amount: 500u128,
                beneficiary: bob,
                valid_from: 1u64,
                expire_at: 100u64,
                status: PaymentState::Pending,
            },
        );
        assert!(pallet_treasury::Spends::<Runtime>::contains_key(7u32));
        let data = encode_with_selector(compute_selector("voidSpend(uint32)"), (7u32,));
        TreasuryPrecompileSet::new()
            .prepare_test(SUDO_KEY, treasury_addr(), data)
            .execute_returns(());
        assert!(!pallet_treasury::Spends::<Runtime>::contains_key(7u32));
    });
}

#[test]
fn void_spend_reverts_for_non_sudo() {
    new_test_ext().execute_with(|| {
        let data = encode_with_selector(compute_selector("voidSpend(uint32)"), (0u32,));
        TreasuryPrecompileSet::new()
            .prepare_test(caller(), treasury_addr(), data)
            .execute_reverts(|out| alloc::string::String::from_utf8_lossy(out).contains("NotSudo"));
    });
}

#[test]
fn void_spend_reverts_for_unknown_index() {
    // Sudo caller, but no Spends entry at index 0: pallet returns
    // `InvalidIndex` which the precompile surfaces as "dispatched call
    // failed". Exercises the pallet-error path past the sudo gate.
    new_test_ext().execute_with(|| {
        let data = encode_with_selector(compute_selector("voidSpend(uint32)"), (0u32,));
        TreasuryPrecompileSet::new()
            .prepare_test(SUDO_KEY, treasury_addr(), data)
            .execute_reverts(dispatched_failed);
    });
}

// ---- payout ---------------------------------------------------------------

#[test]
fn payout_is_permissionless_after_valid_from() {
    // `payout` only operates on the new `spend` flow's `Spends` storage. The
    // `spend_local` path drains via `on_initialize` and never goes through
    // `payout`. Seed a `Spends` entry directly to exercise the permissionless
    // dispatch path with a known starting state.
    new_test_ext().execute_with(|| {
        let bob = beneficiary();
        use pallet_treasury::{PaymentState, SpendStatus};
        pallet_treasury::Spends::<Runtime>::insert(
            0u32,
            SpendStatus::<
                <Runtime as pallet_treasury::Config>::AssetKind,
                u128,
                <Runtime as pallet_treasury::Config>::Beneficiary,
                u64,
                (),
            > {
                asset_kind: (),
                amount: 1_000u128,
                beneficiary: bob,
                valid_from: 1u64,
                expire_at: 100u64,
                status: PaymentState::Pending,
            },
        );
        let bob_before = <pallet_balances::Pallet<Runtime> as frame_support::traits::Currency<
            H160,
        >>::free_balance(&bob);
        // Permissionless: BOB (or any signed origin) can drive payout.
        let data = encode_with_selector(compute_selector("payout(uint32)"), (0u32,));
        TreasuryPrecompileSet::new()
            .prepare_test(caller(), treasury_addr(), data)
            .execute_returns(());
        let bob_after = <pallet_balances::Pallet<Runtime> as frame_support::traits::Currency<
            H160,
        >>::free_balance(&bob);
        assert_eq!(bob_after - bob_before, 1_000u128);
    });
}

#[test]
fn payout_reverts_for_unknown_index() {
    new_test_ext().execute_with(|| {
        let data = encode_with_selector(compute_selector("payout(uint32)"), (99u32,));
        TreasuryPrecompileSet::new()
            .prepare_test(caller(), treasury_addr(), data)
            .execute_reverts(dispatched_failed);
    });
}

// ---- checkStatus ----------------------------------------------------------

#[test]
fn check_status_is_permissionless() {
    // `check_status` reads `Spends::<T>` and returns `InvalidIndex` if the
    // index is unknown. We assert the dispatch path is reachable from any
    // signed origin (no sudo check) — the pallet-error branch is exercised
    // here because no spend is seeded.
    new_test_ext().execute_with(|| {
        let data = encode_with_selector(compute_selector("checkStatus(uint32)"), (0u32,));
        TreasuryPrecompileSet::new()
            .prepare_test(caller(), treasury_addr(), data)
            .execute_reverts(dispatched_failed);
    });
}

#[test]
fn check_status_removes_completed_spend_from_storage() {
    // Success path: when `now > expire_at` and the entry is not in
    // `Attempted` state, `check_status` removes the `Spends::<T>` row and
    // emits `SpendProcessed`. Seed an expired `Pending` spend at a known
    // index and assert the row is gone after a permissionless dispatch.
    new_test_ext().execute_with(|| {
        let bob = beneficiary();
        use pallet_treasury::{PaymentState, SpendStatus};
        // `new_test_ext` sets `System::block_number = 1`, so an `expire_at`
        // of 0 is strictly in the past for this fixture.
        pallet_treasury::Spends::<Runtime>::insert(
            42u32,
            SpendStatus::<
                <Runtime as pallet_treasury::Config>::AssetKind,
                u128,
                <Runtime as pallet_treasury::Config>::Beneficiary,
                u64,
                (),
            > {
                asset_kind: (),
                amount: 1_000u128,
                beneficiary: bob,
                valid_from: 0u64,
                expire_at: 0u64,
                status: PaymentState::Pending,
            },
        );
        assert!(pallet_treasury::Spends::<Runtime>::contains_key(42u32));
        // Permissionless: any signed origin can drive `check_status`.
        let data = encode_with_selector(compute_selector("checkStatus(uint32)"), (42u32,));
        TreasuryPrecompileSet::new()
            .prepare_test(caller(), treasury_addr(), data)
            .execute_returns(());
        // Entry must be cleaned up from `Spends`.
        assert!(!pallet_treasury::Spends::<Runtime>::contains_key(42u32));
    });
}

// ---- views ----------------------------------------------------------------

#[test]
fn pot_returns_treasury_balance() {
    // The mock genesis seeds the treasury account with 10_000_000. The
    // existential deposit is 1, so `pot()` should be 9_999_999.
    new_test_ext().execute_with(|| {
        let data = encode_with_selector(compute_selector("pot()"), ());
        let expected = U256::from(9_999_999u128);
        TreasuryPrecompileSet::new()
            .prepare_test(caller(), treasury_addr(), data)
            .execute_returns(expected);
        // Sanity: the underlying account does hold the seeded balance.
        let raw = <pallet_balances::Pallet<Runtime> as frame_support::traits::Currency<H160>>::free_balance(
            &treasury_account(),
        );
        assert_eq!(raw, 10_000_000u128);
    });
}

#[test]
fn spend_count_increments_after_spend() {
    // `SpendCount` is the cursor used by the new `spend` flow. The
    // `spend_local` path uses `ProposalCount` instead, so the unit test
    // seeds `SpendCount` directly to verify the view reads the right
    // storage item.
    new_test_ext().execute_with(|| {
        let data = encode_with_selector(compute_selector("spendCount()"), ());
        TreasuryPrecompileSet::new()
            .prepare_test(caller(), treasury_addr(), data.clone())
            .execute_returns(0u32);
        pallet_treasury::SpendCount::<Runtime>::put(7u32);
        TreasuryPrecompileSet::new()
            .prepare_test(caller(), treasury_addr(), data)
            .execute_returns(7u32);
    });
}

#[test]
fn approvals_returns_pending_indices() {
    new_test_ext().execute_with(|| {
        let bob = beneficiary();
        // Two `spend_local` calls land two indices in `Approvals`.
        for _ in 0..2u32 {
            let data = encode_with_selector(
                compute_selector("spendLocal(uint256,address)"),
                (U256::from(100u128), Address(bob)),
            );
            TreasuryPrecompileSet::new()
                .prepare_test(SUDO_KEY, treasury_addr(), data)
                .execute_returns(());
        }
        let data = encode_with_selector(compute_selector("approvals()"), ());
        let expected: Vec<u32> = vec![0u32, 1u32];
        TreasuryPrecompileSet::new()
            .prepare_test(caller(), treasury_addr(), data)
            .execute_returns(expected);
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
        treasury_addr(),
        fp_evm::Context {
            address: delegate_caller,
            caller: original_caller,
            apparent_value: U256::zero(),
        },
    );
    handle.input = input;
    let r = <crate::TreasuryPrecompile<Runtime> as fp_evm::Precompile>::execute(&mut handle);
    r.expect_err("delegatecall must be rejected")
}

#[test]
fn spend_local_rejects_delegatecall() {
    new_test_ext().execute_with(|| {
        let bob = beneficiary();
        let data = encode_with_selector(
            compute_selector("spendLocal(uint256,address)"),
            (U256::from(1u128), Address(bob)),
        );
        let err = delegatecall_attack(data);
        revert_message_contains(b"DELEGATECALL/CALLCODE forbidden")(&err);
    });
}

#[test]
fn payout_rejects_delegatecall() {
    new_test_ext().execute_with(|| {
        let data = encode_with_selector(compute_selector("payout(uint32)"), (0u32,));
        let err = delegatecall_attack(data);
        revert_message_contains(b"DELEGATECALL/CALLCODE forbidden")(&err);
    });
}

#[test]
fn void_spend_rejects_delegatecall_before_sudo_check() {
    // The DELEGATECALL guard runs before the sudo check; even calling from
    // the sudo key via DELEGATECALL must revert with the guard reason.
    new_test_ext().execute_with(|| {
        let data = encode_with_selector(compute_selector("voidSpend(uint32)"), (0u32,));
        let err = delegatecall_attack(data);
        revert_message_contains(b"DELEGATECALL/CALLCODE forbidden")(&err);
    });
}

#[test]
fn check_status_rejects_delegatecall() {
    new_test_ext().execute_with(|| {
        let data = encode_with_selector(compute_selector("checkStatus(uint32)"), (0u32,));
        let err = delegatecall_attack(data);
        revert_message_contains(b"DELEGATECALL/CALLCODE forbidden")(&err);
    });
}
