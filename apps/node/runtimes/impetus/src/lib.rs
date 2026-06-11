//! Impetus mainnet runtime (chain id 388266).

#![cfg_attr(not(feature = "std"), no_std)]
// `construct_runtime!` does a lot of recursion and requires us to increase the limit to 256.
#![recursion_limit = "256"]
#![allow(clippy::new_without_default, clippy::or_fun_call)]
#![cfg_attr(feature = "runtime-benchmarks", warn(unused_crate_dependencies))]

extern crate alloc;

mod genesis_config_preset;
pub mod session_keys;
pub use session_keys::SessionKeys;
pub mod weights;

// Make the WASM binary available.
#[cfg(feature = "std")]
include!(concat!(env!("OUT_DIR"), "/wasm_binary.rs"));

pub use runtime_common::{
	opaque, AccountId, AccountIndex, Address, Balance, BlockHashCount, BlockLength, BlockNumber,
	BlockWeights, DigestItem, EnableManualSeal, Hash, Hashing,
	BLOCK_GAS_LIMIT, DAYS, EXISTENTIAL_DEPOSIT, HOURS, MAXIMUM_BLOCK_LENGTH, MAXIMUM_BLOCK_WEIGHT,
	MILLISECS_PER_BLOCK, MINUTES, NORMAL_DISPATCH_RATIO, Nonce, SLOT_DURATION, Signature,
	WEIGHT_MILLISECS_PER_BLOCK,
};
pub use runtime_common::{
	BONDING_DURATION_ERAS, MAX_NOMINATIONS, MAX_NOMINATORS_PER_VALIDATOR, MAX_VALIDATOR_COUNT,
	REPORT_LONGEVITY, SESSIONS_PER_ERA, SESSION_PERIOD,
	SLASH_DEFER_DURATION_ERAS, UNIT,
};

use alloc::{borrow::Cow, vec, vec::Vec};
use core::marker::PhantomData;
use ethereum::AuthorizationList;
use scale_codec::{Decode, Encode};
use sp_api::impl_runtime_apis;
use sp_consensus_babe::AuthorityId as BabeId;
use sp_consensus_grandpa::{AuthorityId as GrandpaId, AuthorityList as GrandpaAuthorityList};
use sp_core::{crypto::KeyTypeId, ConstU128, OpaqueMetadata, H160, H256, U256};
use sp_runtime::{
	generic,
	traits::{
		BlakeTwo256, Block as BlockT, DispatchInfoOf, Dispatchable, Get, IdentityLookup, NumberFor,
		PostDispatchInfoOf, UniqueSaturatedInto,
	},
	transaction_validity::{TransactionSource, TransactionValidity, TransactionValidityError},
	ApplyExtrinsicResult, ExtrinsicInclusionMode, Permill,
};
use sp_version::RuntimeVersion;
// Substrate FRAME
#[cfg(feature = "with-paritydb-weights")]
use frame_support::weights::constants::ParityDbWeight as RuntimeDbWeight;
#[cfg(feature = "with-rocksdb-weights")]
use frame_support::weights::constants::RocksDbWeight as RuntimeDbWeight;
use frame_support::{
	derive_impl,
	genesis_builder_helper::build_state,
	parameter_types,
	traits::{
		fungible,
		tokens::imbalance::{ResolveTo, Imbalance as _},
		ConstU32, ConstU64, ConstU8, Nothing, OnFinalize, OnUnbalanced,
	},
	weights::{ConstantMultiplier, IdentityFee, Weight},
	PalletId,
};
use sp_runtime::traits::ConvertInto;
use sp_staking::{EraIndex, SessionIndex};

// `frame_support::runtime` only resolves snake_case crate-path identifiers in
// the pallet declaration block (it cannot accept `pallet_session::historical`
// as a path). Re-alias the submodule so it can be named directly.
use pallet_session::historical as pallet_session_historical;
use pallet_transaction_payment::FungibleAdapter;
use polkadot_runtime_common::SlowAdjustingFeeUpdate;
use sp_genesis_builder::PresetId;
// Frontier
use fp_evm::weight_per_gas;
use fp_rpc::TransactionStatus;
use pallet_ethereum::{Call::transact, PostLogContent, Transaction as EthereumTransaction};
use pallet_evm::{
	Account as EVMAccount, EnsureAccountId20, FeeCalculator, IdentityAddressMapping, Runner,
};

// A few exports that help ease life for downstream crates.
pub use frame_system::Call as SystemCall;
pub use pallet_balances::Call as BalancesCall;
pub use pallet_timestamp::Call as TimestampCall;

/// Block header type as expected by this runtime.
pub type Header = generic::Header<BlockNumber, BlakeTwo256>;

/// Block type as expected by this runtime.
pub type Block = generic::Block<Header, UncheckedExtrinsic>;

/// A Block signed with a Justification
pub type SignedBlock = generic::SignedBlock<Block>;

/// BlockId type as expected by this runtime.
pub type BlockId = generic::BlockId<Block>;

/// The SignedExtension to the basic transaction logic.
pub type SignedExtra = cumulus_pallet_weight_reclaim::StorageWeightReclaim<
	Runtime,
	(
		frame_system::CheckNonZeroSender<Runtime>,
		frame_system::CheckSpecVersion<Runtime>,
		frame_system::CheckTxVersion<Runtime>,
		frame_system::CheckGenesis<Runtime>,
		frame_system::CheckEra<Runtime>,
		frame_system::CheckNonce<Runtime>,
		frame_system::CheckWeight<Runtime>,
		pallet_transaction_payment::ChargeTransactionPayment<Runtime>,
	),
>;

/// Unchecked extrinsic type as expected by this runtime.
pub type UncheckedExtrinsic =
	fp_self_contained::UncheckedExtrinsic<Address, RuntimeCall, Signature, SignedExtra>;

/// Extrinsic type that has already been checked.
pub type CheckedExtrinsic =
	fp_self_contained::CheckedExtrinsic<AccountId, RuntimeCall, SignedExtra, H160>;

/// The payload being signed in transactions.
pub type SignedPayload = generic::SignedPayload<RuntimeCall, SignedExtra>;

/// Executive: handles dispatch to the various modules.
pub type Executive = frame_executive::Executive<
	Runtime,
	Block,
	frame_system::ChainContext<Runtime>,
	Runtime,
	AllPalletsWithSystem,
>;

#[sp_version::runtime_version]
pub const VERSION: RuntimeVersion = RuntimeVersion {
	spec_name: Cow::Borrowed("impetus"),
	impl_name: Cow::Borrowed("impetus"),
	authoring_version: 1,
	// 6: M-1 production staking timing (28-day bonding), M-2 slashable genesis
	//    set, M-3 benchmarked WeightInfo, M-7 standard EVM precompiles 0x06..0x09.
	// 7: M-6 fee model — Substrate + EVM base fees split 80% treasury / 20%
	//    author; WeightToFee calibrated to ~1 gwei EVM transfer parity.
	// 8: tokenomics v1 — hard cap 1B IPT, staking inflation disabled
	//    (EraPayout = ()), fee split 50/50, pallet-vesting for team allocation.
	spec_version: 9,
	impl_version: 1,
	apis: RUNTIME_API_VERSIONS,
	// Bump ONLY when the extrinsic signing payload changes: call index
	// add/remove/reorder in `construct_runtime`, or a signed-extension change.
	// EVM-precompile registration (M-7), weight wiring (M-3), and staking-config
	// changes (M-1/M-2) do NOT alter extrinsic encoding, so this stays at 1 —
	// bumping it would force wallets/tooling to update signing logic for nothing.
	transaction_version: 1,
	system_version: 1,
};

/// The version information used to identify this runtime when compiled natively.
#[cfg(feature = "std")]
pub fn native_version() -> sp_version::NativeVersion {
	sp_version::NativeVersion {
		runtime_version: VERSION,
		can_author_with: Default::default(),
	}
}

parameter_types! {
	pub const Version: RuntimeVersion = VERSION;
	pub const SS58Prefix: u16 = 11434;
}

// Configure FRAME pallets to include in runtime.
#[derive_impl(frame_system::config_preludes::SolochainDefaultConfig as frame_system::DefaultConfig)]
impl frame_system::Config for Runtime {
	/// Block & extrinsics weights: base values and limits.
	type BlockWeights = BlockWeights;
	/// The maximum length of a block (in bytes).
	type BlockLength = BlockLength;
	/// The index type for storing how many extrinsics an account has signed.
	type Nonce = Nonce;
	/// The type for hashing blocks and tries.
	type Hash = Hash;
	/// The hashing algorithm used.
	type Hashing = Hashing;
	/// The identifier used to distinguish between accounts.
	type AccountId = AccountId;
	/// The lookup mechanism to get account ID from whatever is passed in dispatchers.
	type Lookup = IdentityLookup<AccountId>;
	/// The block type.
	type Block = Block;
	/// Maximum number of block number to block hash mappings to keep (oldest pruned first).
	type BlockHashCount = BlockHashCount;
	/// The weight of database operations that the runtime can invoke.
	type DbWeight = RuntimeDbWeight;
	/// Version of the runtime.
	type Version = Version;
	/// The data to be stored in an account.
	type AccountData = pallet_balances::AccountData<Balance>;
	/// SS58 prefix for the Impetus mainnet.
	type SS58Prefix = SS58Prefix;
	type MaxConsumers = ConstU32<16>;
}

