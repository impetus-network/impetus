# Betting Backend Implementation Plan (Plan A)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add multi-number betting, bet updating (with dynamic gas), and new precompile functions to the Artemis betting pallet.

**Architecture:** Re-key `Bets` storage from `(RoundId, AccountId)` to `(RoundId, AccountId, u8)` for multi-number support. Add `place_bets` (batch) and `update_bet` (increase=gasless, decrease=paid) pallet calls. Expose `placeBets`, `updateBet`, `getBets` through the EVM precompile. Update Solidity interface and shared ABI/types.

**Tech Stack:** Rust (Substrate FRAME pallet, Frontier EVM precompile), Solidity (interface only), TypeScript (shared types/ABI)

---

## File Map

| File | Responsibility |
|------|---------------|
| `packages/node/pallets/betting/src/types.rs` | Remove `number` from `BetInfo`, keep `token`, `amount`, `claimed` |
| `packages/node/pallets/betting/src/lib.rs` | Re-key `Bets` to `StorageNMap`, add `place_bets`/`update_bet`, add `BetUpdated` event, update `place_bet`/`claim_winnings`/`admin_claim_pool`, add new errors |
| `packages/node/pallets/betting/src/tests.rs` | Tests for all new/changed pallet functions |
| `packages/node/precompiles/betting/src/lib.rs` | Add `placeBets`/`updateBet`/`getBets` selectors, update `getBet` to include number param, add dynamic array ABI decode/encode |
| `packages/node/precompiles/betting/src/tests.rs` | Precompile tests for new functions |
| `packages/contracts/contracts/interfaces/IBettingPrecompile.sol` | Add `placeBets`, `updateBet`, `getBets`; update `getBet`; add `BetUpdated` event |
| `packages/shared/src/types/betting.ts` | Update `BetInfo` type (remove `number`) |
| `packages/shared/abis/IBettingPrecompile.json` | Regenerated ABI |

---

### Task 1: Update BetInfo type -- remove `number` field

**Files:**
- Modify: `packages/node/pallets/betting/src/types.rs`

The `number` field moves from the struct to the storage key. This is a breaking change that all later tasks depend on.

- [ ] **Step 1: Update BetInfo struct**

Edit `packages/node/pallets/betting/src/types.rs`. Replace the `BetInfo` struct:

```rust
#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, MaxEncodedLen, TypeInfo)]
pub struct BetInfo<Balance> {
	pub token: TokenId,
	pub amount: Balance,
	pub claimed: bool,
}
```

Remove the `number: u8` field entirely.

- [ ] **Step 2: Verify compilation fails (expected -- dependents reference `number`)**

Run:
```bash
cd packages/node && cargo check -p pallet-betting 2>&1 | head -20
```

Expected: Compilation errors referencing `bet.number` and `BetInfo { number, ... }` in `lib.rs` and `tests.rs`. This is correct -- we will fix these in subsequent tasks.

- [ ] **Step 3: Commit**

```bash
git add packages/node/pallets/betting/src/types.rs
git commit -m "refactor(pallet-betting): remove number field from BetInfo

Number is now part of the storage key (RoundId, AccountId, u8)
instead of being stored inside BetInfo."
```

---

### Task 2: Re-key Bets storage and update existing pallet functions

**Files:**
- Modify: `packages/node/pallets/betting/src/lib.rs`

Change `Bets` from `StorageDoubleMap<RoundId, AccountId, BetInfo>` to `StorageNMap<(RoundId, AccountId, u8), BetInfo>`. Update all existing functions that access `Bets`.

- [ ] **Step 1: Add StorageNMap import and re-key Bets storage**

In `packages/node/pallets/betting/src/lib.rs`, add `storage::Key` import. Change:

```rust
use frame_support::{
    pallet_prelude::*,
    traits::{
        Currency, ExistenceRequirement, UnixTime,
        fungibles::{self, Mutate as FungiblesMutate},
        tokens::Preservation,
    },
    PalletId,
};
```

to:

```rust
use frame_support::{
    pallet_prelude::*,
    storage::Key,
    traits::{
        Currency, ExistenceRequirement, UnixTime,
        fungibles::{self, Mutate as FungiblesMutate},
        tokens::Preservation,
    },
    PalletId,
};
```

Replace the `Bets` storage declaration:

```rust
/// Bet info per (round, account, number)
#[pallet::storage]
pub type Bets<T: Config> = StorageNMap<
    _,
    (
        Key<Blake2_128Concat, RoundId>,
        Key<Blake2_128Concat, T::AccountId>,
        Key<Blake2_128Concat, u8>,
    ),
    BetInfo<BalanceOf<T>>,
>;
```

- [ ] **Step 2: Add new error variants**

Add to the `Error<T>` enum:

```rust
DuplicateNumber,
BetNotFound,
TokenMismatch,
ArrayLengthMismatch,
TooManyBets,
```

- [ ] **Step 3: Add BetUpdated event**

Add to the `Event<T>` enum:

```rust
BetUpdated {
    round_id: RoundId,
    who: T::AccountId,
    number: u8,
    token: TokenId,
    old_amount: BalanceOf<T>,
    new_amount: BalanceOf<T>,
},
```

- [ ] **Step 4: Update `place_bet` to use new storage key**

In the `place_bet` function, change the `AlreadyBet` check from:

```rust
// Check user has not already bet in this round
ensure!(
    !Bets::<T>::contains_key(round_id, &who),
    Error::<T>::AlreadyBet
);
```

to:

```rust
// Check user has not already bet this number in this round
ensure!(
    !Bets::<T>::contains_key((round_id, &who, number)),
    Error::<T>::AlreadyBet
);
```

Change the `Bets::insert` call from:

```rust
Bets::<T>::insert(
    round_id,
    &who,
    BetInfo {
        number,
        token,
        amount,
        claimed: false,
    },
);
```

