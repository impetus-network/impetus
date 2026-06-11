#![cfg(test)]

//! Minimal test runtime for `precompile-staking-admin`.
//!
//! The aim is to exercise the precompile's dispatch path (origin construction,
//! codec, payload shaping, post-conditions in `pallet-staking` storage), NOT to
//! reproduce the full NPoS economic machinery. We therefore:
//!
//! * use `H160` as `AccountId` so address mapping is the identity, matching the
//!   production `runtimes/impetus` configuration;
//! * stub `ElectionProvider` / `GenesisElectionProvider` with `NoElection`;
//! * route `VoterList` and `TargetList` through the `UseValidatorsMap` /
//!   `UseNominatorsAndValidatorsMap` adapters shipped by `pallet-staking`,
//!   sidestepping the `pallet-bags-list` dependency;
//! * wire `pallet-sudo` so the sudo-gating happy-path tests can call
//!   `set_validator_count` / `force_new_era` / etc. with `caller == SUDO_KEY`.
//!
//! This is the gold template T2 (`precompile-staking`) plus the sudo wiring
//! pattern from T5 (`precompile-fast-unstake`) and T6 (`precompile-treasury`).

use frame_support::{
    construct_runtime, parameter_types,
    traits::{ConstU128, ConstU32, ConstU64, Everything, FindAuthor, Nothing, VariantCountOf},
    weights::Weight,
};
use pallet_evm::{EnsureAddressNever, EnsureAddressRoot, IdentityAddressMapping};
use sp_core::{H160, U256};
use sp_runtime::{
    traits::{BlakeTwo256, IdentityLookup},
    BuildStorage, ConsensusEngineId,
};

use crate::StakingAdminPrecompileSet;

pub type Block = frame_system::mocking::MockBlock<Runtime>;
pub type AccountId = H160;
pub type Balance = u128;

construct_runtime!(
    pub enum Runtime {
        System:    frame_system,
        Balances:  pallet_balances,
        Timestamp: pallet_timestamp,
        EVM:       pallet_evm,
        Sudo:      pallet_sudo,
        Staking:   pallet_staking,
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
    pub PrecompilesValue: StakingAdminPrecompileSet = StakingAdminPrecompileSet::new();
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
    type PrecompilesType = StakingAdminPrecompileSet;
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

impl pallet_sudo::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type RuntimeCall = RuntimeCall;
    type WeightInfo = ();
}

// --- pallet-staking minimal config ---------------------------------------

parameter_types! {
    pub const SessionsPerEra: sp_staking::SessionIndex = 3;
    pub const BondingDuration: sp_staking::EraIndex = 3;
    pub const SlashDeferDuration: sp_staking::EraIndex = 0;
    pub static HistoryDepth: u32 = 84;
    pub static MaxExposurePageSize: u32 = 64;
    pub static MaxUnlockingChunks: u32 = 32;
    pub static MaxValidatorSet: u32 = 100;
    pub static MaxControllersInDeprecationBatch: u32 = 100;
    pub static MaxBackersPerWinner: u32 = 256;
    pub static MaxWinnersPerPage: u32 = 100;
}

pub struct NoElectionProvider;

impl frame_election_provider_support::ElectionProvider for NoElectionProvider {
    type AccountId = AccountId;
    type BlockNumber = u64;
    type Error = &'static str;
    type Pages = ConstU32<1>;
    type DataProvider = pallet_staking::Pallet<Runtime>;
    type MaxWinnersPerPage = MaxWinnersPerPage;
    type MaxBackersPerWinner = MaxBackersPerWinner;
    type MaxBackersPerWinnerFinal = MaxBackersPerWinner;
    fn elect(
        _page: frame_election_provider_support::PageIndex,
    ) -> Result<frame_election_provider_support::BoundedSupportsOf<Self>, Self::Error> {
        Err("NoElectionProvider: cannot elect in mock")
    }
    fn start() -> Result<(), Self::Error> {
        Ok(())
    }
    fn duration() -> Self::BlockNumber {
        0
    }
    fn status() -> Result<Option<frame_support::weights::Weight>, ()> {
        Err(())
    }
}

impl pallet_staking::Config for Runtime {
    type OldCurrency = Balances;
    type Currency = Balances;
    type RuntimeHoldReason = RuntimeHoldReason;
    type CurrencyBalance = Balance;
    type UnixTime = Timestamp;
    type CurrencyToVote = sp_staking::currency_to_vote::SaturatingCurrencyToVote;
    type RewardRemainder = ();
    type RuntimeEvent = RuntimeEvent;
    type Slash = ();
    type Reward = ();
    type SessionsPerEra = SessionsPerEra;
    type BondingDuration = BondingDuration;
    type SlashDeferDuration = SlashDeferDuration;
    type AdminOrigin = frame_system::EnsureRoot<AccountId>;
    type SessionInterface = ();
    type EraPayout = ();
    type NextNewSession = ();
    type MaxExposurePageSize = MaxExposurePageSize;
    type MaxValidatorSet = MaxValidatorSet;
    type ElectionProvider = NoElectionProvider;
    type GenesisElectionProvider = NoElectionProvider;
    type VoterList = pallet_staking::UseNominatorsAndValidatorsMap<Self>;
    type TargetList = pallet_staking::UseValidatorsMap<Self>;
    type NominationsQuota = pallet_staking::FixedNominationsQuota<16>;
    type MaxUnlockingChunks = MaxUnlockingChunks;
    type MaxControllersInDeprecationBatch = MaxControllersInDeprecationBatch;
    type HistoryDepth = HistoryDepth;
    type EventListeners = ();
    type Filter = Nothing;
    type BenchmarkingConfig = pallet_staking::TestBenchmarkingConfig;
    type WeightInfo = ();
}

// --- ext builder + helpers -----------------------------------------------

/// Sudo key used by sudo-gated happy-path tests.
pub const SUDO_KEY: H160 = H160(hex_literal::hex!(
    "00000000000000000000000000000000000000ff"
));

pub fn new_test_ext() -> sp_io::TestExternalities {
    let mut t = frame_system::GenesisConfig::<Runtime>::default()
        .build_storage()
        .unwrap();

    // Pre-fund some test accounts + the sudo key so any fee-paying paths
    // inside `try_dispatch` do not panic.
    let mut balances: alloc::vec::Vec<(H160, Balance)> = (1u64..=20u64)
        .map(|i| (H160::from_low_u64_be(i), 1_000_000_000_000u128))
        .collect();
    balances.push((SUDO_KEY, 1_000_000_000_000u128));
    pallet_balances::GenesisConfig::<Runtime> {
        balances,
        ..Default::default()
    }
    .assimilate_storage(&mut t)
    .unwrap();

    pallet_sudo::GenesisConfig::<Runtime> {
        key: Some(SUDO_KEY),
    }
    .assimilate_storage(&mut t)
    .unwrap();

    let mut ext = sp_io::TestExternalities::new(t);
    ext.execute_with(|| {
        System::set_block_number(1);
        // Default `ValidatorCount` for tests that read it before any
        // admin-side write touches it.
        pallet_staking::ValidatorCount::<Runtime>::put(4);
        pallet_staking::CurrentEra::<Runtime>::put(0);
    });
    ext
}

/// Address of the staking-admin precompile in the mock.
pub fn staking_admin_addr() -> H160 {
    H160::from_low_u64_be(crate::PRECOMPILE_ADDRESS)
}
