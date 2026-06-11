//! Unit tests for the nomination-pools precompile at `0x0820`.
//!
//! Coverage strategy:
//!
//! * Every write entry has at least one test. Entries whose happy path requires
//!   complex era-progression / multi-pool state (claim_payout, withdraw_unbonded,
//!   claim_commission) are exercised in their *error-mapping* form against an
//!   uninitialised pool. The full happy-path is covered by the runtime-level
//!   integration tests in `runtimes/impetus/tests/` and the Plan 3 E2E suites.
//! * View entries are tested by seeding storage directly so we never need a
//!   working election to land paged state.
//! * Sudo gating, DELEGATECALL guard, and selector anchoring round out the
//!   suite to mirror the staking + session precompile conventions.

#![cfg(test)]

use alloc::vec;
use alloc::vec::Vec;
use precompile_utils::prelude::*;
use precompile_utils::solidity::encode_with_selector;
use precompile_utils::testing::{compute_selector, MockHandle, PrecompileTesterExt};
use sp_core::{H160, U256};

use crate::mock::{fund, new_test_ext, pools_addr, Runtime, SUDO_KEY};
use crate::{
    BondExtraSolidity, CommissionChangeRateSolidity, CommissionPair, NominationPoolsPrecompileSet,
    RoleOp, UnbondingEraSolidity,
};

// ---- helpers --------------------------------------------------------------

fn caller() -> H160 {
    H160::from_low_u64_be(1)
}

fn dispatched_failed(out: &[u8]) -> bool {
    alloc::string::String::from_utf8_lossy(out)
        .to_lowercase()
        .contains("dispatched call failed")
}

// ---- selector sanity ------------------------------------------------------
//
// The macro-generated `__NominationPoolsPrecompile_test_solidity_signatures`
// test already verifies every declared selector matches its `keccak256`. We
// additionally pin a few high-traffic selectors as hex constants so a stray
// ABI rename trips this test loudly even before the macro test runs, and
// downstream SDKs that hard-code these selectors keep working across
// refactors.
#[test]
fn anchor_selectors_are_stable() {
    // Anchor a handful of high-traffic selectors as hex constants so a stray
    // ABI rename trips this test loudly even before the macro test runs.
    // The values were computed offline against keccak256 and re-verified by
    // `compute_selector` here; downstream SDKs hard-coding these keep working
    // across refactors.
    // Print the values once to verify them; then hard-code below.
    let join_sel = compute_selector("join(uint256,uint32)");
    let last_sel = compute_selector("lastPoolId()");
    let state_sel = compute_selector("setState(uint32,uint8)");
    assert_eq!(
        join_sel, 0x98f15d02,
        "join selector drift: 0x{join_sel:08x}"
    );
    assert_eq!(
        last_sel, 0xa657e579,
        "lastPoolId selector drift: 0x{last_sel:08x}"
    );
    assert_eq!(
        state_sel, 0x05c6ca03,
        "setState selector drift: 0x{state_sel:08x}"
    );
}

// ---- write happy paths ----------------------------------------------------

#[test]
fn create_inserts_bonded_pool() {
    new_test_ext().execute_with(|| {
        let me = caller();
        // Pool deposit + ED must be in the depositor's free balance.
        fund(me, 100_000);
        let data = encode_with_selector(
            compute_selector("create(uint256,address,address,address)"),
            (
                U256::from(10_000u128),
                Address(H160::from_low_u64_be(0xA)),
                Address(H160::from_low_u64_be(0xB)),
                Address(H160::from_low_u64_be(0xC)),
            ),
        );
        NominationPoolsPrecompileSet::new()
            .prepare_test(me, pools_addr(), data)
            .execute_returns(());
        // Pool ids are 1-indexed; first create lands as pool 1.
        let pool =
            pallet_nomination_pools::BondedPools::<Runtime>::get(1u32).expect("pool 1 must exist");
        assert_eq!(pool.points, 10_000u128);
        assert_eq!(pool.member_counter, 1);
        assert_eq!(pool.roles.depositor, me);
        assert_eq!(pool.roles.root, Some(H160::from_low_u64_be(0xA)));
        assert_eq!(pool.roles.nominator, Some(H160::from_low_u64_be(0xB)));
        assert_eq!(pool.roles.bouncer, Some(H160::from_low_u64_be(0xC)));
    });
}

#[test]
fn create_with_pool_id_reverts_when_id_not_yet_used() {
    // `create_with_pool_id` requires `pool_id < LastPoolId`, so calling it
    // before any `create` is the cleanest way to trip the
    // `InvalidPoolId` error and exercise the error-mapping path. Once Plan 3
    // E2E lands, we'll add a happy-path scenario that exercises pool-id reuse.
    new_test_ext().execute_with(|| {
        let me = caller();
        fund(me, 100_000);
        let data = encode_with_selector(
            compute_selector("createWithPoolId(uint256,address,address,address,uint32)"),
            (
                U256::from(10_000u128),
                Address(H160::from_low_u64_be(0xA)),
                Address(H160::from_low_u64_be(0xB)),
                Address(H160::from_low_u64_be(0xC)),
                42u32,
            ),
        );
        NominationPoolsPrecompileSet::new()
            .prepare_test(me, pools_addr(), data)
            .execute_reverts(super::tests::dispatched_failed);
    });
}