to:

```rust
Bets::<T>::insert(
    (round_id, &who, number),
    BetInfo {
        token,
        amount,
        claimed: false,
    },
);
```

- [ ] **Step 5: Update `claim_winnings` to use new storage key**

In `claim_winnings`, the function currently reads a single bet per user. With multi-number support, the user specifies which number to claim. Add `number: u8` parameter:

Change function signature from:

```rust
pub fn claim_winnings(
    origin: OriginFor<T>,
    round_id: RoundId,
) -> DispatchResult {
```

to:

```rust
pub fn claim_winnings(
    origin: OriginFor<T>,
    round_id: RoundId,
    number: u8,
) -> DispatchResult {
```

Update the bet lookup from:

```rust
let mut bet = Bets::<T>::get(round_id, &who)
    .ok_or(Error::<T>::NotWinner)?;
```

to:

```rust
let mut bet = Bets::<T>::get((round_id, &who, number))
    .ok_or(Error::<T>::NotWinner)?;
```

Remove the old number match check (was `ensure!(bet.number == result, ...)`), replace with:

```rust
let result = round_info.result.ok_or(Error::<T>::RoundNotResolved)?;
ensure!(number == result, Error::<T>::NotWinner);
```

Update the bet write-back from:

```rust
bet.claimed = true;
Bets::<T>::insert(round_id, &who, bet.clone());
```

to:

```rust
bet.claimed = true;
Bets::<T>::insert((round_id, &who, number), bet.clone());
```

- [ ] **Step 6: Update `admin_claim_pool` to use new storage iteration**

In `admin_claim_pool`, change the iteration from:

```rust
let mut tokens = BTreeSet::<TokenId>::new();
for (_account, bet) in Bets::<T>::iter_prefix(round_id) {
    tokens.insert(bet.token);
}
```

to iterating the NMap with a partial key prefix. Use `iter_key_prefix` with round_id:

```rust
let mut tokens = BTreeSet::<TokenId>::new();
for ((_account, _number), bet) in Bets::<T>::iter_prefix((round_id,)) {
    tokens.insert(bet.token);
}
```

Note: `StorageNMap::iter_prefix` with a single-element tuple `(round_id,)` iterates all entries whose first key is `round_id`, returning the remaining keys and value.

- [ ] **Step 7: Verify pallet compiles (tests may still fail)**

Run:
```bash
cd packages/node && cargo check -p pallet-betting
```

Expected: Compiles successfully. Tests will be fixed in Task 4.

- [ ] **Step 8: Commit**

```bash
git add packages/node/pallets/betting/src/lib.rs
git commit -m "refactor(pallet-betting): re-key Bets to StorageNMap

Storage key is now (RoundId, AccountId, u8) for multi-number
betting. Updated place_bet, claim_winnings, admin_claim_pool.
Added BetUpdated event, new error variants."
```

---

### Task 3: Add `place_bets` and `update_bet` pallet functions

**Files:**
- Modify: `packages/node/pallets/betting/src/lib.rs`

- [ ] **Step 1: Add `place_bets` function**

Add after the existing `place_bet` function, using `call_index(10)`:

```rust
/// Place multiple bets for the current open round.
/// Gasless for users (Pays::No).
#[pallet::call_index(10)]
#[pallet::weight((10_000, Pays::No))]
pub fn place_bets(
    origin: OriginFor<T>,
    numbers: Vec<u8>,
    amounts: Vec<BalanceOf<T>>,
    tokens: Vec<TokenId>,
) -> DispatchResult {
    let who = ensure_signed(origin)?;

    // Validate arrays are same length
    let len = numbers.len();
    ensure!(len == amounts.len() && len == tokens.len(), Error::<T>::ArrayLengthMismatch);
    ensure!(len > 0 && len <= 100, Error::<T>::TooManyBets);

    // Check for duplicate numbers
    let mut seen = BTreeSet::<u8>::new();
    for &n in &numbers {
        ensure!(n <= 99, Error::<T>::InvalidNumber);
        ensure!(seen.insert(n), Error::<T>::DuplicateNumber);
    }

    // Validate all amounts non-zero and all tokens supported
    for i in 0..len {
        ensure!(amounts[i] > Zero::zero(), Error::<T>::InvalidAmount);
        match tokens[i] {
            None => {
                ensure!(
                    NativeTokenSupported::<T>::get(),
                    Error::<T>::UnsupportedToken
                );
            }
            Some(id) => {
                ensure!(
                    SupportedTokens::<T>::get(id),
                    Error::<T>::UnsupportedToken
                );
            }
        }
    }

    // Determine current round
    let now_ms = T::TimeProvider::now().as_millis();
    let now_secs = (now_ms / 1000) as u64;
    let round_id = round::current_round_id(now_secs);

    // Check no duplicates with existing bets
    for &n in &numbers {
        ensure!(
            !Bets::<T>::contains_key((round_id, &who, n)),
            Error::<T>::AlreadyBet
        );
    }

    // Aggregate total per token for transfers
    let mut native_total: BalanceOf<T> = Zero::zero();
    let mut asset_totals = BTreeSet::<(u32, BalanceOf<T>)>::new();
    // Use a BTreeMap for asset totals
    let mut asset_map = alloc::collections::btree_map::BTreeMap::<u32, BalanceOf<T>>::new();
    for i in 0..len {
        match tokens[i] {
            None => {
                native_total = native_total.saturating_add(amounts[i]);
            }
            Some(id) => {
                let entry = asset_map.entry(id).or_insert(Zero::zero());
                *entry = entry.saturating_add(amounts[i]);
            }
        }
    }

    // Transfer native tokens
    let pallet_account = Self::account_id();
    if native_total > Zero::zero() {
        T::Currency::transfer(
            &who,
            &pallet_account,
            native_total,
            ExistenceRequirement::KeepAlive,
        )
        .map_err(|_| Error::<T>::InsufficientBalance)?;
    }

    // Transfer asset tokens
    for (id, total) in &asset_map {
        if *total > Zero::zero() {
            <T::Assets as FungiblesMutate<T::AccountId>>::transfer(
                *id,
                &who,
                &pallet_account,
                *total,
                Preservation::Preserve,
            )
            .map_err(|_| Error::<T>::InsufficientBalance)?;
        }
    }

    // Create or update round entry
    if !Rounds::<T>::contains_key(round_id) {
        let close_ts = round::round_close_timestamp(round_id);
        Rounds::<T>::insert(
            round_id,
            RoundInfo {
                close_timestamp: close_ts,
                status: RoundStatus::Open,
                result: None,
            },
        );
    }

    // Insert bets and emit events
    for i in 0..len {
        Bets::<T>::insert(
            (round_id, &who, numbers[i]),
            BetInfo {
                token: tokens[i],
                amount: amounts[i],
                claimed: false,
            },
        );

        Self::deposit_event(Event::BetPlaced {
            round_id,
            who: who.clone(),
            number: numbers[i],
            token: tokens[i],
            amount: amounts[i],
        });
    }

    Ok(())
}
```

