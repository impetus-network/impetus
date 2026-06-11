# Cross-Chain VRF Relay Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build bidirectional cross-chain relay: Chainlink VRF on Base Sepolia generates lottery results, OZ Monitor+Relayer bridges them to/from Artemis.

**Architecture:** 3 Solidity contracts (IResultSource interface, ChainlinkVRFResultSource on Base, CrossChainResultReceiver + SingleRelayerVerifier on Artemis) + 2 self-hosted OZ services (monitor + relayer) with multi-network configs and TypeScript handlers.

**Tech Stack:** Solidity 0.8.28, Hardhat, Chainlink VRF v2.5, OpenZeppelin Contracts v5, OZ Monitor, OZ Relayer, ethers.js, TypeScript

---

## File Map

### Contracts (packages/contracts/)

| File | Responsibility |
|------|---------------|
| `contracts/interfaces/IResultSource.sol` | Shared interface: requestResult, getResult, hasResult + LotteryResult struct |
| `contracts/interfaces/IResultVerifier.sol` | Verifier strategy interface: verify(roundId, result, proof) |
| `contracts/results/ChainlinkVRFResultSource.sol` | Base Sepolia: Chainlink VRF v2.5 consumer, generates lottery results |
| `contracts/results/CrossChainResultReceiver.sol` | Artemis: receives relayed results, delegates to IResultVerifier |
| `contracts/verifiers/SingleRelayerVerifier.sol` | ECDSA signature check against trusted relayer address |
| `test/SingleRelayerVerifier.test.ts` | Unit tests for signature verification |
| `test/CrossChainResultReceiver.test.ts` | Unit tests for receiver logic |
| `test/ChainlinkVRFResultSource.test.ts` | Unit tests with mocked VRF Coordinator |
| `test/helpers/setup.ts` | Existing test helpers (no changes) |
| `scripts/deploy-vrf-source.ts` | Deploy script for Base Sepolia |
| `scripts/deploy-receiver.ts` | Deploy script for Artemis |

### OZ Infrastructure (infra/)

| File | Responsibility |
|------|---------------|
| `infra/oz-monitor/config.yaml` | Multi-network monitor: watch Artemis + Base Sepolia events |
| `infra/oz-monitor/handlers/relay-to-base.ts` | Handler: ResultRequested → requestResult on Base |
| `infra/oz-monitor/handlers/relay-to-artemis.ts` | Handler: ResultFulfilled → submitResult on Artemis |
| `infra/oz-relayer/config.yaml` | Multi-network relayer: 2 signers (Base + Artemis) |
| `infra/docker-compose.yml` | 2 services: oz-monitor + oz-relayer |
| `infra/.env.example` | Template for all required env vars |

### Config changes

| File | Change |
|------|--------|
| `packages/contracts/hardhat.config.ts` | Add `base_sepolia` network |
| `packages/contracts/package.json` | Add `@chainlink/contracts` dependency |

---

### Task 1: Add Chainlink dependency and Base Sepolia network

**Files:**
- Modify: `packages/contracts/package.json`
- Modify: `packages/contracts/hardhat.config.ts`

- [ ] **Step 1: Add @chainlink/contracts dependency**

```bash
cd packages/contracts && pnpm add -D @chainlink/contracts
```

- [ ] **Step 2: Add base_sepolia network to hardhat config**

In `packages/contracts/hardhat.config.ts`, add the `base_sepolia` network alongside the existing `substrate` network:

```typescript
import type { HardhatUserConfig } from "hardhat/config";
import "@nomicfoundation/hardhat-toolbox";

const config: HardhatUserConfig = {
  solidity: {
    version: "0.8.28",
    settings: {
      evmVersion: "cancun",
    },
  },
  networks: {
    substrate: {
      url: "http://127.0.0.1:9944",
      chainId: 322,
      accounts: [
        // Mnemonic: "test test test test test test test test test test test junk"
        "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80", // #0 admin
        "0x59c6995e998f97a5a0044966f0945389dc9e86dae88c7a8412f4603b6b78690d", // #1
        "0x5de4111afa1a4b94908f83103eb1f1706367c2e68ca870fc3fb9a804cdab365a", // #2
      ],
    },
    base_sepolia: {
      url: process.env.BASE_SEPOLIA_RPC_URL ?? "https://sepolia.base.org",
      chainId: 84532,
      accounts: process.env.BASE_DEPLOYER_KEY ? [process.env.BASE_DEPLOYER_KEY] : [],
    },
  },
};

export default config;
```

- [ ] **Step 3: Verify compilation still works**

Run: `cd packages/contracts && pnpm compile`
Expected: Compilation finished successfully

- [ ] **Step 4: Commit**

```bash
git add packages/contracts/package.json packages/contracts/pnpm-lock.yaml packages/contracts/hardhat.config.ts
git commit -m "chore(contracts): add chainlink dependency and base sepolia network"
```

---

### Task 2: IResultSource and IResultVerifier interfaces

**Files:**
- Create: `packages/contracts/contracts/interfaces/IResultSource.sol`
- Create: `packages/contracts/contracts/interfaces/IResultVerifier.sol`

- [ ] **Step 1: Create IResultSource interface**

