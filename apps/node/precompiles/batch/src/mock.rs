#![cfg(test)]

use frame_support::{
    construct_runtime, parameter_types,
    traits::{ConstU128, ConstU32, ConstU64, Everything, FindAuthor},
    weights::Weight,
};
use pallet_evm::{EnsureAddressNever, EnsureAddressRoot, IdentityAddressMapping};
use sp_core::{H160, U256};
use sp_runtime::{
    traits::{BlakeTwo256, IdentityLookup},
    BuildStorage, ConsensusEngineId,
};

use crate::BatchPrecompileSet;

pub type Block = frame_system::mocking::MockBlock<Runtime>;
pub type AccountId = H160;
pub type Balance = u128;

construct_runtime!(
    pub enum Runtime {
        System:    frame_system,
        Balances:  pallet_balances,
        Timestamp: pallet_timestamp,
        EVM:       pallet_evm,
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
    type FreezeIdentifier = ();
    type MaxFreezes = ();
    type RuntimeHoldReason = ();
    type RuntimeFreezeReason = ();
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

pub struct FindAuthorAlice;
impl FindAuthor<H160> for FindAuthorAlice {
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
    pub PrecompilesValue: BatchPrecompileSet = BatchPrecompileSet::new();
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
    type PrecompilesType = BatchPrecompileSet;
    type PrecompilesValue = PrecompilesValue;
    type ChainId = ConstU64<322>;
    type BlockGasLimit = BlockGasLimit;
    type Runner = pallet_evm::runner::stack::Runner<Self>;
    type OnChargeTransaction = ();
    type OnCreate = ();
    type FindAuthor = FindAuthorAlice;
    type GasLimitPovSizeRatio = GasLimitPovSizeRatio;
    type GasLimitStorageGrowthRatio = GasLimitStorageGrowthRatio;
    type TransactionGasLimit = TransactionGasLimit;
    type Timestamp = Timestamp;
    type WeightInfo = ();
    type AccountProvider = pallet_evm::FrameSystemAccountProvider<Self>;
    type CreateOriginFilter = ();
    type CreateInnerOriginFilter = ();
}

pub fn new_test_ext() -> sp_io::TestExternalities {
    let t = frame_system::GenesisConfig::<Runtime>::default()
        .build_storage()
        .unwrap();
    let mut ext = sp_io::TestExternalities::new(t);
    ext.execute_with(|| System::set_block_number(1));
    ext
}
