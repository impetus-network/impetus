# Multi-chain runtime support implementation plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace single `frontier-template-runtime` with two production runtimes (`impetus-runtime` for mainnet 388266, `impulse-runtime` for testnet 322644 + dev) sharing a `runtime-common` crate, and rewire `apps/node/node/` to dispatch on chain spec id.

**Architecture:** Polkadot/Kusama pattern — one node binary embeds two WASM blobs; chain spec id (`impetus | impulse | dev | <path.json>`) drives runtime dispatch in `service::build_full` and `command::*`. Common crate holds type aliases, helper structs, custom `pallet_manual_seal`, generic-ized `gasless` module, generic precompile set, weights, and genesis helpers. Each runtime crate re-exports common and supplies its own `RuntimeVersion`, `SS58Prefix`, pallet-config impls, `runtime!{}` macro invocation, and `impl_runtime_apis!{}` block.

**Tech Stack:** Rust 2021, Substrate stable2512 (FRAME, sp-version, sc-service, sc-cli), Frontier (pallet-evm, pallet-ethereum, fp-evm), Polkadot SDK (`polkadot-runtime-common`, `cumulus-pallet-weight-reclaim`), `substrate-wasm-builder`, `serde_json`, Hardhat E2E suite via `cast`/`pnpm`.

---

## File structure

**New files:**

- `apps/node/runtimes/common/Cargo.toml` — workspace dep manifest
- `apps/node/runtimes/common/src/lib.rs` — type aliases, constants, helpers, custom pallets, genesis helpers, re-exports
- `apps/node/runtimes/common/src/precompiles.rs` — moved verbatim from old runtime
- `apps/node/runtimes/common/src/gasless.rs` — moved + generic-ized
- `apps/node/runtimes/common/src/genesis_helpers.rs` — `admin_account()`, `mnemonic_accounts()`, `endowed_accounts()`
- `apps/node/runtimes/common/src/weights/mod.rs` — moved verbatim
- `apps/node/runtimes/common/src/weights/pallet_evm_precompile_curve25519.rs` — moved verbatim
- `apps/node/runtimes/common/src/weights/pallet_evm_precompile_sha3fips.rs` — moved verbatim
- `apps/node/runtimes/impetus/Cargo.toml`
- `apps/node/runtimes/impetus/build.rs`
- `apps/node/runtimes/impetus/src/lib.rs` — `spec_name="impetus"`, `SS58Prefix=11434`, pallet impls, `runtime!{}`, `impl_runtime_apis!{}`
- `apps/node/runtimes/impetus/src/genesis_config_preset.rs` — `IMPETUS_CHAIN_ID=388266`
- `apps/node/runtimes/impulse/Cargo.toml`
- `apps/node/runtimes/impulse/build.rs`
- `apps/node/runtimes/impulse/src/lib.rs` — `spec_name="impulse"`, `SS58Prefix=11348`, pallet impls, `runtime!{}`, `impl_runtime_apis!{}`
- `apps/node/runtimes/impulse/src/genesis_config_preset.rs` — `IMPULSE_CHAIN_ID=322644`
- `apps/node/chain-specs/impetus.json` — generated raw spec, committed

**Modified files:**

- `apps/node/Cargo.toml` — workspace members + workspace deps
- `apps/node/node/Cargo.toml` — drop `frontier-template-runtime`, add three new runtime crates
- `apps/node/node/src/chain_spec.rs` — full rewrite around `ChainProfile`
- `apps/node/node/src/command.rs` — `load_spec` map + dispatch in subcommands
- `apps/node/node/src/service.rs` — `build_full` + `new_chain_ops` dispatch on `Network`
- `apps/node/node/src/benchmarking.rs` — duplicate builders per runtime
- `apps/node/Dockerfile` — `--chain` argument default
- `apps/node/README.md` — chain instructions
- `AGENTS.md` (repo root) — chain table + `Sudo / admin account` block
- `apps/node/AGENTS.md` (project) — same content as repo root if that's the symlink/include

**Deleted files:**

- `apps/node/runtime/` (entire directory after Phase C)

---

## Task 1: Add `runtime-common` workspace member with empty crate skeleton

**Files:**
- Create: `apps/node/runtimes/common/Cargo.toml`
- Create: `apps/node/runtimes/common/src/lib.rs`
- Modify: `apps/node/Cargo.toml:2-7` (workspace `members` array)

- [ ] **Step 1: Create the crate skeleton**

Create `apps/node/runtimes/common/Cargo.toml`:

```toml
[package]
name = "runtime-common"
version = "0.0.0"
license = "Unlicense"
description = "Shared types and helpers used by every Impetus/Impulse runtime."
publish = false
authors = { workspace = true }
edition = { workspace = true }
repository = { workspace = true }

[dependencies]

[features]
default = ["std"]
std = []
runtime-benchmarks = []
with-rocksdb-weights = []
with-paritydb-weights = []
```

Create `apps/node/runtimes/common/src/lib.rs`:

```rust
#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;
```

- [ ] **Step 2: Add to workspace members**

Edit `apps/node/Cargo.toml:2-7` to:

```toml
[workspace]
members = [
	"node",
	"runtime",
	"runtimes/common",
	"pallets/gasless-registry",
	"precompiles/gasless-registry",
]
resolver = "2"
```

(`runtime` stays for now — we will not delete it until Phase C.)

- [ ] **Step 3: Verify workspace builds**

Run: `cd apps/node && cargo check --workspace`

Expected: success with `Compiling runtime-common v0.0.0` line and no errors.

- [ ] **Step 4: Commit**

```bash
git add apps/node/Cargo.toml apps/node/runtimes/common/
git commit -m "feat(node): scaffold runtime-common crate"
```

---

## Task 2: Move type aliases, time/weight constants, opaque module to `runtime-common`

**Files:**
- Modify: `apps/node/runtimes/common/Cargo.toml`
- Modify: `apps/node/runtimes/common/src/lib.rs`

This task ports stable type plumbing from `apps/node/runtime/src/lib.rs:73-156, 162-180, 203-220` into common. The old runtime crate stays unchanged.

- [ ] **Step 1: Add minimum dependencies needed for type aliases**

Edit `apps/node/runtimes/common/Cargo.toml` `[dependencies]`:

```toml
[dependencies]
scale-codec = { workspace = true }
scale-info = { workspace = true }
sp-core = { workspace = true }
sp-runtime = { workspace = true }
sp-version = { workspace = true }
frame-support = { workspace = true, features = ["experimental"] }
frame-system = { workspace = true }
pallet-balances = { workspace = true }
pallet-transaction-payment = { workspace = true }
pallet-aura = { workspace = true }
pallet-grandpa = { workspace = true }
fp-account = { workspace = true, features = ["serde"] }
fp-self-contained = { workspace = true, features = ["serde"] }
sp-consensus-aura = { workspace = true }
sp-consensus-grandpa = { workspace = true }
cumulus-pallet-weight-reclaim = { workspace = true }
```

Update `[features]` `std`:

```toml
std = [
	"scale-codec/std",
	"scale-info/std",
	"sp-core/std",
	"sp-runtime/std",
	"sp-version/std",
	"frame-support/std",
	"frame-system/std",
	"pallet-balances/std",
	"pallet-transaction-payment/std",
	"pallet-aura/std",
	"pallet-grandpa/std",
	"fp-account/std",
	"fp-self-contained/std",
	"sp-consensus-aura/std",
	"sp-consensus-grandpa/std",
	"cumulus-pallet-weight-reclaim/std",
]
```

- [ ] **Step 2: Port type aliases and constants into `lib.rs`**

Replace `apps/node/runtimes/common/src/lib.rs` body. Note that **`Block`, `UncheckedExtrinsic`, and `SignedExtra` are NOT defined here** — each downstream runtime crate owns those because they depend on the concrete `RuntimeCall`. Common only exports the pieces that do not need `RuntimeCall`:

```rust
#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

use frame_support::{
	parameter_types,
	weights::{constants::WEIGHT_REF_TIME_PER_MILLIS, Weight},
};
use sp_runtime::{
	generic, impl_opaque_keys,
	traits::{BlakeTwo256, IdentifyAccount, Verify},
	Perbill,
};

use fp_account::EthereumSignature;
use sp_consensus_aura::sr25519::AuthorityId as AuraId;
use sp_consensus_grandpa::AuthorityId as GrandpaId;

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
	use alloc::vec::Vec;
	use super::{generic, impl_opaque_keys, BlakeTwo256, BlockNumber};

	pub use sp_runtime::OpaqueExtrinsic as UncheckedExtrinsic;

	pub type Header = generic::Header<BlockNumber, BlakeTwo256>;
	pub type Block = generic::Block<Header, UncheckedExtrinsic>;
	pub type BlockId = generic::BlockId<Block>;

	impl_opaque_keys! {
		pub struct SessionKeys {
			pub aura: super::AuraId,
			pub grandpa: super::GrandpaId,
		}
	}
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
		::max_with_normal_ratio(MAXIMUM_BLOCK_LENGTH, NORMAL_DISPATCH_RATIO);
}
```

- [ ] **Step 3: Verify common crate builds**

Run: `cd apps/node && cargo check -p runtime-common`

Expected: success.

- [ ] **Step 4: Commit**

```bash
git add apps/node/runtimes/common/
git commit -m "feat(node): seed runtime-common with shared type aliases"
```

---

## Task 3: Add helper structs and `EnableManualSeal` storage to `runtime-common`

**Files:**
- Modify: `apps/node/runtimes/common/Cargo.toml`
- Modify: `apps/node/runtimes/common/src/lib.rs`

Move four helpers verbatim with adjusted imports — ranges in old runtime listed inline.

- [ ] **Step 1: Add deps used by helpers**

Edit `apps/node/runtimes/common/Cargo.toml` `[dependencies]` to include (already has aura/grandpa from Task 2):

```toml
pallet-base-fee = { workspace = true }
pallet-evm = { workspace = true }
pallet-ethereum = { workspace = true }
fp-rpc = { workspace = true }
fp-evm = { workspace = true, features = ["serde"] }
sp-api = { workspace = true }
```

Add same crates to `std` feature list.

- [ ] **Step 2: Append helpers to `lib.rs`**

Append to `apps/node/runtimes/common/src/lib.rs`:

```rust
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
pub struct FindAuthorTruncated<F, R>(PhantomData<(F, R)>);

impl<F, R> FindAuthor<H160> for FindAuthorTruncated<F, R>
where
	F: FindAuthor<u32>,
	R: pallet_aura::Config<AuthorityId = sp_consensus_aura::sr25519::AuthorityId>,
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
```

> The `TransactionConverter` impl of `fp_rpc::ConvertTransaction` requires
> the runtime's `UncheckedExtrinsic` and `RuntimeCall`, so it stays generic
> here and each runtime crate adds the trait impl in its own `lib.rs`.

- [ ] **Step 3: Verify**

Run: `cd apps/node && cargo check -p runtime-common`

Expected: success.

- [ ] **Step 4: Commit**

```bash
git add apps/node/runtimes/common/
git commit -m "feat(node): port helper structs to runtime-common"
```

---

## Task 4: Move `precompiles.rs` and weights to `runtime-common`

