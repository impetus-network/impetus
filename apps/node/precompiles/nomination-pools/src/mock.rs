#![cfg(test)]

//! Minimal test runtime for `precompile-nomination-pools`.
//!
//! The aim is to exercise the precompile's dispatch path (origin construction,
//! codec, payload shaping, storage views), NOT the full staking economics
//! behind nomination pools. We therefore:
//!
//! * use `H160` as `AccountId` so address mapping is the identity, matching
//!   the production `runtimes/impetus` configuration;
//! * stub `StakeStrategy` with the legacy `TransferStake` adapter pointing at
//!   a custom `StakingMock` that fulfils `sp_staking::StakingInterface`. This
//!   sidesteps the need to wire pallet-staking + bags-list + election in the
//!   test runtime, while still letting `create` / `nominate` / `unbond` etc
//!   reach the pool's bonding path;
//! * skip `pallet_session` / `pallet_staking` entirely. The precompile only
//!   depends on `pallet_nomination_pools::Config` + `pallet_sudo::Config`.
//!
//! Storage-view tests insert pool state directly via
//! `pallet_nomination_pools::BondedPools / PoolMembers / Metadata / LastPoolId`
//! so we never need a working election to land paged storage.

use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use core::cell::RefCell;
use frame_support::{
    construct_runtime,
    dispatch::DispatchResult,
    parameter_types,
    traits::{
        fungible::Mutate as FungibleMutate, ConstU128, ConstU32, ConstU64, ConstU8, Everything,
        FindAuthor, Nothing, VariantCountOf,
    },
    weights::Weight,
    PalletId,
};
use pallet_evm::{EnsureAddressNever, EnsureAddressRoot, IdentityAddressMapping};
use sp_core::{H160, U256};
use sp_runtime::{
    traits::{BlakeTwo256, Convert, IdentityLookup},
    BuildStorage, ConsensusEngineId, DispatchError, FixedU128, Perbill,
};

use crate::NominationPoolsPrecompileSet;

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
        Pools:     pallet_nomination_pools,
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
    pub PrecompilesValue: NominationPoolsPrecompileSet = NominationPoolsPrecompileSet::new();
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
    type PrecompilesType = NominationPoolsPrecompileSet;
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

// --- pallet-nomination-pools minimal config ------------------------------

// In-memory staking ledger used by `StakingMock` to fulfil `StakingInterface`
// for the nomination-pools pallet. This keeps the precompile tests focused on
// the dispatch path without dragging pallet-staking + bags-list + election
// providers into the mock runtime.
thread_local! {
    pub static BONDED: RefCell<BTreeMap<AccountId, Balance>> = RefCell::new(Default::default());
    pub static UNBONDING: RefCell<BTreeMap<AccountId, Vec<(u32, Balance)>>> =
        RefCell::new(Default::default());
    pub static CURRENT_ERA: RefCell<u32> = const { RefCell::new(0) };
    pub static MIN_NOMINATOR_BOND: RefCell<Balance> = const { RefCell::new(1) };
    pub static NOMINATIONS: RefCell<BTreeMap<AccountId, Vec<AccountId>>> =
        RefCell::new(Default::default());
}

pub struct StakingMock;

impl sp_staking::StakingInterface for StakingMock {
    type Balance = Balance;
    type AccountId = AccountId;
    type CurrencyToVote = sp_staking::currency_to_vote::SaturatingCurrencyToVote;