- [ ] **Step 2: Add `update_bet` function**

Add after `place_bets`, using `call_index(11)`:

```rust
/// Update a bet's amount. Increase is gasless, decrease/remove pays gas.
#[pallet::call_index(11)]
#[pallet::weight((10_000, Pays::Yes))]
pub fn update_bet(
    origin: OriginFor<T>,
    round_id: RoundId,
    number: u8,
    new_amount: BalanceOf<T>,
    token: TokenId,
) -> DispatchResultWithPostInfo {
    let who = ensure_signed(origin)?;

    // Round must exist and be Open
    let round_info =
        Rounds::<T>::get(round_id).ok_or(Error::<T>::RoundNotFound)?;
    ensure!(
        round_info.status == RoundStatus::Open,
        Error::<T>::RoundNotOpen
    );

    // Round must still be accepting bets (not past cutoff)
    let now_ms = T::TimeProvider::now().as_millis();
    let now_secs = (now_ms / 1000) as u64;
    ensure!(
        round::is_round_open(now_secs, round_id),
        Error::<T>::RoundNotOpen
    );

    // Bet must exist
    let old_bet = Bets::<T>::get((round_id, &who, number))
        .ok_or(Error::<T>::BetNotFound)?;

    // Token must match
    ensure!(old_bet.token == token, Error::<T>::TokenMismatch);

    let old_amount = old_bet.amount;
    let pallet_account = Self::account_id();

    if new_amount == Zero::zero() {
        // Remove bet entirely -- refund full amount
        match token {
            None => {
                T::Currency::transfer(
                    &pallet_account,
                    &who,
                    old_amount,
                    ExistenceRequirement::AllowDeath,
                )
                .map_err(|_| Error::<T>::PoolEmpty)?;
            }
            Some(id) => {
                <T::Assets as FungiblesMutate<T::AccountId>>::transfer(
                    id,
                    &pallet_account,
                    &who,
                    old_amount,
                    Preservation::Expendable,
                )
                .map_err(|_| Error::<T>::PoolEmpty)?;
            }
        }

        Bets::<T>::remove((round_id, &who, number));
    } else if new_amount > old_amount {
        // Increase -- transfer difference to pallet
        let diff = new_amount.saturating_sub(old_amount);
        match token {
            None => {
                T::Currency::transfer(
                    &who,
                    &pallet_account,
                    diff,
                    ExistenceRequirement::KeepAlive,
                )
                .map_err(|_| Error::<T>::InsufficientBalance)?;
            }
            Some(id) => {
                <T::Assets as FungiblesMutate<T::AccountId>>::transfer(
                    id,
                    &who,
                    &pallet_account,
                    diff,
                    Preservation::Preserve,
                )
                .map_err(|_| Error::<T>::InsufficientBalance)?;
            }
        }

        Bets::<T>::insert(
            (round_id, &who, number),
            BetInfo {
                token,
                amount: new_amount,
                claimed: false,
            },
        );
    } else if new_amount < old_amount {
        // Decrease -- refund difference
        let diff = old_amount.saturating_sub(new_amount);
        match token {
            None => {
                T::Currency::transfer(
                    &pallet_account,
                    &who,
                    diff,
                    ExistenceRequirement::AllowDeath,
                )
                .map_err(|_| Error::<T>::PoolEmpty)?;
            }
            Some(id) => {
                <T::Assets as FungiblesMutate<T::AccountId>>::transfer(
                    id,
                    &pallet_account,
                    &who,
                    diff,
                    Preservation::Expendable,
                )
                .map_err(|_| Error::<T>::PoolEmpty)?;
            }
        }

        Bets::<T>::insert(
            (round_id, &who, number),
            BetInfo {
                token,
                amount: new_amount,
                claimed: false,
            },
        );
    }
    // If new_amount == old_amount, no-op (no transfer, no storage change)

    Self::deposit_event(Event::BetUpdated {
        round_id,
        who,
        number,
        token,
        old_amount,
        new_amount,
    });

    // Dynamic gas: increase or same = gasless, decrease or remove = paid
    let pays = if new_amount >= old_amount {
        Pays::No
    } else {
        Pays::Yes
    };

    Ok(Some(10_000).into())
        .map(|_: PostDispatchInfo| PostDispatchInfo {
            actual_weight: Some(frame_support::weights::Weight::from_parts(10_000, 0)),
            pays_fee: pays,
        })
}
```