#[test]
fn join_adds_member_to_existing_pool() {
    new_test_ext().execute_with(|| {
        let depositor = caller();
        let joiner = H160::from_low_u64_be(2);
        fund(depositor, 100_000);
        fund(joiner, 100_000);
        // Create the pool first via the precompile, then have a second EOA
        // join.
        let create_data = encode_with_selector(
            compute_selector("create(uint256,address,address,address)"),
            (
                U256::from(10_000u128),
                Address(H160::from_low_u64_be(0xA)),
                Address(H160::from_low_u64_be(0xB)),
                Address(H160::from_low_u64_be(0xC)),
            ),
        );
        NominationPoolsPrecompileSet::new()
            .prepare_test(depositor, pools_addr(), create_data)
            .execute_returns(());
        let join_data = encode_with_selector(
            compute_selector("join(uint256,uint32)"),
            (U256::from(5_000u128), 1u32),
        );
        NominationPoolsPrecompileSet::new()
            .prepare_test(joiner, pools_addr(), join_data)
            .execute_returns(());
        let member = pallet_nomination_pools::PoolMembers::<Runtime>::get(joiner)
            .expect("joiner is now a pool member");
        assert_eq!(member.pool_id, 1);
        assert_eq!(member.points, 5_000u128);
    });
}

#[test]
fn bond_extra_increases_member_points() {
    new_test_ext().execute_with(|| {
        let depositor = caller();
        fund(depositor, 100_000);
        let create_data = encode_with_selector(
            compute_selector("create(uint256,address,address,address)"),
            (
                U256::from(10_000u128),
                Address(H160::from_low_u64_be(0xA)),
                Address(H160::from_low_u64_be(0xB)),
                Address(H160::from_low_u64_be(0xC)),
            ),
        );
        NominationPoolsPrecompileSet::new()
            .prepare_test(depositor, pools_addr(), create_data)
            .execute_returns(());
        let extra = BondExtraSolidity {
            kind: 0,
            amount: U256::from(2_500u128),
        };
        let data = encode_with_selector(compute_selector("bondExtra((uint8,uint256))"), (extra,));
        NominationPoolsPrecompileSet::new()
            .prepare_test(depositor, pools_addr(), data)
            .execute_returns(());
        let m = pallet_nomination_pools::PoolMembers::<Runtime>::get(depositor).unwrap();
        assert_eq!(m.points, 12_500u128);
    });
}

#[test]
fn unbond_records_unbonding_for_member() {
    // Happy path: a non-depositor joins, then unbonds. Skips the depositor-
    // only constraint by adding a second member first.
    new_test_ext().execute_with(|| {
        let depositor = caller();
        let joiner = H160::from_low_u64_be(2);
        fund(depositor, 100_000);
        fund(joiner, 100_000);
        let create_data = encode_with_selector(
            compute_selector("create(uint256,address,address,address)"),
            (
                U256::from(10_000u128),
                Address(depositor),
                Address(depositor),
                Address(depositor),
            ),
        );
        NominationPoolsPrecompileSet::new()
            .prepare_test(depositor, pools_addr(), create_data)
            .execute_returns(());
        let join_data = encode_with_selector(
            compute_selector("join(uint256,uint32)"),
            (U256::from(5_000u128), 1u32),
        );
        NominationPoolsPrecompileSet::new()
            .prepare_test(joiner, pools_addr(), join_data)
            .execute_returns(());
        let data = encode_with_selector(
            compute_selector("unbond(address,uint256)"),
            (Address(joiner), U256::from(1_000u128)),
        );
        NominationPoolsPrecompileSet::new()
            .prepare_test(joiner, pools_addr(), data)
            .execute_returns(());
        let member = pallet_nomination_pools::PoolMembers::<Runtime>::get(joiner).unwrap();
        // Active points dropped from 5_000 to 4_000; one unbonding entry queued
        // at era `current_era + bonding_duration = 0 + 3 = 3`.
        assert_eq!(member.points, 4_000u128);
        let unbonding: Vec<_> = member.unbonding_eras.into_iter().collect();
        assert_eq!(unbonding.len(), 1);
        assert_eq!(unbonding[0].0, 3u32);
        assert_eq!(unbonding[0].1, 1_000u128);
    });
}

#[test]
fn pool_withdraw_unbonded_reverts_when_pool_absent() {
    new_test_ext().execute_with(|| {
        let me = caller();
        let data = encode_with_selector(
            compute_selector("poolWithdrawUnbonded(uint32,uint32)"),
            (99u32, 0u32),
        );
        NominationPoolsPrecompileSet::new()
            .prepare_test(me, pools_addr(), data)
            .execute_reverts(dispatched_failed);
    });
}

