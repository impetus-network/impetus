# Gasless Registry Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add an admin-managed gasless registry for selected Frontier EVM call selectors.

**Architecture:** Add `pallet-gasless-registry` as the policy store and evaluator. Add a runtime-local EVM runner wrapper that sets scoped transaction context around Frontier `call` execution, then replace `pallet_evm::Config::OnChargeTransaction` with a custom fee charger that reads that context and waives EVM gas withdrawal only for eligible calls. Keep Frontier transaction-pool validation unchanged, so zero-balance gasless UX remains out of scope.

**Tech Stack:** Rust, Substrate FRAME, Frontier EVM, `environmental` scoped runtime context, Hardhat E2E smoke tests

---

## File Map

| File | Responsibility |
|------|---------------|
| `packages/node/Cargo.toml` | Add `pallet-gasless-registry` as a workspace member and workspace dependency |
| `packages/node/pallets/gasless-registry/Cargo.toml` | New pallet manifest |
| `packages/node/pallets/gasless-registry/src/lib.rs` | Rule storage, admin calls, evaluator, events, errors, weight trait |
| `packages/node/pallets/gasless-registry/src/tests.rs` | Pallet unit tests for rules and evaluation |
| `packages/node/pallets/gasless-registry/src/benchmarking.rs` | Benchmarks for dispatchables and evaluator storage read |
| `packages/node/runtime/Cargo.toml` | Add runtime deps and features for the new pallet and `environmental` |
| `packages/node/runtime/src/gasless.rs` | Runtime EVM runner wrapper, scoped context, and custom fee charger |
| `packages/node/runtime/src/lib.rs` | Register pallet, configure constants, switch EVM runner and fee charger |
| `packages/node/node/src/chain_spec.rs` | Seed the dev chain with a gasless rule for live-node smoke tests |
| `packages/contracts/test/GaslessRegistry.test.ts` | Live-node Hardhat smoke tests for gasless and paid fallback behavior |

---

## Explicit Trade-Offs

The design spec asks for an independent test contract so the registry remains
generic. This plan uses the existing betting precompile for the first live-node
Hardhat smoke test because the dev chain can seed a deterministic rule for a
fixed precompile address without adding transaction-submission tooling for root
runtime calls.

That smoke test proves the Frontier fee-waiver path works against a real EVM
transaction, but it is not the generic-contract coverage requested by the spec.
Generic behavior is covered by pallet evaluator tests in this plan. A later
test can add a standalone Solidity contract once the test harness has a clean
way to submit `sudo(GaslessRegistry::set_rule(...))` or another management
origin call after deployment.

The plan also keeps these runtime-level checks as explicit MVP follow-ups:

- `eth_call` and `eth_estimateGas` do not mutate registry state.
- Gasless-eligible transactions from low-balance callers still fail Frontier
  transaction-pool balance validation.
- Registry evaluation overhead is included in block-weight accounting beyond
  the benchmarked pallet weight.

These are not ignored. They are listed as residual implementation risks because
they require runtime/API-level tests or deeper Frontier weight integration than
the first MVP smoke path.

---

### Task 1: Scaffold `pallet-gasless-registry`

**Files:**
- Modify: `packages/node/Cargo.toml`
- Create: `packages/node/pallets/gasless-registry/Cargo.toml`
- Create: `packages/node/pallets/gasless-registry/src/lib.rs`
- Create: `packages/node/pallets/gasless-registry/src/tests.rs`

- [ ] **Step 1: Add the pallet to the workspace**

Edit `packages/node/Cargo.toml`.

Add the member:

```toml
members = [
	"node",
	"runtime",
	"pallets/betting",
	"pallets/gasless-registry",
	"precompiles/betting",
]
```

Add the local dependency:

```toml
pallet-gasless-registry = { path = "pallets/gasless-registry", default-features = false }
```

- [ ] **Step 2: Create the pallet manifest**

Create `packages/node/pallets/gasless-registry/Cargo.toml`:

```toml
[package]
name = "pallet-gasless-registry"
version = "0.1.0"
license = "Unlicense"
description = "Admin-managed gasless EVM selector registry"
publish = false
authors = { workspace = true }
edition = { workspace = true }
repository = { workspace = true }

[dependencies]
scale-codec = { workspace = true }
scale-info = { workspace = true }

frame-benchmarking = { workspace = true, optional = true }
frame-support = { workspace = true }
frame-system = { workspace = true }
sp-core = { workspace = true }
sp-runtime = { workspace = true }

[dev-dependencies]
sp-io = { workspace = true, features = ["std"] }

[features]
default = ["std"]
std = [
	"scale-codec/std",
	"scale-info/std",
	"frame-benchmarking?/std",
	"frame-support/std",
	"frame-system/std",
	"sp-core/std",
	"sp-runtime/std",
]
runtime-benchmarks = [
	"frame-benchmarking/runtime-benchmarks",
	"frame-support/runtime-benchmarks",
	"frame-system/runtime-benchmarks",
	"sp-runtime/runtime-benchmarks",
]
try-runtime = [
	"frame-support/try-runtime",
	"frame-system/try-runtime",
	"sp-runtime/try-runtime",
]
```

- [ ] **Step 3: Write the pallet implementation**

Create `packages/node/pallets/gasless-registry/src/lib.rs`:

```rust
#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

#[cfg(test)]
mod tests;

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
	extern crate alloc;

	use alloc::vec::Vec;
	use frame_support::{
		pallet_prelude::*,
		traits::EnsureOrigin,
		weights::constants::RocksDbWeight,
	};
	use frame_system::pallet_prelude::*;
	use sp_core::{H160, U256};

	#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, MaxEncodedLen, TypeInfo)]
	pub struct Rule {
		pub enabled: bool,
		pub min_value: U256,
	}

	#[derive(Clone, Copy, Debug, Eq, PartialEq)]
	pub enum GaslessDecision {
		Gasless,
		Paid,
	}

	impl GaslessDecision {
		pub fn is_gasless(self) -> bool {
			matches!(self, Self::Gasless)
		}
	}

	pub trait WeightInfo {
		fn set_rule() -> Weight;
		fn remove_rule() -> Weight;
		fn evaluate() -> Weight;
	}

	impl WeightInfo for () {
		fn set_rule() -> Weight {
			Weight::from_parts(10_000, 0)
		}

		fn remove_rule() -> Weight {
			Weight::from_parts(10_000, 0)
		}

		fn evaluate() -> Weight {
			Weight::from_parts(10_000, 0)
		}
	}

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config<RuntimeEvent: From<Event<Self>>> {
		type ManageOrigin: EnsureOrigin<Self::RuntimeOrigin>;

		#[pallet::constant]
		type MaxGaslessGasLimit: Get<u64>;

		type WeightInfo: WeightInfo;
	}

	#[pallet::storage]
	pub type Rules<T: Config> =
		StorageMap<_, Blake2_128Concat, (H160, [u8; 4]), Rule>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		RuleSet {
			contract: H160,
			selector: [u8; 4],
			enabled: bool,
			min_value: U256,
		},
		RuleRemoved {
			contract: H160,
			selector: [u8; 4],
		},
	}

	#[pallet::error]
	pub enum Error<T> {
		RuleNotFound,
	}

	#[pallet::genesis_config]
	#[derive(frame_support::DefaultNoBound)]
	pub struct GenesisConfig<T: Config> {
		pub rules: Vec<(H160, [u8; 4], U256, bool)>,
		#[serde(skip)]
		pub _phantom: core::marker::PhantomData<T>,
	}

	#[pallet::genesis_build]
	impl<T: Config> BuildGenesisConfig for GenesisConfig<T> {
		fn build(&self) {
			for (contract, selector, min_value, enabled) in &self.rules {
				Rules::<T>::insert(
					(*contract, *selector),
					Rule {
						enabled: *enabled,
						min_value: *min_value,
					},
				);
			}
		}
	}

	impl<T: Config> Pallet<T> {
		pub fn evaluate(
			contract: H160,
			calldata: &[u8],
			value: U256,
			gas_limit: u64,
		) -> GaslessDecision {
			if calldata.len() < 4 {
				return GaslessDecision::Paid;
			}

			if gas_limit > T::MaxGaslessGasLimit::get() {
				return GaslessDecision::Paid;
			}

			let selector = [calldata[0], calldata[1], calldata[2], calldata[3]];
			match Rules::<T>::get((contract, selector)) {
				Some(rule) if rule.enabled && value >= rule.min_value => {
					GaslessDecision::Gasless
				}
				_ => GaslessDecision::Paid,
			}
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::call_index(0)]
		#[pallet::weight(T::WeightInfo::set_rule())]
		pub fn set_rule(
			origin: OriginFor<T>,
			contract: H160,
			selector: [u8; 4],
			min_value: U256,
			enabled: bool,
		) -> DispatchResult {
			T::ManageOrigin::ensure_origin(origin)?;

			Rules::<T>::insert((contract, selector), Rule { enabled, min_value });
			Self::deposit_event(Event::RuleSet {
				contract,
				selector,
				enabled,
				min_value,
			});

			Ok(())
		}

		#[pallet::call_index(1)]
		#[pallet::weight(T::WeightInfo::remove_rule())]
		pub fn remove_rule(
			origin: OriginFor<T>,
			contract: H160,
			selector: [u8; 4],
		) -> DispatchResult {
			T::ManageOrigin::ensure_origin(origin)?;
			ensure!(
				Rules::<T>::contains_key((contract, selector)),
				Error::<T>::RuleNotFound
			);

			Rules::<T>::remove((contract, selector));
			Self::deposit_event(Event::RuleRemoved { contract, selector });

			Ok(())
		}
	}
}
```

