//! Unit tests for the staking-admin precompile at `0x0840`.
//!
//! Coverage strategy mirrors T6 (`precompile-treasury`):
//!
//! * Every one of the 11 write entries has a sudo-happy-path test. Where the
//!   post-condition is reachable in a pallet-staking mock with `NoElection`
//!   (validator-count tuning, era forcing, invulnerables, staking-configs)
//!   we assert the storage update directly. For entries that need state we
//!   can't easily seed without driving a full election (`forceUnstake`,
//!   `cancelDeferredSlash`, `chillOther`) we use error-mapping tests that
//!   verify the dispatch happens past the sudo gate.
//! * Three representative sudo-rejection tests confirm the gate fires
//!   uniformly across signatures.
//! * Three representative DELEGATECALL-guard tests confirm the guard runs
//!   *before* the sudo check.
//! * `anchor_selectors_are_stable` pins keccak256 selectors for the most-
//!   callable entries so a stray ABI rename surfaces loudly.

#![cfg(test)]

use alloc::vec;
use alloc::vec::Vec;
use precompile_utils::prelude::*;
use precompile_utils::solidity::encode_with_selector;
use precompile_utils::testing::{compute_selector, MockHandle, PrecompileTesterExt};
use sp_core::{H160, U256};

use crate::mock::{new_test_ext, staking_admin_addr, Runtime, SUDO_KEY};
use crate::StakingAdminPrecompileSet;

// ---- helpers --------------------------------------------------------------

fn caller() -> H160 {
    H160::from_low_u64_be(1)
}

fn alice() -> H160 {
    H160::from_low_u64_be(0xA1)
}

fn bob() -> H160 {
    H160::from_low_u64_be(0xB0)
}

fn dispatched_failed(out: &[u8]) -> bool {
    alloc::string::String::from_utf8_lossy(out)
        .to_lowercase()
        .contains("dispatched call failed")
}

// ---- selector anchors -----------------------------------------------------

/// Pin a stable hex constant for each declared selector. The macro-generated
/// `__StakingAdminPrecompile_test_solidity_signatures` test verifies that
/// every declared selector matches its `keccak256`; these anchors catch a
/// stray ABI rename loudly even before the macro test runs, and keep
/// downstream SDKs hard-coding the selectors working across refactors.
#[test]
fn anchor_selectors_are_stable() {
    let set_validator_count_sel = compute_selector("setValidatorCount(uint32)");
    let increase_validator_count_sel = compute_selector("increaseValidatorCount(uint32)");
    let scale_validator_count_sel = compute_selector("scaleValidatorCount(uint8)");
    let set_invulnerables_sel = compute_selector("setInvulnerables(address[])");
    let force_unstake_sel = compute_selector("forceUnstake(address,uint32)");
    let force_new_era_sel = compute_selector("forceNewEra()");
    let force_no_eras_sel = compute_selector("forceNoEras()");
    let force_new_era_always_sel = compute_selector("forceNewEraAlways()");
    let cancel_deferred_slash_sel = compute_selector("cancelDeferredSlash(uint32,uint32[])");
    let set_staking_configs_sel =
        compute_selector("setStakingConfigs(uint256,uint256,uint32,uint32,uint8,uint32)");
    let chill_other_sel = compute_selector("chillOther(address)");

    // Frozen as the canonical contract for the chain's external SDKs. The
    // macro-generated test verifies each selector matches its `keccak256`
    // signature; these anchors give a fast pinpoint when one entry drifts.
    assert_eq!(
        set_validator_count_sel, 0xf88528c1,
        "setValidatorCount selector drift: 0x{set_validator_count_sel:08x}"
    );
    assert_eq!(
        increase_validator_count_sel, 0x6535e399,
        "increaseValidatorCount selector drift: 0x{increase_validator_count_sel:08x}"
    );
    assert_eq!(
        scale_validator_count_sel, 0xd923d5ce,
        "scaleValidatorCount selector drift: 0x{scale_validator_count_sel:08x}"
    );
    assert_eq!(
        set_invulnerables_sel, 0xb106ada4,
        "setInvulnerables selector drift: 0x{set_invulnerables_sel:08x}"
    );
    assert_eq!(
        force_unstake_sel, 0x56d40fbd,
        "forceUnstake selector drift: 0x{force_unstake_sel:08x}"
    );
    assert_eq!(
        force_new_era_sel, 0x8c633dc1,
        "forceNewEra selector drift: 0x{force_new_era_sel:08x}"
    );
    assert_eq!(
        force_no_eras_sel, 0xa4d6111d,
        "forceNoEras selector drift: 0x{force_no_eras_sel:08x}"
    );
    assert_eq!(
        force_new_era_always_sel, 0x2a8d48fa,
        "forceNewEraAlways selector drift: 0x{force_new_era_always_sel:08x}"
    );
    assert_eq!(
        cancel_deferred_slash_sel, 0x0dc65b9c,
        "cancelDeferredSlash selector drift: 0x{cancel_deferred_slash_sel:08x}"
    );
    assert_eq!(
        set_staking_configs_sel, 0xb70eff86,
        "setStakingConfigs selector drift: 0x{set_staking_configs_sel:08x}"
    );
    assert_eq!(
        chill_other_sel, 0x7fbeac46,
        "chillOther selector drift: 0x{chill_other_sel:08x}"
    );
}