Note: `update_bet` returns `DispatchResultWithPostInfo` instead of `DispatchResult` to allow dynamic `Pays`. The function signature annotation uses `Pays::Yes` as default (pessimistic), then overrides to `Pays::No` when amount increases.

- [ ] **Step 3: Verify compilation**

Run:
```bash
cd packages/node && cargo check -p pallet-betting
```

Expected: Compiles (tests may still fail due to `tests.rs` not updated yet).

- [ ] **Step 4: Commit**

```bash
git add packages/node/pallets/betting/src/lib.rs
git commit -m "feat(pallet-betting): add place_bets and update_bet functions

place_bets: batch multi-number betting in single tx (gasless)
update_bet: increase (gasless) or decrease/remove (paid) bet amount"
```

---

### Task 4: Update pallet unit tests

**Files:**
- Modify: `packages/node/pallets/betting/src/tests.rs`

All existing tests reference the old `Bets` storage key `(round_id, who)` and `BetInfo { number, ... }`. They must be updated to use `(round_id, who, number)` and the new `BetInfo` without `number`. Additionally, `claim_winnings` now takes a `number` parameter.

- [ ] **Step 1: Update all existing test `Bets::get` calls**

Throughout `tests.rs`, replace all patterns of:
- `Bets::<Test>::get(round_id, &ALICE)` with `Bets::<Test>::get((round_id, &ALICE, NUMBER))` where `NUMBER` is the bet number used in that test
- `Bets::<Test>::get(round_id, &BOB)` similarly

Replace all `BetInfo` struct literals -- remove the `number` field:
- `BetInfo { number: N, token: ..., amount: ..., claimed: ... }` becomes `BetInfo { token: ..., amount: ..., claimed: ... }`

Replace all `claim_winnings(origin, round_id)` calls with `claim_winnings(origin, round_id, number)` where `number` is the winning number used in that test.

- [ ] **Step 2: Add test for place_bets**

```rust
#[test]
fn place_bets_multiple_numbers() {
    new_test_ext().execute_with(|| {
        let ts = before_cutoff_timestamp_secs();
        set_timestamp(ts * 1000);
        Admin::<Test>::put(ADMIN);

        let round_id = current_round_id(ts);

        assert_ok!(Betting::place_bets(
            RuntimeOrigin::signed(ALICE),
            vec![10, 20, 30],
            vec![100, 200, 300],
            vec![None, None, None],
        ));

        // Verify all 3 bets exist
        let bet10 = Bets::<Test>::get((round_id, &ALICE, 10u8)).unwrap();
        assert_eq!(bet10.amount, 100);
        let bet20 = Bets::<Test>::get((round_id, &ALICE, 20u8)).unwrap();
        assert_eq!(bet20.amount, 200);
        let bet30 = Bets::<Test>::get((round_id, &ALICE, 30u8)).unwrap();
        assert_eq!(bet30.amount, 300);

        // Total transferred: 600
        assert_eq!(
            Balances::free_balance(ALICE),
            INITIAL_BALANCE - 600
        );
    });
}

#[test]
fn place_bets_rejects_duplicate_numbers() {
    new_test_ext().execute_with(|| {
        let ts = before_cutoff_timestamp_secs();
        set_timestamp(ts * 1000);

        assert_noop!(
            Betting::place_bets(
                RuntimeOrigin::signed(ALICE),
                vec![10, 10],
                vec![100, 200],
                vec![None, None],
            ),
            pallet_betting::Error::<Test>::DuplicateNumber
        );
    });
}

#[test]
fn place_bets_rejects_mismatched_arrays() {
    new_test_ext().execute_with(|| {
        let ts = before_cutoff_timestamp_secs();
        set_timestamp(ts * 1000);

        assert_noop!(
            Betting::place_bets(
                RuntimeOrigin::signed(ALICE),
                vec![10, 20],
                vec![100],
                vec![None, None],
            ),
            pallet_betting::Error::<Test>::ArrayLengthMismatch
        );
    });
}

#[test]
fn place_bets_rejects_already_bet_number() {
    new_test_ext().execute_with(|| {
        let ts = before_cutoff_timestamp_secs();
        set_timestamp(ts * 1000);

        // Place a single bet on number 10
        assert_ok!(Betting::place_bet(
            RuntimeOrigin::signed(ALICE),
            10,
            None,
            100,
        ));

        // Try to batch-bet including number 10 again
        assert_noop!(
            Betting::place_bets(
                RuntimeOrigin::signed(ALICE),
                vec![10, 20],
                vec![100, 200],
                vec![None, None],
            ),
            pallet_betting::Error::<Test>::AlreadyBet
        );
    });
}
```

- [ ] **Step 3: Add tests for update_bet**

