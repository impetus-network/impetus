#![cfg_attr(not(feature = "std"), no_std)]

//! Nomination-pools precompile at EVM address `0x0820` (2080).
//!
//! Exposes Solidity-friendly bindings for `pallet-nomination-pools` so EVM
//! contracts (and EOAs going through them) can join, manage, and inspect
//! nomination pools on the Impetus NPoS chain.
//!
//! The crate is intentionally pallet-set-agnostic: every write entry dispatches
//! the corresponding `pallet_nomination_pools::Call` via
//! [`precompile_utils::substrate::RuntimeHelper`], with `handle.context().caller`
//! converted into `Runtime::AccountId` via the runtime's `From<H160>` impl and
//! used as the signed origin. Matches the staking + session precompile pattern
//! (and Moonbeam's reference design): the original EOA pays the fees and is
//! recorded as the dispatching pool member / pool admin.
//!
//! `setConfigs` is the only sudo-gated entry: the precompile checks that the
//! caller matches `pallet_sudo::Key` before dispatching the call as
//! `RawOrigin::Root`. All other entries forward the EOA-signed origin and let
//! the pallet's own permission checks (root / nominator / bouncer roles,
//! depositor checks, etc.) decide whether the call is allowed.

extern crate alloc;

use alloc::vec::Vec;
use core::marker::PhantomData;

use fp_evm::PrecompileHandle;
use frame_support::dispatch::{GetDispatchInfo, PostDispatchInfo, RawOrigin};
use precompile_utils::prelude::*;
use precompile_utils::EvmResult;
use sp_core::{ConstU32, H160, U256};
use sp_runtime::traits::{Dispatchable, StaticLookup, UniqueSaturatedInto};
use sp_runtime::{FixedPointNumber, Perbill};

/// Precompile address: 0x0820 (2080).
pub const PRECOMPILE_ADDRESS: u64 = 2080;

/// Codec bound for `nominate(uint32, address[])` input arrays. Pallet-staking's
/// `NominationsQuota` (typically 16) is the chain's real cap, enforced inside
/// the dispatch.
pub const MAX_NOMINATIONS: u32 = 256;

/// Codec bound for `setMetadata(uint32, bytes)`. Matches the upstream
/// `MaxMetadataLen` ceiling configured on impetus (`256`).
pub const MAX_METADATA_BYTES: u32 = 256;

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

// ---- Solidity-side ABI structs ---------------------------------------------

/// Solidity `BondExtra` tuple: `(uint8 kind, uint256 amount)`.
///
/// * `kind = 0` → `BondExtra::FreeBalance(amount)`
/// * `kind = 1` → `BondExtra::Rewards` (`amount` is ignored)
#[derive(Default, Debug, Clone, solidity::Codec)]
pub struct BondExtraSolidity {
    pub kind: u8,
    pub amount: U256,
}

/// Solidity role-op tuple: `(uint8 op, address account)`.
///
/// * `op = 0` → `ConfigOp::Noop` (`account` ignored)
/// * `op = 1` → `ConfigOp::Set(account)`
/// * `op = 2` → `ConfigOp::Remove` (`account` ignored)
#[derive(Default, Debug, Clone, solidity::Codec)]
pub struct RoleOp {
    pub op: u8,
    pub account: Address,
}

/// Solidity commission pair: `(uint32 commission, address payee)`. `commission`
/// is in Perbill parts (`0..=1_000_000_000`). A `commission == 0` and
/// `payee == 0x0` tuple unsets the pool's commission (matches upstream
/// `Option<(Perbill, AccountId)>::None`).
#[derive(Default, Debug, Clone, solidity::Codec)]
pub struct CommissionPair {
    pub commission: u32,
    pub payee: Address,
}

/// Solidity commission-change-rate tuple: `(uint32 maxIncrease, uint32 minDelay)`.
/// `maxIncrease` is in Perbill parts; `minDelay` is a block count.
#[derive(Default, Debug, Clone, solidity::Codec)]
pub struct CommissionChangeRateSolidity {
    pub max_increase: u32,
    pub min_delay: u32,
}

