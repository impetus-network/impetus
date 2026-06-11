mod common;
use common::*;

use frame_support::assert_ok;
use impetus_runtime::{Runtime, RuntimeOrigin, Staking};
use pallet_staking::Forcing;

#[test]
fn force_new_era_flag_is_set() {
    ExtBuilder::default().build().execute_with(|| {
        assert_ok!(Staking::force_new_era(RuntimeOrigin::root()));
        assert_eq!(
            pallet_staking::ForceEra::<Runtime>::get(),
            Forcing::ForceNew
        );
    });
}

#[test]
fn force_no_eras_disables_election() {
    ExtBuilder::default().build().execute_with(|| {
        assert_ok!(Staking::force_no_eras(RuntimeOrigin::root()));
        assert_eq!(
            pallet_staking::ForceEra::<Runtime>::get(),
            Forcing::ForceNone
        );
    });
}
