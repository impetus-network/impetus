# Lottery Betting Smart Contracts

## Overview

Permissionless lottery betting system inspired by Vietnamese lo de (number betting).
Singleton architecture following Uniswap V4 patterns: single LotteryManager contract,
pluggable hooks for extensibility, flash accounting for batch operations, and
owner/controller fee separation.

Deployable on any EVM chain. Not Artemis-specific.

## Domain

Five bet types derived from Vietnamese lottery betting:

| Type | Input | Win condition | Payout model |
|------|-------|---------------|--------------|
| DE | 1 number (00-99) | Matches last 2 digits of special prize | Fixed rate (e.g. 1:70) |
| LO | 1 number (00-99) | Matches last 2 digits of any prize | Per occurrence (e.g. 1:3 per hit) |
| XIEN2 | 2 numbers | Both appear in results | Fixed rate (e.g. 1:15) |
| XIEN3 | 3 numbers | All 3 appear in results | Fixed rate (e.g. 1:40) |
| XIEN4 | 4 numbers | All 4 appear in results | Fixed rate (e.g. 1:100) |

Lottery result structure: 1 special prize (2-digit) + 27 prize entries (2-digit each),
mirroring Northern Vietnam lottery (XSKTMB) format. The special prize is also included
as the first entry in allPrizes, so LO and XIEN bets can match against it.

## Architecture

### Singleton LotteryManager (Uniswap V4 pattern)

One contract manages all games and rounds. No factory pattern, no per-game deployments.

```
LotteryManager (singleton)
  ├── Game registry         GameKey → GameId → GameState
  ├── Round state           GameId → roundId → RoundState
  ├── Bet storage           GameId → roundId → bettor → Bet[]
  ├── Protocol fees         token → accrued amount
  ├── Hooks dispatch        Per-game pluggable logic
  └── Unlock/callback       Flash accounting for batch ops

IResultSource (pluggable)
  ├── VRFResultSource       On-chain random (MVP)
  └── OracleResultSource    External lottery feed (phase 2)

ILotteryHooks (pluggable per game)
  └── Custom logic at each lifecycle point

ILotteryFeeController (pluggable)
  └── Protocol fee configuration + collection
```

Benefits of singleton over factory:
- Gas efficiency: no cross-contract calls for shared state
- Shared token accounting across games
- Hooks upgradeable per game without state migration
- Single approval per token covers all games

### GameKey and GameId

```solidity
struct PayoutRates {
    uint16 de;       // e.g. 7000 = 70x
    uint16 lo;       // e.g. 300 = 3x (per occurrence)
    uint16 xien2;    // e.g. 1500 = 15x
    uint16 xien3;    // e.g. 4000 = 40x
    uint16 xien4;    // e.g. 10000 = 100x
}

struct GameKey {
    address token;              // address(0) = native token
    IResultSource resultSource; // VRF or oracle contract
    uint8 poolMode;             // 0 = bookmaker, 1 = parimutuel
    uint256 interval;           // 0 = one-shot, >0 = recurring (seconds)
    uint256 bettingWindow;      // seconds before lock that betting opens
    PayoutRates rates;          // payout multipliers (ignored in parimutuel)
    ILotteryHooks hooks;        // address(0) = no hooks
}

type GameId is bytes32;
// GameId = keccak256(abi.encode(GameKey))
```

Anyone calls `initialize(GameKey)` to create a game. Reverts if GameId already exists.

### Pool Modes

**Bookmaker mode (poolMode = 0):**
- Game creator deposits funds as prize pool
- Creator can top-up or withdraw excess anytime between rounds
- `placeBet()` calculates worst-case payout and rejects if pool insufficient
- After settlement: winnings paid from pool, losing bets added to pool
- Creator bears the risk and profit

**Parimutuel mode (poolMode = 1):**
- No creator deposit required
- All bet amounts go into round pool
- After settlement: pool distributed to winners proportionally
- PayoutRates in GameKey ignored; payout = (winner share / total winners) * pool
- No one bears directional risk

### Bookmaker exposure tracking

For each active round in bookmaker mode, track worst-case exposure:

- DE: `rate * amount` (single outcome)
- LO: `maxOccurrences * rate * amount` where maxOccurrences = 27 (all prizes)
- XIEN2/3/4: `rate * amount` (single outcome: all numbers present or not)

On `placeBet()`: `requiredPool += worstCasePayout`. Revert if `pool.balance < totalExposure`.

## Round Lifecycle

### States

