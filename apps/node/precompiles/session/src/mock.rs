#![cfg(test)]

//! Minimal test runtime for `precompile-session`.
//!
//! The aim is to exercise the precompile's dispatch path (origin construction,
//! codec, payload shaping, storage views), NOT to reproduce the full session
//! rotation / authority machinery. We therefore:
//!
//! * use `H160` as both `AccountId` and `ValidatorId` so the production
//!   converter (`ConvertInto`) is the identity — same shape as
//!   `runtimes/impetus`;
//! * use `sp_runtime::testing::UintAuthorityId` as `Keys`, which gives us
//!   `OpaqueKeys` with `ownership_proof_is_valid` returning `true` and the
//!   full `Member + Parameter + MaybeSerializeDeserialize` derive set out of
//!   the box, so the `setKeys` proof argument is irrelevant to dispatch;
//! * provide a stubbed `SessionHandler` / `SessionManager` that do nothing —
//!   the precompile never triggers session rotation;
//! * skip `pallet-session::historical` entirely — the precompile doesn't need
//!   it and pulling it in would require staking too.
//!
//! Storage-view tests insert state directly via `pallet_session::CurrentIndex`,
//! `pallet_session::NextKeys`, and `pallet_session::QueuedKeys` so we never
//! need a working session-rotation pipeline to land that state.

use frame_support::{
    construct_runtime, parameter_types,
    traits::{ConstU32, ConstU64, Everything, FindAuthor},
    weights::Weight,
};
use pallet_evm::{EnsureAddressNever, EnsureAddressRoot, IdentityAddressMapping};
use sp_core::{H160, U256};
use sp_runtime::{
    impl_opaque_keys,
    testing::UintAuthorityId,
    traits::{BlakeTwo256, ConvertInto, IdentityLookup, OpaqueKeys},
    BuildStorage, ConsensusEngineId, KeyTypeId,
};

use crate::SessionPrecompileSet;

pub type Block = frame_system::mocking::MockBlock<Runtime>;
pub type AccountId = H160;
pub type Balance = u128;

impl_opaque_keys! {
    pub struct MockSessionKeys {
        pub dummy: UintAuthorityId,
    }
}

impl From<UintAuthorityId> for MockSessionKeys {
    fn from(dummy: UintAuthorityId) -> Self {
        Self { dummy }
    }
}

construct_runtime!(
    pub enum Runtime {
        System:    frame_system,
        Balances:  pallet_balances,
        Timestamp: pallet_timestamp,
        EVM:       pallet_evm,
        Session:   pallet_session,
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
    type ExistentialDeposit = frame_support::traits::ConstU128<1>;
    type AccountStore = System;
    type WeightInfo = ();
    type MaxLocks = ();
    type MaxReserves = ();
    type ReserveIdentifier = [u8; 8];
    type FreezeIdentifier = RuntimeFreezeReason;
    type MaxFreezes = ConstU32<1>;
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
    pub PrecompilesValue: SessionPrecompileSet = SessionPrecompileSet::new();
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
    type PrecompilesType = SessionPrecompileSet;
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

// ---- pallet-session minimal config -------------------------------------

// Period in blocks for the periodic session rotator. The mock never advances
// far enough to hit a rotation, but `PeriodicSessions` still needs constants.
parameter_types! {
    pub const Period: u64 = 5;
    pub const Offset: u64 = 0;
}

/// `SessionHandler` stub: announces the `DUMMY` key type so the genesis
/// integrity check in `pallet_session::GenesisConfig::build` is satisfied, and
/// no-ops every life-cycle callback. The precompile never drives rotation.
pub struct NoopSessionHandler;
impl pallet_session::SessionHandler<AccountId> for NoopSessionHandler {
    const KEY_TYPE_IDS: &'static [KeyTypeId] = &[sp_core::crypto::key_types::DUMMY];

    fn on_genesis_session<Ks: OpaqueKeys>(_validators: &[(AccountId, Ks)]) {}

    fn on_new_session<Ks: OpaqueKeys>(
        _changed: bool,
        _validators: &[(AccountId, Ks)],
        _queued_validators: &[(AccountId, Ks)],
    ) {
    }

    fn on_disabled(_validator_index: u32) {}

    fn on_before_session_ending() {}
}

/// `SessionManager` stub: returns the empty validator set every session.
pub struct NoopSessionManager;
impl pallet_session::SessionManager<AccountId> for NoopSessionManager {
    fn new_session(_index: sp_staking::SessionIndex) -> Option<Vec<AccountId>> {
        None
    }
    fn end_session(_index: sp_staking::SessionIndex) {}
    fn start_session(_index: sp_staking::SessionIndex) {}
}

impl pallet_session::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type ValidatorId = AccountId;
    type ValidatorIdOf = ConvertInto;
    type ShouldEndSession = pallet_session::PeriodicSessions<Period, Offset>;
    type NextSessionRotation = pallet_session::PeriodicSessions<Period, Offset>;
    type SessionManager = NoopSessionManager;
    type SessionHandler = NoopSessionHandler;
    type Keys = UintAuthorityId;
    type DisablingStrategy = pallet_session::disabling::UpToLimitWithReEnablingDisablingStrategy;
    type WeightInfo = ();
    type Currency = Balances;
    type KeyDeposit = ();
}

// ---- ext builder + helpers --------------------------------------------

pub fn new_test_ext() -> sp_io::TestExternalities {
    let mut t = frame_system::GenesisConfig::<Runtime>::default()
        .build_storage()
        .unwrap();

    pallet_balances::GenesisConfig::<Runtime> {
        balances: (1u64..=20u64)
            .map(|i| (H160::from_low_u64_be(i), 1_000_000_000_000u128))
            .collect(),
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

/// Address of the session precompile in the mock.
pub fn session_addr() -> H160 {
    H160::from_low_u64_be(crate::PRECOMPILE_ADDRESS)
}