Create `packages/contracts/contracts/interfaces/IResultSource.sol`:

```solidity
// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

struct LotteryResult {
    uint8 specialPrize;
    uint8[] allPrizes;
}

interface IResultSource {
    event ResultRequested(uint256 indexed roundId);
    event ResultFulfilled(uint256 indexed roundId);

    function requestResult(uint256 roundId) external;
    function getResult(uint256 roundId) external view returns (LotteryResult memory);
    function hasResult(uint256 roundId) external view returns (bool);
}
```

- [ ] **Step 2: Create IResultVerifier interface**

Create `packages/contracts/contracts/interfaces/IResultVerifier.sol`:

```solidity
// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import {LotteryResult} from "./IResultSource.sol";

interface IResultVerifier {
    function verify(
        uint256 roundId,
        LotteryResult calldata result,
        bytes calldata proof
    ) external view returns (bool);
}
```

- [ ] **Step 3: Verify compilation**

Run: `cd packages/contracts && pnpm compile`
Expected: Compilation finished successfully

- [ ] **Step 4: Commit**

```bash
git add packages/contracts/contracts/interfaces/IResultSource.sol packages/contracts/contracts/interfaces/IResultVerifier.sol
git commit -m "feat(contracts): add IResultSource and IResultVerifier interfaces"
```

---

### Task 3: SingleRelayerVerifier contract + tests

**Files:**
- Create: `packages/contracts/contracts/verifiers/SingleRelayerVerifier.sol`
- Create: `packages/contracts/test/SingleRelayerVerifier.test.ts`

- [ ] **Step 1: Write the failing tests**

Create `packages/contracts/test/SingleRelayerVerifier.test.ts`:

```typescript
import { expect } from "chai";
import { ethers } from "hardhat";

describe("SingleRelayerVerifier", function () {
  const TRUSTED_RELAYER_KEY = "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80";
  const OTHER_KEY = "0x59c6995e998f97a5a0044966f0945389dc9e86dae88c7a8412f4603b6b78690d";

  async function deployVerifier() {
    const trustedRelayer = new ethers.Wallet(TRUSTED_RELAYER_KEY);
    const otherSigner = new ethers.Wallet(OTHER_KEY);

    const factory = await ethers.getContractFactory("SingleRelayerVerifier");
    const verifier = await factory.deploy(trustedRelayer.address);
    await verifier.waitForDeployment();

    return { verifier, trustedRelayer, otherSigner };
  }

  function encodeLotteryResult(specialPrize: number, allPrizes: number[]) {
    return ethers.AbiCoder.defaultAbiCoder().encode(
      ["uint8", "uint8[]"],
      [specialPrize, allPrizes]
    );
  }

  async function signResult(
    signer: InstanceType<typeof ethers.Wallet>,
    roundId: bigint,
    specialPrize: number,
    allPrizes: number[]
  ): Promise<string> {
    const hash = ethers.keccak256(
      ethers.AbiCoder.defaultAbiCoder().encode(
        ["uint256", "uint8", "uint8[]"],
        [roundId, specialPrize, allPrizes]
      )
    );
    return signer.signMessage(ethers.getBytes(hash));
  }

  it("returns true for valid signature from trusted relayer", async function () {
    const { verifier, trustedRelayer } = await deployVerifier();
    const roundId = 1n;
    const specialPrize = 42;
    const allPrizes = Array.from({ length: 27 }, (_, i) => i % 100);

    const signature = await signResult(trustedRelayer, roundId, specialPrize, allPrizes);

    const result = await verifier.verify(
      roundId,
      { specialPrize, allPrizes },
      signature
    );
    expect(result).to.be.true;
  });

  it("returns false for signature from wrong signer", async function () {
    const { verifier, otherSigner } = await deployVerifier();
    const roundId = 1n;
    const specialPrize = 42;
    const allPrizes = Array.from({ length: 27 }, (_, i) => i % 100);

    const signature = await signResult(otherSigner, roundId, specialPrize, allPrizes);

    const result = await verifier.verify(
      roundId,
      { specialPrize, allPrizes },
      signature
    );
    expect(result).to.be.false;
  });

  it("returns false for tampered result", async function () {
    const { verifier, trustedRelayer } = await deployVerifier();
    const roundId = 1n;
    const specialPrize = 42;
    const allPrizes = Array.from({ length: 27 }, (_, i) => i % 100);

    const signature = await signResult(trustedRelayer, roundId, specialPrize, allPrizes);

    // Tamper: change specialPrize
    const result = await verifier.verify(
      roundId,
      { specialPrize: 99, allPrizes },
      signature
    );
    expect(result).to.be.false;
  });

  it("exposes trustedRelayer address", async function () {
    const { verifier, trustedRelayer } = await deployVerifier();
    expect(await verifier.trustedRelayer()).to.equal(trustedRelayer.address);
  });
});
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd packages/contracts && pnpm test -- --grep "SingleRelayerVerifier"`
Expected: FAIL — "SingleRelayerVerifier" contract not found

- [ ] **Step 3: Implement SingleRelayerVerifier**

Create `packages/contracts/contracts/verifiers/SingleRelayerVerifier.sol`:

```solidity
// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import {IResultVerifier} from "../interfaces/IResultVerifier.sol";
import {LotteryResult} from "../interfaces/IResultSource.sol";
import {ECDSA} from "@openzeppelin/contracts/utils/cryptography/ECDSA.sol";
import {MessageHashUtils} from "@openzeppelin/contracts/utils/cryptography/MessageHashUtils.sol";

contract SingleRelayerVerifier is IResultVerifier {
    address public immutable trustedRelayer;

    error ZeroAddress();

    constructor(address _trustedRelayer) {
        if (_trustedRelayer == address(0)) revert ZeroAddress();
        trustedRelayer = _trustedRelayer;
    }

    function verify(
        uint256 roundId,
        LotteryResult calldata result,
        bytes calldata proof
    ) external view returns (bool) {
        bytes32 hash = keccak256(
            abi.encode(roundId, result.specialPrize, result.allPrizes)
        );
        bytes32 ethSignedHash = MessageHashUtils.toEthSignedMessageHash(hash);
        address signer = ECDSA.recover(ethSignedHash, proof);
        return signer == trustedRelayer;
    }
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cd packages/contracts && pnpm test -- --grep "SingleRelayerVerifier"`
Expected: 4 passing

- [ ] **Step 5: Commit**

```bash
git add packages/contracts/contracts/verifiers/SingleRelayerVerifier.sol packages/contracts/test/SingleRelayerVerifier.test.ts
git commit -m "feat(contracts): add SingleRelayerVerifier with ECDSA verification"
```

---

### Task 4: CrossChainResultReceiver contract + tests

**Files:**
- Create: `packages/contracts/contracts/results/CrossChainResultReceiver.sol`
- Create: `packages/contracts/test/CrossChainResultReceiver.test.ts`

- [ ] **Step 1: Write the failing tests**

Create `packages/contracts/test/CrossChainResultReceiver.test.ts`:

```typescript
import { expect } from "chai";
import { ethers } from "hardhat";

describe("CrossChainResultReceiver", function () {
  const RELAYER_KEY = "0x59c6995e998f97a5a0044966f0945389dc9e86dae88c7a8412f4603b6b78690d";

  async function deployReceiver() {
    const [owner, nonOwner] = await ethers.getSigners();
    const relayer = new ethers.Wallet(RELAYER_KEY);

    // Deploy verifier
    const verifierFactory = await ethers.getContractFactory("SingleRelayerVerifier");
    const verifier = await verifierFactory.deploy(relayer.address);
    await verifier.waitForDeployment();

    // Deploy receiver
    const receiverFactory = await ethers.getContractFactory("CrossChainResultReceiver");
    const receiver = await receiverFactory.deploy(await verifier.getAddress());
    await receiver.waitForDeployment();

    return { receiver, verifier, relayer, owner, nonOwner };
  }

  async function signResult(
    signer: InstanceType<typeof ethers.Wallet>,
    roundId: bigint,
    specialPrize: number,
    allPrizes: number[]
  ): Promise<string> {
    const hash = ethers.keccak256(
      ethers.AbiCoder.defaultAbiCoder().encode(
        ["uint256", "uint8", "uint8[]"],
        [roundId, specialPrize, allPrizes]
      )
    );
    return signer.signMessage(ethers.getBytes(hash));
  }

  const ROUND_ID = 1n;
  const SPECIAL_PRIZE = 42;
  const ALL_PRIZES = Array.from({ length: 27 }, (_, i) => i % 100);

  describe("requestResult", function () {
    it("emits ResultRequested and marks round as requested", async function () {
      const { receiver } = await deployReceiver();

      await expect(receiver.requestResult(ROUND_ID))
        .to.emit(receiver, "ResultRequested")
        .withArgs(ROUND_ID);

      expect(await receiver.requested(ROUND_ID)).to.be.true;
    });

    it("reverts if round already requested", async function () {
      const { receiver } = await deployReceiver();
      await receiver.requestResult(ROUND_ID);

      await expect(receiver.requestResult(ROUND_ID))
        .to.be.revertedWithCustomError(receiver, "RoundAlreadyRequested");
    });
  });

  describe("submitResult", function () {
    it("stores result with valid signature for requested round", async function () {
      const { receiver, relayer } = await deployReceiver();
      await receiver.requestResult(ROUND_ID);

      const signature = await signResult(relayer, ROUND_ID, SPECIAL_PRIZE, ALL_PRIZES);

      await expect(
        receiver.submitResult(
          ROUND_ID,
          { specialPrize: SPECIAL_PRIZE, allPrizes: ALL_PRIZES },
          signature
        )
      ).to.emit(receiver, "ResultFulfilled").withArgs(ROUND_ID);

      expect(await receiver.hasResult(ROUND_ID)).to.be.true;

      const result = await receiver.getResult(ROUND_ID);
      expect(result.specialPrize).to.equal(SPECIAL_PRIZE);
      expect(result.allPrizes).to.have.lengthOf(27);
      expect(result.allPrizes[0]).to.equal(ALL_PRIZES[0]);
    });

    it("reverts if round not requested", async function () {
      const { receiver, relayer } = await deployReceiver();
      const signature = await signResult(relayer, ROUND_ID, SPECIAL_PRIZE, ALL_PRIZES);

      await expect(
        receiver.submitResult(
          ROUND_ID,
          { specialPrize: SPECIAL_PRIZE, allPrizes: ALL_PRIZES },
          signature
        )
      ).to.be.revertedWithCustomError(receiver, "RoundNotRequested");
    });

    it("reverts if round already fulfilled", async function () {
      const { receiver, relayer } = await deployReceiver();
      await receiver.requestResult(ROUND_ID);

      const signature = await signResult(relayer, ROUND_ID, SPECIAL_PRIZE, ALL_PRIZES);
      await receiver.submitResult(
        ROUND_ID,
        { specialPrize: SPECIAL_PRIZE, allPrizes: ALL_PRIZES },
        signature
      );

      await expect(
        receiver.submitResult(
          ROUND_ID,
          { specialPrize: SPECIAL_PRIZE, allPrizes: ALL_PRIZES },
          signature
        )
      ).to.be.revertedWithCustomError(receiver, "RoundAlreadyFulfilled");
    });

    it("reverts with invalid signature", async function () {
      const { receiver } = await deployReceiver();
      await receiver.requestResult(ROUND_ID);

      // Sign with a different key (not the trusted relayer)
      const wrongSigner = new ethers.Wallet(
        "0x5de4111afa1a4b94908f83103eb1f1706367c2e68ca870fc3fb9a804cdab365a"
      );
      const signature = await signResult(wrongSigner, ROUND_ID, SPECIAL_PRIZE, ALL_PRIZES);

      await expect(
        receiver.submitResult(
          ROUND_ID,
          { specialPrize: SPECIAL_PRIZE, allPrizes: ALL_PRIZES },
          signature
        )
      ).to.be.revertedWithCustomError(receiver, "InvalidProof");
    });
  });

  describe("getResult", function () {
    it("reverts if round not fulfilled", async function () {
      const { receiver } = await deployReceiver();
      await receiver.requestResult(ROUND_ID);

      await expect(receiver.getResult(ROUND_ID))
        .to.be.revertedWithCustomError(receiver, "RoundNotFulfilled");
    });
  });

  describe("setVerifier", function () {
    it("allows owner to change verifier", async function () {
      const { receiver, owner } = await deployReceiver();

      const newVerifierFactory = await ethers.getContractFactory("SingleRelayerVerifier");
      const newVerifier = await newVerifierFactory.deploy(owner.address);
      await newVerifier.waitForDeployment();

      await receiver.setVerifier(await newVerifier.getAddress());
      expect(await receiver.verifier()).to.equal(await newVerifier.getAddress());
    });

    it("reverts if called by non-owner", async function () {
      const { receiver, nonOwner } = await deployReceiver();

      await expect(
        receiver.connect(nonOwner).setVerifier(ethers.ZeroAddress)
      ).to.be.revertedWithCustomError(receiver, "OwnableUnauthorizedAccount");
    });
  });
});
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd packages/contracts && pnpm test -- --grep "CrossChainResultReceiver"`
Expected: FAIL — "CrossChainResultReceiver" contract not found