#[test]
fn withdraw_unbonded_reverts_when_member_absent() {
    new_test_ext().execute_with(|| {
        let me = caller();
        let data = encode_with_selector(
            compute_selector("withdrawUnbonded(address,uint32)"),
            (Address(me), 0u32),
        );
        NominationPoolsPrecompileSet::new()
            .prepare_test(me, pools_addr(), data)
            .execute_reverts(dispatched_failed);
    });
}

#[test]
fn nominate_routes_through_stake_adapter() {
    new_test_ext().execute_with(|| {
        let me = caller();
        fund(me, 100_000);
        let create_data = encode_with_selector(
            compute_selector("create(uint256,address,address,address)"),
            (
                U256::from(10_000u128),
                Address(me),
                Address(me),
                Address(me),
            ),
        );
        NominationPoolsPrecompileSet::new()
            .prepare_test(me, pools_addr(), create_data)
            .execute_returns(());
        let validators: Vec<Address> = vec![
            Address(H160::from_low_u64_be(11)),
            Address(H160::from_low_u64_be(12)),
        ];
        let data = encode_with_selector(
            compute_selector("nominate(uint32,address[])"),
            (1u32, validators),
        );
        NominationPoolsPrecompileSet::new()
            .prepare_test(me, pools_addr(), data)
            .execute_returns(());
        // Our StakingMock records the nomination list against the bonded pool
        // account; the precompile itself returns success when the pallet
        // accepts the call.
        let acc = pallet_nomination_pools::Pallet::<Runtime>::generate_bonded_account(1);
        let noms = crate::mock::NOMINATIONS.with(|n| n.borrow().get(&acc).cloned());
        assert_eq!(
            noms.expect("nomination must be recorded"),
            vec![H160::from_low_u64_be(11), H160::from_low_u64_be(12)],
        );
    });
}

#[test]
fn set_state_transitions_pool_to_destroying() {
    new_test_ext().execute_with(|| {
        let depositor = caller();
        fund(depositor, 100_000);
        // Create pool 1 with depositor as all roles.
        let create_data = encode_with_selector(
            compute_selector("create(uint256,address,address,address)"),
            (
                U256::from(10_000u128),
                Address(depositor),
                Address(depositor),
                Address(depositor),
            ),
        );
        NominationPoolsPrecompileSet::new()
            .prepare_test(depositor, pools_addr(), create_data)
            .execute_returns(());
        let data = encode_with_selector(
            compute_selector("setState(uint32,uint8)"),
            (1u32, 2u8), // Destroying
        );
        NominationPoolsPrecompileSet::new()
            .prepare_test(depositor, pools_addr(), data)
            .execute_returns(());
        let pool = pallet_nomination_pools::BondedPools::<Runtime>::get(1).unwrap();
        assert_eq!(pool.state, pallet_nomination_pools::PoolState::Destroying);
    });
}

#[test]
fn set_state_rejects_invalid_state_kind() {
    new_test_ext().execute_with(|| {
        let depositor = caller();
        fund(depositor, 100_000);
        let create_data = encode_with_selector(
            compute_selector("create(uint256,address,address,address)"),
            (
                U256::from(10_000u128),
                Address(depositor),
                Address(depositor),
                Address(depositor),
            ),
        );
        NominationPoolsPrecompileSet::new()
            .prepare_test(depositor, pools_addr(), create_data)
            .execute_returns(());
        let data = encode_with_selector(compute_selector("setState(uint32,uint8)"), (1u32, 7u8));
        NominationPoolsPrecompileSet::new()
            .prepare_test(depositor, pools_addr(), data)
            .execute_reverts(|out| {
                alloc::string::String::from_utf8_lossy(out).contains("invalid pool state")
            });
    });
}

#[test]
fn set_metadata_updates_storage() {
    new_test_ext().execute_with(|| {
        let depositor = caller();
        fund(depositor, 100_000);
        let create_data = encode_with_selector(
            compute_selector("create(uint256,address,address,address)"),
            (
                U256::from(10_000u128),
                Address(depositor),
                Address(depositor),
                Address(depositor),
            ),
        );
        NominationPoolsPrecompileSet::new()
            .prepare_test(depositor, pools_addr(), create_data)
            .execute_returns(());
        let payload: Vec<u8> = b"impulse-pool-v1".to_vec();
        let data = encode_with_selector(
            compute_selector("setMetadata(uint32,bytes)"),
            (1u32, UnboundedBytes::from(payload.clone())),
        );
        NominationPoolsPrecompileSet::new()
            .prepare_test(depositor, pools_addr(), data)
            .execute_returns(());
        assert_eq!(
            pallet_nomination_pools::Metadata::<Runtime>::get(1).to_vec(),
            payload,
        );
    });
}