parameter_types! {
	// BABE epoch length in slots. Session rotation is driven from this schedule
	// (ShouldEndSession = Babe), so one epoch == one session == SESSION_PERIOD.
	pub const EpochDuration: u64 = SESSION_PERIOD as u64;
	pub const ExpectedBlockTime: u64 = MILLISECS_PER_BLOCK;
	pub const MaxAuthorities: u32 = MAX_VALIDATOR_COUNT;
	pub const ReportLongevity: u64 = REPORT_LONGEVITY;
	// GRANDPA set-id -> session history kept for the full bonding window so
	// equivocations remain reportable across the entire unbonding period.
	// Both constants are already u32 (EraIndex / SessionIndex), so no cast.
	pub const BondingSessionEntries: u32 = BONDING_DURATION_ERAS * SESSIONS_PER_ERA;
}

impl pallet_babe::Config for Runtime {
	type EpochDuration = EpochDuration;
	type ExpectedBlockTime = ExpectedBlockTime;
	// Epoch enactment is externally triggered by pallet-session, whose rotation
	// is itself BABE-slot-driven (ShouldEndSession = Babe) -- the standard BABE
	// coupling. This keeps epoch boundaries on the slot clock, immune to missed
	// slots / block-height lag.
	type EpochChangeTrigger = pallet_babe::ExternalTrigger;
	type DisabledValidators = Session;
	// pallet-babe does not re-export its `weights` module; its `WeightInfo for
	// ()` impl already returns real reference-hardware weights for
	// check_equivocation_proof (~88ms), so `()` is the intended production
	// value, not a zero/placeholder.
	type WeightInfo = ();
	type MaxAuthorities = MaxAuthorities;
	type MaxNominators = ConstU32<MAX_NOMINATORS_PER_VALIDATOR>;
	type KeyOwnerProof = sp_session::MembershipProof;
	type EquivocationReportSystem = pallet_babe::EquivocationReportSystem<
		Self,
		Offences,
		Historical,
		ReportLongevity,
	>;
}

impl pallet_grandpa::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	// pallet-grandpa does not re-export its `weights` module; its
	// `WeightInfo for ()` impl already returns real reference-hardware weights
	// (check_equivocation_proof ~78ms, note_stalled with a DB write), so `()`
	// is the intended production value, not a zero/placeholder.
	type WeightInfo = ();
	type MaxAuthorities = MaxAuthorities;
	type MaxNominators = ConstU32<MAX_NOMINATORS_PER_VALIDATOR>;
	// Must cover the full slashable window: equivocation proofs reference a
	// past set id, and bonding is now 28 eras. Sizing this to BondingDuration
	// sessions keeps old set-id -> session mappings available long enough to
	// report and slash GRANDPA equivocations across the whole unbonding period.
	type MaxSetIdSessionEntries = BondingSessionEntries;
	type KeyOwnerProof = sp_session::MembershipProof;
	type EquivocationReportSystem = pallet_grandpa::EquivocationReportSystem<
		Self,
		Offences,
		Historical,
		ReportLongevity,
	>;
}

impl cumulus_pallet_weight_reclaim::Config for Runtime {
	// weight-reclaim keeps its `weights` module private; its `WeightInfo for ()`
	// impl carries real reference-hardware weights, so `()` is the intended
	// production value here.
	type WeightInfo = ();
}

impl pallet_timestamp::Config for Runtime {
	type Moment = u64;
	// Babe validates the timestamp on each block; delegate slot-alignment to it.
	type OnTimestampSet = Babe;
	type MinimumPeriod = ConstU64<{ SLOT_DURATION / 2 }>;
	type WeightInfo = pallet_timestamp::weights::SubstrateWeight<Runtime>;
}

impl pallet_balances::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeHoldReason = RuntimeHoldReason;
	type RuntimeFreezeReason = RuntimeFreezeReason;
	type WeightInfo = pallet_balances::weights::SubstrateWeight<Self>;
	type Balance = Balance;
	type DustRemoval = ();
	type ExistentialDeposit = ConstU128<EXISTENTIAL_DEPOSIT>;
	type AccountStore = System;
	type ReserveIdentifier = [u8; 8];
	type FreezeIdentifier = RuntimeFreezeReason;
	type MaxLocks = ConstU32<50>;
	type MaxReserves = ConstU32<50>;
	type MaxFreezes = ConstU32<1>;
	type DoneSlashHandler = ();
}

// --- M-6: fee model -------------------------------------------------------
//
// Transaction fees (Substrate dispatch fees AND the EVM EIP-1559 base fee) are
// split 50% to the Treasury pot / 50% to the current block author. Tips /
// priority fees always go 100% to the author (handled by the adapters, not by
// `DealWithFees`). Nothing is burned. The author share is raised from 20%->50%
// because staking inflation is disabled (EraPayout = ()) so validators rely
// entirely on fees for income. This applies to impetus only; impulse keeps the
// upstream burn behaviour (it has no treasury).

/// Account getter that resolves to the current block author, falling back to
/// the treasury account when no author is set yet (e.g. genesis / pre-authoring
/// edge) so the 20% author share is never silently dropped.
pub struct AuthorOrTreasury;
impl frame_support::traits::TypedGet for AuthorOrTreasury {
	type Type = AccountId;
	fn get() -> AccountId {
		pallet_authorship::Pallet::<Runtime>::author().unwrap_or_else(TreasuryAccount::get)
	}
}

/// `OnUnbalanced` handler for the fee credit: 80% to treasury, 20% to author.
pub struct DealWithFees;
impl OnUnbalanced<fungible::Credit<AccountId, Balances>> for DealWithFees {
	fn on_nonzero_unbalanced(amount: fungible::Credit<AccountId, Balances>) {
		// ration(50, 50): equal split — validators earn more from fees since
		// staking inflation is disabled (EraPayout = ()).
		let (to_treasury, to_author) = amount.ration(50, 50);
		ResolveTo::<TreasuryAccount, Balances>::on_unbalanced(to_treasury);
		ResolveTo::<AuthorOrTreasury, Balances>::on_unbalanced(to_author);
	}
}

parameter_types! {
	// Calibrate Substrate dispatch fees to roughly match a 1-gwei EVM transfer
	// (~0.00002 IPT). A balances transfer is ~4e8 ref-time weight units and the
	// target fee is ~2e13 planck, so the multiplier is ~2e13 / 4e8 = 50_000.
	// (LengthToFee stays IdentityFee so byte cost does not dominate.)
	pub const FeeMultiplier: Balance = 50_000;
}

impl pallet_transaction_payment::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type OnChargeTransaction = FungibleAdapter<Balances, DealWithFees>;
	type WeightToFee = ConstantMultiplier<Balance, FeeMultiplier>;
	type LengthToFee = IdentityFee<Balance>;
	/// Parameterized slow adjusting fee updated based on
	/// <https://research.web3.foundation/Polkadot/overview/token-economics#2-slow-adjusting-mechanism>
	type FeeMultiplierUpdate = SlowAdjustingFeeUpdate<Runtime>;
	type OperationalFeeMultiplier = ConstU8<5>;
	type WeightInfo = pallet_transaction_payment::weights::SubstrateWeight<Runtime>;
}

impl pallet_sudo::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeCall = RuntimeCall;
	type WeightInfo = pallet_sudo::weights::SubstrateWeight<Self>;
}

parameter_types! {
	pub const AssetDeposit: Balance = 100;
	pub const AssetAccountDeposit: Balance = 1;
	pub const ApprovalDeposit: Balance = 1;
	pub const MetadataDepositBase: Balance = 10;
	pub const MetadataDepositPerByte: Balance = 1;
}

impl pallet_assets::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Balance = Balance;
	type AssetId = u32;
	type AssetIdParameter = scale_codec::Compact<u32>;
	type Currency = Balances;
	type CreateOrigin = frame_support::traits::AsEnsureOriginWithArg<frame_system::EnsureSigned<AccountId>>;
	type ForceOrigin = frame_system::EnsureRoot<AccountId>;
	type AssetDeposit = AssetDeposit;
	type AssetAccountDeposit = AssetAccountDeposit;
	type MetadataDepositBase = MetadataDepositBase;
	type MetadataDepositPerByte = MetadataDepositPerByte;
	type ApprovalDeposit = ApprovalDeposit;
	type StringLimit = ConstU32<50>;
	type Freezer = ();
	type Extra = ();
	type CallbackHandle = ();
	type WeightInfo = pallet_assets::weights::SubstrateWeight<Self>;
	type RemoveItemsLimit = ConstU32<1000>;
	type ReserveData = ();
	type Holder = ();
	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkHelper = ();
}

impl pallet_evm_chain_id::Config for Runtime {}

parameter_types! {
	pub BlockGasLimit: U256 = U256::from(BLOCK_GAS_LIMIT);
	pub const GasLimitPovSizeRatio: u64 = BLOCK_GAS_LIMIT.saturating_div(runtime_common::MAX_POV_SIZE);
	pub const GasLimitStorageGrowthRatio: u64 = BLOCK_GAS_LIMIT.saturating_div(runtime_common::MAX_STORAGE_GROWTH);
	pub PrecompilesValue: runtime_common::FrontierPrecompilesNpos<Runtime> = runtime_common::FrontierPrecompilesNpos::<_>::new();
	pub WeightPerGas: Weight = Weight::from_parts(weight_per_gas(BLOCK_GAS_LIMIT, NORMAL_DISPATCH_RATIO, WEIGHT_MILLISECS_PER_BLOCK), 0);
	pub TransactionGasLimit: Option<U256> = Some(fp_evm::MAX_TRANSACTION_GAS_LIMIT);
}