- [ ] **Step 4: Write pallet tests first**

Create `packages/node/pallets/gasless-registry/src/tests.rs`:

```rust
use frame_support::{assert_noop, assert_ok, derive_impl, traits::ConstU64};
use sp_core::{H160, U256};
use sp_runtime::BuildStorage;

use crate::{
	self as pallet_gasless_registry, Error, Event, GaslessDecision, Rules,
};

type Block = frame_system::mocking::MockBlock<Test>;

frame_support::construct_runtime!(
	pub enum Test {
		System: frame_system,
		GaslessRegistry: pallet_gasless_registry,
	}
);

#[derive_impl(frame_system::config_preludes::TestDefaultConfig)]
impl frame_system::Config for Test {
	type Block = Block;
}

impl pallet_gasless_registry::Config for Test {
	type ManageOrigin = frame_system::EnsureRoot<u64>;
	type MaxGaslessGasLimit = ConstU64<1_000_000>;
	type WeightInfo = ();
}

const CONTRACT: H160 = H160([0x11; 20]);
const OTHER_CONTRACT: H160 = H160([0x22; 20]);
const SELECTOR: [u8; 4] = [0xaa, 0xbb, 0xcc, 0xdd];

fn calldata() -> Vec<u8> {
	vec![0xaa, 0xbb, 0xcc, 0xdd, 0x01, 0x02]
}

fn new_test_ext() -> sp_io::TestExternalities {
	let storage = frame_system::GenesisConfig::<Test>::default()
		.build_storage()
		.unwrap();
	let mut ext = sp_io::TestExternalities::new(storage);
	ext.execute_with(|| System::set_block_number(1));
	ext
}

#[test]
fn genesis_build_seeds_rules() {
	let mut storage = frame_system::GenesisConfig::<Test>::default()
		.build_storage()
		.unwrap();

	pallet_gasless_registry::GenesisConfig::<Test> {
		rules: vec![(CONTRACT, SELECTOR, U256::from(10), true)],
		_phantom: Default::default(),
	}
	.assimilate_storage(&mut storage)
	.unwrap();

	let mut ext = sp_io::TestExternalities::new(storage);
	ext.execute_with(|| {
		let rule = Rules::<Test>::get((CONTRACT, SELECTOR)).unwrap();
		assert!(rule.enabled);
		assert_eq!(rule.min_value, U256::from(10));
	});
}

#[test]
fn set_rule_stores_enabled_rule() {
	new_test_ext().execute_with(|| {
		assert_ok!(GaslessRegistry::set_rule(
			RuntimeOrigin::root(),
			CONTRACT,
			SELECTOR,
			U256::from(10),
			true,
		));

		let rule = Rules::<Test>::get((CONTRACT, SELECTOR)).unwrap();
		assert!(rule.enabled);
		assert_eq!(rule.min_value, U256::from(10));
		System::assert_last_event(RuntimeEvent::GaslessRegistry(Event::RuleSet {
			contract: CONTRACT,
			selector: SELECTOR,
			enabled: true,
			min_value: U256::from(10),
		}));
	});
}

#[test]
fn set_rule_accepts_zero_min_value_and_disabled_rule() {
	new_test_ext().execute_with(|| {
		assert_ok!(GaslessRegistry::set_rule(
			RuntimeOrigin::root(),
			CONTRACT,
			SELECTOR,
			U256::zero(),
			false,
		));

		let rule = Rules::<Test>::get((CONTRACT, SELECTOR)).unwrap();
		assert!(!rule.enabled);
		assert_eq!(rule.min_value, U256::zero());
	});
}

#[test]
fn set_rule_requires_root() {
	new_test_ext().execute_with(|| {
		assert_noop!(
			GaslessRegistry::set_rule(
				RuntimeOrigin::signed(1),
				CONTRACT,
				SELECTOR,
				U256::zero(),
				true,
			),
			sp_runtime::DispatchError::BadOrigin,
		);
	});
}

#[test]
fn remove_rule_deletes_existing_rule() {
	new_test_ext().execute_with(|| {
		assert_ok!(GaslessRegistry::set_rule(
			RuntimeOrigin::root(),
			CONTRACT,
			SELECTOR,
			U256::zero(),
			true,
		));

		assert_ok!(GaslessRegistry::remove_rule(
			RuntimeOrigin::root(),
			CONTRACT,
			SELECTOR,
		));

		assert!(!Rules::<Test>::contains_key((CONTRACT, SELECTOR)));
		System::assert_last_event(RuntimeEvent::GaslessRegistry(Event::RuleRemoved {
			contract: CONTRACT,
			selector: SELECTOR,
		}));
	});
}

#[test]
fn remove_rule_rejects_missing_rule() {
	new_test_ext().execute_with(|| {
		assert_noop!(
			GaslessRegistry::remove_rule(RuntimeOrigin::root(), CONTRACT, SELECTOR),
			Error::<Test>::RuleNotFound,
		);
	});
}

#[test]
fn evaluate_returns_gasless_for_matching_enabled_rule() {
	new_test_ext().execute_with(|| {
		assert_ok!(GaslessRegistry::set_rule(
			RuntimeOrigin::root(),
			CONTRACT,
			SELECTOR,
			U256::from(10),
			true,
		));

		assert_eq!(
			GaslessRegistry::evaluate(CONTRACT, &calldata(), U256::from(10), 500_000),
			GaslessDecision::Gasless,
		);
	});
}

#[test]
fn evaluate_returns_paid_for_non_matching_cases() {
	new_test_ext().execute_with(|| {
		assert_ok!(GaslessRegistry::set_rule(
			RuntimeOrigin::root(),
			CONTRACT,
			SELECTOR,
			U256::from(10),
			true,
		));
		assert_ok!(GaslessRegistry::set_rule(
			RuntimeOrigin::root(),
			OTHER_CONTRACT,
			SELECTOR,
			U256::zero(),
			false,
		));

		assert_eq!(
			GaslessRegistry::evaluate(CONTRACT, &[0xaa, 0xbb], U256::from(10), 500_000),
			GaslessDecision::Paid,
		);
		assert_eq!(
			GaslessRegistry::evaluate(OTHER_CONTRACT, &calldata(), U256::from(10), 500_000),
			GaslessDecision::Paid,
		);
		assert_eq!(
			GaslessRegistry::evaluate(CONTRACT, &calldata(), U256::from(9), 500_000),
			GaslessDecision::Paid,
		);
		assert_eq!(
			GaslessRegistry::evaluate(CONTRACT, &calldata(), U256::from(10), 1_000_001),
			GaslessDecision::Paid,
		);
	});
}
```

