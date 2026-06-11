# Artemis Next Steps Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add admin panel, expand tests, prepare Docker testnet deployment.

**Architecture:** Shared ABI/types in `@artemis/shared`, Ponder GraphQL for reads, precompile via wagmi for writes. Docker Compose for multi-node testnet.

**Tech Stack:** React 19, wagmi v2, Ponder v0.9, Hardhat, Playwright + dappwright, Docker

---

### Task 1: Shared ABI and types

**Files:**
- Create: `packages/shared/src/abis/GaslessRegistry.ts`
- Create: `packages/shared/src/types.ts`
- Modify: `packages/shared/src/index.ts`
- Modify: `packages/shared/src/constants/addresses.ts`
- Modify: `packages/indexer/abis/GaslessRegistry.ts`

- [ ] **Step 1: Create shared ABI file**

Create `packages/shared/src/abis/GaslessRegistry.ts`:

```typescript
export const GaslessRegistryAbi = [
  {
    type: "event",
    name: "RuleSet",
    inputs: [
      { name: "contract_", type: "address", indexed: true },
      { name: "selector", type: "bytes4", indexed: true },
      { name: "enabled", type: "bool", indexed: false },
      { name: "minValue", type: "uint256", indexed: false },
    ],
  },
  {
    type: "event",
    name: "RuleRemoved",
    inputs: [
      { name: "contract_", type: "address", indexed: true },
      { name: "selector", type: "bytes4", indexed: true },
    ],
  },
  {
    type: "function",
    name: "getRule",
    stateMutability: "view",
    inputs: [
      { name: "contract_", type: "address" },
      { name: "selector", type: "bytes4" },
    ],
    outputs: [
      { name: "enabled", type: "bool" },
      { name: "minValue", type: "uint256" },
    ],
  },
  {
    type: "function",
    name: "isGasless",
    stateMutability: "view",
    inputs: [
      { name: "contract_", type: "address" },
      { name: "input", type: "bytes" },
      { name: "value", type: "uint256" },
      { name: "gasLimit", type: "uint256" },
    ],
    outputs: [{ name: "", type: "bool" }],
  },
  {
    type: "function",
    name: "setRule",
    stateMutability: "nonpayable",
    inputs: [
      { name: "contract_", type: "address" },
      { name: "selector", type: "bytes4" },
      { name: "minValue", type: "uint256" },
      { name: "enabled", type: "bool" },
    ],
    outputs: [],
  },
  {
    type: "function",
    name: "removeRule",
    stateMutability: "nonpayable",
    inputs: [
      { name: "contract_", type: "address" },
      { name: "selector", type: "bytes4" },
    ],
    outputs: [],
  },
] as const;
```

- [ ] **Step 2: Create shared types file**

Create `packages/shared/src/types.ts`:

```typescript
import type { Address, Hex } from "viem";

export interface GaslessRule {
  id: string;
  contract: Address;
  selector: Hex;
  enabled: boolean;
  minValue: bigint;
  updatedAtBlock: bigint;
}
```

Note: `@artemis/shared` does not have `viem` as a dependency. Add it:

```bash
cd packages/shared && pnpm add -D viem
```

- [ ] **Step 3: Update shared exports**

Update `packages/shared/src/index.ts`:

```typescript
export {
  CHAIN_CONFIG,
  GASLESS_REGISTRY_ADDRESS,
  NATIVE_TOKEN_ADDRESS,
} from "./constants";
export { GaslessRegistryAbi } from "./abis/GaslessRegistry";
export type { GaslessRule } from "./types";
```

- [ ] **Step 4: Add SUDO_ADDRESS to shared constants**

Append to `packages/shared/src/constants/addresses.ts`:

```typescript
export const SUDO_ADDRESS = "0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266" as const;
```

Update `packages/shared/src/constants/index.ts`:

```typescript
export { GASLESS_REGISTRY_ADDRESS, NATIVE_TOKEN_ADDRESS, SUDO_ADDRESS } from "./addresses";
export { CHAIN_CONFIG } from "./chain";
```

Update `packages/shared/src/index.ts` to include `SUDO_ADDRESS`:

```typescript
export {
  CHAIN_CONFIG,
  GASLESS_REGISTRY_ADDRESS,
  NATIVE_TOKEN_ADDRESS,
  SUDO_ADDRESS,
} from "./constants";
export { GaslessRegistryAbi } from "./abis/GaslessRegistry";
export type { GaslessRule } from "./types";
```

- [ ] **Step 5: Update indexer to re-export from shared**

Replace `packages/indexer/abis/GaslessRegistry.ts` with:

```typescript
export { GaslessRegistryAbi } from "@artemis/shared";
```

- [ ] **Step 6: Build and verify**

Run: `pnpm turbo build --filter=@artemis/shared --filter=@artemis/indexer`
Expected: Both build successfully.

- [ ] **Step 7: Commit**

```bash
git add packages/shared packages/indexer/abis/GaslessRegistry.ts
git commit -m "feat(shared): add gasless registry ABI, types, and sudo address"
```

---

### Task 2: Indexer environment setup

**Files:**
- Create: `packages/indexer/.env.example`
- Modify: `packages/indexer/.env.local`

- [ ] **Step 1: Create .env.example**

Create `packages/indexer/.env.example`:

```
PONDER_RPC_URL_322=http://127.0.0.1:9944
DATABASE_URL=postgresql://postgres:postgres@localhost:5432/artemis_indexer
```

- [ ] **Step 2: Fix database name in .env.local**

Update `packages/indexer/.env.local` — change `betting_indexer` to `artemis_indexer`:

```
DATABASE_URL=postgresql://postgres:postgres@localhost:5432/artemis_indexer
PONDER_RPC_URL_322=http://127.0.0.1:9944
```

- [ ] **Step 3: Commit**

```bash
git add packages/indexer/.env.example packages/indexer/.env.local
git commit -m "chore(indexer): add .env.example, rename database to artemis_indexer"
```

---

### Task 3: Admin page — nav link and route

**Files:**
- Modify: `packages/webapp/src/App.tsx`
- Modify: `packages/webapp/src/components/layout/AppLayout.tsx`
- Create: `packages/webapp/src/pages/Admin.tsx`

- [ ] **Step 1: Add Admin route to App.tsx**

Replace `packages/webapp/src/App.tsx`:

```typescript
import { Routes, Route } from "react-router";
import { AppLayout } from "@/components/layout/AppLayout";
import { Home } from "@/pages/Home";
import { Admin } from "@/pages/Admin";

export default function App() {
  return (
    <Routes>
      <Route element={<AppLayout />}>
        <Route index element={<Home />} />
        <Route path="admin" element={<Admin />} />
      </Route>
    </Routes>
  );
}
```

- [ ] **Step 2: Add Admin nav link to AppLayout**

Replace `packages/webapp/src/components/layout/AppLayout.tsx`:

```typescript
import { ConnectButton } from "@rainbow-me/rainbowkit";
import { NavLink, Outlet } from "react-router";
import { cn } from "@/lib/utils";

function NavItem({ to, children }: { to: string; children: React.ReactNode }) {
  return (
    <NavLink
      to={to}
      className={({ isActive }) =>
        cn("text-sm", isActive ? "text-foreground font-medium" : "text-muted-foreground hover:text-foreground")
      }
    >
      {children}
    </NavLink>
  );
}

export function AppLayout() {
  return (
    <div className="min-h-screen bg-background text-foreground">
      <header className="border-b border-border">
        <nav aria-label="Main navigation" className="mx-auto flex max-w-5xl items-center justify-between px-4 py-3">
          <div className="flex items-center gap-6">
            <NavLink to="/" className="text-lg font-bold">
              Artemis
            </NavLink>
            <NavItem to="/">Home</NavItem>
            <NavItem to="/admin">Admin</NavItem>
          </div>
          <ConnectButton />
        </nav>
      </header>
      <main className="mx-auto max-w-5xl px-4 py-8">
        <Outlet />
      </main>
    </div>
  );
}
```

- [ ] **Step 3: Create placeholder Admin page (will be filled in next tasks)**

Create `packages/webapp/src/pages/Admin.tsx`:

```typescript
import { useAccount } from "wagmi";
import { SUDO_ADDRESS } from "@artemis/shared";
import { AdminGuard } from "@/components/admin/AdminGuard";

export function Admin() {
  return (
    <AdminGuard>
      <div className="flex flex-col gap-6">
        <h1 className="text-2xl font-bold">Gasless Registry Admin</h1>
        <p className="text-muted-foreground">Loading...</p>
      </div>
    </AdminGuard>
  );
}
```

- [ ] **Step 4: Create AdminGuard component**

Create `packages/webapp/src/components/admin/AdminGuard.tsx`:

```typescript
import type { ReactNode } from "react";
import { useAccount } from "wagmi";
import { ConnectButton } from "@rainbow-me/rainbowkit";
import { SUDO_ADDRESS } from "@artemis/shared";

interface AdminGuardProps {
  children: ReactNode;
}

export function AdminGuard({ children }: AdminGuardProps) {
  const { address, isConnected } = useAccount();

  if (!isConnected) {
    return (
      <div className="flex flex-col items-center gap-4 pt-24">
        <p className="text-muted-foreground">Connect your wallet to access admin panel</p>
        <ConnectButton />
      </div>
    );
  }

  if (address?.toLowerCase() !== SUDO_ADDRESS.toLowerCase()) {
    return (
      <div className="flex flex-col items-center gap-4 pt-24" data-testid="admin-unauthorized">
        <h2 className="text-xl font-bold">Unauthorized</h2>
        <p className="text-muted-foreground">
          Only the sudo account can access this page.
        </p>
      </div>
    );
  }

  return <>{children}</>;
}
```

- [ ] **Step 5: Build and verify**

Run: `cd packages/webapp && pnpm build`
Expected: Build succeeds.

- [ ] **Step 6: Commit**

```bash
git add packages/webapp/src/App.tsx packages/webapp/src/components/layout/AppLayout.tsx packages/webapp/src/pages/Admin.tsx packages/webapp/src/components/admin/AdminGuard.tsx
git commit -m "feat(webapp): add admin route with sudo-only guard"
```

---

### Task 4: Admin page — RuleList component

**Files:**
- Create: `packages/webapp/src/components/admin/RuleList.tsx`
- Create: `packages/webapp/src/lib/graphql.ts`

- [ ] **Step 1: Create GraphQL helper**

Create `packages/webapp/src/lib/graphql.ts`:

```typescript
const INDEXER_URL = import.meta.env.VITE_INDEXER_URL ?? "http://localhost:42069/graphql";

export async function queryIndexer<T>(query: string, variables?: Record<string, unknown>): Promise<T> {
  const response = await fetch(INDEXER_URL, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ query, variables }),
  });

  if (!response.ok) {
    throw new Error(`Indexer request failed: ${response.status}`);
  }

  const json = await response.json();
  if (json.errors) {
    throw new Error(json.errors[0]?.message ?? "GraphQL error");
  }

  return json.data as T;
}
```

