#![cfg_attr(not(feature = "std"), no_std)]

//! Staking precompile at EVM address `0x0810` (2064).
//!
//! Exposes Solidity-friendly bindings for `pallet-staking` so EVM contracts can
//! bond, validate, nominate, claim rewards, and inspect staking storage on the
//! Impetus NPoS chain.
//!
//! The crate is intentionally pallet-set-agnostic: every entry point dispatches
//! the appropriate `pallet_staking::Call` via [`precompile_utils::substrate::RuntimeHelper`],
//! using `handle.context().caller` mapped through `pallet_evm::AddressMapping`
//! as the signed origin. This matches Moonbeam's reference pattern and means
//! the original EOA pays the fees and is recorded as the staking account.

extern crate alloc;

use alloc::vec::Vec;
use core::marker::PhantomData;

use fp_evm::PrecompileHandle;
use frame_support::dispatch::{GetDispatchInfo, PostDispatchInfo, RawOrigin};
use frame_support::traits::Get;
use precompile_utils::prelude::*;
use precompile_utils::EvmResult;
use sp_core::{H160, U256};
use sp_runtime::traits::{Dispatchable, StaticLookup, UniqueSaturatedInto};
use sp_runtime::Perbill;

/// Precompile address: 0x0810 (2064).
pub const PRECOMPILE_ADDRESS: u64 = 2064;

/// Codec bound for `nominate(address[])` / `kick(address[])` input arrays.
/// Kept generous; pallet-staking's `NominationsQuota` enforces the chain's
/// real cap inside the dispatch (typically 16 nominations).
pub const MAX_TARGETS: u32 = 256;

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

/// Solidity reward-destination enum: `(uint8 kind, address account)`.
///
/// `kind`: 0=Staked, 1=Stash, 2=Controller (deprecated, mapped to Account),
/// 3=Account, 4=None.  `account` is only consulted when `kind == 2 || kind == 3`.
#[derive(Default, Debug, Clone, solidity::Codec)]
pub struct RewardDestination {
    pub kind: u8,
    pub account: Address,
}

/// Solidity validator preferences: `(uint16 commissionPercent, bool blocked)`.
///
/// `commissionPercent` is the integer commission percent in `0..=100`. It is
/// converted to a Perbill via `parts_per_billion = percent * 10_000_000`, so
/// the value `50` denotes a Perbill of `500_000_000` (= 50%). Inputs above
/// `100` are rejected at the entry point.
#[derive(Default, Debug, Clone, solidity::Codec)]
pub struct ValidatorPrefsSolidity {
    pub commission_percent: u16,
    pub blocked: bool,
}

/// Solidity unlocking-chunk row: `(uint32 era, uint256 value)`.
#[derive(Default, Debug, Clone, solidity::Codec)]
pub struct UnlockingChunkSolidity {
    pub era: u32,
    pub value: U256,
}

/// Solidity `IndividualExposure` row: `(address who, uint256 value)`.
#[derive(Default, Debug, Clone, solidity::Codec)]
pub struct IndividualExposureSolidity {
    pub who: Address,
    pub value: U256,
}

/// Solidity `ErasRewardPoints` individual row: `(address who, uint32 points)`.
#[derive(Default, Debug, Clone, solidity::Codec)]
pub struct IndividualPointsSolidity {
    pub who: Address,
    pub points: u32,
}

fn convert_payee<AccountId: From<H160>>(
    payee: RewardDestination,
) -> EvmResult<pallet_staking::RewardDestination<AccountId>> {
    use pallet_staking::RewardDestination::*;
    Ok(match payee.kind {
        0 => Staked,
        1 => Stash,
        // `Controller` is deprecated upstream; mirror behavior by routing it to
        // an explicit `Account(account)` payee. Stable2603 rejects the legacy
        // `Controller` variant in `set_payee`, so we must not surface it.
        2 => Account(payee.account.0.into()),
        3 => Account(payee.account.0.into()),
        4 => None,
        _ => return Err(revert("invalid reward destination kind")),
    })
}

