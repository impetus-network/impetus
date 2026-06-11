#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

use alloc::vec;
use core::marker::PhantomData;

use fp_evm::{ExitError, ExitRevert, ExitSucceed, PrecompileFailure, PrecompileOutput};
use frame_support::dispatch::RawOrigin;
use pallet_evm::{Precompile, PrecompileHandle, PrecompileResult};
use sp_core::{H160, H256, U256};

/// Precompile address: 0x0800 (2048)
pub const PRECOMPILE_ADDRESS: u64 = 2048;

// Function selectors (keccak256 of signature, first 4 bytes)
const GET_RULE: [u8; 4] = hex_literal::hex!("67e01e58"); // getRule(address,bytes4)
const IS_GASLESS: [u8; 4] = hex_literal::hex!("3cbccf5d"); // isGasless(address,bytes,uint256,uint256)
const SET_RULE: [u8; 4] = hex_literal::hex!("e4f4eb64"); // setRule(address,bytes4,uint256,bool)
const REMOVE_RULE: [u8; 4] = hex_literal::hex!("68070a34"); // removeRule(address,bytes4)

// Event topic hashes (keccak256 of event signature)
const RULE_SET_TOPIC: H256 = H256(hex_literal::hex!(
	"d33b3f7f907343ac80a40e0a81bc42efae6d2cb9ff383511a6930ae9d60c9c1c"
));
const RULE_REMOVED_TOPIC: H256 = H256(hex_literal::hex!(
	"3acf3cee52347289b24fe5683e3478c2d19fce279d156ad5cc8b6b2c4bbacdb7"
));

// Gas costs
const GAS_READ: u64 = 200;
const GAS_WRITE: u64 = 20_000;

pub struct GaslessRegistryPrecompile<R>(PhantomData<R>);

impl<R> Precompile for GaslessRegistryPrecompile<R>
where
	R: pallet_gasless_registry::Config + pallet_sudo::Config + pallet_evm::Config,
	<R as frame_system::Config>::AccountId: Into<H160>,
	<R as frame_system::Config>::RuntimeOrigin:
		From<RawOrigin<<R as frame_system::Config>::AccountId>>,
{
	fn execute(handle: &mut impl PrecompileHandle) -> PrecompileResult {
		let input = handle.input().to_vec();
		if input.len() < 4 {
			return Err(PrecompileFailure::Error {
				exit_status: ExitError::Other("input too short".into()),
			});
		}

		let selector = [input[0], input[1], input[2], input[3]];
		let data = &input[4..];

		match selector {
			GET_RULE => Self::get_rule(handle, data),
			IS_GASLESS => Self::is_gasless(handle, data),
			SET_RULE => Self::set_rule(handle, data),
			REMOVE_RULE => Self::remove_rule(handle, data),
			_ => Err(PrecompileFailure::Error {
				exit_status: ExitError::Other("unknown selector".into()),
			}),
		}
	}
}

