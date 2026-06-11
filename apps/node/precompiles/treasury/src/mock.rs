#![cfg(test)]

//! Minimal test runtime for `precompile-treasury`.
//!
//! The aim is to exercise the precompile's dispatch path (origin construction,
//! codec, payload shaping, storage views), NOT the full economics of
//! `pallet-treasury`. We therefore:
//!
//! * use `H160` as `AccountId` so address mapping is the identity, matching
//!   the production `runtimes/impetus` configuration;
//! * set `SpendOrigin = EnsureRoot<AccountId>` so the precompile's
//!   `RawOrigin::Root` dispatch lands the `spend_local` path. (Production uses
//!   `NeverEnsureOrigin<Balance>`, which the unit tests do NOT exercise — the
//!   delta is irrelevant to the precompile's behavior and is covered by
//!   Plan 3's E2E suite against a live `impetus` runtime in Task 18.)
//! * wire `Paymaster = PayFromAccount<Balances, TreasuryAccount>` and
//!   `BalanceConverter = UnityAssetBalanceConversion` so `payout` against the
//!   `spend` flow has working plumbing, even though the unit tests focus on
//!   `spend_local`.

use alloc::vec::Vec;
use frame_support::{
    construct_runtime, parameter_types,
    traits::{
        tokens::{PayFromAccount, UnityAssetBalanceConversion},
        ConstU128, ConstU32, ConstU64, Everything, FindAuthor, VariantCountOf,
    },
    weights::Weight,
    PalletId,
};
use pallet_evm::{EnsureAddressNever, EnsureAddressRoot, IdentityAddressMapping};
use sp_core::{H160, U256};
use sp_runtime::{
    traits::{AccountIdConversion, BlakeTwo256, IdentityLookup},
    BuildStorage, ConsensusEngineId, Permill,
};

use crate::TreasuryPrecompileSet;

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
        Treasury:  pallet_treasury,
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
    pub PrecompilesValue: TreasuryPrecompileSet = TreasuryPrecompileSet::new();
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
    type PrecompilesType = TreasuryPrecompileSet;
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

// --- pallet-treasury config -------------------------------------------

parameter_types! {
    pub const TreasuryPalletId: PalletId = PalletId(*b"py/trsry");
    // Resolve once at runtime initialization so `PayFromAccount` does not
    // re-hash the pallet ID per call (matches the production pattern).
    pub TreasuryAccount: AccountId = TreasuryPalletId::get().into_account_truncating();
    pub const SpendPeriod: u64 = 10;
    pub const NoBurn: Permill = Permill::from_percent(0);
    pub const PayoutPeriod: u64 = 5;
    pub const MaxApprovals: u32 = 100;
}

impl pallet_treasury::Config for Runtime {
    type PalletId = TreasuryPalletId;
    type Currency = Balances;
    // `EnsureRoot` for both spend + reject so the precompile's
    // `RawOrigin::Root` dispatch lands. Production uses
    // `NeverEnsureOrigin<Balance>` for `SpendOrigin` (governance-gated); the
    // delta is irrelevant to the precompile's behavior.
    type RejectOrigin = frame_system::EnsureRoot<AccountId>;
    type RuntimeEvent = RuntimeEvent;
    type SpendPeriod = SpendPeriod;
    type Burn = NoBurn;
    type BurnDestination = ();
    type WeightInfo = ();
    type SpendFunds = ();
    type MaxApprovals = MaxApprovals;
    type SpendOrigin = frame_system::EnsureRootWithSuccess<AccountId, MaxBalance>;
    type AssetKind = ();
    type Beneficiary = AccountId;
    type BeneficiaryLookup = IdentityLookup<AccountId>;
    type Paymaster = PayFromAccount<Balances, TreasuryAccount>;
    type BalanceConverter = UnityAssetBalanceConversion;
    type PayoutPeriod = PayoutPeriod;
    type BlockNumberProvider = System;
    #[cfg(feature = "runtime-benchmarks")]
    type BenchmarkHelper = ();
}

parameter_types! {
    /// Max amount the root spend origin can authorize per call. Generous so
    /// tests never trip `InsufficientPermission` due to the cap.
    pub const MaxBalance: Balance = Balance::MAX;
}

// --- ext builder + helpers ---------------------------------------------

/// Sudo key used by sudo-gated happy-path tests.
pub const SUDO_KEY: H160 = H160(hex_literal::hex!(
    "00000000000000000000000000000000000000ff"
));

pub fn new_test_ext() -> sp_io::TestExternalities {
    let mut t = frame_system::GenesisConfig::<Runtime>::default()
        .build_storage()
        .unwrap();

    let treasury_account = TreasuryPalletId::get().into_account_truncating();

    // Pre-fund a few test accounts + the sudo key + the treasury pot so
    // `spend_local` -> `on_initialize` payouts have funds to disburse.
    let mut balances: Vec<(H160, Balance)> = (1u64..=20u64)
        .map(|i| (H160::from_low_u64_be(i), 1_000_000_000_000u128))
        .collect();
    balances.push((SUDO_KEY, 1_000_000_000_000u128));
    balances.push((treasury_account, 10_000_000u128));
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

/// Address of the treasury precompile in the mock.
pub fn treasury_addr() -> H160 {
    H160::from_low_u64_be(crate::PRECOMPILE_ADDRESS)
}

/// Helper: derived treasury pot account.
pub fn treasury_account() -> AccountId {
    TreasuryPalletId::get().into_account_truncating()
}