/// Solidity `PoolMember::unbonding_eras` row: `(uint32 era, uint256 points)`.
#[derive(Default, Debug, Clone, solidity::Codec)]
pub struct UnbondingEraSolidity {
    pub era: u32,
    pub points: U256,
}

pub struct NominationPoolsPrecompile<Runtime>(PhantomData<Runtime>);

#[precompile_utils_macro::precompile]
impl<Runtime> NominationPoolsPrecompile<Runtime>
where
    Runtime: pallet_evm::Config + pallet_nomination_pools::Config + pallet_sudo::Config,
    <Runtime as frame_system::Config>::AccountId: From<H160> + Into<H160> + Clone,
    <Runtime as frame_system::Config>::RuntimeCall: Dispatchable<
            RuntimeOrigin = <Runtime as frame_system::Config>::RuntimeOrigin,
            PostInfo = PostDispatchInfo,
        > + GetDispatchInfo
        + From<pallet_nomination_pools::Call<Runtime>>,
    <Runtime as frame_system::Config>::RuntimeOrigin:
        From<RawOrigin<<Runtime as frame_system::Config>::AccountId>>,
    pallet_nomination_pools::BalanceOf<Runtime>: Into<U256>,
    U256: UniqueSaturatedInto<pallet_nomination_pools::BalanceOf<Runtime>>,
    // BlockNumber for commission-change-rate construction. Restricted to `u32`
    // (the pool change-rate is a small block count, the same constraint as
    // upstream's `min_delay` semantics). Production impetus uses `u32`-equiv
    // block numbers, so this stays a pragmatic match.
    pallet_nomination_pools::BlockNumberFor<Runtime>: From<u32>,
    // Reward counter inner type (e.g. `u128` for `FixedU128`). Surfaced as
    // `U256` raw parts so EVM callers can decode it with the FixedPointNumber
    // base constant on their side.
    <<Runtime as pallet_nomination_pools::Config>::RewardCounter as FixedPointNumber>::Inner:
        Into<U256>,
{
    // ---- write entries -------------------------------------------------

    #[precompile::public("join(uint256,uint32)")]
    fn join(handle: &mut impl PrecompileHandle, amount: U256, pool_id: u32) -> EvmResult {
        delegate_guard(handle)?;
        let amount = balance_from_u256::<Runtime>(amount)?;
        let origin = origin_of::<Runtime>(handle);
        let call = pallet_nomination_pools::Call::<Runtime>::join { amount, pool_id };
        RuntimeHelper::<Runtime>::try_dispatch(handle, RawOrigin::Signed(origin).into(), call, 0)?;
        Ok(())
    }

    #[precompile::public("bondExtra((uint8,uint256))")]
    fn bond_extra(handle: &mut impl PrecompileHandle, extra: BondExtraSolidity) -> EvmResult {
        delegate_guard(handle)?;
        let extra = decode_bond_extra::<Runtime>(extra)?;
        let origin = origin_of::<Runtime>(handle);
        let call = pallet_nomination_pools::Call::<Runtime>::bond_extra { extra };
        RuntimeHelper::<Runtime>::try_dispatch(handle, RawOrigin::Signed(origin).into(), call, 0)?;
        Ok(())
    }

    #[precompile::public("claimPayout()")]
    fn claim_payout(handle: &mut impl PrecompileHandle) -> EvmResult {
        delegate_guard(handle)?;
        let origin = origin_of::<Runtime>(handle);
        let call = pallet_nomination_pools::Call::<Runtime>::claim_payout {};
        RuntimeHelper::<Runtime>::try_dispatch(handle, RawOrigin::Signed(origin).into(), call, 0)?;
        Ok(())
    }

    #[precompile::public("unbond(address,uint256)")]
    fn unbond(
        handle: &mut impl PrecompileHandle,
        member_account: Address,
        unbonding_points: U256,
    ) -> EvmResult {
        delegate_guard(handle)?;
        let member_account = lookup::<Runtime>(member_account);
        let unbonding_points = balance_from_u256::<Runtime>(unbonding_points)?;
        let origin = origin_of::<Runtime>(handle);
        let call = pallet_nomination_pools::Call::<Runtime>::unbond {
            member_account,
            unbonding_points,
        };
        RuntimeHelper::<Runtime>::try_dispatch(handle, RawOrigin::Signed(origin).into(), call, 0)?;
        Ok(())
    }

    #[precompile::public("poolWithdrawUnbonded(uint32,uint32)")]
    fn pool_withdraw_unbonded(
        handle: &mut impl PrecompileHandle,
        pool_id: u32,
        num_slashing_spans: u32,
    ) -> EvmResult {
        delegate_guard(handle)?;
        let origin = origin_of::<Runtime>(handle);
        let call = pallet_nomination_pools::Call::<Runtime>::pool_withdraw_unbonded {
            pool_id,
            num_slashing_spans,
        };
        RuntimeHelper::<Runtime>::try_dispatch(handle, RawOrigin::Signed(origin).into(), call, 0)?;
        Ok(())
    }

    #[precompile::public("withdrawUnbonded(address,uint32)")]
    fn withdraw_unbonded(
        handle: &mut impl PrecompileHandle,
        member_account: Address,
        num_slashing_spans: u32,
    ) -> EvmResult {
        delegate_guard(handle)?;
        let member_account = lookup::<Runtime>(member_account);
        let origin = origin_of::<Runtime>(handle);
        let call = pallet_nomination_pools::Call::<Runtime>::withdraw_unbonded {
            member_account,
            num_slashing_spans,
        };
        RuntimeHelper::<Runtime>::try_dispatch(handle, RawOrigin::Signed(origin).into(), call, 0)?;
        Ok(())
    }

    #[precompile::public("create(uint256,address,address,address)")]
    fn create(
        handle: &mut impl PrecompileHandle,
        amount: U256,
        root: Address,
        nominator: Address,
        bouncer: Address,
    ) -> EvmResult {
        delegate_guard(handle)?;
        let amount = balance_from_u256::<Runtime>(amount)?;
        let root = lookup::<Runtime>(root);
        let nominator = lookup::<Runtime>(nominator);
        let bouncer = lookup::<Runtime>(bouncer);
        let origin = origin_of::<Runtime>(handle);
        let call = pallet_nomination_pools::Call::<Runtime>::create {
            amount,
            root,
            nominator,
            bouncer,
        };
        RuntimeHelper::<Runtime>::try_dispatch(handle, RawOrigin::Signed(origin).into(), call, 0)?;
        Ok(())
    }

    #[precompile::public("createWithPoolId(uint256,address,address,address,uint32)")]
    fn create_with_pool_id(
        handle: &mut impl PrecompileHandle,
        amount: U256,
        root: Address,
        nominator: Address,
        bouncer: Address,
        pool_id: u32,
    ) -> EvmResult {
        delegate_guard(handle)?;
        let amount = balance_from_u256::<Runtime>(amount)?;
        let root = lookup::<Runtime>(root);
        let nominator = lookup::<Runtime>(nominator);
        let bouncer = lookup::<Runtime>(bouncer);
        let origin = origin_of::<Runtime>(handle);
        let call = pallet_nomination_pools::Call::<Runtime>::create_with_pool_id {
            amount,
            root,
            nominator,
            bouncer,
            pool_id,
        };
        RuntimeHelper::<Runtime>::try_dispatch(handle, RawOrigin::Signed(origin).into(), call, 0)?;
        Ok(())
    }

    #[precompile::public("nominate(uint32,address[])")]
    fn nominate(
        handle: &mut impl PrecompileHandle,
        pool_id: u32,
        validators: BoundedVec<Address, GetMaxNominations>,
    ) -> EvmResult {
        delegate_guard(handle)?;
        let validators: Vec<Address> = validators.into();
        // `pallet_nomination_pools::nominate` takes `Vec<AccountId>` directly
        // (no Lookup), matching the stable2603 signature.
        let validators: Vec<<Runtime as frame_system::Config>::AccountId> =
            validators.into_iter().map(|v| v.0.into()).collect();
        let origin = origin_of::<Runtime>(handle);
        let call = pallet_nomination_pools::Call::<Runtime>::nominate {
            pool_id,
            validators,
        };
        RuntimeHelper::<Runtime>::try_dispatch(handle, RawOrigin::Signed(origin).into(), call, 0)?;
        Ok(())
    }

    #[precompile::public("setState(uint32,uint8)")]
    fn set_state(handle: &mut impl PrecompileHandle, pool_id: u32, state: u8) -> EvmResult {
        delegate_guard(handle)?;
        let state = match state {
            0 => pallet_nomination_pools::PoolState::Open,
            1 => pallet_nomination_pools::PoolState::Blocked,
            2 => pallet_nomination_pools::PoolState::Destroying,
            _ => return Err(revert("invalid pool state")),
        };
        let origin = origin_of::<Runtime>(handle);
        let call = pallet_nomination_pools::Call::<Runtime>::set_state { pool_id, state };
        RuntimeHelper::<Runtime>::try_dispatch(handle, RawOrigin::Signed(origin).into(), call, 0)?;
        Ok(())
    }

    #[precompile::public("setMetadata(uint32,bytes)")]
    fn set_metadata(
        handle: &mut impl PrecompileHandle,
        pool_id: u32,
        metadata: BoundedBytes<ConstU32<MAX_METADATA_BYTES>>,
    ) -> EvmResult {
        delegate_guard(handle)?;
        let metadata = metadata.as_bytes().to_vec();
        let origin = origin_of::<Runtime>(handle);
        let call = pallet_nomination_pools::Call::<Runtime>::set_metadata { pool_id, metadata };
        RuntimeHelper::<Runtime>::try_dispatch(handle, RawOrigin::Signed(origin).into(), call, 0)?;
        Ok(())
    }

    /// Sudo-gated. Forwards to the pallet as `RawOrigin::Root` once the caller
    /// has been verified against `pallet_sudo::Key`. Each numeric argument is
    /// SCALE-encoded as `ConfigOp::Set(value)` — the precompile does not
    /// currently expose `Noop` or `Remove` operations; callers may simply pass
    /// the same value to keep a field unchanged. Surface area can be expanded
    /// later if needed.
    #[precompile::public("setConfigs(uint256,uint256,uint32,uint32,uint32,uint32)")]
    fn set_configs(
        handle: &mut impl PrecompileHandle,
        min_join_bond: U256,
        min_create_bond: U256,
        max_pools: u32,
        max_members: u32,
        max_members_per_pool: u32,
        global_max_commission: u32,
    ) -> EvmResult {
        delegate_guard(handle)?;
        sudo_only::<Runtime>(handle)?;
        use pallet_nomination_pools::ConfigOp::Set;
        let call = pallet_nomination_pools::Call::<Runtime>::set_configs {
            min_join_bond: Set(balance_from_u256::<Runtime>(min_join_bond)?),
            min_create_bond: Set(balance_from_u256::<Runtime>(min_create_bond)?),
            max_pools: Set(max_pools),
            max_members: Set(max_members),
            max_members_per_pool: Set(max_members_per_pool),
            global_max_commission: Set(Perbill::from_parts(global_max_commission)),
        };
        RuntimeHelper::<Runtime>::try_dispatch(handle, RawOrigin::Root.into(), call, 0)?;
        Ok(())
    }

    /// `roles`: a 3-tuple of (root, nominator, bouncer) role-ops. Order matters
    /// and matches `pallet_nomination_pools::Call::update_roles`'s arguments.
    #[precompile::public("updateRoles(uint32,(uint8,address),(uint8,address),(uint8,address))")]
    fn update_roles(
        handle: &mut impl PrecompileHandle,
        pool_id: u32,
        new_root: RoleOp,
        new_nominator: RoleOp,
        new_bouncer: RoleOp,
    ) -> EvmResult {
        delegate_guard(handle)?;
        let new_root = decode_role_op::<Runtime>(new_root)?;
        let new_nominator = decode_role_op::<Runtime>(new_nominator)?;
        let new_bouncer = decode_role_op::<Runtime>(new_bouncer)?;
        let origin = origin_of::<Runtime>(handle);
        let call = pallet_nomination_pools::Call::<Runtime>::update_roles {
            pool_id,
            new_root,
            new_nominator,
            new_bouncer,
        };
        RuntimeHelper::<Runtime>::try_dispatch(handle, RawOrigin::Signed(origin).into(), call, 0)?;
        Ok(())
    }

    #[precompile::public("chill(uint32)")]
    fn chill(handle: &mut impl PrecompileHandle, pool_id: u32) -> EvmResult {
        delegate_guard(handle)?;
        let origin = origin_of::<Runtime>(handle);
        let call = pallet_nomination_pools::Call::<Runtime>::chill { pool_id };
        RuntimeHelper::<Runtime>::try_dispatch(handle, RawOrigin::Signed(origin).into(), call, 0)?;
        Ok(())
    }

    #[precompile::public("bondExtraOther(address,(uint8,uint256))")]
    fn bond_extra_other(
        handle: &mut impl PrecompileHandle,
        member: Address,
        extra: BondExtraSolidity,
    ) -> EvmResult {
        delegate_guard(handle)?;
        let member = lookup::<Runtime>(member);
        let extra = decode_bond_extra::<Runtime>(extra)?;
        let origin = origin_of::<Runtime>(handle);
        let call = pallet_nomination_pools::Call::<Runtime>::bond_extra_other { member, extra };
        RuntimeHelper::<Runtime>::try_dispatch(handle, RawOrigin::Signed(origin).into(), call, 0)?;
        Ok(())
    }

    /// `(commission, payee)`: the empty pair `(0, 0x0)` unsets the pool's
    /// commission (`None`); any non-empty `(non_zero, non_zero)` pair installs
    /// `Some((Perbill, payee))`. Mixed pairs (`(0, payee)` or `(parts, 0x0)`)
    /// are rejected as ambiguous: the upstream pallet would normalise
    /// `Some((0, payee))` to `None` and silently drop the payee, so the
    /// precompile refuses the call instead of letting the caller assume their
    /// payee was recorded. Pass `(0, 0x0)` explicitly to unset.
    #[precompile::public("setCommission(uint32,(uint32,address))")]
    fn set_commission(
        handle: &mut impl PrecompileHandle,
        pool_id: u32,
        new_commission: CommissionPair,
    ) -> EvmResult {
        delegate_guard(handle)?;
        let new_commission = if new_commission.payee.0 == H160::zero() {
            // Unset path: both commission and payee must be zero.
            if new_commission.commission != 0 {
                return Err(revert(
                    "payee=0 requires commission=0 (use empty pair to unset)",
                ));
            }
            None
        } else {
            // Set path: commission must be non-zero, otherwise the pallet
            // would coerce `Some((0, payee))` back to `None` and discard the
            // payee. Surface the ambiguity to the caller.
            if new_commission.commission == 0 {
                return Err(revert(
                    "commission=0 with non-zero payee is ambiguous; pass (0,0x0) to unset",
                ));
            }
            let perbill = Perbill::from_parts(new_commission.commission);
            let payee: <Runtime as frame_system::Config>::AccountId = new_commission.payee.0.into();
            Some((perbill, payee))
        };
        let origin = origin_of::<Runtime>(handle);
        let call = pallet_nomination_pools::Call::<Runtime>::set_commission {
            pool_id,
            new_commission,
        };
        RuntimeHelper::<Runtime>::try_dispatch(handle, RawOrigin::Signed(origin).into(), call, 0)?;
        Ok(())
    }

    #[precompile::public("setCommissionMax(uint32,uint32)")]
    fn set_commission_max(
        handle: &mut impl PrecompileHandle,
        pool_id: u32,
        max_commission: u32,
    ) -> EvmResult {
        delegate_guard(handle)?;
        let max_commission = Perbill::from_parts(max_commission);
        let origin = origin_of::<Runtime>(handle);
        let call = pallet_nomination_pools::Call::<Runtime>::set_commission_max {
            pool_id,
            max_commission,
        };
        RuntimeHelper::<Runtime>::try_dispatch(handle, RawOrigin::Signed(origin).into(), call, 0)?;
        Ok(())
    }

    #[precompile::public("setCommissionChangeRate(uint32,(uint32,uint32))")]
    fn set_commission_change_rate(
        handle: &mut impl PrecompileHandle,
        pool_id: u32,
        change_rate: CommissionChangeRateSolidity,
    ) -> EvmResult {
        delegate_guard(handle)?;
        let change_rate = pallet_nomination_pools::CommissionChangeRate {
            max_increase: Perbill::from_parts(change_rate.max_increase),
            min_delay: change_rate.min_delay.into(),
        };
        let origin = origin_of::<Runtime>(handle);
        let call = pallet_nomination_pools::Call::<Runtime>::set_commission_change_rate {
            pool_id,
            change_rate,
        };
        RuntimeHelper::<Runtime>::try_dispatch(handle, RawOrigin::Signed(origin).into(), call, 0)?;
        Ok(())
    }

    #[precompile::public("claimCommission(uint32)")]
    fn claim_commission(handle: &mut impl PrecompileHandle, pool_id: u32) -> EvmResult {
        delegate_guard(handle)?;
        let origin = origin_of::<Runtime>(handle);
        let call = pallet_nomination_pools::Call::<Runtime>::claim_commission { pool_id };
        RuntimeHelper::<Runtime>::try_dispatch(handle, RawOrigin::Signed(origin).into(), call, 0)?;
        Ok(())
    }

    // ---- view entries --------------------------------------------------

    /// `(points, state, memberCounter, [root, nominator, bouncer],
    ///   (commission, max, (maxIncrease, minDelay), payee))`.
    ///
    /// * `state`: `0=Open`, `1=Blocked`, `2=Destroying`.
    /// * `roles` is encoded as Solidity `address[]` (a dynamic array) because
    ///   the frontier `precompile-utils` solidity Codec does not implement
    ///   fixed-size arrays (`[T; N]`). It is **always** exactly three elements
    ///   in the fixed order `[root, nominator, bouncer]`; callers may safely
    ///   read `roles[0..3]` without bounds-checking the length. Any entry is
    ///   `0x0` when the corresponding role is `None`. The depositor is **not**
    ///   surfaced through this entry — pair `bondedPools` with the pallet's
    ///   `Created` event if you need the depositor.
    /// * `commission.commission` is the current commission Perbill parts;
    ///   `max` is the max-commission Perbill parts; the inner change-rate
    ///   tuple is the configured change-rate (zero when unset). `payee`
    ///   carries the commission beneficiary, `0x0` when commission is unset.
    /// Returns `(0, 0, 0, [0,0,0], (0, 0, (0,0), 0x0))` when the pool does not
    /// exist; callers should pair this with `bondedPools.memberCounter > 0` or
    /// `bondedPools.points > 0` to distinguish absent vs zero-state pools.
    #[precompile::public("bondedPools(uint32)")]
    #[precompile::view]
    #[allow(clippy::type_complexity)]
    fn bonded_pools(
        _handle: &mut impl PrecompileHandle,
        pool_id: u32,
    ) -> EvmResult<(
        U256,
        u8,
        u32,
        Vec<Address>,
        (u32, u32, CommissionChangeRateSolidity, Address),
    )> {
        match pallet_nomination_pools::BondedPools::<Runtime>::get(pool_id) {
            Some(inner) => {
                let state_code: u8 = match inner.state {
                    pallet_nomination_pools::PoolState::Open => 0,
                    pallet_nomination_pools::PoolState::Blocked => 1,
                    pallet_nomination_pools::PoolState::Destroying => 2,
                };
                let role_addr = |o: Option<<Runtime as frame_system::Config>::AccountId>| {
                    o.map(|a| Address(a.into()))
                        .unwrap_or(Address(H160::zero()))
                };
                let roles: Vec<Address> = alloc::vec![
                    role_addr(inner.roles.root.clone()),
                    role_addr(inner.roles.nominator.clone()),
                    role_addr(inner.roles.bouncer.clone()),
                ];
                let (cur_commission, payee_addr) = match inner.commission.current.clone() {
                    Some((p, a)) => (p.deconstruct(), Address(a.into())),
                    None => (0u32, Address(H160::zero())),
                };
                let max_commission = inner
                    .commission
                    .max
                    .map(|m| m.deconstruct())
                    .unwrap_or(0u32);
                let change_rate = inner
                    .commission
                    .change_rate
                    .map(|c| CommissionChangeRateSolidity {
                        max_increase: c.max_increase.deconstruct(),
                        min_delay: u32_from_block_number::<Runtime>(c.min_delay),
                    })
                    .unwrap_or_default();
                Ok((
                    inner.points.into(),
                    state_code,
                    inner.member_counter,
                    roles,
                    (cur_commission, max_commission, change_rate, payee_addr),
                ))
            }
            None => Ok((
                U256::zero(),
                0u8,
                0u32,
                alloc::vec![Address(H160::zero()); 3],
                (
                    0u32,
                    0u32,
                    CommissionChangeRateSolidity::default(),
                    Address(H160::zero()),
                ),
            )),
        }
    }

    /// `(poolId, points, lastRecordedRewardCounter, unbondingEras[])`.
    ///
    /// `lastRecordedRewardCounter` is a `FixedU128`; we cast it to a `U256` by
    /// scale-encoding the underlying `u128` value. Callers wanting the
    /// fractional reward counter should decode it with the FixedU128 base
    /// constant (`10^18`).
    #[precompile::public("poolMembers(address)")]
    #[precompile::view]
    fn pool_members(
        _handle: &mut impl PrecompileHandle,
        account: Address,
    ) -> EvmResult<(u32, U256, U256, Vec<UnbondingEraSolidity>)> {
        let acc: <Runtime as frame_system::Config>::AccountId = account.0.into();
        match pallet_nomination_pools::PoolMembers::<Runtime>::get(&acc) {
            Some(m) => {
                let counter_u256: U256 = m.last_recorded_reward_counter.into_inner().into();
                let unbonding: Vec<UnbondingEraSolidity> = m
                    .unbonding_eras
                    .iter()
                    .map(|(era, points)| UnbondingEraSolidity {
                        era: *era,
                        points: (*points).into(),
                    })
                    .collect();
                Ok((m.pool_id, m.points.into(), counter_u256, unbonding))
            }
            None => Ok((0u32, U256::zero(), U256::zero(), Vec::new())),
        }
    }

    /// Raw metadata bytes registered for `poolId`, empty if unset.
    #[precompile::public("metadata(uint32)")]
    #[precompile::view]
    fn metadata(_handle: &mut impl PrecompileHandle, pool_id: u32) -> EvmResult<UnboundedBytes> {
        let bytes = pallet_nomination_pools::Metadata::<Runtime>::get(pool_id).to_vec();
        Ok(bytes.into())
    }

    /// Ever-increasing counter of all pools that have ever been created.
    #[precompile::public("lastPoolId()")]
    #[precompile::view]
    fn last_pool_id(_handle: &mut impl PrecompileHandle) -> EvmResult<u32> {
        Ok(pallet_nomination_pools::LastPoolId::<Runtime>::get())
    }
}