- [ ] **Step 5: Run pallet tests**

Run:

```bash
cd packages/node && cargo test -p pallet-gasless-registry
```

Expected: all `pallet-gasless-registry` tests pass.

- [ ] **Step 6: Commit**

```bash
git add packages/node/Cargo.toml packages/node/pallets/gasless-registry
git commit -m "feat(gasless): add registry pallet"
```

---

### Task 2: Register the pallet in the runtime

**Files:**
- Modify: `packages/node/runtime/Cargo.toml`
- Modify: `packages/node/runtime/src/lib.rs`
- Modify: `packages/node/node/src/chain_spec.rs`

- [ ] **Step 1: Add runtime dependencies**

Edit `packages/node/runtime/Cargo.toml`.

Add under local dependencies:

```toml
pallet-gasless-registry = { workspace = true }
```

Add `environmental` under dependencies:

```toml
environmental = { workspace = true }
```

Add to `std`:

```toml
"environmental/std",
"pallet-gasless-registry/std",
```

Add to `runtime-benchmarks`:

```toml
"pallet-gasless-registry/runtime-benchmarks",
```

- [ ] **Step 2: Configure the pallet**

Edit `packages/node/runtime/src/lib.rs`.

Add the runtime module declaration near the existing modules:

```rust
mod gasless;
```

