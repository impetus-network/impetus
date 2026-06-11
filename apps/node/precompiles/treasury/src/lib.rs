#![cfg_attr(not(feature = "std"), no_std)]

//! Treasury precompile at EVM address `0x0830` (2096).
//!
//! Exposes Solidity-friendly bindings for `pallet-treasury` so EVM contracts
//! (and EOAs going through them) can drive the treasury spend lifecycle on the
//! Impetus NPoS chain.
//!
//! ## stable2603 API redesign
//!
//! The original Plan 3 design wrapped `proposeSpend` / `approveProposal` /
//! `rejectProposal`. Those extrinsics were removed upstream in stable2603 in
//! favor of the `spend_local` / `spend` / `payout` / `void_spend` /
//! `check_status` flow (see polkadot-sdk #5961). This crate therefore exposes
//! the new flow only:
//!
//! * `spendLocal(uint256,address)` — root-gated, native-currency, queues an
//!   approved spend in `Approvals` and a `Proposal` in `Proposals`.
//! * `payout(uint32)` — permissionless, claims an approved spend after
//!   `valid_from` and before `expire_at`. Only meaningful for the `spend`
//!   call path (the legacy `spend_local` path drains via `on_initialize` at
//!   each spend period rather than via `payout`).
//! * `voidSpend(uint32)` — root-gated, cancels a pending / failed `spend`
//!   (NOT a `spend_local` proposal — those use `remove_approval`, intentionally
//!   omitted here).
//! * `checkStatus(uint32)` — permissionless, advances the state machine of a
//!   `Spends::<T>` entry.
//! * `pot()` — current treasury balance net of ED.
//! * `spendCount()` — `SpendCount::<T>` storage view.
//! * `approvals()` — list of pending `spend_local` proposal indices.
//!
//! `spend_local` and `void_spend` both call into pallet entry points that take
//! `SpendOrigin::ensure_origin(origin)?` / `RejectOrigin::ensure_origin(origin)?`.
//! The production `runtimes/impetus` config sets `SpendOrigin =
//! NeverEnsureOrigin<Balance>` and `RejectOrigin = EnsureRoot<AccountId>`, so
//! the only way to drive these flows is via root. The precompile therefore:
//!
//! 1. Runs the [`delegate_guard`] (the same one shared with T2-T5) so an
//!    intermediate contract cannot reuse the EOA's identity.
//! 2. Runs [`sudo_only`] to verify that `handle.context().caller` matches
//!    `pallet_sudo::Key`.
//! 3. Dispatches the underlying `pallet_treasury::Call` with
//!    `RawOrigin::Root.into()`.
//!
//! Non-root callers get a stable `NotSudo` revert. The DELEGATECALL guard
//! intentionally runs *before* the sudo check so a delegatecall from the sudo
//! key still surfaces `DELEGATECALL/CALLCODE forbidden`.

extern crate alloc;

use alloc::vec::Vec;
use core::marker::PhantomData;

use fp_evm::PrecompileHandle;
use frame_support::dispatch::{GetDispatchInfo, PostDispatchInfo, RawOrigin};
use precompile_utils::prelude::*;
use precompile_utils::EvmResult;
use sp_core::{H160, U256};
use sp_runtime::traits::{Dispatchable, StaticLookup, UniqueSaturatedInto};

/// Precompile address: 0x0830 (2096).
pub const PRECOMPILE_ADDRESS: u64 = 2096;

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

pub struct TreasuryPrecompile<Runtime>(PhantomData<Runtime>);

