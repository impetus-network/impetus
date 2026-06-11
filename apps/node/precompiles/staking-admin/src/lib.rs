#![cfg_attr(not(feature = "std"), no_std)]

//! Staking-admin precompile at EVM address `0x0840` (2112).
//!
//! Exposes Solidity-friendly bindings for the **sudo-gated** subset of
//! `pallet-staking` so the chain's admin key can drive validator-count tuning,
//! era forcing, slash cancellation, and global staking-config updates from a
//! plain EVM transaction.
//!
//! ## Permission model
//!
//! All 11 write entries are root-only in stable2603 (`set_validator_count`,
//! `force_new_era`, `set_staking_configs`, `chill_other`, etc. all call
//! `ensure_root` or `T::AdminOrigin::ensure_origin` — and the production
//! `runtimes/impetus` config sets `AdminOrigin = EnsureRoot<AccountId>`). The
//! precompile therefore:
//!
//! 1. Runs [`delegate_guard`] so an intermediate contract cannot reuse the
//!    EOA's signed origin via DELEGATECALL / CALLCODE.
//! 2. Runs [`sudo_only`] to verify that `handle.context().caller` matches
//!    `pallet_sudo::Key`.
//! 3. Dispatches the underlying `pallet_staking::Call` with
//!    `RawOrigin::Root.into()`.
//!
//! Non-sudo callers get a stable `NotSudo` revert. The DELEGATECALL guard
//! intentionally runs *before* the sudo check so a delegatecall from the sudo
//! key still surfaces `DELEGATECALL/CALLCODE forbidden`.
//!
//! ## stable2603 API notes
//!
//! * `set_invulnerables` takes `Vec<T::AccountId>` directly (no lookup
//!   adaptor).
//! * `force_unstake` takes `(T::AccountId, u32)` directly (no lookup
//!   adaptor).
//! * `cancel_deferred_slash` takes `(EraIndex, Vec<u32>)`; `slash_indices`
//!   must be sorted ascending and unique.
//! * `set_staking_configs` takes **7** `ConfigOp` arguments, not 6 — the last
//!   is `max_staked_rewards: ConfigOp<Percent>`. The Solidity ABI surfaces
//!   the first 6 (the ones the production runtime cares about) and passes
//!   `Noop` for `max_staked_rewards`. If/when a future runtime upgrade wants
//!   to drive that, add a separate entry rather than widening this ABI.
//! * `chill_other` takes `stash: T::AccountId` (per stable2603; the
//!   controller path is collapsed). The pallet itself is permissionless, but
//!   the precompile keeps it sudo-gated for parity with the rest of this
//!   surface (Plan 3 explicitly groups it here, not in `precompile-staking`).

extern crate alloc;

use alloc::vec::Vec;
use core::marker::PhantomData;

use fp_evm::PrecompileHandle;
use frame_support::dispatch::{GetDispatchInfo, PostDispatchInfo, RawOrigin};
use pallet_staking::ConfigOp;
use precompile_utils::prelude::*;
use precompile_utils::EvmResult;
use sp_core::{H160, U256};
use sp_runtime::traits::{Dispatchable, UniqueSaturatedInto};
use sp_runtime::{Perbill, Percent};

/// Precompile address: 0x0840 (2112).
pub const PRECOMPILE_ADDRESS: u64 = 2112;

/// Codec bound for `setInvulnerables(address[])`. Generous; pallet-staking
/// enforces the chain's real cap via the validator-set bound during election.
pub const MAX_INVULNERABLES: u32 = 256;

/// Codec bound for `cancelDeferredSlash(uint32, uint32[])`. Matches the
/// pallet's own `slash_indices.len() as u32` upper bound for the weight
/// formula in stable2603.
pub const MAX_SLASH_INDICES: u32 = 512;

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

pub struct StakingAdminPrecompile<Runtime>(PhantomData<Runtime>);