Add constants and config near the EVM configuration:

```rust
parameter_types! {
	pub const MaxGaslessGasLimit: u64 = 5_000_000;
}

impl pallet_gasless_registry::Config for Runtime {
	type ManageOrigin = frame_system::EnsureRoot<AccountId>;
	type MaxGaslessGasLimit = MaxGaslessGasLimit;
	type WeightInfo = ();
}
```

Add the pallet to `construct_runtime!` after `Betting`:

```rust
#[runtime::pallet_index(14)]
pub type GaslessRegistry = pallet_gasless_registry;
```

Add it to runtime benchmarks:

```rust
[pallet_gasless_registry, GaslessRegistry]
```

- [ ] **Step 3: Verify and seed a dev-chain rule for smoke tests**

Verify the selector before editing `chain_spec.rs`:

```bash
cast sig 'placeBet(uint8,address,uint256)'
```

Expected:

```text
0x3e72be67
```

This matches the precompile crate's selector convention in
`packages/node/precompiles/betting/src/lib.rs`, which computes the first four
bytes of `keccak256(signature)`.

Edit `packages/node/node/src/chain_spec.rs`.

Add constants near `UNITS`:

```rust
const BETTING_PRECOMPILE: H160 = H160(hex!("0000000000000000000000000000000000000801"));
const PLACE_BET_SELECTOR: [u8; 4] = hex!("3e72be67");
```

Add the gasless genesis patch inside `testnet_genesis`:

```rust
"gaslessRegistry": {
	"rules": vec![(
		BETTING_PRECOMPILE,
		PLACE_BET_SELECTOR,
		U256::from(UNITS),
		true,
	)]
},
```

This keeps production behavior explicit through chain spec configuration while
making the local dev node usable for deterministic gasless smoke tests.

- [ ] **Step 4: Verify runtime and node compilation**

Run:

```bash
cd packages/node && cargo check -p frontier-template-runtime && cargo check -p frontier-template-node
```

Expected: runtime and node compile. Warnings are acceptable only if they are unrelated to files changed in this task.

- [ ] **Step 5: Commit**

```bash
git add packages/node/runtime/Cargo.toml packages/node/runtime/src/lib.rs packages/node/node/src/chain_spec.rs
git commit -m "feat(runtime): register gasless registry"
```

---

### Task 3: Add runtime EVM context and fee charger

**Files:**
- Create: `packages/node/runtime/src/gasless.rs`
- Modify: `packages/node/runtime/src/lib.rs`

- [ ] **Step 1: Create scoped context and fee charger**

Create `packages/node/runtime/src/gasless.rs`:

```rust
use alloc::vec::Vec;
use core::marker::PhantomData;

use ethereum::AuthorizationList;
use pallet_evm::{
	runner::Runner as EvmRunner, Error as EvmError, OnChargeEVMTransaction,
};
use sp_core::{H160, H256, U256};
use frame_support::weights::Weight;

use crate::Runtime;

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

pub struct GaslessEvmFee;

impl OnChargeEVMTransaction<Runtime> for GaslessEvmFee {
	type LiquidityInfo =
		GaslessLiquidityInfo<<() as OnChargeEVMTransaction<Runtime>>::LiquidityInfo>;

	fn withdraw_fee(who: &H160, fee: U256) -> Result<Self::LiquidityInfo, EvmError<Runtime>> {
		if fee.is_zero() {
			return <() as OnChargeEVMTransaction<Runtime>>::withdraw_fee(who, fee)
				.map(GaslessLiquidityInfo::Paid);
		}

		let gasless = GASLESS_EVM_CONTEXT::with(|context| {
			context.is_transactional
				&& pallet_gasless_registry::Pallet::<Runtime>::evaluate(
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
			<() as OnChargeEVMTransaction<Runtime>>::withdraw_fee(who, fee)
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
				let tip = <() as OnChargeEVMTransaction<Runtime>>::correct_and_deposit_fee(
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
			<() as OnChargeEVMTransaction<Runtime>>::pay_priority_fee(tip);
		}
	}
}

pub struct GaslessEvmRunner(PhantomData<Runtime>);
```