```rust
#[test]
fn update_bet_increase_amount() {
    new_test_ext().execute_with(|| {
        let ts = before_cutoff_timestamp_secs();
        set_timestamp(ts * 1000);
        Admin::<Test>::put(ADMIN);

        let round_id = current_round_id(ts);

        assert_ok!(Betting::place_bet(
            RuntimeOrigin::signed(ALICE),
            42,
            None,
            100,
        ));

        assert_ok!(Betting::update_bet(
            RuntimeOrigin::signed(ALICE),
            round_id,
            42,
            250,
            None,
        ));

        let bet = Bets::<Test>::get((round_id, &ALICE, 42u8)).unwrap();
        assert_eq!(bet.amount, 250);

        // Total transferred: 250 (100 initial + 150 increase)
        assert_eq!(
            Balances::free_balance(ALICE),
            INITIAL_BALANCE - 250
        );
    });
}

#[test]
fn update_bet_decrease_amount() {
    new_test_ext().execute_with(|| {
        let ts = before_cutoff_timestamp_secs();
        set_timestamp(ts * 1000);
        Admin::<Test>::put(ADMIN);

        let round_id = current_round_id(ts);

        assert_ok!(Betting::place_bet(
            RuntimeOrigin::signed(ALICE),
            42,
            None,
            300,
        ));

        assert_ok!(Betting::update_bet(
            RuntimeOrigin::signed(ALICE),
            round_id,
            42,
            100,
            None,
        ));

        let bet = Bets::<Test>::get((round_id, &ALICE, 42u8)).unwrap();
        assert_eq!(bet.amount, 100);

        // Refunded 200, so balance = INITIAL - 100
        assert_eq!(
            Balances::free_balance(ALICE),
            INITIAL_BALANCE - 100
        );
    });
}

#[test]
fn update_bet_remove_with_zero() {
    new_test_ext().execute_with(|| {
        let ts = before_cutoff_timestamp_secs();
        set_timestamp(ts * 1000);
        Admin::<Test>::put(ADMIN);

        let round_id = current_round_id(ts);

        assert_ok!(Betting::place_bet(
            RuntimeOrigin::signed(ALICE),
            42,
            None,
            300,
        ));

        assert_ok!(Betting::update_bet(
            RuntimeOrigin::signed(ALICE),
            round_id,
            42,
            0,
            None,
        ));

        // Bet should be removed
        assert!(Bets::<Test>::get((round_id, &ALICE, 42u8)).is_none());

        // Full refund
        assert_eq!(
            Balances::free_balance(ALICE),
            INITIAL_BALANCE
        );
    });
}

#[test]
fn update_bet_rejects_wrong_token() {
    new_test_ext().execute_with(|| {
        let ts = before_cutoff_timestamp_secs();
        set_timestamp(ts * 1000);
        Admin::<Test>::put(ADMIN);

        let round_id = current_round_id(ts);

        assert_ok!(Betting::place_bet(
            RuntimeOrigin::signed(ALICE),
            42,
            None,  // native token
            100,
        ));

        assert_noop!(
            Betting::update_bet(
                RuntimeOrigin::signed(ALICE),
                round_id,
                42,
                200,
                Some(1),  // wrong token
            ),
            pallet_betting::Error::<Test>::TokenMismatch
        );
    });
}

#[test]
fn update_bet_rejects_nonexistent_bet() {
    new_test_ext().execute_with(|| {
        let ts = before_cutoff_timestamp_secs();
        set_timestamp(ts * 1000);
        Admin::<Test>::put(ADMIN);

        let round_id = current_round_id(ts);

        assert_noop!(
            Betting::update_bet(
                RuntimeOrigin::signed(ALICE),
                round_id,
                42,
                200,
                None,
            ),
            pallet_betting::Error::<Test>::BetNotFound
        );
    });
}

#[test]
fn update_bet_rejects_closed_round() {
    new_test_ext().execute_with(|| {
        let ts_before = before_cutoff_timestamp_secs();
        set_timestamp(ts_before * 1000);
        Admin::<Test>::put(ADMIN);

        let round_id = current_round_id(ts_before);

        assert_ok!(Betting::place_bet(
            RuntimeOrigin::signed(ALICE),
            42,
            None,
            100,
        ));

        // Force close the round
        assert_ok!(Betting::force_close_round(
            RuntimeOrigin::signed(ADMIN),
            round_id,
        ));

        // Move time past cutoff so round is no longer open
        let ts_after = after_cutoff_timestamp_secs();
        set_timestamp(ts_after * 1000);

        assert_noop!(
            Betting::update_bet(
                RuntimeOrigin::signed(ALICE),
                round_id,
                42,
                200,
                None,
            ),
            pallet_betting::Error::<Test>::RoundNotOpen
        );
    });
}
```

- [ ] **Step 4: Run all pallet tests**

Run:
```bash
cd packages/node && cargo test -p pallet-betting
```

Expected: All tests pass. Fix any compilation or assertion errors.

- [ ] **Step 5: Commit**

```bash
git add packages/node/pallets/betting/src/tests.rs
git commit -m "test(pallet-betting): update tests for multi-number betting

Update existing tests for new StorageNMap key and BetInfo without
number field. Add tests for place_bets, update_bet (increase,
decrease, remove, error cases)."
```

---

### Task 5: Update precompile -- add `placeBets`, `updateBet`, `getBets`, update `getBet`

**Files:**
- Modify: `packages/node/precompiles/betting/src/lib.rs`

- [ ] **Step 1: Add new selector functions**

After the existing selector functions, add:

```rust
fn selector_place_bets() -> [u8; 4] {
    selector("placeBets(uint8[],uint256[],address[])")
}

fn selector_update_bet() -> [u8; 4] {
    selector("updateBet(uint256,uint8,uint256,address)")
}

fn selector_get_bets() -> [u8; 4] {
    selector("getBets(uint256,address)")
}
```

Update the existing `getBet` selector to include the new `number` parameter:

```rust
fn selector_get_bet() -> [u8; 4] {
    selector("getBet(uint256,address,uint8)")
}
```

- [ ] **Step 2: Add dynamic array ABI decode helpers**

After the existing decode helpers, add:

```rust
/// Decode a dynamic uint8[] array from ABI-encoded data.
/// `offset` is the byte position of the offset pointer in the data.
fn decode_u8_array(data: &[u8], offset: usize) -> Result<Vec<u8>, PrecompileFailure> {
    let array_offset = decode_u256(data, offset)?
        .try_into()
        .map_err(|_| PrecompileFailure::Error {
            exit_status: ExitError::Other(Cow::Borrowed("ABI decode: offset overflow")),
        })?;
    let array_offset: usize = array_offset;
    let length = decode_u256(data, array_offset)?
        .try_into()
        .map_err(|_| PrecompileFailure::Error {
            exit_status: ExitError::Other(Cow::Borrowed("ABI decode: length overflow")),
        })?;
    let length: usize = length;
    let mut result = Vec::with_capacity(length);
    for i in 0..length {
        result.push(decode_u8(data, array_offset + 32 + i * 32)?);
    }
    Ok(result)
}

/// Decode a dynamic uint256[] array from ABI-encoded data.
fn decode_u256_array(data: &[u8], offset: usize) -> Result<Vec<U256>, PrecompileFailure> {
    let array_offset: usize = decode_u256(data, offset)?
        .try_into()
        .map_err(|_| PrecompileFailure::Error {
            exit_status: ExitError::Other(Cow::Borrowed("ABI decode: offset overflow")),
        })?;
    let length: usize = decode_u256(data, array_offset)?
        .try_into()
        .map_err(|_| PrecompileFailure::Error {
            exit_status: ExitError::Other(Cow::Borrowed("ABI decode: length overflow")),
        })?;
    let mut result = Vec::with_capacity(length);
    for i in 0..length {
        result.push(decode_u256(data, array_offset + 32 + i * 32)?);
    }
    Ok(result)
}

/// Decode a dynamic address[] array from ABI-encoded data.
fn decode_address_array(data: &[u8], offset: usize) -> Result<Vec<H160>, PrecompileFailure> {
    let array_offset: usize = decode_u256(data, offset)?
        .try_into()
        .map_err(|_| PrecompileFailure::Error {
            exit_status: ExitError::Other(Cow::Borrowed("ABI decode: offset overflow")),
        })?;
    let length: usize = decode_u256(data, array_offset)?
        .try_into()
        .map_err(|_| PrecompileFailure::Error {
            exit_status: ExitError::Other(Cow::Borrowed("ABI decode: length overflow")),
        })?;
    let mut result = Vec::with_capacity(length);
    for i in 0..length {
        result.push(decode_address(data, array_offset + 32 + i * 32)?);
    }
    Ok(result)
}
```

- [ ] **Step 3: Add `placeBets` handler**

In the `execute` function, add a new `else if` branch after the `place_bet` handler:

```rust
} else if sel == selector_place_bets() {
    handle.record_cost(GAS_COST_WRITE)?;

    // Decode three dynamic arrays: uint8[], uint256[], address[]
    // ABI layout: 3 offset pointers (at 0, 32, 64), then array data
    let numbers = decode_u8_array(&data, 0)?;
    let amounts_u256 = decode_u256_array(&data, 32)?;
    let token_addrs = decode_address_array(&data, 64)?;

    let len = numbers.len();

    let mut amounts: Vec<pallet_betting::pallet::BalanceOf<R>> = Vec::with_capacity(len);
    for val in &amounts_u256 {
        let a: u128 = val.try_into().map_err(|_| PrecompileFailure::Error {
            exit_status: ExitError::Other(Cow::Borrowed("Amount overflow")),
        })?;
        amounts.push(a.try_into().map_err(|_| PrecompileFailure::Error {
            exit_status: ExitError::Other(Cow::Borrowed("Amount conversion overflow")),
        })?);
    }

    let tokens: Vec<TokenId> = token_addrs.iter().map(|a| address_to_token_id(*a)).collect();

    let caller_h160 = handle.context().caller;
    let caller =
        <R as pallet_evm::Config>::AddressMapping::into_account_id(caller_h160);
    let origin: <R as frame_system::Config>::RuntimeOrigin =
        RawOrigin::Signed(caller).into();

    pallet_betting::Pallet::<R>::place_bets(origin, numbers.clone(), amounts.clone(), tokens.clone())
        .map_err(dispatch_error_to_precompile_failure)?;

    // Emit BetPlaced events for each bet
    let now_ms = <R as pallet_betting::Config>::TimeProvider::now().as_millis();
    let now_secs = (now_ms / 1000) as u64;
    let round_id = pallet_betting::round::current_round_id(now_secs);

    for i in 0..len {
        let mut log_data = Vec::with_capacity(96);
        log_data.extend_from_slice(&encode_u8(numbers[i]));
        log_data.extend_from_slice(&encode_address(token_addrs[i]));
        log_data.extend_from_slice(&encode_u256(amounts_u256[i]));
        handle
            .log(
                handle.code_address(),
                vec![
                    event_topic("BetPlaced(uint256,address,uint8,address,uint256)"),
                    u256_to_topic(U256::from(round_id)),
                    address_to_topic(caller_h160),
                ],
                log_data,
            )
            .map_err(|_| PrecompileFailure::Error {
                exit_status: ExitError::Other(Cow::Borrowed("Failed to emit log")),
            })?;
    }

    ok_empty()
```

- [ ] **Step 4: Add `updateBet` handler**

```rust
} else if sel == selector_update_bet() {
    handle.record_cost(GAS_COST_WRITE)?;

    let round_id = decode_u32_from_u256(&data, 0)? as RoundId;
    let number = decode_u8(&data, 32)?;
    let new_amount_u128 = decode_u128_from_u256(&data, 64)?;
    let token_addr = decode_address(&data, 96)?;

    let token = address_to_token_id(token_addr);
    let new_amount: pallet_betting::pallet::BalanceOf<R> =
        new_amount_u128
            .try_into()
            .map_err(|_| PrecompileFailure::Error {
                exit_status: ExitError::Other(Cow::Borrowed("Amount conversion overflow")),
            })?;

    let caller_h160 = handle.context().caller;
    let caller =
        <R as pallet_evm::Config>::AddressMapping::into_account_id(caller_h160);

    // Read old amount for event
    let old_amount_u128: u128 = pallet_betting::Bets::<R>::get((round_id, &caller, number))
        .map(|b| b.amount.into())
        .unwrap_or(0u128);

    let origin: <R as frame_system::Config>::RuntimeOrigin =
        RawOrigin::Signed(caller).into();

    pallet_betting::Pallet::<R>::update_bet(origin, round_id, number, new_amount, token)
        .map_err(dispatch_error_to_precompile_failure)?;

    // Emit BetUpdated(uint256 indexed roundId, address indexed user, uint8 number, address token, uint256 oldAmount, uint256 newAmount)
    let mut log_data = Vec::with_capacity(128);
    log_data.extend_from_slice(&encode_u8(number));
    log_data.extend_from_slice(&encode_address(token_addr));
    log_data.extend_from_slice(&encode_u128(old_amount_u128));
    log_data.extend_from_slice(&encode_u128(new_amount_u128));
    handle
        .log(
            handle.code_address(),
            vec![
                event_topic("BetUpdated(uint256,address,uint8,address,uint256,uint256)"),
                u256_to_topic(U256::from(round_id)),
                address_to_topic(caller_h160),
            ],
            log_data,
        )
        .map_err(|_| PrecompileFailure::Error {
            exit_status: ExitError::Other(Cow::Borrowed("Failed to emit log")),
        })?;

    ok_empty()
```

