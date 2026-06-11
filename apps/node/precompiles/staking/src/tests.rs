//! Unit tests for the staking precompile at `0x0810`.
//!
//! Each write entry has at least one happy-path test that calls through the
//! precompile and asserts the resulting `pallet-staking` storage state. The
//! DELEGATECALL guard, codec bounds, and pallet error mapping are exercised
//! via dedicated tests. View entries are tested by seeding storage and
//! comparing the decoded return tuple.
//!
//! NOTE: pallet-staking's real test mock relies on a full election provider
//! and bags-list — we deliberately stub those out (see `mock.rs`), so era-
//! based flows (`payout_stakers`, `payout_stakers_by_page`, `reap_stash`)
//! are exercised in their *error-mapping* form rather than the happy path.
//! Full era progression is covered by the runtime integration tests in
//! `runtimes/impetus/tests/` and by the Plan 3 E2E suites.

#![cfg(test)]

use alloc::vec;
use alloc::vec::Vec;
use frame_support::dispatch::RawOrigin;
use precompile_utils::prelude::*;
use precompile_utils::solidity::encode_with_selector;
use precompile_utils::testing::{compute_selector, MockHandle, PrecompileTesterExt};
use sp_core::{H160, U256};

use crate::mock::{force_bond, new_test_ext, staking_addr, Runtime};
use crate::{
    IndividualPointsSolidity, RewardDestination, StakingPrecompileSet, UnlockingChunkSolidity,
    ValidatorPrefsSolidity,
};

// ---- helpers --------------------------------------------------------------

fn caller() -> H160 {
    H160::from_low_u64_be(1)
}

fn rd(kind: u8, account: H160) -> RewardDestination {
    RewardDestination {
        kind,
        account: Address(account),
    }
}

// ---- selector sanity ------------------------------------------------------
//
// The macro-generated `__StakingPrecompile_test_solidity_signatures` test
// already verifies every declared selector matches its `keccak256` signature.
// We additionally pin three high-traffic selectors as hex constants so an
// accidental ABI rename (e.g. `currentEra()` → `getCurrentEra()`) trips this
// test loudly even before the macro test runs, and downstream SDKs that
// hard-code these selectors keep working across refactors.
#[test]
fn anchor_view_selectors_are_stable() {
    // keccak256("currentEra()")[..4]                      = 0x973628f6
    assert_eq!(compute_selector("currentEra()"), 0x973628f6);
    // keccak256("historyDepth()")[..4]                    = 0xfdad314c
    assert_eq!(compute_selector("historyDepth()"), 0xfdad314c);
    // keccak256("bond(uint256,(uint8,address))")[..4]     = 0xec13e8ba
    assert_eq!(
        compute_selector("bond(uint256,(uint8,address))"),
        0xec13e8ba
    );
}

// ---- write happy paths ----------------------------------------------------

#[test]
fn bond_dispatches_signed_call_and_records_ledger() {
    new_test_ext().execute_with(|| {
        let me = caller();
        let data = encode_with_selector(
            compute_selector("bond(uint256,(uint8,address))"),
            (U256::from(1_000u128), rd(0, H160::zero())),
        );
        StakingPrecompileSet::new()
            .prepare_test(me, staking_addr(), data)
            .execute_returns(());
        let ledger = pallet_staking::Ledger::<Runtime>::get(me).expect("ledger exists");
        assert_eq!(ledger.stash, me);
        assert_eq!(ledger.active, 1_000u128);
        assert_eq!(ledger.total, 1_000u128);
    });
}

#[test]
fn bond_extra_increases_active_balance() {
    new_test_ext().execute_with(|| {
        let me = caller();
        force_bond(me, 500);
        let data = encode_with_selector(
            compute_selector("bondExtra(uint256)"),
            (U256::from(250u128),),
        );
        StakingPrecompileSet::new()
            .prepare_test(me, staking_addr(), data)
            .execute_returns(());
        let ledger = pallet_staking::Ledger::<Runtime>::get(me).unwrap();
        assert_eq!(ledger.active, 750u128);
    });
}

