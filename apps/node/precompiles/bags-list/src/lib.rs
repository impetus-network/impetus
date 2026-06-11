#![cfg_attr(not(feature = "std"), no_std)]

//! Bags-list precompile at EVM address `0x0838` (2104).
//!
//! Exposes Solidity-friendly bindings for `pallet-bags-list` (specifically the
//! `Instance1` variant wired as `VoterList` in the impetus runtime) so EVM
//! contracts can drive the two permissionless write entries (`rebag`,
//! `putInFrontOf`) and read the small set of views the chain exposes for
//! validator / nominator UX (list size, per-account score, per-account bag).
//!
//! ## Permission model
//!
//! `rebag` and `put_in_front_of` are permissionless in `pallet-bags-list`:
//! anyone can pay the fee to nudge the list back into balance. The precompile
//! therefore forwards the caller's signed origin to the pallet and does not
//! gate the dispatch on sudo. The pallet enforces its own state checks
//! (`NodeNotFound`, `NotHeavier`, `NotInSameBag`, `Locked`).
//!
//! ## DELEGATECALL guard
//!
//! Every write entry runs [`delegate_guard`] before dispatch, mirroring the
//! T2-T6 precompile convention: an intermediate contract cannot reuse the
//! EOA's signed origin via `DELEGATECALL` / `CALLCODE` to drive list ops the
//! EOA never authorised.
//!
//! ## stable2603 API drift
//!
//! - `pallet_bags_list::Node::score` and `bag_upper` are public **fields**,
//!   not accessor methods, in stable2603. The `score()` method is
//!   `pub(crate)` and `bag_upper()` is gated behind
//!   `feature = "runtime-benchmarks"`, so this crate reads `node.score` /
//!   `node.bag_upper` directly.
//! - The pallet is instanced (`Config<Instance1>`), so all storage / call
//!   paths thread the `Instance1` type parameter.
//! - `ListNodes::<T, I>` is a `CountedStorageMap`, so `CounterForListNodes`
//!   gives the total node count in O(1).

extern crate alloc;

use core::marker::PhantomData;

use fp_evm::PrecompileHandle;
use frame_support::dispatch::{GetDispatchInfo, PostDispatchInfo, RawOrigin};
use precompile_utils::prelude::*;
use precompile_utils::EvmResult;
use sp_core::H160;
use sp_runtime::traits::{Dispatchable, StaticLookup};

/// Precompile address: 0x0838 (2104).
pub const PRECOMPILE_ADDRESS: u64 = 2104;

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

pub struct BagsListPrecompile<Runtime>(PhantomData<Runtime>);

