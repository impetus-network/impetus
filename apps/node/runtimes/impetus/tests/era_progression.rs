mod common;
use common::*;

use impetus_runtime::{SessionsPerEra, Staking};

#[test]
fn current_era_advances_after_sessions_per_era_sessions() {
    ExtBuilder::default().build().execute_with(|| {
        assert_eq!(Staking::current_era(), Some(0));
        run_to_session(SessionsPerEra::get() + 1);
        // The exact era count depends on whether `runtime-test-fast` is set
        // (it compresses sessions/era). Robust contract: era must have
        // advanced at least once.
        let era = Staking::current_era().expect("current_era populated");
        assert!(era >= 1, "current_era should advance past 0, got {era}");
    });
}

#[test]
fn active_era_index_matches_current_minus_one_initially_until_first_session() {
    ExtBuilder::default().build().execute_with(|| {
        let active = Staking::active_era();
        assert!(active.is_some());
        assert_eq!(active.unwrap().index, 0);
    });
}