- [ ] **Step 5: Add `getBets` handler**

```rust
} else if sel == selector_get_bets() {
    handle.record_cost(GAS_COST_READ)?;

    let round_id = decode_u32_from_u256(&data, 0)? as RoundId;
    let user_addr = decode_address(&data, 32)?;
    let user_account =
        <R as pallet_evm::Config>::AddressMapping::into_account_id(user_addr);

    // Collect all bets for this user in this round
    let mut numbers_vec: Vec<u8> = Vec::new();
    let mut tokens_vec: Vec<H160> = Vec::new();
    let mut amounts_vec: Vec<u128> = Vec::new();
    let mut claimed_vec: Vec<bool> = Vec::new();

    // Iterate all numbers 0-99 and check if bet exists
    for n in 0u8..=99 {
        if let Some(bet) = pallet_betting::Bets::<R>::get((round_id, &user_account, n)) {
            numbers_vec.push(n);
            tokens_vec.push(token_id_to_address(bet.token));
            amounts_vec.push(bet.amount.into());
            claimed_vec.push(bet.claimed);
        }
    }

    let count = numbers_vec.len();

    // ABI encode: 4 dynamic arrays
    // Layout: 4 offset pointers, then 4 arrays
    // Each array: length (32 bytes) + length * 32 bytes of data
    let header_size = 4 * 32; // 4 offset pointers
    let array_size = |len: usize| -> usize { 32 + len * 32 };

    let offset0 = header_size;
    let offset1 = offset0 + array_size(count);
    let offset2 = offset1 + array_size(count);
    let offset3 = offset2 + array_size(count);

    let mut output = Vec::with_capacity(offset3 + array_size(count));

    // Offset pointers
    output.extend_from_slice(&encode_u256(U256::from(offset0)));
    output.extend_from_slice(&encode_u256(U256::from(offset1)));
    output.extend_from_slice(&encode_u256(U256::from(offset2)));
    output.extend_from_slice(&encode_u256(U256::from(offset3)));

    // numbers array
    output.extend_from_slice(&encode_u256(U256::from(count)));
    for &n in &numbers_vec {
        output.extend_from_slice(&encode_u8(n));
    }

    // tokens array
    output.extend_from_slice(&encode_u256(U256::from(count)));
    for &t in &tokens_vec {
        output.extend_from_slice(&encode_address(t));
    }

    // amounts array
    output.extend_from_slice(&encode_u256(U256::from(count)));
    for &a in &amounts_vec {
        output.extend_from_slice(&encode_u128(a));
    }

    // claimed array
    output.extend_from_slice(&encode_u256(U256::from(count)));
    for &c in &claimed_vec {
        output.extend_from_slice(&encode_bool(c));
    }

    ok_with_output(output)
```

- [ ] **Step 6: Update `getBet` handler for new signature**

The `getBet` selector changed from `getBet(uint256,address)` to `getBet(uint256,address,uint8)`. Update the handler:

```rust
} else if sel == selector_get_bet() {
    handle.record_cost(GAS_COST_READ)?;

    let round_id = decode_u32_from_u256(&data, 0)? as RoundId;
    let user_addr = decode_address(&data, 32)?;
    let number = decode_u8(&data, 64)?;

    let user_account =
        <R as pallet_evm::Config>::AddressMapping::into_account_id(user_addr);

    let mut output = Vec::with_capacity(128);

    match pallet_betting::Bets::<R>::get((round_id, &user_account, number)) {
        Some(bet) => {
            let amount_u128: u128 = bet.amount.into();
            output.extend_from_slice(&encode_u8(number));
            output.extend_from_slice(&encode_address(token_id_to_address(bet.token)));
            output.extend_from_slice(&encode_u128(amount_u128));
            output.extend_from_slice(&encode_bool(bet.claimed));
        }
        None => {
            output.extend_from_slice(&encode_u8(0));
            output.extend_from_slice(&encode_address(H160::zero()));
            output.extend_from_slice(&encode_u128(0));
            output.extend_from_slice(&encode_bool(false));
        }
    }

    ok_with_output(output)
```

- [ ] **Step 7: Update `claim_winnings` handler to pass `number` parameter**

In the `claim_winnings` handler, add number decoding and pass it to the pallet call. The new selector is `claimWinnings(uint256,uint8)`:

Update:
```rust
fn selector_claim_winnings() -> [u8; 4] {
    selector("claimWinnings(uint256,uint8)")
}
```

In the handler, decode the number:
```rust
let round_id = decode_u32_from_u256(&data, 0)? as RoundId;
let number = decode_u8(&data, 32)?;
```

Update the bet lookup:
```rust
let bet = pallet_betting::Bets::<R>::get((round_id, &caller, number));
```

Update the pallet call:
```rust
pallet_betting::Pallet::<R>::claim_winnings(origin, round_id, number)
```

