#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

use core::marker::PhantomData;

use precompile_utils::prelude::*;
use sp_core::{H256, U256};

#[cfg(test)]
use sp_core::H160;

/// Precompile address: 0x0808 (2056).
pub const PRECOMPILE_ADDRESS: u64 = 2056;

/// Maximum number of sub-calls per batch. Codec-enforced.
pub const MAX_BATCH_SIZE: u32 = 256;

/// Maximum size of a single sub-call's `callData` in bytes (2 MiB). Codec-enforced.
pub const CALL_DATA_LIMIT: u32 = 2 * 1024 * 1024;

/// Fixed overhead charged once at the top of every batch dispatch.
pub const BASE_OVERHEAD: u64 = 1_000;

/// Per-sub-call decode/dispatch overhead charged after length validation.
pub const PER_SUBCALL_OVERHEAD: u64 = 1_500;

/// `keccak256("SubcallSucceeded(uint256)")`.
pub const SUBCALL_SUCCEEDED_TOPIC: [u8; 32] = [
    0xbf, 0x85, 0x54, 0x84, 0x63, 0x39, 0x29, 0xc3, 0xd6, 0x68, 0x8e, 0xb3, 0xca, 0xf8, 0xef, 0xf9,
    0x10, 0xfb, 0x4b, 0xef, 0x03, 0x0a, 0x8d, 0x7d, 0xbc, 0x93, 0x90, 0xd2, 0x67, 0x59, 0x71, 0x4d,
];

/// `keccak256("SubcallFailed(uint256)")`.
pub const SUBCALL_FAILED_TOPIC: [u8; 32] = [
    0xdb, 0xc5, 0xd0, 0x6f, 0x4f, 0x87, 0x7f, 0x95, 0x9b, 0x1f, 0xf1, 0x2d, 0x21, 0x61, 0xcd, 0xd6,
    0x93, 0xfa, 0x8e, 0x44, 0x2e, 0xe5, 0x3f, 0x17, 0x90, 0xb2, 0x80, 0x4b, 0x24, 0x88, 0x1f, 0x05,
];

/// Topic constants converted to `H256` for use with `handle.log(...)`.
pub fn subcall_succeeded_topic() -> H256 {
    H256(SUBCALL_SUCCEEDED_TOPIC)
}
pub fn subcall_failed_topic() -> H256 {
    H256(SUBCALL_FAILED_TOPIC)
}

pub mod mode;

use crate::mode::{dispatch, BatchMode, GetCallDataLimit, GetMaxBatchSize};

pub struct BatchPrecompile<Runtime>(PhantomData<Runtime>);

#[precompile_utils_macro::precompile]
impl<Runtime> BatchPrecompile<Runtime>
where
    Runtime: pallet_evm::Config,
{
    #[precompile::public("batchSome(address[],uint256[],bytes[],uint64[])")]
    fn batch_some(
        handle: &mut impl PrecompileHandle,
        to: BoundedVec<Address, GetMaxBatchSize>,
        value: BoundedVec<U256, GetMaxBatchSize>,
        call_data: BoundedVec<BoundedBytes<GetCallDataLimit>, GetMaxBatchSize>,
        gas_limit: BoundedVec<u64, GetMaxBatchSize>,
    ) -> EvmResult {
        dispatch(handle, BatchMode::Some, to, value, call_data, gas_limit)
    }

    #[precompile::public("batchSomeUntilFailure(address[],uint256[],bytes[],uint64[])")]
    fn batch_some_until_failure(
        handle: &mut impl PrecompileHandle,
        to: BoundedVec<Address, GetMaxBatchSize>,
        value: BoundedVec<U256, GetMaxBatchSize>,
        call_data: BoundedVec<BoundedBytes<GetCallDataLimit>, GetMaxBatchSize>,
        gas_limit: BoundedVec<u64, GetMaxBatchSize>,
    ) -> EvmResult {
        dispatch(
            handle,
            BatchMode::SomeUntilFailure,
            to,
            value,
            call_data,
            gas_limit,
        )
    }

    #[precompile::public("batchAll(address[],uint256[],bytes[],uint64[])")]
    fn batch_all(
        handle: &mut impl PrecompileHandle,
        to: BoundedVec<Address, GetMaxBatchSize>,
        value: BoundedVec<U256, GetMaxBatchSize>,
        call_data: BoundedVec<BoundedBytes<GetCallDataLimit>, GetMaxBatchSize>,
        gas_limit: BoundedVec<u64, GetMaxBatchSize>,
    ) -> EvmResult {
        dispatch(handle, BatchMode::All, to, value, call_data, gas_limit)
    }
}

/// `PrecompileSet` adapter used by the mock runtime only. The production
/// `runtimes/common::FrontierPrecompiles` integration is added in Task 14.
#[cfg(test)]
pub struct BatchPrecompileSet(PhantomData<crate::mock::Runtime>);

#[cfg(test)]
impl Default for BatchPrecompileSet {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
impl BatchPrecompileSet {
    pub fn new() -> Self {
        Self(PhantomData)
    }
}

#[cfg(test)]
impl fp_evm::PrecompileSet for BatchPrecompileSet {
    fn execute(
        &self,
        handle: &mut impl fp_evm::PrecompileHandle,
    ) -> Option<fp_evm::PrecompileResult> {
        if handle.code_address() == H160::from_low_u64_be(PRECOMPILE_ADDRESS) {
            let r: fp_evm::PrecompileResult =
                <BatchPrecompile<crate::mock::Runtime> as fp_evm::Precompile>::execute(handle);
            Some(r)
        } else {
            None
        }
    }
    fn is_precompile(&self, address: H160, _gas: u64) -> fp_evm::IsPrecompileResult {
        fp_evm::IsPrecompileResult::Answer {
            is_precompile: address == H160::from_low_u64_be(PRECOMPILE_ADDRESS),
            extra_cost: 0,
        }
    }
}

#[cfg(test)]
mod mock;

#[cfg(test)]
mod selector_tests {
    use precompile_utils::testing::compute_selector;