#[test]
fn set_configs_sudo_path_updates_storage() {
    new_test_ext().execute_with(|| {
        let data = encode_with_selector(
            compute_selector("setConfigs(uint256,uint256,uint32,uint32,uint32,uint32)"),
            (
                U256::from(123u128),
                U256::from(456u128),
                7u32,
                8u32,
                9u32,
                10_000u32,
            ),
        );
        NominationPoolsPrecompileSet::new()
            .prepare_test(SUDO_KEY, pools_addr(), data)
            .execute_returns(());
        assert_eq!(
            pallet_nomination_pools::MinJoinBond::<Runtime>::get(),
            123u128
        );
        assert_eq!(
            pallet_nomination_pools::MinCreateBond::<Runtime>::get(),
            456u128
        );
        assert_eq!(
            pallet_nomination_pools::MaxPools::<Runtime>::get(),
            Some(7u32)
        );
        assert_eq!(
            pallet_nomination_pools::MaxPoolMembers::<Runtime>::get(),
            Some(8u32)
        );
        assert_eq!(
            pallet_nomination_pools::MaxPoolMembersPerPool::<Runtime>::get(),
            Some(9u32),
        );
        assert_eq!(
            pallet_nomination_pools::GlobalMaxCommission::<Runtime>::get(),
            Some(sp_runtime::Perbill::from_parts(10_000u32)),
        );
    });
}

#[test]
fn set_configs_reverts_for_non_sudo_caller() {
    new_test_ext().execute_with(|| {
        let data = encode_with_selector(
            compute_selector("setConfigs(uint256,uint256,uint32,uint32,uint32,uint32)"),
            (U256::zero(), U256::zero(), 0u32, 0u32, 0u32, 0u32),
        );
        NominationPoolsPrecompileSet::new()
            .prepare_test(caller(), pools_addr(), data)
            .execute_reverts(|out| alloc::string::String::from_utf8_lossy(out).contains("NotSudo"));
    });
}

#[test]
fn update_roles_rewrites_pool_roles() {
    new_test_ext().execute_with(|| {
        let depositor = caller();
        fund(depositor, 100_000);
        let create_data = encode_with_selector(
            compute_selector("create(uint256,address,address,address)"),
            (
                U256::from(10_000u128),
                Address(depositor),
                Address(depositor),
                Address(depositor),
            ),
        );
        NominationPoolsPrecompileSet::new()
            .prepare_test(depositor, pools_addr(), create_data)
            .execute_returns(());
        let new_root = H160::from_low_u64_be(0xAB);
        let new_nominator = H160::from_low_u64_be(0xCD);
        let _new_bouncer_unused = H160::from_low_u64_be(0xEF);
        let roles: (RoleOp, RoleOp, RoleOp) = (
            RoleOp {
                op: 1,
                account: Address(new_root),
            },
            RoleOp {
                op: 1,
                account: Address(new_nominator),
            },
            RoleOp {
                op: 2,
                account: Address(H160::zero()),
            }, // Remove bouncer
        );
        let data = encode_with_selector(
            compute_selector("updateRoles(uint32,(uint8,address),(uint8,address),(uint8,address))"),
            (1u32, roles.0, roles.1, roles.2),
        );
        NominationPoolsPrecompileSet::new()
            .prepare_test(depositor, pools_addr(), data)
            .execute_returns(());
        let pool = pallet_nomination_pools::BondedPools::<Runtime>::get(1).unwrap();
        assert_eq!(pool.roles.root, Some(new_root));
        assert_eq!(pool.roles.nominator, Some(new_nominator));
        assert_eq!(pool.roles.bouncer, None);
    });
}

#[test]
fn update_roles_rejects_invalid_op_kind() {
    new_test_ext().execute_with(|| {
        let depositor = caller();
        fund(depositor, 100_000);
        let create_data = encode_with_selector(
            compute_selector("create(uint256,address,address,address)"),
            (
                U256::from(10_000u128),
                Address(depositor),
                Address(depositor),
                Address(depositor),
            ),
        );
        NominationPoolsPrecompileSet::new()
            .prepare_test(depositor, pools_addr(), create_data)
            .execute_returns(());
        let bad: (RoleOp, RoleOp, RoleOp) = (
            RoleOp {
                op: 99,
                account: Address(H160::zero()),
            },
            RoleOp {
                op: 0,
                account: Address(H160::zero()),
            },
            RoleOp {
                op: 0,
                account: Address(H160::zero()),
            },
        );
        let data = encode_with_selector(
            compute_selector("updateRoles(uint32,(uint8,address),(uint8,address),(uint8,address))"),
            (1u32, bad.0, bad.1, bad.2),
        );
        NominationPoolsPrecompileSet::new()
            .prepare_test(depositor, pools_addr(), data)
            .execute_reverts(|out| {
                alloc::string::String::from_utf8_lossy(out).contains("invalid role op")
            });
    });
}