#[precompile_utils_macro::precompile]
impl<Runtime> TreasuryPrecompile<Runtime>
where
    Runtime: pallet_evm::Config + pallet_treasury::Config + pallet_sudo::Config,
    <Runtime as frame_system::Config>::AccountId: From<H160> + Into<H160> + Clone + PartialEq,
    <Runtime as frame_system::Config>::RuntimeCall: Dispatchable<
            RuntimeOrigin = <Runtime as frame_system::Config>::RuntimeOrigin,
            PostInfo = PostDispatchInfo,
        > + GetDispatchInfo
        + From<pallet_treasury::Call<Runtime>>,
    <Runtime as frame_system::Config>::RuntimeOrigin:
        From<RawOrigin<<Runtime as frame_system::Config>::AccountId>>,
    U256: UniqueSaturatedInto<pallet_treasury::BalanceOf<Runtime>>,
    pallet_treasury::BalanceOf<Runtime>: Into<U256>,
{
    // ---- write entries -------------------------------------------------

    /// Root-only direct spend of native currency from the treasury pot to
    /// `beneficiary`. The amount is registered in `Approvals::<T>` and paid
    /// out automatically every `SpendPeriod` via `on_initialize`. Reverts
    /// with `NotSudo` for any caller other than `pallet_sudo::Key`.
    ///
    /// stable2603 note: the underlying `spend_local` extrinsic uses
    /// `T::Lookup` (frame-system's lookup) to resolve the beneficiary, NOT
    /// `T::BeneficiaryLookup`. In the impetus runtime this is
    /// `IdentityLookup<AccountId>` so the EVM `address` flows through as-is.
    #[precompile::public("spendLocal(uint256,address)")]
    fn spend_local(
        handle: &mut impl PrecompileHandle,
        amount: U256,
        beneficiary: Address,
    ) -> EvmResult {
        delegate_guard(handle)?;
        sudo_only::<Runtime>(handle)?;
        let amount = balance_from_u256::<Runtime>(amount)?;
        let beneficiary_acct: <Runtime as frame_system::Config>::AccountId = beneficiary.0.into();
        let beneficiary_lookup =
            <<Runtime as frame_system::Config>::Lookup as StaticLookup>::unlookup(beneficiary_acct);
        #[allow(deprecated)]
        let call = pallet_treasury::Call::<Runtime>::spend_local {
            amount,
            beneficiary: beneficiary_lookup,
        };
        RuntimeHelper::<Runtime>::try_dispatch(handle, RawOrigin::Root.into(), call, 0)?;
        Ok(())
    }

    /// Permissionless. Claim a previously approved `spend` (NOT a
    /// `spend_local` proposal). Reverts if `now < valid_from`
    /// (`EarlyPayout`), if the spend has expired (`SpendExpired`), if the
    /// payment was already attempted (`AlreadyAttempted`), or if the index
    /// is unknown (`InvalidIndex`).
    #[precompile::public("payout(uint32)")]
    fn payout(handle: &mut impl PrecompileHandle, index: u32) -> EvmResult {
        delegate_guard(handle)?;
        let origin: <Runtime as frame_system::Config>::AccountId = handle.context().caller.into();
        let call = pallet_treasury::Call::<Runtime>::payout { index };
        RuntimeHelper::<Runtime>::try_dispatch(handle, RawOrigin::Signed(origin).into(), call, 0)?;
        Ok(())
    }

    /// Root-only. Cancel a pending / failed `spend` (NOT `spend_local`).
    /// Reverts with `NotSudo` for any caller other than `pallet_sudo::Key`.
    /// Pallet reverts with `InvalidIndex` if the spend is not in
    /// `Spends::<T>`, or `AlreadyAttempted` if the payout already cleared.
    #[precompile::public("voidSpend(uint32)")]
    fn void_spend(handle: &mut impl PrecompileHandle, index: u32) -> EvmResult {
        delegate_guard(handle)?;
        sudo_only::<Runtime>(handle)?;
        let call = pallet_treasury::Call::<Runtime>::void_spend { index };
        RuntimeHelper::<Runtime>::try_dispatch(handle, RawOrigin::Root.into(), call, 0)?;
        Ok(())
    }

    /// Permissionless. Advance the state machine of a `Spends::<T>` entry —
    /// removes the entry if the payout has succeeded / expired and refunds
    /// the call fee in that case. Reverts with `InvalidIndex` if the spend
    /// is not in `Spends`, `NotAttempted` if `payout` has not yet been
    /// called, or `Inconclusive` if the paymaster reports the payment as
    /// `InProgress`.
    #[precompile::public("checkStatus(uint32)")]
    fn check_status(handle: &mut impl PrecompileHandle, index: u32) -> EvmResult {
        delegate_guard(handle)?;
        let origin: <Runtime as frame_system::Config>::AccountId = handle.context().caller.into();
        let call = pallet_treasury::Call::<Runtime>::check_status { index };
        RuntimeHelper::<Runtime>::try_dispatch(handle, RawOrigin::Signed(origin).into(), call, 0)?;
        Ok(())
    }

    // ---- view entries --------------------------------------------------

    /// Current balance of the treasury pot, net of the existential deposit.
    /// Equivalent to `pallet_treasury::Pallet::<Runtime>::pot()`.
    #[precompile::public("pot()")]
    #[precompile::view]
    fn pot(_handle: &mut impl PrecompileHandle) -> EvmResult<U256> {
        let pot = pallet_treasury::Pallet::<Runtime>::pot();
        Ok(pot.into())
    }

    /// Current value of `SpendCount::<T>` — the next `spend` index.
    #[precompile::public("spendCount()")]
    #[precompile::view]
    fn spend_count(_handle: &mut impl PrecompileHandle) -> EvmResult<u32> {
        Ok(pallet_treasury::SpendCount::<Runtime>::get())
    }

    /// Pending `spend_local` proposal indices waiting for the next spend
    /// period. `Approvals::<T>` is a `BoundedVec<u32, MaxApprovals>`; the
    /// bound is enforced inside the pallet — the view widens to `Vec<u32>`
    /// for the Solidity ABI.
    #[precompile::public("approvals()")]
    #[precompile::view]
    fn approvals(_handle: &mut impl PrecompileHandle) -> EvmResult<Vec<u32>> {
        #[allow(deprecated)]
        let approvals: Vec<u32> = pallet_treasury::Approvals::<Runtime>::get()
            .into_iter()
            .collect();
        Ok(approvals)
    }
}

