# Rounds, Betting & Admin Design

## Overview

Add multi-number betting, bet editing, and admin panel to the Artemis betting platform. Changes span Rust pallet + precompile, Solidity interface, shared ABI, indexer, and webapp frontend.

## Scope

| Layer | Changes |
|-------|---------|
| Pallet (Rust) | Re-key `Bets` storage for multi-number, add `place_bets`, `update_bet` |
| Precompile (Rust) | Expose new pallet functions to EVM |
| Solidity interface | Add `placeBets`, `updateBet`, `getBets`; update `getBet` signature |
| Shared (TS) | Update ABI, types |
| Indexer | Handle `BetUpdated` event, support amount=0 deletions |
| Webapp (TS) | Rounds list, round detail + bet form, admin panel |

## Pallet Changes

### Storage: Multi-number betting

Current `Bets` storage uses `(RoundId, AccountId)` key -- limits each user to 1 bet per round.

Change to triple key `(RoundId, AccountId, u8)` so a user can bet on multiple numbers in the same round:

```
// Before
Bets<T> = StorageDoubleMap<RoundId, AccountId, BetInfo>

// After
Bets<T> = StorageNMap<(RoundId, AccountId, u8), BetInfo>
```

`BetInfo` struct changes:
- Remove `number` field (now part of the storage key)
- Keep: `token`, `amount`, `claimed`

### New function: `place_bets`

```rust
pub fn place_bets(
    origin: OriginFor<T>,
    numbers: Vec<u8>,
    amounts: Vec<BalanceOf<T>>,
    tokens: Vec<TokenId>,
) -> DispatchResult
```

- Gasless (`Pays::No`)
- Validates: all arrays same length, each number 0-99, each amount > 0, each token supported
- Validates: no duplicate numbers in the input array
- Validates: user has not already bet on any of these numbers in this round
- Transfers total amount per token to pallet account
- Inserts one `BetInfo` per number
- Emits one `BetPlaced` event per number

### New function: `update_bet`

```rust
pub fn update_bet(
    origin: OriginFor<T>,
    round_id: RoundId,
    number: u8,
    new_amount: BalanceOf<T>,
    token: TokenId,
) -> DispatchResult
```

- Dynamic gas: `Pays::No` if `new_amount >= old_amount`, `Pays::Yes` if `new_amount < old_amount`
- Only when round status is `Open`
- If `new_amount > old_amount`: transfer difference from user to pallet
- If `new_amount < old_amount`: refund difference from pallet to user
- If `new_amount == 0`: refund full amount, remove bet from storage
- Token must match existing bet's token
- Emits `BetUpdated` event

### New event: `BetUpdated`

```rust
Event::BetUpdated {
    round_id: RoundId,
    who: T::AccountId,
    number: u8,
    token: TokenId,
    old_amount: BalanceOf<T>,
    new_amount: BalanceOf<T>,
}
```

When `new_amount == 0`, this event signals a cancellation (bet removed).

### Updated function: `place_bet`

Keep existing `place_bet` for single-number backward compatibility. Update to use new `StorageNMap` key `(round_id, who, number)`. Change `AlreadyBet` check from per-user to per-user-per-number.

### Updated function: `getBet`

Add `number: u8` parameter to look up specific bet:

```rust
pub fn get_bet(round_id: RoundId, user: T::AccountId, number: u8) -> Option<BetInfo>
```

### New read function: `get_bets`

Return all bets for a user in a round. Iterates `Bets` prefix `(round_id, user)`.

### Anti-spam design

| Function | Gas |
|----------|-----|
| `placeBet` | Gasless (`Pays::No`) |
| `placeBets` | Gasless (`Pays::No`) |
| `updateBet` (increase or same amount) | Gasless (`Pays::No`) |
| `updateBet` (decrease or remove) | Paid (`Pays::Yes`) |
| `submitResult` | Paid (admin) |
| `adminClaimPool` | Paid (admin) |
| `forceCloseRound` | Paid (admin) |
| `claimWinnings` | Gasless (`Pays::No`) |

Rationale: placing and increasing bets is encouraged (gasless). Decreasing/removing costs gas to prevent bet/cancel spam loops.

## Precompile Changes

Expose new pallet functions to EVM at precompile address `0x0000000000000000000000000000000000000801`.

New selectors:
- `placeBets(uint8[],uint256[],address[])`
- `updateBet(uint256,uint8,uint256,address)`
- `getBets(uint256,address)` returns `(uint8[],address[],uint256[],bool[])`

Updated selectors:
- `getBet(uint256,address,uint8)` -- added `number` parameter

## Solidity Interface