- [ ] **Step 3: Implement CrossChainResultReceiver**

Create `packages/contracts/contracts/results/CrossChainResultReceiver.sol`:

```solidity
// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import {IResultSource, LotteryResult} from "../interfaces/IResultSource.sol";
import {IResultVerifier} from "../interfaces/IResultVerifier.sol";
import {Ownable} from "@openzeppelin/contracts/access/Ownable.sol";

contract CrossChainResultReceiver is IResultSource, Ownable {
    IResultVerifier public verifier;

    mapping(uint256 roundId => LotteryResult) internal results;
    mapping(uint256 roundId => bool) public requested;
    mapping(uint256 roundId => bool) public fulfilled;

    error RoundAlreadyRequested(uint256 roundId);
    error RoundNotRequested(uint256 roundId);
    error RoundAlreadyFulfilled(uint256 roundId);
    error RoundNotFulfilled(uint256 roundId);
    error InvalidProof();

    constructor(address _verifier) Ownable(msg.sender) {
        verifier = IResultVerifier(_verifier);
    }

    function requestResult(uint256 roundId) external {
        if (requested[roundId]) revert RoundAlreadyRequested(roundId);
        requested[roundId] = true;
        emit ResultRequested(roundId);
    }

    function submitResult(
        uint256 roundId,
        LotteryResult calldata result,
        bytes calldata proof
    ) external {
        if (!requested[roundId]) revert RoundNotRequested(roundId);
        if (fulfilled[roundId]) revert RoundAlreadyFulfilled(roundId);
        if (!verifier.verify(roundId, result, proof)) revert InvalidProof();

        results[roundId] = result;
        fulfilled[roundId] = true;
        emit ResultFulfilled(roundId);
    }

    function getResult(uint256 roundId) external view returns (LotteryResult memory) {
        if (!fulfilled[roundId]) revert RoundNotFulfilled(roundId);
        return results[roundId];
    }

    function hasResult(uint256 roundId) external view returns (bool) {
        return fulfilled[roundId];
    }

    function setVerifier(IResultVerifier _verifier) external onlyOwner {
        verifier = _verifier;
    }
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cd packages/contracts && pnpm test -- --grep "CrossChainResultReceiver"`
Expected: 7 passing