#[test]
fn unbond_creates_unlocking_chunk() {
    new_test_ext().execute_with(|| {
        let me = caller();
        force_bond(me, 1_000);
        let data =
            encode_with_selector(compute_selector("unbond(uint256)"), (U256::from(300u128),));
        StakingPrecompileSet::new()
            .prepare_test(me, staking_addr(), data)
            .execute_returns(());
        let ledger = pallet_staking::Ledger::<Runtime>::get(me).unwrap();
        assert_eq!(ledger.active, 700u128);
        assert_eq!(ledger.unlocking.len(), 1);
        assert_eq!(ledger.unlocking[0].value, 300u128);
    });
}

#[test]
fn withdraw_unbonded_dispatch_succeeds_with_no_chunks() {
    new_test_ext().execute_with(|| {
        let me = caller();
        force_bond(me, 100);
        // No matured chunks yet → call is a no-op but must dispatch cleanly.
        let data = encode_with_selector(compute_selector("withdrawUnbonded(uint32)"), (0u32,));
        StakingPrecompileSet::new()
            .prepare_test(me, staking_addr(), data)
            .execute_returns(());
    });
}

#[test]
fn validate_registers_validator_prefs() {
    new_test_ext().execute_with(|| {
        let me = caller();
        force_bond(me, 1_000);
        // commission = 50 percent → Perbill::from_parts(50 * 10_000_000) = 500_000_000 (50%).
        let prefs = ValidatorPrefsSolidity {
            commission_percent: 50,
            blocked: false,
        };
        let data = encode_with_selector(compute_selector("validate((uint16,bool))"), (prefs,));
        StakingPrecompileSet::new()
            .prepare_test(me, staking_addr(), data)
            .execute_returns(());
        let stored = pallet_staking::Validators::<Runtime>::get(me);
        assert_eq!(stored.commission.deconstruct(), 500_000_000u32);
        assert!(!stored.blocked);
    });
}

#[test]
fn nominate_registers_targets() {
    new_test_ext().execute_with(|| {
        let me = caller();
        let v1 = H160::from_low_u64_be(2);
        let v2 = H160::from_low_u64_be(3);
        // Targets must already be validators (and not blocked) for nominate.
        force_bond(v1, 100);
        force_bond(v2, 100);
        pallet_staking::Pallet::<Runtime>::validate(
            RawOrigin::Signed(v1).into(),
            Default::default(),
        )
        .unwrap();
        pallet_staking::Pallet::<Runtime>::validate(
            RawOrigin::Signed(v2).into(),
            Default::default(),
        )
        .unwrap();
        force_bond(me, 1_000);
        let targets: Vec<Address> = vec![Address(v1), Address(v2)];
        let data = encode_with_selector(compute_selector("nominate(address[])"), (targets,));
        StakingPrecompileSet::new()
            .prepare_test(me, staking_addr(), data)
            .execute_returns(());
        let n = pallet_staking::Nominators::<Runtime>::get(me).expect("nominations stored");
        assert_eq!(n.targets.len(), 2);
        assert_eq!(n.targets[0], v1);
        assert_eq!(n.targets[1], v2);
    });
}

#[test]
fn chill_clears_validator_and_nominator_state() {
    new_test_ext().execute_with(|| {
        let me = caller();
        force_bond(me, 1_000);
        pallet_staking::Pallet::<Runtime>::validate(
            RawOrigin::Signed(me).into(),
            Default::default(),
        )
        .unwrap();
        assert!(pallet_staking::Validators::<Runtime>::contains_key(me));
        let data = encode_with_selector(compute_selector("chill()"), ());
        StakingPrecompileSet::new()
            .prepare_test(me, staking_addr(), data)
            .execute_returns(());
        assert!(!pallet_staking::Validators::<Runtime>::contains_key(me));
    });
}