impl pallet_evm::Config for Runtime {
	type AccountProvider = pallet_evm::FrameSystemAccountProvider<Self>;
	type FeeCalculator = BaseFee;
	type GasWeightMapping = pallet_evm::FixedGasWeightMapping<Self>;
	type WeightPerGas = WeightPerGas;
	type BlockHashMapping = pallet_ethereum::EthereumBlockHashMapping<Self>;
	type CallOrigin = EnsureAccountId20;
	type WithdrawOrigin = EnsureAccountId20;
	type AddressMapping = IdentityAddressMapping;
	type Currency = Balances;
	type PrecompilesType = runtime_common::FrontierPrecompilesNpos<Self>;
	type PrecompilesValue = PrecompilesValue;
	type ChainId = EVMChainId;
	type BlockGasLimit = BlockGasLimit;
	type TransactionGasLimit = TransactionGasLimit;
	type Runner = runtime_common::GaslessEvmRunner<Self>;
	// Gasless waiver wraps the real charger: matched gasless calls pay nothing;
	// everything else routes the EVM base fee through DealWithFees (80% treasury
	// / 20% author). Tips still go 100% to the author inside EVMFungibleAdapter.
	type OnChargeTransaction =
		runtime_common::GaslessEvmFee<Self, pallet_evm::EVMFungibleAdapter<Balances, DealWithFees>>;
	type OnCreate = ();
	type FindAuthor = runtime_common::FindAuthorFromAuthorship<Self>;
	type GasLimitPovSizeRatio = GasLimitPovSizeRatio;
	type GasLimitStorageGrowthRatio = GasLimitStorageGrowthRatio;
	type Timestamp = Timestamp;
	type CreateOriginFilter = ();
	type CreateInnerOriginFilter = ();
	type WeightInfo = pallet_evm::weights::SubstrateWeight<Self>;
}

parameter_types! {
	pub const PostBlockAndTxnHashes: PostLogContent = PostLogContent::BlockAndTxnHashes;
	pub const AllowUnprotectedTxs: bool = false;
}

impl pallet_ethereum::Config for Runtime {
	type StateRoot = pallet_ethereum::IntermediateStateRoot<Self::Version>;
	type PostLogContent = PostBlockAndTxnHashes;
	type ExtraDataLength = ConstU32<30>;
	type AllowUnprotectedTxs = AllowUnprotectedTxs;
}

parameter_types! {
	pub BoundDivision: U256 = U256::from(1024);
}

impl pallet_dynamic_fee::Config for Runtime {
	type MinGasPriceBoundDivisor = BoundDivision;
}

parameter_types! {
	pub DefaultBaseFeePerGas: U256 = U256::from(1_000_000_000);
	pub DefaultElasticity: Permill = Permill::from_parts(125_000);
}
impl pallet_base_fee::Config for Runtime {
	type Threshold = runtime_common::BaseFeeThreshold;
	type DefaultBaseFeePerGas = DefaultBaseFeePerGas;
	type DefaultElasticity = DefaultElasticity;
}

impl runtime_common::pallet_manual_seal::Config for Runtime {}

parameter_types! {
	pub const MaxGaslessGasLimit: u64 = 5_000_000;
}

/// Babe primary VRF slot ratio (1 in 4). Permissive enough that secondary
/// slots cover any epoch where no primary winner emerges.
pub const BABE_GENESIS_EPOCH_CONFIG: sp_consensus_babe::BabeEpochConfiguration =
	sp_consensus_babe::BabeEpochConfiguration {
		c: (1, 4),
		allowed_slots: sp_consensus_babe::AllowedSlots::PrimaryAndSecondaryVRFSlots,
	};

impl pallet_gasless_registry::Config for Runtime {
	type ManageOrigin = frame_system::EnsureRoot<AccountId>;
	type MaxGaslessGasLimit = MaxGaslessGasLimit;
	// Custom pallet: its `WeightInfo for ()` impl already returns real
	// (non-zero) RocksDb-based weights for set_rule/remove_rule/evaluate, so
	// `()` is the intended production value here — not a placeholder. Replace
	// with a generated `weights::WeightInfo<Runtime>` only after running the
	// pallet's own benchmark on reference hardware.
	type WeightInfo = ();
}

// =====================================================================
// NPoS pallet stack (Plan 2). Pallets land in a single atomic commit
// because individual configs do not compile until `construct_runtime!`
// names them. Ordering mirrors the dependency DAG: session → staking →
// epm/offences/bags-list → im-online/authority-discovery → treasury →
// pools/fast-unstake → authorship (re-impl with FindAuthor=Session).
// =====================================================================

// --- T7: pallet-session + historical ---------------------------------

parameter_types! {
	// Note: session rotation is BABE-epoch-driven (see pallet_session::Config),
	// so no block-based `Period`/`Offset` parameters are needed here.
	pub const SessionsPerEra: SessionIndex = SESSIONS_PER_ERA;
	pub const BondingDuration: EraIndex = BONDING_DURATION_ERAS;
	pub const SlashDeferDuration: EraIndex = SLASH_DEFER_DURATION_ERAS;
}

impl pallet_session::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type ValidatorId = AccountId;
	// `ValidatorId == AccountId`, so the identity converter is correct here.
	type ValidatorIdOf = ConvertInto;
	// BABE chains MUST drive session rotation from BABE's slot-based epoch
	// schedule, not block-counting `PeriodicSessions`. With block-based sessions,
	// any missed slot makes block height lag the slot clock, so BABE's slot-based
	// epoch boundary arrives before the block-based session boundary and the
	// runtime never emits the NextEpochDescriptor digest the client expects ->
	// "Expected epoch change" import failure / chain halt at the first epoch.
	// Matches every BABE chain (Polkadot/Westend/Rococo, substrate node,
	// Joystream, Humanode): ShouldEndSession = NextSessionRotation = Babe.
	type ShouldEndSession = Babe;
	type NextSessionRotation = Babe;
	type SessionManager = pallet_session::historical::NoteHistoricalRoot<Self, Staking>;
	type SessionHandler = <SessionKeys as sp_runtime::traits::OpaqueKeys>::KeyTypeIdProviders;
	type Keys = SessionKeys;
	type DisablingStrategy = pallet_session::disabling::UpToLimitWithReEnablingDisablingStrategy;
	type WeightInfo = pallet_session::weights::SubstrateWeight<Runtime>;
	type Currency = Balances;
	type KeyDeposit = ();
}

impl pallet_session::historical::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	// We track the validator set with no extra identification data; staking
	// re-derives exposures from its own snapshots.
	type FullIdentification = ();
	type FullIdentificationOf = pallet_staking::UnitIdentificationOf<Self>;
}

// --- T8: pallet-staking ----------------------------------------------

parameter_types! {
	pub const MaxExposurePageSize: u32 = MAX_NOMINATORS_PER_VALIDATOR;
	pub const MaxUnlockingChunks: u32 = 32;
	pub const HistoryDepth: u32 = 84;
	pub const MaxControllersInDeprecationBatch: u32 = 256;
	pub const MaxValidatorSet: u32 = MAX_VALIDATOR_COUNT;
}

pub struct StakingBenchmarkingConfig;
impl pallet_staking::BenchmarkingConfig for StakingBenchmarkingConfig {
	type MaxNominators = ConstU32<1000>;
	type MaxValidators = ConstU32<1000>;
}

impl pallet_staking::Config for Runtime {
	// `OldCurrency` is the deprecated lockable-balance API. Plan 2 chains
	// have never run the legacy lock pallet, but the trait bound still
	// requires a concrete `InspectLockableCurrency`; `Balances` satisfies
	// both old and new (`FunHoldMutate`) requirements.
	type OldCurrency = Balances;
	type Currency = Balances;
	type RuntimeHoldReason = RuntimeHoldReason;
	type CurrencyBalance = Balance;
	type UnixTime = Timestamp;
	type CurrencyToVote = sp_staking::currency_to_vote::SaturatingCurrencyToVote;
	// `pallet-staking` requires `OnUnbalanced<NegativeImbalance>` (Currency-based);
	// `Treasury` itself implements the newer fungible imbalance shape. Forward
	// slashes/leftovers to the treasury account directly via `ResolveTo`.
	type RewardRemainder =
		frame_support::traits::tokens::imbalance::ResolveTo<TreasuryAccount, Balances>;
	type RuntimeEvent = RuntimeEvent;
	type Slash =
		frame_support::traits::tokens::imbalance::ResolveTo<TreasuryAccount, Balances>;
	type Reward = ();
	type SessionsPerEra = SessionsPerEra;
	type BondingDuration = BondingDuration;
	type SlashDeferDuration = SlashDeferDuration;
	type AdminOrigin = frame_system::EnsureRoot<AccountId>;
	type SessionInterface = Self;
	// Hard cap: no staking inflation. EraPayout = () returns (0, 0) for every era
	// so total issuance stays fixed at the 1B genesis supply. Validators are
	// compensated entirely through the 50% fee share (DealWithFees above).
	// RewardRemainder/Reward/Slash are unchanged; remainder is a no-op (0 surplus).
	type EraPayout = ();
	type NextNewSession = Session;
	type MaxExposurePageSize = MaxExposurePageSize;
	type MaxValidatorSet = MaxValidatorSet;
	type ElectionProvider = ElectionProviderMultiPhase;
	type GenesisElectionProvider =
		frame_election_provider_support::onchain::OnChainExecution<
			runtime_common::OnChainSeqPhragmen<Self>,
		>;
	type VoterList = VoterList;
	type TargetList = pallet_staking::UseValidatorsMap<Self>;
	type NominationsQuota = pallet_staking::FixedNominationsQuota<MAX_NOMINATIONS>;
	type MaxUnlockingChunks = MaxUnlockingChunks;
	type MaxControllersInDeprecationBatch = MaxControllersInDeprecationBatch;
	type HistoryDepth = HistoryDepth;
	type EventListeners = NominationPools;
	type Filter = Nothing;
	type BenchmarkingConfig = StakingBenchmarkingConfig;
	type WeightInfo = pallet_staking::weights::SubstrateWeight<Runtime>;
}