**Files:**
- Modify: `apps/node/runtimes/common/Cargo.toml`
- Create: `apps/node/runtimes/common/src/precompiles.rs`
- Create: `apps/node/runtimes/common/src/weights/mod.rs`
- Create: `apps/node/runtimes/common/src/weights/pallet_evm_precompile_curve25519.rs`
- Create: `apps/node/runtimes/common/src/weights/pallet_evm_precompile_sha3fips.rs`
- Modify: `apps/node/runtimes/common/src/lib.rs` (add `pub mod` declarations)

`precompiles.rs` is already generic over `R`, so we copy verbatim.

- [ ] **Step 1: Add precompile + sudo deps**

Edit `apps/node/runtimes/common/Cargo.toml` `[dependencies]`:

```toml
pallet-sudo = { workspace = true }
pallet-evm-precompile-modexp = { workspace = true }
pallet-evm-precompile-sha3fips = { workspace = true }
pallet-evm-precompile-simple = { workspace = true }
pallet-evm-precompile-curve25519 = { workspace = true }
pallet-evm-precompile-curve25519-benchmarking = { workspace = true }
pallet-evm-precompile-sha3fips-benchmarking = { workspace = true }
pallet-gasless-registry = { workspace = true }
precompile-gasless-registry = { workspace = true }
```

Add the same to `std` and `runtime-benchmarks` feature lists (mirror old `runtime/Cargo.toml:149-164`).

- [ ] **Step 2: Copy weights tree**

```bash
cp apps/node/runtime/src/weights/mod.rs apps/node/runtimes/common/src/weights/mod.rs
cp apps/node/runtime/src/weights/pallet_evm_precompile_curve25519.rs apps/node/runtimes/common/src/weights/pallet_evm_precompile_curve25519.rs
cp apps/node/runtime/src/weights/pallet_evm_precompile_sha3fips.rs apps/node/runtimes/common/src/weights/pallet_evm_precompile_sha3fips.rs
```

- [ ] **Step 3: Copy precompiles file verbatim**

```bash
cp apps/node/runtime/src/precompiles.rs apps/node/runtimes/common/src/precompiles.rs
```

- [ ] **Step 4: Wire modules in `lib.rs`**

Append to `apps/node/runtimes/common/src/lib.rs`:

```rust
pub mod precompiles;
pub mod weights;

pub use precompiles::FrontierPrecompiles;
```

- [ ] **Step 5: Verify**

Run: `cd apps/node && cargo check -p runtime-common`

Expected: success. The `precompiles` module references `crate::weights::*`, and `weights/mod.rs` declares its two children.

- [ ] **Step 6: Commit**

```bash
git add apps/node/runtimes/common/
git commit -m "feat(node): port precompiles and weights to runtime-common"
```

---

## Task 5: Generic-ize `gasless.rs` and add to `runtime-common`

**Files:**
- Modify: `apps/node/runtimes/common/Cargo.toml`
- Create: `apps/node/runtimes/common/src/gasless.rs`
- Modify: `apps/node/runtimes/common/src/lib.rs`

The old `gasless.rs` hardcodes `crate::Runtime`. We must replace with a generic `R` constrained by all the pallet configs the unit `()` impl plus `pallet_gasless_registry` need.

- [ ] **Step 1: Add `environmental` and `ethereum` deps**

Edit `apps/node/runtimes/common/Cargo.toml` `[dependencies]`:

```toml
environmental = { workspace = true }
ethereum = { workspace = true }
```

Add to `std`:

```toml
"environmental/std",
"ethereum/std",
```

Add `pallet-gasless-registry/runtime-benchmarks` to `runtime-benchmarks`.

- [ ] **Step 2: Write the generic-ized `gasless.rs`**

Create `apps/node/runtimes/common/src/gasless.rs`:

```rust
use alloc::vec::Vec;
use core::marker::PhantomData;

use ethereum::AuthorizationList;
use fp_evm::{CallInfo, CreateInfo};
use frame_support::weights::Weight;
use pallet_evm::{
	runner::{Runner, RunnerError},
	Error as EvmError, EvmConfig, OnChargeEVMTransaction,
};
use sp_core::{H160, H256, U256};

#[derive(Clone)]
pub struct GaslessEvmContext {
	pub target: H160,
	pub input: Vec<u8>,
	pub value: U256,
	pub gas_limit: u64,
	pub is_transactional: bool,
}

environmental::environmental!(GASLESS_EVM_CONTEXT: GaslessEvmContext);

pub enum GaslessLiquidityInfo<I> {
	Paid(I),
	Gasless,
}

impl<I: Default> Default for GaslessLiquidityInfo<I> {
	fn default() -> Self {
		Self::Paid(I::default())
	}
}

pub struct GaslessEvmFee<R>(PhantomData<R>);

impl<R> OnChargeEVMTransaction<R> for GaslessEvmFee<R>
where
	R: pallet_evm::Config + pallet_gasless_registry::Config,
	(): OnChargeEVMTransaction<R>,
	pallet_evm::BalanceOf<R>: TryFrom<U256> + Into<U256>,
{
	type LiquidityInfo = GaslessLiquidityInfo<<() as OnChargeEVMTransaction<R>>::LiquidityInfo>;

	fn withdraw_fee(who: &H160, fee: U256) -> Result<Self::LiquidityInfo, EvmError<R>> {
		if fee.is_zero() {
			return <() as OnChargeEVMTransaction<R>>::withdraw_fee(who, fee)
				.map(GaslessLiquidityInfo::Paid);
		}

		let gasless = GASLESS_EVM_CONTEXT::with(|context| {
			context.is_transactional
				&& pallet_gasless_registry::Pallet::<R>::evaluate(
					context.target,
					&context.input,
					context.value,
					context.gas_limit,
				)
				.is_gasless()
		})
		.unwrap_or(false);

		if gasless {
			Ok(GaslessLiquidityInfo::Gasless)
		} else {
			<() as OnChargeEVMTransaction<R>>::withdraw_fee(who, fee)
				.map(GaslessLiquidityInfo::Paid)
		}
	}

	fn correct_and_deposit_fee(
		who: &H160,
		corrected_fee: U256,
		base_fee: U256,
		already_withdrawn: Self::LiquidityInfo,
	) -> Self::LiquidityInfo {
		match already_withdrawn {
			GaslessLiquidityInfo::Gasless => GaslessLiquidityInfo::Gasless,
			GaslessLiquidityInfo::Paid(paid) => {
				let tip = <() as OnChargeEVMTransaction<R>>::correct_and_deposit_fee(
					who,
					corrected_fee,
					base_fee,
					paid,
				);
				GaslessLiquidityInfo::Paid(tip)
			}
		}
	}

	fn pay_priority_fee(tip: Self::LiquidityInfo) {
		if let GaslessLiquidityInfo::Paid(tip) = tip {
			<() as OnChargeEVMTransaction<R>>::pay_priority_fee(tip);
		}
	}
}

pub struct GaslessEvmRunner<R>(PhantomData<R>);

impl<R> Runner<R> for GaslessEvmRunner<R>
where
	R: pallet_evm::Config,
	pallet_evm::BalanceOf<R>: TryFrom<U256> + Into<U256>,
{
	type Error = EvmError<R>;

	fn validate(
		source: H160,
		target: Option<H160>,
		input: Vec<u8>,
		value: U256,
		gas_limit: u64,
		max_fee_per_gas: Option<U256>,
		max_priority_fee_per_gas: Option<U256>,
		nonce: Option<U256>,
		access_list: Vec<(H160, Vec<H256>)>,
		authorization_list: Vec<(U256, H160, U256, Option<H160>)>,
		is_transactional: bool,
		weight_limit: Option<Weight>,
		proof_size_base_cost: Option<u64>,
		evm_config: &EvmConfig,
	) -> Result<(), RunnerError<Self::Error>> {
		pallet_evm::runner::stack::Runner::<R>::validate(
			source,
			target,
			input,
			value,
			gas_limit,
			max_fee_per_gas,
			max_priority_fee_per_gas,
			nonce,
			access_list,
			authorization_list,
			is_transactional,
			weight_limit,
			proof_size_base_cost,
			evm_config,
		)
	}

	fn call(
		source: H160,
		target: H160,
		input: Vec<u8>,
		value: U256,
		gas_limit: u64,
		max_fee_per_gas: Option<U256>,
		max_priority_fee_per_gas: Option<U256>,
		nonce: Option<U256>,
		access_list: Vec<(H160, Vec<H256>)>,
		authorization_list: AuthorizationList,
		is_transactional: bool,
		validate: bool,
		weight_limit: Option<Weight>,
		proof_size_base_cost: Option<u64>,
		config: &EvmConfig,
	) -> Result<CallInfo, RunnerError<Self::Error>> {
		let mut context = GaslessEvmContext {
			target,
			input: input.clone(),
			value,
			gas_limit,
			is_transactional,
		};

		GASLESS_EVM_CONTEXT::using_once(&mut context, || {
			pallet_evm::runner::stack::Runner::<R>::call(
				source,
				target,
				input,
				value,
				gas_limit,
				max_fee_per_gas,
				max_priority_fee_per_gas,
				nonce,
				access_list,
				authorization_list,
				is_transactional,
				validate,
				weight_limit,
				proof_size_base_cost,
				config,
			)
		})
	}

	fn create(
		source: H160,
		init: Vec<u8>,
		value: U256,
		gas_limit: u64,
		max_fee_per_gas: Option<U256>,
		max_priority_fee_per_gas: Option<U256>,
		nonce: Option<U256>,
		access_list: Vec<(H160, Vec<H256>)>,
		authorization_list: AuthorizationList,
		is_transactional: bool,
		validate: bool,
		weight_limit: Option<Weight>,
		proof_size_base_cost: Option<u64>,
		config: &EvmConfig,
	) -> Result<CreateInfo, RunnerError<Self::Error>> {
		pallet_evm::runner::stack::Runner::<R>::create(
			source,
			init,
			value,
			gas_limit,
			max_fee_per_gas,
			max_priority_fee_per_gas,
			nonce,
			access_list,
			authorization_list,
			is_transactional,
			validate,
			weight_limit,
			proof_size_base_cost,
			config,
		)
	}

	fn create2(
		source: H160,
		init: Vec<u8>,
		salt: H256,
		value: U256,
		gas_limit: u64,
		max_fee_per_gas: Option<U256>,
		max_priority_fee_per_gas: Option<U256>,
		nonce: Option<U256>,
		access_list: Vec<(H160, Vec<H256>)>,
		authorization_list: AuthorizationList,
		is_transactional: bool,
		validate: bool,
		weight_limit: Option<Weight>,
		proof_size_base_cost: Option<u64>,
		config: &EvmConfig,
	) -> Result<CreateInfo, RunnerError<Self::Error>> {
		pallet_evm::runner::stack::Runner::<R>::create2(
			source,
			init,
			salt,
			value,
			gas_limit,
			max_fee_per_gas,
			max_priority_fee_per_gas,
			nonce,
			access_list,
			authorization_list,
			is_transactional,
			validate,
			weight_limit,
			proof_size_base_cost,
			config,
		)
	}

	fn create_force_address(
		source: H160,
		init: Vec<u8>,
		value: U256,
		gas_limit: u64,
		max_fee_per_gas: Option<U256>,
		max_priority_fee_per_gas: Option<U256>,
		nonce: Option<U256>,
		access_list: Vec<(H160, Vec<H256>)>,
		authorization_list: AuthorizationList,
		is_transactional: bool,
		validate: bool,
		weight_limit: Option<Weight>,
		proof_size_base_cost: Option<u64>,
		config: &EvmConfig,
		contract_address: H160,
	) -> Result<CreateInfo, RunnerError<Self::Error>> {
		pallet_evm::runner::stack::Runner::<R>::create_force_address(
			source,
			init,
			value,
			gas_limit,
			max_fee_per_gas,
			max_priority_fee_per_gas,
			nonce,
			access_list,
			authorization_list,
			is_transactional,
			validate,
			weight_limit,
			proof_size_base_cost,
			config,
			contract_address,
		)
	}
}
```