#[test]
fn set_payee_updates_reward_destination() {
    new_test_ext().execute_with(|| {
        let me = caller();
        let dest = H160::from_low_u64_be(0xCAFE);
        force_bond(me, 1_000);
        let data = encode_with_selector(
            compute_selector("setPayee((uint8,address))"),
            (rd(3, dest),),
        );
        StakingPrecompileSet::new()
            .prepare_test(me, staking_addr(), data)
            .execute_returns(());
        let stored = pallet_staking::Payee::<Runtime>::get(me).expect("payee stored");
        assert!(matches!(
            stored,
            pallet_staking::RewardDestination::Account(a) if a == dest
        ));
    });
}

#[test]
fn rebond_moves_unlocking_chunks_back_to_active() {
    new_test_ext().execute_with(|| {
        let me = caller();
        force_bond(me, 1_000);
        pallet_staking::Pallet::<Runtime>::unbond(RawOrigin::Signed(me).into(), 300).unwrap();
        let pre = pallet_staking::Ledger::<Runtime>::get(me).unwrap();
        assert_eq!(pre.active, 700u128);
        let data =
            encode_with_selector(compute_selector("rebond(uint256)"), (U256::from(200u128),));
        StakingPrecompileSet::new()
            .prepare_test(me, staking_addr(), data)
            .execute_returns(());
        let post = pallet_staking::Ledger::<Runtime>::get(me).unwrap();
        assert_eq!(post.active, 900u128);
    });
}

#[test]
fn kick_removes_specific_nominators_from_validator() {
    new_test_ext().execute_with(|| {
        let validator = H160::from_low_u64_be(2);
        let nominator = H160::from_low_u64_be(3);
        force_bond(validator, 1_000);
        force_bond(nominator, 500);
        pallet_staking::Pallet::<Runtime>::validate(
            RawOrigin::Signed(validator).into(),
            Default::default(),
        )
        .unwrap();
        pallet_staking::Pallet::<Runtime>::nominate(
            RawOrigin::Signed(nominator).into(),
            vec![validator],
        )
        .unwrap();
        let pre = pallet_staking::Nominators::<Runtime>::get(nominator).unwrap();
        assert_eq!(pre.targets.len(), 1);
        let targets: Vec<Address> = vec![Address(nominator)];
        let data = encode_with_selector(compute_selector("kick(address[])"), (targets,));
        StakingPrecompileSet::new()
            .prepare_test(validator, staking_addr(), data)
            .execute_returns(());
        let post = pallet_staking::Nominators::<Runtime>::get(nominator).unwrap();
        assert_eq!(post.targets.len(), 0);
    });
}

#[test]
fn chill_other_when_caller_is_self_behaves_like_chill() {
    new_test_ext().execute_with(|| {
        let me = caller();
        force_bond(me, 1_000);
        pallet_staking::Pallet::<Runtime>::validate(
            RawOrigin::Signed(me).into(),
            Default::default(),
        )
        .unwrap();
        let data = encode_with_selector(compute_selector("chillOther(address)"), (Address(me),));
        StakingPrecompileSet::new()
            .prepare_test(me, staking_addr(), data)
            .execute_returns(());
        assert!(!pallet_staking::Validators::<Runtime>::contains_key(me));
    });
}

#[test]
fn force_apply_min_commission_floors_commission() {
    new_test_ext().execute_with(|| {
        let v = caller();
        force_bond(v, 1_000);
        pallet_staking::Pallet::<Runtime>::validate(
            RawOrigin::Signed(v).into(),
            pallet_staking::ValidatorPrefs {
                commission: sp_runtime::Perbill::from_percent(2),
                blocked: false,
            },
        )
        .unwrap();
        // Raise MinCommission to 10%.
        pallet_staking::MinCommission::<Runtime>::put(sp_runtime::Perbill::from_percent(10));
        let data = encode_with_selector(
            compute_selector("forceApplyMinCommission(address)"),
            (Address(v),),
        );
        StakingPrecompileSet::new()
            .prepare_test(v, staking_addr(), data)
            .execute_returns(());
        let stored = pallet_staking::Validators::<Runtime>::get(v);
        assert_eq!(stored.commission, sp_runtime::Perbill::from_percent(10));
    });
}