    #[test]
    fn selectors_match_canonical_signatures() {
        assert_eq!(
            compute_selector("batchSome(address[],uint256[],bytes[],uint64[])"),
            0x79df4b9c
        );
        assert_eq!(
            compute_selector("batchSomeUntilFailure(address[],uint256[],bytes[],uint64[])"),
            0xcf0491c7
        );
        assert_eq!(
            compute_selector("batchAll(address[],uint256[],bytes[],uint64[])"),
            0x96e292b8
        );
    }
}

#[cfg(test)]
mod batch_all_tests {
    use precompile_utils::solidity::encode_with_selector;
    use precompile_utils::testing::{PrecompileTesterExt, SubcallOutput};
    use sp_core::{H160, U256};

    use crate::{BatchPrecompileSet, PRECOMPILE_ADDRESS, SUBCALL_SUCCEEDED_TOPIC};

    fn batch_addr() -> H160 {
        H160::from_low_u64_be(PRECOMPILE_ADDRESS)
    }

    #[test]
    fn batch_all_revert_in_middle_bubbles_outer_revert_and_emits_no_events() {
        crate::mock::new_test_ext().execute_with(|| {
            let caller = H160::from_low_u64_be(0xAA);
            let t1 = H160::from_low_u64_be(0x01);
            let t2 = H160::from_low_u64_be(0x02);
            let t3 = H160::from_low_u64_be(0x03);

            let selector: u32 = 0x96e292b8;
            let to: alloc::vec::Vec<precompile_utils::prelude::Address> = alloc::vec![
                precompile_utils::prelude::Address(t1),
                precompile_utils::prelude::Address(t2),
                precompile_utils::prelude::Address(t3),
            ];
            let value: alloc::vec::Vec<U256> = alloc::vec![U256::zero(); 3];
            let call_data: alloc::vec::Vec<precompile_utils::prelude::UnboundedBytes> = alloc::vec![
                alloc::vec![].into(),
                alloc::vec![].into(),
                alloc::vec![].into(),
            ];
            let gas_limit: alloc::vec::Vec<u64> = alloc::vec![100_000u64; 3];
            let data = encode_with_selector(selector, (to, value, call_data, gas_limit));

            let precompile_set = BatchPrecompileSet::new();

            // t2 reverts; t1 succeeds (emits SubcallSucceeded(0)); t3 never reached.
            // The tester does NOT rollback logs on revert, so SubcallSucceeded(0) is
            // observed even though the outer call reverts.
            precompile_set
                .prepare_test(caller, batch_addr(), data)
                .with_subcall_handle(move |subcall| {
                    if subcall.address == t2 {
                        SubcallOutput::revert()
                    } else {
                        SubcallOutput::succeed()
                    }
                })
                .expect_log(fp_evm::Log {
                    address: batch_addr(),
                    topics: alloc::vec![sp_core::H256(SUBCALL_SUCCEEDED_TOPIC)],
                    data: U256::from(0u64).to_big_endian().to_vec(),
                })
                // SubcallOutput::revert() has empty output bytes. After bubbling
                // through mode.rs, PrecompileFailure::Revert { output: vec![] }.
                // decode_revert_message returns its sentinel when bytes are too
                // short to be ABI-encoded; we match on that sentinel to confirm
                // the empty revert data propagated verbatim.
                .execute_reverts(|decoded| decoded == b"decode_revert_message: error");
        });
    }

    #[test]
    fn batch_all_three_successful_subcalls_emit_three_succeeded_events() {
        crate::mock::new_test_ext().execute_with(|| {
            let caller = H160::from_low_u64_be(0xAA);
            let t1 = H160::from_low_u64_be(0x01);
            let t2 = H160::from_low_u64_be(0x02);
            let t3 = H160::from_low_u64_be(0x03);

            // Encode batchAll(address[],uint256[],bytes[],uint64[]) calldata.
            // selector = 0x96e292b8
            let selector: u32 = 0x96e292b8;

            let to: alloc::vec::Vec<precompile_utils::prelude::Address> = alloc::vec![
                precompile_utils::prelude::Address(t1),
                precompile_utils::prelude::Address(t2),
                precompile_utils::prelude::Address(t3),
            ];
            let value: alloc::vec::Vec<U256> = alloc::vec![U256::zero(); 3];
            // Empty call data per sub-call (using UnboundedBytes)
            let call_data: alloc::vec::Vec<precompile_utils::prelude::UnboundedBytes> = alloc::vec![
                alloc::vec![].into(),
                alloc::vec![].into(),
                alloc::vec![].into(),
            ];
            let gas_limit: alloc::vec::Vec<u64> = alloc::vec![100_000u64; 3];

            let data = encode_with_selector(selector, (to, value, call_data, gas_limit));

            let precompile_set = BatchPrecompileSet::new();

            fn log_succeeded(addr: H160, idx: u64) -> fp_evm::Log {
                // U256::to_big_endian() returns [u8; 32] in this version of primitive-types.
                let data = U256::from(idx).to_big_endian().to_vec();
                fp_evm::Log {
                    address: addr,
                    topics: alloc::vec![sp_core::H256(SUBCALL_SUCCEEDED_TOPIC)],
                    data,
                }
            }

            precompile_set
                .prepare_test(caller, batch_addr(), data)
                .with_subcall_handle(|_subcall| SubcallOutput::succeed())
                .expect_log(log_succeeded(batch_addr(), 0))
                .expect_log(log_succeeded(batch_addr(), 1))
                .expect_log(log_succeeded(batch_addr(), 2))
                .execute_returns(());
        });
    }
}

#[cfg(test)]
mod batch_some_tests {
    use precompile_utils::solidity::encode_with_selector;
    use precompile_utils::testing::{PrecompileTesterExt, SubcallOutput};
    use sp_core::{H160, U256};

    use crate::{
        BatchPrecompileSet, PRECOMPILE_ADDRESS, SUBCALL_FAILED_TOPIC, SUBCALL_SUCCEEDED_TOPIC,
    };

    fn batch_addr() -> H160 {
        H160::from_low_u64_be(PRECOMPILE_ADDRESS)
    }

    fn log_succeeded(addr: H160, idx: u64) -> fp_evm::Log {
        fp_evm::Log {
            address: addr,
            topics: alloc::vec![sp_core::H256(SUBCALL_SUCCEEDED_TOPIC)],
            data: U256::from(idx).to_big_endian().to_vec(),
        }
    }

