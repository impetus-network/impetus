use core::marker::PhantomData;
use frame_support::dispatch::{GetDispatchInfo, PostDispatchInfo};
use pallet_evm::{
    IsPrecompileResult, Precompile, PrecompileHandle, PrecompileResult, PrecompileSet,
};
use sp_core::{H160, U256};
use sp_runtime::traits::Dispatchable;

use pallet_evm_precompile_blake2::Blake2F;
use pallet_evm_precompile_bn128::{Bn128Add, Bn128Mul, Bn128Pairing};
use pallet_evm_precompile_curve25519 as curve25519_precompile;
use pallet_evm_precompile_modexp::Modexp;
use pallet_evm_precompile_sha3fips::Sha3FIPS256;
use pallet_evm_precompile_simple::{ECRecover, ECRecoverPublicKey, Identity, Ripemd160, Sha256};
use precompile_bags_list::BagsListPrecompile;
use precompile_batch::BatchPrecompile;
use precompile_fast_unstake::FastUnstakePrecompile;
use precompile_gasless_registry::GaslessRegistryPrecompile;
use precompile_nomination_pools::NominationPoolsPrecompile;
use precompile_session::SessionPrecompile;
use precompile_staking::StakingPrecompile;
use precompile_staking_admin::StakingAdminPrecompile;
use precompile_treasury::TreasuryPrecompile;

/// Precompile set shipped on impulse (testnet) and dev mode.
///
/// Stable surface: Ethereum stdlib (1..=9 — ecrecover, sha256, ripemd160,
/// identity, modexp, bn128 add/mul/pairing, blake2f), curve25519 / Sha3 /
/// ECRecoverPK (1024..=1027), gasless registry (0x0800), batch (0x0808).
/// Adding entries here requires bumping `spec_version` on both runtimes.
pub struct FrontierPrecompilesBasic<R>(PhantomData<R>);

impl<R> Default for FrontierPrecompilesBasic<R>
where
    R: pallet_evm::Config,
{
    fn default() -> Self {
        Self(Default::default())
    }
}

impl<R> FrontierPrecompilesBasic<R>
where
    R: pallet_evm::Config,
{
    pub fn new() -> Self {
        Self::default()
    }

    pub fn used_addresses() -> [H160; 15] {
        [
            hash(1),
            hash(2),
            hash(3),
            hash(4),
            hash(5),
            hash(6),
            hash(7),
            hash(8),
            hash(9),
            hash(1024),
            hash(1025),
            hash(1026),
            hash(1027),
            hash(precompile_gasless_registry::PRECOMPILE_ADDRESS),
            hash(precompile_batch::PRECOMPILE_ADDRESS),
        ]
    }
}

impl<R> PrecompileSet for FrontierPrecompilesBasic<R>
where
    R: pallet_evm::Config
        + frame_system::Config
        + pallet_gasless_registry::Config
        + pallet_sudo::Config,
    <R as frame_system::Config>::AccountId: Into<H160>,
    <R as frame_system::Config>::RuntimeOrigin:
        From<frame_support::dispatch::RawOrigin<<R as frame_system::Config>::AccountId>>,
{
    fn execute(&self, handle: &mut impl PrecompileHandle) -> Option<PrecompileResult> {
        match handle.code_address() {
            a if a == hash(1) => Some(ECRecover::execute(handle)),
            a if a == hash(2) => Some(Sha256::execute(handle)),
            a if a == hash(3) => Some(Ripemd160::execute(handle)),
            a if a == hash(4) => Some(Identity::execute(handle)),
            a if a == hash(5) => Some(Modexp::execute(handle)),
            a if a == hash(6) => Some(Bn128Add::execute(handle)),
            a if a == hash(7) => Some(Bn128Mul::execute(handle)),
            a if a == hash(8) => Some(Bn128Pairing::execute(handle)),
            a if a == hash(9) => Some(Blake2F::execute(handle)),
            a if a == hash(1024) => Some(Sha3FIPS256::<
                R,
                crate::weights::pallet_evm_precompile_sha3fips::WeightInfo<R>,
            >::execute(handle)),
            a if a == hash(1025) => Some(ECRecoverPublicKey::execute(handle)),
            a if a == hash(1026) => Some(curve25519_precompile::Curve25519Add::<
                R,
                crate::weights::pallet_evm_precompile_curve25519::WeightInfo<R>,
            >::execute(handle)),
            a if a == hash(1027) => Some(curve25519_precompile::Curve25519ScalarMul::<
                R,
                crate::weights::pallet_evm_precompile_curve25519::WeightInfo<R>,
            >::execute(handle)),
            a if a == hash(precompile_gasless_registry::PRECOMPILE_ADDRESS) => {
                Some(GaslessRegistryPrecompile::<R>::execute(handle))
            }
            a if a == hash(precompile_batch::PRECOMPILE_ADDRESS) => {
                Some(BatchPrecompile::<R>::execute(handle))
            }
            _ => None,
        }
    }

    fn is_precompile(&self, address: H160, _gas: u64) -> IsPrecompileResult {
        IsPrecompileResult::Answer {
            is_precompile: Self::used_addresses().contains(&address),
            extra_cost: 0,
        }
    }
}