// ---- shared helpers ---------------------------------------------------

/// Reject DELEGATECALL / CALLCODE so an intermediary contract cannot reuse the
/// EOA's signed origin without explicit `msg.value` authorization. Matches the
/// guard in `precompile-batch` / `precompile-staking` / `precompile-session`.
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

fn lookup<Runtime: frame_system::Config>(
    addr: Address,
) -> <<Runtime as frame_system::Config>::Lookup as StaticLookup>::Source
where
    <Runtime as frame_system::Config>::AccountId: From<H160>,
{
    let acc: <Runtime as frame_system::Config>::AccountId = addr.0.into();
    <<Runtime as frame_system::Config>::Lookup as StaticLookup>::unlookup(acc)
}

fn balance_from_u256<R: pallet_nomination_pools::Config>(
    v: U256,
) -> EvmResult<pallet_nomination_pools::BalanceOf<R>>
where
    U256: UniqueSaturatedInto<pallet_nomination_pools::BalanceOf<R>>,
    pallet_nomination_pools::BalanceOf<R>: Into<U256>,
{
    // Mirror the staking precompile pattern: convert, round-trip, and revert on
    // saturation so callers see an explicit error instead of a silently
    // clamped balance.
    let converted: pallet_nomination_pools::BalanceOf<R> = v.unique_saturated_into();
    let round_trip: U256 = converted.into();
    if round_trip != v {
        return Err(revert("balance overflow"));
    }
    Ok(converted)
}