- [ ] **Step 2: Switch the runtime fee charger**

Edit `packages/node/runtime/src/lib.rs`.

Change:

```rust
type OnChargeTransaction = ();
```

to:

```rust
type OnChargeTransaction = gasless::GaslessEvmFee;
```

- [ ] **Step 3: Verify runtime compilation**

Run:

```bash
cd packages/node && cargo check -p frontier-template-runtime
```

Expected: runtime compiles with the custom fee charger type available.

- [ ] **Step 4: Commit the fee charger**

```bash
git add packages/node/runtime/src/gasless.rs packages/node/runtime/src/lib.rs
git commit -m "feat(gasless): add evm fee charger"
```

---

### Task 4: Add EVM runner wrapper for transaction context

**Files:**
- Modify: `packages/node/runtime/src/gasless.rs`
- Modify: `packages/node/runtime/src/lib.rs`

- [ ] **Step 1: Implement `GaslessEvmRunner`**

Append this implementation to `packages/node/runtime/src/gasless.rs`:

```rust
impl EvmRunner<Runtime> for GaslessEvmRunner {
	type Error = EvmError<Runtime>;

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
		evm_config: &evm::Config,
	) -> Result<(), pallet_evm::runner::RunnerError<Self::Error>> {
		pallet_evm::runner::stack::Runner::<Runtime>::validate(
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
		config: &evm::Config,
	) -> Result<fp_evm::CallInfo, pallet_evm::runner::RunnerError<Self::Error>> {
		let mut context = GaslessEvmContext {
			target,
			input: input.clone(),
			value,
			gas_limit,
			is_transactional,
		};

		GASLESS_EVM_CONTEXT::using_once(&mut context, || {
			pallet_evm::runner::stack::Runner::<Runtime>::call(
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
		config: &evm::Config,
	) -> Result<fp_evm::CreateInfo, pallet_evm::runner::RunnerError<Self::Error>> {
		pallet_evm::runner::stack::Runner::<Runtime>::create(
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
		config: &evm::Config,
	) -> Result<fp_evm::CreateInfo, pallet_evm::runner::RunnerError<Self::Error>> {
		pallet_evm::runner::stack::Runner::<Runtime>::create2(
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
		config: &evm::Config,
		contract_address: H160,
	) -> Result<fp_evm::CreateInfo, pallet_evm::runner::RunnerError<Self::Error>> {
		pallet_evm::runner::stack::Runner::<Runtime>::create_force_address(
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

- [ ] **Step 2: Switch the runtime EVM runner**

Edit `packages/node/runtime/src/lib.rs`.

Change:

```rust
type Runner = pallet_evm::runner::stack::Runner<Self>;
```

to:

```rust
type Runner = gasless::GaslessEvmRunner;
```

- [ ] **Step 3: Run runtime check**

Run:

```bash
cd packages/node && cargo check -p frontier-template-runtime
```

Expected: runtime compiles.

- [ ] **Step 4: Commit**

```bash
git add packages/node/runtime/src/gasless.rs packages/node/runtime/src/lib.rs
git commit -m "feat(gasless): pass evm call context"
```

---

### Task 5: Add benchmarks and replace measured fallback weights

**Files:**
- Modify: `packages/node/pallets/gasless-registry/src/lib.rs`
- Create: `packages/node/pallets/gasless-registry/src/benchmarking.rs`
- Modify: `packages/node/runtime/src/lib.rs`

- [ ] **Step 1: Add benchmark code**

Create `packages/node/pallets/gasless-registry/src/benchmarking.rs`:

```rust
#![cfg(feature = "runtime-benchmarks")]

use super::*;
use frame_benchmarking::{benchmarks, impl_benchmark_test_suite, whitelisted_caller};
use frame_system::RawOrigin;
use sp_core::{H160, U256};

