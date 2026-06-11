#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

use frame_support::{
	parameter_types,
	weights::{constants::WEIGHT_REF_TIME_PER_MILLIS, Weight},
};
use sp_runtime::{
	generic,
	traits::{BlakeTwo256, IdentifyAccount, Verify},
	Perbill,
};

use fp_account::EthereumSignature;

/// Block number type.
pub type BlockNumber = u32;

/// 512-bit signature used on chain.
pub type Signature = EthereumSignature;

/// Account identifier (H160-derived).
pub type AccountId = <<Signature as Verify>::Signer as IdentifyAccount>::AccountId;

pub type AccountIndex = u32;
pub type Balance = u128;
pub type Nonce = u32;
pub type Hash = sp_core::H256;
pub type Hashing = BlakeTwo256;
pub type DigestItem = generic::DigestItem;
pub type Address = AccountId;
pub type Header = generic::Header<BlockNumber, BlakeTwo256>;

pub mod opaque {
	use super::{generic, BlakeTwo256, BlockNumber};

	pub use sp_runtime::OpaqueExtrinsic as UncheckedExtrinsic;

	pub type Header = generic::Header<BlockNumber, BlakeTwo256>;
	pub type Block = generic::Block<Header, UncheckedExtrinsic>;
	pub type BlockId = generic::BlockId<Block>;
}

pub const MILLISECS_PER_BLOCK: u64 = 6000;
pub const SLOT_DURATION: u64 = MILLISECS_PER_BLOCK;
pub const MINUTES: BlockNumber = 60_000 / (MILLISECS_PER_BLOCK as BlockNumber);
pub const HOURS: BlockNumber = MINUTES * 60;
pub const DAYS: BlockNumber = HOURS * 24;

pub const NORMAL_DISPATCH_RATIO: Perbill = Perbill::from_percent(75);
pub const WEIGHT_MILLISECS_PER_BLOCK: u64 = 2000;
pub const MAXIMUM_BLOCK_WEIGHT: Weight = Weight::from_parts(
	WEIGHT_MILLISECS_PER_BLOCK * WEIGHT_REF_TIME_PER_MILLIS,
	u64::MAX,
);
pub const MAXIMUM_BLOCK_LENGTH: u32 = 5 * 1024 * 1024;

parameter_types! {
	pub const BlockHashCount: BlockNumber = 256;
	pub BlockWeights: frame_system::limits::BlockWeights = frame_system::limits::BlockWeights
		::with_sensible_defaults(MAXIMUM_BLOCK_WEIGHT, NORMAL_DISPATCH_RATIO);
	pub BlockLength: frame_system::limits::BlockLength = frame_system::limits::BlockLength
		::builder()
		.max_length(MAXIMUM_BLOCK_LENGTH)
		.modify_max_length_for_class(
			frame_support::dispatch::DispatchClass::Normal,
			|v| *v = NORMAL_DISPATCH_RATIO * MAXIMUM_BLOCK_LENGTH,
		)
		.build();
}

use core::marker::PhantomData;
use frame_support::traits::{FindAuthor, OnTimestampSet};
use sp_core::H160;
use sp_runtime::{ConsensusEngineId, Permill};

parameter_types! {
	pub storage EnableManualSeal: bool = false;
}

/// Wraps `pallet_aura::Pallet` so that nodes started with manual seal stop
/// driving slot timestamps off the system clock.
pub struct ConsensusOnTimestampSet<T>(PhantomData<T>);

impl<T: pallet_aura::Config> OnTimestampSet<T::Moment> for ConsensusOnTimestampSet<T> {
	fn on_timestamp_set(moment: T::Moment) {
		if EnableManualSeal::get() {
			return;
		}
		<pallet_aura::Pallet<T> as OnTimestampSet<T::Moment>>::on_timestamp_set(moment)
	}
}

/// Project the active Aura authority key into a 20-byte EVM address.
///
/// The bound on `R::AuthorityId` requires `ByteArray` (a fixed-length key
/// that can be sliced into bytes) but does not pin the concrete key type,
/// so this works with any Aura/session key regardless of the underlying
/// sr25519 or ed25519 variant.
pub struct FindAuthorTruncated<F, R>(PhantomData<(F, R)>);