- [ ] **Step 5: Commit**

```bash
git add packages/contracts/contracts/results/CrossChainResultReceiver.sol packages/contracts/test/CrossChainResultReceiver.test.ts
git commit -m "feat(contracts): add CrossChainResultReceiver with verifier strategy"
```

---

### Task 5: ChainlinkVRFResultSource contract + tests

**Files:**
- Create: `packages/contracts/contracts/results/ChainlinkVRFResultSource.sol`
- Create: `packages/contracts/test/ChainlinkVRFResultSource.test.ts`

- [ ] **Step 1: Write the failing tests**

Create `packages/contracts/test/ChainlinkVRFResultSource.test.ts`:

```typescript
import { expect } from "chai";
import { ethers } from "hardhat";

describe("ChainlinkVRFResultSource", function () {
  async function deployWithMock() {
    const mockFactory = await ethers.getContractFactory("MockVRFCoordinator");
    const mock = await mockFactory.deploy();
    await mock.waitForDeployment();

    const keyHash = ethers.zeroPadValue("0x01", 32);
    const subscriptionId = 1n;

    const factory = await ethers.getContractFactory("ChainlinkVRFResultSource");
    const source = await factory.deploy(
      await mock.getAddress(),
      keyHash,
      subscriptionId
    );
    await source.waitForDeployment();

    return { source, mock, keyHash, subscriptionId };
  }

  describe("requestResult", function () {
    it("emits ResultRequested and stores roundId mapping", async function () {
      const { source } = await deployWithMock();

      await expect(source.requestResult(1n))
        .to.emit(source, "ResultRequested")
        .withArgs(1n);

      const requestId = await source.roundToRequest(1n);
      expect(requestId).to.be.gt(0n);
    });

    it("reverts if round already requested", async function () {
      const { source } = await deployWithMock();
      await source.requestResult(1n);

      await expect(source.requestResult(1n))
        .to.be.revertedWithCustomError(source, "RoundAlreadyRequested");
    });
  });

  describe("fulfillRandomWords", function () {
    it("generates valid lottery result with 28 numbers in range 0-99", async function () {
      const { source, mock } = await deployWithMock();
      await source.requestResult(1n);

      const requestId = await source.roundToRequest(1n);
      const word0 = ethers.toBigInt(ethers.randomBytes(32));
      const word1 = ethers.toBigInt(ethers.randomBytes(32));

      await mock.fulfillRequest(requestId, [word0, word1]);

      expect(await source.hasResult(1n)).to.be.true;

      const result = await source.getResult(1n);
      expect(result.specialPrize).to.be.lte(99);
      expect(result.allPrizes).to.have.lengthOf(27);

      // allPrizes[0] should equal specialPrize
      expect(result.allPrizes[0]).to.equal(result.specialPrize);

      // All values in range 0-99
      for (const prize of result.allPrizes) {
        expect(prize).to.be.lte(99);
      }
    });

    it("emits ResultFulfilled with correct roundId", async function () {
      const { source, mock } = await deployWithMock();
      await source.requestResult(5n);

      const requestId = await source.roundToRequest(5n);
      const word0 = ethers.toBigInt(ethers.randomBytes(32));
      const word1 = ethers.toBigInt(ethers.randomBytes(32));

      await expect(mock.fulfillRequest(requestId, [word0, word1]))
        .to.emit(source, "ResultFulfilled")
        .withArgs(5n);
    });
  });

  describe("getResult", function () {
    it("reverts if round not fulfilled", async function () {
      const { source } = await deployWithMock();
      await source.requestResult(1n);

      await expect(source.getResult(1n))
        .to.be.revertedWithCustomError(source, "RoundNotFulfilled");
    });

    it("reverts for unknown round", async function () {
      const { source } = await deployWithMock();

      await expect(source.getResult(999n))
        .to.be.revertedWithCustomError(source, "RoundNotFulfilled");
    });
  });

  describe("hasResult", function () {
    it("returns false before fulfill", async function () {
      const { source } = await deployWithMock();
      expect(await source.hasResult(1n)).to.be.false;
    });
  });
});
```

- [ ] **Step 2: Create MockVRFCoordinator for tests**

The mock must implement `IVRFCoordinatorV2Plus.requestRandomWords(VRFV2PlusClient.RandomWordsRequest)`
because `VRFConsumerBaseV2Plus` calls `s_vrfCoordinator` which is typed to that interface.

Create `packages/contracts/contracts/test/MockVRFCoordinator.sol`:

```solidity
// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import {VRFV2PlusClient} from "@chainlink/contracts/src/v0.8/vrf/dev/libraries/VRFV2PlusClient.sol";

contract MockVRFCoordinator {
    uint256 public nextRequestId = 1;
    mapping(uint256 => address) public consumers;

    // Matches IVRFCoordinatorV2Plus.requestRandomWords signature
    function requestRandomWords(
        VRFV2PlusClient.RandomWordsRequest calldata /* req */
    ) external returns (uint256 requestId) {
        requestId = nextRequestId++;
        consumers[requestId] = msg.sender;
    }

    // Test helper: trigger fulfillment callback on consumer
    function fulfillRequest(uint256 requestId, uint256[] calldata randomWords) external {
        address consumer = consumers[requestId];
        require(consumer != address(0), "unknown request");
        (bool success, ) = consumer.call(
            abi.encodeWithSignature(
                "rawFulfillRandomWords(uint256,uint256[])",
                requestId,
                randomWords
            )
        );
        require(success, "fulfill failed");
    }
}
```