fn perbill_from_percent(percent: u16) -> EvmResult<Perbill> {
    // Reject inputs outside the documented `0..=100` band so callers see an
    // explicit revert instead of a silently saturated value. The valid range
    // maps onto parts-per-billion via `percent * 10_000_000`, which always
    // fits in `u32` for `percent <= 100`.
    if percent > 100 {
        return Err(revert("commission must be 0..=100 percent"));
    }
    Ok(Perbill::from_parts((percent as u32) * 10_000_000))
}

fn percent_from_perbill(p: Perbill) -> u16 {
    let parts = p.deconstruct() / 10_000_000;
    parts.min(u16::MAX as u32) as u16
}

pub struct StakingPrecompile<Runtime>(PhantomData<Runtime>);

#[precompile_utils_macro::precompile]
impl<Runtime> StakingPrecompile<Runtime>
where
    Runtime: pallet_evm::Config + pallet_staking::Config,
    <Runtime as frame_system::Config>::AccountId: From<H160> + Into<H160> + Clone,
    <Runtime as frame_system::Config>::RuntimeCall: Dispatchable<
            RuntimeOrigin = <Runtime as frame_system::Config>::RuntimeOrigin,
            PostInfo = PostDispatchInfo,
        > + GetDispatchInfo
        + From<pallet_staking::Call<Runtime>>,
    <Runtime as frame_system::Config>::RuntimeOrigin:
        From<RawOrigin<<Runtime as frame_system::Config>::AccountId>>,
    pallet_staking::BalanceOf<Runtime>: Into<U256>,
    U256: UniqueSaturatedInto<pallet_staking::BalanceOf<Runtime>>,
{
    // ---- write entries -------------------------------------------------

    #[precompile::public("bond(uint256,(uint8,address))")]
    fn bond(
        handle: &mut impl PrecompileHandle,
        value: U256,
        payee: RewardDestination,
    ) -> EvmResult {
        delegate_guard(handle)?;
        let value = balance_from_u256::<Runtime>(value)?;
        let payee = convert_payee::<Runtime::AccountId>(payee)?;
        let origin = origin_of::<Runtime>(handle);
        let call = pallet_staking::Call::<Runtime>::bond { value, payee };
        RuntimeHelper::<Runtime>::try_dispatch(handle, RawOrigin::Signed(origin).into(), call, 0)?;
        Ok(())
    }

    #[precompile::public("bondExtra(uint256)")]
    fn bond_extra(handle: &mut impl PrecompileHandle, max_additional: U256) -> EvmResult {
        delegate_guard(handle)?;
        let max_additional = balance_from_u256::<Runtime>(max_additional)?;
        let origin = origin_of::<Runtime>(handle);
        let call = pallet_staking::Call::<Runtime>::bond_extra { max_additional };
        RuntimeHelper::<Runtime>::try_dispatch(handle, RawOrigin::Signed(origin).into(), call, 0)?;
        Ok(())
    }

    #[precompile::public("unbond(uint256)")]
    fn unbond(handle: &mut impl PrecompileHandle, value: U256) -> EvmResult {
        delegate_guard(handle)?;
        let value = balance_from_u256::<Runtime>(value)?;
        let origin = origin_of::<Runtime>(handle);
        let call = pallet_staking::Call::<Runtime>::unbond { value };
        RuntimeHelper::<Runtime>::try_dispatch(handle, RawOrigin::Signed(origin).into(), call, 0)?;
        Ok(())
    }

    #[precompile::public("withdrawUnbonded(uint32)")]
    fn withdraw_unbonded(handle: &mut impl PrecompileHandle, num_slashing_spans: u32) -> EvmResult {
        delegate_guard(handle)?;
        let origin = origin_of::<Runtime>(handle);
        let call = pallet_staking::Call::<Runtime>::withdraw_unbonded { num_slashing_spans };
        RuntimeHelper::<Runtime>::try_dispatch(handle, RawOrigin::Signed(origin).into(), call, 0)?;
        Ok(())
    }

    #[precompile::public("validate((uint16,bool))")]
    fn validate(handle: &mut impl PrecompileHandle, prefs: ValidatorPrefsSolidity) -> EvmResult {
        delegate_guard(handle)?;
        let prefs = pallet_staking::ValidatorPrefs {
            commission: perbill_from_percent(prefs.commission_percent)?,
            blocked: prefs.blocked,
        };
        let origin = origin_of::<Runtime>(handle);
        let call = pallet_staking::Call::<Runtime>::validate { prefs };
        RuntimeHelper::<Runtime>::try_dispatch(handle, RawOrigin::Signed(origin).into(), call, 0)?;
        Ok(())
    }

    #[precompile::public("nominate(address[])")]
    fn nominate(
        handle: &mut impl PrecompileHandle,
        targets: BoundedVec<Address, GetMaxTargets>,
    ) -> EvmResult {
        delegate_guard(handle)?;
        let targets: Vec<Address> = targets.into();
        let targets: Vec<_> = targets
            .into_iter()
            .map(|t| {
                let acc: Runtime::AccountId = t.0.into();
                <Runtime::Lookup as StaticLookup>::unlookup(acc)
            })
            .collect();
        let origin = origin_of::<Runtime>(handle);
        let call = pallet_staking::Call::<Runtime>::nominate { targets };
        RuntimeHelper::<Runtime>::try_dispatch(handle, RawOrigin::Signed(origin).into(), call, 0)?;
        Ok(())
    }

    #[precompile::public("chill()")]
    fn chill(handle: &mut impl PrecompileHandle) -> EvmResult {
        delegate_guard(handle)?;
        let origin = origin_of::<Runtime>(handle);
        let call = pallet_staking::Call::<Runtime>::chill {};
        RuntimeHelper::<Runtime>::try_dispatch(handle, RawOrigin::Signed(origin).into(), call, 0)?;
        Ok(())
    }

    #[precompile::public("setPayee((uint8,address))")]
    fn set_payee(handle: &mut impl PrecompileHandle, payee: RewardDestination) -> EvmResult {
        delegate_guard(handle)?;
        let payee = convert_payee::<Runtime::AccountId>(payee)?;
        let origin = origin_of::<Runtime>(handle);
        let call = pallet_staking::Call::<Runtime>::set_payee { payee };
        RuntimeHelper::<Runtime>::try_dispatch(handle, RawOrigin::Signed(origin).into(), call, 0)?;
        Ok(())
    }

    #[precompile::public("payoutStakers(address,uint32)")]
    fn payout_stakers(
        handle: &mut impl PrecompileHandle,
        validator_stash: Address,
        era: u32,
    ) -> EvmResult {
        delegate_guard(handle)?;
        let validator_stash: Runtime::AccountId = validator_stash.0.into();
        let origin = origin_of::<Runtime>(handle);
        let call = pallet_staking::Call::<Runtime>::payout_stakers {
            validator_stash,
            era,
        };
        RuntimeHelper::<Runtime>::try_dispatch(handle, RawOrigin::Signed(origin).into(), call, 0)?;
        Ok(())
    }

    #[precompile::public("payoutStakersByPage(address,uint32,uint32)")]
    fn payout_stakers_by_page(
        handle: &mut impl PrecompileHandle,
        validator_stash: Address,
        era: u32,
        page: u32,
    ) -> EvmResult {
        delegate_guard(handle)?;
        let validator_stash: Runtime::AccountId = validator_stash.0.into();
        let origin = origin_of::<Runtime>(handle);
        let call = pallet_staking::Call::<Runtime>::payout_stakers_by_page {
            validator_stash,
            era,
            page,
        };
        RuntimeHelper::<Runtime>::try_dispatch(handle, RawOrigin::Signed(origin).into(), call, 0)?;
        Ok(())
    }

    #[precompile::public("rebond(uint256)")]
    fn rebond(handle: &mut impl PrecompileHandle, value: U256) -> EvmResult {
        delegate_guard(handle)?;
        let value = balance_from_u256::<Runtime>(value)?;
        let origin = origin_of::<Runtime>(handle);
        let call = pallet_staking::Call::<Runtime>::rebond { value };
        RuntimeHelper::<Runtime>::try_dispatch(handle, RawOrigin::Signed(origin).into(), call, 0)?;
        Ok(())
    }

    #[precompile::public("kick(address[])")]
    fn kick(
        handle: &mut impl PrecompileHandle,
        who: BoundedVec<Address, GetMaxTargets>,
    ) -> EvmResult {
        delegate_guard(handle)?;
        let who: Vec<Address> = who.into();
        let who: Vec<_> = who
            .into_iter()
            .map(|t| {
                let acc: Runtime::AccountId = t.0.into();
                <Runtime::Lookup as StaticLookup>::unlookup(acc)
            })
            .collect();
        let origin = origin_of::<Runtime>(handle);
        let call = pallet_staking::Call::<Runtime>::kick { who };
        RuntimeHelper::<Runtime>::try_dispatch(handle, RawOrigin::Signed(origin).into(), call, 0)?;
        Ok(())
    }

    #[precompile::public("chillOther(address)")]
    fn chill_other(handle: &mut impl PrecompileHandle, stash: Address) -> EvmResult {
        delegate_guard(handle)?;
        let stash: Runtime::AccountId = stash.0.into();
        let origin = origin_of::<Runtime>(handle);
        let call = pallet_staking::Call::<Runtime>::chill_other { stash };
        RuntimeHelper::<Runtime>::try_dispatch(handle, RawOrigin::Signed(origin).into(), call, 0)?;
        Ok(())
    }

    #[precompile::public("forceApplyMinCommission(address)")]
    fn force_apply_min_commission(
        handle: &mut impl PrecompileHandle,
        validator_stash: Address,
    ) -> EvmResult {
        delegate_guard(handle)?;
        let validator_stash: Runtime::AccountId = validator_stash.0.into();
        let origin = origin_of::<Runtime>(handle);
        let call = pallet_staking::Call::<Runtime>::force_apply_min_commission { validator_stash };
        RuntimeHelper::<Runtime>::try_dispatch(handle, RawOrigin::Signed(origin).into(), call, 0)?;
        Ok(())
    }

    #[precompile::public("reapStash(address,uint32)")]
    fn reap_stash(
        handle: &mut impl PrecompileHandle,
        stash: Address,
        num_slashing_spans: u32,
    ) -> EvmResult {
        delegate_guard(handle)?;
        let stash: Runtime::AccountId = stash.0.into();
        let origin = origin_of::<Runtime>(handle);
        let call = pallet_staking::Call::<Runtime>::reap_stash {
            stash,
            num_slashing_spans,
        };
        RuntimeHelper::<Runtime>::try_dispatch(handle, RawOrigin::Signed(origin).into(), call, 0)?;
        Ok(())
    }

    // ---- view entries --------------------------------------------------

    #[precompile::public("currentEra()")]
    #[precompile::view]
    fn current_era(_handle: &mut impl PrecompileHandle) -> EvmResult<u32> {
        Ok(pallet_staking::CurrentEra::<Runtime>::get().unwrap_or_default())
    }

    #[precompile::public("activeEra()")]
    #[precompile::view]
    fn active_era(_handle: &mut impl PrecompileHandle) -> EvmResult<(u32, u64)> {
        match pallet_staking::ActiveEra::<Runtime>::get() {
            Some(info) => Ok((info.index, info.start.unwrap_or_default())),
            None => Ok((0, 0)),
        }
    }

    #[precompile::public("minNominatorBond()")]
    #[precompile::view]
    fn min_nominator_bond(_handle: &mut impl PrecompileHandle) -> EvmResult<U256> {
        Ok(pallet_staking::MinNominatorBond::<Runtime>::get().into())
    }

    #[precompile::public("minValidatorBond()")]
    #[precompile::view]
    fn min_validator_bond(_handle: &mut impl PrecompileHandle) -> EvmResult<U256> {
        Ok(pallet_staking::MinValidatorBond::<Runtime>::get().into())
    }

    #[precompile::public("validatorCount()")]
    #[precompile::view]
    fn validator_count(_handle: &mut impl PrecompileHandle) -> EvmResult<u32> {
        Ok(pallet_staking::ValidatorCount::<Runtime>::get())
    }

    #[precompile::public("bonded(address)")]
    #[precompile::view]
    fn bonded(_handle: &mut impl PrecompileHandle, stash: Address) -> EvmResult<Address> {
        let stash_acc: Runtime::AccountId = stash.0.into();
        match pallet_staking::Bonded::<Runtime>::get(&stash_acc) {
            Some(controller) => Ok(Address(controller.into())),
            None => Ok(Address(H160::zero())),
        }
    }

    #[precompile::public("ledger(address)")]
    #[precompile::view]
    fn ledger(
        _handle: &mut impl PrecompileHandle,
        stash: Address,
    ) -> EvmResult<(U256, U256, Vec<UnlockingChunkSolidity>)> {
        let stash_acc: Runtime::AccountId = stash.0.into();
        // stable2603: `Ledger` is keyed by stash (controllers were unified with stashes).
        match pallet_staking::Ledger::<Runtime>::get(&stash_acc) {
            Some(l) => {
                let unlocking: Vec<UnlockingChunkSolidity> = l
                    .unlocking
                    .iter()
                    .map(|c| UnlockingChunkSolidity {
                        era: c.era,
                        value: c.value.into(),
                    })
                    .collect();
                Ok((l.active.into(), l.total.into(), unlocking))
            }
            None => Ok((U256::zero(), U256::zero(), Vec::new())),
        }
    }

    #[precompile::public("validators(address)")]
    #[precompile::view]
    fn validators(_handle: &mut impl PrecompileHandle, stash: Address) -> EvmResult<(u16, bool)> {
        let stash_acc: Runtime::AccountId = stash.0.into();
        let prefs = pallet_staking::Validators::<Runtime>::get(&stash_acc);
        Ok((percent_from_perbill(prefs.commission), prefs.blocked))
    }

    #[precompile::public("nominators(address)")]
    #[precompile::view]
    fn nominators(
        _handle: &mut impl PrecompileHandle,
        stash: Address,
    ) -> EvmResult<(Vec<Address>, u32, bool)> {
        let stash_acc: Runtime::AccountId = stash.0.into();
        match pallet_staking::Nominators::<Runtime>::get(&stash_acc) {
            Some(n) => {
                let targets: Vec<Address> =
                    n.targets.into_iter().map(|t| Address(t.into())).collect();
                Ok((targets, n.submitted_in, n.suppressed))
            }
            None => Ok((Vec::new(), 0, false)),
        }
    }

    #[precompile::public("erasStakers(uint32,address)")]
    #[precompile::view]
    fn eras_stakers(
        _handle: &mut impl PrecompileHandle,
        era: u32,
        validator: Address,
    ) -> EvmResult<(U256, U256, Vec<IndividualExposureSolidity>)> {
        let validator_acc: Runtime::AccountId = validator.0.into();
        // Prefer the paged overview + paged pages (stable2603). Fall back to
        // legacy `ErasStakers` for chains that have not migrated yet.
        if let Some(overview) =
            pallet_staking::ErasStakersOverview::<Runtime>::get(era, &validator_acc)
        {
            let mut others: Vec<IndividualExposureSolidity> = Vec::new();
            for page in 0..overview.page_count {
                if let Some(p) =
                    pallet_staking::ErasStakersPaged::<Runtime>::get((era, &validator_acc, page))
                {
                    for e in p.others.into_iter() {
                        others.push(IndividualExposureSolidity {
                            who: Address(e.who.into()),
                            value: e.value.into(),
                        });
                    }
                }
            }
            return Ok((overview.total.into(), overview.own.into(), others));
        }
        let legacy = pallet_staking::ErasStakers::<Runtime>::get(era, &validator_acc);
        let others: Vec<IndividualExposureSolidity> = legacy
            .others
            .into_iter()
            .map(|e| IndividualExposureSolidity {
                who: Address(e.who.into()),
                value: e.value.into(),
            })
            .collect();
        Ok((legacy.total.into(), legacy.own.into(), others))
    }

    #[precompile::public("erasValidatorReward(uint32)")]
    #[precompile::view]
    fn eras_validator_reward(_handle: &mut impl PrecompileHandle, era: u32) -> EvmResult<U256> {
        Ok(pallet_staking::ErasValidatorReward::<Runtime>::get(era)
            .map(|b| b.into())
            .unwrap_or_else(U256::zero))
    }

    #[precompile::public("erasRewardPoints(uint32)")]
    #[precompile::view]
    fn eras_reward_points(
        _handle: &mut impl PrecompileHandle,
        era: u32,
    ) -> EvmResult<(u32, Vec<IndividualPointsSolidity>)> {
        let pts = pallet_staking::ErasRewardPoints::<Runtime>::get(era);
        let individual: Vec<IndividualPointsSolidity> = pts
            .individual
            .into_iter()
            .map(|(who, points)| IndividualPointsSolidity {
                who: Address(who.into()),
                points,
            })
            .collect();
        Ok((pts.total, individual))
    }

    #[precompile::public("minActiveStake()")]
    #[precompile::view]
    fn min_active_stake(_handle: &mut impl PrecompileHandle) -> EvmResult<U256> {
        Ok(pallet_staking::MinimumActiveStake::<Runtime>::get().into())
    }

    #[precompile::public("counterForValidators()")]
    #[precompile::view]
    fn counter_for_validators(_handle: &mut impl PrecompileHandle) -> EvmResult<u32> {
        Ok(pallet_staking::Validators::<Runtime>::count())
    }

    #[precompile::public("counterForNominators()")]
    #[precompile::view]
    fn counter_for_nominators(_handle: &mut impl PrecompileHandle) -> EvmResult<u32> {
        Ok(pallet_staking::Nominators::<Runtime>::count())
    }

    #[precompile::public("historyDepth()")]
    #[precompile::view]
    fn history_depth(_handle: &mut impl PrecompileHandle) -> EvmResult<u32> {
        Ok(<Runtime as pallet_staking::Config>::HistoryDepth::get())
    }
}