impl<R> GaslessRegistryPrecompile<R>
where
	R: pallet_gasless_registry::Config + pallet_sudo::Config + pallet_evm::Config,
	<R as frame_system::Config>::AccountId: Into<H160>,
	<R as frame_system::Config>::RuntimeOrigin:
		From<RawOrigin<<R as frame_system::Config>::AccountId>>,
{
	/// getRule(address contract_, bytes4 selector) -> (bool enabled, uint256 minValue)
	fn get_rule(handle: &mut impl PrecompileHandle, data: &[u8]) -> PrecompileResult {
		handle
			.record_cost(GAS_READ)
			.map_err(|e| PrecompileFailure::Error { exit_status: e })?;

		if data.len() < 64 {
			return Err(revert("invalid input length"));
		}

		let contract = read_address(data, 0);
		let selector = read_bytes4(data, 32);

		let (enabled, min_value) =
			match pallet_gasless_registry::Rules::<R>::get((contract, selector)) {
				Some(rule) => (rule.enabled, rule.min_value),
				None => (false, U256::zero()),
			};

		let mut output = vec![0u8; 64];
		write_bool(&mut output, 0, enabled);
		write_u256(&mut output, 32, min_value);

		Ok(PrecompileOutput {
			exit_status: ExitSucceed::Returned,
			output,
		})
	}

	/// isGasless(address contract_, bytes calldata_, uint256 value, uint256 gasLimit) -> bool
	fn is_gasless(handle: &mut impl PrecompileHandle, data: &[u8]) -> PrecompileResult {
		handle
			.record_cost(GAS_READ)
			.map_err(|e| PrecompileFailure::Error { exit_status: e })?;

		if data.len() < 128 {
			return Err(revert("invalid input length"));
		}

		let contract = read_address(data, 0);
		let calldata_offset = read_u256(data, 32).low_u64() as usize;
		let value = read_u256(data, 64);
		let gas_limit = read_u256(data, 96).low_u64();

		if data.len() < calldata_offset + 32 {
			return Err(revert("invalid calldata offset"));
		}
		let calldata_len = read_u256(data, calldata_offset).low_u64() as usize;
		if data.len() < calldata_offset + 32 + calldata_len {
			return Err(revert("calldata out of bounds"));
		}
		let calldata = &data[calldata_offset + 32..calldata_offset + 32 + calldata_len];

		let decision =
			pallet_gasless_registry::Pallet::<R>::evaluate(contract, calldata, value, gas_limit);

		let mut output = vec![0u8; 32];
		write_bool(&mut output, 0, decision.is_gasless());

		Ok(PrecompileOutput {
			exit_status: ExitSucceed::Returned,
			output,
		})
	}

	/// setRule(address contract_, bytes4 selector, uint256 minValue, bool enabled)
	fn set_rule(handle: &mut impl PrecompileHandle, data: &[u8]) -> PrecompileResult {
		if handle.is_static() {
			return Err(PrecompileFailure::Error {
				exit_status: ExitError::Other("cannot call in static context".into()),
			});
		}
		handle
			.record_cost(GAS_WRITE)
			.map_err(|e| PrecompileFailure::Error { exit_status: e })?;

		if data.len() < 128 {
			return Err(revert("invalid input length"));
		}

		Self::ensure_admin(handle.context().caller)?;

		let contract = read_address(data, 0);
		let selector = read_bytes4(data, 32);
		let min_value = read_u256(data, 64);
		let enabled = read_bool(data, 96);

		pallet_gasless_registry::Pallet::<R>::set_rule(
			RawOrigin::Root.into(),
			contract,
			selector,
			min_value,
			enabled,
		)
		.map_err(|_| revert("set_rule dispatch failed"))?;

		let addr = handle.code_address();
		emit_log(handle, addr, contract, selector, enabled, min_value)?;

		Ok(PrecompileOutput {
			exit_status: ExitSucceed::Returned,
			output: vec![],
		})
	}

	/// removeRule(address contract_, bytes4 selector)
	fn remove_rule(handle: &mut impl PrecompileHandle, data: &[u8]) -> PrecompileResult {
		if handle.is_static() {
			return Err(PrecompileFailure::Error {
				exit_status: ExitError::Other("cannot call in static context".into()),
			});
		}
		handle
			.record_cost(GAS_WRITE)
			.map_err(|e| PrecompileFailure::Error { exit_status: e })?;

		if data.len() < 64 {
			return Err(revert("invalid input length"));
		}

		Self::ensure_admin(handle.context().caller)?;

		let contract = read_address(data, 0);
		let selector = read_bytes4(data, 32);

		pallet_gasless_registry::Pallet::<R>::remove_rule(
			RawOrigin::Root.into(),
			contract,
			selector,
		)
		.map_err(|_| revert("remove_rule dispatch failed"))?;

		let addr = handle.code_address();
		handle
			.log(
				addr,
				vec![
					RULE_REMOVED_TOPIC,
					addr_topic(contract),
					bytes4_topic(selector),
				],
				vec![],
			)
			.map_err(|e| PrecompileFailure::Error { exit_status: e })?;

		Ok(PrecompileOutput {
			exit_status: ExitSucceed::Returned,
			output: vec![],
		})
	}

	fn ensure_admin(caller: H160) -> Result<(), PrecompileFailure> {
		let sudo_key = pallet_sudo::Key::<R>::get().ok_or_else(|| revert("no sudo key set"))?;
		let sudo_h160: H160 = sudo_key.into();
		if caller != sudo_h160 {
			return Err(revert("caller is not admin"));
		}
		Ok(())
	}
}

// --- EVM log emission ---

fn emit_log(
	handle: &mut impl PrecompileHandle,
	precompile_addr: H160,
	contract: H160,
	selector: [u8; 4],
	enabled: bool,
	min_value: U256,
) -> Result<(), PrecompileFailure> {
	let mut data = vec![0u8; 64];
	write_bool(&mut data, 0, enabled);
	write_u256(&mut data, 32, min_value);

	handle
		.log(
			precompile_addr,
			vec![RULE_SET_TOPIC, addr_topic(contract), bytes4_topic(selector)],
			data,
		)
		.map_err(|e| PrecompileFailure::Error { exit_status: e })
}

// --- ABI helpers ---

fn read_address(data: &[u8], offset: usize) -> H160 {
	H160::from_slice(&data[offset + 12..offset + 32])
}

fn read_bytes4(data: &[u8], offset: usize) -> [u8; 4] {
	[
		data[offset],
		data[offset + 1],
		data[offset + 2],
		data[offset + 3],
	]
}

fn read_u256(data: &[u8], offset: usize) -> U256 {
	U256::from_big_endian(&data[offset..offset + 32])
}

fn read_bool(data: &[u8], offset: usize) -> bool {
	data[offset + 31] != 0
}

fn write_bool(buf: &mut [u8], offset: usize, value: bool) {
	buf[offset + 31] = u8::from(value);
}

fn write_u256(buf: &mut [u8], offset: usize, value: U256) {
	buf[offset..offset + 32].copy_from_slice(&value.to_big_endian());
}

fn addr_topic(addr: H160) -> H256 {
	let mut t = [0u8; 32];
	t[12..32].copy_from_slice(addr.as_bytes());
	H256(t)
}

fn bytes4_topic(sel: [u8; 4]) -> H256 {
	let mut t = [0u8; 32];
	t[0..4].copy_from_slice(&sel);
	H256(t)
}

fn revert(reason: &str) -> PrecompileFailure {
	let reason_bytes = reason.as_bytes();
	let padded_len = reason_bytes.len().div_ceil(32) * 32;
	let mut output = vec![0u8; 4 + 32 + 32 + padded_len];
	// Error(string) selector
	output[0..4].copy_from_slice(&[0x08, 0xc3, 0x79, 0xa0]);
	// Offset to string data
	output[4..36].copy_from_slice(&encode_u256(32));
	// String length
	output[36..68].copy_from_slice(&encode_u256(reason_bytes.len() as u64));
	// String data
	output[68..68 + reason_bytes.len()].copy_from_slice(reason_bytes);

	PrecompileFailure::Revert {
		exit_status: ExitRevert::Reverted,
		output,
	}
}

fn encode_u256(val: u64) -> [u8; 32] {
	U256::from(val).to_big_endian()
}