/// Precompile set shipped on impetus (mainnet, NPoS).
///
/// Extends the Basic surface (1..=9, 1024..=1027, gasless registry,
/// batch) with seven NPoS precompiles at 0x0810..=0x0840:
///
/// - 0x0810 (2064) — staking
/// - 0x0818 (2072) — session
/// - 0x0820 (2080) — nomination pools
/// - 0x0828 (2088) — fast-unstake
/// - 0x0830 (2096) — treasury
/// - 0x0838 (2104) — bags-list (Instance1)
/// - 0x0840 (2112) — staking admin (sudo-gated)
pub struct FrontierPrecompilesNpos<R>(PhantomData<R>);

impl<R> Default for FrontierPrecompilesNpos<R>
where
    R: pallet_evm::Config,
{
    fn default() -> Self {
        Self(Default::default())
    }
}

impl<R> FrontierPrecompilesNpos<R>
where
    R: pallet_evm::Config,
{
    pub fn new() -> Self {
        Self::default()
    }

    pub fn used_addresses() -> [H160; 22] {
        [
            // Basic surface — must mirror FrontierPrecompilesBasic::used_addresses().
            hash(1),
            hash(2),
            hash(3),
            hash(4),
            hash(5),
            hash(6),
            hash(7),
            hash(8),
            hash(9),
            hash(1024),
            hash(1025),
            hash(1026),
            hash(1027),
            hash(precompile_gasless_registry::PRECOMPILE_ADDRESS),
            hash(precompile_batch::PRECOMPILE_ADDRESS),
            // NPoS surface (0x0810..=0x0840).
            hash(precompile_staking::PRECOMPILE_ADDRESS),
            hash(precompile_session::PRECOMPILE_ADDRESS),
            hash(precompile_nomination_pools::PRECOMPILE_ADDRESS),
            hash(precompile_fast_unstake::PRECOMPILE_ADDRESS),
            hash(precompile_treasury::PRECOMPILE_ADDRESS),
            hash(precompile_bags_list::PRECOMPILE_ADDRESS),
            hash(precompile_staking_admin::PRECOMPILE_ADDRESS),
        ]
    }
}

