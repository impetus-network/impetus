//! Opaque session keys for the impulse (testnet) runtime.
//!
//! Aura + Grandpa only — staking-free, single permissioned authority set.
//! Per-runtime location keeps impetus free to grow a 4-key set without
//! coupling the testnet to Babe types.

use alloc::vec::Vec;

use sp_consensus_aura::sr25519::AuthorityId as AuraId;
use sp_consensus_grandpa::AuthorityId as GrandpaId;
use sp_runtime::impl_opaque_keys;

impl_opaque_keys! {
    pub struct SessionKeys {
        pub aura: AuraId,
        pub grandpa: GrandpaId,
    }
}
