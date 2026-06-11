# Cross-Chain VRF Relay (Base Sepolia → Artemis)

## Overview

Bidirectional relay infrastructure for requesting Chainlink VRF randomness from
Artemis and receiving results back. Uses OZ Monitor + OZ Relayer (self-hosted)
for event-driven cross-chain communication.

This is the foundation layer. The lottery betting contracts (LotteryManager, hooks,
flash accounting) will be built on top in a later phase.

## Architecture

```
Artemis (chain 322)              OZ Infra (self-hosted)              Base Sepolia (chain 84532)
┌──────────────────────┐     ┌──────────────────────────┐     ┌──────────────────────────┐
│ CrossChainResult     │     │                          │     │ ChainlinkVRFResult       │
│   Receiver           │     │ Pair 1: Artemis → Base   │     │   Source                 │
│                      │     │   oz-monitor-artemis     │     │                          │
│ requestResult()      │     │     watch Artemis for    │     │ requestResult(roundId)   │
│   → emit Result      │────▶│     ResultRequested      │────▶│   → Chainlink VRF v2.5   │
│     Requested()      │     │   oz-relayer-base        │     │   → requestRandomWords() │
│                      │     │     call Base contract   │     │                          │
│                      │     │                          │     │ fulfillRandomWords()     │
│ submitResult()       │     │ Pair 2: Base → Artemis   │     │   → store result         │
│   → verify signature │◀────│   oz-monitor-base        │◀────│   → emit Result          │
│   → store result     │     │     watch Base for       │     │     Fulfilled()          │
│                      │     │     ResultFulfilled      │     │                          │
│ getResult()          │     │   oz-relayer-artemis     │     └──────────────────────────┘
│ hasResult()          │     │     sign + submit to     │
└──────────────────────┘     │     Artemis receiver     │
                             └──────────────────────────┘
```

## Flow (step by step)

1. Caller on Artemis calls `CrossChainResultReceiver.requestResult(roundId)`
2. Contract emits `ResultRequested(roundId)`
3. **oz-monitor-artemis** detects event on Artemis
4. **oz-relayer-base** calls `ChainlinkVRFResultSource.requestResult(roundId)` on Base Sepolia
5. Chainlink VRF Coordinator processes request (waits 3 block confirmations)
6. Chainlink calls `fulfillRandomWords()` on ChainlinkVRFResultSource
7. Contract generates lottery result (1 special prize + 27 prizes) and emits `ResultFulfilled(roundId, ...)`
8. **oz-monitor-base** detects `ResultFulfilled` event on Base Sepolia
9. **oz-relayer-artemis** signs `(roundId, result)` and calls `CrossChainResultReceiver.submitResult()` on Artemis
10. CrossChainResultReceiver verifies relayer signature via `SingleRelayerVerifier`
11. Result stored on Artemis, queryable via `getResult(roundId)`

## Contracts

### IResultSource (shared interface)

```solidity
// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

struct LotteryResult {
    uint8 specialPrize;    // 00-99
    uint8[] allPrizes;     // 27 entries, each 00-99. allPrizes[0] == specialPrize
}

interface IResultSource {
    event ResultRequested(uint256 indexed roundId);
    event ResultFulfilled(uint256 indexed roundId);

    function requestResult(uint256 roundId) external;
    function getResult(uint256 roundId) external view returns (LotteryResult memory);
    function hasResult(uint256 roundId) external view returns (bool);
}
```

### ChainlinkVRFResultSource (Base Sepolia)

Deployed on Base Sepolia. Receives VRF requests relayed from Artemis and generates
random lottery results.

```solidity
contract ChainlinkVRFResultSource is VRFConsumerBaseV2Plus, IResultSource {
    bytes32 public immutable i_keyHash;
    uint256 public immutable i_subscriptionId;
    uint16 public constant REQUEST_CONFIRMATIONS = 3;
    uint32 public constant CALLBACK_GAS_LIMIT = 300_000;
    uint32 public constant NUM_WORDS = 2;

    mapping(uint256 roundId => uint256 requestId) public roundToRequest;
    mapping(uint256 requestId => uint256 roundId) public requestToRound;
    mapping(uint256 roundId => LotteryResult) internal results;
    mapping(uint256 roundId => bool) public fulfilled;

    error RoundAlreadyRequested(uint256 roundId);
    error RoundNotFulfilled(uint256 roundId);
}
```

Key behaviors:
- `requestResult(roundId)` — reverts if roundId already requested. Calls Chainlink VRF.
  Maps roundId ↔ requestId for lookup.
- `fulfillRandomWords(requestId, randomWords)` — MUST NOT revert. Derives 28 values
  from 2 random words via sequential keccak256 hashing. Stores result, emits
  `ResultFulfilled(roundId)`.
- `getResult(roundId)` — returns stored result. Reverts if not yet fulfilled.
- `hasResult(roundId)` — returns true if fulfilled.
- Anyone can call `requestResult()` — permissionless. OZ Relayer is the expected caller
  but not enforced.