impl<F, R> FindAuthor<H160> for FindAuthorTruncated<F, R>
where
	F: FindAuthor<u32>,
	R: pallet_aura::Config,
	R::AuthorityId: sp_core::crypto::ByteArray,
{
	fn find_author<'a, I>(digests: I) -> Option<H160>
	where
		I: 'a + IntoIterator<Item = (ConsensusEngineId, &'a [u8])>,
	{
		use sp_core::crypto::ByteArray;
		if let Some(author_index) = F::find_author(digests) {
			let authority_id =
				pallet_aura::Authorities::<R>::get()[author_index as usize].clone();
			return Some(H160::from_slice(&authority_id.to_raw_vec()[4..24]));
		}
		None
	}
}

/// Derive the EVM `coinbase` address from the current block author as tracked
/// by `pallet_authorship`.
///
/// Returns `None` (and therefore `block.coinbase == H160::zero()`) whenever
/// `pallet_authorship` has no author yet — which is the expected behaviour
/// during Plan 1 smoke tests while `pallet_authorship::Config::FindAuthor`
/// is still the `()` placeholder.
pub struct FindAuthorFromAuthorship<T>(PhantomData<T>);

impl<T> FindAuthor<H160> for FindAuthorFromAuthorship<T>
where
	T: pallet_authorship::Config,
	T::AccountId: Into<H160>,
{
	fn find_author<'a, I>(_digests: I) -> Option<H160>
	where
		I: 'a + IntoIterator<Item = (ConsensusEngineId, &'a [u8])>,
	{
		pallet_authorship::Pallet::<T>::author().map(Into::into)
	}
}

pub struct BaseFeeThreshold;

impl pallet_base_fee::BaseFeeThreshold for BaseFeeThreshold {
	fn lower() -> Permill {
		Permill::zero()
	}
	fn ideal() -> Permill {
		Permill::from_parts(500_000)
	}
	fn upper() -> Permill {
		Permill::from_parts(1_000_000)
	}
}

/// Wraps an Ethereum extrinsic into the runtime's `UncheckedExtrinsic`.
/// Each runtime supplies its own `UncheckedExtrinsic`; the converter is
/// generic over `B: BlockT`.
#[derive(Clone)]
pub struct TransactionConverter<B>(PhantomData<B>);

impl<B> Default for TransactionConverter<B> {
	fn default() -> Self {
		Self(PhantomData)
	}
}

pub const EXISTENTIAL_DEPOSIT: u128 = 0;
pub const BLOCK_GAS_LIMIT: u64 = 75_000_000;
pub const MAX_POV_SIZE: u64 = 5 * 1024 * 1024;
pub const MAX_STORAGE_GROWTH: u64 = 400 * 1024;

#[frame_support::pallet]
pub mod pallet_manual_seal {
	use super::*;
	use frame_support::pallet_prelude::*;

	#[pallet::pallet]
	pub struct Pallet<T>(PhantomData<T>);

	#[pallet::config]
	pub trait Config: frame_system::Config {}

	#[pallet::genesis_config]
	#[derive(frame_support::DefaultNoBound)]
	pub struct GenesisConfig<T> {
		pub enable: bool,
		#[serde(skip)]
		pub _config: PhantomData<T>,
	}

	#[pallet::genesis_build]
	impl<T: Config> BuildGenesisConfig for GenesisConfig<T> {
		fn build(&self) {
			EnableManualSeal::set(&self.enable);
		}
	}
}

pub mod precompiles;
pub mod weights;

pub use precompiles::{FrontierPrecompilesBasic, FrontierPrecompilesNpos};

pub mod gasless;
pub use gasless::{GaslessEvmContext, GaslessEvmFee, GaslessEvmRunner, GaslessLiquidityInfo};

pub mod genesis_helpers;
pub use genesis_helpers::{admin_account, endowed_accounts, mnemonic_accounts};

pub mod staking_constants;
pub use staking_constants::{
    BLOCKS_PER_SESSION, BONDING_DURATION_ERAS, MAX_NOMINATIONS, MAX_NOMINATORS_PER_VALIDATOR,
    MAX_VALIDATOR_COUNT, MIN_NOMINATOR_BOND, MIN_VALIDATOR_BOND, REPORT_LONGEVITY,
    SESSIONS_PER_ERA, SESSION_OFFSET, SESSION_PERIOD, SLASH_DEFER_DURATION_ERAS, UNIT,
    VALIDATOR_COUNT_TARGET,
};

pub mod reward_curve;
pub use reward_curve::RewardCurve;

pub mod voter_bags;

pub mod staking_election;
pub use staking_election::{
    ElectionBoundsMultiPhase, ElectionBoundsOnChain, MaxActiveValidators,
    MaxOnChainElectableTargets, MaxOnChainElectingVoters, MaxNominations, OnChainSeqPhragmen,
};
