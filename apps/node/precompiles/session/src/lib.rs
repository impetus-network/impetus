#![cfg_attr(not(feature = "std"), no_std)]

//! Session precompile at EVM address `0x0818` (2072).
//!
//! Exposes Solidity-friendly bindings for `pallet-session` so EVM contracts
//! (and EOAs going through them) can register or purge session keys, and read
//! the current session index, queued keys, and per-validator next keys on the
//! Impetus NPoS chain.
//!
//! Sessions are pallet-set-agnostic from the precompile's perspective: every
//! write entry dispatches the corresponding `pallet_session::Call` via
//! [`precompile_utils::substrate::RuntimeHelper`], with `handle.context().caller`
//! converted into `Runtime::AccountId` via the runtime's `From<H160>` impl and
//! used as the signed origin. This mirrors the staking precompile pattern (and
//! Moonbeam's reference design): the original EOA pays the fees and is recorded
//! as the validator account.
//!
//! `Runtime::Keys` (the `SessionKeys` opaque struct) is SCALE-encoded on the
//! Solidity side and SCALE-decoded inside the precompile. On Impetus the
//! production keys are `(babe, grandpa, im_online, authority_discovery)` — a
//! 4-key tuple of `sr25519` / `ed25519` / `sr25519` / `sr25519` public keys,
//! 128 bytes total. We pick a 512-byte upper bound on the `BoundedBytes` input
//! to give comfortable headroom for future key-bag growth without exposing an
//! unbounded codec surface.

extern crate alloc;

use alloc::vec::Vec;
use core::marker::PhantomData;

use fp_evm::PrecompileHandle;
use frame_support::dispatch::{GetDispatchInfo, PostDispatchInfo, RawOrigin};
use precompile_utils::prelude::*;
use precompile_utils::EvmResult;
use scale_codec::{Decode, Encode};
use sp_core::{ConstU32, H160};
use sp_runtime::traits::{Convert, Dispatchable};

/// Precompile address: 0x0818 (2072).
pub const PRECOMPILE_ADDRESS: u64 = 2072;

/// Codec bound for the SCALE-encoded `keys` argument of `setKeys`.
///
/// Impetus production keys are 128 bytes (4 keys × 32 bytes each). 512 bytes
/// leaves room for ~12 sr25519/ed25519 keys without exposing an unbounded
/// codec surface.
pub const MAX_KEYS_BYTES: u32 = 512;

/// Codec bound for the `proof` argument of `setKeys`.
///
/// `pallet_session::Call::set_keys` accepts an opaque `Vec<u8>` ownership
/// proof. 512 bytes is plenty for any practical signature payload.
pub const MAX_PROOF_BYTES: u32 = 512;

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

pub struct SessionPrecompile<Runtime>(PhantomData<Runtime>);