- [ ] **Step 8: Verify compilation**

Run:
```bash
cd packages/node && cargo check -p precompile-betting
```

Expected: Compiles. If there are errors in precompile tests, they will be fixed in Task 6.

- [ ] **Step 9: Commit**

```bash
git add packages/node/precompiles/betting/src/lib.rs
git commit -m "feat(precompile-betting): add placeBets, updateBet, getBets

New EVM selectors for batch betting, bet updates, and multi-bet
read. Updated getBet and claimWinnings signatures for number param."
```

---

### Task 6: Update precompile tests

**Files:**
- Modify: `packages/node/precompiles/betting/src/tests.rs`

Update existing precompile tests for the new function signatures (getBet now takes number, claimWinnings takes number). Add basic tests for new selectors if the test infrastructure supports it.

- [ ] **Step 1: Read and update existing precompile tests**

Read `packages/node/precompiles/betting/src/tests.rs` to understand the test structure. Update all calls to use new ABI encoding for changed selectors.

Key changes:
- `getBet` calldata: append `encode_u8(number)` as third parameter
- `claimWinnings` calldata: append `encode_u8(number)` as second parameter
- Any `BetInfo` references: remove `number` field

- [ ] **Step 2: Run precompile tests**

Run:
```bash
cd packages/node && cargo test -p precompile-betting
```

Expected: All tests pass.

- [ ] **Step 3: Commit**

```bash
git add packages/node/precompiles/betting/src/tests.rs
git commit -m "test(precompile-betting): update tests for new function signatures"
```

---

### Task 7: Update Solidity interface

**Files:**
- Modify: `packages/contracts/contracts/interfaces/IBettingPrecompile.sol`

- [ ] **Step 1: Update the interface**

Replace the entire file content:

```solidity
// SPDX-License-Identifier: MIT
pragma solidity ^0.8.28;

interface IBettingPrecompile {
    // Write functions
    function placeBet(uint8 number, address token, uint256 amount) external;
    function placeBets(uint8[] calldata numbers, uint256[] calldata amounts, address[] calldata tokens) external;
    function updateBet(uint256 roundId, uint8 number, uint256 newAmount, address token) external;
    function submitResult(uint256 roundId, uint8 number) external;
    function claimWinnings(uint256 roundId, uint8 number) external;
    function adminClaimPool(uint256 roundId) external;
    function forceCloseRound(uint256 roundId) external;

    // Read functions
    function getCurrentRound() external view returns (
        uint256 roundId,
        uint256 closeTimestamp,
        uint8 status
    );

    function getBet(uint256 roundId, address user, uint8 number) external view returns (
        uint8 num,
        address token,
        uint256 amount,
        bool claimed
    );

    function getBets(uint256 roundId, address user) external view returns (
        uint8[] memory numbers,
        address[] memory tokens,
        uint256[] memory amounts,
        bool[] memory claimed
    );

    // Events
    event BetPlaced(uint256 indexed roundId, address indexed user, uint8 number, address token, uint256 amount);
    event BetUpdated(uint256 indexed roundId, address indexed user, uint8 number, address token, uint256 oldAmount, uint256 newAmount);
    event ResultSubmitted(uint256 indexed roundId, uint8 number);
    event WinningsClaimed(uint256 indexed roundId, address indexed user, address token, uint256 amount);
    event PoolClaimed(uint256 indexed roundId, address indexed admin, address token, uint256 amount);
}
```

- [ ] **Step 2: Compile Solidity to regenerate ABI**

Run:
```bash
cd packages/contracts && pnpm hardhat compile
```

Expected: Compiles without errors.

- [ ] **Step 3: Commit**

```bash
git add packages/contracts/contracts/interfaces/IBettingPrecompile.sol
git commit -m "feat(contracts): update IBettingPrecompile with placeBets, updateBet, getBets"
```

---

### Task 8: Update shared package ABI and types

**Files:**
- Modify: `packages/shared/src/types/betting.ts`
- Modify: `packages/shared/abis/IBettingPrecompile.json` (regenerated)

- [ ] **Step 1: Regenerate ABI**

Run:
```bash
cd packages/shared && pnpm copy-abi
```

This copies and extracts the ABI from the Hardhat compilation artifacts.

- [ ] **Step 2: Update BetInfo type**

Edit `packages/shared/src/types/betting.ts`:

```typescript
export enum RoundStatus {
  Open = 0,
  Closed = 1,
  Resolved = 2,
  Settled = 3,
}

export interface BetInfo {
  readonly token: string;
  readonly amount: bigint;
  readonly claimed: boolean;
}

export interface RoundInfo {
  readonly roundId: bigint;
  readonly closeTimestamp: bigint;
  readonly status: RoundStatus;
}
```

Remove the `number` field from `BetInfo`.

- [ ] **Step 3: Build shared package**

Run:
```bash
cd packages/shared && pnpm build
```

Expected: Compiles.

- [ ] **Step 4: Commit**

```bash
git add packages/shared/abis/IBettingPrecompile.json packages/shared/src/types/betting.ts
git commit -m "feat(shared): update ABI and types for multi-number betting"
```

---

### Task 9: Full build verification

- [ ] **Step 1: Run pallet tests**

Run:
```bash
cd packages/node && cargo test -p pallet-betting
```

Expected: All tests pass.

- [ ] **Step 2: Run precompile tests**

Run:
```bash
cd packages/node && cargo test -p precompile-betting
```

Expected: All tests pass.

- [ ] **Step 3: Build the full node**

Run:
```bash
cd packages/node && cargo build --release
```

Expected: Builds successfully.

- [ ] **Step 4: Build TypeScript packages**

Run:
```bash
cd /Users/huyduan/projects/blockchain && pnpm turbo build
```

Expected: All packages build.