- [ ] **Step 2: Create RuleList component**

Create `packages/webapp/src/components/admin/RuleList.tsx`:

```typescript
import { useCallback, useEffect, useState } from "react";
import { useWriteContract, useWaitForTransactionReceipt } from "wagmi";
import { GASLESS_REGISTRY_ADDRESS, GaslessRegistryAbi } from "@artemis/shared";
import type { GaslessRule } from "@artemis/shared";
import { Button } from "@/components/ui/button";
import { queryIndexer } from "@/lib/graphql";

const RULES_QUERY = `
  query {
    gasless_ruless {
      items {
        id
        contract
        selector
        enabled
        minValue
        updatedAtBlock
      }
    }
  }
`;

interface RulesResponse {
  gasless_ruless: {
    items: GaslessRule[];
  };
}

interface RuleListProps {
  refreshKey: number;
}

export function RuleList({ refreshKey }: RuleListProps) {
  const [rules, setRules] = useState<GaslessRule[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const { writeContract, data: txHash, isPending } = useWriteContract();
  const { isLoading: isConfirming, isSuccess: isConfirmed } = useWaitForTransactionReceipt({
    hash: txHash,
  });

  const fetchRules = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const data = await queryIndexer<RulesResponse>(RULES_QUERY);
      setRules(data.gasless_ruless.items.filter((r) => r.enabled));
    } catch (e) {
      setError(e instanceof Error ? e.message : "Failed to fetch rules");
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    fetchRules();
  }, [fetchRules, refreshKey]);

  useEffect(() => {
    if (isConfirmed) {
      // Wait for indexer to catch up then refresh
      const timeout = setTimeout(fetchRules, 3000);
      return () => clearTimeout(timeout);
    }
  }, [isConfirmed, fetchRules]);

  function handleToggle(rule: GaslessRule) {
    writeContract({
      address: GASLESS_REGISTRY_ADDRESS,
      abi: GaslessRegistryAbi,
      functionName: "setRule",
      args: [rule.contract, rule.selector, BigInt(rule.minValue), !rule.enabled],
    });
  }

  function handleRemove(rule: GaslessRule) {
    if (!confirm(`Remove rule for ${rule.contract} / ${rule.selector}?`)) return;
    writeContract({
      address: GASLESS_REGISTRY_ADDRESS,
      abi: GaslessRegistryAbi,
      functionName: "removeRule",
      args: [rule.contract, rule.selector],
    });
  }

  if (loading) return <p className="text-muted-foreground">Loading rules...</p>;
  if (error) return <p className="text-red-500">Error: {error}</p>;
  if (rules.length === 0) return <p className="text-muted-foreground">No gasless rules configured.</p>;

  return (
    <div className="overflow-x-auto">
      {(isPending || isConfirming) && (
        <p className="mb-2 text-sm text-muted-foreground">
          {isPending ? "Confirm in wallet..." : "Waiting for confirmation..."}
        </p>
      )}
      <table className="w-full text-sm" data-testid="rule-list">
        <thead>
          <tr className="border-b border-border text-left text-muted-foreground">
            <th className="pb-2 pr-4">Contract</th>
            <th className="pb-2 pr-4">Selector</th>
            <th className="pb-2 pr-4">Min Value (ART)</th>
            <th className="pb-2 pr-4">Status</th>
            <th className="pb-2">Actions</th>
          </tr>
        </thead>
        <tbody>
          {rules.map((rule) => (
            <tr key={rule.id} className="border-b border-border">
              <td className="py-3 pr-4 font-mono text-xs">{rule.contract}</td>
              <td className="py-3 pr-4 font-mono text-xs">{rule.selector}</td>
              <td className="py-3 pr-4">{String(rule.minValue)}</td>
              <td className="py-3 pr-4">
                <span className={rule.enabled ? "text-green-600" : "text-red-500"}>
                  {rule.enabled ? "Enabled" : "Disabled"}
                </span>
              </td>
              <td className="flex gap-2 py-3">
                <Button size="sm" variant="outline" onClick={() => handleToggle(rule)}>
                  {rule.enabled ? "Disable" : "Enable"}
                </Button>
                <Button size="sm" variant="ghost" className="text-red-500" onClick={() => handleRemove(rule)}>
                  Remove
                </Button>
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}
```

- [ ] **Step 3: Commit**

```bash
git add packages/webapp/src/lib/graphql.ts packages/webapp/src/components/admin/RuleList.tsx
git commit -m "feat(webapp): add RuleList component with GraphQL fetch"
```

---

### Task 5: Admin page — AddRuleForm and final composition

**Files:**
- Create: `packages/webapp/src/components/admin/AddRuleForm.tsx`
- Modify: `packages/webapp/src/pages/Admin.tsx`

- [ ] **Step 1: Create AddRuleForm component**

Create `packages/webapp/src/components/admin/AddRuleForm.tsx`:

```typescript
import { type FormEvent, useState } from "react";
import { isAddress, isHex } from "viem";
import { parseEther } from "viem";
import { useWriteContract, useWaitForTransactionReceipt } from "wagmi";
import { GASLESS_REGISTRY_ADDRESS, GaslessRegistryAbi } from "@artemis/shared";
import { Button } from "@/components/ui/button";

interface AddRuleFormProps {
  onSuccess: () => void;
}

export function AddRuleForm({ onSuccess }: AddRuleFormProps) {
  const [contractAddr, setContractAddr] = useState("");
  const [selector, setSelector] = useState("");
  const [minValue, setMinValue] = useState("0");
  const [enabled, setEnabled] = useState(true);
  const [validationError, setValidationError] = useState<string | null>(null);

  const { writeContract, data: txHash, isPending, error: writeError } = useWriteContract();
  const { isLoading: isConfirming, isSuccess: isConfirmed } = useWaitForTransactionReceipt({
    hash: txHash,
  });

  if (isConfirmed) {
    // Reset form and notify parent
    setTimeout(() => {
      setContractAddr("");
      setSelector("");
      setMinValue("0");
      setEnabled(true);
      onSuccess();
    }, 0);
  }

  function handleSubmit(e: FormEvent) {
    e.preventDefault();
    setValidationError(null);

    if (!isAddress(contractAddr)) {
      setValidationError("Invalid contract address");
      return;
    }

    const selectorHex = selector.startsWith("0x") ? selector : `0x${selector}`;
    if (!isHex(selectorHex) || selectorHex.length !== 10) {
      setValidationError("Selector must be 4 bytes (e.g. 0xa9059cbb)");
      return;
    }

    const minValueWei = parseEther(minValue || "0");

    writeContract({
      address: GASLESS_REGISTRY_ADDRESS,
      abi: GaslessRegistryAbi,
      functionName: "setRule",
      args: [contractAddr, selectorHex as `0x${string}`, minValueWei, enabled],
    });
  }

  return (
    <form onSubmit={handleSubmit} className="flex flex-col gap-4 rounded-lg border border-border p-4">
      <h2 className="font-medium">Add Gasless Rule</h2>

      <div className="grid gap-4 sm:grid-cols-2">
        <div className="flex flex-col gap-1">
          <label htmlFor="contract" className="text-sm text-muted-foreground">Contract Address</label>
          <input
            id="contract"
            type="text"
            value={contractAddr}
            onChange={(e) => setContractAddr(e.target.value)}
            placeholder="0x..."
            className="rounded border border-border bg-background px-3 py-2 text-sm font-mono"
            required
          />
        </div>
        <div className="flex flex-col gap-1">
          <label htmlFor="selector" className="text-sm text-muted-foreground">Function Selector (bytes4)</label>
          <input
            id="selector"
            type="text"
            value={selector}
            onChange={(e) => setSelector(e.target.value)}
            placeholder="0xa9059cbb"
            className="rounded border border-border bg-background px-3 py-2 text-sm font-mono"
            required
          />
        </div>
        <div className="flex flex-col gap-1">
          <label htmlFor="minValue" className="text-sm text-muted-foreground">Min Value (ART)</label>
          <input
            id="minValue"
            type="number"
            step="any"
            min="0"
            value={minValue}
            onChange={(e) => setMinValue(e.target.value)}
            className="rounded border border-border bg-background px-3 py-2 text-sm"
          />
        </div>
        <div className="flex items-end gap-2 pb-1">
          <label className="flex items-center gap-2 text-sm">
            <input
              type="checkbox"
              checked={enabled}
              onChange={(e) => setEnabled(e.target.checked)}
              className="h-4 w-4"
            />
            Enabled
          </label>
        </div>
      </div>

      {validationError && <p className="text-sm text-red-500">{validationError}</p>}
      {writeError && <p className="text-sm text-red-500">{writeError.message}</p>}
      {isPending && <p className="text-sm text-muted-foreground">Confirm in wallet...</p>}
      {isConfirming && <p className="text-sm text-muted-foreground">Waiting for confirmation...</p>}

      <Button type="submit" disabled={isPending || isConfirming}>
        Add Rule
      </Button>
    </form>
  );
}
```

- [ ] **Step 2: Update Admin page to compose components**

Replace `packages/webapp/src/pages/Admin.tsx`:

```typescript
import { useState } from "react";
import { AdminGuard } from "@/components/admin/AdminGuard";
import { RuleList } from "@/components/admin/RuleList";
import { AddRuleForm } from "@/components/admin/AddRuleForm";

export function Admin() {
  const [refreshKey, setRefreshKey] = useState(0);

  return (
    <AdminGuard>
      <div className="flex flex-col gap-6">
        <h1 className="text-2xl font-bold">Gasless Registry Admin</h1>
        <AddRuleForm onSuccess={() => setRefreshKey((k) => k + 1)} />
        <RuleList refreshKey={refreshKey} />
      </div>
    </AdminGuard>
  );
}
```

- [ ] **Step 3: Build and verify**

Run: `cd packages/webapp && pnpm build`
Expected: Build succeeds.

- [ ] **Step 4: Commit**

```bash
git add packages/webapp/src/components/admin/AddRuleForm.tsx packages/webapp/src/pages/Admin.tsx
git commit -m "feat(webapp): add admin panel with rule management"
```

---

### Task 6: TestNFT contract

**Files:**
- Create: `packages/contracts/contracts/TestNFT.sol`

- [ ] **Step 1: Create TestNFT.sol**

Create `packages/contracts/contracts/TestNFT.sol`:

```solidity
// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import "@openzeppelin/contracts/token/ERC721/ERC721.sol";

contract TestNFT is ERC721 {
    constructor() ERC721("TestNFT", "TNFT") {}

    function mint(address to, uint256 tokenId) external {
        _mint(to, tokenId);
    }
}
```

- [ ] **Step 2: Compile**

Run: `cd packages/contracts && pnpm build`
Expected: Compiles successfully.

- [ ] **Step 3: Commit**

```bash
git add packages/contracts/contracts/TestNFT.sol
git commit -m "feat(contracts): add TestNFT ERC721 for gasless testing"
```

---