// --- T9: pallet-offences + pallet-bags-list --------------------------

impl pallet_offences::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type IdentificationTuple = pallet_session::historical::IdentificationTuple<Self>;
	type OnOffenceHandler = Staking;
}

parameter_types! {
	pub const BagThresholds: &'static [sp_npos_elections::VoteWeight] =
		&runtime_common::voter_bags::THRESHOLDS;
}

impl pallet_bags_list::Config<pallet_bags_list::Instance1> for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type ScoreProvider = Staking;
	type BagThresholds = BagThresholds;
	type Score = sp_npos_elections::VoteWeight;
	// Disable auto-rebagging; the dev-fast NPoS chain has small bag counts
	// and rebalance pressure is low.
	type MaxAutoRebagPerBlock = ();
	type WeightInfo = pallet_bags_list::weights::SubstrateWeight<Runtime>;
}

// --- T10: pallet-election-provider-multi-phase -----------------------

parameter_types! {
	pub const SignedPhase: BlockNumber = 25;
	pub const UnsignedPhase: BlockNumber = 25;
	pub const SignedMaxSubmissions: u32 = 10;
	pub const SignedMaxRefunds: u32 = 5;
	pub const SignedMaxWeight: Weight =
		Weight::from_parts(WEIGHT_MILLISECS_PER_BLOCK * 200, u64::MAX);
	pub const MinerMaxLength: u32 = 4 * 1024 * 1024;
	pub const MinerMaxWeight: Weight =
		Weight::from_parts(WEIGHT_MILLISECS_PER_BLOCK * 200, u64::MAX);
	pub const MinerTxPriority: u64 = 5_000_000_000;
	pub const OffchainRepeat: BlockNumber = 5;
	// 10% geometric per-submission deposit growth, matching Polkadot's default.
	pub const SignedDepositIncreaseFactor: sp_runtime::Percent =
		sp_runtime::Percent::from_percent(10);
}

frame_election_provider_support::generate_solution_type!(
	#[compact]
	pub struct NposCompactSolution16::<
		VoterIndex = u32,
		TargetIndex = u16,
		Accuracy = sp_runtime::PerU16,
		MaxVoters = runtime_common::MaxOnChainElectingVoters,
	>(16)
);

// stable2603 wired `OnChainExecution` so it directly implements both
// `ElectionProvider` and `InstantElectionProvider`; reuse it as both
// `Fallback` and `GovernanceFallback` instead of hand-rolling another
// wrapper that would need every new associated type the trait grew.
pub type EpmOnChainFallback =
	frame_election_provider_support::onchain::OnChainExecution<
		runtime_common::OnChainSeqPhragmen<Runtime>,
	>;

// stable2603 dropped the public `NoopElectionProviderBenchmarkingConfig`
// helper, so define a local equivalent. The values are conservative and
// only consumed when compiling with `--features runtime-benchmarks`.
pub struct EpmBenchmarkConfig;
impl pallet_election_provider_multi_phase::BenchmarkingConfig for EpmBenchmarkConfig {
	const VOTERS: [u32; 2] = [1000, 2000];
	const TARGETS: [u32; 2] = [500, 1000];
	const ACTIVE_VOTERS: [u32; 2] = [500, 800];
	const DESIRED_TARGETS: [u32; 2] = [200, 400];
	const SNAPSHOT_MAXIMUM_VOTERS: u32 = 1000;
	const MINER_MAXIMUM_VOTERS: u32 = 1000;
	const MAXIMUM_TARGETS: u32 = 300;
}

impl pallet_election_provider_multi_phase::MinerConfig for Runtime {
	type AccountId = AccountId;
	type MaxLength = MinerMaxLength;
	type MaxWeight = MinerMaxWeight;
	type Solution = NposCompactSolution16;
	type MaxVotesPerVoter =
		<<Self as pallet_election_provider_multi_phase::Config>::DataProvider
			as frame_election_provider_support::ElectionDataProvider>::MaxVotesPerVoter;
	type MaxWinners = runtime_common::MaxActiveValidators;
	type MaxBackersPerWinner = runtime_common::staking_election::MaxBackersPerWinner;

	fn solution_weight(_v: u32, _t: u32, _a: u32, _d: u32) -> Weight {
		MinerMaxWeight::get()
	}
}

// `pallet-election-provider-multi-phase` requires `CreateBare` so the
// off-chain miner can submit its unsigned solution. With the Frontier
// self-contained extrinsic envelope we synthesize a bare extrinsic by
// hand.
impl<LocalCall> frame_system::offchain::CreateTransactionBase<LocalCall> for Runtime
where
	RuntimeCall: From<LocalCall>,
{
	type RuntimeCall = RuntimeCall;
	type Extrinsic = UncheckedExtrinsic;
}

impl<LocalCall> frame_system::offchain::CreateBare<LocalCall> for Runtime
where
	RuntimeCall: From<LocalCall>,
{
	fn create_bare(call: Self::RuntimeCall) -> Self::Extrinsic {
		UncheckedExtrinsic::new_bare(call)
	}
}

impl pallet_election_provider_multi_phase::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type EstimateCallFee = TransactionPayment;
	type SignedPhase = SignedPhase;
	type UnsignedPhase = UnsignedPhase;
	type BetterSignedThreshold = runtime_common::staking_election::BetterSignedThreshold;
	type OffchainRepeat = OffchainRepeat;
	type MinerTxPriority = MinerTxPriority;
	type SignedMaxSubmissions = SignedMaxSubmissions;
	type SignedRewardBase = runtime_common::staking_election::SignedRewardBase;
	type SignedDepositBase = pallet_election_provider_multi_phase::GeometricDepositBase<
		Balance,
		runtime_common::staking_election::SignedDepositBase,
		SignedDepositIncreaseFactor,
	>;
	type SignedDepositByte = runtime_common::staking_election::SignedDepositByte;
	type SignedDepositWeight = ();
	type SignedMaxWeight = SignedMaxWeight;
	type SignedMaxRefunds = SignedMaxRefunds;
	// `pallet-election-provider-multi-phase` still uses the legacy
	// `Currency::NegativeImbalance`, which is incompatible with the modern
	// `ResolveTo<TreasuryAccount, Balances>` (fungible-credit shaped). Drop
	// EPM signed-deposit slashes for Plan 2; future plans can wire a
	// dedicated handler once governance ships.
	type SlashHandler = ();
	type RewardHandler = ();
	type MinerConfig = Self;
	type DataProvider = Staking;
	type Fallback = EpmOnChainFallback;
	type GovernanceFallback = EpmOnChainFallback;
	type Solver = frame_election_provider_support::SequentialPhragmen<
		AccountId,
		pallet_election_provider_multi_phase::SolutionAccuracyOf<Self>,
	>;
	type BenchmarkingConfig = EpmBenchmarkConfig;
	type ForceOrigin = frame_system::EnsureRoot<AccountId>;
	type MaxWinners = runtime_common::MaxActiveValidators;
	type MaxBackersPerWinner = runtime_common::staking_election::MaxBackersPerWinner;
	type ElectionBounds = runtime_common::ElectionBoundsMultiPhase;
	type WeightInfo = pallet_election_provider_multi_phase::weights::SubstrateWeight<Runtime>;
}

// --- T11: pallet-im-online + pallet-authority-discovery --------------

parameter_types! {
	pub const ImOnlineUnsignedPriority: u64 = u64::MAX / 2;
	pub const MaxKeys: u32 = 1024;
	pub const MaxPeerInHeartbeats: u32 = 10_000;
}

impl pallet_im_online::Config for Runtime {
	type AuthorityId = pallet_im_online::sr25519::AuthorityId;
	type RuntimeEvent = RuntimeEvent;
	// Slot-based, aligned with BABE epochs (see pallet_session::Config above).
	type NextSessionRotation = Babe;
	type ValidatorSet = Historical;
	type ReportUnresponsiveness = Offences;
	type UnsignedPriority = ImOnlineUnsignedPriority;
	type WeightInfo = pallet_im_online::weights::SubstrateWeight<Runtime>;
	type MaxKeys = MaxKeys;
	type MaxPeerInHeartbeats = MaxPeerInHeartbeats;
}

impl pallet_authority_discovery::Config for Runtime {
	type MaxAuthorities = MaxAuthorities;
}

// --- T12: pallet-treasury --------------------------------------------

parameter_types! {
	pub const SpendPeriod: BlockNumber = HOURS;
	pub const Burn: Permill = Permill::from_percent(0);
	pub const TreasuryPalletId: PalletId = PalletId(*b"py/trsry");
	// Resolve the treasury account once at runtime initialization so
	// `Paymaster = PayFromAccount<Balances, TreasuryAccount>` does not need to
	// hash the pallet ID per call. `AccountIdConversion` is impl-ed on the
	// `PalletId` value, not on the getter — hence the explicit `::get()`.
	pub TreasuryAccount: AccountId =
		<PalletId as sp_runtime::traits::AccountIdConversion<AccountId>>
			::into_account_truncating(&TreasuryPalletId::get());
	pub const MaxApprovals: u32 = 100;
	pub const PayoutPeriod: BlockNumber = 30 * DAYS;
	/// Maximum amount per `spend_local` / `spend` call when dispatched via
	/// `SpendOrigin = EnsureRootWithSuccess`. Set to `u128::MAX` so Root can
	/// transfer any pot balance; Plan 4 will tighten this when a governance
	/// origin replaces Root.
	pub const MaxSpendOriginAmount: Balance = Balance::MAX;
}