    fn log_failed(addr: H160, idx: u64) -> fp_evm::Log {
        fp_evm::Log {
            address: addr,
            topics: alloc::vec![sp_core::H256(SUBCALL_FAILED_TOPIC)],
            data: U256::from(idx).to_big_endian().to_vec(),
        }
    }

    #[test]
    fn batch_some_middle_failure_continues_and_emits_mixed_events() {
        crate::mock::new_test_ext().execute_with(|| {
            let caller = H160::from_low_u64_be(0xAA);
            let t1 = H160::from_low_u64_be(0x01);
            let t2 = H160::from_low_u64_be(0x02);
            let t3 = H160::from_low_u64_be(0x03);

            // selector for batchSome(address[],uint256[],bytes[],uint64[])
            let selector: u32 = 0x79df4b9c;

            let to: alloc::vec::Vec<precompile_utils::prelude::Address> = alloc::vec![
                precompile_utils::prelude::Address(t1),
                precompile_utils::prelude::Address(t2),
                precompile_utils::prelude::Address(t3),
            ];
            let value: alloc::vec::Vec<U256> = alloc::vec![U256::zero(); 3];
            let call_data: alloc::vec::Vec<precompile_utils::prelude::UnboundedBytes> = alloc::vec![
                alloc::vec![].into(),
                alloc::vec![].into(),
                alloc::vec![].into(),
            ];
            let gas_limit: alloc::vec::Vec<u64> = alloc::vec![100_000u64; 3];

            let data = encode_with_selector(selector, (to, value, call_data, gas_limit));

            let precompile_set = BatchPrecompileSet::new();

            // t2 reverts; batchSome keeps going (best-effort mode).
            // Expected log sequence: SubcallSucceeded(0), SubcallFailed(1), SubcallSucceeded(2).
            // Outer call must SUCCEED (does NOT revert).
            precompile_set
                .prepare_test(caller, batch_addr(), data)
                .with_subcall_handle(move |subcall| {
                    if subcall.address == t2 {
                        SubcallOutput::revert()
                    } else {
                        SubcallOutput::succeed()
                    }
                })
                .expect_log(log_succeeded(batch_addr(), 0))
                .expect_log(log_failed(batch_addr(), 1))
                .expect_log(log_succeeded(batch_addr(), 2))
                .execute_returns(());
        });
    }
}

#[cfg(test)]
mod batch_some_until_failure_tests {
    use precompile_utils::solidity::encode_with_selector;
    use precompile_utils::testing::{PrecompileTesterExt, SubcallOutput};
    use sp_core::{H160, U256};

    use crate::{
        BatchPrecompileSet, PRECOMPILE_ADDRESS, SUBCALL_FAILED_TOPIC, SUBCALL_SUCCEEDED_TOPIC,
    };

    fn batch_addr() -> H160 {
        H160::from_low_u64_be(PRECOMPILE_ADDRESS)
    }

    fn log_succeeded(addr: H160, idx: u64) -> fp_evm::Log {
        fp_evm::Log {
            address: addr,
            topics: alloc::vec![sp_core::H256(SUBCALL_SUCCEEDED_TOPIC)],
            data: U256::from(idx).to_big_endian().to_vec(),
        }
    }

    fn log_failed(addr: H160, idx: u64) -> fp_evm::Log {
        fp_evm::Log {
            address: addr,
            topics: alloc::vec![sp_core::H256(SUBCALL_FAILED_TOPIC)],
            data: U256::from(idx).to_big_endian().to_vec(),
        }
    }

    /// `batchSomeUntilFailure` with t2 reverting: emits SubcallSucceeded(0),
    /// SubcallFailed(1), then STOPS — SubcallSucceeded(2) must NOT appear.
    /// The outer call must succeed (not revert).
    #[test]
    fn batch_some_until_failure_stops_after_first_failure() {
        crate::mock::new_test_ext().execute_with(|| {
            let caller = H160::from_low_u64_be(0xAA);
            let t1 = H160::from_low_u64_be(0x01);
            let t2 = H160::from_low_u64_be(0x02);
            let t3 = H160::from_low_u64_be(0x03);

            // selector for batchSomeUntilFailure(address[],uint256[],bytes[],uint64[])
            let selector: u32 = 0xcf0491c7;

            let to: alloc::vec::Vec<precompile_utils::prelude::Address> = alloc::vec![
                precompile_utils::prelude::Address(t1),
                precompile_utils::prelude::Address(t2),
                precompile_utils::prelude::Address(t3),
            ];
            let value: alloc::vec::Vec<U256> = alloc::vec![U256::zero(); 3];
            let call_data: alloc::vec::Vec<precompile_utils::prelude::UnboundedBytes> = alloc::vec![
                alloc::vec![].into(),
                alloc::vec![].into(),
                alloc::vec![].into(),
            ];
            let gas_limit: alloc::vec::Vec<u64> = alloc::vec![100_000u64; 3];

            let data = encode_with_selector(selector, (to, value, call_data, gas_limit));

            let precompile_set = BatchPrecompileSet::new();

            // t2 reverts; batchSomeUntilFailure must break after index 1.
            // t3 (index 2) must never be reached — no SubcallSucceeded(2) log.
            // The assert_eq! inside execute_returns verifies the exact log set.
            precompile_set
                .prepare_test(caller, batch_addr(), data)
                .with_subcall_handle(move |subcall| {
                    if subcall.address == t2 {
                        SubcallOutput::revert()
                    } else {
                        SubcallOutput::succeed()
                    }
                })
                .expect_log(log_succeeded(batch_addr(), 0))
                .expect_log(log_failed(batch_addr(), 1))
                // No expect_log for index 2 — the exact-match assertion in
                // execute_returns will fail if a third log is emitted.
                .execute_returns(());
        });
    }
}

#[cfg(test)]
mod self_call_tests {
    use precompile_utils::solidity::encode_with_selector;
    use precompile_utils::testing::PrecompileTesterExt;
    use sp_core::{H160, U256};

    use crate::{BatchPrecompileSet, PRECOMPILE_ADDRESS};

    fn batch_addr() -> H160 {
        H160::from_low_u64_be(PRECOMPILE_ADDRESS)
    }

