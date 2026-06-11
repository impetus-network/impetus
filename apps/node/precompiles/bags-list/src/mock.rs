#![cfg(test)]

//! Minimal test runtime for `precompile-bags-list`.
//!
//! Goal: exercise the precompile's dispatch path and views, NOT the full NPoS
//! economics that drive scores in production. We therefore:
//!
//! * use `H160` as `AccountId` (identity address mapping), matching
//!   `runtimes/impetus`;
//! * provide a `StaticScoreProvider` backed by a `thread_local!` map so each
//!   test can set scores directly. The bags-list pallet calls
//!   `T::ScoreProvider::score(&who)` from `rebag` to decide which bag a node
//!   belongs in;
//! * use a small four-bucket `BagThresholds` slice so it is easy to seed
//!   scores that fall into specific bags;
//! * skip `pallet-staking` entirely. The precompile only depends on
//!   `pallet_bags_list::Config<Instance1>` (and transitively `frame_system` +
//!   `pallet_evm`).

use alloc::vec::Vec;
use core::cell::RefCell;
use frame_election_provider_support::{ScoreProvider, SortedListProvider};
use frame_support::{
    construct_runtime, parameter_types,
    traits::{ConstU128, ConstU32, ConstU64, Everything, FindAuthor, VariantCountOf},
    weights::Weight,
};
use pallet_evm::{EnsureAddressNever, EnsureAddressRoot, IdentityAddressMapping};
use sp_core::{H160, U256};
use sp_runtime::{
    traits::{BlakeTwo256, IdentityLookup},
    BuildStorage, ConsensusEngineId,
};

use crate::BagsListPrecompileSet;

pub type Block = frame_system::mocking::MockBlock<Runtime>;
pub type AccountId = H160;
pub type Balance = u128;

construct_runtime!(
    pub enum Runtime {
        System:    frame_system,
        Balances:  pallet_balances,
        Timestamp: pallet_timestamp,
        EVM:       pallet_evm,
        VoterList: pallet_bags_list::<Instance1>,
    }
);

parameter_types! {
    pub const BlockHashCount: u64 = 250;
}

impl frame_system::Config for Runtime {
    type AccountId = AccountId;
    type Block = Block;
    type BlockHashCount = BlockHashCount;
    type BlockWeights = ();
    type BlockLength = ();
    type BaseCallFilter = Everything;
    type RuntimeOrigin = RuntimeOrigin;
    type RuntimeCall = RuntimeCall;
    type RuntimeTask = ();
    type Nonce = u64;
    type Hash = sp_core::H256;
    type Hashing = BlakeTwo256;
    type Lookup = IdentityLookup<Self::AccountId>;
    type RuntimeEvent = RuntimeEvent;
    type DbWeight = ();
    type Version = ();
    type PalletInfo = PalletInfo;
    type AccountData = pallet_balances::AccountData<Balance>;
    type OnNewAccount = ();
    type OnKilledAccount = ();
    type SystemWeightInfo = ();
    type SS58Prefix = ();
    type OnSetCode = ();
    type MaxConsumers = ConstU32<16>;
    type MultiBlockMigrator = ();
    type SingleBlockMigrations = ();
    type PreInherents = ();
    type PostInherents = ();
    type PostTransactions = ();
    type ExtensionsWeightInfo = ();
}

impl pallet_balances::Config for Runtime {
    type Balance = Balance;
    type DustRemoval = ();
    type RuntimeEvent = RuntimeEvent;
    type ExistentialDeposit = ConstU128<1>;
    type AccountStore = System;
    type WeightInfo = ();
    type MaxLocks = ();
    type MaxReserves = ();
    type ReserveIdentifier = [u8; 8];
    type FreezeIdentifier = RuntimeFreezeReason;
    type MaxFreezes = VariantCountOf<RuntimeFreezeReason>;
    type RuntimeHoldReason = RuntimeHoldReason;
    type RuntimeFreezeReason = RuntimeFreezeReason;
    type DoneSlashHandler = ();
}

parameter_types! {
    pub const MinimumPeriod: u64 = 1;
}

impl pallet_timestamp::Config for Runtime {
    type Moment = u64;
    type OnTimestampSet = ();
    type MinimumPeriod = MinimumPeriod;
    type WeightInfo = ();
}

pub struct FindAuthorNone;
impl FindAuthor<H160> for FindAuthorNone {
    fn find_author<'a, I>(_digests: I) -> Option<H160>
    where
        I: 'a + IntoIterator<Item = (ConsensusEngineId, &'a [u8])>,
    {
        None
    }
}

parameter_types! {
    pub BlockGasLimit: U256 = U256::from(75_000_000u64);
    pub WeightPerGas: Weight = Weight::from_parts(20_000, 0);
    pub GasLimitPovSizeRatio: u64 = 0;
    pub GasLimitStorageGrowthRatio: u64 = 0;
    pub TransactionGasLimit: Option<U256> = None;
    pub PrecompilesValue: BagsListPrecompileSet = BagsListPrecompileSet::new();
}