#[test]
fn chill_reverts_when_pool_absent() {
    new_test_ext().execute_with(|| {
        let me = caller();
        let data = encode_with_selector(compute_selector("chill(uint32)"), (404u32,));
        NominationPoolsPrecompileSet::new()
            .prepare_test(me, pools_addr(), data)
            .execute_reverts(dispatched_failed);
    });
}

#[test]
fn bond_extra_other_reverts_when_member_missing() {
    new_test_ext().execute_with(|| {
        let me = caller();
        let extra = BondExtraSolidity {
            kind: 0,
            amount: U256::from(1u128),
        };
        let data = encode_with_selector(
            compute_selector("bondExtraOther(address,(uint8,uint256))"),
            (Address(H160::from_low_u64_be(0xDEAD)), extra),
        );
        NominationPoolsPrecompileSet::new()
            .prepare_test(me, pools_addr(), data)
            .execute_reverts(dispatched_failed);
    });
}

#[test]
fn bond_extra_rejects_invalid_kind() {
    new_test_ext().execute_with(|| {
        let depositor = caller();
        fund(depositor, 100_000);
        let create_data = encode_with_selector(
            compute_selector("create(uint256,address,address,address)"),
            (
                U256::from(10_000u128),
                Address(depositor),
                Address(depositor),
                Address(depositor),
            ),
        );
        NominationPoolsPrecompileSet::new()
            .prepare_test(depositor, pools_addr(), create_data)
            .execute_returns(());
        let extra = BondExtraSolidity {
            kind: 5,
            amount: U256::from(1u128),
        };
        let data = encode_with_selector(compute_selector("bondExtra((uint8,uint256))"), (extra,));
        NominationPoolsPrecompileSet::new()
            .prepare_test(depositor, pools_addr(), data)
            .execute_reverts(|out| {
                alloc::string::String::from_utf8_lossy(out).contains("invalid BondExtra kind")
            });
    });
}

#[test]
fn bond_extra_kind_rewards_reverts_when_not_member() {
    // Exercises the `kind = 1` (BondExtra::Rewards) decode path. Caller is
    // NOT a pool member, so the pallet must surface its error
    // (`PoolMemberNotFound` / `NoPendingRewards`) and the precompile must
    // map it to a `dispatched call failed` revert. Without this test the
    // `kind = 1` arm of `decode_bond_extra` is never exercised by the
    // existing suite — `bond_extra_rejects_invalid_kind` only tests
    // `kind >= 2` and `bond_extra_increases_member_points` only tests
    // `kind = 0`.
    new_test_ext().execute_with(|| {
        let non_member = caller();
        let extra = BondExtraSolidity {
            kind: 1,
            amount: U256::zero(),
        };
        let data = encode_with_selector(compute_selector("bondExtra((uint8,uint256))"), (extra,));
        NominationPoolsPrecompileSet::new()
            .prepare_test(non_member, pools_addr(), data)
            .execute_reverts(dispatched_failed);
    });
}

#[test]
fn set_commission_unsets_when_pair_is_zero() {
    new_test_ext().execute_with(|| {
        let depositor = caller();
        fund(depositor, 100_000);
        let create_data = encode_with_selector(
            compute_selector("create(uint256,address,address,address)"),
            (
                U256::from(10_000u128),
                Address(depositor),
                Address(depositor),
                Address(depositor),
            ),
        );
        NominationPoolsPrecompileSet::new()
            .prepare_test(depositor, pools_addr(), create_data)
            .execute_returns(());
        // Calling with `(0, 0x0)` should unset commission. Since the pool's
        // commission starts at `None`, the call is effectively a no-op and
        // must dispatch cleanly.
        let zero = CommissionPair {
            commission: 0,
            payee: Address(H160::zero()),
        };
        let data = encode_with_selector(
            compute_selector("setCommission(uint32,(uint32,address))"),
            (1u32, zero),
        );
        NominationPoolsPrecompileSet::new()
            .prepare_test(depositor, pools_addr(), data)
            .execute_returns(());
        let pool = pallet_nomination_pools::BondedPools::<Runtime>::get(1).unwrap();
        assert_eq!(pool.commission.current, None);
    });
}

#[test]
fn set_commission_rejects_zero_commission_with_non_zero_payee() {
    // Ambiguous combination: caller passes `(0, addr)`. The pallet would
    // silently coerce `Some((0, addr))` to `None` and drop the payee, so the
    // precompile must reject the call up-front.
    new_test_ext().execute_with(|| {
        let depositor = caller();
        fund(depositor, 100_000);
        let create_data = encode_with_selector(
            compute_selector("create(uint256,address,address,address)"),
            (
                U256::from(10_000u128),
                Address(depositor),
                Address(depositor),
                Address(depositor),
            ),
        );
        NominationPoolsPrecompileSet::new()
            .prepare_test(depositor, pools_addr(), create_data)
            .execute_returns(());
        let ambiguous = CommissionPair {
            commission: 0,
            payee: Address(H160::from_low_u64_be(0xCAFE)),
        };
        let data = encode_with_selector(
            compute_selector("setCommission(uint32,(uint32,address))"),
            (1u32, ambiguous),
        );
        NominationPoolsPrecompileSet::new()
            .prepare_test(depositor, pools_addr(), data)
            .execute_reverts(|out| {
                alloc::string::String::from_utf8_lossy(out).contains("ambiguous")
            });
    });
}