// ---- shared helpers ---------------------------------------------------

/// Reject DELEGATECALL / CALLCODE so an intermediary contract cannot reuse the
/// EOA's signed origin without explicit `msg.value` authorisation. Matches the
/// guard in `precompile-batch` / `precompile-staking` / `precompile-session` /
/// `precompile-nomination-pools` / `precompile-fast-unstake`.
fn delegate_guard(handle: &impl PrecompileHandle) -> EvmResult<()> {
    if handle.code_address() != handle.context().address {
        return Err(revert("DELEGATECALL/CALLCODE forbidden"));
    }
    Ok(())
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

/// Saturating-into conversion from `U256` to `BalanceOf<R>` with an explicit
/// round-trip check so genuinely-oversized values surface as a revert rather
/// than silently saturating. Matches the shared pattern in T2-T5.
fn balance_from_u256<R: pallet_treasury::Config>(
    v: U256,
) -> EvmResult<pallet_treasury::BalanceOf<R>>
where
    U256: UniqueSaturatedInto<pallet_treasury::BalanceOf<R>>,
    pallet_treasury::BalanceOf<R>: Into<U256>,
{
    let converted: pallet_treasury::BalanceOf<R> = v.unique_saturated_into();
    let round_trip: U256 = converted.into();
    if round_trip != v {
        return Err(revert("balance overflow"));
    }
    Ok(converted)
}

/// `PrecompileSet` adapter used by the mock runtime only. Production wiring in
/// `runtimes/common::FrontierPrecompilesNpos` lands in Task 9.
#[cfg(test)]
pub struct TreasuryPrecompileSet(PhantomData<crate::mock::Runtime>);

#[cfg(test)]
impl TreasuryPrecompileSet {
    pub fn new() -> Self {
        Self(PhantomData)
    }
}

#[cfg(test)]
impl Default for TreasuryPrecompileSet {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
impl fp_evm::PrecompileSet for TreasuryPrecompileSet {
    fn execute(
        &self,
        handle: &mut impl fp_evm::PrecompileHandle,
    ) -> Option<fp_evm::PrecompileResult> {
        if handle.code_address() == H160::from_low_u64_be(PRECOMPILE_ADDRESS) {
            let r: fp_evm::PrecompileResult =
                <TreasuryPrecompile<crate::mock::Runtime> as fp_evm::Precompile>::execute(handle);
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
