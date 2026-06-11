# Betting Blockchain Monorepo — Design Spec

## Overview

A Substrate-based blockchain for betting applications with EVM compatibility. MVP is a daily predict-number game (0-99). Betting logic lives in a custom Rust pallet with gasless transactions (`Pays::No`), exposed to EVM via custom precompiles so the frontend can use viem/Wagmi.

## Goals

- Custom Substrate solochain with Frontier EVM support
- Gasless betting via `Pays::No` at pallet level
- EVM-accessible betting through custom precompiles
- Multi-token support (native + ERC-20/assets pallet tokens)
- MVP: daily predict-number game
- Future: sports betting, prediction markets, casino-style games

## Monorepo Structure

```
blockchain/
  pnpm-workspace.yaml
  turbo.json
  package.json

  packages/
    node/                        — Substrate solochain (Rust/Cargo workspace)
      node/                      — Node binary
      runtime/                   — FRAME runtime + Frontier EVM + custom pallets
      pallets/
        betting/                 — Core betting pallet (Pays::No)
      precompiles/
        betting/                 — EVM precompile bridge to betting pallet

    contracts/                   — Hardhat project
      contracts/
        interfaces/
          IBettingPrecompile.sol — Solidity interface for precompile
        mocks/
          MockBetting.sol        — Mock for unit tests
      test/
        BettingPrecompile.test.ts
      hardhat.config.ts

    shared/                      — Shared TypeScript package
      abis/                      — Precompile ABI (auto-generated from interface)
      types/                     — Shared TypeScript types
      constants/                 — Addresses, chain config

    webapp/                      — Vite + React (post-MVP)
      src/
        hooks/
        components/
        config/
```

### Workspace boundaries

- `packages/node/` is a Rust/Cargo workspace, not a pnpm package. Turborepo orchestrates it via build scripts only.
- `contracts/`, `shared/`, and `webapp/` are TypeScript packages in pnpm workspace.
- `shared/` is depended on by both `contracts/` (for tests) and `webapp/`.

## Tooling

| Tool | Purpose |
|------|---------|
| pnpm workspaces | Package management |
| Turborepo | Build orchestration, caching, parallel builds |
| Hardhat | Contract testing, precompile integration tests |
| Cargo | Substrate node, pallets, precompiles |

## Substrate Node (`packages/node/`)

### Base