#[precompile_utils_macro::precompile]
impl<Runtime> BagsListPrecompile<Runtime>
where
    Runtime: pallet_evm::Config + pallet_bags_list::Config<pallet_bags_list::Instance1>,
    <Runtime as frame_system::Config>::AccountId: From<H160> + Into<H160>,
    <Runtime as frame_system::Config>::RuntimeCall: Dispatchable<
            RuntimeOrigin = <Runtime as frame_system::Config>::RuntimeOrigin,
            PostInfo = PostDispatchInfo,
        > + GetDispatchInfo
        + From<pallet_bags_list::Call<Runtime, pallet_bags_list::Instance1>>,
    <Runtime as frame_system::Config>::RuntimeOrigin:
        From<RawOrigin<<Runtime as frame_system::Config>::AccountId>>,
    <Runtime as pallet_bags_list::Config<pallet_bags_list::Instance1>>::Score: Into<u64>,
{
    // ---- write entries -------------------------------------------------

    /// Permissionless. Move the caller in front of `lighter` within the
    /// caller's bag. Reverts via the pallet when:
    ///
    /// - `lighter` is heavier than the caller (`NotHeavier`),
    /// - the two nodes are in different bags (`NotInSameBag`),
    /// - either node is missing (`NodeNotFound`),
    /// - the pallet is locked (`Locked`).
    ///
    /// The Solidity ABI `lighter` parameter is the account whose node will be
    /// pushed behind the caller's, NOT a relative offset.
    #[precompile::public("putInFrontOf(address)")]
    fn put_in_front_of(handle: &mut impl PrecompileHandle, lighter: Address) -> EvmResult {
        delegate_guard(handle)?;
        let origin: <Runtime as frame_system::Config>::AccountId = handle.context().caller.into();
        let lighter_acct: <Runtime as frame_system::Config>::AccountId = lighter.0.into();
        let lighter_lookup =
            <<Runtime as frame_system::Config>::Lookup as StaticLookup>::unlookup(lighter_acct);
        let call =
            pallet_bags_list::Call::<Runtime, pallet_bags_list::Instance1>::put_in_front_of {
                lighter: lighter_lookup,
            };
        RuntimeHelper::<Runtime>::try_dispatch(handle, RawOrigin::Signed(origin).into(), call, 0)?;
        Ok(())
    }

    /// Permissionless. Rebag `dislocated` if its score (as read from
    /// `T::ScoreProvider`) places it in a different bag than its current
    /// `bag_upper`. No-op (still returns `Ok`) if the node is already in the
    /// correct bag. Reverts when the node is unknown (`NodeNotFound`) or the
    /// pallet is locked (`Locked`).
    #[precompile::public("rebag(address)")]
    fn rebag(handle: &mut impl PrecompileHandle, dislocated: Address) -> EvmResult {
        delegate_guard(handle)?;
        let origin: <Runtime as frame_system::Config>::AccountId = handle.context().caller.into();
        let dislocated_acct: <Runtime as frame_system::Config>::AccountId = dislocated.0.into();
        let dislocated_lookup =
            <<Runtime as frame_system::Config>::Lookup as StaticLookup>::unlookup(dislocated_acct);
        let call = pallet_bags_list::Call::<Runtime, pallet_bags_list::Instance1>::rebag {
            dislocated: dislocated_lookup,
        };
        RuntimeHelper::<Runtime>::try_dispatch(handle, RawOrigin::Signed(origin).into(), call, 0)?;
        Ok(())
    }

    // ---- view entries --------------------------------------------------

    /// Total number of nodes currently in the list. Reads
    /// `CounterForListNodes::<T, Instance1>` (O(1)).
    #[precompile::public("listSize()")]
    #[precompile::view]
    fn list_size(_handle: &mut impl PrecompileHandle) -> EvmResult<u32> {
        Ok(pallet_bags_list::ListNodes::<
            Runtime,
            pallet_bags_list::Instance1,
        >::count())
    }

    /// Stored score of `who` in the list, narrowed to `u64` for the Solidity
    /// ABI. Returns `0` if `who` is not a member of the list. Note: this is
    /// the score the pallet has cached in `ListNodes::<T>` at the last
    /// insert / rebag, NOT what `T::ScoreProvider` would report right now —
    /// callers that need the live score should call `rebag(who)` first.
    #[precompile::public("score(address)")]
    #[precompile::view]
    fn score(_handle: &mut impl PrecompileHandle, who: Address) -> EvmResult<u64> {
        let acct: <Runtime as frame_system::Config>::AccountId = who.0.into();
        let stored =
            pallet_bags_list::ListNodes::<Runtime, pallet_bags_list::Instance1>::get(&acct)
                .map(|n| n.score.into())
                .unwrap_or_default();
        Ok(stored)
    }

    /// Upper bound (threshold) of the bag `who` is currently in, narrowed to
    /// `u64` for the Solidity ABI. Returns `0` if `who` is not a member of
    /// the list.
    #[precompile::public("bagOf(address)")]
    #[precompile::view]
    fn bag_of(_handle: &mut impl PrecompileHandle, who: Address) -> EvmResult<u64> {
        let acct: <Runtime as frame_system::Config>::AccountId = who.0.into();
        let bag_upper =
            pallet_bags_list::ListNodes::<Runtime, pallet_bags_list::Instance1>::get(&acct)
                .map(|n| n.bag_upper.into())
                .unwrap_or_default();
        Ok(bag_upper)
    }
}

// ---- shared helpers ---------------------------------------------------

/// Reject DELEGATECALL / CALLCODE so an intermediary contract cannot reuse the
/// EOA's signed origin without explicit `msg.value` authorisation. Matches the
/// guard in T2-T6 precompiles.
fn delegate_guard(handle: &impl PrecompileHandle) -> EvmResult<()> {
    if handle.code_address() != handle.context().address {
        return Err(revert("DELEGATECALL/CALLCODE forbidden"));
    }
    Ok(())
}

/// `PrecompileSet` adapter used by the mock runtime only. Production wiring in
/// `runtimes/common::FrontierPrecompilesNpos` lands in Task 9.
#[cfg(test)]
pub struct BagsListPrecompileSet(PhantomData<crate::mock::Runtime>);

#[cfg(test)]
impl BagsListPrecompileSet {
    pub fn new() -> Self {
        Self(PhantomData)
    }
}

#[cfg(test)]
impl Default for BagsListPrecompileSet {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
impl fp_evm::PrecompileSet for BagsListPrecompileSet {
    fn execute(
        &self,
        handle: &mut impl fp_evm::PrecompileHandle,
    ) -> Option<fp_evm::PrecompileResult> {
        if handle.code_address() == H160::from_low_u64_be(PRECOMPILE_ADDRESS) {
            let r: fp_evm::PrecompileResult =
                <BagsListPrecompile<crate::mock::Runtime> as fp_evm::Precompile>::execute(handle);
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