// ---- setValidatorCount ----------------------------------------------------

#[test]
fn set_validator_count_via_sudo_updates_storage() {
    new_test_ext().execute_with(|| {
        let data = encode_with_selector(compute_selector("setValidatorCount(uint32)"), (42u32,));
        StakingAdminPrecompileSet::new()
            .prepare_test(SUDO_KEY, staking_admin_addr(), data)
            .execute_returns(());
        assert_eq!(pallet_staking::ValidatorCount::<Runtime>::get(), 42);
    });
}

#[test]
fn set_validator_count_reverts_for_non_sudo() {
    new_test_ext().execute_with(|| {
        let data = encode_with_selector(compute_selector("setValidatorCount(uint32)"), (42u32,));
        StakingAdminPrecompileSet::new()
            .prepare_test(caller(), staking_admin_addr(), data)
            .execute_reverts(|out| alloc::string::String::from_utf8_lossy(out).contains("NotSudo"));
    });
}

// ---- increaseValidatorCount ----------------------------------------------

#[test]
fn increase_validator_count_via_sudo_increments() {
    new_test_ext().execute_with(|| {
        // Seed a known value then increment by 5; `new_test_ext` already
        // seeds 4 but make the starting point explicit so future tweaks to
        // the ext builder don't silently shift the assertion.
        pallet_staking::ValidatorCount::<Runtime>::put(10);
        let data =
            encode_with_selector(compute_selector("increaseValidatorCount(uint32)"), (5u32,));
        StakingAdminPrecompileSet::new()
            .prepare_test(SUDO_KEY, staking_admin_addr(), data)
            .execute_returns(());
        assert_eq!(pallet_staking::ValidatorCount::<Runtime>::get(), 15);
    });
}

// ---- scaleValidatorCount -------------------------------------------------

#[test]
fn scale_validator_count_via_sudo_scales() {
    // The pallet uses `old + factor.mul_floor(old)`. With `MaxValidatorSet`
    // set to 100 in the mock, start from 40 so a 50% factor lands `60 < 100`
    // and avoids tripping `TooManyValidators`.
    new_test_ext().execute_with(|| {
        pallet_staking::ValidatorCount::<Runtime>::put(40);
        let data = encode_with_selector(compute_selector("scaleValidatorCount(uint8)"), (50u8,));
        StakingAdminPrecompileSet::new()
            .prepare_test(SUDO_KEY, staking_admin_addr(), data)
            .execute_returns(());
        assert_eq!(pallet_staking::ValidatorCount::<Runtime>::get(), 60);
    });
}

#[test]
fn scale_validator_count_rejects_out_of_range_factor() {
    new_test_ext().execute_with(|| {
        let data = encode_with_selector(compute_selector("scaleValidatorCount(uint8)"), (101u8,));
        StakingAdminPrecompileSet::new()
            .prepare_test(SUDO_KEY, staking_admin_addr(), data)
            .execute_reverts(|out| {
                alloc::string::String::from_utf8_lossy(out).contains("0..=100 percent")
            });
    });
}

// ---- setInvulnerables ----------------------------------------------------

#[test]
fn set_invulnerables_via_sudo_replaces_list() {
    new_test_ext().execute_with(|| {
        let list = vec![Address(alice()), Address(bob())];
        let data = encode_with_selector(compute_selector("setInvulnerables(address[])"), (list,));
        StakingAdminPrecompileSet::new()
            .prepare_test(SUDO_KEY, staking_admin_addr(), data)
            .execute_returns(());
        let stored = pallet_staking::Invulnerables::<Runtime>::get();
        assert_eq!(stored, vec![alice(), bob()]);
    });
}

#[test]
fn set_invulnerables_reverts_for_non_sudo() {
    new_test_ext().execute_with(|| {
        let list = vec![Address(alice())];
        let data = encode_with_selector(compute_selector("setInvulnerables(address[])"), (list,));
        StakingAdminPrecompileSet::new()
            .prepare_test(caller(), staking_admin_addr(), data)
            .execute_reverts(|out| alloc::string::String::from_utf8_lossy(out).contains("NotSudo"));
    });
}