- Fork from **Frontier solochain template** ([polkadot-evm/frontier/template](https://github.com/polkadot-evm/frontier/tree/master/template))
- Standard pallets: `pallet-balances`, `pallet-sudo`, `pallet-timestamp`, `pallet-transaction-payment`
- Frontier pallets: `pallet-evm`, `pallet-ethereum`, `pallet-base-fee`
- Custom pallets: `pallet-betting`

### Local development

- Build: `cargo build --release`
- Run: single-node dev mode (`--dev`)
- EVM RPC: `http://localhost:9944`

## Betting Pallet (`pallets/betting/`)

### Design

Core betting logic for the predict-number game. All dispatchable calls related to betting are feeless for users via `Pays::No`.

### Storage

- `Rounds`: `StorageMap<RoundId, RoundInfo>` — round metadata (status, close timestamp, result)
- `Bets`: `StorageDoubleMap<RoundId, AccountId, BetInfo>` — user bets per round
- `SupportedTokens`: `StorageMap<TokenId, bool>` — enabled tokens for betting
- `Admin`: `StorageValue<AccountId>` — admin account

### Types

```rust
struct RoundInfo {
    close_timestamp: u64,        // 18:00 GMT+7 daily
    status: RoundStatus,         // Open, Closed, Resolved, Settled
    result: Option<u8>,          // 0-99, set by admin
    total_pool: BTreeMap<TokenId, Balance>,
}

struct BetInfo {
    number: u8,                  // 0-99
    token: TokenId,
    amount: Balance,
}

enum RoundStatus {
    Open,
    Closed,
    Resolved,
    Settled,
}
```

### Dispatchable calls

| Call | Who | Fee | Description |
|------|-----|-----|-------------|
| `place_bet(number, token, amount)` | Any user | `Pays::No` | Place bet on current round. Round determined by `block.timestamp` vs 18:00 GMT+7 cutoff |
| `submit_result(round_id, number)` | Admin | `Pays::Yes` | Submit winning number for a closed round |
| `claim_winnings(round_id)` | Winner | `Pays::No` | Claim proportional share of pool |
| `add_supported_token(token_id)` | Admin | `Pays::Yes` | Enable a token for betting |
| `remove_supported_token(token_id)` | Admin | `Pays::Yes` | Disable a token |

### Round lifecycle

```
Auto-determined by block.timestamp:

Before 18:00 GMT+7  →  Current day round is OPEN
                        Users call place_bet()

After 18:00 GMT+7   →  Current day round is CLOSED (auto, no tx needed)
                        place_bet() goes to next day's round

Admin calls          →  submit_result(round_id, number)
                        Round becomes RESOLVED

Winners call         →  claim_winnings(round_id)
                        After all claims or timeout → SETTLED
```

### Multi-token

- Native token: direct balance transfer
- Asset pallet tokens: via `pallet-assets` integration
- Each token has independent pool per round
- Winnings distributed proportionally within same token pool

### Winning logic

- **Payout rate**: winners receive `1 x N` where N is the configurable multiplier (e.g. N=90 means bet 1 token, win 90 tokens)
- Payout comes from the total pool for that token
- If pool insufficient for full payout: winners split available pool proportionally
- **No winner**: admin can claim the entire pool for that round
- **Admin margin**: pool balance after paying winners goes to admin (house edge)

## Betting Precompile (`precompiles/betting/`)

### Purpose

Bridge between EVM and the betting pallet. Exposes pallet functions at a fixed EVM address so viem/Wagmi can interact directly.

### Address

`0x0000000000000000000000000000000000000801` (configurable in runtime)

### Solidity Interface

```solidity
interface IBettingPrecompile {
    /// Place a bet on the current round
    /// @param number The predicted number (0-99)
    /// @param token Token address (address(0) for native)
    /// @param amount Bet amount
    function placeBet(uint8 number, address token, uint256 amount) external;

    /// Submit the winning number for a round (admin only)
    /// @param roundId The round identifier
    /// @param number The winning number (0-99)
    function submitResult(uint256 roundId, uint8 number) external;

    /// Claim winnings for a resolved round
    /// @param roundId The round identifier
    function claimWinnings(uint256 roundId) external;

    /// Get current round info
    function getCurrentRound() external view returns (
        uint256 roundId,
        uint256 closeTimestamp,
        uint8 status
    );

    /// Get user bet for a round
    function getBet(uint256 roundId, address user) external view returns (
        uint8 number,
        address token,
        uint256 amount
    );

    /// Events
    event BetPlaced(uint256 indexed roundId, address indexed user, uint8 number, address token, uint256 amount);
    event ResultSubmitted(uint256 indexed roundId, uint8 number);
    event WinningsClaimed(uint256 indexed roundId, address indexed user, address token, uint256 amount);
}
```

### Implementation

- Rust precompile implementation using Frontier's precompile framework
- Maps Solidity function selectors to pallet dispatchable calls
- Handles EVM address ↔ Substrate AccountId conversion
- Gasless: precompile dispatches with `Pays::No` origin

## Testing Strategy

### Rust unit tests (`pallets/betting/`)

- Round lifecycle (open → closed → resolved → settled)
- Bet placement validation (number range, supported token, sufficient balance)
- Winning calculation and distribution
- Admin-only access control
- Timestamp-based round determination
- Edge cases: no winners, single winner, multiple winners

### Hardhat integration tests (`packages/contracts/`)

- Precompile callable from EVM (viem/ethers)
- Full flow: place bet → submit result → claim winnings
- Multi-token betting through precompile
- Error cases: bet on closed round, non-admin submit, etc.
- Runs against local Substrate node with `--dev`

## Security Considerations

- `Pays::No` abuse: rate limit bets per user per round (1 bet per user per round)
- Admin is trusted entity — no anti-cheat mechanism for result submission
- Token transfer validation: ensure user has sufficient balance before accepting bet
- Round close enforcement: `block.timestamp` based, no manual intervention needed
- Precompile input validation: number must be 0-99, amount must be > 0

## Out of Scope (Post-MVP)

- Frontend (webapp) with Vite + React + viem/Wagmi
- Relayer service (not needed due to Pays::No)
- Sports betting, prediction markets, casino games
- Oracle pallet for external data feeds
- Custom governance for admin rotation
- Multi-sig admin
- Chainlink VRF for randomness
- PAPI integration