    #[test]
    fn batch_all_rejects_self_call() {
        crate::mock::new_test_ext().execute_with(|| {
            let caller = H160::from_low_u64_be(0xAA);

            // batchAll([batch_addr()], [0], [[]], [100_000])
            let selector: u32 = 0x96e292b8;
            let to: alloc::vec::Vec<precompile_utils::prelude::Address> =
                alloc::vec![precompile_utils::prelude::Address(batch_addr())];
            let value: alloc::vec::Vec<U256> = alloc::vec![U256::zero()];
            let call_data: alloc::vec::Vec<precompile_utils::prelude::UnboundedBytes> =
                alloc::vec![alloc::vec![].into()];
            let gas_limit: alloc::vec::Vec<u64> = alloc::vec![100_000u64];
            let data = encode_with_selector(selector, (to, value, call_data, gas_limit));

            BatchPrecompileSet::new()
                .prepare_test(caller, batch_addr(), data)
                .execute_reverts(|out| {
                    out.windows(b"self-call forbidden".len())
                        .any(|w| w == b"self-call forbidden")
                });
        });
    }

    #[test]
    fn batch_some_rejects_self_call() {
        crate::mock::new_test_ext().execute_with(|| {
            let caller = H160::from_low_u64_be(0xAA);

            // batchSome([batch_addr()], [0], [[]], [100_000])
            let selector: u32 = 0x79df4b9c;
            let to: alloc::vec::Vec<precompile_utils::prelude::Address> =
                alloc::vec![precompile_utils::prelude::Address(batch_addr())];
            let value: alloc::vec::Vec<U256> = alloc::vec![U256::zero()];
            let call_data: alloc::vec::Vec<precompile_utils::prelude::UnboundedBytes> =
                alloc::vec![alloc::vec![].into()];
            let gas_limit: alloc::vec::Vec<u64> = alloc::vec![100_000u64];
            let data = encode_with_selector(selector, (to, value, call_data, gas_limit));

            BatchPrecompileSet::new()
                .prepare_test(caller, batch_addr(), data)
                .execute_reverts(|out| {
                    out.windows(b"self-call forbidden".len())
                        .any(|w| w == b"self-call forbidden")
                });
        });
    }
}

/// Tests verifying that the dispatch loop passes the correct native value
/// (`transfer.value`) and caller context to each sub-call.
///
/// The stable2603 mock does NOT execute real balance transfers — `MockHandle::call`
/// ignores the `Transfer` struct and simply invokes the caller-supplied subcall
/// handler.  Full balance-state assertions are therefore deferred to the E2E
/// suite (Tasks 19-20).  What we CAN verify at the unit level is the metadata
/// that `mode::dispatch` constructs and hands to `handle.call`:
///
/// * `subcall.transfer.unwrap().value` — the raw `U256` value forwarded per slot.
/// * `subcall.context.apparent_value` — the same value, surfaced through `Context`.
/// * `subcall.context.caller`         — must equal the OUTER CALLER (0xAA),
///   NOT the precompile address — matching Moonbeam's reference design.
/// * `subcall.address`                — the target for that iteration.
///
/// Interior mutability (`RefCell<Vec<_>>`) is required because
/// `with_subcall_handle` expects `Fn`, not `FnMut`.
#[cfg(test)]
mod value_forwarding_tests {
    use std::cell::RefCell;
    use std::rc::Rc;

    use precompile_utils::solidity::encode_with_selector;
    use precompile_utils::testing::{PrecompileTesterExt, SubcallOutput};
    use sp_core::{H160, U256};

    use crate::{BatchPrecompileSet, PRECOMPILE_ADDRESS, SUBCALL_SUCCEEDED_TOPIC};

    fn batch_addr() -> H160 {
        H160::from_low_u64_be(PRECOMPILE_ADDRESS)
    }

    /// Encode `batchAll([t1, t2], [3, 5], [[], []], [100_000, 100_000])` and
    /// confirm the subcall handler receives the exact values and caller the
    /// dispatch loop built.
    #[test]
    fn batch_all_forwards_value_to_each_target_via_subcall_metadata() {
        crate::mock::new_test_ext().execute_with(|| {
            let caller = H160::from_low_u64_be(0xAA);
            let t1 = H160::from_low_u64_be(0x01);
            let t2 = H160::from_low_u64_be(0x02);

            // batchAll(address[],uint256[],bytes[],uint64[])
            let selector: u32 = 0x96e292b8;

            let to: alloc::vec::Vec<precompile_utils::prelude::Address> = alloc::vec![
                precompile_utils::prelude::Address(t1),
                precompile_utils::prelude::Address(t2),
            ];
            // Non-zero values so the dispatch loop builds Some(Transfer { .. }) for both.
            let value: alloc::vec::Vec<U256> = alloc::vec![U256::from(3u64), U256::from(5u64)];
            let call_data: alloc::vec::Vec<precompile_utils::prelude::UnboundedBytes> =
                alloc::vec![alloc::vec![].into(), alloc::vec![].into(),];
            let gas_limit: alloc::vec::Vec<u64> = alloc::vec![100_000u64; 2];

            let data = encode_with_selector(selector, (to, value, call_data, gas_limit));

            // Accumulate (target, transfer_value, apparent_value, caller) per subcall.
            // We use Rc<RefCell<_>> because the closure is Fn (not FnMut).
            type Observation = (H160, U256, U256, H160);
            let observations: Rc<RefCell<alloc::vec::Vec<Observation>>> =
                Rc::new(RefCell::new(alloc::vec::Vec::new()));
            let obs_clone = Rc::clone(&observations);

            BatchPrecompileSet::new()
                .prepare_test(caller, batch_addr(), data)
                .with_subcall_handle(move |subcall| {
                    // transfer must be Some(_) because both values are non-zero.
                    let transfer_value = subcall
                        .transfer
                        .as_ref()
                        .map(|t| t.value)
                        .unwrap_or(U256::zero());

                    obs_clone.borrow_mut().push((
                        subcall.address,
                        transfer_value,
                        subcall.context.apparent_value,
                        subcall.context.caller,
                    ));

                    SubcallOutput::succeed()
                })
                .expect_log(fp_evm::Log {
                    address: batch_addr(),
                    topics: alloc::vec![sp_core::H256(SUBCALL_SUCCEEDED_TOPIC)],
                    data: U256::from(0u64).to_big_endian().to_vec(),
                })
                .expect_log(fp_evm::Log {
                    address: batch_addr(),
                    topics: alloc::vec![sp_core::H256(SUBCALL_SUCCEEDED_TOPIC)],
                    data: U256::from(1u64).to_big_endian().to_vec(),
                })
                .execute_returns(());

            let obs = observations.borrow();
            assert_eq!(obs.len(), 2, "handler must be invoked exactly twice");

            // --- index 0: target t1, value 3 ---
            let (addr0, tv0, av0, caller0) = obs[0];
            assert_eq!(addr0, t1, "index 0: wrong target");
            assert_eq!(tv0, U256::from(3u64), "index 0: wrong transfer.value");
            assert_eq!(
                av0,
                U256::from(3u64),
                "index 0: wrong context.apparent_value"
            );
            assert_eq!(
                caller0,
                caller,
                "index 0: caller must be the outer caller, not the precompile"
            );

            // --- index 1: target t2, value 5 ---
            let (addr1, tv1, av1, caller1) = obs[1];
            assert_eq!(addr1, t2, "index 1: wrong target");
            assert_eq!(tv1, U256::from(5u64), "index 1: wrong transfer.value");
            assert_eq!(
                av1,
                U256::from(5u64),
                "index 1: wrong context.apparent_value"
            );
            assert_eq!(
                caller1,
                caller,
                "index 1: caller must be the outer caller, not the precompile"
            );
        });
    }
}

#[cfg(test)]
mod topic_tests {
    use super::*;
    use sha3::{Digest, Keccak256};