// ---- forceUnstake ---------------------------------------------------------

#[test]
fn force_unstake_via_sudo_dispatches_pallet_error_for_unbonded() {
    // `force_unstake` on a stash that was never bonded reverts inside the
    // pallet (`Error::NotStash` or similar). We assert the dispatch happens
    // past the sudo gate; the staking-bonding setup needed to drive the
    // happy path requires election-provider plumbing that's deliberately
    // stubbed in this mock. The E2E suite (T20) covers the success path
    // against a live impetus runtime.
    new_test_ext().execute_with(|| {
        let data = encode_with_selector(
            compute_selector("forceUnstake(address,uint32)"),
            (Address(alice()), 0u32),
        );
        StakingAdminPrecompileSet::new()
            .prepare_test(SUDO_KEY, staking_admin_addr(), data)
            .execute_reverts(dispatched_failed);
    });
}

// ---- forceNewEra / forceNoEras / forceNewEraAlways ----------------------

#[test]
fn force_new_era_via_sudo_sets_force() {
    new_test_ext().execute_with(|| {
        let data = encode_with_selector(compute_selector("forceNewEra()"), ());
        StakingAdminPrecompileSet::new()
            .prepare_test(SUDO_KEY, staking_admin_addr(), data)
            .execute_returns(());
        assert_eq!(
            pallet_staking::ForceEra::<Runtime>::get(),
            pallet_staking::Forcing::ForceNew
        );
    });
}

#[test]
fn force_no_eras_via_sudo_sets_force() {
    new_test_ext().execute_with(|| {
        let data = encode_with_selector(compute_selector("forceNoEras()"), ());
        StakingAdminPrecompileSet::new()
            .prepare_test(SUDO_KEY, staking_admin_addr(), data)
            .execute_returns(());
        assert_eq!(
            pallet_staking::ForceEra::<Runtime>::get(),
            pallet_staking::Forcing::ForceNone
        );
    });
}

#[test]
fn force_new_era_always_via_sudo_sets_force() {
    new_test_ext().execute_with(|| {
        let data = encode_with_selector(compute_selector("forceNewEraAlways()"), ());
        StakingAdminPrecompileSet::new()
            .prepare_test(SUDO_KEY, staking_admin_addr(), data)
            .execute_returns(());
        assert_eq!(
            pallet_staking::ForceEra::<Runtime>::get(),
            pallet_staking::Forcing::ForceAlways
        );
    });
}

// ---- cancelDeferredSlash --------------------------------------------------

#[test]
fn cancel_deferred_slash_via_sudo_dispatches() {
    // With no `UnappliedSlashes` seeded at the target era, the pallet returns
    // `EmptyTargets` or `InvalidSlashIndex`. We assert the dispatch happens
    // past the sudo gate; the happy-path mutation (entry removal) requires
    // an offence to land which the mock's `NoElection` precludes. E2E
    // coverage in T20 against the live impetus runtime drives the real
    // post-condition.
    new_test_ext().execute_with(|| {
        let indices: Vec<u32> = vec![0u32, 1u32];
        let data = encode_with_selector(
            compute_selector("cancelDeferredSlash(uint32,uint32[])"),
            (0u32, indices),
        );
        StakingAdminPrecompileSet::new()
            .prepare_test(SUDO_KEY, staking_admin_addr(), data)
            .execute_reverts(dispatched_failed);
    });
}

// ---- setStakingConfigs ----------------------------------------------------

#[test]
fn set_staking_configs_via_sudo_updates_all() {
    new_test_ext().execute_with(|| {
        let data = encode_with_selector(
            compute_selector("setStakingConfigs(uint256,uint256,uint32,uint32,uint8,uint32)"),
            (
                U256::from(123u128),
                U256::from(456u128),
                7u32,
                9u32,
                25u8,
                10_000_000u32, // 1% in parts-per-billion
            ),
        );
        StakingAdminPrecompileSet::new()
            .prepare_test(SUDO_KEY, staking_admin_addr(), data)
            .execute_returns(());
        assert_eq!(pallet_staking::MinNominatorBond::<Runtime>::get(), 123u128);
        assert_eq!(pallet_staking::MinValidatorBond::<Runtime>::get(), 456u128);
        assert_eq!(
            pallet_staking::MaxNominatorsCount::<Runtime>::get(),
            Some(7u32)
        );
        assert_eq!(
            pallet_staking::MaxValidatorsCount::<Runtime>::get(),
            Some(9u32)
        );
        assert_eq!(
            pallet_staking::ChillThreshold::<Runtime>::get(),
            Some(sp_runtime::Percent::from_percent(25))
        );
        assert_eq!(
            pallet_staking::MinCommission::<Runtime>::get(),
            sp_runtime::Perbill::from_parts(10_000_000u32)
        );
    });
}

