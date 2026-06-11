//! Opaque session keys for the impetus (mainnet/NPoS) runtime.
//!
//! Four keys, registered atomically per validator via Session::set_keys.
//! Plan 2 wires `pallet-session` so the value types here are now the
//! pallet wrappers rather than the raw `*::AuthorityId` keys — only the
//! pallets implement `OneSessionHandler<AccountId>`, which is what
//! `<SessionKeys as OpaqueKeys>::KeyTypeIdProviders` needs.

use alloc::vec::Vec;
use sp_runtime::impl_opaque_keys;

use super::{AuthorityDiscovery, Babe, Grandpa, ImOnline};

impl_opaque_keys! {
    pub struct SessionKeys {
        pub babe: Babe,
        pub grandpa: Grandpa,
        pub im_online: ImOnline,
        pub authority_discovery: AuthorityDiscovery,
    }
}