fn decode_bond_extra<R: pallet_nomination_pools::Config>(
    extra: BondExtraSolidity,
) -> EvmResult<pallet_nomination_pools::BondExtra<pallet_nomination_pools::BalanceOf<R>>>
where
    U256: UniqueSaturatedInto<pallet_nomination_pools::BalanceOf<R>>,
    pallet_nomination_pools::BalanceOf<R>: Into<U256>,
{
    Ok(match extra.kind {
        0 => pallet_nomination_pools::BondExtra::FreeBalance(balance_from_u256::<R>(extra.amount)?),
        1 => pallet_nomination_pools::BondExtra::Rewards,
        _ => return Err(revert("invalid BondExtra kind")),
    })
}

fn decode_role_op<R: frame_system::Config>(
    role: RoleOp,
) -> EvmResult<pallet_nomination_pools::ConfigOp<<R as frame_system::Config>::AccountId>>
where
    <R as frame_system::Config>::AccountId: From<H160>,
{
    Ok(match role.op {
        0 => pallet_nomination_pools::ConfigOp::Noop,
        1 => pallet_nomination_pools::ConfigOp::Set(role.account.0.into()),
        2 => pallet_nomination_pools::ConfigOp::Remove,
        _ => return Err(revert("invalid role op")),
    })
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

/// Best-effort `BlockNumber -> u32` conversion for ABI surface; saturates so
/// downstream Solidity tooling never sees a panic.
fn u32_from_block_number<R: pallet_nomination_pools::Config>(
    bn: pallet_nomination_pools::BlockNumberFor<R>,
) -> u32
where
    pallet_nomination_pools::BlockNumberFor<R>: TryInto<u32>,
{
    bn.try_into().unwrap_or(u32::MAX)
}

/// Codec bound for the `address[]` argument of `nominate`.
pub struct GetMaxNominations;
impl frame_support::traits::Get<u32> for GetMaxNominations {
    fn get() -> u32 {
        MAX_NOMINATIONS
    }
}

/// `PrecompileSet` adapter used by the mock runtime only. Production wiring in
/// `runtimes/common::FrontierPrecompilesNpos` lands in Task 9.
#[cfg(test)]
pub struct NominationPoolsPrecompileSet(PhantomData<crate::mock::Runtime>);

#[cfg(test)]
impl NominationPoolsPrecompileSet {
    pub fn new() -> Self {
        Self(PhantomData)
    }
}

#[cfg(test)]
impl Default for NominationPoolsPrecompileSet {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
impl fp_evm::PrecompileSet for NominationPoolsPrecompileSet {
    fn execute(
        &self,
        handle: &mut impl fp_evm::PrecompileHandle,
    ) -> Option<fp_evm::PrecompileResult> {
        if handle.code_address() == H160::from_low_u64_be(PRECOMPILE_ADDRESS) {
            let r: fp_evm::PrecompileResult =
                <NominationPoolsPrecompile<crate::mock::Runtime> as fp_evm::Precompile>::execute(
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