impl pallet_treasury::Config for Runtime {
	type PalletId = TreasuryPalletId;
	type Currency = Balances;
	type RejectOrigin = frame_system::EnsureRoot<AccountId>;
	type RuntimeEvent = RuntimeEvent;
	type SpendPeriod = SpendPeriod;
	type Burn = Burn;
	type BurnDestination = ();
	type SpendFunds = ();
	type MaxApprovals = MaxApprovals;
	type WeightInfo = pallet_treasury::weights::SubstrateWeight<Runtime>;
	// Plan 3 (T18): unlock `spend_local` / `void_spend` for the T6 treasury
	// precompile. The precompile dispatches as `RawOrigin::Root` after a
	// sudo-only gate, so `SpendOrigin` must accept Root. Plan 2's
	// `NeverEnsureOrigin` placeholder permanently rejected every origin —
	// including Root — which made the precompile's write entries
	// undispatchable in production. `EnsureRootWithSuccess<.., u128::MAX>`
	// allows Root to spend up to the maximum balance; Plan 4 (governance v2)
	// will replace this with a council / referenda origin.
	type SpendOrigin = frame_system::EnsureRootWithSuccess<AccountId, MaxSpendOriginAmount>;
	type AssetKind = ();
	type Beneficiary = AccountId;
	type BeneficiaryLookup = sp_runtime::traits::IdentityLookup<AccountId>;
	type Paymaster = frame_support::traits::tokens::PayFromAccount<Balances, TreasuryAccount>;
	type BalanceConverter = frame_support::traits::tokens::UnityAssetBalanceConversion;
	type PayoutPeriod = PayoutPeriod;
	type BlockNumberProvider = System;
	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkHelper = TreasuryBenchmarkHelper;
}

/// Treasury benchmark argument factory. Required because `AccountId20`
/// (the `fp_account` H160-shaped account) does not implement `FromEntropy`,
/// so the default `()` `ArgumentsFactory` impl from `pallet-treasury` cannot
/// derive a benchmark beneficiary. Hashes the upstream 32-byte seed with
/// `blake2_256` and takes the first 20 bytes; rehashing (rather than slicing
/// the raw seed) keeps the address distribution uniform across the small
/// seed sequence the bench harness emits.
#[cfg(feature = "runtime-benchmarks")]
pub struct TreasuryBenchmarkHelper;

#[cfg(feature = "runtime-benchmarks")]
impl pallet_treasury::ArgumentsFactory<(), AccountId> for TreasuryBenchmarkHelper {
	fn create_asset_kind(_seed: u32) {}
	fn create_beneficiary(seed: [u8; 32]) -> AccountId {
		use sp_runtime::traits::Hash;
		let hashed = <Hashing as Hash>::hash(&seed);
		let mut bytes = [0u8; 20];
		bytes.copy_from_slice(&hashed.as_bytes()[..20]);
		H160::from(bytes).into()
	}
}

// --- T13: pallet-nomination-pools + pallet-fast-unstake --------------

parameter_types! {
	pub const PoolsPalletId: PalletId = PalletId(*b"py/nopls");
	pub const MaxPointsToBalance: u8 = 10;
}

impl pallet_nomination_pools::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = pallet_nomination_pools::weights::SubstrateWeight<Runtime>;
	type Currency = Balances;
	type RuntimeFreezeReason = RuntimeFreezeReason;
	type RewardCounter = sp_runtime::FixedU128;
	type BalanceToU256 = runtime_common::staking_election::BalanceToU256;
	type U256ToBalance = runtime_common::staking_election::U256ToBalance;
	// `TransferStake` is the legacy strategy that copies funds in/out of
	// the pool's bonded account. The newer `DelegateStake` strategy
	// requires `pallet-delegated-staking` which Plan 2 does not ship.
	#[allow(deprecated)]
	type StakeAdapter = pallet_nomination_pools::adapter::TransferStake<Self, Staking>;
	type PostUnbondingPoolsWindow = ConstU32<4>;
	type MaxMetadataLen = ConstU32<256>;
	type MaxUnbonding = ConstU32<8>;
	type PalletId = PoolsPalletId;
	type MaxPointsToBalance = MaxPointsToBalance;
	type AdminOrigin = frame_system::EnsureRoot<AccountId>;
	type BlockNumberProvider = System;
	type Filter = Nothing;
}

parameter_types! {
	pub const FastUnstakeDeposit: Balance = UNIT;
	pub const FastUnstakeBatchSize: u32 = 4;
}

impl pallet_fast_unstake::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type ControlOrigin = frame_system::EnsureRoot<AccountId>;
	type BatchSize = FastUnstakeBatchSize;
	type Deposit = FastUnstakeDeposit;
	type Currency = Balances;
	type Staking = Staking;
	type MaxErasToCheckPerBlock = ConstU32<1>;
	type WeightInfo = pallet_fast_unstake::weights::SubstrateWeight<Runtime>;
}

// --- T14: pallet-vesting ---------------------------------------------

parameter_types! {
	/// Minimum amount that can be locked in a vesting schedule (1 IPT).
	pub const MinVestedTransfer: Balance = UNIT;
	/// Maximum concurrent vesting schedules per account.
	pub const MaxVestingSchedules: u32 = 28;
	/// Vested (still-locked) funds may be used for everything EXCEPT transfer and
	/// reserve — i.e. they can pay fees but cannot be moved out or locked
	/// elsewhere until they vest. Matches the Polkadot/Westend convention.
	pub UnvestedFundsAllowedWithdrawReasons: frame_support::traits::WithdrawReasons =
		frame_support::traits::WithdrawReasons::except(
			frame_support::traits::WithdrawReasons::TRANSFER
				| frame_support::traits::WithdrawReasons::RESERVE,
		);
}

impl pallet_vesting::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type BlockNumberToBalance = sp_runtime::traits::ConvertInto;
	type MinVestedTransfer = MinVestedTransfer;
	type WeightInfo = pallet_vesting::weights::SubstrateWeight<Runtime>;
	type UnvestedFundsAllowedWithdrawReasons = UnvestedFundsAllowedWithdrawReasons;
	// Solo chain — use the local frame_system block number directly.
	type BlockNumberProvider = System;
	const MAX_VESTING_SCHEDULES: u32 = 28;
}

// --- T15: pallet-authorship (FindAuthor swap) ------------------------

impl pallet_authorship::Config for Runtime {
	type FindAuthor = pallet_session::FindAccountFromAuthorIndex<Self, Babe>;
	type EventHandler = (Staking, ImOnline);
}

// Create the runtime by composing the FRAME pallets that were previously configured.
#[frame_support::runtime]
mod runtime {
	#[runtime::runtime]
	#[runtime::derive(
		RuntimeEvent,
		RuntimeCall,
		RuntimeError,
		RuntimeOrigin,
		RuntimeFreezeReason,
		RuntimeHoldReason,
		RuntimeSlashReason,
		RuntimeLockId,
		RuntimeTask
	)]
	pub struct Runtime;

	#[runtime::pallet_index(0)]
	pub type System = frame_system;

	#[runtime::pallet_index(1)]
	pub type Timestamp = pallet_timestamp;

	#[runtime::pallet_index(2)]
	pub type Babe = pallet_babe;

	#[runtime::pallet_index(3)]
	pub type Grandpa = pallet_grandpa;

	#[runtime::pallet_index(4)]
	pub type Balances = pallet_balances;

	#[runtime::pallet_index(5)]
	pub type TransactionPayment = pallet_transaction_payment;

	#[runtime::pallet_index(6)]
	pub type Sudo = pallet_sudo;

	#[runtime::pallet_index(7)]
	pub type Ethereum = pallet_ethereum;

	#[runtime::pallet_index(8)]
	pub type EVM = pallet_evm;

	#[runtime::pallet_index(9)]
	pub type EVMChainId = pallet_evm_chain_id;

	#[runtime::pallet_index(10)]
	pub type BaseFee = pallet_base_fee;

	#[runtime::pallet_index(11)]
	pub type ManualSeal = runtime_common::pallet_manual_seal;

	#[runtime::pallet_index(12)]
	pub type Assets = pallet_assets;

	#[runtime::pallet_index(13)]
	pub type Vesting = pallet_vesting;

	#[runtime::pallet_index(14)]
	pub type GaslessRegistry = pallet_gasless_registry;

	#[runtime::pallet_index(17)]
	pub type Authorship = pallet_authorship;

	#[runtime::pallet_index(18)]
	pub type Session = pallet_session;

	#[runtime::pallet_index(19)]
	pub type Historical = pallet_session_historical;

	#[runtime::pallet_index(20)]
	pub type Staking = pallet_staking;

	#[runtime::pallet_index(21)]
	pub type Offences = pallet_offences;

	#[runtime::pallet_index(22)]
	pub type ElectionProviderMultiPhase = pallet_election_provider_multi_phase;

	#[runtime::pallet_index(23)]
	pub type VoterList = pallet_bags_list<Instance1>;

	#[runtime::pallet_index(24)]
	pub type AuthorityDiscovery = pallet_authority_discovery;

	#[runtime::pallet_index(25)]
	pub type ImOnline = pallet_im_online;

	#[runtime::pallet_index(26)]
	pub type NominationPools = pallet_nomination_pools;

	#[runtime::pallet_index(27)]
	pub type Treasury = pallet_treasury;

	#[runtime::pallet_index(28)]
	pub type FastUnstake = pallet_fast_unstake;
}

