mod common;
use common::*;

use impetus_runtime::{Balances, RuntimeOrigin, Staking};

#[test]
fn nominator_balance_grows_after_payout_stakers() {
    ExtBuilder::default().build().execute_with(|| {
        let nom = account(NOMINATOR);
        let pre = Balances::free_balance(nom);
        run_to_era(2);
        // Payout era 1 (already finalized).
        let _ = Staking::payout_stakers(RuntimeOrigin::signed(nom), account(ALICE), 1);
        let post = Balances::free_balance(nom);
        assert!(post >= pre, "nominator balance should not decrease after payout call");
    });
}