```solidity
interface IBettingPrecompile {
    // Existing
    function placeBet(uint8 number, address token, uint256 amount) external;
    function submitResult(uint256 roundId, uint8 number) external;
    function claimWinnings(uint256 roundId) external;
    function adminClaimPool(uint256 roundId) external;
    function forceCloseRound(uint256 roundId) external;
    function getCurrentRound() external view returns (uint256 roundId, uint256 closeTimestamp, uint8 status);

    // Updated signature
    function getBet(uint256 roundId, address user, uint8 number) external view returns (uint8 num, address token, uint256 amount, bool claimed);

    // New
    function placeBets(uint8[] calldata numbers, uint256[] calldata amounts, address[] calldata tokens) external;
    function updateBet(uint256 roundId, uint8 number, uint256 newAmount, address token) external;
    function getBets(uint256 roundId, address user) external view returns (uint8[] memory numbers, address[] memory tokens, uint256[] memory amounts, bool[] memory claimed);

    // Existing events
    event BetPlaced(uint256 indexed roundId, address indexed user, uint8 number, address token, uint256 amount);
    event ResultSubmitted(uint256 indexed roundId, uint8 number);
    event WinningsClaimed(uint256 indexed roundId, address indexed user, address token, uint256 amount);
    event PoolClaimed(uint256 indexed roundId, address indexed admin, address token, uint256 amount);

    // New event
    event BetUpdated(uint256 indexed roundId, address indexed user, uint8 number, address token, uint256 oldAmount, uint256 newAmount);
}
```

## Shared Package

Update `packages/shared`:
- Regenerate ABI from updated Solidity interface
- Update `BetInfo` type: remove `number` field (now a separate key)
- Add `BetUpdated` event type

## Indexer Updates

### New event handler: `BetUpdated`

```
on BetUpdated:
  if newAmount == 0:
    delete bet record (roundId, user, number)
    decrement round.totalBets
    subtract oldAmount from round.totalVolume
  else:
    update bet record amount
    adjust round.totalVolume by (newAmount - oldAmount)
  update userStats.totalWagered
  update dailyStats, protocolStats
```

### Schema changes

- `bet.id` changes from `${roundId}-${user}` to `${roundId}-${user}-${number}` to support multi-number

## Frontend

### `/rounds` -- Rounds page

**Data sources:**
- Current round: `getCurrentRound()` via wagmi `useReadContract`
- User's bets in current round: `getBets(roundId, user)` via wagmi
- Past rounds + results: indexer GraphQL query on `round` table
- User's bet history: indexer GraphQL query on `bet` table filtered by user

**UI:**
- Current round card: round ID, status badge, close timestamp with countdown timer
- User's active bets list (if connected): number, amount, token, edit button
- Past rounds table: round ID, status, winning number, total volume, user's result (won/lost/no bet)

### `/rounds/:id` -- Round Detail + Place Bet

**Place bet form:**
- Number grid (10x10, numbers 0-99): click to select/deselect numbers
- Selected numbers shown as chips with amount + token input each
- Submit: calls `placeBets` if multiple numbers, `placeBet` if single
- Only visible when round is Open and wallet is connected

**Bet management (existing bets):**
- List user's bets in this round
- Each bet shows: number, amount, token
- Edit button: inline edit amount, calls `updateBet`
- Remove button: calls `updateBet` with amount=0

**Round info:**
- Round ID, status, close timestamp
- Winning number (if Resolved)
- Total bets, total volume (from indexer)

**Claim winnings:**
- Visible when round is Resolved and user has winning bet(s) not yet claimed
- Button calls `claimWinnings(roundId)`

### `/admin` -- Admin Panel

**Wallet guard:** Only show admin actions when connected wallet is the sudo account (`0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266`).

**Submit Result:**
- Select/input round ID (default: current round)
- Input winning number (0-99)
- Calls `submitResult(roundId, number)`

**Admin Claim Pool:**
- Select/input round ID
- Shows pool balance if available (from indexer)
- Calls `adminClaimPool(roundId)`

**Force Close Round:**
- Select/input round ID
- Confirmation dialog before executing
- Calls `forceCloseRound(roundId)`

### Wagmi Hooks

New custom hooks wrapping wagmi:
- `useCurrentRound()` -- reads `getCurrentRound`, auto-refetch on block
- `useUserBets(roundId)` -- reads `getBets` for connected user
- `usePlaceBet()` / `usePlaceBets()` -- write hooks
- `useUpdateBet()` -- write hook
- `useClaimWinnings()` -- write hook
- `useSubmitResult()` -- write hook (admin)
- `useAdminClaimPool()` -- write hook (admin)
- `useForceCloseRound()` -- write hook (admin)

### Indexer GraphQL Queries

- `rounds(orderBy: "id", orderDirection: "desc", limit: 20)` -- rounds list
- `round(id: roundId)` -- single round
- `bets(where: { user: address }, orderBy: "timestamp", orderDirection: "desc")` -- user bet history
- `bets(where: { roundId: id })` -- all bets in a round

## E2E Tests

### Test: Place single bet
1. Connect wallet, navigate to current round
2. Select number, enter amount
3. Submit, approve tx
4. Verify bet appears in user's bets list

### Test: Place multiple bets
1. Select multiple numbers with different amounts
2. Submit `placeBets`, approve tx
3. Verify all bets appear

### Test: Update bet (increase)
1. Place bet, then edit amount upward
2. Verify updated amount displayed

### Test: Update bet (remove, amount=0)
1. Place bet, then remove it
2. Verify bet removed from list

### Test: Admin submit result
1. Connect with admin wallet
2. Submit result for a round
3. Verify round status changes to Resolved

### Test: Claim winnings
1. Place winning bet, admin submits matching result
2. Claim winnings
3. Verify balance increased

## Out of Scope

- Gasless pallet (separate spec)
- Real-time WebSocket subscriptions for live updates
- Mobile-responsive optimization
- Production deployment