    fn minimum_nominator_bond() -> Self::Balance {
        MIN_NOMINATOR_BOND.with(|b| *b.borrow())
    }
    fn minimum_validator_bond() -> Self::Balance {
        MIN_NOMINATOR_BOND.with(|b| *b.borrow())
    }
    fn stash_by_ctrl(_controller: &Self::AccountId) -> Result<Self::AccountId, DispatchError> {
        Err(DispatchError::Other("not implemented in mock"))
    }
    fn bonding_duration() -> sp_staking::EraIndex {
        3
    }
    fn current_era() -> sp_staking::EraIndex {
        CURRENT_ERA.with(|e| *e.borrow())
    }
    fn stake(who: &Self::AccountId) -> Result<sp_staking::Stake<Balance>, DispatchError> {
        let bonded = BONDED.with(|b| b.borrow().get(who).copied()).unwrap_or(0);
        let unbonding_sum: Balance = UNBONDING
            .with(|u| u.borrow().get(who).cloned())
            .unwrap_or_default()
            .iter()
            .map(|(_, v)| *v)
            .sum();
        if bonded == 0 && unbonding_sum == 0 {
            Err(DispatchError::Other("not bonded"))
        } else {
            Ok(sp_staking::Stake {
                total: bonded.saturating_add(unbonding_sum),
                active: bonded,
            })
        }
    }
    fn is_virtual_staker(_who: &Self::AccountId) -> bool {
        false
    }
    fn bond_extra(who: &Self::AccountId, extra: Self::Balance) -> DispatchResult {
        BONDED.with(|b| {
            let mut m = b.borrow_mut();
            *m.entry(*who).or_insert(0) += extra;
        });
        Ok(())
    }
    fn unbond(who: &Self::AccountId, amount: Self::Balance) -> DispatchResult {
        BONDED.with(|b| {
            let mut m = b.borrow_mut();
            if let Some(v) = m.get_mut(who) {
                *v = v.saturating_sub(amount);
            }
        });
        let era = Self::current_era() + Self::bonding_duration();
        UNBONDING.with(|u| {
            u.borrow_mut().entry(*who).or_default().push((era, amount));
        });
        Ok(())
    }
    fn set_payee(_stash: &Self::AccountId, _reward_acc: &Self::AccountId) -> DispatchResult {
        Ok(())
    }
    fn chill(_who: &Self::AccountId) -> DispatchResult {
        Ok(())
    }
    fn withdraw_unbonded(
        who: Self::AccountId,
        _num_slashing_spans: u32,
    ) -> Result<bool, DispatchError> {
        let current = Self::current_era();
        let drained = UNBONDING.with(|u| {
            let mut m = u.borrow_mut();
            let v = m.entry(who).or_default();
            v.retain(|(era, _)| *era > current);
            let killed = v.is_empty();
            let still_bonded = BONDED.with(|b| b.borrow().get(&who).copied()).unwrap_or(0);
            killed && still_bonded == 0
        });
        Ok(drained)
    }
    fn bond(
        stash: &Self::AccountId,
        value: Self::Balance,
        _payee: &Self::AccountId,
    ) -> DispatchResult {
        BONDED.with(|b| {
            b.borrow_mut().insert(*stash, value);
        });
        Ok(())
    }
    fn nominate(who: &Self::AccountId, validators: Vec<Self::AccountId>) -> DispatchResult {
        NOMINATIONS.with(|n| {
            n.borrow_mut().insert(*who, validators);
        });
        Ok(())
    }
    fn desired_validator_count() -> u32 {
        // Graceful stub: pallet-nomination-pools never reads this in the paths
        // exercised by the precompile, but a future stable2603 minor bump
        // could surface it. Return zero instead of panicking.
        0
    }
    fn election_ongoing() -> bool {
        false
    }
    fn force_unstake(_who: Self::AccountId) -> DispatchResult {
        // Graceful stub: same rationale as `desired_validator_count` above.
        Ok(())
    }
    fn is_exposed_in_era(_who: &Self::AccountId, _era: &sp_staking::EraIndex) -> bool {
        false
    }
    fn status(
        who: &Self::AccountId,
    ) -> Result<sp_staking::StakerStatus<Self::AccountId>, DispatchError> {
        let bonded = BONDED.with(|b| b.borrow().get(who).copied()).unwrap_or(0);
        if bonded == 0 {
            return Err(DispatchError::Other("NotStash"));
        }
        match NOMINATIONS.with(|n| n.borrow().get(who).cloned()) {
            Some(noms) => Ok(sp_staking::StakerStatus::Nominator(noms)),
            None => Ok(sp_staking::StakerStatus::Idle),
        }
    }
    fn slash_reward_fraction() -> Perbill {
        Perbill::zero()
    }
    fn set_era(era: sp_staking::EraIndex) {
        CURRENT_ERA.with(|e| *e.borrow_mut() = era);
    }
}