- [ ] **Step 3: Run tests to verify they fail**

Run: `cd packages/contracts && pnpm test -- --grep "ChainlinkVRFResultSource"`
Expected: FAIL — "ChainlinkVRFResultSource" contract not found

- [ ] **Step 4: Implement ChainlinkVRFResultSource**

Create `packages/contracts/contracts/results/ChainlinkVRFResultSource.sol`:

```solidity
// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import {VRFConsumerBaseV2Plus} from "@chainlink/contracts/src/v0.8/vrf/dev/VRFConsumerBaseV2Plus.sol";
import {VRFV2PlusClient} from "@chainlink/contracts/src/v0.8/vrf/dev/libraries/VRFV2PlusClient.sol";
import {IResultSource, LotteryResult} from "../interfaces/IResultSource.sol";

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

    constructor(
        address vrfCoordinator,
        bytes32 keyHash,
        uint256 subscriptionId
    ) VRFConsumerBaseV2Plus(vrfCoordinator) {
        i_keyHash = keyHash;
        i_subscriptionId = subscriptionId;
    }

    function requestResult(uint256 roundId) external {
        if (roundToRequest[roundId] != 0) revert RoundAlreadyRequested(roundId);

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

        roundToRequest[roundId] = requestId;
        requestToRound[requestId] = roundId;
        emit ResultRequested(roundId);
    }

    function fulfillRandomWords(
        uint256 requestId,
        uint256[] calldata randomWords
    ) internal override {
        uint256 roundId = requestToRound[requestId];
        if (roundId == 0) return; // silently ignore unknown requests

        uint8 specialPrize = uint8(
            uint256(keccak256(abi.encode(randomWords[0], uint256(0)))) % 100
        );

        uint8[] memory allPrizes = new uint8[](27);
        allPrizes[0] = specialPrize;
        for (uint256 i = 1; i < 27; ) {
            allPrizes[i] = uint8(
                uint256(keccak256(abi.encode(randomWords[0], randomWords[1], i))) % 100
            );
            unchecked { ++i; }
        }

        results[roundId] = LotteryResult({
            specialPrize: specialPrize,
            allPrizes: allPrizes
        });
        fulfilled[roundId] = true;

        emit ResultFulfilled(roundId);
    }

    function getResult(uint256 roundId) external view returns (LotteryResult memory) {
        if (!fulfilled[roundId]) revert RoundNotFulfilled(roundId);
        return results[roundId];
    }

    function hasResult(uint256 roundId) external view returns (bool) {
        return fulfilled[roundId];
    }
}
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `cd packages/contracts && pnpm test -- --grep "ChainlinkVRFResultSource"`
Expected: 6 passing

- [ ] **Step 6: Commit**

```bash
git add packages/contracts/contracts/results/ChainlinkVRFResultSource.sol packages/contracts/contracts/test/MockVRFCoordinator.sol packages/contracts/test/ChainlinkVRFResultSource.test.ts
git commit -m "feat(contracts): add ChainlinkVRFResultSource with VRF v2.5"
```

---

### Task 6: Deploy scripts

**Files:**
- Create: `packages/contracts/scripts/deploy-vrf-source.ts`
- Create: `packages/contracts/scripts/deploy-receiver.ts`

- [ ] **Step 1: Create Base Sepolia deploy script**

Create `packages/contracts/scripts/deploy-vrf-source.ts`:

```typescript
import { ethers } from "hardhat";

async function main() {
  const vrfCoordinator = process.env.VRF_COORDINATOR_ADDRESS;
  const keyHash = process.env.VRF_KEY_HASH;
  const subscriptionId = process.env.VRF_SUBSCRIPTION_ID;

  if (!vrfCoordinator || !keyHash || !subscriptionId) {
    throw new Error(
      "Required env vars: VRF_COORDINATOR_ADDRESS, VRF_KEY_HASH, VRF_SUBSCRIPTION_ID"
    );
  }

  const factory = await ethers.getContractFactory("ChainlinkVRFResultSource");
  const contract = await factory.deploy(vrfCoordinator, keyHash, subscriptionId);
  await contract.waitForDeployment();

  const address = await contract.getAddress();
  console.log(`ChainlinkVRFResultSource deployed to: ${address}`);
  console.log(`Add ${address} as consumer in VRF subscription dashboard`);
}

main().catch((error) => {
  console.error(error);
  process.exitCode = 1;
});
```

- [ ] **Step 2: Create Artemis deploy script**

Create `packages/contracts/scripts/deploy-receiver.ts`:

```typescript
import { ethers } from "hardhat";