Random number derivation from 2 VRF words:
```
specialPrize = uint8(keccak256(abi.encode(word0, 0)) % 100)
allPrizes[0] = specialPrize
allPrizes[i] = uint8(keccak256(abi.encode(word0, word1, i)) % 100)  for i in 1..26
```

### CrossChainResultReceiver (Artemis)

Deployed on Artemis. Implements `IResultSource`. Receives results from OZ Relayer
and verifies via pluggable `IResultVerifier`.

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

    mapping(uint256 roundId => LotteryResult) internal results;
    mapping(uint256 roundId => bool) public requested;
    mapping(uint256 roundId => bool) public fulfilled;

    error RoundNotRequested(uint256 roundId);
    error RoundAlreadyFulfilled(uint256 roundId);
    error InvalidProof();
}
```

Key behaviors:
- `requestResult(roundId)` — marks roundId as requested, emits `ResultRequested(roundId)`.
  OZ Monitor watches this event. Reverts if already requested.
- `submitResult(roundId, result, proof)` — called by OZ Relayer. Verifies proof via
  `verifier.verify()`. Reverts if not requested, already fulfilled, or proof invalid.
  Stores result, emits `ResultFulfilled(roundId)`.
- `getResult(roundId)` / `hasResult(roundId)` — standard read interface.
- `setVerifier(IResultVerifier)` — owner only. Enables upgrade path.

### SingleRelayerVerifier (Artemis)

MVP verifier. Checks ECDSA signature from a single trusted relayer.

```solidity
contract SingleRelayerVerifier is IResultVerifier {
    address public immutable trustedRelayer;

    function verify(
        uint256 roundId,
        LotteryResult calldata result,
        bytes calldata proof
    ) external view returns (bool) {
        bytes32 hash = keccak256(abi.encode(roundId, result.specialPrize, result.allPrizes));
        bytes32 ethSignedHash = MessageHashUtils.toEthSignedMessageHash(hash);
        address signer = ECDSA.recover(ethSignedHash, proof);
        return signer == trustedRelayer;
    }
}
```

The `trustedRelayer` address is the OZ Relayer's managed signer address on Artemis.

## OZ Infrastructure

### 2 services, multi-network

| Service | Role |
|---------|------|
| **oz-monitor** | Watch events on both Artemis and Base Sepolia, trigger handlers |
| **oz-relayer** | Submit transactions on both chains, manage keys/nonce/gas |

Both run in a single `docker-compose.yml`.

### oz-monitor config (single instance, 2 networks)

```yaml
networks:
  artemis:
    type: evm
    chain_id: 322
    rpc_urls:
      - url: "http://127.0.0.1:9944"
        weight: 100
    average_blocktime_ms: 6000
  base_sepolia:
    type: evm
    chain_id: 84532
    rpc_urls:
      - url: "https://sepolia.base.org"
        weight: 100
    average_blocktime_ms: 2000

monitors:
  watch_artemis_requests:
    network: artemis
    contract_address: "${CROSS_CHAIN_RECEIVER_ADDRESS}"
    events:
      - signature: "ResultRequested(uint256)"
    handler: relay_to_base

  watch_base_results:
    network: base_sepolia
    contract_address: "${VRF_RESULT_SOURCE_ADDRESS}"
    events:
      - signature: "ResultFulfilled(uint256)"
    handler: relay_to_artemis
```

### oz-relayer config (single instance, 2 signers)

```yaml
networks:
  artemis:
    type: evm
    chain_id: 322
    rpc_urls:
      - url: "http://127.0.0.1:9944"
        weight: 100
    average_blocktime_ms: 6000
    is_testnet: true
  base_sepolia:
    type: evm
    chain_id: 84532
    rpc_urls:
      - url: "https://sepolia.base.org"
        weight: 100
    average_blocktime_ms: 2000

relayers:
  base_signer:
    network: base_sepolia
    signer:
      type: local
      path: "${BASE_RELAYER_KEY_PATH}"
    gas_policy:
      type: eip1559
      max_fee_per_gas: "1000000000"
      max_priority_fee: "100000000"

  artemis_signer:
    network: artemis
    signer:
      type: local
      path: "${ARTEMIS_RELAYER_KEY_PATH}"
    gas_policy:
      type: legacy
      gas_price: "1000000000"