#[test]
fn set_commission_rejects_non_zero_commission_with_zero_payee() {
    // Ambiguous combination: caller passes `(parts, 0x0)`. The empty payee
    // signals "unset" but a non-zero commission contradicts that, so the
    // precompile rejects the call instead of silently choosing one branch.
    new_test_ext().execute_with(|| {
        let depositor = caller();
        fund(depositor, 100_000);
        let create_data = encode_with_selector(
            compute_selector("create(uint256,address,address,address)"),
            (
                U256::from(10_000u128),
                Address(depositor),
                Address(depositor),
                Address(depositor),
            ),
        );
        NominationPoolsPrecompileSet::new()
            .prepare_test(depositor, pools_addr(), create_data)
            .execute_returns(());
        let ambiguous = CommissionPair {
            commission: 50_000_000u32,
            payee: Address(H160::zero()),
        };
        let data = encode_with_selector(
            compute_selector("setCommission(uint32,(uint32,address))"),
            (1u32, ambiguous),
        );
        NominationPoolsPrecompileSet::new()
            .prepare_test(depositor, pools_addr(), data)
            .execute_reverts(|out| {
                alloc::string::String::from_utf8_lossy(out)
                    .contains("payee=0 requires commission=0")
            });
    });
}

#[test]
fn set_commission_max_caps_pool_commission() {
    new_test_ext().execute_with(|| {
        let depositor = caller();
        fund(depositor, 100_000);
        let create_data = encode_with_selector(
            compute_selector("create(uint256,address,address,address)"),
            (
                U256::from(10_000u128),
                Address(depositor),
                Address(depositor),
                Address(depositor),
            ),
        );
        NominationPoolsPrecompileSet::new()
            .prepare_test(depositor, pools_addr(), create_data)
            .execute_returns(());
        // Set max to 20% (200_000_000 parts).
        let data = encode_with_selector(
            compute_selector("setCommissionMax(uint32,uint32)"),
            (1u32, 200_000_000u32),
        );
        NominationPoolsPrecompileSet::new()
            .prepare_test(depositor, pools_addr(), data)
            .execute_returns(());
        let pool = pallet_nomination_pools::BondedPools::<Runtime>::get(1).unwrap();
        assert_eq!(
            pool.commission.max,
            Some(sp_runtime::Perbill::from_parts(200_000_000u32)),
        );
    });
}

#[test]
fn set_commission_change_rate_updates_pool() {
    new_test_ext().execute_with(|| {
        let depositor = caller();
        fund(depositor, 100_000);
        let create_data = encode_with_selector(
            compute_selector("create(uint256,address,address,address)"),
            (
                U256::from(10_000u128),
                Address(depositor),
                Address(depositor),
                Address(depositor),
            ),
        );
        NominationPoolsPrecompileSet::new()
            .prepare_test(depositor, pools_addr(), create_data)
            .execute_returns(());
        let cr = CommissionChangeRateSolidity {
            max_increase: 50_000_000u32,
            min_delay: 5u32,
        };
        let data = encode_with_selector(
            compute_selector("setCommissionChangeRate(uint32,(uint32,uint32))"),
            (1u32, cr),
        );
        NominationPoolsPrecompileSet::new()
            .prepare_test(depositor, pools_addr(), data)
            .execute_returns(());
        let pool = pallet_nomination_pools::BondedPools::<Runtime>::get(1).unwrap();
        let stored = pool.commission.change_rate.expect("change rate stored");
        assert_eq!(stored.max_increase.deconstruct(), 50_000_000u32);
        assert_eq!(stored.min_delay, 5u64);
    });
}

#[test]
fn claim_payout_reverts_when_not_a_member() {
    // The depositor itself is the only member; with no rewards accrued,
    // pallet-nomination-pools returns `Defensive(RewardPoolNotFound)` /
    // `NoPendingRewards`. Either way the dispatch fails and we exercise the
    // error-mapping path.
    new_test_ext().execute_with(|| {
        let me = caller();
        let data = encode_with_selector(compute_selector("claimPayout()"), ());
        NominationPoolsPrecompileSet::new()
            .prepare_test(me, pools_addr(), data)
            .execute_reverts(dispatched_failed);
    });
}