### Task 7: Gasless ERC721 tests

**Files:**
- Create: `packages/contracts/test/GaslessERC721.test.ts`

- [ ] **Step 1: Create ERC721 gasless test file**

Create `packages/contracts/test/GaslessERC721.test.ts`:

```typescript
import { expect } from "chai";
import { ethers } from "hardhat";
import { ApiPromise, WsProvider, Keyring } from "@polkadot/api";
import { DEV_ACCOUNTS, getBalance } from "./helpers/setup";

// transferFrom(address,address,uint256) selector
const TRANSFER_FROM_SELECTOR = "0x23b872dd";

describe("GaslessERC721", function () {
  this.timeout(120_000);

  let api: ApiPromise;

  before(async function () {
    try {
      await ethers.provider.getBlockNumber();
    } catch {
      this.skip();
    }
    const provider = new WsProvider("ws://127.0.0.1:9944");
    api = await ApiPromise.create({ provider });
  });

  after(async function () {
    if (api) await api.disconnect();
  });

  async function registerGaslessRule(
    contractAddress: string,
    selector: string,
    minValue: bigint,
    enabled: boolean,
  ): Promise<void> {
    const keyring = new Keyring({ type: "ethereum" });
    const alice = keyring.addFromUri(DEV_ACCOUNTS.alice.privateKey);

    const selectorBytes = selector.replace("0x", "");
    const setRuleCall = api.tx.gaslessRegistry.setRule(
      contractAddress,
      `0x${selectorBytes}`,
      minValue,
      enabled,
    );
    const sudoCall = api.tx.sudo.sudo(setRuleCall);

    await new Promise<void>((resolve, reject) => {
      sudoCall.signAndSend(alice, ({ status, dispatchError }) => {
        if (dispatchError) reject(new Error(dispatchError.toString()));
        if (status.isInBlock || status.isFinalized) resolve();
      });
    });
    await new Promise((r) => setTimeout(r, 2000));
  }

  it("transfers NFT gaslessly when transferFrom is registered", async function () {
    const alice = new ethers.Wallet(DEV_ACCOUNTS.alice.privateKey, ethers.provider);
    const bob = new ethers.Wallet(DEV_ACCOUNTS.bob.privateKey, ethers.provider);

    // Deploy TestNFT
    const factory = await ethers.getContractFactory("TestNFT", alice);
    const nft = await factory.deploy();
    await nft.waitForDeployment();
    const nftAddress = await nft.getAddress();

    // Mint token #1 to Bob
    const mintTx = await nft.mint(bob.address, 1);
    await mintTx.wait();

    // Register transferFrom as gasless
    await registerGaslessRule(nftAddress, TRANSFER_FROM_SELECTOR, 0n, true);

    // Bob transfers NFT to Charlie — should be gasless
    const bobNft = nft.connect(bob);
    const charlie = DEV_ACCOUNTS.charlie.address;
    const bobBalanceBefore = await getBalance(bob.address);

    const tx = await bobNft.transferFrom(bob.address, charlie, 1, { gasLimit: 100_000 });
    const receipt = await tx.wait();
    expect(receipt?.status).to.equal(1);

    const bobBalanceAfter = await getBalance(bob.address);
    expect(bobBalanceBefore - bobBalanceAfter).to.equal(0n);

    // Verify NFT ownership changed
    expect(await nft.ownerOf(1)).to.equal(charlie);
  });
});
```

- [ ] **Step 2: Verify tests pass (requires live dev node)**

Run: `cd packages/contracts && pnpm test --grep GaslessERC721`
Expected: 1 test passes.

- [ ] **Step 3: Commit**

```bash
git add packages/contracts/test/GaslessERC721.test.ts
git commit -m "test(contracts): add gasless ERC721 transferFrom test"
```

---

### Task 8: Gasless edge case tests

**Files:**
- Create: `packages/contracts/test/GaslessEdgeCases.test.ts`

- [ ] **Step 1: Create edge case test file**

Create `packages/contracts/test/GaslessEdgeCases.test.ts`:

```typescript
import { expect } from "chai";
import { ethers } from "hardhat";
import { ApiPromise, WsProvider, Keyring } from "@polkadot/api";
import { DEV_ACCOUNTS, getBalance } from "./helpers/setup";

const TRANSFER_SELECTOR = "0xa9059cbb";
const APPROVE_SELECTOR = "0x095ea7b3";
const SUPPLY = ethers.parseEther("1000000");

describe("GaslessEdgeCases", function () {
  this.timeout(120_000);

  let api: ApiPromise;

  before(async function () {
    try {
      await ethers.provider.getBlockNumber();
    } catch {
      this.skip();
    }
    const provider = new WsProvider("ws://127.0.0.1:9944");
    api = await ApiPromise.create({ provider });
  });

  after(async function () {
    if (api) await api.disconnect();
  });

  async function sudoSetRule(
    contractAddress: string,
    selector: string,
    minValue: bigint,
    enabled: boolean,
  ): Promise<void> {
    const keyring = new Keyring({ type: "ethereum" });
    const alice = keyring.addFromUri(DEV_ACCOUNTS.alice.privateKey);
    const selectorBytes = selector.replace("0x", "");
    const call = api.tx.gaslessRegistry.setRule(contractAddress, `0x${selectorBytes}`, minValue, enabled);
    const sudoCall = api.tx.sudo.sudo(call);
    await new Promise<void>((resolve, reject) => {
      sudoCall.signAndSend(alice, ({ status, dispatchError }) => {
        if (dispatchError) reject(new Error(dispatchError.toString()));
        if (status.isInBlock || status.isFinalized) resolve();
      });
    });
    await new Promise((r) => setTimeout(r, 2000));
  }

  async function sudoRemoveRule(contractAddress: string, selector: string): Promise<void> {
    const keyring = new Keyring({ type: "ethereum" });
    const alice = keyring.addFromUri(DEV_ACCOUNTS.alice.privateKey);
    const selectorBytes = selector.replace("0x", "");
    const call = api.tx.gaslessRegistry.removeRule(contractAddress, `0x${selectorBytes}`);
    const sudoCall = api.tx.sudo.sudo(call);
    await new Promise<void>((resolve, reject) => {
      sudoCall.signAndSend(alice, ({ status, dispatchError }) => {
        if (dispatchError) reject(new Error(dispatchError.toString()));
        if (status.isInBlock || status.isFinalized) resolve();
      });
    });
    await new Promise((r) => setTimeout(r, 2000));
  }

  it("falls back to paid when rule is disabled", async function () {
    const bob = new ethers.Wallet(DEV_ACCOUNTS.bob.privateKey, ethers.provider);
    const factory = await ethers.getContractFactory("TestToken", bob);
    const token = await factory.deploy(SUPPLY);
    await token.waitForDeployment();
    const tokenAddress = await token.getAddress();

    // Register then disable
    await sudoSetRule(tokenAddress, TRANSFER_SELECTOR, 0n, true);
    await sudoSetRule(tokenAddress, TRANSFER_SELECTOR, 0n, false);

    const bobBalanceBefore = await getBalance(bob.address);
    const tx = await token.transfer(DEV_ACCOUNTS.charlie.address, ethers.parseEther("10"), { gasLimit: 100_000 });
    await tx.wait();
    const bobBalanceAfter = await getBalance(bob.address);

    expect(bobBalanceBefore - bobBalanceAfter).to.be.greaterThan(0n);
  });

  it("falls back to paid when rule is removed", async function () {
    const bob = new ethers.Wallet(DEV_ACCOUNTS.bob.privateKey, ethers.provider);
    const factory = await ethers.getContractFactory("TestToken", bob);
    const token = await factory.deploy(SUPPLY);
    await token.waitForDeployment();
    const tokenAddress = await token.getAddress();

    await sudoSetRule(tokenAddress, TRANSFER_SELECTOR, 0n, true);
    await sudoRemoveRule(tokenAddress, TRANSFER_SELECTOR);

    const bobBalanceBefore = await getBalance(bob.address);
    const tx = await token.transfer(DEV_ACCOUNTS.charlie.address, ethers.parseEther("10"), { gasLimit: 100_000 });
    await tx.wait();
    const bobBalanceAfter = await getBalance(bob.address);

    expect(bobBalanceBefore - bobBalanceAfter).to.be.greaterThan(0n);
  });

  it("evaluates multiple selectors independently on same contract", async function () {
    const bob = new ethers.Wallet(DEV_ACCOUNTS.bob.privateKey, ethers.provider);
    const factory = await ethers.getContractFactory("TestToken", bob);
    const token = await factory.deploy(SUPPLY);
    await token.waitForDeployment();
    const tokenAddress = await token.getAddress();

    // Register transfer as gasless, but NOT approve
    await sudoSetRule(tokenAddress, TRANSFER_SELECTOR, 0n, true);

    // Transfer should be gasless
    const balBefore1 = await getBalance(bob.address);
    const tx1 = await token.transfer(DEV_ACCOUNTS.charlie.address, ethers.parseEther("1"), { gasLimit: 100_000 });
    await tx1.wait();
    const balAfter1 = await getBalance(bob.address);
    expect(balBefore1 - balAfter1).to.equal(0n);

    // Approve should be paid (not registered)
    const balBefore2 = await getBalance(bob.address);
    const tx2 = await token.approve(DEV_ACCOUNTS.charlie.address, ethers.parseEther("100"), { gasLimit: 100_000 });
    await tx2.wait();
    const balAfter2 = await getBalance(bob.address);
    expect(balBefore2 - balAfter2).to.be.greaterThan(0n);
  });

  it("falls back to paid when value below minValue", async function () {
    const alice = new ethers.Wallet(DEV_ACCOUNTS.alice.privateKey, ethers.provider);
    const bob = new ethers.Wallet(DEV_ACCOUNTS.bob.privateKey, ethers.provider);
    const factory = await ethers.getContractFactory("TestToken", alice);
    const token = await factory.deploy(SUPPLY);
    await token.waitForDeployment();
    const tokenAddress = await token.getAddress();

    // Seed Bob with tokens
    const seedTx = await token.transfer(bob.address, ethers.parseEther("1000"));
    await seedTx.wait();

    // Register with minValue = 1 ART (requires msg.value >= 1 ether)
    await sudoSetRule(tokenAddress, TRANSFER_SELECTOR, ethers.parseEther("1"), true);

    // Transfer with value=0 — below minValue, should be paid
    const bobToken = token.connect(bob);
    const balBefore = await getBalance(bob.address);
    const tx = await bobToken.transfer(DEV_ACCOUNTS.charlie.address, ethers.parseEther("1"), { gasLimit: 100_000 });
    await tx.wait();
    const balAfter = await getBalance(bob.address);
    expect(balBefore - balAfter).to.be.greaterThan(0n);
  });
});
```

- [ ] **Step 2: Run tests**

Run: `cd packages/contracts && pnpm test --grep GaslessEdgeCases`
Expected: 4 tests pass.

- [ ] **Step 3: Commit**

