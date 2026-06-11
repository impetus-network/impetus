mod common;
use common::*;

use frame_support::assert_ok;
use impetus_runtime::{FastUnstake, Runtime, RuntimeOrigin, Staking};

#[test]
fn register_fast_unstake_appends_to_queue() {
    ExtBuilder::default().build().execute_with(|| {
        // Fast-unstake's register call returns `CallNotAllowed` when
        // `ErasToCheckPerBlock` is 0 (the default). Enable the feature first
        // via the `control` extrinsic; in the impetus runtime this gate is
        // `ControlOrigin = EnsureRoot`.
        assert_ok!(FastUnstake::control(RuntimeOrigin::root(), 1));

        let stash = account(DAVE);
        // Validator must chill first to be eligible.
        assert_ok!(Staking::chill(RuntimeOrigin::signed(stash)));
        assert_ok!(FastUnstake::register_fast_unstake(RuntimeOrigin::signed(stash)));
        assert!(pallet_fast_unstake::Queue::<Runtime>::contains_key(stash));
    });
}