#[test]
fn payout_stakers_reverts_when_no_era_reward_recorded() {
    new_test_ext().execute_with(|| {
        let me = caller();
        let data = encode_with_selector(
            compute_selector("payoutStakers(address,uint32)"),
            (Address(me), 0u32),
        );
        StakingPrecompileSet::new()
            .prepare_test(me, staking_addr(), data)
            .execute_reverts(|out| {
                let msg = alloc::string::String::from_utf8_lossy(out).to_lowercase();
                // pallet-staking returns InvalidEraToReward / NotStash; the
                // dispatch error is forwarded verbatim by precompile-utils.
                msg.contains("dispatched call failed")
            });
    });
}

#[test]
fn payout_stakers_by_page_reverts_when_no_era_reward_recorded() {
    new_test_ext().execute_with(|| {
        let me = caller();
        let data = encode_with_selector(
            compute_selector("payoutStakersByPage(address,uint32,uint32)"),
            (Address(me), 0u32, 0u32),
        );
        StakingPrecompileSet::new()
            .prepare_test(me, staking_addr(), data)
            .execute_reverts(|out| {
                alloc::string::String::from_utf8_lossy(out)
                    .to_lowercase()
                    .contains("dispatched call failed")
            });
    });
}

#[test]
fn reap_stash_dispatch_error_when_funded() {
    new_test_ext().execute_with(|| {
        let me = caller();
        force_bond(me, 1_000);
        let data = encode_with_selector(
            compute_selector("reapStash(address,uint32)"),
            (Address(me), 0u32),
        );
        StakingPrecompileSet::new()
            .prepare_test(me, staking_addr(), data)
            .execute_reverts(|out| {
                alloc::string::String::from_utf8_lossy(out)
                    .to_lowercase()
                    .contains("dispatched call failed")
            });
    });
}

// ---- pallet-error mapping ------------------------------------------------

#[test]
fn bond_twice_reverts_with_already_bonded() {
    new_test_ext().execute_with(|| {
        let me = caller();
        force_bond(me, 1_000);
        let data = encode_with_selector(
            compute_selector("bond(uint256,(uint8,address))"),
            (U256::from(500u128), rd(0, H160::zero())),
        );
        StakingPrecompileSet::new()
            .prepare_test(me, staking_addr(), data)
            .execute_reverts(|out| {
                let msg = alloc::string::String::from_utf8_lossy(out);
                msg.contains("AlreadyBonded") || msg.contains("Dispatched call failed")
            });
    });
}

#[test]
fn nominate_empty_targets_reverts() {
    new_test_ext().execute_with(|| {
        let me = caller();
        force_bond(me, 1_000);
        let targets: Vec<Address> = vec![];
        let data = encode_with_selector(compute_selector("nominate(address[])"), (targets,));
        StakingPrecompileSet::new()
            .prepare_test(me, staking_addr(), data)
            .execute_reverts(|out| {
                alloc::string::String::from_utf8_lossy(out).contains("Dispatched call failed")
            });
    });
}

#[test]
fn set_payee_invalid_kind_reverts() {
    new_test_ext().execute_with(|| {
        let me = caller();
        force_bond(me, 1_000);
        let data = encode_with_selector(
            compute_selector("setPayee((uint8,address))"),
            (rd(99, H160::zero()),),
        );
        StakingPrecompileSet::new()
            .prepare_test(me, staking_addr(), data)
            .execute_reverts(|out| {
                alloc::string::String::from_utf8_lossy(out).contains("invalid reward destination")
            });
    });
}

// ---- delegatecall guard ---------------------------------------------------