- [ ] **Step 3: Wire module in `lib.rs`**

Append to `apps/node/runtimes/common/src/lib.rs`:

```rust
pub mod gasless;
pub use gasless::{GaslessEvmContext, GaslessEvmFee, GaslessEvmRunner, GaslessLiquidityInfo};
```

- [ ] **Step 4: Verify**

Run: `cd apps/node && cargo check -p runtime-common`

Expected: success. If trait-bound errors surface from `<() as OnChargeEVMTransaction<R>>`, add `pallet_balances::Config` to the `R` bound.

- [ ] **Step 5: Commit**

```bash
git add apps/node/runtimes/common/
git commit -m "feat(node): port gasless module to runtime-common with generic Runtime"
```

---

## Task 6: Add genesis helpers to `runtime-common`

**Files:**
- Create: `apps/node/runtimes/common/src/genesis_helpers.rs`
- Modify: `apps/node/runtimes/common/Cargo.toml`
- Modify: `apps/node/runtimes/common/src/lib.rs`

This is the canonical home for `admin_account()`, `mnemonic_accounts()` (5 entries per spec), and `endowed_accounts()`.

- [ ] **Step 1: Add `hex-literal`**

Edit `apps/node/runtimes/common/Cargo.toml` `[dependencies]`:

```toml
hex-literal = { workspace = true }
```

- [ ] **Step 2: Write the helpers**

Create `apps/node/runtimes/common/src/genesis_helpers.rs`:

```rust
use alloc::vec;
use alloc::vec::Vec;

use hex_literal::hex;

use crate::AccountId;

/// Sudo / admin account derived from `ADMIN_MNEMONIC` (account #0). Pinned in
/// every genesis to keep the address deterministic across builds.
pub fn admin_account() -> AccountId {
	AccountId::from(hex!("d2aE0A2139dC83Cb920e3cd7B9F640922D14b872"))
}

/// Pre-funded dev users derived from the canonical Hardhat mnemonic
/// `test test test test test test test test test test test junk`
/// (HD path `m/44'/60'/0'/0/N`). These are NOT sudo — they are seeded so
/// E2E suites and Hardhat-derived wallets have spendable balances.
pub fn mnemonic_accounts() -> Vec<AccountId> {
	vec![
		AccountId::from(hex!("f39Fd6e51aad88F6F4ce6aB8827279cffFb92266")), // #0
		AccountId::from(hex!("70997970C51812dc3A010C7d01b50e0d17dc79C8")), // #1
		AccountId::from(hex!("3C44CdDdB6a900fa2b585dd299e03d12FA4293BC")), // #2
		AccountId::from(hex!("90F79bf6EB2c4f870365E785982E1f101E93b906")), // #3
		AccountId::from(hex!("15d34AAf54267DB7D7c367839AAf71A00a2C6A65")), // #4
	]
}

/// Endowed accounts at genesis: admin first, followed by Hardhat dev users.
pub fn endowed_accounts() -> Vec<AccountId> {
	let mut accounts = vec![admin_account()];
	accounts.extend(mnemonic_accounts());
	accounts
}
```

- [ ] **Step 3: Wire module**

Append to `apps/node/runtimes/common/src/lib.rs`:

```rust
pub mod genesis_helpers;
pub use genesis_helpers::{admin_account, endowed_accounts, mnemonic_accounts};
```

- [ ] **Step 4: Verify**

Run: `cd apps/node && cargo check -p runtime-common`

Expected: success.

- [ ] **Step 5: Commit**

```bash
git add apps/node/runtimes/common/
git commit -m "feat(node): add genesis helpers to runtime-common"
```

---

## Task 7: Scaffold `impetus-runtime` crate

**Files:**
- Create: `apps/node/runtimes/impetus/Cargo.toml`
- Create: `apps/node/runtimes/impetus/build.rs`
- Create: `apps/node/runtimes/impetus/src/lib.rs`
- Modify: `apps/node/Cargo.toml:2-7` (add member)
- Modify: `apps/node/Cargo.toml:175-178` (add workspace dep)

- [ ] **Step 1: Create `Cargo.toml`**

Create `apps/node/runtimes/impetus/Cargo.toml` (mirror `apps/node/runtime/Cargo.toml`, plus `runtime-common` dep):

```toml
[package]
name = "impetus-runtime"
version = "0.0.0"
license = "Unlicense"
description = "Impetus mainnet runtime (chain id 388266)."
publish = false
authors = { workspace = true }
edition = { workspace = true }
repository = { workspace = true }

[package.metadata.docs.rs]
targets = ["x86_64-unknown-linux-gnu"]

[dependencies]
runtime-common = { path = "../common", default-features = false }
environmental = { workspace = true }
ethereum = { workspace = true }
hex-literal = { workspace = true }
scale-codec = { workspace = true }
scale-info = { workspace = true }
serde_json = { workspace = true, default-features = false, features = ["alloc"] }

# Substrate
sp-api = { workspace = true }
sp-block-builder = { workspace = true }
sp-consensus-aura = { workspace = true }
sp-consensus-grandpa = { workspace = true }
sp-core = { workspace = true }
sp-genesis-builder = { workspace = true }
sp-inherents = { workspace = true }
sp-offchain = { workspace = true }
sp-runtime = { workspace = true }
sp-session = { workspace = true }
sp-std = { workspace = true }
sp-transaction-pool = { workspace = true }
sp-version = { workspace = true }
# FRAME
frame-benchmarking = { workspace = true, optional = true }
frame-executive = { workspace = true }
frame-support = { workspace = true, features = ["experimental"] }
frame-system = { workspace = true }
frame-system-benchmarking = { workspace = true, optional = true }
frame-system-rpc-runtime-api = { workspace = true }
pallet-assets = { workspace = true }
pallet-aura = { workspace = true }
pallet-balances = { workspace = true, features = ["insecure_zero_ed"] }
pallet-grandpa = { workspace = true }
pallet-sudo = { workspace = true }
pallet-timestamp = { workspace = true }
pallet-transaction-payment = { workspace = true }
pallet-transaction-payment-rpc-runtime-api = { workspace = true }
# Local
pallet-gasless-registry = { workspace = true }
precompile-gasless-registry = { workspace = true }
# Frontier
fp-account = { workspace = true, features = ["serde"] }
fp-evm = { workspace = true, features = ["serde"] }
fp-rpc = { workspace = true }
fp-self-contained = { workspace = true, features = ["serde"] }
pallet-base-fee = { workspace = true }
pallet-dynamic-fee = { workspace = true }
pallet-ethereum = { workspace = true }
pallet-evm = { workspace = true }
pallet-evm-chain-id = { workspace = true }
pallet-evm-precompile-curve25519 = { workspace = true }
pallet-evm-precompile-curve25519-benchmarking = { workspace = true }
pallet-evm-precompile-modexp = { workspace = true }
pallet-evm-precompile-sha3fips = { workspace = true }
pallet-evm-precompile-sha3fips-benchmarking = { workspace = true }
pallet-evm-precompile-simple = { workspace = true }
# Polkadot
polkadot-runtime-common = { workspace = true }
# Cumulus
cumulus-pallet-weight-reclaim = { workspace = true }

[build-dependencies]
substrate-wasm-builder = { workspace = true, optional = true }

[features]
default = ["std", "with-paritydb-weights"]
with-rocksdb-weights = ["runtime-common/with-rocksdb-weights"]
with-paritydb-weights = ["runtime-common/with-paritydb-weights"]
std = [
	"runtime-common/std",
	"environmental/std",
	"ethereum/std",
	"scale-codec/std",
	"scale-info/std",
	"serde_json/std",
	"sp-api/std",
	"sp-block-builder/std",
	"sp-consensus-aura/std",
	"sp-consensus-grandpa/std",
	"sp-core/std",
	"sp-genesis-builder/std",
	"sp-inherents/std",
	"sp-offchain/std",
	"sp-runtime/std",
	"sp-session/std",
	"sp-std/std",
	"sp-transaction-pool/std",
	"sp-version/std",
	"substrate-wasm-builder",
	"frame-benchmarking?/std",
	"frame-executive/std",
	"frame-support/std",
	"frame-system/std",
	"frame-system-benchmarking?/std",
	"frame-system-rpc-runtime-api/std",
	"pallet-assets/std",
	"pallet-aura/std",
	"pallet-balances/std",
	"pallet-grandpa/std",
	"pallet-sudo/std",
	"pallet-timestamp/std",
	"pallet-transaction-payment/std",
	"pallet-transaction-payment-rpc-runtime-api/std",
	"pallet-gasless-registry/std",
	"precompile-gasless-registry/std",
	"fp-account/std",
	"fp-evm/std",
	"fp-rpc/std",
	"fp-self-contained/std",
	"pallet-base-fee/std",
	"pallet-dynamic-fee/std",
	"pallet-ethereum/std",
	"pallet-evm/std",
	"pallet-evm-chain-id/std",
	"pallet-evm-precompile-modexp/std",
	"pallet-evm-precompile-sha3fips/std",
	"pallet-evm-precompile-sha3fips-benchmarking/std",
	"pallet-evm-precompile-simple/std",
	"pallet-evm-precompile-curve25519/std",
	"pallet-evm-precompile-curve25519-benchmarking/std",
	"polkadot-runtime-common/std",
	"cumulus-pallet-weight-reclaim/std",
]
runtime-benchmarks = [
	"runtime-common/runtime-benchmarks",
	"frame-benchmarking/runtime-benchmarks",
	"frame-system-benchmarking/runtime-benchmarks",
	"frame-system/runtime-benchmarks",
	"pallet-assets/runtime-benchmarks",
	"pallet-balances/runtime-benchmarks",
	"pallet-grandpa/runtime-benchmarks",
	"pallet-timestamp/runtime-benchmarks",
	"pallet-sudo/runtime-benchmarks",
	"pallet-ethereum/runtime-benchmarks",
	"pallet-evm/runtime-benchmarks",
	"pallet-gasless-registry/runtime-benchmarks",
	"pallet-evm-precompile-curve25519-benchmarking/runtime-benchmarks",
	"pallet-evm-precompile-sha3fips-benchmarking/runtime-benchmarks",
	"polkadot-runtime-common/runtime-benchmarks",
]
```

- [ ] **Step 2: Create `build.rs`**

Copy `apps/node/runtime/build.rs` verbatim to `apps/node/runtimes/impetus/build.rs` (it just calls `substrate_wasm_builder`).

- [ ] **Step 3: Create empty `lib.rs`**

Create `apps/node/runtimes/impetus/src/lib.rs`:

```rust
#![cfg_attr(not(feature = "std"), no_std)]
#![recursion_limit = "256"]