/// Wraps an Ethereum extrinsic into the runtime's `UncheckedExtrinsic`. The
/// generic `B: BlockT` keeps the public API identical to the original
/// `runtime_common::TransactionConverter` while satisfying the orphan rule —
/// the concrete impl below references the runtime-local `UncheckedExtrinsic`
/// and `RuntimeCall::transact`.
#[derive(Clone)]
pub struct TransactionConverter<B>(PhantomData<B>);

impl<B> Default for TransactionConverter<B> {
	fn default() -> Self {
		Self(PhantomData)
	}
}

impl<B: BlockT> fp_rpc::ConvertTransaction<<B as BlockT>::Extrinsic> for TransactionConverter<B> {
	fn convert_transaction(
		&self,
		transaction: pallet_ethereum::Transaction,
	) -> <B as BlockT>::Extrinsic {
		let extrinsic = UncheckedExtrinsic::new_bare(
			pallet_ethereum::Call::<Runtime>::transact { transaction }.into(),
		);
		let encoded = extrinsic.encode();
		<B as BlockT>::Extrinsic::decode(&mut &encoded[..])
			.expect("Encoded extrinsic is always valid")
	}
}

impl fp_self_contained::SelfContainedCall for RuntimeCall {
	type SignedInfo = H160;

	fn is_self_contained(&self) -> bool {
		match self {
			RuntimeCall::Ethereum(call) => call.is_self_contained(),
			_ => false,
		}
	}

	fn check_self_contained(&self) -> Option<Result<Self::SignedInfo, TransactionValidityError>> {
		match self {
			RuntimeCall::Ethereum(call) => call.check_self_contained(),
			_ => None,
		}
	}

	fn validate_self_contained(
		&self,
		info: &Self::SignedInfo,
		dispatch_info: &DispatchInfoOf<RuntimeCall>,
		len: usize,
	) -> Option<TransactionValidity> {
		match self {
			RuntimeCall::Ethereum(call) => call.validate_self_contained(info, dispatch_info, len),
			_ => None,
		}
	}

	fn pre_dispatch_self_contained(
		&self,
		info: &Self::SignedInfo,
		dispatch_info: &DispatchInfoOf<RuntimeCall>,
		len: usize,
	) -> Option<Result<(), TransactionValidityError>> {
		match self {
			RuntimeCall::Ethereum(call) => {
				call.pre_dispatch_self_contained(info, dispatch_info, len)
			}
			_ => None,
		}
	}

	fn apply_self_contained(
		self,
		info: Self::SignedInfo,
	) -> Option<sp_runtime::DispatchResultWithInfo<PostDispatchInfoOf<Self>>> {
		match self {
			call @ RuntimeCall::Ethereum(pallet_ethereum::Call::transact { .. }) => {
				Some(call.dispatch(RuntimeOrigin::from(
					pallet_ethereum::RawOrigin::EthereumTransaction(info),
				)))
			}
			_ => None,
		}
	}
}

#[cfg(feature = "runtime-benchmarks")]
mod benches {
	frame_benchmarking::define_benchmarks!(
		[frame_benchmarking, BaselineBench::<Runtime>]
		[frame_system, SystemBench::<Runtime>]
		[pallet_balances, Balances]
		[pallet_timestamp, Timestamp]
		[pallet_sudo, Sudo]
		[pallet_evm, EVM]
		[pallet_evm_precompile_curve25519, EVMPrecompileCurve25519Bench::<Runtime>]
		[pallet_evm_precompile_sha3fips, EVMPrecompileSha3FIPSBench::<Runtime>]
		// NPoS stack — production weights gate. Each entry tells the benchmark
		// CLI which `Pallet` type holds the `#[benchmarks]` block. Helper crates
		// (session / offences / nomination-pools) provide cross-pallet benches
		// that the upstream pallet cannot define alone because they need to
		// reach into pallet-staking.
		[pallet_babe, Babe]
		[pallet_grandpa, Grandpa]
		[pallet_session, SessionBench::<Runtime>]
		[pallet_staking, Staking]
		[pallet_offences, OffencesBench::<Runtime>]
		[pallet_bags_list, VoterList]
		[pallet_election_provider_multi_phase, ElectionProviderMultiPhase]
		[pallet_im_online, ImOnline]
		[pallet_nomination_pools, NominationPoolsBench::<Runtime>]
		[pallet_treasury, Treasury]
		[pallet_fast_unstake, FastUnstake]
		[pallet_vesting, Vesting]
		// EPM-support helper exposes phragmen / phragmms algorithm benchmarks.
		// Keeps the election engine timings separate from the EPM pallet so
		// the cost of the algorithm itself can be compared independently.
		[pallet_election_provider_support_benchmarking, ElectionProviderSupportBench::<Runtime>]
	);
}