```
OPEN → LOCKED → SETTLED
                  │
         CANCELLED (refund path, from OPEN or LOCKED)
```

| State | Description |
|-------|-------------|
| OPEN | Accepting bets |
| LOCKED | Betting closed, awaiting result from result source |
| SETTLED | Result received, winners can claim |
| CANCELLED | Round cancelled, all bets refundable |

### Time-based round derivation (recurring games)

Rounds exist implicitly based on timestamps. No explicit creation needed.

```solidity
function getRoundId(GameId gameId, uint256 timestamp) public view returns (uint256) {
    GameState storage game = games[gameId];
    return (timestamp - game.startTime) / game.interval;
}

function getLockTime(GameId gameId, uint256 roundId) public view returns (uint256) {
    GameState storage game = games[gameId];
    return game.startTime + (roundId + 1) * game.interval;
}

function getBettingOpensTime(GameId gameId, uint256 roundId) public view returns (uint256) {
    return getLockTime(gameId, roundId) - game.bettingWindow;
}
```

When a user calls `placeBet()`, the contract derives `roundId = getRoundId(block.timestamp)`.
If `block.timestamp >= lockTime` for that round, the bet goes to the next round automatically.

Multiple rounds can be LOCKED simultaneously awaiting results. Each round has independent state.

### One-shot rounds (interval = 0)

Creator specifies explicit `lockTime` via a separate `initializeOneShot(GameKey, uint256 lockTime)`
function. Single round (roundId = 0), no recurrence. The `bettingWindow` in GameKey still
applies — betting opens at `lockTime - bettingWindow`.

### Cancellation

- Creator can cancel a round if not yet SETTLED
- Auto-cancel if result source does not return within a configurable timeout
- All bets refundable via `claim()` on cancelled rounds

## Hooks (Uniswap V4 pattern)

### Interface

```solidity
interface ILotteryHooks {
    function beforeInitialize(address creator, GameKey calldata key)
        external returns (bytes4);
    function afterInitialize(address creator, GameKey calldata key, GameId gameId)
        external returns (bytes4);

    function beforePlaceBet(GameId gameId, address bettor, BetParams calldata params)
        external returns (bytes4);
    function afterPlaceBet(GameId gameId, address bettor, BetParams calldata params)
        external returns (bytes4);

    function beforeRoundLock(GameId gameId, uint256 roundId)
        external returns (bytes4);
    function afterRoundSettle(GameId gameId, uint256 roundId, LotteryResult calldata result)
        external returns (bytes4);

    function beforeClaim(GameId gameId, address claimer, uint256 roundId)
        external returns (bytes4);
    function afterClaim(GameId gameId, address claimer, uint256 roundId, uint256 payout)
        external returns (bytes4);
}
```

Each hook returns its own selector to confirm success (same pattern as V4).
LotteryManager skips hooks that are not flagged as active.

### Permission flags via address bits

Following V4, the hooks contract address encodes which hooks are active:

```
bit 15: beforeInitialize
bit 14: afterInitialize
bit 13: beforePlaceBet
bit 12: afterPlaceBet
bit 11: beforeRoundLock
bit 10: afterRoundSettle
bit 9:  beforeClaim
bit 8:  afterClaim
```

LotteryManager validates on `initialize()` that the hooks address flags match the
functions actually implemented. This avoids storage reads to check which hooks are active.

### Example hook use cases

- **Whitelist/KYC:** `beforePlaceBet` checks bettor against allowlist
- **Referral rewards:** `afterPlaceBet` records referrer, `afterClaim` distributes bonus
- **Max bet limits:** `beforePlaceBet` enforces per-bettor caps
- **Loyalty tiers:** `afterClaim` updates bettor tier, `beforePlaceBet` applies tier-based limits
- **Custom settlement:** `afterRoundSettle` triggers additional actions (airdrops, NFT minting)
- **Cooldown periods:** `beforePlaceBet` enforces minimum time between bets

## Unlock / Flash Accounting (Uniswap V4 pattern)

### Mechanism

```solidity
function unlock(bytes calldata data) external returns (bytes memory) {
    // 1. Set transient lock flag
    // 2. Callback: IUnlockCallback(msg.sender).unlockCallback(data)
    // 3. Verify all currency deltas net to zero
    // 4. Clear lock
}

interface IUnlockCallback {
    function unlockCallback(bytes calldata data) external returns (bytes memory);
}
```

All state-changing operations (bet, claim, deposit, withdraw) require being inside
an `unlock()` context. Token transfers are deferred — only net amounts transfer at
the end.