async function main() {
  const trustedRelayer = process.env.TRUSTED_RELAYER_ADDRESS;

  if (!trustedRelayer) {
    throw new Error("Required env var: TRUSTED_RELAYER_ADDRESS");
  }

  // Deploy verifier
  const verifierFactory = await ethers.getContractFactory("SingleRelayerVerifier");
  const verifier = await verifierFactory.deploy(trustedRelayer);
  await verifier.waitForDeployment();
  const verifierAddress = await verifier.getAddress();
  console.log(`SingleRelayerVerifier deployed to: ${verifierAddress}`);

  // Deploy receiver
  const receiverFactory = await ethers.getContractFactory("CrossChainResultReceiver");
  const receiver = await receiverFactory.deploy(verifierAddress);
  await receiver.waitForDeployment();
  const receiverAddress = await receiver.getAddress();
  console.log(`CrossChainResultReceiver deployed to: ${receiverAddress}`);
}

main().catch((error) => {
  console.error(error);
  process.exitCode = 1;
});
```

- [ ] **Step 3: Verify scripts compile (dry run check)**

Run: `cd packages/contracts && npx hardhat compile`
Expected: Compilation finished successfully (scripts use deployed contract factories)

- [ ] **Step 4: Commit**

```bash
git add packages/contracts/scripts/deploy-vrf-source.ts packages/contracts/scripts/deploy-receiver.ts
git commit -m "feat(contracts): add deploy scripts for Base Sepolia and Artemis"
```

---

### Task 7: OZ Monitor configuration

**Files:**
- Create: `infra/oz-monitor/config.yaml`
- Create: `infra/oz-monitor/handlers/relay-to-base.ts`
- Create: `infra/oz-monitor/handlers/relay-to-artemis.ts`

- [ ] **Step 1: Create monitor config**

Create `infra/oz-monitor/config.yaml`:

```yaml
networks:
  artemis:
    type: evm
    chain_id: 322
    rpc_urls:
      - url: "${ARTEMIS_RPC_URL}"
        weight: 100
    average_blocktime_ms: 6000

  base_sepolia:
    type: evm
    chain_id: 84532
    rpc_urls:
      - url: "${BASE_SEPOLIA_RPC_URL}"
        weight: 100
    average_blocktime_ms: 2000

monitors:
  watch_artemis_requests:
    network: artemis
    contract_address: "${CROSS_CHAIN_RECEIVER_ADDRESS}"
    events:
      - signature: "ResultRequested(uint256)"
    handler: handlers/relay-to-base

  watch_base_results:
    network: base_sepolia
    contract_address: "${VRF_RESULT_SOURCE_ADDRESS}"
    events:
      - signature: "ResultFulfilled(uint256)"
    handler: handlers/relay-to-artemis
```

- [ ] **Step 2: Create relay-to-base handler**

Create `infra/oz-monitor/handlers/relay-to-base.ts`:

```typescript
import { ethers } from "ethers";

const VRF_SOURCE_ABI = [
  "function requestResult(uint256 roundId) external",
];

interface MonitorEvent {
  event: string;
  args: {
    roundId: bigint;
  };
}

interface RelayerClient {
  sendTransaction(params: {
    to: string;
    data: string;
    relayer: string;
  }): Promise<{ hash: string }>;
}

export async function handle(
  event: MonitorEvent,
  relayer: RelayerClient
): Promise<void> {
  const roundId = event.args.roundId;
  const vrfSourceAddress = process.env.VRF_RESULT_SOURCE_ADDRESS;

  if (!vrfSourceAddress) {
    throw new Error("VRF_RESULT_SOURCE_ADDRESS not set");
  }

  const iface = new ethers.Interface(VRF_SOURCE_ABI);
  const data = iface.encodeFunctionData("requestResult", [roundId]);

  const tx = await relayer.sendTransaction({
    to: vrfSourceAddress,
    data,
    relayer: "base_signer",
  });

  console.log(`Relayed requestResult(${roundId}) to Base: ${tx.hash}`);
}
```

- [ ] **Step 3: Create relay-to-artemis handler**

Create `infra/oz-monitor/handlers/relay-to-artemis.ts`:

```typescript
import { ethers } from "ethers";

const VRF_SOURCE_ABI = [
  "function getResult(uint256 roundId) external view returns (tuple(uint8 specialPrize, uint8[] allPrizes))",
];

const RECEIVER_ABI = [
  "function submitResult(uint256 roundId, tuple(uint8 specialPrize, uint8[] allPrizes) result, bytes proof) external",
];

interface MonitorEvent {
  event: string;
  args: {
    roundId: bigint;
  };
  network: string;
}

interface RelayerClient {
  sendTransaction(params: {
    to: string;
    data: string;
    relayer: string;
  }): Promise<{ hash: string }>;
  getSigner(relayer: string): ethers.Wallet;
}