extern crate alloc;

#[cfg(feature = "std")]
include!(concat!(env!("OUT_DIR"), "/wasm_binary.rs"));

pub use runtime_common::*;
```

- [ ] **Step 4: Add to workspace**

Edit `apps/node/Cargo.toml:2-7`:

```toml
members = [
	"node",
	"runtime",
	"runtimes/common",
	"runtimes/impetus",
	"pallets/gasless-registry",
	"precompiles/gasless-registry",
]
```

Add to `apps/node/Cargo.toml` workspace deps section (next to existing `frontier-template-runtime` line):

```toml
runtime-common = { path = "runtimes/common", default-features = false }
impetus-runtime = { path = "runtimes/impetus", default-features = false }
```

- [ ] **Step 5: Verify**

Run: `cd apps/node && cargo check -p impetus-runtime`

Expected: success (the crate compiles a near-empty lib that just re-exports common; WASM build is gated to release builds and may show as the final lto step).

- [ ] **Step 6: Commit**

```bash
git add apps/node/Cargo.toml apps/node/runtimes/impetus/
git commit -m "feat(node): scaffold impetus-runtime crate"
```

---

## Task 8: Populate `impetus-runtime/src/lib.rs` with full runtime body

**Files:**
- Modify: `apps/node/runtimes/impetus/src/lib.rs`
- Create: `apps/node/runtimes/impetus/src/genesis_config_preset.rs`

This task ports `apps/node/runtime/src/lib.rs:65-1146` into Impetus, applying these mechanical changes:

| Old | New |
|---|---|
| `spec_name: Cow::Borrowed("frontier-template")` (`runtime/src/lib.rs:184`) | `spec_name: Cow::Borrowed("impetus")` |
| `impl_name: Cow::Borrowed("frontier-template")` (`runtime/src/lib.rs:185`) | `impl_name: Cow::Borrowed("impetus")` |
| `pub const SS58Prefix: u8 = 42;` (`runtime/src/lib.rs:219`) | `pub const SS58Prefix: u16 = 11434;` |
| `type SS58Prefix = SS58Prefix;` in `frame_system::Config` | unchanged — `frame_system::Config::SS58Prefix` accepts `Get<u16>` in stable2512 |
| `type Runner = gasless::GaslessEvmRunner;` (`runtime/src/lib.rs:411`) | `type Runner = runtime_common::GaslessEvmRunner<Self>;` |
| `type OnChargeTransaction = gasless::GaslessEvmFee;` (`runtime/src/lib.rs:412`) | `type OnChargeTransaction = runtime_common::GaslessEvmFee<Self>;` |
| `type FindAuthor = FindAuthorTruncated<Aura>;` (`runtime/src/lib.rs:414`) | `type FindAuthor = runtime_common::FindAuthorTruncated<Aura, Self>;` |
| `type OnTimestampSet = ConsensusOnTimestampSet<Self>;` (`runtime/src/lib.rs:292`) | `type OnTimestampSet = runtime_common::ConsensusOnTimestampSet<Self>;` |
| `type Threshold = BaseFeeThreshold;` (`runtime/src/lib.rs:460`) | `type Threshold = runtime_common::BaseFeeThreshold;` |
| `type PrecompilesType = FrontierPrecompiles<Self>;` (`runtime/src/lib.rs:407`) | `type PrecompilesType = runtime_common::FrontierPrecompiles<Self>;` |
| `pub PrecompilesValue: FrontierPrecompiles<Runtime> = FrontierPrecompiles::<_>::new();` (`runtime/src/lib.rs:393`) | `pub PrecompilesValue: runtime_common::FrontierPrecompiles<Runtime> = runtime_common::FrontierPrecompiles::<_>::new();` |
| `mod gasless;` / `mod precompiles;` / `mod weights;` (top of `runtime/src/lib.rs`) | delete — re-exported via `runtime_common` |
| `pallet_manual_seal` inline mod (`runtime/src/lib.rs:466-490`) | delete — use `runtime_common::pallet_manual_seal` |
| `impl pallet_manual_seal::Config for Runtime {}` (`runtime/src/lib.rs:492`) | `impl runtime_common::pallet_manual_seal::Config for Runtime {}` |
| Index 11 in `runtime!{}` (`runtime/src/lib.rs:554`) | `pub type ManualSeal = runtime_common::pallet_manual_seal;` |
| `EnableManualSeal` storage (`runtime/src/lib.rs:276-278`) | delete — re-export from `runtime_common::EnableManualSeal` |
| Constants `EXISTENTIAL_DEPOSIT`, `BLOCK_GAS_LIMIT`, etc. | re-export from `runtime_common` |
| Type aliases `BlockNumber`, `Signature`, `AccountId`, `Balance`, `Nonce`, `Hash`, `Hashing`, `DigestItem`, `Address`, `Header`, `Block`, `SignedBlock`, `BlockId`, `SignedExtra`, `UncheckedExtrinsic`, `CheckedExtrinsic`, `SignedPayload`, `Executive`, `opaque` mod | KEEP in this crate (concrete `Block`/`SignedExtra` need `RuntimeCall`/`UncheckedExtrinsic` defined here). `BlockNumber`, `Hashing`, `AccountId`, `Balance`, `Nonce`, `Hash`, `Signature`, `MILLISECS_PER_BLOCK`, `SLOT_DURATION` may simply be `pub use runtime_common::{ ... };`. |
| Section `pub fn native_version() -> sp_version::NativeVersion` | keep |

> Two test functions live in `runtime/src/lib.rs:1135-1146`. Move them into
> `impetus-runtime` and update if needed (no spec_name reference, just weight
> arithmetic, so they port unchanged).

- [ ] **Step 1: Copy `runtime/src/lib.rs` and apply transformations**

```bash
cp apps/node/runtime/src/lib.rs apps/node/runtimes/impetus/src/lib.rs
```

Then open the file and apply every row from the table above. Pay special attention to:

1. The doc comment header — replace "Substrate Node Template runtime" with "Impetus mainnet runtime."
2. Top-level `mod gasless; mod genesis_config_preset; mod precompiles; mod weights;` block (lines 11-14) → keep only `mod genesis_config_preset;`.
3. The `use precompiles::FrontierPrecompiles;` line (`runtime/src/lib.rs:70`) → delete (use `runtime_common::FrontierPrecompiles` qualified).
4. The `pub mod pallet_manual_seal { ... }` block (`runtime/src/lib.rs:466-490`) → delete entirely.
5. The `parameter_types! { pub storage EnableManualSeal: bool = false; }` block (`runtime/src/lib.rs:276-278`) → delete.
6. The `pub const EXISTENTIAL_DEPOSIT: u128 = 0;` line (`runtime/src/lib.rs:297`) → replace with `use runtime_common::EXISTENTIAL_DEPOSIT;`.
7. The `const BLOCK_GAS_LIMIT: u64 = 75_000_000;` block (`runtime/src/lib.rs:384-388`) → replace with imports from `runtime_common`.
8. The `BLOCK_GAS_LIMIT` references in `parameter_types!` (`runtime/src/lib.rs:389-395`) → still work because `runtime_common::BLOCK_GAS_LIMIT` is in scope.
9. `ConsensusOnTimestampSet`, `FindAuthorTruncated`, `BaseFeeThreshold`, `TransactionConverter` definitions (`runtime/src/lib.rs:280-289, 369-382, 447-458, 564-585`) → delete entirely (use qualified paths from `runtime_common`).
10. The `runtime!{}` macro block (`runtime/src/lib.rs:506-562`) — change pallet 11:

```rust
#[runtime::pallet_index(11)]
pub type ManualSeal = runtime_common::pallet_manual_seal;
```

11. After the type-alias section, insert:

```rust
pub use runtime_common::{
	opaque, AccountId, AccountIndex, Address, Balance, BlockHashCount, BlockLength, BlockNumber,
	BlockWeights, DAYS, DigestItem, EXISTENTIAL_DEPOSIT, EnableManualSeal,
	HOURS, Hash, Hashing, MAXIMUM_BLOCK_LENGTH, MAXIMUM_BLOCK_WEIGHT, MILLISECS_PER_BLOCK,
	MINUTES, NORMAL_DISPATCH_RATIO, Nonce, SLOT_DURATION, Signature,
	WEIGHT_MILLISECS_PER_BLOCK,
};
```

12. Update `RuntimeVersion` (`runtime/src/lib.rs:182-192`):

```rust
#[sp_version::runtime_version]
pub const VERSION: RuntimeVersion = RuntimeVersion {
	spec_name: Cow::Borrowed("impetus"),
	impl_name: Cow::Borrowed("impetus"),
	authoring_version: 1,
	spec_version: 1,
	impl_version: 1,
	apis: RUNTIME_API_VERSIONS,
	transaction_version: 1,
	system_version: 1,
};
```

13. Update SS58 (`runtime/src/lib.rs:219`):

```rust
parameter_types! {
	pub const Version: RuntimeVersion = VERSION;
	pub const SS58Prefix: u16 = 11434;
}
```

(the existing `BlockHashCount`, `BlockWeights`, `BlockLength` parameter_types come from `runtime_common`.)

14. Update the `pallet_manual_seal::Config` impl (`runtime/src/lib.rs:492`):

```rust
impl runtime_common::pallet_manual_seal::Config for Runtime {}
```

- [ ] **Step 2: Create `genesis_config_preset.rs`**

Copy from `runtime/src/genesis_config_preset.rs` and modify:

```bash
cp apps/node/runtime/src/genesis_config_preset.rs apps/node/runtimes/impetus/src/genesis_config_preset.rs
```

Edit the new file:

- Replace `use crate::{ ... };` import to use `crate` types (which are now re-exported from `runtime_common`).
- Replace `const ARTEMIS_CHAIN_ID: u64 = 322;` (line 15) with `const IMPETUS_CHAIN_ID: u64 = 388266;`.
- Replace remaining references to `ARTEMIS_CHAIN_ID` with `IMPETUS_CHAIN_ID`.
- Delete the inline `admin_account()`, `mnemonic_accounts()`, `endowed_accounts()` functions; replace `use` with:

```rust
use runtime_common::{admin_account, endowed_accounts};
```

- The `development()` function still uses `IMPETUS_CHAIN_ID` and the imported helpers.

- [ ] **Step 3: Verify the runtime builds**

Run: `cd apps/node && cargo check -p impetus-runtime`

Expected: success. If any error references `gasless::*` or `precompiles::*`, fix the qualified path to `runtime_common::*`.

If WASM build is triggered for release, run instead `cd apps/node && cargo check -p impetus-runtime --no-default-features --features std` to keep verification fast.

- [ ] **Step 4: Commit**

```bash
git add apps/node/runtimes/impetus/
git commit -m "feat(node): port runtime body into impetus-runtime"
```

---

## Task 9: Scaffold and populate `impulse-runtime`

**Files:**
- Create: `apps/node/runtimes/impulse/Cargo.toml` (mirror Impetus)
- Create: `apps/node/runtimes/impulse/build.rs`
- Create: `apps/node/runtimes/impulse/src/lib.rs`
- Create: `apps/node/runtimes/impulse/src/genesis_config_preset.rs`
- Modify: `apps/node/Cargo.toml` (member + workspace dep)

- [ ] **Step 1: Copy from impetus and rename**

```bash
cp -R apps/node/runtimes/impetus apps/node/runtimes/impulse
```

Edit `apps/node/runtimes/impulse/Cargo.toml`:

- `name = "impulse-runtime"`
- `description = "Impulse testnet runtime (chain id 322644, used for dev mode)."`

- [ ] **Step 2: Patch `RuntimeVersion`, SS58, and chain id**

Edit `apps/node/runtimes/impulse/src/lib.rs`:

```rust
#[sp_version::runtime_version]
pub const VERSION: RuntimeVersion = RuntimeVersion {
	spec_name: Cow::Borrowed("impulse"),
	impl_name: Cow::Borrowed("impulse"),
	authoring_version: 1,
	spec_version: 1,
	impl_version: 1,
	apis: RUNTIME_API_VERSIONS,
	transaction_version: 1,
	system_version: 1,
};

