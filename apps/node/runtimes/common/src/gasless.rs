use alloc::vec::Vec;
use core::marker::PhantomData;

use ethereum::AuthorizationList;
use fp_evm::{CallInfo, CreateInfo, StateOverride};
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

/// EVM fee handler that waives the fee for calls matching an enabled gasless
/// rule, and otherwise delegates to `Inner`.
///
/// `Inner` is the underlying [`OnChargeEVMTransaction`] that actually moves the
/// (non-waived) fee. It defaults to `()`, which burns the base fee (Frontier's
/// default) — impulse keeps this. impetus passes
/// `pallet_evm::EVMFungibleAdapter<Balances, DealWithFees>` so the base fee is
/// split 80% treasury / 20% author instead of burned.
pub struct GaslessEvmFee<R, Inner = ()>(PhantomData<(R, Inner)>);

impl<R, Inner> OnChargeEVMTransaction<R> for GaslessEvmFee<R, Inner>
where
	R: pallet_evm::Config + pallet_gasless_registry::Config,
	Inner: OnChargeEVMTransaction<R>,
	pallet_evm::BalanceOf<R>: TryFrom<U256> + Into<U256>,
{
	type LiquidityInfo = GaslessLiquidityInfo<Inner::LiquidityInfo>;

	fn withdraw_fee(who: &H160, fee: U256) -> Result<Self::LiquidityInfo, EvmError<R>> {
		if fee.is_zero() {
			return Inner::withdraw_fee(who, fee).map(GaslessLiquidityInfo::Paid);
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
			Inner::withdraw_fee(who, fee).map(GaslessLiquidityInfo::Paid)
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
				let tip = Inner::correct_and_deposit_fee(who, corrected_fee, base_fee, paid);
				GaslessLiquidityInfo::Paid(tip)
			}
		}
	}

	fn pay_priority_fee(tip: Self::LiquidityInfo) {
		if let GaslessLiquidityInfo::Paid(tip) = tip {
			Inner::pay_priority_fee(tip);
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
		state_override: StateOverride,
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
				state_override,
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