    fn topic(sig: &str) -> [u8; 32] {
        let mut h = Keccak256::new();
        h.update(sig.as_bytes());
        h.finalize().into()
    }

    #[test]
    fn subcall_succeeded_topic_matches_signature() {
        assert_eq!(SUBCALL_SUCCEEDED_TOPIC, topic("SubcallSucceeded(uint256)"));
    }

    #[test]
    fn subcall_failed_topic_matches_signature() {
        assert_eq!(SUBCALL_FAILED_TOPIC, topic("SubcallFailed(uint256)"));
    }
}

/// Tests verifying gas-limit forwarding semantics:
///
/// * `gas_limit[i] == 0`  → forward all remaining gas to the sub-call.
/// * `gas_limit[i] > remaining` → cap the sub-call gas at remaining.
///
/// Both tests use `Rc<RefCell<Option<u64>>>` to capture `subcall.target_gas`
/// from inside the `Fn` closure supplied to `with_subcall_handle`.
#[cfg(test)]
mod gas_limit_tests {
    use std::cell::RefCell;
    use std::rc::Rc;

    use precompile_utils::solidity::encode_with_selector;
    use precompile_utils::testing::{PrecompileTesterExt, SubcallOutput};
    use sp_core::{H160, U256};

    use crate::{BatchPrecompileSet, PRECOMPILE_ADDRESS, SUBCALL_SUCCEEDED_TOPIC};

    fn batch_addr() -> H160 {
        H160::from_low_u64_be(PRECOMPILE_ADDRESS)
    }

    /// Encode `batchAll([target], [0], [[]], [gas_limit_val])` calldata.
    fn encode_batch_all_single(target: H160, gas_limit_val: u64) -> alloc::vec::Vec<u8> {
        let selector: u32 = 0x96e292b8; // batchAll(address[],uint256[],bytes[],uint64[])
        let to: alloc::vec::Vec<precompile_utils::prelude::Address> =
            alloc::vec![precompile_utils::prelude::Address(target)];
        let value: alloc::vec::Vec<U256> = alloc::vec![U256::zero()];
        let call_data: alloc::vec::Vec<precompile_utils::prelude::UnboundedBytes> =
            alloc::vec![alloc::vec![].into()];
        let gas_limit: alloc::vec::Vec<u64> = alloc::vec![gas_limit_val];
        encode_with_selector(selector, (to, value, call_data, gas_limit))
    }

    /// When `gas_limit[i] == 0`, the dispatch loop forwards ALL remaining gas
    /// (`remaining_gas()` at the point of the call) to the sub-call.
    ///
    /// With a 500_000 outer budget and overhead of BASE_OVERHEAD (1_000) +
    /// PER_SUBCALL_OVERHEAD (1_500) + call_cost (700), the remaining gas
    /// forwarded should be well above 400_000. We use a tolerance window
    /// instead of an exact value to stay robust against minor call_cost changes.
    #[test]
    fn batch_all_forwards_all_gas_when_limit_is_zero() {
        crate::mock::new_test_ext().execute_with(|| {
            let caller = H160::from_low_u64_be(0xAA);
            let t1 = H160::from_low_u64_be(0x01);

            let total_gas: u64 = 500_000;
            let data = encode_batch_all_single(t1, 0 /* zero = forward all */);

            // Capture the target_gas the dispatch loop passes to the sub-call.
            let captured: Rc<RefCell<Option<u64>>> = Rc::new(RefCell::new(None));
            let captured_clone = Rc::clone(&captured);

            BatchPrecompileSet::new()
                .prepare_test(caller, batch_addr(), data)
                .with_target_gas(Some(total_gas))
                .with_subcall_handle(move |subcall| {
                    *captured_clone.borrow_mut() = subcall.target_gas;
                    SubcallOutput::succeed()
                })
                .expect_log(fp_evm::Log {
                    address: batch_addr(),
                    topics: alloc::vec![sp_core::H256(SUBCALL_SUCCEEDED_TOPIC)],
                    data: U256::from(0u64).to_big_endian().to_vec(),
                })
                .execute_returns(());

            let observed = captured
                .borrow()
                .expect("subcall handler must have been invoked");
            // remaining = 500_000 - BASE_OVERHEAD(1_000) - PER_SUBCALL(1_500) - call_cost(~700)
            // => ~496_800. Accept anything > 400_000 to tolerate call_cost variance.
            assert!(
                observed > 400_000,
                "gas_limit[i]==0 must forward remaining gas; got {observed}"
            );
            // Must NOT be forwarding the original 0 or u64::MAX.
            assert!(
                observed < total_gas,
                "forwarded gas must be less than the total budget; got {observed}"
            );
        });
    }