```

### Handler logic (glue between monitor and relayer)

Each monitor event triggers a handler. The handler reads the event, fetches
necessary data, and submits a transaction via the corresponding relayer signer.

**Handler: relay_to_base (Artemis ResultRequested → Base requestResult):**
1. Parse `ResultRequested(roundId)` from monitor event
2. Call `base_signer` relayer to execute `ChainlinkVRFResultSource.requestResult(roundId)`

**Handler: relay_to_artemis (Base ResultFulfilled → Artemis submitResult):**
1. Parse `ResultFulfilled(roundId)` from monitor event
2. Read full result from `ChainlinkVRFResultSource.getResult(roundId)` on Base
3. Sign `(roundId, specialPrize, allPrizes)` with `artemis_signer` key
4. Call `artemis_signer` relayer to execute `CrossChainResultReceiver.submitResult(roundId, result, signature)`

## Chainlink VRF Setup (Base Sepolia)

### Prerequisites

1. Create VRF v2.5 subscription at vrf.chain.link
2. Fund subscription with testnet LINK (Base Sepolia faucet)
3. Deploy `ChainlinkVRFResultSource` with:
   - `vrfCoordinator`: Base Sepolia VRF Coordinator address
   - `keyHash`: Base Sepolia gas lane key hash
   - `subscriptionId`: from step 1
4. Add deployed contract as consumer in VRF subscription dashboard

### Base Sepolia VRF parameters

| Parameter | Value |
|-----------|-------|
| VRF Coordinator | Chainlink docs: Base Sepolia coordinator address |
| Key hash (gas lane) | Chainlink docs: Base Sepolia key hash |
| Request confirmations | 3 |
| Callback gas limit | 300,000 |
| Num words | 2 |
| Payment | LINK token (nativePayment: false) |

## File Structure

```
packages/contracts/
  contracts/
    interfaces/
      IResultSource.sol
      IResultVerifier.sol
    results/
      ChainlinkVRFResultSource.sol    (deploy on Base Sepolia)
      CrossChainResultReceiver.sol    (deploy on Artemis)
    verifiers/
      SingleRelayerVerifier.sol       (deploy on Artemis)
  test/
    ChainlinkVRFResultSource.test.ts
    CrossChainResultReceiver.test.ts
    SingleRelayerVerifier.test.ts
  deploy/
    01-deploy-vrf-source.ts           (Base Sepolia)
    02-deploy-receiver.ts             (Artemis)
    03-deploy-verifier.ts             (Artemis)

infra/
  oz-monitor/
    config.yaml                       Multi-network monitor config
    handlers/
      relay-to-base.ts                ResultRequested → requestResult on Base
      relay-to-artemis.ts             ResultFulfilled → submitResult on Artemis
  oz-relayer/
    config.yaml                       Multi-network relayer config (2 signers)
  docker-compose.yml                  2 services, restart: unless-stopped
```

## Testing Strategy

### Unit tests (Hardhat, local)

| Test | What |
|------|------|
| ChainlinkVRFResultSource | Mock VRF Coordinator, verify requestResult stores mapping, fulfillRandomWords generates valid 28 numbers (0-99), revert on duplicate roundId |
| CrossChainResultReceiver | requestResult emits event, submitResult with valid signature stores result, revert on invalid signature, revert on unrequested roundId, revert on duplicate fulfill |
| SingleRelayerVerifier | Valid signature returns true, wrong signer returns false, malformed signature returns false |

### Integration tests (Base Sepolia + Artemis dev node)

| Test | What |
|------|------|
| VRF end-to-end on Base | Deploy VRFResultSource on Base Sepolia, request result, wait for Chainlink fulfill, verify result readable |
| Relay end-to-end | Full pipeline: requestResult on Artemis → OZ Monitor → Base VRF → OZ Monitor → Artemis receiver. Verify result matches Base-side result |

### Manual verification

1. Deploy all contracts
2. Start OZ Monitor + Relayer (docker-compose)
3. Call `requestResult(1)` on Artemis CrossChainResultReceiver
4. Wait for Chainlink VRF fulfill (~30s on Base Sepolia)
5. Wait for OZ Relayer to submit result to Artemis (~15s)
6. Call `getResult(1)` on Artemis — verify 1 specialPrize + 27 allPrizes returned
7. Verify result on Artemis matches result on Base Sepolia

## Security Considerations

| Threat | Mitigation |
|--------|-----------|
| Relayer submits fake result | SingleRelayerVerifier checks ECDSA signature. Relayer key is managed by OZ Relayer |
| Relayer goes offline | Round stays in requested state. Docker restart policy auto-recovers. OZ Monitor rescans missed blocks on restart |
| Duplicate result submission | `submitResult` reverts if roundId already fulfilled |
| VRF callback revert | `fulfillRandomWords` never reverts — store-then-emit pattern |
| Relayer key compromise | Owner calls `setVerifier()` to swap to new verifier with new relayer address |
| Replay attack | roundId is unique per request. Cannot resubmit same roundId |

## Out of Scope

- LotteryManager, betting logic, hooks, flash accounting (next phase)
- MultiSigVerifier, LightClientVerifier (upgrade path defined by IResultVerifier)
- Round timeout / auto-cancel
- Protocol fee
- Production deployment (Base mainnet)
- Frontend integration

## Dependencies

| Package | Used by |
|---------|---------|
| `@chainlink/contracts` ^1.0.0 | ChainlinkVRFResultSource (VRFConsumerBaseV2Plus) |
| `@openzeppelin/contracts` ^5.0.0 | ECDSA, MessageHashUtils, Ownable |
| `openzeppelin-monitor` (self-hosted) | Event watching on both chains |
| `openzeppelin-relayer` (self-hosted) | Transaction submission on both chains |