pub struct BalanceToU256;
impl Convert<Balance, U256> for BalanceToU256 {
    fn convert(n: Balance) -> U256 {
        n.into()
    }
}

pub struct U256ToBalance;
impl Convert<U256, Balance> for U256ToBalance {
    fn convert(n: U256) -> Balance {
        n.try_into().unwrap_or(Balance::MAX)
    }
}

parameter_types! {
    pub const PoolsPalletId: PalletId = PalletId(*b"py/nopls");
    pub const PostUnbondingPoolsWindow: u32 = 2;
    pub const MaxMetadataLen: u32 = 256;
    #[derive(Clone, PartialEq, Eq, Debug)]
    pub const MaxUnbonding: u32 = 8;
}

impl pallet_nomination_pools::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type WeightInfo = ();
    type Currency = Balances;
    type RuntimeFreezeReason = RuntimeFreezeReason;
    type RewardCounter = FixedU128;
    type BalanceToU256 = BalanceToU256;
    type U256ToBalance = U256ToBalance;
    // Matches production impetus runtime (runtimes/impetus/src/lib.rs uses
    // `TransferStake<Self, Staking>`). `DelegateStake` is the upstream
    // successor but is not currently used by impetus. We silence the upstream
    // deprecation here because the simpler adapter is sufficient to drive the
    // precompile's dispatch path without dragging `pallet-staking` +
    // `pallet-bags-list` into the test runtime.
    #[allow(deprecated)]
    type StakeAdapter = pallet_nomination_pools::adapter::TransferStake<Self, StakingMock>;
    type PostUnbondingPoolsWindow = PostUnbondingPoolsWindow;
    type PalletId = PoolsPalletId;
    type MaxMetadataLen = MaxMetadataLen;
    type MaxUnbonding = MaxUnbonding;
    type MaxPointsToBalance = ConstU8<10>;
    type AdminOrigin = frame_system::EnsureRoot<AccountId>;
    type BlockNumberProvider = System;
    type Filter = Nothing;
}

// --- ext builder + helpers -----------------------------------------------

/// Reset the in-memory staking ledger between tests so state from one test
/// can't leak into another via the `thread_local!` cells.
pub fn reset_staking_mock() {
    BONDED.with(|b| b.borrow_mut().clear());
    UNBONDING.with(|u| u.borrow_mut().clear());
    NOMINATIONS.with(|n| n.borrow_mut().clear());
    CURRENT_ERA.with(|e| *e.borrow_mut() = 0);
}

/// Sudo key used by `setConfigs` happy-path tests.
pub const SUDO_KEY: H160 = H160(hex_literal::hex!(
    "00000000000000000000000000000000000000ff"
));

pub fn new_test_ext() -> sp_io::TestExternalities {
    reset_staking_mock();

    let mut t = frame_system::GenesisConfig::<Runtime>::default()
        .build_storage()
        .unwrap();

    // Pre-fund several test accounts so `create` / `join` (which need `>= ED +
    // amount`) can land.
    let mut balances: Vec<(H160, Balance)> = (1u64..=20u64)
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
    });
    ext
}

/// Address of the nomination-pools precompile in the mock.
pub fn pools_addr() -> H160 {
    H160::from_low_u64_be(crate::PRECOMPILE_ADDRESS)
}

/// Convenience: top up an account so the pallet's `Currency::transfer` checks
/// (used by `TransferStake::pledge_bond`) pass on tests that pull from the
/// account's free balance.
pub fn fund(who: H160, amount: Balance) {
    <Balances as FungibleMutate<AccountId>>::mint_into(&who, amount).unwrap();
}