    /// When `gas_limit[i]` exceeds remaining gas, the dispatch loop caps the
    /// sub-call gas at `remaining_gas()` rather than forwarding the requested
    /// amount.
    ///
    /// Outer budget is set to 50_000. gas_limit[i] = 10_000_000 (far above budget).
    /// The sub-call must receive at most 50_000 (minus overhead), not 10_000_000.
    #[test]
    fn batch_all_caps_subcall_gas_to_remaining() {
        crate::mock::new_test_ext().execute_with(|| {
            let caller = H160::from_low_u64_be(0xAA);
            let t1 = H160::from_low_u64_be(0x01);

            let total_gas: u64 = 50_000;
            let requested_limit: u64 = 10_000_000; // far exceeds total_gas
            let data = encode_batch_all_single(t1, requested_limit);

            let captured: Rc<RefCell<Option<u64>>> = Rc::new(RefCell::new(None));
            let captured_clone = Rc::clone(&captured);

            BatchPrecompileSet::new()
                .prepare_test(caller, batch_addr(), data)
                .with_target_gas(Some(total_gas))
                .with_subcall_handle(move |subcall| {
                    *captured_clone.borrow_mut() = subcall.target_gas;
                    SubcallOutput::succeed()
                })
                .expect_log(fp_evm::Log {
                    address: batch_addr(),
                    topics: alloc::vec![sp_core::H256(SUBCALL_SUCCEEDED_TOPIC)],
                    data: U256::from(0u64).to_big_endian().to_vec(),
                })
                .execute_returns(());

            let observed = captured
                .borrow()
                .expect("subcall handler must have been invoked");
            // Must be capped at remaining (which is <= total_gas), never at requested_limit.
            assert!(
                observed <= total_gas,
                "gas must be capped at remaining ({total_gas}); got {observed}"
            );
            assert!(
                observed < requested_limit,
                "gas must NOT be forwarded as the requested {requested_limit}; got {observed}"
            );
        });
    }
}

/// Tests verifying that `BoundedVec<_, ConstU32<MAX_BATCH_SIZE>>` and
/// `BoundedBytes<ConstU32<CALL_DATA_LIMIT>>` reject out-of-bounds inputs at
/// ABI-decode time — BEFORE the dispatch function runs.
///
/// The ABI encoder (`encode_with_selector`) accepts plain `Vec<_>` of any
/// length.  The bound is checked by the precompile-utils `BoundedVec::read`
/// and `BoundedBytesString::read` implementations: when `array_size > S::get()`
/// they return `RevertReason::value_is_too_large("length")`.  After the macro-
/// generated `in_field("<arg_name>")` backtrace injection, the final revert
/// message contains `"Value is too large for length"`.
///
/// Neither test needs a `with_subcall_handle` because the codec revert fires
/// inside the argument-decode phase, before `dispatch()` is ever invoked.
#[cfg(test)]
mod codec_bound_tests {
    use precompile_utils::solidity::encode_with_selector;
    use precompile_utils::testing::PrecompileTesterExt;
    use sp_core::{H160, U256};

    use crate::{BatchPrecompileSet, CALL_DATA_LIMIT, MAX_BATCH_SIZE, PRECOMPILE_ADDRESS};

    fn batch_addr() -> H160 {
        H160::from_low_u64_be(PRECOMPILE_ADDRESS)
    }

    /// Sending MAX_BATCH_SIZE + 1 = 257 entries in the `to` array must revert
    /// during codec decode before dispatch runs.  No subcall handler is needed.
    #[test]
    fn rejects_batch_with_more_than_max_subcalls_via_codec() {
        crate::mock::new_test_ext().execute_with(|| {
            let caller = H160::from_low_u64_be(0xAA);
            let target = H160::from_low_u64_be(0x01);
            let n = MAX_BATCH_SIZE as usize + 1; // 257

            // batchAll(address[],uint256[],bytes[],uint64[]) selector = 0x96e292b8
            let selector: u32 = 0x96e292b8;

            let to: alloc::vec::Vec<precompile_utils::prelude::Address> =
                alloc::vec![precompile_utils::prelude::Address(target); n];
            let value: alloc::vec::Vec<U256> = alloc::vec![U256::zero(); n];
            let call_data: alloc::vec::Vec<precompile_utils::prelude::UnboundedBytes> =
                (0..n).map(|_| alloc::vec![].into()).collect();
            let gas_limit: alloc::vec::Vec<u64> = alloc::vec![100_000u64; n];

            let data = encode_with_selector(selector, (to, value, call_data, gas_limit));

            // The codec rejects before dispatch — no subcall handler required.
            BatchPrecompileSet::new()
                .prepare_test(caller, batch_addr(), data)
                // Macro wraps each arg with in_field("<name>") so the revert message is:
                // "to: Value is too large for length"
                // Accept any substring that unambiguously identifies a bound violation.
                .execute_reverts(|out| {
                    let msg = alloc::string::String::from_utf8_lossy(out).to_lowercase();
                    msg.contains("too large") || msg.contains("length") || msg.contains("exceed")
                });
        });
    }

    /// Sending a single sub-call whose `callData` exceeds CALL_DATA_LIMIT (2 MiB)
    /// must revert during codec decode before dispatch runs.
    #[test]
    fn rejects_oversized_call_data_via_codec() {
        crate::mock::new_test_ext().execute_with(|| {
            let caller = H160::from_low_u64_be(0xAA);
            let target = H160::from_low_u64_be(0x01);

            // batchAll selector
            let selector: u32 = 0x96e292b8;

            let to: alloc::vec::Vec<precompile_utils::prelude::Address> =
                alloc::vec![precompile_utils::prelude::Address(target)];
            let value: alloc::vec::Vec<U256> = alloc::vec![U256::zero()];
            // callData[0] = CALL_DATA_LIMIT + 1 bytes = 2 MiB + 1 — exceeds BoundedBytes limit.
            let oversized: alloc::vec::Vec<u8> = alloc::vec![0u8; CALL_DATA_LIMIT as usize + 1];
            let call_data: alloc::vec::Vec<precompile_utils::prelude::UnboundedBytes> =
                alloc::vec![oversized.into()];
            let gas_limit: alloc::vec::Vec<u64> = alloc::vec![100_000u64];

            let data = encode_with_selector(selector, (to, value, call_data, gas_limit));

            // The codec rejects before dispatch — no subcall handler required.
            BatchPrecompileSet::new()
                .prepare_test(caller, batch_addr(), data)
                .execute_reverts(|out| {
                    let msg = alloc::string::String::from_utf8_lossy(out).to_lowercase();
                    msg.contains("too large") || msg.contains("length") || msg.contains("exceed")
                });
        });
    }
}

/// Tests verifying post-decode, pre-dispatch validation: length mismatch
/// across the four argument arrays must revert before any sub-call is issued,
/// and an all-empty batch must succeed as a no-op.
#[cfg(test)]
mod misc_validation_tests {
    use precompile_utils::solidity::encode_with_selector;
    use precompile_utils::testing::PrecompileTesterExt;
    use sp_core::{H160, U256};

