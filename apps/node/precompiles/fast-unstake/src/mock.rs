#![cfg(test)]

//! Minimal test runtime for `precompile-fast-unstake`.
//!
//! The aim is to exercise the precompile's dispatch path (origin construction,
//! codec, payload shaping, storage views), NOT the full staking economics
//! behind fast-unstake. We therefore:
//!
//! * use `H160` as `AccountId` so address mapping is the identity, matching
//!   the production `runtimes/impetus` configuration;
//! * provide a `StakingMock` implementing `sp_staking::StakingInterface` so
//!   `pallet-fast-unstake` can resolve `stash_by_ctrl`, `chill`,
//!   `fully_unbond`, `is_unbonding`, etc. without dragging
//!   `pallet-staking` + `pallet-bags-list` + election providers into the
//!   test runtime;
//! * skip `pallet_session` / `pallet_staking` entirely. The precompile only
//!   depends on `pallet_fast_unstake::Config` + `pallet_sudo::Config` (and
//!   transitively the `Staking` associated type).
//!
//! Storage-view tests insert fast-unstake state directly via
//! `pallet_fast_unstake::Queue / Head / ErasToCheckPerBlock` so we never need
//! a working election to land paged storage.

use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use core::cell::RefCell;
use frame_support::{
    construct_runtime,
    dispatch::DispatchResult,
    parameter_types,
    traits::{
        fungible::Mutate as FungibleMutate, ConstU128, ConstU32, ConstU64, Everything, FindAuthor,
        VariantCountOf,
    },
    weights::Weight,
};
use pallet_evm::{EnsureAddressNever, EnsureAddressRoot, IdentityAddressMapping};
use sp_core::{H160, U256};
use sp_runtime::{
    traits::{BlakeTwo256, IdentityLookup},
    BuildStorage, ConsensusEngineId, DispatchError, Perbill,
};

use crate::FastUnstakePrecompileSet;

pub type Block = frame_system::mocking::MockBlock<Runtime>;
pub type AccountId = H160;
pub type Balance = u128;