### Benefits

- **Batch bets:** place multiple bets across games in 1 tx, single token transfer
- **Batch claims:** claim winnings from multiple rounds in 1 tx
- **Atomic bet+claim:** claim old round winnings and re-bet in same tx
- **Gas savings:** only net token deltas transfer, reducing ERC20 calls

### Delta tracking

```solidity
// Transient storage (EIP-1153)
mapping(address caller => mapping(address token => int256)) currencyDelta;
```

- `placeBet()` increases caller's negative delta (owes tokens)
- `claim()` increases caller's positive delta (owed tokens)
- At end of `unlock()`, each delta must be zero (settled via take/settle helpers)

## Protocol Fee (Uniswap V4 pattern)

### Governance separation

```
Owner (governance / multisig)
  └→ setFeeController(address)    // only power: swap controller

FeeController (operational)
  ├→ fetchFee(GameKey) → uint16   // returns fee in basis points
  └→ collectProtocolFees(recipient, token, amount)  // withdraw accrued fees
```

Owner cannot collect fees. Controller cannot replace itself. Compromised controller
key: owner revokes. Compromised owner key: controller funds are safe.

### Fee mechanics

```solidity
uint16 constant MAX_PROTOCOL_FEE = 1000; // 10% cap

mapping(address token => uint256) public protocolFeesAccrued;
```

Fee deducted during settlement:
- Bookmaker mode: fee taken from losing bet amounts before they enter creator pool
- Parimutuel mode: fee taken from total pool before distribution to winners

Default fee is 0 until controller sets it. If controller reverts or returns > MAX,
fee falls back to 0.

### ILotteryFeeController

```solidity
interface ILotteryFeeController {
    function fetchFee(GameKey calldata key) external view returns (uint16 feeBps);
}
```

LotteryManager calls this during settlement. Controller implementation decides fee
per game based on any criteria (token, pool mode, hooks address, etc.).

## Result Sources

### Interface

```solidity
struct LotteryResult {
    uint8 specialPrize;   // 00-99, last 2 digits of special prize
    uint8[] allPrizes;    // 00-99 each, last 2 digits of all 27 prizes
}

interface IResultSource {
    function requestResult(uint256 roundId) external;
    function getResult(uint256 roundId) external view returns (LotteryResult memory);
    function hasResult(uint256 roundId) external view returns (bool);
}
```

### Deployment models

The `IResultSource` interface enables two deployment models:

**Model 1 — Single-chain (e.g. Base mainnet):**
Betting contract and VRF on the same chain. `ChainlinkVRFResultSource` implements
`IResultSource` directly. No cross-chain infrastructure needed.

**Model 2 — Cross-chain (e.g. betting on Artemis, VRF on Base):**
VRF runs on Base (Chainlink VRF v2.5). Results relayed to Artemis via OZ Relayer.
`CrossChainResultReceiver` on Artemis implements `IResultSource`.

LotteryManager only knows `IResultSource` — same contract works in both models.

### ChainlinkVRFResultSource (Base mainnet)

Chainlink VRF v2.5 subscription model. Deployed on Base (or any chain with
Chainlink VRF support).

```solidity
contract ChainlinkVRFResultSource is VRFConsumerBaseV2Plus, IResultSource {
    // VRF config
    bytes32 public immutable i_keyHash;        // gas lane
    uint256 public immutable i_subscriptionId;
    uint16 public constant REQUEST_CONFIRMATIONS = 3;
    uint32 public constant CALLBACK_GAS_LIMIT = 300_000;
    uint32 public constant NUM_WORDS = 2;      // 512 bits, enough for 28 values

    // roundId → requestId mapping
    mapping(uint256 => uint256) public roundRequests;
    // requestId → result
    mapping(uint256 => LotteryResult) internal results;
    mapping(uint256 => bool) public fulfilled;

    function requestResult(uint256 roundId) external {
        uint256 requestId = s_vrfCoordinator.requestRandomWords(
            VRFV2PlusClient.RandomWordsRequest({
                keyHash: i_keyHash,
                subId: i_subscriptionId,
                requestConfirmations: REQUEST_CONFIRMATIONS,
                callbackGasLimit: CALLBACK_GAS_LIMIT,
                numWords: NUM_WORDS,
                extraArgs: VRFV2PlusClient._argsToBytes(
                    VRFV2PlusClient.ExtraArgsV1({nativePayment: false})
                )
            })
        );
        roundRequests[roundId] = requestId;
    }

    function fulfillRandomWords(
        uint256 requestId,
        uint256[] calldata randomWords
    ) internal override {
        // MUST NOT revert — Chainlink will not retry
        // Derive 28 values from 2 random words using sequential hashing
        uint8 specialPrize = uint8(uint256(keccak256(abi.encode(randomWords[0], 0))) % 100);
        uint8[] memory allPrizes = new uint8[](27);
        for (uint256 i = 0; i < 27; i++) {
            allPrizes[i] = uint8(uint256(keccak256(abi.encode(randomWords[0], randomWords[1], i + 1))) % 100);
        }
        results[requestId] = LotteryResult(specialPrize, allPrizes);
        fulfilled[requestId] = true;
        emit ResultFulfilled(requestId, specialPrize, allPrizes);
    }

    function getResult(uint256 roundId) external view returns (LotteryResult memory) { ... }
    function hasResult(uint256 roundId) external view returns (bool) { ... }
}
```