parameter_types! {
	pub const Version: RuntimeVersion = VERSION;
	pub const SS58Prefix: u16 = 11348;
}
```

Edit `apps/node/runtimes/impulse/src/genesis_config_preset.rs`:

- `const IMPULSE_CHAIN_ID: u64 = 322644;`
- Replace remaining `IMPETUS_CHAIN_ID` references with `IMPULSE_CHAIN_ID`.

- [ ] **Step 3: Update doc comment**

Edit the top-of-file comment in `apps/node/runtimes/impulse/src/lib.rs` to reflect "Impulse testnet runtime."

- [ ] **Step 4: Add to workspace**

Edit `apps/node/Cargo.toml`:

```toml
members = [
	"node",
	"runtime",
	"runtimes/common",
	"runtimes/impetus",
	"runtimes/impulse",
	"pallets/gasless-registry",
	"precompiles/gasless-registry",
]
```

Add workspace dep:

```toml
impulse-runtime = { path = "runtimes/impulse", default-features = false }
```

- [ ] **Step 5: Verify**

Run: `cd apps/node && cargo check -p impulse-runtime`

Expected: success.

- [ ] **Step 6: Commit**

```bash
git add apps/node/Cargo.toml apps/node/runtimes/impulse/
git commit -m "feat(node): add impulse-runtime mirroring impetus"
```

---

## Task 10: Add new runtime crates to node `Cargo.toml`

**Files:**
- Modify: `apps/node/node/Cargo.toml:81` (currently `frontier-template-runtime = { workspace = true, features = ["std"] }`)

We add the three new deps but keep the old `frontier-template-runtime` line for now — the next tasks will switch the source code over before we drop it.

- [ ] **Step 1: Edit `node/Cargo.toml`**

Insert above the existing `frontier-template-runtime` line:

```toml
runtime-common = { workspace = true, features = ["std"] }
impetus-runtime = { workspace = true, features = ["std"] }
impulse-runtime = { workspace = true, features = ["std"] }
```

(Leave `frontier-template-runtime = { workspace = true, features = ["std"] }` in place; it will be removed in Task 16.)

- [ ] **Step 2: Verify**

Run: `cd apps/node && cargo check -p frontier-template-node`

Expected: success — node still uses old runtime, but new crates are now linked.

- [ ] **Step 3: Commit**

```bash
git add apps/node/node/Cargo.toml
git commit -m "feat(node): wire impetus/impulse/runtime-common into node deps"
```

---

## Task 11: Refactor `chain_spec.rs` around `ChainProfile`

**Files:**
- Modify: `apps/node/node/src/chain_spec.rs` (full rewrite)

We remove direct dependence on `frontier_template_runtime` and depend on impetus + impulse.

- [ ] **Step 1: Write failing test for chain id resolution**

Append to `apps/node/node/src/chain_spec.rs` (at the bottom, before any final `}`):

```rust
#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn impetus_spec_has_chain_id_388266() {
		let spec = impetus_config();
		let json: serde_json::Value = serde_json::from_str(&spec.as_json(false).unwrap()).unwrap();
		assert_eq!(json["genesis"]["runtimeGenesis"]["patch"]["evmChainId"]["chainId"], 388266);
	}

	#[test]
	fn impulse_spec_has_chain_id_322644() {
		let spec = impulse_config();
		let json: serde_json::Value = serde_json::from_str(&spec.as_json(false).unwrap()).unwrap();
		assert_eq!(json["genesis"]["runtimeGenesis"]["patch"]["evmChainId"]["chainId"], 322644);
	}

	#[test]
	fn dev_spec_enables_manual_seal() {
		let spec = development_config();
		let json: serde_json::Value = serde_json::from_str(&spec.as_json(false).unwrap()).unwrap();
		assert_eq!(json["genesis"]["runtimeGenesis"]["patch"]["manualSeal"]["enable"], true);
	}
}
```

- [ ] **Step 2: Verify tests fail**

Run: `cd apps/node && cargo test -p frontier-template-node chain_spec::tests`

Expected: FAIL — `impetus_config` and `impulse_config` do not exist yet.

- [ ] **Step 3: Replace `chain_spec.rs` body**

Replace `apps/node/node/src/chain_spec.rs` (excluding the `#[cfg(test)] mod tests` block we just added) with:

```rust
use std::collections::BTreeMap;

use sc_chain_spec::{ChainType, Properties};
use sc_service::ChainSpec as ChainSpecTrait;
use sp_core::{H160, U256};

use runtime_common::{admin_account, endowed_accounts, AccountId, Balance};

pub type ChainSpec = sc_service::GenericChainSpec;

const UNITS: Balance = 1_000_000_000_000_000_000;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Network {
	Impetus,
	Impulse,
}

impl Network {
	pub fn from_spec_id(id: &str) -> Self {
		match id {
			"impetus" | "mainnet" => Network::Impetus,
			_ => Network::Impulse,
		}
	}
}

pub struct ChainProfile {
	pub network: Network,
	pub display_name: &'static str,
	pub spec_id: &'static str,
	pub evm_chain_id: u64,
	pub token_symbol: &'static str,
	pub ss58_prefix: u32,
	pub chain_type: ChainType,
	pub manual_seal: bool,
}

fn impetus_profile() -> ChainProfile {
	ChainProfile {
		network: Network::Impetus,
		display_name: "Impetus",
		spec_id: "impetus",
		evm_chain_id: 388266,
		token_symbol: "IPT",
		ss58_prefix: 11434,
		chain_type: ChainType::Live,
		manual_seal: false,
	}
}

fn impulse_profile() -> ChainProfile {
	ChainProfile {
		network: Network::Impulse,
		display_name: "Impulse Testnet",
		spec_id: "impulse",
		evm_chain_id: 322644,
		token_symbol: "IPL",
		ss58_prefix: 11348,
		chain_type: ChainType::Live,
		manual_seal: false,
	}
}

fn dev_profile() -> ChainProfile {
	ChainProfile {
		network: Network::Impulse,
		display_name: "Impulse Dev",
		spec_id: "dev",
		evm_chain_id: 322644,
		token_symbol: "IPL",
		ss58_prefix: 11348,
		chain_type: ChainType::Development,
		manual_seal: true,
	}
}

pub fn impetus_config() -> ChainSpec {
	let profile = impetus_profile();
	let wasm = impetus_runtime::WASM_BINARY.expect("Impetus WASM not built");
	build_spec(&profile, wasm)
}

pub fn impulse_config() -> ChainSpec {
	let profile = impulse_profile();
	let wasm = impulse_runtime::WASM_BINARY.expect("Impulse WASM not built");
	build_spec(&profile, wasm)
}

pub fn development_config() -> ChainSpec {
	let profile = dev_profile();
	let wasm = impulse_runtime::WASM_BINARY.expect("Impulse WASM not built (used by dev)");
	build_spec(&profile, wasm)
}

fn build_spec(profile: &ChainProfile, wasm: &[u8]) -> ChainSpec {
	ChainSpec::builder(wasm, Default::default())
		.with_name(profile.display_name)
		.with_id(profile.spec_id)
		.with_chain_type(profile.chain_type.clone())
		.with_properties(properties(profile))
		.with_genesis_config_patch(genesis_patch(
			admin_account(),
			endowed_accounts(),
			profile.evm_chain_id,
			profile.manual_seal,
		))
		.build()
}

fn properties(profile: &ChainProfile) -> Properties {
	let mut props = Properties::new();
	props.insert("tokenDecimals".into(), 18.into());
	props.insert("tokenSymbol".into(), profile.token_symbol.into());
	props.insert("ss58Format".into(), profile.ss58_prefix.into());
	props.insert("isEthereum".into(), true.into());
	props
}

fn genesis_patch(
	sudo_key: AccountId,
	endowed: Vec<AccountId>,
	chain_id: u64,
	enable_manual_seal: bool,
) -> serde_json::Value {
	let evm_accounts: BTreeMap<H160, fp_evm::GenesisAccount> = endowed
		.iter()
		.map(|account| {
			(
				H160::from(*account),
				fp_evm::GenesisAccount {
					balance: U256::from(1_000_000u128) * U256::from(UNITS),
					code: Default::default(),
					nonce: Default::default(),
					storage: Default::default(),
				},
			)
		})
		.collect();

	serde_json::json!({
		"sudo": { "key": Some(sudo_key.clone()) },
		"balances": {
			"balances": endowed
				.iter()
				.cloned()
				.map(|k| (k, 1_000_000u128 * UNITS))
				.collect::<Vec<_>>()
		},
		"aura": { "authorities": Vec::<sp_consensus_aura::sr25519::AuthorityId>::new() },
		"grandpa": { "authorities": Vec::<(sp_consensus_grandpa::AuthorityId, u64)>::new() },
		"evmChainId": { "chainId": chain_id },
		"evm": { "accounts": evm_accounts },
		"manualSeal": { "enable": enable_manual_seal },
		"gaslessRegistry": { "rules": [] }
	})
}
```

> The original `chain_spec.rs` populated authorities with `//Alice`. Per
> spec decision, we keep that mode for now — but `ChainSpec::builder`
> serializes authorities at runtime when the chain spec is loaded, not
> here. Authority injection happens via the keystore CLI in normal node
> operation. The patch above leaves the field empty; the dev workflow
> still uses `--alice` via `sc_cli::RunCmd` to seed Aura/Grandpa keys.

- [ ] **Step 4: Verify tests pass**

Run: `cd apps/node && cargo test -p frontier-template-node chain_spec::tests`

Expected: 3 PASS.

- [ ] **Step 5: Commit**

```bash
git add apps/node/node/src/chain_spec.rs
git commit -m "feat(node): rewrite chain_spec around ChainProfile + dual runtime"
```