impl<R> PrecompileSet for FrontierPrecompilesNpos<R>
where
    R: pallet_evm::Config
        + frame_system::Config
        + pallet_gasless_registry::Config
        + pallet_sudo::Config
        + pallet_staking::Config
        + pallet_session::Config
        + pallet_nomination_pools::Config
        + pallet_fast_unstake::Config
        + pallet_treasury::Config
        + pallet_bags_list::Config<pallet_bags_list::Instance1>,
    <R as frame_system::Config>::AccountId: From<H160> + Into<H160> + Clone + PartialEq,
    <R as frame_system::Config>::RuntimeOrigin:
        From<frame_support::dispatch::RawOrigin<<R as frame_system::Config>::AccountId>>,
    <R as frame_system::Config>::RuntimeCall: Dispatchable<
            RuntimeOrigin = <R as frame_system::Config>::RuntimeOrigin,
            PostInfo = PostDispatchInfo,
        > + GetDispatchInfo
        + From<pallet_staking::Call<R>>
        + From<pallet_session::Call<R>>
        + From<pallet_nomination_pools::Call<R>>
        + From<pallet_fast_unstake::Call<R>>
        + From<pallet_treasury::Call<R>>
        + From<pallet_bags_list::Call<R, pallet_bags_list::Instance1>>,
    <R as pallet_session::Config>::ValidatorId: Into<H160> + Clone,
    <R as pallet_bags_list::Config<pallet_bags_list::Instance1>>::Score: Into<u64>,
    pallet_staking::BalanceOf<R>: Into<U256>,
    U256: sp_runtime::traits::UniqueSaturatedInto<pallet_staking::BalanceOf<R>>,
    pallet_nomination_pools::BalanceOf<R>: Into<U256>,
    U256: sp_runtime::traits::UniqueSaturatedInto<pallet_nomination_pools::BalanceOf<R>>,
    pallet_nomination_pools::BlockNumberFor<R>: From<u32>,
    <<R as pallet_nomination_pools::Config>::RewardCounter as sp_runtime::FixedPointNumber>::Inner:
        Into<U256>,
    pallet_fast_unstake::types::BalanceOf<R>: Into<U256>,
    pallet_treasury::BalanceOf<R>: Into<U256>,
    U256: sp_runtime::traits::UniqueSaturatedInto<pallet_treasury::BalanceOf<R>>,
{
    fn execute(&self, handle: &mut impl PrecompileHandle) -> Option<PrecompileResult> {
        match handle.code_address() {
            // Delegate Basic surface (1..=9, 1024..=1027, gasless, batch).
            a if a == hash(1)
                || a == hash(2)
                || a == hash(3)
                || a == hash(4)
                || a == hash(5)
                || a == hash(6)
                || a == hash(7)
                || a == hash(8)
                || a == hash(9)
                || a == hash(1024)
                || a == hash(1025)
                || a == hash(1026)
                || a == hash(1027)
                || a == hash(precompile_gasless_registry::PRECOMPILE_ADDRESS)
                || a == hash(precompile_batch::PRECOMPILE_ADDRESS) =>
            {
                FrontierPrecompilesBasic::<R>::default().execute(handle)
            }
            // NPoS surface.
            a if a == hash(precompile_staking::PRECOMPILE_ADDRESS) => {
                Some(StakingPrecompile::<R>::execute(handle))
            }
            a if a == hash(precompile_session::PRECOMPILE_ADDRESS) => {
                Some(SessionPrecompile::<R>::execute(handle))
            }
            a if a == hash(precompile_nomination_pools::PRECOMPILE_ADDRESS) => {
                Some(NominationPoolsPrecompile::<R>::execute(handle))
            }
            a if a == hash(precompile_fast_unstake::PRECOMPILE_ADDRESS) => {
                Some(FastUnstakePrecompile::<R>::execute(handle))
            }
            a if a == hash(precompile_treasury::PRECOMPILE_ADDRESS) => {
                Some(TreasuryPrecompile::<R>::execute(handle))
            }
            a if a == hash(precompile_bags_list::PRECOMPILE_ADDRESS) => {
                Some(BagsListPrecompile::<R>::execute(handle))
            }
            a if a == hash(precompile_staking_admin::PRECOMPILE_ADDRESS) => {
                Some(StakingAdminPrecompile::<R>::execute(handle))
            }
            _ => None,
        }
    }

    fn is_precompile(&self, address: H160, _gas: u64) -> IsPrecompileResult {
        IsPrecompileResult::Answer {
            is_precompile: Self::used_addresses().contains(&address),
            extra_cost: 0,
        }
    }
}

fn hash(a: u64) -> H160 {
    H160::from_low_u64_be(a)
}