Key details:
- 2 random words (512 bits) provide sufficient entropy for 28 values mod 100
- `fulfillRandomWords` must never revert — Chainlink does not retry failed callbacks
- Anyone can call `requestResult()` — permissionless settlement trigger
- Subscription funded with LINK or native token (Base ETH)

### Cross-chain architecture (Base → Artemis)

```
Base                              OZ Infra (self-hosted)              Artemis
┌───────────────────────┐     ┌─────────────────────────┐     ┌──────────────────────────┐
│ ChainlinkVRFResult    │     │ oz-monitor              │     │ CrossChainResultReceiver │
│   Source              │     │   watch Base for         │     │   implements IResultSource│
│                       │     │   ResultFulfilled events │     │                          │
│ requestResult()       │     │         │                │     │ submitResult(             │
│   → Chainlink VRF     │     │         ▼                │     │   roundId,               │
│   → fulfillRandom()   │     │ oz-relayer               │     │   result,                │
│   → emit Result       │─────│   sign result payload    │─────│   proof                  │
│     Fulfilled()       │     │   submit tx to Artemis   │     │ )                        │
└───────────────────────┘     │   managed nonce/gas/retry│     │   → verifier.verify()    │
                              └─────────────────────────┘     │   → store result         │
                                                              └──────────────────────────┘
```

**Flow:**
1. Round LOCKED on Artemis
2. OZ Monitor (or anyone) calls `requestResult(roundId)` on Base VRFResultSource
3. Chainlink VRF fulfills → emits `ResultFulfilled(roundId, specialPrize, allPrizes)`
4. OZ Monitor detects event, forwards to OZ Relayer
5. OZ Relayer signs `(roundId, result, sourceChainId)` and submits to Artemis
6. `CrossChainResultReceiver.submitResult()` verifies signature via `IResultVerifier`
7. LotteryManager reads result via `IResultSource` interface — unaware of cross-chain

### CrossChainResultReceiver (Artemis)

```solidity
interface IResultVerifier {
    function verify(
        uint256 roundId,
        LotteryResult calldata result,
        bytes calldata proof
    ) external view returns (bool);
}

contract CrossChainResultReceiver is IResultSource {
    IResultVerifier public verifier;
    address public owner;

    mapping(uint256 => LotteryResult) internal results;
    mapping(uint256 => bool) public fulfilled;

    function setVerifier(IResultVerifier newVerifier) external onlyOwner;

    function submitResult(
        uint256 roundId,
        LotteryResult calldata result,
        bytes calldata proof
    ) external {
        require(!fulfilled[roundId], "already fulfilled");
        require(verifier.verify(roundId, result, proof), "invalid proof");
        results[roundId] = result;
        fulfilled[roundId] = true;
        emit ResultReceived(roundId);
    }

    function requestResult(uint256 roundId) external {
        // No-op on Artemis side. Request originates on Base via OZ Monitor.
        // Emits event for OZ Monitor to pick up and trigger Base-side request.
        emit ResultRequested(roundId);
    }

    function getResult(uint256 roundId) external view returns (LotteryResult memory) { ... }
    function hasResult(uint256 roundId) external view returns (bool) { ... }
}
```

### Verifier upgrade path

`IResultVerifier` is a strategy pattern. Owner swaps verifier without changing
LotteryManager or CrossChainResultReceiver logic:

```
Phase 1 (MVP):   SingleRelayerVerifier
                   verify() = ecrecover(hash, sig) == trustedRelayer
                   Simple ECDSA signature from OZ Relayer
     │
     ▼  setVerifier()
Phase 2:          MultiSigVerifier
                   verify() = count valid sigs >= threshold
                   Multiple OZ Relayer instances sign independently
     │
     ▼  setVerifier()
Phase 3:          LightClientVerifier
                   verify() = verify Base block header + Merkle inclusion proof
                   Trustless but complex — requires Base light client on Artemis
```

All three implement the same `IResultVerifier` interface. Upgrade is a single
`setVerifier()` call — no migration, no redeployment.

### OZ Relayer configuration (self-hosted)

**oz-monitor** — watches Base for VRF fulfillment events:
- Network: Base mainnet (chain ID 8453)
- Contract: ChainlinkVRFResultSource address
- Event: `ResultFulfilled(uint256 indexed roundId, uint8 specialPrize, uint8[] allPrizes)`

**oz-relayer** — submits results to Artemis:
```json
{
  "type": "evm",
  "network": "artemis",
  "chain_id": 322,
  "rpc_urls": ["http://artemis-rpc:9944"],
  "symbol": "ART",
  "average_blocktime_ms": 6000,
  "is_testnet": true
}
```

Relayer benefits over custom script:
- Managed nonce tracking (no stuck transactions)
- Automatic gas bumping for slow transactions
- Built-in retry with exponential backoff
- Key management (no raw private keys in env vars)
- Health monitoring

### OracleResultSource (phase 2)

Trusted operator posts real XSKTMB lottery results. Can be deployed on any chain
(Base or Artemis) depending on where the betting contract lives:

```solidity
function submitResult(uint256 roundId, LotteryResult calldata result) external onlyOperator;
```

Same `IResultSource` interface — betting contract does not need changes.

## Bet Placement

### BetParams

```solidity
enum BetType { DE, LO, XIEN2, XIEN3, XIEN4 }

struct BetParams {
    GameId gameId;
    BetType betType;
    uint8[] numbers;    // length must match bet type
    uint256 amount;
}
```

### Validation

- Each number in range 0-99
- numbers.length: 1 for DE/LO, 2 for XIEN2, 3 for XIEN3, 4 for XIEN4
- No duplicate numbers in XIEN bets
- Round must be OPEN (derived from block.timestamp)
- Bookmaker mode: pool must cover worst-case exposure after this bet

### Settlement logic

On `settle(GameId, uint256 roundId)`:

1. Fetch result from result source
2. Mark round as SETTLED
3. Store result on-chain for claim verification

On `claim(GameId, uint256 roundId)`:

1. Check round is SETTLED (or CANCELLED for refund)
2. For each bet by caller in that round:
   - **DE:** `numbers[0] == result.specialPrize` → payout = amount * rates.de / 100
   - **LO:** count occurrences of `numbers[0]` in `result.allPrizes` → payout = count * amount * rates.lo / 100
   - **XIEN2/3/4:** all numbers present in `result.allPrizes` → payout = amount * rates.xienN / 100
3. Deduct protocol fee from payout
4. Credit delta (flash accounting) or direct transfer

Parimutuel mode settlement:
1. Calculate total pool minus protocol fee: `netPool = totalPool - (totalPool * feeBps / 10000)`
2. Identify all winning bets. Weight = bet amount (all bet types weighted equally)
3. Each winner's payout = `(winnerBetAmount / totalWinningBetAmount) * netPool`
4. If no winners, pool rolls over to next round (recurring) or refunds all bettors (one-shot)

## Security Measures

| Threat | Mitigation |
|--------|-----------|
| VRF manipulation | Chainlink VRF v2.5 with cryptographic proof on Base. Cross-chain: verifier strategy validates relay integrity |
| LO unbounded payout | Worst-case exposure = 27 * rate * amount. Checked on placeBet in bookmaker mode |
| Front-running results | Time-based lock — bets after lockTime revert. Result request only after lock |
| Fee-on-transfer tokens | Measure balanceAfter - balanceBefore for all ERC20 transfers in |
| Reentrancy | Unlock/flash accounting pattern + checks-effects-interactions. ReentrancyGuard on unlock |
| Timestamp manipulation | Acceptable tolerance ~15s. Betting windows should be minutes minimum |
| Relay manipulation | IResultVerifier strategy: ECDSA sig (MVP) → multi-sig → light client. Results publicly verifiable on Base |
| Relay liveness | OZ Relayer retry + gas bumping. Auto-cancel round on timeout if result not delivered |
| VRF callback revert | fulfillRandomWords must never revert. Store-then-process pattern |
| Hook griefing | Gas-limited external calls to hooks. Revert in hook reverts the action |
| Pool insolvency | Per-round exposure tracking. Reject bets that exceed pool coverage |