#[test]
fn bond_rejects_delegatecall() {
    new_test_ext().execute_with(|| {
        let original_caller = H160::from_low_u64_be(0xAA);
        let delegate_caller = H160::from_low_u64_be(0xBB);
        let mut handle = MockHandle::new(
            staking_addr(),
            fp_evm::Context {
                address: delegate_caller,
                caller: original_caller,
                apparent_value: U256::zero(),
            },
        );
        // Encode bond selector + payload manually and set `input`.
        let data = encode_with_selector(
            compute_selector("bond(uint256,(uint8,address))"),
            (U256::from(100u128), rd(0, H160::zero())),
        );
        handle.input = data;
        let r = <crate::StakingPrecompile<Runtime> as fp_evm::Precompile>::execute(&mut handle);
        let err = r.expect_err("bond must reject delegatecall");
        match err {
            fp_evm::PrecompileFailure::Revert { output, .. } => {
                let needle = b"DELEGATECALL/CALLCODE forbidden";
                assert!(output.windows(needle.len()).any(|w| w == needle));
            }
            other => panic!("expected Revert, got {other:?}"),
        }
    });
}

#[test]
fn nominate_rejects_delegatecall() {
    new_test_ext().execute_with(|| {
        let original_caller = H160::from_low_u64_be(0xAA);
        let delegate_caller = H160::from_low_u64_be(0xBB);
        let mut handle = MockHandle::new(
            staking_addr(),
            fp_evm::Context {
                address: delegate_caller,
                caller: original_caller,
                apparent_value: U256::zero(),
            },
        );
        let targets: Vec<Address> = vec![Address(H160::from_low_u64_be(2))];
        let data = encode_with_selector(compute_selector("nominate(address[])"), (targets,));
        handle.input = data;
        let r = <crate::StakingPrecompile<Runtime> as fp_evm::Precompile>::execute(&mut handle);
        match r.expect_err("nominate must reject delegatecall") {
            fp_evm::PrecompileFailure::Revert { output, .. } => {
                assert!(output
                    .windows(b"DELEGATECALL/CALLCODE forbidden".len())
                    .any(|w| w == b"DELEGATECALL/CALLCODE forbidden"));
            }
            other => panic!("expected Revert, got {other:?}"),
        }
    });
}

#[test]
fn payout_stakers_rejects_delegatecall() {
    new_test_ext().execute_with(|| {
        let original_caller = H160::from_low_u64_be(0xAA);
        let delegate_caller = H160::from_low_u64_be(0xBB);
        let mut handle = MockHandle::new(
            staking_addr(),
            fp_evm::Context {
                address: delegate_caller,
                caller: original_caller,
                apparent_value: U256::zero(),
            },
        );
        let data = encode_with_selector(
            compute_selector("payoutStakers(address,uint32)"),
            (Address(original_caller), 0u32),
        );
        handle.input = data;
        let r = <crate::StakingPrecompile<Runtime> as fp_evm::Precompile>::execute(&mut handle);
        match r.expect_err("payoutStakers must reject delegatecall") {
            fp_evm::PrecompileFailure::Revert { output, .. } => {
                assert!(output
                    .windows(b"DELEGATECALL/CALLCODE forbidden".len())
                    .any(|w| w == b"DELEGATECALL/CALLCODE forbidden"));
            }
            other => panic!("expected Revert, got {other:?}"),
        }
    });
}

// ---- views ----------------------------------------------------------------

#[test]
fn current_era_returns_zero_initially() {
    new_test_ext().execute_with(|| {
        let data = encode_with_selector(compute_selector("currentEra()"), ());
        StakingPrecompileSet::new()
            .prepare_test(caller(), staking_addr(), data)
            .execute_returns(0u32);
    });
}

#[test]
fn active_era_reads_pallet_storage() {
    new_test_ext().execute_with(|| {
        pallet_staking::ActiveEra::<Runtime>::put(pallet_staking::ActiveEraInfo {
            index: 7,
            start: Some(12345),
        });
        let data = encode_with_selector(compute_selector("activeEra()"), ());
        StakingPrecompileSet::new()
            .prepare_test(caller(), staking_addr(), data)
            .execute_returns((7u32, 12345u64));
    });
}