    use crate::{BatchPrecompileSet, PRECOMPILE_ADDRESS};

    fn batch_addr() -> H160 {
        H160::from_low_u64_be(PRECOMPILE_ADDRESS)
    }

    /// `batchAll([t1, t1], [0], [[], []], [100_000, 100_000])` — the `value`
    /// array has 1 element while `to`, `call_data`, and `gas_limit` each have 2.
    /// All four `BoundedVec`s decode successfully (length <= MAX_BATCH_SIZE),
    /// but `dispatch()` must reject the call with "length mismatch" before
    /// attempting any sub-call.
    #[test]
    fn rejects_length_mismatch() {
        crate::mock::new_test_ext().execute_with(|| {
            let caller = H160::from_low_u64_be(0xAA);
            let t1 = H160::from_low_u64_be(0x01);

            // batchAll(address[],uint256[],bytes[],uint64[]) = 0x96e292b8
            let selector: u32 = 0x96e292b8;

            let to: alloc::vec::Vec<precompile_utils::prelude::Address> = alloc::vec![
                precompile_utils::prelude::Address(t1),
                precompile_utils::prelude::Address(t1),
            ];
            // Intentionally 1 element while others have 2 — triggers length mismatch.
            let value: alloc::vec::Vec<U256> = alloc::vec![U256::zero()];
            let call_data: alloc::vec::Vec<precompile_utils::prelude::UnboundedBytes> =
                alloc::vec![alloc::vec![].into(), alloc::vec![].into(),];
            let gas_limit: alloc::vec::Vec<u64> = alloc::vec![100_000u64; 2];

            let data = encode_with_selector(selector, (to, value, call_data, gas_limit));

            BatchPrecompileSet::new()
                .prepare_test(caller, batch_addr(), data)
                .execute_reverts(|out| {
                    let msg = alloc::string::String::from_utf8_lossy(out).to_lowercase();
                    msg.contains("length mismatch")
                });
        });
    }

    /// `batchAll([], [], [], [])` — all four arrays empty.
    /// The dispatch loop iterates `for i in 0..0`, which is a no-op.
    /// Length validation passes (0 == 0 == 0 == 0), so the call must succeed
    /// and emit no logs at all.
    #[test]
    fn empty_batch_succeeds_with_no_events() {
        crate::mock::new_test_ext().execute_with(|| {
            let caller = H160::from_low_u64_be(0xAA);

            // batchAll(address[],uint256[],bytes[],uint64[]) = 0x96e292b8
            let selector: u32 = 0x96e292b8;

            let to: alloc::vec::Vec<precompile_utils::prelude::Address> = alloc::vec![];
            let value: alloc::vec::Vec<U256> = alloc::vec![];
            let call_data: alloc::vec::Vec<precompile_utils::prelude::UnboundedBytes> =
                alloc::vec![];
            let gas_limit: alloc::vec::Vec<u64> = alloc::vec![];

            let data = encode_with_selector(selector, (to, value, call_data, gas_limit));

            BatchPrecompileSet::new()
                .prepare_test(caller, batch_addr(), data)
                .expect_no_logs()
                .execute_returns(());
        });
    }
}

/// Tests verifying that all three `BatchPrecompile` entry points are NON-payable
/// — i.e. calls with non-zero `msg.value` must revert with "Function is not payable".
///
/// The precompile never holds funds: value transfers are sourced directly from
/// the outer caller's balance via `Transfer.source = handle.context().caller`.
/// Dropping `#[precompile::payable]` enforces `msg.value == 0` at the entry point.
///
/// Uses `PrecompilesModifierTester` from `precompile_utils::testing`, which is
/// present in frontier stable2603 (baf505d).
#[cfg(test)]
mod modifier_tests {
    use precompile_utils::testing::PrecompilesModifierTester;
    use sp_core::H160;

    use crate::{BatchPrecompileSet, PRECOMPILE_ADDRESS};

    /// All three batch selectors must satisfy the default (non-payable) modifier:
    /// - batchSome              = 0x79df4b9c
    /// - batchSomeUntilFailure  = 0xcf0491c7
    /// - batchAll               = 0x96e292b8
    ///
    /// `test_default_modifier` asserts:
    ///   1. NOT view   (calling under `is_static = true` reverts with the
    ///      "non-static" sentinel, confirming it would write state).
    ///   2. NOT payable (calling with `apparent_value = 1` reverts with
    ///      "Function is not payable").
    #[test]
    fn all_three_entries_are_non_payable() {
        let caller = H160::from_low_u64_be(0xAA);
        let precompile_addr = H160::from_low_u64_be(PRECOMPILE_ADDRESS);

        PrecompilesModifierTester::new(BatchPrecompileSet::new(), caller, precompile_addr)
            .test_default_modifier(&[0x79df4b9c, 0xcf0491c7, 0x96e292b8]);
    }
}

/// Tests verifying that the outer call's `is_static` flag is propagated to
/// every sub-call dispatched by `batchAll`.
///
/// When the outer CALL is a STATICCALL (`is_static == true`), every sub-call
/// issued by the batch precompile must also carry `is_static == true`.  The
/// stable2603 mock does not execute real bytecode, so we cannot test the full
/// "state mutation reverts" path here.  Instead we capture the `is_static`
/// boolean from each `Subcall` struct and assert it matches the outer flag.
///
/// NOTE: All three `BatchPrecompile` entry points are non-payable (no
/// `#[precompile::payable]`), which causes the macro-generated modifier check to
/// revert with "Can't call non-static function in static context" before the
/// dispatch loop runs when called via the tester builder.  To test propagation
/// at the dispatch layer we bypass the entry-point macro and call `dispatch()`
/// directly using a `MockHandle` with `is_static = true`.
#[cfg(test)]
mod static_call_tests {
    use std::cell::RefCell;
    use std::rc::Rc;