#[test]
fn set_staking_configs_reverts_for_non_sudo() {
    new_test_ext().execute_with(|| {
        let data = encode_with_selector(
            compute_selector("setStakingConfigs(uint256,uint256,uint32,uint32,uint8,uint32)"),
            (U256::from(1u128), U256::from(1u128), 0u32, 0u32, 0u8, 0u32),
        );
        StakingAdminPrecompileSet::new()
            .prepare_test(caller(), staking_admin_addr(), data)
            .execute_reverts(|out| alloc::string::String::from_utf8_lossy(out).contains("NotSudo"));
    });
}

#[test]
fn set_staking_configs_rejects_out_of_range_chill_threshold() {
    new_test_ext().execute_with(|| {
        let data = encode_with_selector(
            compute_selector("setStakingConfigs(uint256,uint256,uint32,uint32,uint8,uint32)"),
            (
                U256::from(1u128),
                U256::from(1u128),
                0u32,
                0u32,
                101u8, // > 100
                0u32,
            ),
        );
        StakingAdminPrecompileSet::new()
            .prepare_test(SUDO_KEY, staking_admin_addr(), data)
            .execute_reverts(|out| {
                alloc::string::String::from_utf8_lossy(out)
                    .contains("chillThreshold must be 0..=100 percent")
            });
    });
}

#[test]
fn set_staking_configs_rejects_out_of_range_min_commission() {
    new_test_ext().execute_with(|| {
        let data = encode_with_selector(
            compute_selector("setStakingConfigs(uint256,uint256,uint32,uint32,uint8,uint32)"),
            (
                U256::from(1u128),
                U256::from(1u128),
                0u32,
                0u32,
                0u8,
                1_000_000_001u32, // > 1_000_000_000
            ),
        );
        StakingAdminPrecompileSet::new()
            .prepare_test(SUDO_KEY, staking_admin_addr(), data)
            .execute_reverts(|out| {
                alloc::string::String::from_utf8_lossy(out)
                    .contains("minCommission must be 0..=1_000_000_000 parts")
            });
    });
}

// ---- chillOther ----------------------------------------------------------

#[test]
fn chill_other_via_sudo_dispatches_pallet_error_for_unbonded() {
    // `chill_other` reads the stash's ledger; with no bond seeded the pallet
    // returns `NotStash` / `NotController`. We assert the dispatch happens
    // past the sudo gate; happy-path mutation needs a fully-bonded stash
    // chilled at the ChillThreshold which is covered in E2E T20.
    new_test_ext().execute_with(|| {
        let data =
            encode_with_selector(compute_selector("chillOther(address)"), (Address(alice()),));
        StakingAdminPrecompileSet::new()
            .prepare_test(SUDO_KEY, staking_admin_addr(), data)
            .execute_reverts(dispatched_failed);
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
    let original_caller = SUDO_KEY; // exercise the "sudo via delegatecall" path
    let delegate_caller = H160::from_low_u64_be(0xBB);
    let mut handle = MockHandle::new(
        staking_admin_addr(),
        fp_evm::Context {
            address: delegate_caller,
            caller: original_caller,
            apparent_value: U256::zero(),
        },
    );
    handle.input = input;
    let r = <crate::StakingAdminPrecompile<Runtime> as fp_evm::Precompile>::execute(&mut handle);
    r.expect_err("delegatecall must be rejected")
}

#[test]
fn set_validator_count_rejects_delegatecall_before_sudo_check() {
    new_test_ext().execute_with(|| {
        let data = encode_with_selector(compute_selector("setValidatorCount(uint32)"), (42u32,));
        let err = delegatecall_attack(data);
        revert_message_contains(b"DELEGATECALL/CALLCODE forbidden")(&err);
    });
}

#[test]
fn force_new_era_rejects_delegatecall_before_sudo_check() {
    new_test_ext().execute_with(|| {
        let data = encode_with_selector(compute_selector("forceNewEra()"), ());
        let err = delegatecall_attack(data);
        revert_message_contains(b"DELEGATECALL/CALLCODE forbidden")(&err);
    });
}

#[test]
fn chill_other_rejects_delegatecall_before_sudo_check() {
    new_test_ext().execute_with(|| {
        let data =
            encode_with_selector(compute_selector("chillOther(address)"), (Address(alice()),));
        let err = delegatecall_attack(data);
        revert_message_contains(b"DELEGATECALL/CALLCODE forbidden")(&err);
    });
}