#[precompile_utils_macro::precompile]
impl<Runtime> StakingAdminPrecompile<Runtime>
where
    Runtime: pallet_evm::Config + pallet_staking::Config + pallet_sudo::Config,
    <Runtime as frame_system::Config>::AccountId: From<H160> + Into<H160> + Clone + PartialEq,
    <Runtime as frame_system::Config>::RuntimeCall: Dispatchable<
            RuntimeOrigin = <Runtime as frame_system::Config>::RuntimeOrigin,
            PostInfo = PostDispatchInfo,
        > + GetDispatchInfo
        + From<pallet_staking::Call<Runtime>>,
    <Runtime as frame_system::Config>::RuntimeOrigin:
        From<RawOrigin<<Runtime as frame_system::Config>::AccountId>>,
    U256: UniqueSaturatedInto<pallet_staking::BalanceOf<Runtime>>,
    pallet_staking::BalanceOf<Runtime>: Into<U256>,
{
    // ---- validator-count tuning ---------------------------------------

    /// Root-only. Set `ValidatorCount` exactly. Reverts via the pallet if
    /// `new > T::MaxValidatorSet::get()`.
    #[precompile::public("setValidatorCount(uint32)")]
    fn set_validator_count(handle: &mut impl PrecompileHandle, new: u32) -> EvmResult {
        delegate_guard(handle)?;
        sudo_only::<Runtime>(handle)?;
        let call = pallet_staking::Call::<Runtime>::set_validator_count { new };
        RuntimeHelper::<Runtime>::try_dispatch(handle, RawOrigin::Root.into(), call, 0)?;
        Ok(())
    }

    /// Root-only. Add `additional` to the current `ValidatorCount`.
    /// Reverts with `TooManyValidators` if the new total exceeds
    /// `T::MaxValidatorSet`.
    #[precompile::public("increaseValidatorCount(uint32)")]
    fn increase_validator_count(handle: &mut impl PrecompileHandle, additional: u32) -> EvmResult {
        delegate_guard(handle)?;
        sudo_only::<Runtime>(handle)?;
        let call = pallet_staking::Call::<Runtime>::increase_validator_count { additional };
        RuntimeHelper::<Runtime>::try_dispatch(handle, RawOrigin::Root.into(), call, 0)?;
        Ok(())
    }

    /// Root-only. Scale `ValidatorCount` by `factorPercent` (0..=100).
    /// Reverts with `commission must be 0..=100 percent` for out-of-range
    /// inputs; reverts via the pallet with `TooManyValidators` on overflow.
    /// Mirrors `pallet_staking::scale_validator_count(Percent)`.
    #[precompile::public("scaleValidatorCount(uint8)")]
    fn scale_validator_count(handle: &mut impl PrecompileHandle, factor_percent: u8) -> EvmResult {
        delegate_guard(handle)?;
        sudo_only::<Runtime>(handle)?;
        if factor_percent > 100 {
            return Err(revert("scale factor must be 0..=100 percent"));
        }
        let factor = Percent::from_percent(factor_percent);
        let call = pallet_staking::Call::<Runtime>::scale_validator_count { factor };
        RuntimeHelper::<Runtime>::try_dispatch(handle, RawOrigin::Root.into(), call, 0)?;
        Ok(())
    }

    // ---- invulnerables -----------------------------------------------

    /// Root-only. Replace the `Invulnerables` list. The list is bounded by
    /// codec at `MAX_INVULNERABLES`; pallet-staking does not enforce its
    /// own length bound.
    #[precompile::public("setInvulnerables(address[])")]
    fn set_invulnerables(
        handle: &mut impl PrecompileHandle,
        invulnerables: BoundedVec<Address, GetMaxInvulnerables>,
    ) -> EvmResult {
        delegate_guard(handle)?;
        sudo_only::<Runtime>(handle)?;
        let invulnerables: Vec<Address> = invulnerables.into();
        let invulnerables: Vec<<Runtime as frame_system::Config>::AccountId> = invulnerables
            .into_iter()
            .map(|addr| addr.0.into())
            .collect();
        let call = pallet_staking::Call::<Runtime>::set_invulnerables { invulnerables };
        RuntimeHelper::<Runtime>::try_dispatch(handle, RawOrigin::Root.into(), call, 0)?;
        Ok(())
    }

    // ---- forced unstaking ---------------------------------------------

    /// Root-only. Force a stash to become completely unstaked immediately.
    /// `numSlashingSpans` matches the semantics of `withdraw_unbonded`.
    #[precompile::public("forceUnstake(address,uint32)")]
    fn force_unstake(
        handle: &mut impl PrecompileHandle,
        stash: Address,
        num_slashing_spans: u32,
    ) -> EvmResult {
        delegate_guard(handle)?;
        sudo_only::<Runtime>(handle)?;
        let stash: <Runtime as frame_system::Config>::AccountId = stash.0.into();
        let call = pallet_staking::Call::<Runtime>::force_unstake {
            stash,
            num_slashing_spans,
        };
        RuntimeHelper::<Runtime>::try_dispatch(handle, RawOrigin::Root.into(), call, 0)?;
        Ok(())
    }

    // ---- era forcing --------------------------------------------------

    /// Root-only. Force a new era at the end of the next session.
    /// Equivalent to `pallet_staking::force_new_era()` — sets
    /// `ForceEra = Forcing::ForceNew`.
    #[precompile::public("forceNewEra()")]
    fn force_new_era(handle: &mut impl PrecompileHandle) -> EvmResult {
        delegate_guard(handle)?;
        sudo_only::<Runtime>(handle)?;
        let call = pallet_staking::Call::<Runtime>::force_new_era {};
        RuntimeHelper::<Runtime>::try_dispatch(handle, RawOrigin::Root.into(), call, 0)?;
        Ok(())
    }

    /// Root-only. Avoid a new era indefinitely. Sets
    /// `ForceEra = Forcing::ForceNone`.
    #[precompile::public("forceNoEras()")]
    fn force_no_eras(handle: &mut impl PrecompileHandle) -> EvmResult {
        delegate_guard(handle)?;
        sudo_only::<Runtime>(handle)?;
        let call = pallet_staking::Call::<Runtime>::force_no_eras {};
        RuntimeHelper::<Runtime>::try_dispatch(handle, RawOrigin::Root.into(), call, 0)?;
        Ok(())
    }

    /// Root-only. Force a new era at the end of every session indefinitely.
    /// Sets `ForceEra = Forcing::ForceAlways`.
    #[precompile::public("forceNewEraAlways()")]
    fn force_new_era_always(handle: &mut impl PrecompileHandle) -> EvmResult {
        delegate_guard(handle)?;
        sudo_only::<Runtime>(handle)?;
        let call = pallet_staking::Call::<Runtime>::force_new_era_always {};
        RuntimeHelper::<Runtime>::try_dispatch(handle, RawOrigin::Root.into(), call, 0)?;
        Ok(())
    }

    // ---- slash cancellation -------------------------------------------

    /// Admin-only (production wires `AdminOrigin = EnsureRoot`).
    /// `slashIndices` must be sorted ascending and unique; the pallet
    /// reverts otherwise (`NotSortedAndUnique`).
    #[precompile::public("cancelDeferredSlash(uint32,uint32[])")]
    fn cancel_deferred_slash(
        handle: &mut impl PrecompileHandle,
        era: u32,
        slash_indices: BoundedVec<u32, GetMaxSlashIndices>,
    ) -> EvmResult {
        delegate_guard(handle)?;
        sudo_only::<Runtime>(handle)?;
        let slash_indices: Vec<u32> = slash_indices.into();
        let call = pallet_staking::Call::<Runtime>::cancel_deferred_slash { era, slash_indices };
        RuntimeHelper::<Runtime>::try_dispatch(handle, RawOrigin::Root.into(), call, 0)?;
        Ok(())
    }

    // ---- staking-configs ----------------------------------------------

    /// Root-only. Bulk-update the global staking configuration. Each numeric
    /// field is set unconditionally — there is no `Noop` / `Remove` variant
    /// exposed via this ABI; if a future runtime needs that flexibility, add
    /// a dedicated entry instead of widening this surface.
    ///
    /// Parameters:
    /// * `minNominatorBond`  — `MinNominatorBond` (balance).
    /// * `minValidatorBond`  — `MinValidatorBond` (balance).
    /// * `maxNominatorCount` — `MaxNominatorsCount` (u32). `0` is valid (no
    ///   change is impossible — every call sets it).
    /// * `maxValidatorCount` — `MaxValidatorsCount` (u32).
    /// * `chillThresholdPercent` — `ChillThreshold` as integer percent
    ///   (0..=100). Rejected with a revert if out of range.
    /// * `minCommissionPerbillParts` — `MinCommission` as raw parts-per-
    ///   billion (0..=1_000_000_000). Rejected with a revert if out of
    ///   range.
    ///
    /// stable2603 takes a 7th argument `max_staked_rewards: ConfigOp<Percent>`
    /// that this ABI surfaces as a fixed `Noop` — leaving the existing storage
    /// value untouched. If/when the chain wants to drive that, expose it via
    /// a separate entry rather than breaking ABI.
    #[precompile::public("setStakingConfigs(uint256,uint256,uint32,uint32,uint8,uint32)")]
    #[allow(clippy::too_many_arguments)]
    fn set_staking_configs(
        handle: &mut impl PrecompileHandle,
        min_nominator_bond: U256,
        min_validator_bond: U256,
        max_nominator_count: u32,
        max_validator_count: u32,
        chill_threshold_percent: u8,
        min_commission_perbill_parts: u32,
    ) -> EvmResult {
        delegate_guard(handle)?;
        sudo_only::<Runtime>(handle)?;
        if chill_threshold_percent > 100 {
            return Err(revert("chillThreshold must be 0..=100 percent"));
        }
        if min_commission_perbill_parts > 1_000_000_000 {
            return Err(revert("minCommission must be 0..=1_000_000_000 parts"));
        }
        let min_nominator_bond = balance_from_u256::<Runtime>(min_nominator_bond)?;
        let min_validator_bond = balance_from_u256::<Runtime>(min_validator_bond)?;
        let chill_threshold = Percent::from_percent(chill_threshold_percent);
        let min_commission = Perbill::from_parts(min_commission_perbill_parts);
        let call = pallet_staking::Call::<Runtime>::set_staking_configs {
            min_nominator_bond: ConfigOp::Set(min_nominator_bond),
            min_validator_bond: ConfigOp::Set(min_validator_bond),
            max_nominator_count: ConfigOp::Set(max_nominator_count),
            max_validator_count: ConfigOp::Set(max_validator_count),
            chill_threshold: ConfigOp::Set(chill_threshold),
            min_commission: ConfigOp::Set(min_commission),
            // Intentionally `Noop` — see fn docs.
            max_staked_rewards: ConfigOp::Noop,
        };
        RuntimeHelper::<Runtime>::try_dispatch(handle, RawOrigin::Root.into(), call, 0)?;
        Ok(())
    }

    // ---- chill_other --------------------------------------------------

    /// Sudo-gated. Force-chill another stash. The pallet itself is
    /// permissionless and requires the chain to be at the `ChillThreshold`,
    /// but we keep the precompile sudo-gated for parity with the rest of
    /// this admin surface. If a permissionless variant is needed, add it to
    /// `precompile-staking` (T2) rather than relaxing this gate.
    #[precompile::public("chillOther(address)")]
    fn chill_other(handle: &mut impl PrecompileHandle, stash: Address) -> EvmResult {
        delegate_guard(handle)?;
        sudo_only::<Runtime>(handle)?;
        let stash: <Runtime as frame_system::Config>::AccountId = stash.0.into();
        let call = pallet_staking::Call::<Runtime>::chill_other { stash };
        RuntimeHelper::<Runtime>::try_dispatch(handle, RawOrigin::Root.into(), call, 0)?;
        Ok(())
    }
}