---

## Task 12: Update `command.rs::load_spec` for new chain ids

**Files:**
- Modify: `apps/node/node/src/command.rs:42-53` (and surrounding subcommand arms)

- [ ] **Step 1: Replace the `load_spec` implementation**

Replace the body of `load_spec` (`apps/node/node/src/command.rs:42-53`) with:

```rust
fn load_spec(&self, id: &str) -> Result<Box<dyn ChainSpec>, String> {
	Ok(match id {
		"dev" => Box::new(chain_spec::development_config()),
		"" | "impulse" | "testnet" => Box::new(chain_spec::impulse_config()),
		"impetus" | "mainnet" => Box::new(chain_spec::impetus_config()),
		path => Box::new(chain_spec::ChainSpec::from_json_file(
			std::path::PathBuf::from(path),
		)?),
	})
}
```

- [ ] **Step 2: Drop the unused `enable_manual_seal` derivation**

Earlier code mapped `--sealing` to a per-spec flag at chain-spec build time. With the new layout, the `dev` profile already encodes `manual_seal: true` in genesis. Remove this from `load_spec` if you see leftover `let enable_manual_seal = ...` in your diff.

- [ ] **Step 3: Verify build**

Run: `cd apps/node && cargo check -p frontier-template-node`

Expected: success — node still calls `service::build_full(...)` referencing `frontier-template-runtime`, that path unchanged for now.

- [ ] **Step 4: Commit**

```bash
git add apps/node/node/src/command.rs
git commit -m "feat(node): map --chain to impetus/impulse/dev"
```

---

## Task 13: Refactor `service::build_full` and `new_chain_ops` to dispatch on `Network`

**Files:**
- Modify: `apps/node/node/src/service.rs:22-24, 51-52, 759-797`

`new_partial`, `new_full`, `build_aura_grandpa_import_queue`, `build_manual_seal_import_queue` are already generic — we only need to rewrite the entry points and remove the hard-coded `frontier_template_runtime::*` import.

- [ ] **Step 1: Update top-of-file imports**

Replace `apps/node/node/src/service.rs:22-24`:

```rust
// Runtime
use runtime_common::{AccountId, Balance, Nonce};
```

(`opaque::Block`, `RuntimeApi`, `TransactionConverter` will be referenced via the crate-qualified path inside dispatch.)

- [ ] **Step 2: Remove `pub type Client`**

Delete the line at `service.rs:52`:

```rust
pub type Client = FullClient<Block, RuntimeApi, HostFunctions>;
```

(no callers remain after Task 14.)

Also delete `pub type Backend = FullBackend<Block>;` at line 51 — replaced by the generic `FullBackend<B>` callsites.

- [ ] **Step 3: Replace `build_full`**

Replace `apps/node/node/src/service.rs:759-768` with:

```rust
pub async fn build_full(
	config: Configuration,
	eth_config: EthConfiguration,
	sealing: Option<Sealing>,
) -> Result<TaskManager, ServiceError> {
	use crate::chain_spec::Network;

	match Network::from_spec_id(config.chain_spec.id()) {
		Network::Impetus => {
			new_full::<
				impetus_runtime::opaque::Block,
				impetus_runtime::RuntimeApi,
				HostFunctions,
				sc_network::NetworkWorker<_, _>,
			>(config, eth_config, sealing)
			.await
		}
		Network::Impulse => {
			new_full::<
				impulse_runtime::opaque::Block,
				impulse_runtime::RuntimeApi,
				HostFunctions,
				sc_network::NetworkWorker<_, _>,
			>(config, eth_config, sealing)
			.await
		}
	}
}
```

- [ ] **Step 4: Replace `new_chain_ops` with two specialised entry points**

Replace `apps/node/node/src/service.rs:770-797` with:

```rust
pub enum ChainOps {
	Impetus(
		Arc<FullClient<impetus_runtime::opaque::Block, impetus_runtime::RuntimeApi, HostFunctions>>,
		Arc<FullBackend<impetus_runtime::opaque::Block>>,
		BasicQueue<impetus_runtime::opaque::Block>,
		TaskManager,
		FrontierBackend<
			impetus_runtime::opaque::Block,
			FullClient<impetus_runtime::opaque::Block, impetus_runtime::RuntimeApi, HostFunctions>,
		>,
	),
	Impulse(
		Arc<FullClient<impulse_runtime::opaque::Block, impulse_runtime::RuntimeApi, HostFunctions>>,
		Arc<FullBackend<impulse_runtime::opaque::Block>>,
		BasicQueue<impulse_runtime::opaque::Block>,
		TaskManager,
		FrontierBackend<
			impulse_runtime::opaque::Block,
			FullClient<impulse_runtime::opaque::Block, impulse_runtime::RuntimeApi, HostFunctions>,
		>,
	),
}

pub fn new_chain_ops(
	config: &mut Configuration,
	eth_config: &EthConfiguration,
) -> Result<ChainOps, ServiceError> {
	use crate::chain_spec::Network;

	config.keystore = sc_service::config::KeystoreConfig::InMemory;
	match Network::from_spec_id(config.chain_spec.id()) {
		Network::Impetus => {
			let PartialComponents {
				client,
				backend,
				import_queue,
				task_manager,
				other,
				..
			} = new_partial::<
				impetus_runtime::opaque::Block,
				impetus_runtime::RuntimeApi,
				HostFunctions,
				_,
			>(config, eth_config, build_aura_grandpa_import_queue)?;
			Ok(ChainOps::Impetus(
				client,
				backend,
				import_queue,
				task_manager,
				other.3,
			))
		}
		Network::Impulse => {
			let PartialComponents {
				client,
				backend,
				import_queue,
				task_manager,
				other,
				..
			} = new_partial::<
				impulse_runtime::opaque::Block,
				impulse_runtime::RuntimeApi,
				HostFunctions,
				_,
			>(config, eth_config, build_aura_grandpa_import_queue)?;
			Ok(ChainOps::Impulse(
				client,
				backend,
				import_queue,
				task_manager,
				other.3,
			))
		}
	}
}
```

- [ ] **Step 5: Update subcommand callsites in `command.rs`**

Each subcommand arm in `apps/node/node/src/command.rs:66-155` matches on the result of `service::new_chain_ops`. Replace those with `match ops { ChainOps::Impetus(...) => ..., ChainOps::Impulse(...) => ... }` blocks.

For example, `Subcommand::CheckBlock` becomes:

```rust
Some(Subcommand::CheckBlock(cmd)) => {
	let runner = cli.create_runner(cmd)?;
	runner.async_run(|mut config| {
		let ops = service::new_chain_ops(&mut config, &cli.eth)?;
		match ops {
			service::ChainOps::Impetus(client, _, import_queue, task_manager, _) => {
				Ok((cmd.run(client, import_queue), task_manager))
			}
			service::ChainOps::Impulse(client, _, import_queue, task_manager, _) => {
				Ok((cmd.run(client, import_queue), task_manager))
			}
		}
	})
}
```

Apply the same pattern to `ExportBlocks`, `ExportState`, `ImportBlocks`, `Revert`, `FrontierDb`. The `PurgeChain` arm does NOT call `new_chain_ops` and stays as-is.

- [ ] **Step 6: Verify build**

Run: `cd apps/node && cargo check -p frontier-template-node`

Expected: success.

- [ ] **Step 7: Commit**

```bash
git add apps/node/node/src/service.rs apps/node/node/src/command.rs
git commit -m "feat(node): dispatch service entry points on Network"
```

---

## Task 14: Refactor `benchmarking.rs` per runtime

**Files:**
- Modify: `apps/node/node/src/benchmarking.rs` (full rewrite)
- Modify: `apps/node/node/src/command.rs:156-211` (Benchmark subcommand)

The original file has one `RemarkBuilder`, one `TransferKeepAliveBuilder`, one `create_benchmark_extrinsic` — all hardcoded to `frontier_template_runtime`. We split it into two parallel `pub mod impetus { ... }` and `pub mod impulse { ... }` modules with identical bodies, differing only in the imported runtime crate. `inherent_benchmark_data` stays at the top level (runtime-agnostic).

- [ ] **Step 1: Rewrite `benchmarking.rs` with parallel modules**

Replace the entire contents of `apps/node/node/src/benchmarking.rs` with:

```rust
//! Contains code to setup the command invocations in [`super::command`] which would
//! otherwise bloat that module.

use std::{sync::Arc, time::Duration};

use scale_codec::Encode;
use sc_cli::Result;
use sc_client_api::BlockBackend;
use sp_core::{ecdsa, Pair};
use sp_inherents::{InherentData, InherentDataProvider};
use sp_runtime::{generic::Era, OpaqueExtrinsic, SaturatedConversion};

use fp_account::AccountId20;

use crate::client::FullClient;
use crate::service::HostFunctions;

/// Generates inherent data for the `benchmark overhead` command.
pub fn inherent_benchmark_data() -> Result<InherentData> {
	let mut inherent_data = InherentData::new();
	let d = Duration::from_millis(0);
	let timestamp = sp_timestamp::InherentDataProvider::new(d.into());

	futures::executor::block_on(timestamp.provide_inherent_data(&mut inherent_data))
		.map_err(|e| format!("creating inherent data: {e:?}"))?;
	Ok(inherent_data)
}

pub mod impetus {
	use super::*;
	use impetus_runtime as runtime;
	use runtime::{AccountId, Balance, BalancesCall, SystemCall};

	pub type Client = FullClient<runtime::opaque::Block, runtime::RuntimeApi, HostFunctions>;

	pub struct RemarkBuilder {
		client: Arc<Client>,
	}

	impl RemarkBuilder {
		pub fn new(client: Arc<Client>) -> Self {
			Self { client }
		}
	}

	impl frame_benchmarking_cli::ExtrinsicBuilder for RemarkBuilder {
		fn pallet(&self) -> &str {
			"system"
		}

		fn extrinsic(&self) -> &str {
			"remark"
		}

		fn build(&self, nonce: u32) -> std::result::Result<OpaqueExtrinsic, &'static str> {
			let acc = ecdsa::Pair::from_string("//Bob", None).expect("static values are valid; qed");
			let extrinsic: OpaqueExtrinsic = create_benchmark_extrinsic(
				self.client.as_ref(),
				acc,
				SystemCall::remark { remark: vec![] }.into(),
				nonce,
			)
			.into();
			Ok(extrinsic)
		}
	}

	pub struct TransferKeepAliveBuilder {
		client: Arc<Client>,
		dest: AccountId,
		value: Balance,
	}

	impl TransferKeepAliveBuilder {
		pub fn new(client: Arc<Client>, dest: AccountId, value: Balance) -> Self {
			Self {
				client,
				dest,
				value,
			}
		}
	}

	impl frame_benchmarking_cli::ExtrinsicBuilder for TransferKeepAliveBuilder {
		fn pallet(&self) -> &str {
			"balances"
		}

		fn extrinsic(&self) -> &str {
			"transfer_keep_alive"
		}

		fn build(&self, nonce: u32) -> std::result::Result<OpaqueExtrinsic, &'static str> {
			let acc = ecdsa::Pair::from_string("//Bob", None).expect("static values are valid; qed");
			let extrinsic: OpaqueExtrinsic = create_benchmark_extrinsic(
				self.client.as_ref(),
				acc,
				BalancesCall::transfer_keep_alive {
					dest: self.dest,
					value: self.value,
				}
				.into(),
				nonce,
			)
			.into();
			Ok(extrinsic)
		}
	}

	pub fn create_benchmark_extrinsic(
		client: &Client,
		sender: ecdsa::Pair,
		call: runtime::RuntimeCall,
		nonce: u32,
	) -> runtime::UncheckedExtrinsic {
		let genesis_hash = client
			.block_hash(0)
			.ok()
			.flatten()
			.expect("Genesis block exists; qed");
		let best_hash = client.chain_info().best_hash;
		let best_block = client.chain_info().best_number;

		let period = runtime::BlockHashCount::get()
			.checked_next_power_of_two()
			.map(|c| c / 2)
			.unwrap_or(2) as u64;
		let extra = runtime::SignedExtra::new((
			frame_system::CheckNonZeroSender::<runtime::Runtime>::new(),
			frame_system::CheckSpecVersion::<runtime::Runtime>::new(),
			frame_system::CheckTxVersion::<runtime::Runtime>::new(),
			frame_system::CheckGenesis::<runtime::Runtime>::new(),
			frame_system::CheckMortality::<runtime::Runtime>::from(Era::mortal(
				period,
				best_block.saturated_into(),
			)),
			frame_system::CheckNonce::<runtime::Runtime>::from(nonce),
			frame_system::CheckWeight::<runtime::Runtime>::new(),
			pallet_transaction_payment::ChargeTransactionPayment::<runtime::Runtime>::from(0),
		));

		let raw_payload = runtime::SignedPayload::from_raw(
			call.clone(),
			extra.clone(),
			(
				(),
				runtime::VERSION.spec_version,
				runtime::VERSION.transaction_version,
				genesis_hash,
				best_hash,
				(),
				(),
				(),
			),
		);
		let signature = raw_payload.using_encoded(|e| sender.sign(e));

		runtime::UncheckedExtrinsic::new_signed(
			call,
			AccountId20::from(sender.public()),
			runtime::Signature::new(signature),
			extra,
		)
	}
}

pub mod impulse {
	use super::*;
	use impulse_runtime as runtime;
	use runtime::{AccountId, Balance, BalancesCall, SystemCall};

	pub type Client = FullClient<runtime::opaque::Block, runtime::RuntimeApi, HostFunctions>;

	pub struct RemarkBuilder {
		client: Arc<Client>,
	}

	impl RemarkBuilder {
		pub fn new(client: Arc<Client>) -> Self {
			Self { client }
		}
	}

	impl frame_benchmarking_cli::ExtrinsicBuilder for RemarkBuilder {
		fn pallet(&self) -> &str {
			"system"
		}

		fn extrinsic(&self) -> &str {
			"remark"
		}

		fn build(&self, nonce: u32) -> std::result::Result<OpaqueExtrinsic, &'static str> {
			let acc = ecdsa::Pair::from_string("//Bob", None).expect("static values are valid; qed");
			let extrinsic: OpaqueExtrinsic = create_benchmark_extrinsic(
				self.client.as_ref(),
				acc,
				SystemCall::remark { remark: vec![] }.into(),
				nonce,
			)
			.into();
			Ok(extrinsic)
		}
	}

	pub struct TransferKeepAliveBuilder {
		client: Arc<Client>,
		dest: AccountId,
		value: Balance,
	}

	impl TransferKeepAliveBuilder {
		pub fn new(client: Arc<Client>, dest: AccountId, value: Balance) -> Self {
			Self {
				client,
				dest,
				value,
			}
		}
	}

	impl frame_benchmarking_cli::ExtrinsicBuilder for TransferKeepAliveBuilder {
		fn pallet(&self) -> &str {
			"balances"
		}

		fn extrinsic(&self) -> &str {
			"transfer_keep_alive"
		}

		fn build(&self, nonce: u32) -> std::result::Result<OpaqueExtrinsic, &'static str> {
			let acc = ecdsa::Pair::from_string("//Bob", None).expect("static values are valid; qed");
			let extrinsic: OpaqueExtrinsic = create_benchmark_extrinsic(
				self.client.as_ref(),
				acc,
				BalancesCall::transfer_keep_alive {
					dest: self.dest,
					value: self.value,
				}
				.into(),
				nonce,
			)
			.into();
			Ok(extrinsic)
		}
	}

	pub fn create_benchmark_extrinsic(
		client: &Client,
		sender: ecdsa::Pair,
		call: runtime::RuntimeCall,
		nonce: u32,
	) -> runtime::UncheckedExtrinsic {
		let genesis_hash = client
			.block_hash(0)
			.ok()
			.flatten()
			.expect("Genesis block exists; qed");
		let best_hash = client.chain_info().best_hash;
		let best_block = client.chain_info().best_number;

		let period = runtime::BlockHashCount::get()
			.checked_next_power_of_two()
			.map(|c| c / 2)
			.unwrap_or(2) as u64;
		let extra = runtime::SignedExtra::new((
			frame_system::CheckNonZeroSender::<runtime::Runtime>::new(),
			frame_system::CheckSpecVersion::<runtime::Runtime>::new(),
			frame_system::CheckTxVersion::<runtime::Runtime>::new(),
			frame_system::CheckGenesis::<runtime::Runtime>::new(),
			frame_system::CheckMortality::<runtime::Runtime>::from(Era::mortal(
				period,
				best_block.saturated_into(),
			)),
			frame_system::CheckNonce::<runtime::Runtime>::from(nonce),
			frame_system::CheckWeight::<runtime::Runtime>::new(),
			pallet_transaction_payment::ChargeTransactionPayment::<runtime::Runtime>::from(0),
		));

		let raw_payload = runtime::SignedPayload::from_raw(
			call.clone(),
			extra.clone(),
			(
				(),
				runtime::VERSION.spec_version,
				runtime::VERSION.transaction_version,
				genesis_hash,
				best_hash,
				(),
				(),
				(),
			),
		);
		let signature = raw_payload.using_encoded(|e| sender.sign(e));

		runtime::UncheckedExtrinsic::new_signed(
			call,
			AccountId20::from(sender.public()),
			runtime::Signature::new(signature),
			extra,
		)
	}
}
```

> Both modules are byte-for-byte identical except for the `use ..._runtime as runtime;` import on the third line. Yes, this is duplicate code; the spec accepts the duplication trade-off because runtime types differ structurally.

- [ ] **Step 2: Update `command.rs` Benchmark subcommand**

Replace `apps/node/node/src/command.rs:156-211` (the `#[cfg(feature = "runtime-benchmarks")] Some(Subcommand::Benchmark(cmd))` arm) with the full network-dispatched version below. Note that `Pallet` and `Machine` do not call `new_chain_ops`, so they dispatch on `chain_spec.id()` directly; `Block`/`Storage`/`Overhead`/`Extrinsic` dispatch on the `ChainOps` enum returned by `new_chain_ops`.

```rust
#[cfg(feature = "runtime-benchmarks")]
Some(Subcommand::Benchmark(cmd)) => {
	use crate::benchmarking::{inherent_benchmark_data, impetus, impulse};
	use crate::chain_spec::Network;
	use frame_benchmarking_cli::{BenchmarkCmd, ExtrinsicFactory, SUBSTRATE_REFERENCE_HARDWARE};

	let runner = cli.create_runner(cmd)?;
	match cmd {
		BenchmarkCmd::Pallet(cmd) => runner.sync_run(|config| {
			match Network::from_spec_id(config.chain_spec.id()) {
				Network::Impetus => {
					cmd.run_with_spec::<impetus_runtime::Hashing, ()>(Some(config.chain_spec))
				}
				Network::Impulse => {
					cmd.run_with_spec::<impulse_runtime::Hashing, ()>(Some(config.chain_spec))
				}
			}
		}),
		BenchmarkCmd::Block(cmd) => runner.sync_run(|mut config| {
			let ops = service::new_chain_ops(&mut config, &cli.eth)?;
			match ops {
				service::ChainOps::Impetus(client, _, _, _, _) => cmd.run(client),
				service::ChainOps::Impulse(client, _, _, _, _) => cmd.run(client),
			}
		}),
		BenchmarkCmd::Storage(cmd) => runner.sync_run(|mut config| {
			let ops = service::new_chain_ops(&mut config, &cli.eth)?;
			match ops {
				service::ChainOps::Impetus(client, backend, _, _, _) => {
					let db = backend.expose_db();
					let storage = backend.expose_storage();
					let shared_cache = backend.expose_shared_trie_cache();
					cmd.run(config, client, db, storage, shared_cache)
				}
				service::ChainOps::Impulse(client, backend, _, _, _) => {
					let db = backend.expose_db();
					let storage = backend.expose_storage();
					let shared_cache = backend.expose_shared_trie_cache();
					cmd.run(config, client, db, storage, shared_cache)
				}
			}
		}),
		BenchmarkCmd::Overhead(cmd) => runner.sync_run(|mut config| {
			let chain_name = config.chain_spec.name().to_string();
			let ops = service::new_chain_ops(&mut config, &cli.eth)?;
			match ops {
				service::ChainOps::Impetus(client, _, _, _, _) => {
					let ext_builder = impetus::RemarkBuilder::new(client.clone());
					cmd.run(
						chain_name,
						client,
						inherent_benchmark_data()?,
						Vec::new(),
						&ext_builder,
						false,
					)
				}
				service::ChainOps::Impulse(client, _, _, _, _) => {
					let ext_builder = impulse::RemarkBuilder::new(client.clone());
					cmd.run(
						chain_name,
						client,
						inherent_benchmark_data()?,
						Vec::new(),
						&ext_builder,
						false,
					)
				}
			}
		}),
		BenchmarkCmd::Extrinsic(cmd) => runner.sync_run(|mut config| {
			let ops = service::new_chain_ops(&mut config, &cli.eth)?;
			match ops {
				service::ChainOps::Impetus(client, _, _, _, _) => {
					let ext_factory = ExtrinsicFactory(vec![
						Box::new(impetus::RemarkBuilder::new(client.clone())),
						Box::new(impetus::TransferKeepAliveBuilder::new(
							client.clone(),
							runtime_common::admin_account(),
							1_000_000_000_000_000_000u128,
						)),
					]);
					cmd.run(client, inherent_benchmark_data()?, Vec::new(), &ext_factory)
				}
				service::ChainOps::Impulse(client, _, _, _, _) => {
					let ext_factory = ExtrinsicFactory(vec![
						Box::new(impulse::RemarkBuilder::new(client.clone())),
						Box::new(impulse::TransferKeepAliveBuilder::new(
							client.clone(),
							runtime_common::admin_account(),
							1_000_000_000_000_000_000u128,
						)),
					]);
					cmd.run(client, inherent_benchmark_data()?, Vec::new(), &ext_factory)
				}
			}
		}),
		BenchmarkCmd::Machine(cmd) => {
			runner.sync_run(|config| cmd.run(&config, SUBSTRATE_REFERENCE_HARDWARE.clone()))
		}
	}
}
```