#[test]
fn min_nominator_bond_reads_storage() {
    new_test_ext().execute_with(|| {
        pallet_staking::MinNominatorBond::<Runtime>::put(42u128);
        let data = encode_with_selector(compute_selector("minNominatorBond()"), ());
        StakingPrecompileSet::new()
            .prepare_test(caller(), staking_addr(), data)
            .execute_returns(U256::from(42u128));
    });
}

#[test]
fn validator_count_reads_storage() {
    new_test_ext().execute_with(|| {
        pallet_staking::ValidatorCount::<Runtime>::put(11);
        let data = encode_with_selector(compute_selector("validatorCount()"), ());
        StakingPrecompileSet::new()
            .prepare_test(caller(), staking_addr(), data)
            .execute_returns(11u32);
    });
}

#[test]
fn validators_returns_prefs_after_validate() {
    new_test_ext().execute_with(|| {
        let v = caller();
        force_bond(v, 1_000);
        pallet_staking::Pallet::<Runtime>::validate(
            RawOrigin::Signed(v).into(),
            pallet_staking::ValidatorPrefs {
                commission: sp_runtime::Perbill::from_parts(300_000_000), // 30%
                blocked: true,
            },
        )
        .unwrap();
        let data = encode_with_selector(compute_selector("validators(address)"), (Address(v),));
        // Percent encoding: 300_000_000 / 10_000_000 = 30
        StakingPrecompileSet::new()
            .prepare_test(caller(), staking_addr(), data)
            .execute_returns((30u16, true));
    });
}

#[test]
fn bonded_returns_stash_when_bonded_zero_otherwise() {
    new_test_ext().execute_with(|| {
        let v = H160::from_low_u64_be(5);
        force_bond(v, 1_000);
        // bonded(v) → v (controller == stash in stable2603)
        let data = encode_with_selector(compute_selector("bonded(address)"), (Address(v),));
        StakingPrecompileSet::new()
            .prepare_test(caller(), staking_addr(), data)
            .execute_returns(Address(v));
        // bonded(unbonded) → zero
        let other = H160::from_low_u64_be(0xDEAD);
        let data = encode_with_selector(compute_selector("bonded(address)"), (Address(other),));
        StakingPrecompileSet::new()
            .prepare_test(caller(), staking_addr(), data)
            .execute_returns(Address(H160::zero()));
    });
}

#[test]
fn ledger_returns_active_total_and_unlocking_chunks() {
    new_test_ext().execute_with(|| {
        let me = caller();
        force_bond(me, 1_000);
        pallet_staking::Pallet::<Runtime>::unbond(RawOrigin::Signed(me).into(), 250).unwrap();
        let data = encode_with_selector(compute_selector("ledger(address)"), (Address(me),));
        let expected_active = U256::from(750u128);
        let expected_total = U256::from(1_000u128);
        let expected_chunks: Vec<UnlockingChunkSolidity> = vec![UnlockingChunkSolidity {
            era: 3, // unbond pushes chunk at current_era + BondingDuration (0 + 3)
            value: U256::from(250u128),
        }];
        StakingPrecompileSet::new()
            .prepare_test(caller(), staking_addr(), data)
            .execute_returns((expected_active, expected_total, expected_chunks));
    });
}

#[test]
fn nominators_returns_targets_after_nominate() {
    new_test_ext().execute_with(|| {
        let me = caller();
        let v = H160::from_low_u64_be(2);
        force_bond(v, 100);
        pallet_staking::Pallet::<Runtime>::validate(
            RawOrigin::Signed(v).into(),
            Default::default(),
        )
        .unwrap();
        force_bond(me, 1_000);
        pallet_staking::Pallet::<Runtime>::nominate(RawOrigin::Signed(me).into(), vec![v]).unwrap();
        let data = encode_with_selector(compute_selector("nominators(address)"), (Address(me),));
        StakingPrecompileSet::new()
            .prepare_test(caller(), staking_addr(), data)
            .execute_returns((vec![Address(v)], 0u32, false));
    });
}