// ---- shared helpers ---------------------------------------------------

/// Reject DELEGATECALL / CALLCODE so an intermediary contract cannot reuse the
/// EOA's signed origin without explicit `msg.value` authorisation. Matches the
/// guard in T2-T7.
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
/// than silently saturating. Matches the shared pattern in T2-T6.
fn balance_from_u256<R: pallet_staking::Config>(v: U256) -> EvmResult<pallet_staking::BalanceOf<R>>
where
    U256: UniqueSaturatedInto<pallet_staking::BalanceOf<R>>,
    pallet_staking::BalanceOf<R>: Into<U256>,
{
    let converted: pallet_staking::BalanceOf<R> = v.unique_saturated_into();
    let round_trip: U256 = converted.into();
    if round_trip != v {
        return Err(revert("balance overflow"));
    }
    Ok(converted)
}

/// Codec bound for the `address[]` argument of `setInvulnerables`.
pub struct GetMaxInvulnerables;
impl frame_support::traits::Get<u32> for GetMaxInvulnerables {
    fn get() -> u32 {
        MAX_INVULNERABLES
    }
}

/// Codec bound for the `uint32[]` argument of `cancelDeferredSlash`.
pub struct GetMaxSlashIndices;
impl frame_support::traits::Get<u32> for GetMaxSlashIndices {
    fn get() -> u32 {
        MAX_SLASH_INDICES
    }
}

/// `PrecompileSet` adapter used by the mock runtime only. Production wiring in
/// `runtimes/common::FrontierPrecompilesNpos` lands in Task 9.
#[cfg(test)]
pub struct StakingAdminPrecompileSet(PhantomData<crate::mock::Runtime>);

#[cfg(test)]
impl StakingAdminPrecompileSet {
    pub fn new() -> Self {
        Self(PhantomData)
    }
}

#[cfg(test)]
impl Default for StakingAdminPrecompileSet {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
impl fp_evm::PrecompileSet for StakingAdminPrecompileSet {
    fn execute(
        &self,
        handle: &mut impl fp_evm::PrecompileHandle,
    ) -> Option<fp_evm::PrecompileResult> {
        if handle.code_address() == H160::from_low_u64_be(PRECOMPILE_ADDRESS) {
            let r: fp_evm::PrecompileResult =
                <StakingAdminPrecompile<crate::mock::Runtime> as fp_evm::Precompile>::execute(
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