```bash
git add packages/contracts/test/GaslessEdgeCases.test.ts
git commit -m "test(contracts): add gasless edge case tests"
```

---

### Task 9: Precompile direct call tests

**Files:**
- Create: `packages/contracts/test/GaslessPrecompile.test.ts`

- [ ] **Step 1: Create precompile test file**

Create `packages/contracts/test/GaslessPrecompile.test.ts`:

```typescript
import { expect } from "chai";
import { ethers } from "hardhat";
import { ApiPromise, WsProvider, Keyring } from "@polkadot/api";
import { DEV_ACCOUNTS } from "./helpers/setup";

const PRECOMPILE_ADDRESS = "0x0000000000000000000000000000000000000800";
const TRANSFER_SELECTOR = "0xa9059cbb";
const SUPPLY = ethers.parseEther("1000000");

const PRECOMPILE_ABI = [
  "function getRule(address contract_, bytes4 selector) view returns (bool enabled, uint256 minValue)",
  "function isGasless(address contract_, bytes calldata input, uint256 value, uint256 gasLimit) view returns (bool)",
  "function setRule(address contract_, bytes4 selector, uint256 minValue, bool enabled)",
  "function removeRule(address contract_, bytes4 selector)",
];

describe("GaslessPrecompile", function () {
  this.timeout(120_000);

  let api: ApiPromise;

  before(async function () {
    try {
      await ethers.provider.getBlockNumber();
    } catch {
      this.skip();
    }
    const provider = new WsProvider("ws://127.0.0.1:9944");
    api = await ApiPromise.create({ provider });
  });

  after(async function () {
    if (api) await api.disconnect();
  });

  it("getRule returns stored rule values", async function () {
    const alice = new ethers.Wallet(DEV_ACCOUNTS.alice.privateKey, ethers.provider);

    // Deploy a contract to use as target
    const factory = await ethers.getContractFactory("TestToken", alice);
    const token = await factory.deploy(SUPPLY);
    await token.waitForDeployment();
    const tokenAddress = await token.getAddress();

    // Set rule via substrate extrinsic
    const keyring = new Keyring({ type: "ethereum" });
    const aliceSr = keyring.addFromUri(DEV_ACCOUNTS.alice.privateKey);
    const call = api.tx.gaslessRegistry.setRule(tokenAddress, TRANSFER_SELECTOR, ethers.parseEther("5"), true);
    await new Promise<void>((resolve, reject) => {
      api.tx.sudo.sudo(call).signAndSend(aliceSr, ({ status, dispatchError }) => {
        if (dispatchError) reject(new Error(dispatchError.toString()));
        if (status.isInBlock || status.isFinalized) resolve();
      });
    });
    await new Promise((r) => setTimeout(r, 2000));

    // Read via precompile
    const precompile = new ethers.Contract(PRECOMPILE_ADDRESS, PRECOMPILE_ABI, alice);
    const [enabled, minValue] = await precompile.getRule(tokenAddress, TRANSFER_SELECTOR);

    expect(enabled).to.equal(true);
    expect(minValue).to.equal(ethers.parseEther("5"));
  });

  it("isGasless returns true for eligible call", async function () {
    const alice = new ethers.Wallet(DEV_ACCOUNTS.alice.privateKey, ethers.provider);
    const factory = await ethers.getContractFactory("TestToken", alice);
    const token = await factory.deploy(SUPPLY);
    await token.waitForDeployment();
    const tokenAddress = await token.getAddress();

    // Set rule via substrate
    const keyring = new Keyring({ type: "ethereum" });
    const aliceSr = keyring.addFromUri(DEV_ACCOUNTS.alice.privateKey);
    const call = api.tx.gaslessRegistry.setRule(tokenAddress, TRANSFER_SELECTOR, 0n, true);
    await new Promise<void>((resolve, reject) => {
      api.tx.sudo.sudo(call).signAndSend(aliceSr, ({ status, dispatchError }) => {
        if (dispatchError) reject(new Error(dispatchError.toString()));
        if (status.isInBlock || status.isFinalized) resolve();
      });
    });
    await new Promise((r) => setTimeout(r, 2000));

    // Build sample calldata: transfer(address,uint256)
    const iface = new ethers.Interface(["function transfer(address,uint256)"]);
    const calldata = iface.encodeFunctionData("transfer", [DEV_ACCOUNTS.bob.address, ethers.parseEther("10")]);

    const precompile = new ethers.Contract(PRECOMPILE_ADDRESS, PRECOMPILE_ABI, alice);
    const result = await precompile.isGasless(tokenAddress, calldata, 0, 100_000);
    expect(result).to.equal(true);
  });

  it("reverts when non-admin calls setRule", async function () {
    const bob = new ethers.Wallet(DEV_ACCOUNTS.bob.privateKey, ethers.provider);
    const precompile = new ethers.Contract(PRECOMPILE_ADDRESS, PRECOMPILE_ABI, bob);

    await expect(
      precompile.setRule(
        "0x0000000000000000000000000000000000000001",
        TRANSFER_SELECTOR,
        0,
        true,
        { gasLimit: 100_000 },
      ),
    ).to.be.reverted;
  });

  it("reverts when non-admin calls removeRule", async function () {
    const bob = new ethers.Wallet(DEV_ACCOUNTS.bob.privateKey, ethers.provider);
    const precompile = new ethers.Contract(PRECOMPILE_ADDRESS, PRECOMPILE_ABI, bob);

    await expect(
      precompile.removeRule(
        "0x0000000000000000000000000000000000000001",
        TRANSFER_SELECTOR,
        { gasLimit: 100_000 },
      ),
    ).to.be.reverted;
  });
});
```

