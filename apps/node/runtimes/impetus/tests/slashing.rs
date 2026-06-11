mod common;
use common::*;

use frame_support::assert_ok;
use impetus_runtime::{Offences, Runtime, SlashDeferDuration, Staking, Treasury, UNIT};
use sp_runtime::Perbill;
use sp_staking::{
    offence::{Kind, Offence, ReportOffence},
    SessionIndex,
};

// `pallet_session::historical::Config::FullIdentification` in impetus is `()`
// (the runtime uses `pallet_staking::UnitIdentificationOf`), so each
// identification tuple carries no exposure data alongside the validator id.
type IdTuple = pallet_session::historical::IdentificationTuple<Runtime>;

/// Concrete offence with a fixed 10% slash fraction. The built-in
/// `UnresponsivenessOffence` always returns 0% when offender count < 7/3 of the
/// validator set, which masks a working slashing pipeline in a 4-validator
/// test runtime — hence the local type.
///
/// Note: stable2603 removed `DisableStrategy` from the `Offence` trait, so
/// this impl is intentionally minimal compared to older Substrate docs.
struct TestSlashOffence {
    session_index: SessionIndex,
    validator_set_count: u32,
    offenders: Vec<IdTuple>,
}

impl Offence<IdTuple> for TestSlashOffence {
    // `sp_staking::offence::Kind` is `[u8; 16]`. Literal MUST be exactly 16 bytes.
    const ID: Kind = *b"test_slash_10pct";
    type TimeSlot = SessionIndex;
    fn offenders(&self) -> Vec<IdTuple> {
        self.offenders.clone()
    }
    fn session_index(&self) -> SessionIndex {
        self.session_index
    }
    fn validator_set_count(&self) -> u32 {
        self.validator_set_count
    }
    fn time_slot(&self) -> Self::TimeSlot {
        self.session_index
    }
    fn slash_fraction(&self, _offenders_count: u32) -> Perbill {
        Perbill::from_percent(10)
    }
}

#[test]
fn offence_debits_offender_and_credits_treasury_after_slash_defer() {
    ExtBuilder::default().build().execute_with(|| {
        let offender = account(ALICE);

        // Advance to era 1 so eras_stakers has real exposure for ALICE.
        run_to_era(1);
        let active_era = Staking::active_era().expect("active era after era 1").index;
        let exposure = Staking::eras_stakers(active_era, &offender);
        // The exact exposure depends on the election outcome (phragmen weight
        // distribution between ALICE and the nominator), so we only require
        // a non-zero exposure to reach the slash code path.
        assert!(
            exposure.total > 0,
            "ALICE must have some exposure recorded at era {active_era}, got {}",
            exposure.total
        );
        let _ = UNIT;

        let bond_before = pallet_staking::Ledger::<Runtime>::get(offender)
            .expect("ALICE bonded at era 1")
            .active;
        assert!(bond_before > 0);
        let pot_before = Treasury::pot();
        let session_idx = impetus_runtime::Session::current_index();

        // The Result MUST be Ok — discarding it was the bug in the prior draft.
        // `FullIdentification = ()` so the identification tuple is `(AccountId, ())`;
        // pallet-staking still looks up the offender's exposure from `ErasStakers`
        // when applying the slash, so the `exposure` we computed above does not
        // need to be embedded in the report.
        let _ = exposure;
        assert_ok!(<Offences as ReportOffence<_, IdTuple, TestSlashOffence>>::report_offence(
            vec![],
            TestSlashOffence {
                session_index: session_idx,
                validator_set_count: 4,
                offenders: vec![(offender, ())],
            },
        ));

        // Advance past the slash-defer window so pallet-staking applies the slash.
        run_to_era(active_era + SlashDeferDuration::get() + 1);

        let bond_after = pallet_staking::Ledger::<Runtime>::get(offender)
            .expect("ALICE remains bonded after slash")
            .active;
        let pot_after = Treasury::pot();

        assert!(
            bond_after < bond_before,
            "active bond must decrease after slash applies: before={bond_before}, after={bond_after}"
        );
        assert!(
            pot_after > pot_before,
            "treasury pot must grow when Slash = Treasury: before={pot_before}, after={pot_after}"
        );
    });
}
