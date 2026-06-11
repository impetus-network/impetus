//! Shared test scaffolding for impetus runtime integration tests.
//!
//! Each integration test crate (`tests/<scenario>.rs`) is its own binary
//! compiled by cargo, so this `common.rs` is included as a regular module
//! via `mod common;` at the top of each test file.

#![allow(dead_code, unused_imports)]

use frame_support::traits::{OnFinalize, OnInitialize};
use impetus_runtime::{
    AccountId, Babe, Balance, Balances, BlockNumber, NominationPools, Runtime, Session,
    SessionKeys, Staking, System, Timestamp, Treasury, UNIT,
};
use sp_core::H160;
use sp_runtime::BuildStorage;

pub const ALICE: H160 = H160(hex_literal::hex!("f39Fd6e51aad88F6F4ce6aB8827279cffFb92266"));
pub const BOB: H160 = H160(hex_literal::hex!("70997970C51812dc3A010C7d01b50e0d17dc79C8"));
pub const CHARLIE: H160 = H160(hex_literal::hex!("3C44CdDdB6a900fa2b585dd299e03d12FA4293BC"));
pub const DAVE: H160 = H160(hex_literal::hex!("90F79bf6EB2c4f870365E785982E1f101E93b906"));
pub const NOMINATOR: H160 = H160(hex_literal::hex!("15d34AAf54267DB7D7c367839AAf71A00a2C6A65"));

pub fn account(h160: H160) -> AccountId {
    h160.into()
}

/// Build a non-zero set of dummy session keys for a validator. Using `[i;32]`
/// instead of all-zero bytes keeps the four key slots distinct between
/// validators, which `pallet-session` requires (it rejects duplicate keys
/// during genesis assembly).
fn dummy_session_keys(i: u8) -> SessionKeys {
    let mut bytes = [0u8; 32];
    bytes[0] = i;
    bytes[31] = i;
    let sr = sp_core::sr25519::Public::from_raw(bytes);
    let ed = sp_core::ed25519::Public::from_raw(bytes);
    SessionKeys {
        babe: sr.into(),
        grandpa: ed.into(),
        im_online: sr.into(),
        authority_discovery: sr.into(),
    }
}

pub struct ExtBuilder {
    pub balances: Vec<(AccountId, Balance)>,
    pub initial_validators: Vec<AccountId>,
    pub initial_nominator: Option<(AccountId, Vec<AccountId>)>,
}

impl Default for ExtBuilder {
    fn default() -> Self {
        let validators: Vec<AccountId> = vec![ALICE, BOB, CHARLIE, DAVE]
            .into_iter()
            .map(account)
            .collect();
        let nominator = account(NOMINATOR);
        let balances = vec![
            (account(ALICE), 10_000 * UNIT),
            (account(BOB), 10_000 * UNIT),
            (account(CHARLIE), 10_000 * UNIT),
            (account(DAVE), 10_000 * UNIT),
            (nominator, 10_000 * UNIT),
        ];
        ExtBuilder {
            balances,
            initial_validators: validators,
            initial_nominator: Some((
                nominator,
                vec![account(ALICE), account(BOB), account(CHARLIE)],
            )),
        }
    }
}

impl ExtBuilder {
    pub fn build(self) -> sp_io::TestExternalities {
        let mut storage = frame_system::GenesisConfig::<Runtime>::default()
            .build_storage()
            .unwrap();

        pallet_balances::GenesisConfig::<Runtime> {
            balances: self.balances.clone(),
            ..Default::default()
        }
        .assimilate_storage(&mut storage)
        .unwrap();

        // Distinct dummy session keys per validator. `pallet-session` rejects
        // duplicate keys, so we seed each slot with a unique byte pattern.
        let session_keys: Vec<(AccountId, AccountId, SessionKeys)> = self
            .initial_validators
            .iter()
            .enumerate()
            .map(|(i, v)| (*v, *v, dummy_session_keys((i as u8) + 1)))
            .collect();
        pallet_session::GenesisConfig::<Runtime> {
            keys: session_keys,
            non_authority_keys: vec![],
        }
        .assimilate_storage(&mut storage)
        .unwrap();

        let mut stakers: Vec<_> = self
            .initial_validators
            .iter()
            .map(|v| {
                (
                    *v,
                    *v,
                    2_000 * UNIT,
                    pallet_staking::StakerStatus::<AccountId>::Validator,
                )
            })
            .collect();
        if let Some((nom, targets)) = self.initial_nominator.clone() {
            stakers.push((
                nom,
                nom,
                5_000 * UNIT,
                pallet_staking::StakerStatus::Nominator(targets),
            ));
        }
        pallet_staking::GenesisConfig::<Runtime> {
            validator_count: 4,
            minimum_validator_count: 1,
            invulnerables: vec![],
            slash_reward_fraction: sp_runtime::Perbill::from_percent(10),
            stakers,
            ..Default::default()
        }
        .assimilate_storage(&mut storage)
        .unwrap();

        // Leave the chain at block 0; the first `run_to_block(n)` call will
        // initialize blocks 1..=n via `frame_system::run_to_block_with`.
        let ext: sp_io::TestExternalities = storage.into();
        ext
    }
}