- [ ] **Step 2: Run tests**

Run: `cd packages/contracts && pnpm test --grep GaslessPrecompile`
Expected: 4 tests pass.

- [ ] **Step 3: Commit**

```bash
git add packages/contracts/test/GaslessPrecompile.test.ts
git commit -m "test(contracts): add precompile direct call tests"
```

---

### Task 10: Webapp E2E tests

**Files:**
- Create: `packages/webapp/e2e/02-admin.spec.ts`

- [ ] **Step 1: Create admin E2E test**

Create `packages/webapp/e2e/02-admin.spec.ts`:

```typescript
import { test, expect } from "./fixtures";

test.describe("Admin Panel", () => {
  test("sudo account can access admin page", async ({ appPage }) => {
    await appPage.goto("http://localhost:3000/admin");
    await appPage.waitForTimeout(2000);

    // Account #0 is sudo — should see admin content
    await expect(appPage.getByText("Gasless Registry Admin")).toBeVisible({ timeout: 10_000 });
  });
});
```

Note: Full admin E2E tests (add rule, remove rule) require Ponder indexer running and are fragile with dappwright. Keep initial E2E scope to page access verification.

- [ ] **Step 2: Commit**

```bash
git add packages/webapp/e2e/02-admin.spec.ts
git commit -m "test(webapp): add admin page E2E test"
```

---

### Task 11: Dockerfile for node

**Files:**
- Create: `packages/node/Dockerfile`

- [ ] **Step 1: Create multi-stage Dockerfile**

Create `packages/node/Dockerfile`:

```dockerfile
# Stage 1: Build
FROM rust:1.94-bookworm AS builder

RUN rustup target add wasm32-unknown-unknown

WORKDIR /build
COPY . .

RUN cargo build --release

# Stage 2: Runtime
FROM debian:bookworm-slim

RUN apt-get update && \
    apt-get install -y --no-install-recommends ca-certificates libssl3 && \
    rm -rf /var/lib/apt/lists/*

COPY --from=builder /build/target/release/frontier-template-node /usr/local/bin/node

EXPOSE 9944 30333

ENTRYPOINT ["node"]
```

- [ ] **Step 2: Commit**

```bash
git add packages/node/Dockerfile
git commit -m "build(node): add multi-stage Dockerfile"
```

---

### Task 12: Docker Compose and scripts

**Files:**
- Create: `docker-compose.yml`
- Create: `scripts/docker-build.sh`
- Create: `scripts/docker-up.sh`
- Create: `scripts/docker-down.sh`

- [ ] **Step 1: Create docker-compose.yml**

Create `docker-compose.yml` at repo root:

```yaml
services:
  alice:
    build:
      context: packages/node
      dockerfile: Dockerfile
    command:
      - --chain=local
      - --alice
      - --rpc-port=9944
      - --rpc-cors=all
      - --rpc-external
      - --rpc-methods=unsafe
      - --validator
      - --node-key=0000000000000000000000000000000000000000000000000000000000000001
    ports:
      - "9944:9944"
      - "30333:30333"
    volumes:
      - alice-data:/data
    restart: unless-stopped

  bob:
    build:
      context: packages/node
      dockerfile: Dockerfile
    command:
      - --chain=local
      - --bob
      - --rpc-port=9944
      - --rpc-cors=all
      - --rpc-external
      - --rpc-methods=unsafe
      - --validator
      - --bootnodes=/dns/alice/tcp/30333/p2p/12D3KooWEyoppNCUx8Yx66oV9fJnriXwCcXwDDUA2kj6vnc6iDEp
    ports:
      - "9945:9944"
      - "30334:30333"
    volumes:
      - bob-data:/data
    depends_on:
      - alice
    restart: unless-stopped

volumes:
  alice-data:
  bob-data:
```

- [ ] **Step 2: Create docker-build.sh**

Create `scripts/docker-build.sh`:

```bash
#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$SCRIPT_DIR/.."

echo "Building Artemis node Docker image..."
docker compose -f "$REPO_ROOT/docker-compose.yml" build

echo "Done."
```

- [ ] **Step 3: Create docker-up.sh**

Create `scripts/docker-up.sh`:

```bash
#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$SCRIPT_DIR/.."

echo "Starting 2-node Artemis testnet..."
docker compose -f "$REPO_ROOT/docker-compose.yml" up -d

echo ""
echo "  Alice RPC: http://localhost:9944"
echo "  Bob RPC:   http://localhost:9945"
echo ""
echo "Logs: docker compose logs -f"
```

- [ ] **Step 4: Create docker-down.sh**

Create `scripts/docker-down.sh`:

```bash
#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$SCRIPT_DIR/.."

PURGE=false
for arg in "$@"; do
  case $arg in
    --purge) PURGE=true ;;
  esac
done

echo "Stopping Artemis testnet..."
docker compose -f "$REPO_ROOT/docker-compose.yml" down

if [ "$PURGE" = true ]; then
  echo "Purging chain data volumes..."
  docker compose -f "$REPO_ROOT/docker-compose.yml" down -v
fi

echo "Done."
```

- [ ] **Step 5: Make scripts executable**

```bash
chmod +x scripts/docker-build.sh scripts/docker-up.sh scripts/docker-down.sh
```

- [ ] **Step 6: Commit**

```bash
git add docker-compose.yml scripts/docker-build.sh scripts/docker-up.sh scripts/docker-down.sh packages/node/Dockerfile
git commit -m "build: add Docker Compose multi-node testnet"
```