#[precompile_utils_macro::precompile]
impl<Runtime> SessionPrecompile<Runtime>
where
    Runtime: pallet_evm::Config + pallet_session::Config,
    <Runtime as frame_system::Config>::AccountId: From<H160> + Into<H160> + Clone,
    <Runtime as frame_system::Config>::RuntimeCall: Dispatchable<
            RuntimeOrigin = <Runtime as frame_system::Config>::RuntimeOrigin,
            PostInfo = PostDispatchInfo,
        > + GetDispatchInfo
        + From<pallet_session::Call<Runtime>>,
    <Runtime as frame_system::Config>::RuntimeOrigin:
        From<RawOrigin<<Runtime as frame_system::Config>::AccountId>>,
    <Runtime as pallet_session::Config>::ValidatorId: Into<H160> + Clone,
{
    // ---- write entries -------------------------------------------------

    /// Register the caller's session keys for upcoming sessions.
    ///
    /// `keys` is the SCALE-encoded `Runtime::Keys` (the chain's `SessionKeys`
    /// struct). `proof` is the opaque ownership proof carried verbatim into
    /// `pallet_session::Call::set_keys`.
    #[precompile::public("setKeys(bytes,bytes)")]
    fn set_keys(
        handle: &mut impl PrecompileHandle,
        keys: BoundedBytes<ConstU32<MAX_KEYS_BYTES>>,
        proof: BoundedBytes<ConstU32<MAX_PROOF_BYTES>>,
    ) -> EvmResult {
        delegate_guard(handle)?;
        let keys_bytes = keys.as_bytes();
        let mut cursor: &[u8] = keys_bytes;
        let decoded = <Runtime as pallet_session::Config>::Keys::decode(&mut cursor)
            .map_err(|_| revert("InvalidKey: SCALE decode failed"))?;
        // Reject trailing garbage so a malformed payload cannot silently
        // succeed with a partially-decoded key set.
        if !cursor.is_empty() {
            return Err(revert("InvalidKey: trailing bytes after SCALE decode"));
        }
        let origin = origin_of::<Runtime>(handle);
        let call = pallet_session::Call::<Runtime>::set_keys {
            keys: decoded,
            proof: proof.as_bytes().to_vec(),
        };
        RuntimeHelper::<Runtime>::try_dispatch(handle, RawOrigin::Signed(origin).into(), call, 0)?;
        Ok(())
    }

    /// Purge the caller's session keys.
    #[precompile::public("purgeKeys()")]
    fn purge_keys(handle: &mut impl PrecompileHandle) -> EvmResult {
        delegate_guard(handle)?;
        let origin = origin_of::<Runtime>(handle);
        let call = pallet_session::Call::<Runtime>::purge_keys {};
        RuntimeHelper::<Runtime>::try_dispatch(handle, RawOrigin::Signed(origin).into(), call, 0)?;
        Ok(())
    }

    // ---- view entries --------------------------------------------------

    /// Current session index from `pallet_session::CurrentIndex`.
    #[precompile::public("currentIndex()")]
    #[precompile::view]
    fn current_index(_handle: &mut impl PrecompileHandle) -> EvmResult<u32> {
        Ok(pallet_session::CurrentIndex::<Runtime>::get())
    }

    /// SCALE-encoded `Runtime::Keys` registered for `validator`, or empty if
    /// none are stored. The caller is expected to SCALE-decode the result
    /// against the chain's `SessionKeys` struct.
    #[precompile::public("nextKeys(address)")]
    #[precompile::view]
    fn next_keys(
        _handle: &mut impl PrecompileHandle,
        validator: Address,
    ) -> EvmResult<UnboundedBytes> {
        let account: <Runtime as frame_system::Config>::AccountId = validator.0.into();
        // `NextKeys` is keyed by `ValidatorId`, which on Impetus equals
        // `AccountId`. Use the runtime-provided converter so this stays
        // correct on chains where the two diverge.
        let validator_id =
            match <Runtime as pallet_session::Config>::ValidatorIdOf::convert(account) {
                Some(v) => v,
                None => return Ok(Vec::new().into()),
            };
        let bytes = pallet_session::NextKeys::<Runtime>::get(&validator_id)
            .map(|k| k.encode())
            .unwrap_or_default();
        Ok(bytes.into())
    }

    /// `(validators[], keys[])`: parallel arrays of the queued validators and
    /// their SCALE-encoded `Runtime::Keys`.
    #[precompile::public("queuedKeys()")]
    #[precompile::view]
    fn queued_keys(
        _handle: &mut impl PrecompileHandle,
    ) -> EvmResult<(Vec<Address>, Vec<UnboundedBytes>)> {
        let queued = pallet_session::QueuedKeys::<Runtime>::get();
        let mut validators: Vec<Address> = Vec::with_capacity(queued.len());
        let mut encoded_keys: Vec<UnboundedBytes> = Vec::with_capacity(queued.len());
        for (v, k) in queued.into_iter() {
            validators.push(Address(v.into()));
            encoded_keys.push(k.encode().into());
        }
        Ok((validators, encoded_keys))
    }
}

// ---- shared helpers ---------------------------------------------------

/// Reject DELEGATECALL / CALLCODE so an intermediary contract cannot reuse
/// the EOA's signed origin without explicit `msg.value` authorization. Matches
/// the guard in `precompile-staking` and `precompile-batch`.
fn delegate_guard(handle: &impl PrecompileHandle) -> EvmResult<()> {
    if handle.code_address() != handle.context().address {
        return Err(revert("DELEGATECALL/CALLCODE forbidden"));
    }
    Ok(())
}

fn origin_of<Runtime>(
    handle: &impl PrecompileHandle,
) -> <Runtime as frame_system::Config>::AccountId
where
    Runtime: frame_system::Config,
    <Runtime as frame_system::Config>::AccountId: From<H160>,
{
    handle.context().caller.into()
}

/// `PrecompileSet` adapter used by the mock runtime only. Production wiring
/// in `runtimes/common::FrontierPrecompilesNpos` lands in Task 9.
#[cfg(test)]
pub struct SessionPrecompileSet(PhantomData<crate::mock::Runtime>);

#[cfg(test)]
impl SessionPrecompileSet {
    pub fn new() -> Self {
        Self(PhantomData)
    }
}

#[cfg(test)]
impl Default for SessionPrecompileSet {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
impl fp_evm::PrecompileSet for SessionPrecompileSet {
    fn execute(
        &self,
        handle: &mut impl fp_evm::PrecompileHandle,
    ) -> Option<fp_evm::PrecompileResult> {
        if handle.code_address() == H160::from_low_u64_be(PRECOMPILE_ADDRESS) {
            let r: fp_evm::PrecompileResult =
                <SessionPrecompile<crate::mock::Runtime> as fp_evm::Precompile>::execute(handle);
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