    use fp_evm::{Context, ExitReason, ExitSucceed};
    use precompile_utils::prelude::*;
    use precompile_utils::testing::{MockHandle, Subcall, SubcallOutput};
    use sp_core::{H160, U256};

    use crate::mode::{dispatch, BatchMode, GetCallDataLimit, GetMaxBatchSize};
    use crate::PRECOMPILE_ADDRESS;

    fn batch_addr() -> H160 {
        H160::from_low_u64_be(PRECOMPILE_ADDRESS)
    }

    /// When the outer `MockHandle` has `is_static = true`, `dispatch()` must
    /// forward `is_static == true` to every sub-call it issues.
    ///
    /// We construct the `MockHandle` directly (bypassing the tester builder and
    /// its macro-generated modifier check), set `is_static = true`, register a
    /// subcall handler that captures each sub-call's `is_static` flag, then
    /// call `dispatch()` and assert all captured values are `true`.
    #[test]
    fn batch_all_propagates_static_flag_to_subcalls() {
        crate::mock::new_test_ext().execute_with(|| {
            let caller = H160::from_low_u64_be(0xAA);
            let t1 = H160::from_low_u64_be(0x01);
            let t2 = H160::from_low_u64_be(0x02);

            // Accumulate the `is_static` flag observed for each sub-call.
            let captured: Rc<RefCell<alloc::vec::Vec<bool>>> = Rc::new(RefCell::new(alloc::vec![]));
            let captured_clone = Rc::clone(&captured);

            // Build a MockHandle that represents a STATICCALL to the batch precompile.
            let mut handle = MockHandle::new(
                batch_addr(),
                Context {
                    address: batch_addr(),
                    caller,
                    apparent_value: U256::zero(),
                },
            );
            handle.is_static = true;
            handle.subcall_handle = Some(alloc::boxed::Box::new(move |subcall: Subcall| {
                captured_clone.borrow_mut().push(subcall.is_static);
                SubcallOutput {
                    reason: ExitReason::Succeed(ExitSucceed::Returned),
                    output: alloc::vec![],
                    cost: 0,
                    logs: alloc::vec![],
                }
            }));

            // Build BoundedVec arguments for `dispatch()`.
            let to: BoundedVec<Address, GetMaxBatchSize> =
                alloc::vec![Address(t1), Address(t2),].into();
            let value: BoundedVec<U256, GetMaxBatchSize> =
                alloc::vec![U256::zero(), U256::zero()].into();
            let call_data: BoundedVec<BoundedBytes<GetCallDataLimit>, GetMaxBatchSize> =
                alloc::vec![alloc::vec![].into(), alloc::vec![].into(),].into();
            let gas_limit: BoundedVec<u64, GetMaxBatchSize> =
                alloc::vec![100_000u64, 100_000u64].into();

            // Call dispatch() directly — skips macro-generated modifier layer.
            let result = dispatch(&mut handle, BatchMode::All, to, value, call_data, gas_limit);
            assert!(result.is_ok(), "dispatch must succeed: {result:?}");

            let obs = captured.borrow();
            assert_eq!(
                obs.len(),
                2,
                "subcall handler must be invoked exactly twice"
            );
            assert!(
                obs[0],
                "sub-call 0 must carry is_static == true when outer call is static"
            );
            assert!(
                obs[1],
                "sub-call 1 must carry is_static == true when outer call is static"
            );
        });
    }
}

/// Reject DELEGATECALL / CALLCODE: when the precompile is reached through a
/// delegatecall, `handle.context().caller` is the original outer EOA but
/// `handle.context().address` is the delegate-caller contract. Using that
/// caller as the `Transfer.source` would let arbitrary contracts move native
/// value from the EOA without explicit msg.value authorization. We require
/// `handle.code_address() == handle.context().address`.
#[cfg(test)]
mod delegatecall_guard_tests {
    use fp_evm::Context;
    use precompile_utils::prelude::*;
    use precompile_utils::testing::MockHandle;
    use sp_core::{H160, U256};

    use crate::mode::{dispatch, BatchMode, GetCallDataLimit, GetMaxBatchSize};
    use crate::PRECOMPILE_ADDRESS;

    fn batch_addr() -> H160 {
        H160::from_low_u64_be(PRECOMPILE_ADDRESS)
    }

    #[test]
    fn dispatch_rejects_delegatecall_via_address_mismatch() {
        crate::mock::new_test_ext().execute_with(|| {
            let original_caller = H160::from_low_u64_be(0xAA);
            let delegate_caller = H160::from_low_u64_be(0xBB);
            let t1 = H160::from_low_u64_be(0x01);

            // Simulate DELEGATECALL: code is the precompile's, but the
            // executing address is the delegate-caller contract.
            let mut handle = MockHandle::new(
                batch_addr(),
                Context {
                    address: delegate_caller,
                    caller: original_caller,
                    apparent_value: U256::zero(),
                },
            );

            let to: BoundedVec<Address, GetMaxBatchSize> = alloc::vec![Address(t1)].into();
            let value: BoundedVec<U256, GetMaxBatchSize> = alloc::vec![U256::zero()].into();
            let call_data: BoundedVec<BoundedBytes<GetCallDataLimit>, GetMaxBatchSize> =
                alloc::vec![alloc::vec![].into()].into();
            let gas_limit: BoundedVec<u64, GetMaxBatchSize> = alloc::vec![100_000u64].into();

            let result = dispatch(&mut handle, BatchMode::All, to, value, call_data, gas_limit);
            let err = result.expect_err("dispatch must reject delegatecall");
            match err {
                fp_evm::PrecompileFailure::Revert { output, .. } => {
                    let needle = b"DELEGATECALL/CALLCODE forbidden";
                    assert!(
                        output.windows(needle.len()).any(|w| w == needle),
                        "revert output must mention delegatecall: {output:?}"
                    );
                }
                other => panic!("expected Revert, got {other:?}"),
            }
        });
    }
}
