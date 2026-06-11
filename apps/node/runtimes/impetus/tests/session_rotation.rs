mod common;
use common::*;

use impetus_runtime::{SESSION_PERIOD, Session};

#[test]
fn session_index_increments_each_period() {
    ExtBuilder::default().build().execute_with(|| {
        assert_eq!(Session::current_index(), 0);
        run_to_block(SESSION_PERIOD + 1);
        assert_eq!(Session::current_index(), 1);
        run_to_block(SESSION_PERIOD * 2 + 1);
        assert_eq!(Session::current_index(), 2);
    });
}

#[test]
fn session_validators_pulled_from_staking_on_rotation() {
    ExtBuilder::default().build().execute_with(|| {
        run_to_block(2);
        let pre = Session::validators();
        run_to_session(2);
        let post = Session::validators();
        // Same validator SET in dev (no stake changes between sessions).
        // Compare as sets — phragmen may permute the order when a new era
        // election runs in `runtime-test-fast` mode.
        let mut pre_sorted = pre.clone();
        pre_sorted.sort();
        let mut post_sorted = post.clone();
        post_sorted.sort();
        assert_eq!(pre_sorted, post_sorted);
    });
}
