mod common;
use common::*;

use frame_support::assert_ok;
use impetus_runtime::{Balances, Runtime, RuntimeOrigin, SpendPeriod, Treasury, UNIT};
use sp_runtime::traits::StaticLookup;

/// Plan 2 originally specified `propose_spend + approve_proposal`, but
/// stable2603 removed both extrinsics. Plan 3 / T6 redesigned the treasury
/// precompile around `spend_local`, and T18 unlocked `SpendOrigin` from the
/// Plan 2 `NeverEnsureOrigin` placeholder to
/// `EnsureRootWithSuccess<AccountId, MaxSpendOriginAmount>` so the precompile
/// can dispatch as `RawOrigin::Root` in production.
///
/// This test pins the runtime-side contract that backs the T6 precompile:
///   1. seeds the treasury pot via a regular transfer,
///   2. dispatches `Treasury::spend_local` with `RuntimeOrigin::root()`,
///   3. advances past one `SpendPeriod` so `on_initialize` drains the queued
///      approval into the beneficiary's free balance,
///   4. asserts the beneficiary's free balance increased by exactly the spend
///      amount.
#[test]
fn root_spend_local_credits_beneficiary_after_spend_period() {
    ExtBuilder::default().build().execute_with(|| {
        let proposer = account(ALICE);
        let beneficiary = account(BOB);
        let spend: u128 = 1_000 * UNIT;

        // Seed the treasury pot so the queued spend can pay out. ALICE has
        // 10_000 UNIT at genesis but 2_000 is locked as a validator bond, so
        // the maximum she can transfer is 8_000 UNIT. Seeding 5_000 keeps the
        // assertion margin clear of the bond and any future genesis tweaks.
        let pot_account = pallet_treasury::Pallet::<Runtime>::account_id();
        assert_ok!(Balances::transfer_keep_alive(
            RuntimeOrigin::signed(proposer),
            pot_account,
            5_000 * UNIT,
        ));
        let pot_seeded = Treasury::pot();
        assert!(
            pot_seeded >= spend,
            "treasury pot {pot_seeded} must cover the proposed spend {spend}",
        );

        let beneficiary_pre = Balances::free_balance(beneficiary);

        // stable2603 `spend_local` takes `(amount, beneficiary_lookup)` and is
        // gated by `Config::SpendOrigin`. The runtime now resolves that to
        // `EnsureRootWithSuccess<AccountId, MaxSpendOriginAmount>`, so a Root
        // origin is sufficient. This mirrors the runtime path the T6
        // precompile takes via `RawOrigin::Root.into()`.
        let beneficiary_lookup =
            <<Runtime as frame_system::Config>::Lookup as StaticLookup>::unlookup(beneficiary);
        // `spend_local` is upstream-deprecated in favour of `spend`. Plan 3 T6
        // keeps the legacy path on purpose because Plan 2 set
        // `Paymaster = PayFromAccount<Balances, TreasuryAccount>`, which is
        // only driven by the legacy spend-period flow. Match the precompile's
        // `#[allow(deprecated)]` to keep the warning local to this call.
        #[allow(deprecated)]
        {
            assert_ok!(Treasury::spend_local(
                RuntimeOrigin::root(),
                spend,
                beneficiary_lookup,
            ));
        }

        // Advance past one full SpendPeriod so `on_initialize` drains queued
        // approvals via `PayFromAccount<Balances, TreasuryAccount>`. A small
        // tail gives the spend-period handler a few blocks of slack.
        run_to_block(SpendPeriod::get() + 5);

        let beneficiary_post = Balances::free_balance(beneficiary);
        assert_eq!(
            beneficiary_post,
            beneficiary_pre + spend,
            "beneficiary must receive the exact spend amount",
        );
    });
}