impl_runtime_apis! {
	impl sp_api::Core<Block> for Runtime {
		fn version() -> RuntimeVersion {
			VERSION
		}

		fn execute_block(block: <Block as BlockT>::LazyBlock) {
			Executive::execute_block(block)
		}

		fn initialize_block(header: &<Block as BlockT>::Header) -> ExtrinsicInclusionMode {
			Executive::initialize_block(header)
		}
	}

	impl sp_api::Metadata<Block> for Runtime {
		fn metadata() -> OpaqueMetadata {
			OpaqueMetadata::new(Runtime::metadata().into())
		}

		fn metadata_at_version(version: u32) -> Option<OpaqueMetadata> {
			Runtime::metadata_at_version(version)
		}

		fn metadata_versions() -> Vec<u32> {
			Runtime::metadata_versions()
		}
	}

	impl sp_block_builder::BlockBuilder<Block> for Runtime {
		fn apply_extrinsic(extrinsic: <Block as BlockT>::Extrinsic) -> ApplyExtrinsicResult {
			Executive::apply_extrinsic(extrinsic)
		}

		fn finalize_block() -> <Block as BlockT>::Header {
			Executive::finalize_block()
		}

		fn inherent_extrinsics(data: sp_inherents::InherentData) -> Vec<<Block as BlockT>::Extrinsic> {
			data.create_extrinsics()
		}

		fn check_inherents(
			block: <Block as BlockT>::LazyBlock,
			data: sp_inherents::InherentData,
		) -> sp_inherents::CheckInherentsResult {
			data.check_extrinsics(&block)
		}
	}

	impl sp_transaction_pool::runtime_api::TaggedTransactionQueue<Block> for Runtime {
		fn validate_transaction(
			source: TransactionSource,
			tx: <Block as BlockT>::Extrinsic,
			block_hash: <Block as BlockT>::Hash,
		) -> TransactionValidity {
			Executive::validate_transaction(source, tx, block_hash)
		}
	}

	impl sp_offchain::OffchainWorkerApi<Block> for Runtime {
		fn offchain_worker(header: &<Block as BlockT>::Header) {
			Executive::offchain_worker(header)
		}
	}

	impl sp_genesis_builder::GenesisBuilder<Block> for Runtime {
		fn build_state(config: Vec<u8>) -> sp_genesis_builder::Result {
			build_state::<RuntimeGenesisConfig>(config)
		}

		fn get_preset(id: &Option<PresetId>) -> Option<Vec<u8>> {
			frame_support::genesis_builder_helper::get_preset::<RuntimeGenesisConfig>(id, genesis_config_preset::get_preset)
		}

		fn preset_names() -> Vec<sp_genesis_builder::PresetId> {
			vec![PresetId::from(sp_genesis_builder::DEV_RUNTIME_PRESET)]
		}
	}

	impl sp_session::SessionKeys<Block> for Runtime {
		fn generate_session_keys(owner: Vec<u8>, seed: Option<Vec<u8>>) -> sp_session::OpaqueGeneratedSessionKeys {
			SessionKeys::generate(&owner, seed).into()
		}

		fn decode_session_keys(
			encoded: Vec<u8>,
		) -> Option<Vec<(Vec<u8>, KeyTypeId)>> {
			SessionKeys::decode_into_raw_public_keys(&encoded)
		}
	}

	impl sp_consensus_grandpa::GrandpaApi<Block> for Runtime {
		fn grandpa_authorities() -> GrandpaAuthorityList {
			Grandpa::grandpa_authorities()
		}

		fn current_set_id() -> sp_consensus_grandpa::SetId {
			Grandpa::current_set_id()
		}

		fn submit_report_equivocation_unsigned_extrinsic(
			equivocation_proof: sp_consensus_grandpa::EquivocationProof<
				<Block as BlockT>::Hash,
				NumberFor<Block>,
			>,
			key_owner_proof: sp_consensus_grandpa::OpaqueKeyOwnershipProof,
		) -> Option<()> {
			// Decode the opaque proof back into the concrete
			// `MembershipProof` produced by `pallet_session::historical`,
			// then forward through Grandpa's signed extension. Returns
			// `Some(())` if the unsigned extrinsic is queued successfully.
			let key_owner_proof = key_owner_proof.decode()?;

			Grandpa::submit_unsigned_equivocation_report(
				equivocation_proof,
				key_owner_proof,
			)
		}

		fn generate_key_ownership_proof(
			_set_id: sp_consensus_grandpa::SetId,
			authority_id: GrandpaId,
		) -> Option<sp_consensus_grandpa::OpaqueKeyOwnershipProof> {
			use frame_support::traits::KeyOwnerProofSystem;
			use scale_codec::Encode;

			// Produce a `MembershipProof` for the supplied GRANDPA
			// authority via `pallet_session::historical`, then re-encode
			// it as the opaque proof shape Grandpa's runtime API expects.
			// Required for `pallet_grandpa::EquivocationReportSystem` to
			// accept (and slash on) double-vote reports.
			Historical::prove((sp_consensus_grandpa::KEY_TYPE, authority_id))
				.map(|p| p.encode())
				.map(sp_consensus_grandpa::OpaqueKeyOwnershipProof::new)
		}
	}

	impl sp_consensus_babe::BabeApi<Block> for Runtime {
		fn configuration() -> sp_consensus_babe::BabeConfiguration {
			let epoch_config = Babe::epoch_config().unwrap_or(BABE_GENESIS_EPOCH_CONFIG);
			sp_consensus_babe::BabeConfiguration {
				slot_duration: Babe::slot_duration(),
				epoch_length: EpochDuration::get(),
				c: epoch_config.c,
				authorities: Babe::authorities().to_vec(),
				randomness: Babe::randomness(),
				allowed_slots: epoch_config.allowed_slots,
			}
		}

		fn current_epoch_start() -> sp_consensus_babe::Slot {
			Babe::current_epoch_start()
		}

		fn current_epoch() -> sp_consensus_babe::Epoch {
			Babe::current_epoch()
		}

		fn next_epoch() -> sp_consensus_babe::Epoch {
			Babe::next_epoch()
		}

		fn generate_key_ownership_proof(
			_slot: sp_consensus_babe::Slot,
			authority_id: BabeId,
		) -> Option<sp_consensus_babe::OpaqueKeyOwnershipProof> {
			use frame_support::traits::KeyOwnerProofSystem;
			use scale_codec::Encode;

			// Same shape as the Grandpa proof above — wraps a
			// `pallet_session::historical::MembershipProof` so
			// `pallet_babe::EquivocationReportSystem` can validate
			// double-author reports against the historical session set.
			Historical::prove((sp_consensus_babe::KEY_TYPE, authority_id))
				.map(|p| p.encode())
				.map(sp_consensus_babe::OpaqueKeyOwnershipProof::new)
		}

		fn submit_report_equivocation_unsigned_extrinsic(
			equivocation_proof: sp_consensus_babe::EquivocationProof<<Block as BlockT>::Header>,
			key_owner_proof: sp_consensus_babe::OpaqueKeyOwnershipProof,
		) -> Option<()> {
			let key_owner_proof = key_owner_proof.decode()?;

			Babe::submit_unsigned_equivocation_report(
				equivocation_proof,
				key_owner_proof,
			)
		}
	}

	impl pallet_nomination_pools_runtime_api::NominationPoolsApi<Block, AccountId, Balance> for Runtime {
		fn pending_rewards(member_account: AccountId) -> Balance {
			NominationPools::api_pending_rewards(member_account).unwrap_or_default()
		}
		fn points_to_balance(pool_id: pallet_nomination_pools::PoolId, points: Balance) -> Balance {
			NominationPools::api_points_to_balance(pool_id, points)
		}
		fn balance_to_points(pool_id: pallet_nomination_pools::PoolId, new_funds: Balance) -> Balance {
			NominationPools::api_balance_to_points(pool_id, new_funds)
		}
		fn pool_pending_slash(pool_id: pallet_nomination_pools::PoolId) -> Balance {
			NominationPools::api_pool_pending_slash(pool_id)
		}
		fn member_pending_slash(member: AccountId) -> Balance {
			NominationPools::api_member_pending_slash(member)
		}
		fn pool_needs_delegate_migration(_pool_id: pallet_nomination_pools::PoolId) -> bool {
			false
		}
		fn member_needs_delegate_migration(_member: AccountId) -> bool {
			false
		}
		fn member_total_balance(member: AccountId) -> Balance {
			NominationPools::api_member_total_balance(member)
		}
		fn pool_balance(pool_id: pallet_nomination_pools::PoolId) -> Balance {
			NominationPools::api_pool_balance(pool_id)
		}
		fn pool_accounts(pool_id: pallet_nomination_pools::PoolId) -> (AccountId, AccountId) {
			NominationPools::api_pool_accounts(pool_id)
		}
	}

	impl pallet_staking_runtime_api::StakingApi<Block, Balance, AccountId> for Runtime {
		fn nominations_quota(balance: Balance) -> u32 {
			Staking::api_nominations_quota(balance)
		}
		fn eras_stakers_page_count(era: sp_staking::EraIndex, account: AccountId) -> sp_staking::Page {
			Staking::api_eras_stakers_page_count(era, account)
		}
		fn pending_rewards(era: sp_staking::EraIndex, account: AccountId) -> bool {
			Staking::api_pending_rewards(era, account)
		}
	}

	impl sp_authority_discovery::AuthorityDiscoveryApi<Block> for Runtime {
		fn authorities() -> Vec<sp_authority_discovery::AuthorityId> {
			// Read the authority-discovery key set from pallet-authority-discovery
			// (populated by the session handler from each validator's
			// `ImpetusSessionKeys::authority_discovery`). Returning
			// `Babe::authorities()` projected onto `AuthorityDiscoveryId` was
			// only correct while authority_discovery was the same sr25519 key
			// as babe; operators rotating to a distinct AD key would be
			// invisible on the DHT under the old projection.
			AuthorityDiscovery::authorities()
		}
	}

	impl frame_system_rpc_runtime_api::AccountNonceApi<Block, AccountId, Nonce> for Runtime {
		fn account_nonce(account: AccountId) -> Nonce {
			System::account_nonce(account)
		}
	}

	impl pallet_transaction_payment_rpc_runtime_api::TransactionPaymentApi<
		Block,
		Balance,
	> for Runtime {
		fn query_info(
			uxt: <Block as BlockT>::Extrinsic,
			len: u32
		) -> pallet_transaction_payment_rpc_runtime_api::RuntimeDispatchInfo<Balance> {
			TransactionPayment::query_info(uxt, len)
		}

		fn query_fee_details(
			uxt: <Block as BlockT>::Extrinsic,
			len: u32,
		) -> pallet_transaction_payment::FeeDetails<Balance> {
			TransactionPayment::query_fee_details(uxt, len)
		}

		fn query_weight_to_fee(weight: Weight) -> Balance {
			TransactionPayment::weight_to_fee(weight)
		}

		fn query_length_to_fee(length: u32) -> Balance {
			TransactionPayment::length_to_fee(length)
		}
	}

	impl fp_rpc::EthereumRuntimeRPCApi<Block> for Runtime {
		fn chain_id() -> u64 {
			<Runtime as pallet_evm::Config>::ChainId::get()
		}

		fn account_basic(address: H160) -> EVMAccount {
			let (account, _) = pallet_evm::Pallet::<Runtime>::account_basic(&address);
			account
		}

		fn gas_price() -> U256 {
			let (gas_price, _) = <Runtime as pallet_evm::Config>::FeeCalculator::min_gas_price();
			gas_price
		}

		fn account_code_at(address: H160) -> Vec<u8> {
			pallet_evm::AccountCodes::<Runtime>::get(address)
		}

		fn author() -> H160 {
			<pallet_evm::Pallet<Runtime>>::find_author()
		}

		fn storage_at(address: H160, index: U256) -> H256 {
			pallet_evm::AccountStorages::<Runtime>::get(address, H256::from(index.to_big_endian()))
		}

		fn call(
			from: H160,
			to: H160,
			data: Vec<u8>,
			value: U256,
			gas_limit: U256,
			max_fee_per_gas: Option<U256>,
			max_priority_fee_per_gas: Option<U256>,
			nonce: Option<U256>,
			estimate: bool,
			access_list: Option<Vec<(H160, Vec<H256>)>>,
			authorization_list: Option<AuthorizationList>,
			state_override: fp_evm::StateOverride,
		) -> Result<pallet_evm::CallInfo, sp_runtime::DispatchError> {
			use pallet_evm::GasWeightMapping as _;

			let config = if estimate {
				let mut config = <Runtime as pallet_evm::Config>::config().clone();
				config.estimate = true;
				Some(config)
			} else {
				None
			};

			// Estimated encoded transaction size must be based on the heaviest transaction
			// type (EIP7702Transaction) to be compatible with all transaction types.
			let mut estimated_transaction_len = data.len() +
				// pallet ethereum index: 1
				// transact call index: 1
				// Transaction enum variant: 1
				// chain_id 8 bytes
				// nonce: 32
				// max_priority_fee_per_gas: 32
				// max_fee_per_gas: 32
				// gas_limit: 32
				// action: 21 (enum varianrt + call address)
				// value: 32
				// access_list: 1 (empty vec size)
				// authorization_list: 1 (empty vec size)
				// 65 bytes signature
				259;

			if access_list.is_some() {
				estimated_transaction_len += access_list.encoded_size();
			}

			if authorization_list.is_some() {
				estimated_transaction_len += authorization_list.encoded_size();
			}

			let gas_limit = if gas_limit > U256::from(u64::MAX) {
				u64::MAX
			} else {
				gas_limit.low_u64()
			};
			let without_base_extrinsic_weight = true;

			let (weight_limit, proof_size_base_cost) =
				match <Runtime as pallet_evm::Config>::GasWeightMapping::gas_to_weight(
					gas_limit,
					without_base_extrinsic_weight
				) {
					weight_limit if weight_limit.proof_size() > 0 => {
						(Some(weight_limit), Some(estimated_transaction_len as u64))
					}
					_ => (None, None),
				};

			<Runtime as pallet_evm::Config>::Runner::call(
				from,
				to,
				data,
				value,
				gas_limit.unique_saturated_into(),
				max_fee_per_gas,
				max_priority_fee_per_gas,
				nonce,
				access_list.unwrap_or_default(),
				authorization_list.unwrap_or_default(),
				false,
				true,
				weight_limit,
				proof_size_base_cost,
				state_override,
				config.as_ref().unwrap_or(<Runtime as pallet_evm::Config>::config()),
			).map_err(|err| err.error.into())
		}

		fn create(
			from: H160,
			data: Vec<u8>,
			value: U256,
			gas_limit: U256,
			max_fee_per_gas: Option<U256>,
			max_priority_fee_per_gas: Option<U256>,
			nonce: Option<U256>,
			estimate: bool,
			access_list: Option<Vec<(H160, Vec<H256>)>>,
			authorization_list: Option<AuthorizationList>,
		) -> Result<pallet_evm::CreateInfo, sp_runtime::DispatchError> {
			use pallet_evm::GasWeightMapping as _;

			let config = if estimate {
				let mut config = <Runtime as pallet_evm::Config>::config().clone();
				config.estimate = true;
				Some(config)
			} else {
				None
			};


			let mut estimated_transaction_len = data.len() +
				// from: 20
				// value: 32
				// gas_limit: 32
				// nonce: 32
				// 1 byte transaction action variant
				// chain id 8 bytes
				// 65 bytes signature
				190;

			if max_fee_per_gas.is_some() {
				estimated_transaction_len += 32;
			}
			if max_priority_fee_per_gas.is_some() {
				estimated_transaction_len += 32;
			}
			if access_list.is_some() {
				estimated_transaction_len += access_list.encoded_size();
			}
			if authorization_list.is_some() {
				estimated_transaction_len += authorization_list.encoded_size();
			}

			let gas_limit = if gas_limit > U256::from(u64::MAX) {
				u64::MAX
			} else {
				gas_limit.low_u64()
			};
			let without_base_extrinsic_weight = true;

			let (weight_limit, proof_size_base_cost) =
				match <Runtime as pallet_evm::Config>::GasWeightMapping::gas_to_weight(
					gas_limit,
					without_base_extrinsic_weight
				) {
					weight_limit if weight_limit.proof_size() > 0 => {
						(Some(weight_limit), Some(estimated_transaction_len as u64))
					}
					_ => (None, None),
				};

			<Runtime as pallet_evm::Config>::Runner::create(
				from,
				data,
				value,
				gas_limit.unique_saturated_into(),
				max_fee_per_gas,
				max_priority_fee_per_gas,
				nonce,
				access_list.unwrap_or_default(),
				authorization_list.unwrap_or_default(),
				false,
				true,
				weight_limit,
				proof_size_base_cost,
				config.as_ref().unwrap_or(<Runtime as pallet_evm::Config>::config()),
			).map_err(|err| err.error.into())
		}

		fn current_transaction_statuses() -> Option<Vec<TransactionStatus>> {
			pallet_ethereum::CurrentTransactionStatuses::<Runtime>::get()
		}

		fn current_block() -> Option<pallet_ethereum::Block> {
			pallet_ethereum::CurrentBlock::<Runtime>::get()
		}

		fn current_receipts() -> Option<Vec<pallet_ethereum::Receipt>> {
			pallet_ethereum::CurrentReceipts::<Runtime>::get()
		}

		fn current_all() -> (
			Option<pallet_ethereum::Block>,
			Option<Vec<pallet_ethereum::Receipt>>,
			Option<Vec<TransactionStatus>>
		) {
			(
				pallet_ethereum::CurrentBlock::<Runtime>::get(),
				pallet_ethereum::CurrentReceipts::<Runtime>::get(),
				pallet_ethereum::CurrentTransactionStatuses::<Runtime>::get()
			)
		}

		fn extrinsic_filter(
			xts: Vec<<Block as BlockT>::Extrinsic>,
		) -> Vec<EthereumTransaction> {
			xts.into_iter().filter_map(|xt| match xt.0.function {
				RuntimeCall::Ethereum(transact { transaction }) => Some(transaction),
				_ => None
			}).collect::<Vec<EthereumTransaction>>()
		}

		fn elasticity() -> Option<Permill> {
			Some(pallet_base_fee::Elasticity::<Runtime>::get())
		}

		fn gas_limit_multiplier_support() {}

		fn pending_block(
			xts: Vec<<Block as BlockT>::Extrinsic>,
		) -> (Option<pallet_ethereum::Block>, Option<Vec<TransactionStatus>>) {
			for ext in xts.into_iter() {
				let _ = Executive::apply_extrinsic(ext);
			}

			Ethereum::on_finalize(System::block_number() + 1);

			(
				pallet_ethereum::CurrentBlock::<Runtime>::get(),
				pallet_ethereum::CurrentTransactionStatuses::<Runtime>::get()
			)
		}

		fn initialize_pending_block(header: &<Block as BlockT>::Header) {
			Executive::initialize_block(header);
		}
	}

	impl fp_rpc::ConvertTransactionRuntimeApi<Block> for Runtime {
		fn convert_transaction(transaction: EthereumTransaction) -> <Block as BlockT>::Extrinsic {
			UncheckedExtrinsic::new_bare(
				pallet_ethereum::Call::<Runtime>::transact { transaction }.into(),
			)
		}
	}

	#[cfg(feature = "runtime-benchmarks")]
	impl frame_benchmarking::Benchmark<Block> for Runtime {
		fn benchmark_metadata(extra: bool) -> (
			Vec<frame_benchmarking::BenchmarkList>,
			Vec<frame_support::traits::StorageInfo>,
		) {
			use frame_benchmarking::{baseline, BenchmarkList};
			use frame_support::traits::StorageInfoTrait;

			use baseline::Pallet as BaselineBench;
			use frame_system_benchmarking::Pallet as SystemBench;

			use pallet_evm_precompile_curve25519_benchmarking::Pallet as EVMPrecompileCurve25519Bench;
			use pallet_evm_precompile_sha3fips_benchmarking::Pallet as EVMPrecompileSha3FIPSBench;

			// NPoS bench-helper pallets. These pull benches that span pallets
			// (e.g. `Session::set_keys` must drive `Staking`), so they live in
			// dedicated crates rather than inside their upstream pallet.
			use pallet_session_benchmarking::Pallet as SessionBench;
			use pallet_offences_benchmarking::Pallet as OffencesBench;
			use pallet_nomination_pools_benchmarking::Pallet as NominationPoolsBench;
			use pallet_election_provider_support_benchmarking::Pallet as ElectionProviderSupportBench;

			let mut list = Vec::<BenchmarkList>::new();
			list_benchmarks!(list, extra);

			let storage_info = AllPalletsWithSystem::storage_info();
			(list, storage_info)
		}

		#[allow(non_local_definitions)]
		fn dispatch_benchmark(
			config: frame_benchmarking::BenchmarkConfig
		) -> Result<Vec<frame_benchmarking::BenchmarkBatch>, alloc::string::String> {
			use frame_benchmarking::{baseline, BenchmarkBatch};
			use frame_support::traits::TrackedStorageKey;

			use baseline::Pallet as BaselineBench;
			use frame_system_benchmarking::Pallet as SystemBench;
			use pallet_evm_precompile_curve25519_benchmarking::Pallet as EVMPrecompileCurve25519Bench;
			use pallet_evm_precompile_sha3fips_benchmarking::Pallet as EVMPrecompileSha3FIPSBench;
			use pallet_session_benchmarking::Pallet as SessionBench;
			use pallet_offences_benchmarking::Pallet as OffencesBench;
			use pallet_nomination_pools_benchmarking::Pallet as NominationPoolsBench;
			use pallet_election_provider_support_benchmarking::Pallet as ElectionProviderSupportBench;

			impl baseline::Config for Runtime {}
			impl frame_system_benchmarking::Config for Runtime {}
			impl pallet_evm_precompile_curve25519_benchmarking::Config for Runtime {}
			impl pallet_evm_precompile_sha3fips_benchmarking::Config for Runtime {}
			impl pallet_session_benchmarking::Config for Runtime {
				fn generate_session_keys_and_proof(
					owner: <Runtime as frame_system::Config>::AccountId,
				) -> (<Runtime as pallet_session::Config>::Keys, alloc::vec::Vec<u8>) {
					use scale_codec::Encode;
					let generated = SessionKeys::generate(&owner.encode(), None);
					(generated.keys, generated.proof.encode())
				}
			}
			impl pallet_offences_benchmarking::Config for Runtime {}
			impl pallet_nomination_pools_benchmarking::Config for Runtime {}
			impl pallet_election_provider_support_benchmarking::Config for Runtime {}

			let whitelist: Vec<TrackedStorageKey> = Vec::new();

			let mut batches = Vec::<BenchmarkBatch>::new();
			let params = (&config, &whitelist);
			add_benchmarks!(params, batches);
			Ok(batches)
		}
	}
}

#[cfg(test)]
mod tests {
	use super::{Runtime, WeightPerGas};
	#[test]
	fn configured_base_extrinsic_weight_is_evm_compatible() {
		let min_ethereum_transaction_weight = WeightPerGas::get() * 21_000;
		let base_extrinsic = <Runtime as frame_system::Config>::BlockWeights::get()
			.get(frame_support::dispatch::DispatchClass::Normal)
			.base_extrinsic;
		assert!(base_extrinsic.ref_time() <= min_ethereum_transaction_weight.ref_time());
	}
}