/// Inject a Babe SecondaryPlain pre-digest into the current frame_system
/// digest so `Babe::initialize` can read a slot. The Babe `OnTimestampSet`
/// hook asserts `CurrentSlot == timestamp / slot_duration`, so each block
/// the harness produces must (a) seed a fresh pre-digest, (b) call
/// on_initialize so `CurrentSlot` is populated, then (c) advance the
/// timestamp.
fn seed_babe_pre_digest(slot: sp_consensus_babe::Slot) {
    use scale_codec::Encode;
    use sp_consensus_babe::{
        digests::{PreDigest, SecondaryPlainPreDigest},
        BABE_ENGINE_ID,
    };
    use sp_runtime::DigestItem;

    let pre_digest = PreDigest::SecondaryPlain(SecondaryPlainPreDigest {
        authority_index: 0,
        slot,
    });
    let digest_item = DigestItem::PreRuntime(BABE_ENGINE_ID, pre_digest.encode());
    frame_system::Pallet::<Runtime>::deposit_log(digest_item);
}

pub fn run_to_block(target: BlockNumber) {
    use impetus_runtime::AllPalletsWithSystem;
    use sp_consensus_babe::Slot;
    use sp_runtime::traits::Header as _;
    const BLOCK_MS: u64 = impetus_runtime::MILLISECS_PER_BLOCK;
    const SLOT_MS: u64 = impetus_runtime::SLOT_DURATION;
    let slot_step = BLOCK_MS / SLOT_MS;

    while System::block_number() < target {
        let bn = System::block_number();
        let next = bn + 1;

        if bn > 0 {
            <AllPalletsWithSystem as OnFinalize<BlockNumber>>::on_finalize(bn);
            // `finalize()` returns a Header; we throw the result away but the
            // call is what clears intra-block storage AND the digest so the
            // next block starts clean. Using the proper finalize() is closer
            // to a real block-import path than killing storage by hand.
            let _header = frame_system::Pallet::<Runtime>::finalize();
        }

        // `initialize()` sets number, parent_hash, and the digest, then
        // increments the block number. The digest we pass here is what
        // Babe's `on_initialize` reads back to populate `CurrentSlot`.
        let slot = Slot::from((next as u64) * slot_step);
        let mut digest = sp_runtime::Digest::default();
        {
            use scale_codec::Encode;
            use sp_consensus_babe::{
                digests::{PreDigest, SecondaryPlainPreDigest},
                BABE_ENGINE_ID,
            };
            let pre = PreDigest::SecondaryPlain(SecondaryPlainPreDigest {
                authority_index: 0,
                slot,
            });
            digest.push(sp_runtime::DigestItem::PreRuntime(BABE_ENGINE_ID, pre.encode()));
        }
        let parent_hash = if bn == 0 {
            <impetus_runtime::Hash as Default>::default()
        } else {
            frame_system::Pallet::<Runtime>::block_hash(bn)
        };
        frame_system::Pallet::<Runtime>::initialize(&next, &parent_hash, &digest);
        <AllPalletsWithSystem as OnInitialize<BlockNumber>>::on_initialize(next);
        // Set the timestamp last so Babe's `OnTimestampSet` assertion succeeds
        // (CurrentSlot was just populated by Babe::initialize).
        Timestamp::set_timestamp((next as u64) * BLOCK_MS);
    }
}

pub fn run_to_session(idx: u32) {
    let target_block: BlockNumber = (idx as BlockNumber) * impetus_runtime::SESSION_PERIOD;
    if System::block_number() < target_block {
        run_to_block(target_block);
    }
}

pub fn run_to_era(era: u32) {
    let blocks_per_era: BlockNumber =
        impetus_runtime::SESSION_PERIOD * impetus_runtime::SessionsPerEra::get();
    let target = blocks_per_era * (era + 1);
    if System::block_number() < target {
        run_to_block(target);
    }
}