#[test]
fn eras_validator_reward_zero_when_unset_else_value() {
    new_test_ext().execute_with(|| {
        // unset → 0
        let data = encode_with_selector(compute_selector("erasValidatorReward(uint32)"), (5u32,));
        StakingPrecompileSet::new()
            .prepare_test(caller(), staking_addr(), data)
            .execute_returns(U256::zero());
        // set → value
        pallet_staking::ErasValidatorReward::<Runtime>::insert(5u32, 7_777u128);
        let data = encode_with_selector(compute_selector("erasValidatorReward(uint32)"), (5u32,));
        StakingPrecompileSet::new()
            .prepare_test(caller(), staking_addr(), data)
            .execute_returns(U256::from(7_777u128));
    });
}

#[test]
fn eras_reward_points_returns_total_and_individual() {
    new_test_ext().execute_with(|| {
        let v1 = H160::from_low_u64_be(2);
        let v2 = H160::from_low_u64_be(3);
        let mut individual = alloc::collections::BTreeMap::new();
        individual.insert(v1, 10u32);
        individual.insert(v2, 20u32);
        pallet_staking::ErasRewardPoints::<Runtime>::insert(
            1u32,
            pallet_staking::EraRewardPoints::<H160> {
                total: 30,
                individual,
            },
        );
        let data = encode_with_selector(compute_selector("erasRewardPoints(uint32)"), (1u32,));
        // BTreeMap iterates in ascending key order; v1 < v2 numerically.
        let expected: (u32, Vec<IndividualPointsSolidity>) = (
            30u32,
            vec![
                IndividualPointsSolidity {
                    who: Address(v1),
                    points: 10,
                },
                IndividualPointsSolidity {
                    who: Address(v2),
                    points: 20,
                },
            ],
        );
        StakingPrecompileSet::new()
            .prepare_test(caller(), staking_addr(), data)
            .execute_returns(expected);
    });
}

#[test]
fn counter_for_validators_reads_count() {
    new_test_ext().execute_with(|| {
        let v1 = H160::from_low_u64_be(2);
        let v2 = H160::from_low_u64_be(3);
        force_bond(v1, 100);
        force_bond(v2, 100);
        pallet_staking::Pallet::<Runtime>::validate(
            RawOrigin::Signed(v1).into(),
            Default::default(),
        )
        .unwrap();
        pallet_staking::Pallet::<Runtime>::validate(
            RawOrigin::Signed(v2).into(),
            Default::default(),
        )
        .unwrap();
        let data = encode_with_selector(compute_selector("counterForValidators()"), ());
        StakingPrecompileSet::new()
            .prepare_test(caller(), staking_addr(), data)
            .execute_returns(2u32);
    });
}

#[test]
fn history_depth_returns_config_constant() {
    new_test_ext().execute_with(|| {
        // mock sets HistoryDepth = 84
        let data = encode_with_selector(compute_selector("historyDepth()"), ());
        StakingPrecompileSet::new()
            .prepare_test(caller(), staking_addr(), data)
            .execute_returns(84u32);
    });
}

#[test]
fn eras_stakers_reads_paged_storage() {
    new_test_ext().execute_with(|| {
        let era: u32 = 1;
        let validator = H160::from_low_u64_be(10);
        let n1 = H160::from_low_u64_be(11);
        let n2 = H160::from_low_u64_be(12);

        // Seed the paged-exposure overview (validator metadata at the era).
        let overview: sp_staking::PagedExposureMetadata<u128> = sp_staking::PagedExposureMetadata {
            total: 1_000u128,
            own: 600u128,
            nominator_count: 2,
            page_count: 1,
        };
        pallet_staking::ErasStakersOverview::<Runtime>::insert(era, validator, overview);

        // Seed a single page of nominator exposures (others) for that validator.
        let page_idx: u32 = 0;
        let page: sp_staking::ExposurePage<H160, u128> = sp_staking::ExposurePage {
            page_total: 400u128,
            others: vec![
                sp_staking::IndividualExposure {
                    who: n1,
                    value: 200u128,
                },
                sp_staking::IndividualExposure {
                    who: n2,
                    value: 200u128,
                },
            ],
        };
        pallet_staking::ErasStakersPaged::<Runtime>::insert((era, validator, page_idx), page);

        let data = encode_with_selector(
            compute_selector("erasStakers(uint32,address)"),
            (era, Address(validator)),
        );
        let expected_others: Vec<crate::IndividualExposureSolidity> = vec![
            crate::IndividualExposureSolidity {
                who: Address(n1),
                value: U256::from(200u128),
            },
            crate::IndividualExposureSolidity {
                who: Address(n2),
                value: U256::from(200u128),
            },
        ];
        StakingPrecompileSet::new()
            .prepare_test(caller(), staking_addr(), data)
            .execute_returns((U256::from(1_000u128), U256::from(600u128), expected_others));
    });
}

