#![cfg_attr(not(feature = "std"), no_std)]

//! Fast-unstake precompile at EVM address `0x0828` (2088).
//!
//! Exposes Solidity-friendly bindings for `pallet-fast-unstake` so EVM
//! contracts (and EOAs going through them) can register / deregister for
//! fast-unstaking on the Impetus NPoS chain, plus a small set of read-only
//! views over the pallet's storage.
//!
//! The crate is intentionally pallet-set-agnostic: every write entry dispatches
//! the corresponding `pallet_fast_unstake::Call` via
//! [`precompile_utils::substrate::RuntimeHelper`], with `handle.context().caller`
//! converted into `Runtime::AccountId` via the runtime's `From<H160>` impl and
//! used as the signed origin. Matches the staking + session + pools precompile
//! pattern (and Moonbeam's reference design): the original EOA pays the fees
//! and is the controller against which `T::Staking::stash_by_ctrl` is resolved.
//!
//! `control` is the only sudo-gated entry: the precompile checks that the
//! caller matches `pallet_sudo::Key` before dispatching the call as
//! `RawOrigin::Root`. The other writes (`registerFastUnstake`, `deregister`)
//! forward the EOA-signed origin and let the pallet enforce its own
//! permission / state checks.

extern crate alloc;

use alloc::vec::Vec;
use core::marker::PhantomData;

use fp_evm::PrecompileHandle;
use frame_support::dispatch::{GetDispatchInfo, PostDispatchInfo, RawOrigin};
use precompile_utils::prelude::*;
use precompile_utils::EvmResult;
use sp_core::{H160, U256};
use sp_runtime::traits::Dispatchable;

/// Precompile address: 0x0828 (2088).
pub const PRECOMPILE_ADDRESS: u64 = 2088;

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

pub struct FastUnstakePrecompile<Runtime>(PhantomData<Runtime>);

#[precompile_utils_macro::precompile]
impl<Runtime> FastUnstakePrecompile<Runtime>
where
    Runtime: pallet_evm::Config + pallet_fast_unstake::Config + pallet_sudo::Config,
    <Runtime as frame_system::Config>::AccountId: From<H160> + Into<H160> + Clone,
    <Runtime as frame_system::Config>::RuntimeCall: Dispatchable<
            RuntimeOrigin = <Runtime as frame_system::Config>::RuntimeOrigin,
            PostInfo = PostDispatchInfo,
        > + GetDispatchInfo
        + From<pallet_fast_unstake::Call<Runtime>>,
    <Runtime as frame_system::Config>::RuntimeOrigin:
        From<RawOrigin<<Runtime as frame_system::Config>::AccountId>>,
    pallet_fast_unstake::types::BalanceOf<Runtime>: Into<U256>,
{
    // ---- write entries -------------------------------------------------

    /// Register the calling controller for fast-unstake. The pallet resolves
    /// the controller's stash via `T::Staking::stash_by_ctrl`, chills + fully
    /// unbonds the stash, reserves `Config::Deposit`, and enqueues the stash
    /// in `Queue::<T>`. Requires `ErasToCheckPerBlock != 0` (i.e. the pallet
    /// must be active — call `control(u32 > 0)` first via sudo).
    #[precompile::public("registerFastUnstake()")]
    fn register_fast_unstake(handle: &mut impl PrecompileHandle) -> EvmResult {
        delegate_guard(handle)?;
        let origin = origin_of::<Runtime>(handle);
        let call = pallet_fast_unstake::Call::<Runtime>::register_fast_unstake {};
        RuntimeHelper::<Runtime>::try_dispatch(handle, RawOrigin::Signed(origin).into(), call, 0)?;
        Ok(())
    }

    /// Deregister the calling controller from the fast-unstake queue and
    /// unreserve the deposit. Reverts if the staker is not in `Queue` or is
    /// already being processed in `Head`. The associated stash remains fully
    /// unbonded + chilled (the pallet does not undo the chill done by
    /// `registerFastUnstake`); typical callers follow this with a `rebond`
    /// via the staking precompile.
    #[precompile::public("deregister()")]
    fn deregister(handle: &mut impl PrecompileHandle) -> EvmResult {
        delegate_guard(handle)?;
        let origin = origin_of::<Runtime>(handle);
        let call = pallet_fast_unstake::Call::<Runtime>::deregister {};
        RuntimeHelper::<Runtime>::try_dispatch(handle, RawOrigin::Signed(origin).into(), call, 0)?;
        Ok(())
    }

    /// Sudo-gated. Set `ErasToCheckPerBlock`, the budget the pallet uses in
    /// `on_idle` to advance the unstake queue. Pass `0` to halt the pallet.
    /// Forwards to the pallet as `RawOrigin::Root` once the caller has been
    /// verified against `pallet_sudo::Key`. Pallet enforces
    /// `eras_to_check_per_block <= T::MaxErasToCheckPerBlock`.
    #[precompile::public("control(uint32)")]
    fn control(handle: &mut impl PrecompileHandle, eras_to_check_per_block: u32) -> EvmResult {
        delegate_guard(handle)?;
        sudo_only::<Runtime>(handle)?;
        let call = pallet_fast_unstake::Call::<Runtime>::control {
            eras_to_check: eras_to_check_per_block,
        };
        RuntimeHelper::<Runtime>::try_dispatch(handle, RawOrigin::Root.into(), call, 0)?;
        Ok(())
    }

    // ---- view entries --------------------------------------------------

    /// Stashes currently being processed in `Head::<T>`. Returns the empty
    /// `address[]` when no batch is in-flight.
    ///
    /// stable2603 drift: in upstream `pallet-fast-unstake`, `Head::<T>` holds
    /// a [`pallet_fast_unstake::types::UnstakeRequest`] whose
    /// `stashes: BoundedVec<(AccountId, BalanceOf<T>), BatchSize>` carries up
    /// to `Config::BatchSize` stashes — not a single stash. The view exposes
    /// that batch shape directly so callers can detect whether a specific
    /// stash is "in head" (and therefore no longer deregister-able). Use
    /// [`headDeposits`](Self::head_deposits) if you also need the reserved
    /// deposits per-stash.
    #[precompile::public("head()")]
    #[precompile::view]
    fn head(_handle: &mut impl PrecompileHandle) -> EvmResult<Vec<Address>> {
        let stashes: Vec<Address> = pallet_fast_unstake::Head::<Runtime>::get()
            .map(|h| {
                h.stashes
                    .into_iter()
                    .map(|(s, _deposit)| Address(s.into()))
                    .collect()
            })
            .unwrap_or_default();
        Ok(stashes)
    }

    /// Reserved deposit values for each stash currently in `Head::<T>`, in
    /// the same order as [`head`](Self::head). Returns the empty array when
    /// no batch is in-flight.
    #[precompile::public("headDeposits()")]
    #[precompile::view]
    fn head_deposits(_handle: &mut impl PrecompileHandle) -> EvmResult<Vec<U256>> {
        let deposits: Vec<U256> = pallet_fast_unstake::Head::<Runtime>::get()
            .map(|h| h.stashes.into_iter().map(|(_, d)| d.into()).collect())
            .unwrap_or_default();
        Ok(deposits)
    }

    /// Reserved deposit recorded in `Queue::<T>` for `stash`, or `0` if the
    /// stash is not currently queued. Equivalent to
    /// `pallet_fast_unstake::Queue::<Runtime>::get(stash).unwrap_or(0)`.
    #[precompile::public("queue(address)")]
    #[precompile::view]
    fn queue(_handle: &mut impl PrecompileHandle, stash: Address) -> EvmResult<U256> {
        let acc: <Runtime as frame_system::Config>::AccountId = stash.0.into();
        let deposit = pallet_fast_unstake::Queue::<Runtime>::get(acc).unwrap_or_default();
        Ok(deposit.into())
    }

    /// Current value of `ErasToCheckPerBlock::<T>`. Returns `0` when the
    /// pallet is halted (the default value for `ValueQuery`).
    #[precompile::public("erasToCheckPerBlock()")]
    #[precompile::view]
    fn eras_to_check_per_block(_handle: &mut impl PrecompileHandle) -> EvmResult<u32> {
        Ok(pallet_fast_unstake::ErasToCheckPerBlock::<Runtime>::get())
    }
}

