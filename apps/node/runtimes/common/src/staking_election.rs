//! Solver wiring shared by impetus's NPoS pallets.
//!
//! `pallet-staking::GenesisElectionProvider` and the fallback path for
//! `pallet-election-provider-multi-phase` both use this on-chain Phragmen
//! configuration. The `MaxWinnersPerPage` / voter bounds match the dev-fast
//! `VALIDATOR_COUNT_TARGET` budget.

use crate::{
    staking_constants::{MAX_NOMINATIONS, VALIDATOR_COUNT_TARGET},
    AccountId,
};
use frame_election_provider_support::{
    bounds::{ElectionBounds, ElectionBoundsBuilder},
    SequentialPhragmen,
};
use frame_support::{parameter_types, traits::ConstBool};
use sp_runtime::Perbill;

parameter_types! {
    pub MaxOnChainElectingVoters: u32 = 1024;
    pub MaxOnChainElectableTargets: u32 = 64;
    pub MaxActiveValidators: u32 = VALIDATOR_COUNT_TARGET;
    pub MaxBackersPerWinner: u32 = MaxOnChainElectingVoters::get();
    pub OffchainSolutionLengthLimit: u32 = 4 * 1024 * 1024;
    pub OffchainSolutionWeightLimit: frame_support::weights::Weight =
        frame_support::weights::Weight::from_parts(u64::MAX, u64::MAX);
    pub ElectionBoundsOnChain: ElectionBounds = ElectionBoundsBuilder::default()
        .voters_count(MaxOnChainElectingVoters::get().into())
        .targets_count(MaxOnChainElectableTargets::get().into())
        .build();
    pub ElectionBoundsMultiPhase: ElectionBounds = ElectionBoundsBuilder::default()
        .voters_count(MaxOnChainElectingVoters::get().into())
        .targets_count(MaxOnChainElectableTargets::get().into())
        .build();
    pub SignedRewardBase: u128 = crate::UNIT;
    pub SignedDepositBase: u128 = crate::UNIT;
    pub SignedDepositByte: u128 = 1_000_000;
    pub BetterSignedThreshold: Perbill = Perbill::from_rational(5u32, 10_000u32);
    pub MaxNominations: u32 = MAX_NOMINATIONS;
}

pub type OnChainSeqPhragmenScore = sp_npos_elections::ExtendedBalance;

use sp_core::U256;

/// Lossless `Balance -> U256` conversion used by `pallet-nomination-pools`.
pub struct BalanceToU256;
impl sp_runtime::traits::Convert<crate::Balance, U256> for BalanceToU256 {
    fn convert(n: crate::Balance) -> U256 {
        n.into()
    }
}

/// Saturating `U256 -> Balance` conversion used by `pallet-nomination-pools`.
pub struct U256ToBalance;
impl sp_runtime::traits::Convert<U256, crate::Balance> for U256ToBalance {
    fn convert(n: U256) -> crate::Balance {
        use sp_runtime::traits::UniqueSaturatedInto;
        n.unique_saturated_into()
    }
}

pub struct OnChainSeqPhragmen<R>(core::marker::PhantomData<R>);

impl<R> frame_election_provider_support::onchain::Config for OnChainSeqPhragmen<R>
where
    R: frame_system::Config<AccountId = AccountId>
        + pallet_staking::Config
        + pallet_bags_list::Config<pallet_bags_list::Instance1>,
{
    type Sort = ConstBool<true>;
    type System = R;
    type Solver = SequentialPhragmen<AccountId, Perbill>;
    type DataProvider = pallet_staking::Pallet<R>;
    type WeightInfo = ();
    type Bounds = ElectionBoundsOnChain;
    type MaxBackersPerWinner = MaxBackersPerWinner;
    type MaxWinnersPerPage = MaxActiveValidators;
}