export async function handle(
  event: MonitorEvent,
  relayer: RelayerClient
): Promise<void> {
  const roundId = event.args.roundId;
  const vrfSourceAddress = process.env.VRF_RESULT_SOURCE_ADDRESS;
  const receiverAddress = process.env.CROSS_CHAIN_RECEIVER_ADDRESS;
  const baseRpcUrl = process.env.BASE_SEPOLIA_RPC_URL;

  if (!vrfSourceAddress || !receiverAddress || !baseRpcUrl) {
    throw new Error(
      "Required: VRF_RESULT_SOURCE_ADDRESS, CROSS_CHAIN_RECEIVER_ADDRESS, BASE_SEPOLIA_RPC_URL"
    );
  }

  // Read result from Base
  const baseProvider = new ethers.JsonRpcProvider(baseRpcUrl);
  const vrfSource = new ethers.Contract(vrfSourceAddress, VRF_SOURCE_ABI, baseProvider);
  const result = await vrfSource.getResult(roundId);

  // Sign the result
  const artemisSigner = relayer.getSigner("artemis_signer");
  const hash = ethers.keccak256(
    ethers.AbiCoder.defaultAbiCoder().encode(
      ["uint256", "uint8", "uint8[]"],
      [roundId, result.specialPrize, result.allPrizes]
    )
  );
  const signature = await artemisSigner.signMessage(ethers.getBytes(hash));

  // Submit to Artemis
  const receiverIface = new ethers.Interface(RECEIVER_ABI);
  const data = receiverIface.encodeFunctionData("submitResult", [
    roundId,
    { specialPrize: result.specialPrize, allPrizes: result.allPrizes },
    signature,
  ]);

  const tx = await relayer.sendTransaction({
    to: receiverAddress,
    data,
    relayer: "artemis_signer",
  });

  console.log(`Relayed result for round ${roundId} to Artemis: ${tx.hash}`);
}
```

- [ ] **Step 4: Commit**

```bash
git add infra/oz-monitor/
git commit -m "feat(infra): add OZ Monitor config and cross-chain handlers"
```

---

### Task 8: OZ Relayer configuration and Docker Compose

**Files:**
- Create: `infra/oz-relayer/config.yaml`
- Create: `infra/docker-compose.yml`
- Create: `infra/.env.example`

- [ ] **Step 1: Create relayer config**

Create `infra/oz-relayer/config.yaml`:

```yaml
networks:
  artemis:
    type: evm
    chain_id: 322
    rpc_urls:
      - url: "${ARTEMIS_RPC_URL}"
        weight: 100
    average_blocktime_ms: 6000
    is_testnet: true

  base_sepolia:
    type: evm
    chain_id: 84532
    rpc_urls:
      - url: "${BASE_SEPOLIA_RPC_URL}"
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

- [ ] **Step 2: Create docker-compose.yml**

Create `infra/docker-compose.yml`:

```yaml
services:
  oz-monitor:
    image: openzeppelin/monitor:latest
    restart: unless-stopped
    volumes:
      - ./oz-monitor/config.yaml:/app/config.yaml:ro
      - ./oz-monitor/handlers:/app/handlers:ro
    env_file:
      - .env
    depends_on:
      - oz-relayer

  oz-relayer:
    image: openzeppelin/relayer:latest
    restart: unless-stopped
    volumes:
      - ./oz-relayer/config.yaml:/app/config.yaml:ro
      - relayer-keys:/app/keys:ro
    env_file:
      - .env
    ports:
      - "8080:8080"

volumes:
  relayer-keys:
```

- [ ] **Step 3: Create .env.example**

Create `infra/.env.example`:

```bash
# RPC endpoints
ARTEMIS_RPC_URL=http://127.0.0.1:9944
BASE_SEPOLIA_RPC_URL=https://sepolia.base.org

# Contract addresses (set after deployment)
CROSS_CHAIN_RECEIVER_ADDRESS=
VRF_RESULT_SOURCE_ADDRESS=

# Relayer key paths (inside container)
BASE_RELAYER_KEY_PATH=/app/keys/base-relayer.json
ARTEMIS_RELAYER_KEY_PATH=/app/keys/artemis-relayer.json

# Chainlink VRF (Base Sepolia)
VRF_COORDINATOR_ADDRESS=
VRF_KEY_HASH=
VRF_SUBSCRIPTION_ID=

# Deployer key (Base Sepolia)
BASE_DEPLOYER_KEY=

# Trusted relayer address (Artemis signer public address)
TRUSTED_RELAYER_ADDRESS=
```

- [ ] **Step 4: Add infra to .gitignore (exclude .env, include .env.example)**

Verify `.env` is in `.gitignore` at repo root. If not present, add:

```
infra/.env
```

- [ ] **Step 5: Commit**

```bash
git add infra/oz-relayer/config.yaml infra/docker-compose.yml infra/.env.example
git commit -m "feat(infra): add OZ Relayer config and Docker Compose"
```

---

### Task 9: Run all tests and verify

**Files:** None (verification only)

- [ ] **Step 1: Run full test suite**

Run: `cd packages/contracts && pnpm test`
Expected: All tests pass (SingleRelayerVerifier + CrossChainResultReceiver + ChainlinkVRFResultSource + existing gasless tests)

- [ ] **Step 2: Verify compilation is clean**

Run: `cd packages/contracts && pnpm compile`
Expected: Compilation finished successfully, no warnings

- [ ] **Step 3: Verify turbo build still works**

Run: `pnpm turbo build`
Expected: All packages build successfully

- [ ] **Step 4: Final commit if any formatting/cleanup needed**

If any files need formatting fixes:
```bash
git add -A && git commit -m "chore(contracts): formatting cleanup"
```