construct_runtime!(
    pub enum Runtime {
        System:      frame_system,
        Balances:    pallet_balances,
        Timestamp:   pallet_timestamp,
        EVM:         pallet_evm,
        Sudo:        pallet_sudo,
        FastUnstake: pallet_fast_unstake,
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
    pub PrecompilesValue: FastUnstakePrecompileSet = FastUnstakePrecompileSet::new();
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
    type PrecompilesType = FastUnstakePrecompileSet;
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

// --- StakingMock --------------------------------------------------------
//
// In-memory staking ledger used by `StakingMock` to fulfil `StakingInterface`
// for the fast-unstake pallet. Mirrors the pattern from the nomination-pools
// precompile mock but trimmed to the methods fast-unstake actually exercises
// from its extrinsics (`register_fast_unstake`, `deregister`). The on_idle
// path is NOT exercised by these unit tests; the runtime-level E2E suites in
// Plan 3 cover that end-to-end.

thread_local! {
    pub static BONDED: RefCell<BTreeMap<AccountId, Balance>> = RefCell::new(Default::default());
    pub static UNBONDING: RefCell<BTreeMap<AccountId, Vec<(u32, Balance)>>> =
        RefCell::new(Default::default());
    pub static CURRENT_ERA: RefCell<u32> = const { RefCell::new(0) };
    pub static MIN_NOMINATOR_BOND: RefCell<Balance> = const { RefCell::new(1) };
    // Tracks controllers that were ever bonded so `stash_by_ctrl` resolves
    // them even after `fully_unbond` zeroes out their active balance. In
    // stable2603 the controller path is collapsed to `stash == controller`
    // and `stash_by_ctrl` returns `Ok(controller)` for any account that has
    // a staking ledger, not only those that are still actively bonded.
    pub static EVER_BONDED: RefCell<BTreeMap<AccountId, ()>> =
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
    fn stash_by_ctrl(controller: &Self::AccountId) -> Result<Self::AccountId, DispatchError> {
        // In stable2603 the deprecated controller path collapses to
        // `stash == controller` and the lookup returns `Ok(controller)` for
        // any account with a staking ledger (including fully-unbonded
        // stashes). We track that membership separately from `BONDED` so
        // tests can call `register_fast_unstake` (which zeroes the active
        // balance via `fully_unbond`) and then `deregister`, mirroring the
        // real pallet's behavior. Accounts that were never bonded surface
        // `NotController` (translated here to `DispatchError::Other`).
        if EVER_BONDED.with(|b| b.borrow().contains_key(controller)) {
            Ok(*controller)
        } else {
            Err(DispatchError::Other("not bonded"))
        }
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
        EVER_BONDED.with(|b| {
            b.borrow_mut().insert(*stash, ());
        });
        Ok(())
    }
    fn nominate(_who: &Self::AccountId, _validators: Vec<Self::AccountId>) -> DispatchResult {
        Ok(())
    }
    fn desired_validator_count() -> u32 {
        // Graceful stub: fast-unstake's on_idle reads this, but the unit
        // tests in this crate do not drive on_idle. Return zero rather than
        // panicking so any future test that does touch on_idle gets a
        // deterministic (no-op) shape.
        0
    }
    fn election_ongoing() -> bool {
        false
    }
    fn force_unstake(who: Self::AccountId) -> DispatchResult {
        BONDED.with(|b| {
            b.borrow_mut().remove(&who);
        });
        UNBONDING.with(|u| {
            u.borrow_mut().remove(&who);
        });
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
        Ok(sp_staking::StakerStatus::Idle)
    }
    fn slash_reward_fraction() -> Perbill {
        Perbill::zero()
    }
    fn set_era(era: sp_staking::EraIndex) {
        CURRENT_ERA.with(|e| *e.borrow_mut() = era);
    }
}

// --- pallet-fast-unstake config ----------------------------------------

parameter_types! {
    pub const FastUnstakeDeposit: Balance = 10;
}

impl pallet_fast_unstake::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type Currency = Balances;
    type Deposit = FastUnstakeDeposit;
    type ControlOrigin = frame_system::EnsureRoot<AccountId>;
    type BatchSize = ConstU32<16>;
    type Staking = StakingMock;
    type MaxErasToCheckPerBlock = ConstU32<16>;
    type WeightInfo = ();
}

// --- ext builder + helpers ---------------------------------------------

/// Reset the in-memory staking ledger between tests so state from one test
/// can't leak into another via the `thread_local!` cells.
pub fn reset_staking_mock() {
    BONDED.with(|b| b.borrow_mut().clear());
    UNBONDING.with(|u| u.borrow_mut().clear());
    EVER_BONDED.with(|b| b.borrow_mut().clear());
    CURRENT_ERA.with(|e| *e.borrow_mut() = 0);
}

/// Sudo key used by `control` happy-path tests.
pub const SUDO_KEY: H160 = H160(hex_literal::hex!(
    "00000000000000000000000000000000000000ff"
));

pub fn new_test_ext() -> sp_io::TestExternalities {
    reset_staking_mock();

    let mut t = frame_system::GenesisConfig::<Runtime>::default()
        .build_storage()
        .unwrap();

    // Pre-fund several test accounts so `register_fast_unstake`'s
    // `Currency::reserve(deposit)` call lands.
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

/// Address of the fast-unstake precompile in the mock.
pub fn fast_unstake_addr() -> H160 {
    H160::from_low_u64_be(crate::PRECOMPILE_ADDRESS)
}

/// Convenience: top up an account so the pallet's `Currency::reserve` checks
/// (used by `register_fast_unstake`) pass on tests that pull from the
/// account's free balance.
#[allow(dead_code)]
pub fn fund(who: H160, amount: Balance) {
    <Balances as FungibleMutate<AccountId>>::mint_into(&who, amount).unwrap();
}

/// Pre-bond an account in the StakingMock so `stash_by_ctrl` resolves and
/// the fast-unstake register path can proceed.
pub fn pre_bond(stash: H160, value: Balance) {
    BONDED.with(|b| {
        b.borrow_mut().insert(stash, value);
    });
    EVER_BONDED.with(|b| {
        b.borrow_mut().insert(stash, ());
    });
}

/// Enable the fast-unstake pallet by setting `ErasToCheckPerBlock > 0`.
/// Without this, both `register_fast_unstake` and `deregister` revert with
/// `CallNotAllowed`.
pub fn enable_fast_unstake(eras: u32) {
    pallet_fast_unstake::ErasToCheckPerBlock::<Runtime>::put(eras);
}