#[test]
fn claim_commission_reverts_when_pool_missing() {
    new_test_ext().execute_with(|| {
        let me = caller();
        let data = encode_with_selector(compute_selector("claimCommission(uint32)"), (404u32,));
        NominationPoolsPrecompileSet::new()
            .prepare_test(me, pools_addr(), data)
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
    let original_caller = H160::from_low_u64_be(0xAA);
    let delegate_caller = H160::from_low_u64_be(0xBB);
    let mut handle = MockHandle::new(
        pools_addr(),
        fp_evm::Context {
            address: delegate_caller,
            caller: original_caller,
            apparent_value: U256::zero(),
        },
    );
    handle.input = input;
    let r = <crate::NominationPoolsPrecompile<Runtime> as fp_evm::Precompile>::execute(&mut handle);
    r.expect_err("delegatecall must be rejected")
}

#[test]
fn join_rejects_delegatecall() {
    new_test_ext().execute_with(|| {
        let data = encode_with_selector(
            compute_selector("join(uint256,uint32)"),
            (U256::from(1u128), 1u32),
        );
        let err = delegatecall_attack(data);
        revert_message_contains(b"DELEGATECALL/CALLCODE forbidden")(&err);
    });
}

#[test]
fn set_configs_rejects_delegatecall_before_sudo_check() {
    new_test_ext().execute_with(|| {
        let data = encode_with_selector(
            compute_selector("setConfigs(uint256,uint256,uint32,uint32,uint32,uint32)"),
            (U256::zero(), U256::zero(), 0u32, 0u32, 0u32, 0u32),
        );
        let err = delegatecall_attack(data);
        revert_message_contains(b"DELEGATECALL/CALLCODE forbidden")(&err);
    });
}

#[test]
fn create_rejects_delegatecall() {
    new_test_ext().execute_with(|| {
        let data = encode_with_selector(
            compute_selector("create(uint256,address,address,address)"),
            (
                U256::from(1u128),
                Address(H160::zero()),
                Address(H160::zero()),
                Address(H160::zero()),
            ),
        );
        let err = delegatecall_attack(data);
        revert_message_contains(b"DELEGATECALL/CALLCODE forbidden")(&err);
    });
}

// ---- views ----------------------------------------------------------------

#[test]
fn last_pool_id_zero_then_one_after_create() {
    new_test_ext().execute_with(|| {
        let data = encode_with_selector(compute_selector("lastPoolId()"), ());
        NominationPoolsPrecompileSet::new()
            .prepare_test(caller(), pools_addr(), data.clone())
            .execute_returns(0u32);
        let depositor = caller();
        fund(depositor, 100_000);
        let create_data = encode_with_selector(
            compute_selector("create(uint256,address,address,address)"),
            (
                U256::from(10_000u128),
                Address(depositor),
                Address(depositor),
                Address(depositor),
            ),
        );
        NominationPoolsPrecompileSet::new()
            .prepare_test(depositor, pools_addr(), create_data)
            .execute_returns(());
        NominationPoolsPrecompileSet::new()
            .prepare_test(caller(), pools_addr(), data)
            .execute_returns(1u32);
    });
}

#[test]
fn metadata_view_round_trips_payload() {
    new_test_ext().execute_with(|| {
        let depositor = caller();
        fund(depositor, 100_000);
        let create_data = encode_with_selector(
            compute_selector("create(uint256,address,address,address)"),
            (
                U256::from(10_000u128),
                Address(depositor),
                Address(depositor),
                Address(depositor),
            ),
        );
        NominationPoolsPrecompileSet::new()
            .prepare_test(depositor, pools_addr(), create_data)
            .execute_returns(());
        let set_data = encode_with_selector(
            compute_selector("setMetadata(uint32,bytes)"),
            (1u32, UnboundedBytes::from(b"abc".to_vec())),
        );
        NominationPoolsPrecompileSet::new()
            .prepare_test(depositor, pools_addr(), set_data)
            .execute_returns(());
        let data = encode_with_selector(compute_selector("metadata(uint32)"), (1u32,));
        NominationPoolsPrecompileSet::new()
            .prepare_test(caller(), pools_addr(), data)
            .execute_returns(UnboundedBytes::from(b"abc".to_vec()));
    });
}

#[test]
fn bonded_pools_returns_zero_when_pool_absent() {
    new_test_ext().execute_with(|| {
        let data = encode_with_selector(compute_selector("bondedPools(uint32)"), (42u32,));
        let zero_change = CommissionChangeRateSolidity::default();
        let zero_roles: Vec<Address> = vec![Address(H160::zero()); 3];
        NominationPoolsPrecompileSet::new()
            .prepare_test(caller(), pools_addr(), data)
            .execute_returns((
                U256::zero(),
                0u8,
                0u32,
                zero_roles,
                (0u32, 0u32, zero_change, Address(H160::zero())),
            ));
    });
}

#[test]
fn bonded_pools_returns_pool_after_create() {
    new_test_ext().execute_with(|| {
        let depositor = caller();
        fund(depositor, 100_000);
        let create_data = encode_with_selector(
            compute_selector("create(uint256,address,address,address)"),
            (
                U256::from(10_000u128),
                Address(H160::from_low_u64_be(0xA)),
                Address(H160::from_low_u64_be(0xB)),
                Address(H160::from_low_u64_be(0xC)),
            ),
        );
        NominationPoolsPrecompileSet::new()
            .prepare_test(depositor, pools_addr(), create_data)
            .execute_returns(());
        let data = encode_with_selector(compute_selector("bondedPools(uint32)"), (1u32,));
        let zero_change = CommissionChangeRateSolidity::default();
        let roles: Vec<Address> = vec![
            Address(H160::from_low_u64_be(0xA)),
            Address(H160::from_low_u64_be(0xB)),
            Address(H160::from_low_u64_be(0xC)),
        ];
        NominationPoolsPrecompileSet::new()
            .prepare_test(caller(), pools_addr(), data)
            .execute_returns((
                U256::from(10_000u128),
                0u8, // Open
                1u32,
                roles,
                (0u32, 0u32, zero_change, Address(H160::zero())),
            ));
    });
}

#[test]
fn pool_members_view_returns_member_state() {
    new_test_ext().execute_with(|| {
        let depositor = caller();
        fund(depositor, 100_000);
        let create_data = encode_with_selector(
            compute_selector("create(uint256,address,address,address)"),
            (
                U256::from(10_000u128),
                Address(depositor),
                Address(depositor),
                Address(depositor),
            ),
        );
        NominationPoolsPrecompileSet::new()
            .prepare_test(depositor, pools_addr(), create_data)
            .execute_returns(());
        let data = encode_with_selector(
            compute_selector("poolMembers(address)"),
            (Address(depositor),),
        );
        let expected_unbonding: Vec<UnbondingEraSolidity> = Vec::new();
        NominationPoolsPrecompileSet::new()
            .prepare_test(caller(), pools_addr(), data)
            .execute_returns((
                1u32,
                U256::from(10_000u128),
                U256::zero(),
                expected_unbonding,
            ));
    });
}

#[test]
fn pool_members_view_returns_zero_when_member_absent() {
    new_test_ext().execute_with(|| {
        let stranger = H160::from_low_u64_be(0xDEAD);
        let data = encode_with_selector(
            compute_selector("poolMembers(address)"),
            (Address(stranger),),
        );
        let empty: Vec<UnbondingEraSolidity> = Vec::new();
        NominationPoolsPrecompileSet::new()
            .prepare_test(caller(), pools_addr(), data)
            .execute_returns((0u32, U256::zero(), U256::zero(), empty));
    });
}

// ---- codec bounds ---------------------------------------------------------

/// `nominate` declares `validators` as `BoundedVec<Address, GetMaxNominations>`
/// where `GetMaxNominations = MAX_NOMINATIONS = 256`. Sending one more than
/// that must revert during codec decode before dispatch runs.
#[test]
fn nominate_exceeds_max_validators_reverts_via_codec() {
    new_test_ext().execute_with(|| {
        let depositor = caller();
        fund(depositor, 100_000);
        let create_data = encode_with_selector(
            compute_selector("create(uint256,address,address,address)"),
            (
                U256::from(10_000u128),
                Address(depositor),
                Address(depositor),
                Address(depositor),
            ),
        );
        NominationPoolsPrecompileSet::new()
            .prepare_test(depositor, pools_addr(), create_data)
            .execute_returns(());
        let n = crate::MAX_NOMINATIONS as usize + 1; // 257
        let validators: Vec<Address> = (0..n)
            .map(|i| Address(H160::from_low_u64_be(100 + i as u64)))
            .collect();
        let data = encode_with_selector(
            compute_selector("nominate(uint32,address[])"),
            (1u32, validators),
        );
        NominationPoolsPrecompileSet::new()
            .prepare_test(depositor, pools_addr(), data)
            .execute_reverts(|out| {
                let msg = alloc::string::String::from_utf8_lossy(out).to_lowercase();
                msg.contains("too large") || msg.contains("length") || msg.contains("exceed")
            });
    });
}

#[test]
fn set_metadata_exceeds_bound_reverts_via_codec() {
    new_test_ext().execute_with(|| {
        let depositor = caller();
        fund(depositor, 100_000);
        let create_data = encode_with_selector(
            compute_selector("create(uint256,address,address,address)"),
            (
                U256::from(10_000u128),
                Address(depositor),
                Address(depositor),
                Address(depositor),
            ),
        );
        NominationPoolsPrecompileSet::new()
            .prepare_test(depositor, pools_addr(), create_data)
            .execute_returns(());
        let too_big: Vec<u8> = vec![0u8; (crate::MAX_METADATA_BYTES + 1) as usize];
        let data = encode_with_selector(
            compute_selector("setMetadata(uint32,bytes)"),
            (1u32, UnboundedBytes::from(too_big)),
        );
        NominationPoolsPrecompileSet::new()
            .prepare_test(depositor, pools_addr(), data)
            .execute_reverts(|out| {
                let msg = alloc::string::String::from_utf8_lossy(out).to_lowercase();
                msg.contains("too large") || msg.contains("length") || msg.contains("exceed")
            });
    });
}