benchmarks! {
	set_rule {
		let contract = H160([0x11; 20]);
		let selector = [0xaa, 0xbb, 0xcc, 0xdd];
		let min_value = U256::from(10);
	}: _(RawOrigin::Root, contract, selector, min_value, true)
	verify {
		let rule = Rules::<T>::get((contract, selector)).unwrap();
		assert!(rule.enabled);
		assert_eq!(rule.min_value, min_value);
	}

	remove_rule {
		let contract = H160([0x11; 20]);
		let selector = [0xaa, 0xbb, 0xcc, 0xdd];
		Rules::<T>::insert((contract, selector), Rule {
			enabled: true,
			min_value: U256::from(10),
		});
	}: _(RawOrigin::Root, contract, selector)
	verify {
		assert!(!Rules::<T>::contains_key((contract, selector)));
	}

	evaluate {
		let _caller: T::AccountId = whitelisted_caller();
		let contract = H160([0x11; 20]);
		let selector = [0xaa, 0xbb, 0xcc, 0xdd];
		let calldata = [0xaa, 0xbb, 0xcc, 0xdd, 0x01, 0x02];
		Rules::<T>::insert((contract, selector), Rule {
			enabled: true,
			min_value: U256::from(10),
		});
	}: {
		let decision = Pallet::<T>::evaluate(
			contract,
			&calldata,
			U256::from(10),
			T::MaxGaslessGasLimit::get(),
		);
		assert!(decision.is_gasless());
	}

	impl_benchmark_test_suite!(
		Pallet,
		crate::tests::new_test_ext(),
		crate::tests::Test
	);
}
```

- [ ] **Step 2: Replace unit weights with DB-aware weights**

In `packages/node/pallets/gasless-registry/src/lib.rs`, replace the `impl WeightInfo for ()` body with DB-aware fallback weights:

```rust
impl WeightInfo for () {
	fn set_rule() -> Weight {
		Weight::from_parts(10_000, 0).saturating_add(RocksDbWeight::get().writes(1))
	}

	fn remove_rule() -> Weight {
		Weight::from_parts(10_000, 0)
			.saturating_add(RocksDbWeight::get().reads(1))
			.saturating_add(RocksDbWeight::get().writes(1))
	}

	fn evaluate() -> Weight {
		Weight::from_parts(10_000, 0).saturating_add(RocksDbWeight::get().reads(1))
	}
}
```

- [ ] **Step 3: Run benchmark compile check**

Run:

```bash
cd packages/node && cargo check -p frontier-template-runtime --features runtime-benchmarks
```

Expected: runtime benchmark feature compiles.

- [ ] **Step 4: Commit**

```bash
git add packages/node/pallets/gasless-registry/src/lib.rs packages/node/pallets/gasless-registry/src/benchmarking.rs packages/node/runtime/src/lib.rs
git commit -m "feat(gasless): add registry benchmarks"
```

---

### Task 6: Add live-node Hardhat smoke tests

**Files:**
- Create: `packages/contracts/test/GaslessRegistry.test.ts`

This is a pragmatic smoke test using the existing betting precompile. Do not
interpret it as the spec's independent-contract coverage. It verifies the
runtime fee-waiver path and paid fallback against a deterministic address
registered in the dev chain spec.

- [ ] **Step 1: Add a gasless smoke test**

Create `packages/contracts/test/GaslessRegistry.test.ts`:

```typescript
import { expect } from "chai";
import { ethers } from "hardhat";
import {
  DEV_ACCOUNTS,
  DEFAULT_BET_AMOUNT,
  NATIVE_TOKEN,
  getBalance,
  getBettingPrecompile,
} from "./helpers/setup";

const BETTING_PRECOMPILE = "0x0000000000000000000000000000000000000801";
const PLACE_BET_SELECTOR = "0x3e72be67";

