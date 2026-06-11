use fp_evm::{ExitReason, ExitRevert, PrecompileFailure};
use frame_support::pallet_prelude::ConstU32;
use precompile_utils::prelude::*;
use sp_core::{H160, U256};

use crate::{
    subcall_failed_topic, subcall_succeeded_topic, BASE_OVERHEAD, CALL_DATA_LIMIT, MAX_BATCH_SIZE,
    PER_SUBCALL_OVERHEAD, PRECOMPILE_ADDRESS,
};

/// Behavior on per-sub-call failure.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum BatchMode {
    /// Best-effort: emit `SubcallFailed`, keep going.
    Some,
    /// Stop at first failure but do not revert the outer call.
    SomeUntilFailure,
    /// Atomic: any sub-call failure reverts the outer call with the
    /// sub-call's revert data bubbled verbatim.
    All,
}

pub type GetMaxBatchSize = ConstU32<{ MAX_BATCH_SIZE }>;
pub type GetCallDataLimit = ConstU32<{ CALL_DATA_LIMIT }>;

/// Loop over the four input arrays, dispatch each sub-call, apply mode
/// semantics. See spec section "Execution Flow" for the contract.
pub fn dispatch(
    handle: &mut impl PrecompileHandle,
    mode: BatchMode,
    to: BoundedVec<Address, GetMaxBatchSize>,
    value: BoundedVec<U256, GetMaxBatchSize>,
    call_data: BoundedVec<BoundedBytes<GetCallDataLimit>, GetMaxBatchSize>,
    gas_limit: BoundedVec<u64, GetMaxBatchSize>,
) -> EvmResult {
    // Reject DELEGATECALL / CALLCODE. Both keep `handle.context().caller`
    // pointing at the original EOA while running the precompile's code from
    // the delegatecaller's address; using that caller as `Transfer.source`
    // would let any contract reached via delegatecall drain native value
    // from the EOA without explicit msg.value authorization. We require the
    // executing address to match the precompile's own code address.
    if handle.code_address() != handle.context().address {
        return Err(revert("DELEGATECALL/CALLCODE forbidden"));
    }

    handle.record_cost(BASE_OVERHEAD)?;

    let to: alloc::vec::Vec<Address> = to.into();
    let value: alloc::vec::Vec<U256> = value.into();
    let call_data: alloc::vec::Vec<BoundedBytes<GetCallDataLimit>> = call_data.into();
    let gas_limit: alloc::vec::Vec<u64> = gas_limit.into();

    let n = to.len();
    if value.len() != n || call_data.len() != n || gas_limit.len() != n {
        return Err(revert("length mismatch"));
    }
    handle.record_cost((n as u64).saturating_mul(PER_SUBCALL_OVERHEAD))?;

    let precompile_h160 = H160::from_low_u64_be(PRECOMPILE_ADDRESS);
    let is_static = handle.is_static();
    let outer_caller = handle.context().caller;

    for i in 0..n {
        let target: H160 = to[i].into();
        if target == precompile_h160 {
            return Err(revert("self-call forbidden"));
        }

        let remaining = handle.remaining_gas();
        let sub_gas = if gas_limit[i] == 0 {
            remaining
        } else {
            core::cmp::min(gas_limit[i], remaining)
        };

        let input: alloc::vec::Vec<u8> = call_data[i].clone().into();

        let context = fp_evm::Context {
            address: target,
            caller: outer_caller,
            apparent_value: value[i],
        };

        let transfer = if value[i].is_zero() {
            None
        } else {
            Some(fp_evm::Transfer {
                source: outer_caller,
                target,
                value: value[i],
            })
        };

        let (reason, output) =
            handle.call(target, transfer, input, Some(sub_gas), is_static, &context);

        match (mode, &reason) {
            // Fatal exits unwind the entire execution stack; never swallow
            // them regardless of mode.
            (_, ExitReason::Fatal(exit_status)) => {
                return Err(PrecompileFailure::Fatal {
                    exit_status: exit_status.clone(),
                });
            }
            (_, ExitReason::Succeed(_)) => {
                emit_event(handle, subcall_succeeded_topic(), i as u64)?;
            }
            (BatchMode::All, ExitReason::Revert(_)) => {
                return Err(PrecompileFailure::Revert {
                    exit_status: ExitRevert::Reverted,
                    output,
                });
            }
            (BatchMode::All, _) => {
                return Err(revert(alloc::format!("sub-call {} failed", i)));
            }
            (BatchMode::Some, _) => {
                emit_event(handle, subcall_failed_topic(), i as u64)?;
            }
            (BatchMode::SomeUntilFailure, _) => {
                emit_event(handle, subcall_failed_topic(), i as u64)?;
                break;
            }
        }
    }

    Ok(())
}

fn emit_event(
    handle: &mut impl PrecompileHandle,
    topic: sp_core::H256,
    index: u64,
) -> Result<(), PrecompileFailure> {
    // U256::to_big_endian() returns [u8; 32] in this version of primitive-types.
    let data = U256::from(index).to_big_endian().to_vec();
    handle
        .log(
            H160::from_low_u64_be(PRECOMPILE_ADDRESS),
            alloc::vec![topic],
            data,
        )
        .map_err(|e| PrecompileFailure::Error { exit_status: e })?;
    Ok(())
}