#[test]
fn min_validator_bond_reads_storage() {
    new_test_ext().execute_with(|| {
        pallet_staking::MinValidatorBond::<Runtime>::put(42_000u128);
        let data = encode_with_selector(compute_selector("minValidatorBond()"), ());
        StakingPrecompileSet::new()
            .prepare_test(caller(), staking_addr(), data)
            .execute_returns(U256::from(42_000u128));
    });
}

#[test]
fn min_active_stake_reads_storage() {
    new_test_ext().execute_with(|| {
        pallet_staking::MinimumActiveStake::<Runtime>::put(7_500u128);
        let data = encode_with_selector(compute_selector("minActiveStake()"), ());
        StakingPrecompileSet::new()
            .prepare_test(caller(), staking_addr(), data)
            .execute_returns(U256::from(7_500u128));
    });
}

#[test]
fn counter_for_nominators_reads_count() {
    new_test_ext().execute_with(|| {
        // Build state: two validators + two nominators targeting them.
        let v1 = H160::from_low_u64_be(2);
        let v2 = H160::from_low_u64_be(3);
        let n1 = H160::from_low_u64_be(4);
        let n2 = H160::from_low_u64_be(5);
        force_bond(v1, 100);
        force_bond(v2, 100);
        pallet_staking::Pallet::<Runtime>::validate(
            RawOrigin::Signed(v1).into(),
            Default::default(),
        )
        .unwrap();
        pallet_staking::Pallet::<Runtime>::validate(
            RawOrigin::Signed(v2).into(),
            Default::default(),
        )
        .unwrap();
        force_bond(n1, 500);
        force_bond(n2, 500);
        pallet_staking::Pallet::<Runtime>::nominate(RawOrigin::Signed(n1).into(), vec![v1])
            .unwrap();
        pallet_staking::Pallet::<Runtime>::nominate(RawOrigin::Signed(n2).into(), vec![v2])
            .unwrap();
        let data = encode_with_selector(compute_selector("counterForNominators()"), ());
        StakingPrecompileSet::new()
            .prepare_test(caller(), staking_addr(), data)
            .execute_returns(2u32);
    });
}

// ---- codec bounds --------------------------------------------------------

/// `nominate` declares `targets` as `BoundedVec<Address, GetMaxTargets>` where
/// `GetMaxTargets = MAX_TARGETS = 256`. Sending `MAX_TARGETS + 1 = 257`
/// targets must revert during codec decode before dispatch runs — the
/// precompile-utils `BoundedVec::read` impl surfaces a length-bound error.
#[test]
fn nominate_exceeds_max_targets_reverts_via_codec() {
    new_test_ext().execute_with(|| {
        let me = caller();
        force_bond(me, 1_000);
        let n = crate::MAX_TARGETS as usize + 1; // 257
        let targets: Vec<Address> = (0..n)
            .map(|i| Address(H160::from_low_u64_be(100 + i as u64)))
            .collect();
        let data = encode_with_selector(compute_selector("nominate(address[])"), (targets,));
        StakingPrecompileSet::new()
            .prepare_test(me, staking_addr(), data)
            .execute_reverts(|out| {
                let msg = alloc::string::String::from_utf8_lossy(out).to_lowercase();
                msg.contains("too large") || msg.contains("length") || msg.contains("exceed")
            });
    });
}