describe("GaslessRegistry", function () {
  this.timeout(60_000);

  before(async function () {
    try {
      await ethers.provider.getBlockNumber();
    } catch {
      this.skip();
    }
  });

  it("keeps registered placeBet gasless while still transferring bet value", async function () {
    const bob = await getBettingPrecompile(DEV_ACCOUNTS.bob.privateKey);
    const bobAddress = DEV_ACCOUNTS.bob.address;
    const before = await getBalance(bobAddress);

    const tx = await bob.placeBet(31, NATIVE_TOKEN, DEFAULT_BET_AMOUNT, {
      value: DEFAULT_BET_AMOUNT,
      gasLimit: 500_000,
    });
    const receipt = await tx.wait();
    expect(receipt?.status).to.equal(1);

    const after = await getBalance(bobAddress);
    expect(before - after).to.equal(DEFAULT_BET_AMOUNT);
  });

  it("falls back to paid execution above the gasless gas cap", async function () {
    const charlie = await getBettingPrecompile(DEV_ACCOUNTS.charlie.privateKey);
    const charlieAddress = DEV_ACCOUNTS.charlie.address;
    const before = await getBalance(charlieAddress);

    const tx = await charlie.placeBet(32, NATIVE_TOKEN, DEFAULT_BET_AMOUNT, {
      value: DEFAULT_BET_AMOUNT,
      gasLimit: 6_000_000,
    });
    const receipt = await tx.wait();
    expect(receipt?.status).to.equal(1);

    const after = await getBalance(charlieAddress);
    expect(before - after).to.be.greaterThan(DEFAULT_BET_AMOUNT);
  });

  it("documents the registered selector under test", async function () {
    expect(BETTING_PRECOMPILE).to.equal("0x0000000000000000000000000000000000000801");
    expect(PLACE_BET_SELECTOR).to.match(/^0x[0-9a-f]{8}$/);
  });
});
```

- [ ] **Step 2: Run contract compile**

Run:

```bash
cd packages/contracts && pnpm hardhat compile
```

Expected: Hardhat compiles TypeScript and Solidity artifacts.

- [ ] **Step 3: Run live-node smoke test**

In one terminal, run:

```bash
cd packages/node && cargo run -p frontier-template-node -- --dev
```

In another terminal, run:

```bash
cd packages/contracts && pnpm hardhat test --network substrate test/GaslessRegistry.test.ts
```

Expected: tests pass against the live dev node. The dev chain spec seeds
`placeBet(uint8,address,uint256)` as gasless for the betting precompile.

- [ ] **Step 4: Commit**

```bash
git add packages/contracts/test/GaslessRegistry.test.ts
git commit -m "test(gasless): add hardhat smoke tests"
```

---

### Task 7: Final verification

**Files:**
- All files changed by Tasks 1-6

- [ ] **Step 1: Run Rust tests**

Run:

```bash
cd packages/node && cargo test -p pallet-gasless-registry
```

Expected: all gasless registry pallet tests pass.

- [ ] **Step 2: Run runtime check**

Run:

```bash
cd packages/node && cargo check -p frontier-template-runtime
```

Expected: runtime compiles.

- [ ] **Step 3: Run benchmark feature check**

Run:

```bash
cd packages/node && cargo check -p frontier-template-runtime --features runtime-benchmarks
```

Expected: runtime compiles with benchmark features.

- [ ] **Step 4: Run diff check**

Run:

```bash
git diff --check
```

Expected: no output.

- [ ] **Step 5: Verify the seeded selector**

Run:

```bash
cast sig 'placeBet(uint8,address,uint256)'
```

Expected:

```text
0x3e72be67
```

- [ ] **Step 6: Audit registry mutation sites**

Run:

```bash
rg "Rules::<.*(insert|remove)|Rules::<.*>::(insert|remove)" packages/node
```

Expected: mutation sites are limited to `set_rule`, `remove_rule`, and genesis
build for `pallet-gasless-registry`. The EVM fee charger must only call
`evaluate`, which reads storage.

- [ ] **Step 7: Inspect final history**

Run:

```bash
git log --oneline -8
```

Expected: one focused commit per task, with the newest commits covering pallet, runtime registration, fee charger, context runner, benchmarks, and smoke tests.

---

## Self-Review

- Spec coverage: The plan covers registry storage, admin calls, fallback behavior, `min_value`, `MaxGaslessGasLimit`, scoped EVM transaction context, custom fee charging, unchanged pool validation behavior, benchmarks, and live-node smoke tests.
- Spec divergence: The live-node Hardhat smoke test uses the existing betting precompile instead of an independent Solidity contract. This is an explicit MVP trade-off, not generic-contract coverage.
- Deferred runtime coverage: `eth_call`/`eth_estimateGas` non-mutation, low-balance pool rejection, and full block-weight accounting integration remain residual risks for follow-up runtime/API tests.
- Scope: Zero-balance pool admission, sponsor budgets, per-account usage limits, per-rule gas caps, and Frontier source patching remain out of scope.
- Main implementation risk: Task 4 depends on the Frontier `Runner` trait signature from the checked-out dependency. If compilation reports a method signature mismatch, update the wrapper method signature to match Frontier and preserve the behavior from this plan: only `call` sets gasless context; `create`, `create2`, validation, and force-create remain paid/default.