// ---- shared helpers ---------------------------------------------------

/// Reject DELEGATECALL / CALLCODE so an intermediary contract cannot reuse the
/// EOA's signed origin without explicit `msg.value` authorisation. Matches the
/// guard in `precompile-batch` / `precompile-staking` / `precompile-session` /
/// `precompile-nomination-pools`.
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

/// Verify the caller matches `pallet_sudo::Key`. Surface a stable `NotSudo`
/// revert reason so EVM callers can pattern-match it.
fn sudo_only<R: pallet_sudo::Config>(handle: &impl PrecompileHandle) -> EvmResult<()>
where
    <R as frame_system::Config>::AccountId: From<H160> + PartialEq,
{
    let caller: <R as frame_system::Config>::AccountId = handle.context().caller.into();
    let sudo_key =
        pallet_sudo::Key::<R>::get().ok_or_else(|| revert("NotSudo: no sudo key set"))?;
    if caller != sudo_key {
        return Err(revert("NotSudo"));
    }
    Ok(())
}

/// `PrecompileSet` adapter used by the mock runtime only. Production wiring in
/// `runtimes/common::FrontierPrecompilesNpos` lands in Task 9.
#[cfg(test)]
pub struct FastUnstakePrecompileSet(PhantomData<crate::mock::Runtime>);

#[cfg(test)]
impl FastUnstakePrecompileSet {
    pub fn new() -> Self {
        Self(PhantomData)
    }
}

#[cfg(test)]
impl Default for FastUnstakePrecompileSet {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
impl fp_evm::PrecompileSet for FastUnstakePrecompileSet {
    fn execute(
        &self,
        handle: &mut impl fp_evm::PrecompileHandle,
    ) -> Option<fp_evm::PrecompileResult> {
        if handle.code_address() == H160::from_low_u64_be(PRECOMPILE_ADDRESS) {
            let r: fp_evm::PrecompileResult =
                <FastUnstakePrecompile<crate::mock::Runtime> as fp_evm::Precompile>::execute(
                    handle,
                );
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
