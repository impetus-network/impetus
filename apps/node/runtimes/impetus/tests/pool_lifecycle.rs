mod common;
use common::*;

use frame_support::assert_ok;
use impetus_runtime::{NominationPools, Runtime, RuntimeOrigin, UNIT};

#[test]
fn create_pool_then_join() {
    ExtBuilder::default().build().execute_with(|| {
        let depositor = account(ALICE);
        assert_ok!(NominationPools::create(
            RuntimeOrigin::signed(depositor),
            1_500 * UNIT,
            depositor,
            depositor,
            depositor,
        ));
        let pool_id = pallet_nomination_pools::LastPoolId::<Runtime>::get();
        assert!(pool_id > 0);

        let joiner = account(BOB);
        assert_ok!(NominationPools::join(
            RuntimeOrigin::signed(joiner),
            500 * UNIT,
            pool_id,
        ));
        let member = pallet_nomination_pools::PoolMembers::<Runtime>::get(joiner);
        assert!(member.is_some());
    });
}