impl pallet_evm::Config for Runtime {
    type FeeCalculator = ();
    type GasWeightMapping = pallet_evm::FixedGasWeightMapping<Self>;
    type WeightPerGas = WeightPerGas;
    type BlockHashMapping = pallet_evm::SubstrateBlockHashMapping<Self>;
    type CallOrigin = EnsureAddressRoot<AccountId>;
    type WithdrawOrigin = EnsureAddressNever<AccountId>;
    type AddressMapping = IdentityAddressMapping;
    type Currency = Balances;
    type PrecompilesType = BagsListPrecompileSet;
    type PrecompilesValue = PrecompilesValue;
    type ChainId = ConstU64<322>;
    type BlockGasLimit = BlockGasLimit;
    type Runner = pallet_evm::runner::stack::Runner<Self>;
    type OnChargeTransaction = ();
    type OnCreate = ();
    type FindAuthor = FindAuthorNone;
    type GasLimitPovSizeRatio = GasLimitPovSizeRatio;
    type GasLimitStorageGrowthRatio = GasLimitStorageGrowthRatio;
    type TransactionGasLimit = TransactionGasLimit;
    type Timestamp = Timestamp;
    type WeightInfo = ();
    type AccountProvider = pallet_evm::FrameSystemAccountProvider<Self>;
    type CreateOriginFilter = ();
    type CreateInnerOriginFilter = ();
}

// --- StaticScoreProvider -------------------------------------------------
//
// A thread-local map from AccountId to score. Tests set scores directly via
// [`set_score`] so the bags-list pallet can read them through the
// `ScoreProvider` trait when handling `rebag`. This mirrors the
// `thread_local!` mocking pattern from T4 / T5.

thread_local! {
    pub static SCORES: RefCell<alloc::collections::BTreeMap<AccountId, u64>> =
        RefCell::new(Default::default());
}

pub struct StaticScoreProvider;

impl ScoreProvider<AccountId> for StaticScoreProvider {
    type Score = u64;

    fn score(who: &AccountId) -> Option<Self::Score> {
        SCORES.with(|s| s.borrow().get(who).copied())
    }

    fn set_score_of(who: &AccountId, new_score: Self::Score) {
        SCORES.with(|s| {
            s.borrow_mut().insert(*who, new_score);
        });
    }
}

/// Convenience helper to set the score reported by `StaticScoreProvider` for
/// `who`. Used by tests to control rebagging behaviour.
pub fn set_score(who: AccountId, score: u64) {
    SCORES.with(|s| {
        s.borrow_mut().insert(who, score);
    });
}

/// Wipe the in-memory score map between tests so state from one test cannot
/// leak into another.
pub fn reset_scores() {
    SCORES.with(|s| s.borrow_mut().clear());
}

// --- pallet-bags-list config ------------------------------------------

parameter_types! {
    /// Four-bag layout: scores 0..=10, 11..=100, 101..=1_000, 1_001..=10_000
    /// land in the corresponding bag; anything heavier than `10_000` lands
    /// in the implicit top bag (bag_upper == Score::MAX, here u64::MAX).
    pub const BagThresholdsSlice: &'static [u64] = &[10u64, 100u64, 1_000u64, 10_000u64];
}

impl pallet_bags_list::Config<pallet_bags_list::Instance1> for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type WeightInfo = ();
    type ScoreProvider = StaticScoreProvider;
    type BagThresholds = BagThresholdsSlice;
    type Score = u64;
    type MaxAutoRebagPerBlock = ();
}

// --- ext builder + helpers --------------------------------------------

pub fn new_test_ext() -> sp_io::TestExternalities {
    reset_scores();

    let mut t = frame_system::GenesisConfig::<Runtime>::default()
        .build_storage()
        .unwrap();

    // Pre-fund some test accounts so fee-paying paths inside `try_dispatch`
    // (if/when fees become non-zero in future) do not panic.
    let balances: Vec<(H160, Balance)> = (1u64..=20u64)
        .map(|i| (H160::from_low_u64_be(i), 1_000_000_000_000u128))
        .collect();
    pallet_balances::GenesisConfig::<Runtime> {
        balances,
        ..Default::default()
    }
    .assimilate_storage(&mut t)
    .unwrap();

    let mut ext = sp_io::TestExternalities::new(t);
    ext.execute_with(|| {
        System::set_block_number(1);
    });
    ext
}

/// Address of the bags-list precompile in the mock.
pub fn bags_list_addr() -> H160 {
    H160::from_low_u64_be(crate::PRECOMPILE_ADDRESS)
}

/// Seed the `StaticScoreProvider` with `(who, score)` and insert `who` into
/// the list via the `SortedListProvider::on_insert` API so subsequent
/// `rebag` / `put_in_front_of` calls observe a real list node.
pub fn insert_node(who: AccountId, score: u64) {
    set_score(who, score);
    <pallet_bags_list::Pallet<Runtime, pallet_bags_list::Instance1> as SortedListProvider<
        AccountId,
    >>::on_insert(who, score)
    .expect("on_insert must succeed in test setup");
}