> The Extrinsic branch's transfer dest was previously
> `get_account_id_from_seed::<sp_core::ecdsa::Public>("Alice")` and value
> was `EXISTENTIAL_DEPOSIT` (= 0). That helper was removed in Task 11.
> Use `runtime_common::admin_account()` and `1 IPL` (10^18) as a safe
> non-zero amount — the benchmark only needs a valid extrinsic, the
> transfer success is irrelevant.

- [ ] **Step 3: Verify build**

Run: `cd apps/node && cargo check -p frontier-template-node --features runtime-benchmarks`

Expected: success (or you fix any remaining hardcoded `frontier_template_runtime::` reference).

- [ ] **Step 4: Commit**

```bash
git add apps/node/node/src/benchmarking.rs apps/node/node/src/command.rs
git commit -m "feat(node): duplicate benchmark builders per runtime"
```

---

## Task 15: Drop `frontier-template-runtime` from node deps

**Files:**
- Modify: `apps/node/node/Cargo.toml` (remove old runtime line)

After Tasks 11-14, no source file in `node/src/` references `frontier_template_runtime`. Confirm and drop the dep.

- [ ] **Step 1: Confirm no references**

Run: `cd apps/node && grep -r frontier_template_runtime node/src/`

Expected: no output. If any line remains, fix it before proceeding.

- [ ] **Step 2: Edit `node/Cargo.toml`**

Delete the line `frontier-template-runtime = { workspace = true, features = ["std"] }` from `apps/node/node/Cargo.toml`.

- [ ] **Step 3: Verify**

Run: `cd apps/node && cargo check -p frontier-template-node`

Expected: success.

- [ ] **Step 4: Commit**

```bash
git add apps/node/node/Cargo.toml
git commit -m "refactor(node): drop dead frontier-template-runtime dep"
```

---

## Task 16: Delete `runtime/` crate and clean workspace manifest

**Files:**
- Delete: `apps/node/runtime/`
- Modify: `apps/node/Cargo.toml:2-7` (remove `runtime` member)
- Modify: `apps/node/Cargo.toml:175-178` (remove `frontier-template-runtime` workspace dep)

- [ ] **Step 1: Confirm no callers**

Run: `cd apps/node && grep -r 'frontier-template-runtime\|frontier_template_runtime' . --exclude-dir=target --exclude-dir=runtime`

Expected: no output. If references appear in `Dockerfile`, README, or AGENTS.md, those will be cleaned in Task 18.

- [ ] **Step 2: Remove from workspace manifest**

Edit `apps/node/Cargo.toml:2-7`:

```toml
members = [
	"node",
	"runtimes/common",
	"runtimes/impetus",
	"runtimes/impulse",
	"pallets/gasless-registry",
	"precompiles/gasless-registry",
]
```

Edit `apps/node/Cargo.toml:175-178` — delete the `frontier-template-runtime = { path = "runtime", default-features = false }` workspace dep line.

- [ ] **Step 3: Delete the directory**

```bash
rm -rf apps/node/runtime
```

- [ ] **Step 4: Verify**

Run: `cd apps/node && cargo check --workspace`

Expected: success.

- [ ] **Step 5: Commit**

```bash
git add apps/node/Cargo.toml apps/node/runtime
git commit -m "refactor(node): remove legacy frontier-template-runtime crate"
```

---

## Task 17: Generate and commit raw mainnet chain spec

**Files:**
- Create: `apps/node/chain-specs/impetus.json`

- [ ] **Step 1: Build the node**

Run: `cd apps/node && cargo build --release`

Expected: completes with `target/release/frontier-template-node` produced.

- [ ] **Step 2: Generate the raw chain spec**

```bash
mkdir -p apps/node/chain-specs
./apps/node/target/release/frontier-template-node build-spec \
	--chain impetus --raw --disable-default-bootnode \
	> apps/node/chain-specs/impetus.json
```

Expected: file created, ~30-60 KB. Skim the JSON: `id` should be `"impetus"`, `name` should be `"Impetus"`, the `evmChainId.chainId` path should equal `388266`, and `properties.tokenSymbol` should be `"IPT"`.

- [ ] **Step 3: Sanity check round-trip**

```bash
./apps/node/target/release/frontier-template-node \
	--chain ./apps/node/chain-specs/impetus.json \
	--tmp --validator --alice &
NODE_PID=$!
sleep 10
cast chain-id --rpc-url http://127.0.0.1:9944
kill $NODE_PID
wait $NODE_PID 2>/dev/null
```

Expected: `cast chain-id` prints `388266`.

- [ ] **Step 4: Commit**

```bash
git add apps/node/chain-specs/impetus.json
git commit -m "chore(node): commit raw impetus mainnet chain spec"
```

---

## Task 18: Update documentation and Dockerfile

**Files:**
- Modify: `AGENTS.md` (repo root)
- Modify: `apps/node/AGENTS.md` (if it exists and is not a symlink to root)
- Modify: `apps/node/Dockerfile`
- Modify: `apps/node/README.md`

- [ ] **Step 1: Update repo-root `AGENTS.md` chain header**

In `AGENTS.md` (repo root), replace the section currently reading:

```markdown
- **Chain name:** Artemis
- **Chain ID:** 322
- **Token:** ART (18 decimals)
```

with:

```markdown
- **Mainnet:** Impetus (chain id 388266, token IPT, SS58 11434, spec_name `impetus`)
- **Testnet:** Impulse (chain id 322644, token IPL, SS58 11348, spec_name `impulse`)
- **Dev mode:** alias of Impulse with manual seal enabled (`--chain dev`)
- **Token decimals:** 18
```

If `apps/node/AGENTS.md` exists and is not a symlink to repo root, apply the same edit there.

- [ ] **Step 2: Update Dockerfile chain default**

Edit `apps/node/Dockerfile`. Find any `CMD` or `ENTRYPOINT` line referencing `--chain artemis|local`. Replace `artemis` with `impulse`. If the Dockerfile sets `CMD ["frontier-template-node"]` without `--chain`, leave it alone — the default in `command.rs` is now `impulse_config()`.

- [ ] **Step 3: Update `apps/node/README.md`**

Search for any mention of `--chain artemis`, `--chain local`, `Artemis`, or `ART` token. Update to the new chain ids/aliases. The "Run a local dev node" section should read something like:

```markdown
## Run a local dev node

	cd apps/node && cargo run --release -- --chain dev --tmp --alice

	cast chain-id --rpc-url http://127.0.0.1:9944    # 322644
```

- [ ] **Step 4: Verify**

Run: `grep -r 'artemis\|Artemis\| ART ' apps/node/ AGENTS.md README.md 2>/dev/null | grep -v target`

Expected: no output.

- [ ] **Step 5: Commit**

```bash
git add AGENTS.md apps/node/AGENTS.md apps/node/Dockerfile apps/node/README.md
git commit -m "docs: update chain references for Impetus mainnet + Impulse testnet"
```

---

## Task 19: Final verification

**Files:** None (verification only)

- [ ] **Step 1: Format check**

Run: `cd apps/node && cargo fmt --check`

Expected: no output (code is formatted). If output appears, run `cargo fmt --all` and stage in a separate commit.

- [ ] **Step 2: Clippy sweep**

Run: `cd apps/node && cargo clippy --workspace --all-features -- -D warnings`

Expected: clean. Fix any new warnings inline.

- [ ] **Step 3: Workspace test**

Run: `cd apps/node && cargo test --workspace`

Expected: all tests pass, including the three `chain_spec::tests` from Task 11 and the ported `configured_base_extrinsic_weight_is_evm_compatible` tests in each new runtime crate.

- [ ] **Step 4: Smoke test all three chains**

```bash
cd apps/node
./target/release/frontier-template-node --chain impetus --tmp --alice &
sleep 10
test "$(cast chain-id --rpc-url http://127.0.0.1:9944)" = "388266" || (echo "FAIL: impetus chainId" && false)
test "$(cast balance 0xd2aE0A2139dC83Cb920e3cd7B9F640922D14b872 --rpc-url http://127.0.0.1:9944)" != "0" || (echo "FAIL: admin balance" && false)
kill %1; wait %1 2>/dev/null

./target/release/frontier-template-node --chain impulse --tmp --alice &
sleep 10
test "$(cast chain-id --rpc-url http://127.0.0.1:9944)" = "322644" || (echo "FAIL: impulse chainId" && false)
kill %1; wait %1 2>/dev/null

./target/release/frontier-template-node --chain dev --sealing manual --tmp --alice &
sleep 5
test "$(cast chain-id --rpc-url http://127.0.0.1:9944)" = "322644" || (echo "FAIL: dev chainId" && false)
kill %1; wait %1 2>/dev/null
```

Expected: each `cast chain-id` returns the matching value; the admin balance check returns non-zero.

- [ ] **Step 5: E2E suite passes**

```bash
cd apps/node
./target/release/frontier-template-node --chain dev --sealing manual --tmp --alice &
NODE_PID=$!
sleep 5
cd ../../packages/contracts
pnpm test
TEST_RC=$?
kill $NODE_PID
wait $NODE_PID 2>/dev/null
test $TEST_RC -eq 0 || (echo "FAIL: E2E" && false)
```

Expected: all Hardhat tests green.

- [ ] **Step 6: Commit nothing — verification only**

If any step fails, fix the underlying issue and stage that fix as a separate commit before closing the plan.

---

## Self-review notes

**Spec coverage:**
- Two runtime crates + common: Tasks 1-9.
- Node deps + chain_spec rewrite: Tasks 10-11.
- Command + service + benchmark dispatch: Tasks 12-14.
- Old runtime removal: Tasks 15-16.
- Raw chain spec commit: Task 17.
- Docs/Dockerfile: Task 18.
- Verification: Task 19.

**Open risks called out inline:**
- Task 5 may need extra trait bounds on the generic `R` in `GaslessEvmFee` if `<() as OnChargeEVMTransaction<R>>` requires more than `pallet_evm::Config`. The test is `cargo check -p runtime-common` — if it fails, add `pallet_balances::Config` and `frame_system::Config` bounds.
- Task 8 step 1.11's `pub use` block must match the exact identifiers exported from `runtime-common`. If a name is missing, compilation fails fast and you add it.
- Task 14's macro body has a deliberate `unimplemented!()` — read the **original** `apps/node/node/src/benchmarking.rs:33-220` (which the previous git revision contains via `git show HEAD~N:apps/node/node/src/benchmarking.rs`) and inline the per-runtime body before the verification step.

**Type consistency:**
- `Network` is the same struct in `chain_spec.rs` (Task 11) and used in `service.rs` + `command.rs` (Tasks 12-14). Helper `Network::from_spec_id` is added in Task 11.
- `ChainProfile.ss58_prefix` is `u32` for safety even though both values fit in `u16`; the runtime `parameter_types! { pub const SS58Prefix: u16 }` matches `frame_system::Config::SS58Prefix: Get<u16>`.
- `impetus_runtime::WASM_BINARY` and `impulse_runtime::WASM_BINARY` are `Option<&'static [u8]>` because both crates `include!(concat!(env!("OUT_DIR"), "/wasm_binary.rs"))`.
