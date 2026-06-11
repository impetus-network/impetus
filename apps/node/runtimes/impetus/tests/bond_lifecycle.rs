mod common;
use common::*;

use frame_support::assert_ok;
use impetus_runtime::{BondingDuration, Runtime, RuntimeOrigin, Staking, UNIT};

#[test]
fn bond_extra_increases_ledger_active() {
    ExtBuilder::default().build().execute_with(|| {
        let stash = account(ALICE);
        let pre = pallet_staking::Ledger::<Runtime>::get(stash).unwrap().active;
        assert_ok!(Staking::bond_extra(RuntimeOrigin::signed(stash), 500 * UNIT));
        let post = pallet_staking::Ledger::<Runtime>::get(stash).unwrap().active;
        assert_eq!(post, pre + 500 * UNIT);
    });
}

#[test]
fn unbond_then_withdraw_after_bonding_duration() {
    ExtBuilder::default().build().execute_with(|| {
        let stash = account(ALICE);
        assert_ok!(Staking::unbond(RuntimeOrigin::signed(stash), 500 * UNIT));
        // Withdrawing too early returns Ok but does not credit funds (chunks remain).
        run_to_era(BondingDuration::get() + 1);
        assert_ok!(Staking::withdraw_unbonded(RuntimeOrigin::signed(stash), 0));
        let ledger = pallet_staking::Ledger::<Runtime>::get(stash).unwrap();
        assert!(ledger.unlocking.is_empty(), "unbonded chunks should be withdrawn");
    });
}
