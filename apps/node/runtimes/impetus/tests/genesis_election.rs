mod common;
use common::*;

use impetus_runtime::{Runtime, Session};

#[test]
fn genesis_elects_four_validators() {
    ExtBuilder::default().build().execute_with(|| {
        run_to_block(2);
        let validators = Session::validators();
        assert_eq!(validators.len(), 4, "expected 4 validators elected at genesis");
        assert!(validators.contains(&account(ALICE)));
        assert!(validators.contains(&account(BOB)));
        assert!(validators.contains(&account(CHARLIE)));
        assert!(validators.contains(&account(DAVE)));
    });
}

#[test]
fn nominator_visible_in_staking_bonded() {
    ExtBuilder::default().build().execute_with(|| {
        let ledger = pallet_staking::Ledger::<Runtime>::get(account(NOMINATOR));
        assert!(ledger.is_some(), "nominator must be bonded at genesis");
        let l = ledger.unwrap();
        assert_eq!(l.active, 5_000 * impetus_runtime::UNIT);
    });
}