// ---- shared helpers ---------------------------------------------------

/// Reject DELEGATECALL / CALLCODE so an intermediary contract cannot reuse the
/// EOA's signed origin without explicit `msg.value` authorization. Matches the
/// guard in `precompile-batch`.
fn delegate_guard(handle: &impl PrecompileHandle) -> EvmResult<()> {
    if handle.code_address() != handle.context().address {
        return Err(revert("DELEGATECALL/CALLCODE forbidden"));
    }
    Ok(())
}

fn balance_from_u256<R: pallet_staking::Config>(v: U256) -> EvmResult<pallet_staking::BalanceOf<R>>
where
    U256: UniqueSaturatedInto<pallet_staking::BalanceOf<R>>,
    pallet_staking::BalanceOf<R>: Into<U256>,
{
    // `unique_saturated_into` clamps to the balance's range. We additionally
    // round-trip back to U256 (via the `Into<U256>` bound on `BalanceOf<R>`)
    // and compare to detect lossy conversions caused by exceeding the balance
    // type's capacity, surfacing them as an explicit revert rather than a
    // silently-saturated value.
    let converted: pallet_staking::BalanceOf<R> = v.unique_saturated_into();
    let round_trip: U256 = converted.into();
    if round_trip != v {
        return Err(revert("balance overflow"));
    }
    Ok(converted)
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

/// Codec bound for the `address[]` argument of `nominate` / `kick`.
pub struct GetMaxTargets;
impl frame_support::traits::Get<u32> for GetMaxTargets {
    fn get() -> u32 {
        MAX_TARGETS
    }
}

/// `PrecompileSet` adapter used by the mock runtime only. The production
/// `runtimes/common::FrontierPrecompilesNpos` integration lands in Task 9.
#[cfg(test)]
pub struct StakingPrecompileSet(PhantomData<crate::mock::Runtime>);

#[cfg(test)]
impl StakingPrecompileSet {
    pub fn new() -> Self {
        Self(PhantomData)
    }
}

#[cfg(test)]
impl Default for StakingPrecompileSet {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
impl fp_evm::PrecompileSet for StakingPrecompileSet {
    fn execute(
        &self,
        handle: &mut impl fp_evm::PrecompileHandle,
    ) -> Option<fp_evm::PrecompileResult> {
        if handle.code_address() == H160::from_low_u64_be(PRECOMPILE_ADDRESS) {
            let r: fp_evm::PrecompileResult =
                <StakingPrecompile<crate::mock::Runtime> as fp_evm::Precompile>::execute(handle);
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