## Contract Structure

```
contracts/
  interfaces/
    ILotteryManager.sol          Core singleton interface
    ILotteryHooks.sol            Hooks interface (8 hooks)
    ILotteryFeeController.sol    Fee controller interface
    IResultSource.sol            Result source interface
    IResultVerifier.sol          Cross-chain proof verification interface
    IUnlockCallback.sol          Unlock callback interface
  core/
    LotteryManager.sol           Singleton: games, rounds, bets, settlement
  libraries/
    GameId.sol                   GameKey → GameId derivation
    Hooks.sol                    Permission flags, validation, dispatch
    BetValidator.sol             Input validation, exposure calculation
    SettlementLib.sol            Result matching, payout math
    TransferHelper.sol           Safe native + ERC20 transfers
    RoundDerivation.sol          Timestamp → roundId math
    ProtocolFees.sol             Fee accrual and collection logic
  results/
    ChainlinkVRFResultSource.sol Chainlink VRF v2.5 (Base mainnet)
    CrossChainResultReceiver.sol IResultSource on Artemis, delegates to IResultVerifier
    OracleResultSource.sol       External lottery feed (phase 2)
  verifiers/
    SingleRelayerVerifier.sol    ECDSA signature verification (MVP)
    MultiSigVerifier.sol         N-of-M threshold signatures (phase 2)
    LightClientVerifier.sol      Base block header + Merkle proof (phase 3)
  hooks/
    BaseHook.sol                 Abstract base with no-op defaults

infra/
  oz-monitor/                    OZ Monitor config for Base event watching
  oz-relayer/                    OZ Relayer config for Artemis tx submission
```

## Testing Strategy

| Layer | What to test |
|-------|-------------|
| Unit | SettlementLib: all 5 bet types, edge cases (0 occurrences, 27 occurrences for LO) |
| Unit | RoundDerivation: timestamp math, boundary conditions |
| Unit | BetValidator: valid/invalid inputs, exposure calculation |
| Unit | Hooks.sol: flag encoding/decoding, validation |
| Integration | Full lifecycle: initialize → placeBet → settle → claim for both pool modes |
| Integration | Recurring rounds: verify implicit round creation across intervals |
| Integration | Protocol fee: accrual and collection across multiple games/tokens |
| Integration | Hooks: verify all 8 hooks fire correctly with mock hooks contract |
| Security | Non-creator cannot cancel rounds |
| Security | Chainlink VRF: cryptographic proof verification on Base |
| Integration | Cross-chain: OZ Monitor detects Base event → OZ Relayer submits to Artemis |
| Integration | SingleRelayerVerifier: verify ECDSA signature matches registered relayer |
| Security | Bookmaker exposure: bets rejected when pool insufficient |
| Security | Reentrancy: unlock pattern prevents re-entrance |
| Security | Fee-on-transfer: accounting remains correct |

## Deployment Targets

### Single-chain (Base mainnet)

- LotteryManager + ChainlinkVRFResultSource on Base
- No cross-chain infra needed
- Chainlink VRF v2.5 subscription funded with LINK or ETH

### Cross-chain (Artemis + Base)

- LotteryManager + CrossChainResultReceiver on Artemis
- ChainlinkVRFResultSource on Base
- OZ Monitor + OZ Relayer (self-hosted) bridging results
- SingleRelayerVerifier on Artemis (MVP)

### Dependencies

| Component | Dependency |
|-----------|-----------|
| ChainlinkVRFResultSource | `@chainlink/contracts` v1.x (VRFConsumerBaseV2Plus) |
| LotteryManager | OpenZeppelin Contracts (ReentrancyGuard, IERC20) |
| OZ Monitor | `@openzeppelin/defender-sdk` or self-hosted oz-monitor |
| OZ Relayer | Self-hosted oz-relayer with Artemis custom network config |

## Out of Scope

- Frontend / webapp integration
- Subgraph / indexer for events
- Governance token or DAO for fee controller
- MultiSigVerifier and LightClientVerifier (phase 2/3, interface defined now)
- Upgradeable proxy (deploy new contract if logic changes)
- OZ Relayer deployment automation (documented config only)

## Open Questions

None. All major decisions resolved during brainstorming.
