# Gasless Manager Admin Page Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Admin page at `/admin/gasless` for managing GaslessRegistry precompile rules — list, add, toggle, remove rules, and test gasless status.

**Architecture:** Page fetches rules from Ponder GraphQL indexer via TanStack Query. Write operations use `useScaffoldWriteContract("GaslessRegistry")`. Sudo access guard restricts write UI to admin wallet.

**Tech Stack:** Next.js app router, coss UI (Table, Card, Field, Input, Switch, Button, Badge), TanStack Query, Ponder GraphQL, wagmi/viem

---

### Task 1: Ponder config and useGaslessRules hook

**Files:**
- Create: `packages/ui/config/ponder.ts`
- Create: `packages/ui/hooks/useGaslessRules.ts`

- [ ] **Step 1: Create Ponder URL config**

Create `packages/ui/config/ponder.ts`:

```typescript
export const PONDER_URL = process.env.NEXT_PUBLIC_PONDER_URL || "http://localhost:42069";
```

- [ ] **Step 2: Create useGaslessRules hook**

Create `packages/ui/hooks/useGaslessRules.ts`:

```typescript
"use client";

import { useQuery } from "@tanstack/react-query";
import { PONDER_URL } from "~/config/ponder";

export interface GaslessRuleRow {
  id: string;
  contract: string;
  selector: string;
  enabled: boolean;
  minValue: string;
  updatedAtBlock: string;
}

const RULES_QUERY = `{
  gaslessRuless(orderBy: "updatedAtBlock", orderDirection: "desc", limit: 100) {
    items {
      id
      contract
      selector
      enabled
      minValue
      updatedAtBlock
    }
  }
}`;

async function fetchRules(): Promise<GaslessRuleRow[]> {
  const res = await fetch(`${PONDER_URL}/graphql`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ query: RULES_QUERY }),
  });
  const json = await res.json();
  return json.data?.gaslessRuless?.items ?? [];
}

export function useGaslessRules() {
  return useQuery({
    queryKey: ["gaslessRules"],
    queryFn: fetchRules,
    refetchInterval: 5000,
  });
}
```

- [ ] **Step 3: Add NEXT_PUBLIC_PONDER_URL to .env.local**

Append to `packages/ui/.env.local`:

```
NEXT_PUBLIC_PONDER_URL=http://localhost:42069
```

- [ ] **Step 4: Build and verify**

```bash
cd /Users/huyduan/projects/blockchain/packages/ui && pnpm build
```

- [ ] **Step 5: Commit**

```bash
git add packages/ui/config/ponder.ts packages/ui/hooks/useGaslessRules.ts packages/ui/.env.local
git commit -m "feat(ui): add Ponder config and useGaslessRules hook

Co-Authored-By: Claude Opus 4.6 (1M context) <noreply@anthropic.com>"
```

---

### Task 2: RulesTable component

**Files:**
- Create: `packages/ui/components/admin/RulesTable.tsx`

- [ ] **Step 1: Create RulesTable**

Create `packages/ui/components/admin/RulesTable.tsx`:

```typescript
"use client";

import { type GaslessRuleRow } from "~/hooks/useGaslessRules";
import { useScaffoldWriteContract } from "~/hooks/useScaffoldWriteContract";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "~/components/ui/table";
import { Badge } from "~/components/ui/badge";
import { Button } from "~/components/ui/button";
import { formatEther } from "viem";

interface RulesTableProps {
  rules: GaslessRuleRow[];
  isAdmin: boolean;
}

export function RulesTable({ rules, isAdmin }: RulesTableProps) {
  const { writeAsync, isMining } = useScaffoldWriteContract("GaslessRegistry");

  async function handleToggle(rule: GaslessRuleRow) {
    await writeAsync("setRule", [
      rule.contract as `0x${string}`,
      rule.selector as `0x${string}`,
      BigInt(rule.minValue),
      !rule.enabled,
    ]);
  }

  async function handleRemove(rule: GaslessRuleRow) {
    await writeAsync("removeRule", [
      rule.contract as `0x${string}`,
      rule.selector as `0x${string}`,
    ]);
  }

  if (rules.length === 0) {
    return <p className="text-sm text-muted-foreground">No gasless rules configured.</p>;
  }

  return (
    <Table>
      <TableHeader>
        <TableRow>
          <TableHead>Contract</TableHead>
          <TableHead>Selector</TableHead>
          <TableHead>Status</TableHead>
          <TableHead>Min Value</TableHead>
          {isAdmin && <TableHead>Actions</TableHead>}
        </TableRow>
      </TableHeader>
      <TableBody>
        {rules.map((rule) => (
          <TableRow key={rule.id}>
            <TableCell className="font-mono text-xs">
              {rule.contract.slice(0, 10)}...{rule.contract.slice(-6)}
            </TableCell>
            <TableCell className="font-mono text-xs">{rule.selector}</TableCell>
            <TableCell>
              <Badge variant={rule.enabled ? "success" : "secondary"}>
                {rule.enabled ? "Enabled" : "Disabled"}
              </Badge>
            </TableCell>
            <TableCell className="font-mono text-xs">
              {BigInt(rule.minValue) === 0n ? "0" : formatEther(BigInt(rule.minValue))}
            </TableCell>
            {isAdmin && (
              <TableCell>
                <div className="flex gap-2">
                  <Button
                    variant="outline"
                    size="sm"
                    disabled={isMining}
                    onClick={() => handleToggle(rule)}
                  >
                    {rule.enabled ? "Disable" : "Enable"}
                  </Button>
                  <Button
                    variant="destructive"
                    size="sm"
                    disabled={isMining}
                    onClick={() => handleRemove(rule)}
                  >
                    Remove
                  </Button>
                </div>
              </TableCell>
            )}
          </TableRow>
        ))}
      </TableBody>
    </Table>
  );
}
```

- [ ] **Step 2: Build and verify**

```bash
cd /Users/huyduan/projects/blockchain/packages/ui && pnpm build
```

- [ ] **Step 3: Commit**

```bash
git add packages/ui/components/admin/RulesTable.tsx
git commit -m "feat(ui): add RulesTable component with toggle/remove actions

Co-Authored-By: Claude Opus 4.6 (1M context) <noreply@anthropic.com>"
```

---

### Task 3: AddRuleForm component

**Files:**
- Create: `packages/ui/components/admin/AddRuleForm.tsx`

- [ ] **Step 1: Create AddRuleForm**

Create `packages/ui/components/admin/AddRuleForm.tsx`:

```typescript
"use client";

import { useState } from "react";
import { isAddress, isHex } from "viem";
import { useScaffoldWriteContract } from "~/hooks/useScaffoldWriteContract";
import { Card, CardContent, CardHeader, CardTitle } from "~/components/ui/card";
import { Field, FieldLabel, FieldError } from "~/components/ui/field";
import { Input } from "~/components/ui/input";
import { Switch } from "~/components/ui/switch";
import { Button } from "~/components/ui/button";

export function AddRuleForm() {
  const { writeAsync, isMining } = useScaffoldWriteContract("GaslessRegistry");
  const [contract, setContract] = useState("");
  const [selector, setSelector] = useState("");
  const [minValue, setMinValue] = useState("0");
  const [enabled, setEnabled] = useState(true);
  const [contractError, setContractError] = useState("");
  const [selectorError, setSelectorError] = useState("");

  function validate(): boolean {
    let valid = true;
    setContractError("");
    setSelectorError("");

    if (!contract || !isAddress(contract)) {
      setContractError("Enter a valid address");
      valid = false;
    }
    if (!selector || !isHex(selector) || selector.length !== 10) {
      setSelectorError("Enter a valid bytes4 selector (e.g. 0xa9059cbb)");
      valid = false;
    }
    return valid;
  }

  async function handleSubmit(e: React.FormEvent) {
    e.preventDefault();
    if (!validate()) return;

    try {
      await writeAsync("setRule", [
        contract as `0x${string}`,
        selector as `0x${string}`,
        BigInt(minValue || "0"),
        enabled,
      ]);
      setContract("");
      setSelector("");
      setMinValue("0");
      setEnabled(true);
    } catch {
      // Error shown via toast
    }
  }

  return (
    <Card>
      <CardHeader>
        <CardTitle>Add Rule</CardTitle>
      </CardHeader>
      <CardContent>
        <form onSubmit={handleSubmit} className="flex flex-col gap-4">
          <Field invalid={!!contractError}>
            <FieldLabel>Contract Address</FieldLabel>
            <Input
              type="text"
              placeholder="0x..."
              value={contract}
              onChange={(e) => { setContract(e.target.value); setContractError(""); }}
              className="font-mono"
            />
            {contractError && <FieldError>{contractError}</FieldError>}
          </Field>

          <Field invalid={!!selectorError}>
            <FieldLabel>Function Selector (bytes4)</FieldLabel>
            <Input
              type="text"
              placeholder="0xa9059cbb"
              value={selector}
              onChange={(e) => { setSelector(e.target.value); setSelectorError(""); }}
              className="font-mono"
            />
            {selectorError && <FieldError>{selectorError}</FieldError>}
          </Field>

          <Field>
            <FieldLabel>Min Value (wei)</FieldLabel>
            <Input
              type="text"
              inputMode="numeric"
              placeholder="0"
              value={minValue}
              onChange={(e) => setMinValue(e.target.value)}
              className="font-mono"
            />
          </Field>

          <div className="flex items-center gap-3">
            <Switch checked={enabled} onCheckedChange={setEnabled} />
            <span className="text-sm">{enabled ? "Enabled" : "Disabled"}</span>
          </div>

          <Button type="submit" disabled={isMining}>
            {isMining ? "Submitting..." : "Add Rule"}
          </Button>
        </form>
      </CardContent>
    </Card>
  );
}
```

- [ ] **Step 2: Build and verify**

```bash
cd /Users/huyduan/projects/blockchain/packages/ui && pnpm build
```

- [ ] **Step 3: Commit**

```bash
git add packages/ui/components/admin/AddRuleForm.tsx
git commit -m "feat(ui): add AddRuleForm component for gasless rules

Co-Authored-By: Claude Opus 4.6 (1M context) <noreply@anthropic.com>"
```

---

### Task 4: CheckGaslessForm component

**Files:**
- Create: `packages/ui/components/admin/CheckGaslessForm.tsx`

- [ ] **Step 1: Create CheckGaslessForm**

Create `packages/ui/components/admin/CheckGaslessForm.tsx`:

```typescript
"use client";

import { useState } from "react";
import { isAddress, isHex } from "viem";
import { useScaffoldReadContract } from "~/hooks/useScaffoldReadContract";
import { Card, CardContent, CardHeader, CardTitle } from "~/components/ui/card";
import { Field, FieldLabel, FieldError } from "~/components/ui/field";
import { Input } from "~/components/ui/input";
import { Button } from "~/components/ui/button";
import { Badge } from "~/components/ui/badge";

export function CheckGaslessForm() {
  const [contract, setContract] = useState("");
  const [calldata, setCalldata] = useState("");
  const [value, setValue] = useState("0");
  const [gasLimit, setGasLimit] = useState("21000");
  const [check, setCheck] = useState(false);
  const [contractError, setContractError] = useState("");
  const [calldataError, setCalldataError] = useState("");

  const { data: isGasless, isLoading } = useScaffoldReadContract({
    contractName: "GaslessRegistry",
    functionName: "isGasless",
    args: [
      contract as `0x${string}`,
      calldata as `0x${string}`,
      BigInt(value || "0"),
      BigInt(gasLimit || "21000"),
    ],
    enabled: check && !!contract && !!calldata,
  });

  function validate(): boolean {
    let valid = true;
    setContractError("");
    setCalldataError("");

    if (!contract || !isAddress(contract)) {
      setContractError("Enter a valid address");
      valid = false;
    }
    if (!calldata || !isHex(calldata)) {
      setCalldataError("Enter valid hex calldata");
      valid = false;
    }
    return valid;
  }

  function handleCheck(e: React.FormEvent) {
    e.preventDefault();
    if (!validate()) return;
    setCheck(true);
  }

  return (
    <Card>
      <CardHeader>
        <CardTitle>Check Gasless</CardTitle>
      </CardHeader>
      <CardContent>
        <form onSubmit={handleCheck} className="flex flex-col gap-4">
          <Field invalid={!!contractError}>
            <FieldLabel>Contract Address</FieldLabel>
            <Input
              type="text"
              placeholder="0x..."
              value={contract}
              onChange={(e) => { setContract(e.target.value); setContractError(""); setCheck(false); }}
              className="font-mono"
            />
            {contractError && <FieldError>{contractError}</FieldError>}
          </Field>

          <Field invalid={!!calldataError}>
            <FieldLabel>Calldata (hex)</FieldLabel>
            <Input
              type="text"
              placeholder="0xa9059cbb000..."
              value={calldata}
              onChange={(e) => { setCalldata(e.target.value); setCalldataError(""); setCheck(false); }}
              className="font-mono"
            />
            {calldataError && <FieldError>{calldataError}</FieldError>}
          </Field>

          <div className="grid grid-cols-2 gap-4">
            <Field>
              <FieldLabel>Value (wei)</FieldLabel>
              <Input
                type="text"
                inputMode="numeric"
                placeholder="0"
                value={value}
                onChange={(e) => { setValue(e.target.value); setCheck(false); }}
                className="font-mono"
              />
            </Field>
            <Field>
              <FieldLabel>Gas Limit</FieldLabel>
              <Input
                type="text"
                inputMode="numeric"
                placeholder="21000"
                value={gasLimit}
                onChange={(e) => { setGasLimit(e.target.value); setCheck(false); }}
                className="font-mono"
              />
            </Field>
          </div>

          <div className="flex items-center gap-4">
            <Button type="submit" disabled={isLoading}>
              {isLoading ? "Checking..." : "Check"}
            </Button>
            {check && isGasless !== undefined && (
              <Badge variant={isGasless ? "success" : "destructive"}>
                {isGasless ? "Gasless" : "Not Gasless"}
              </Badge>
            )}
          </div>
        </form>
      </CardContent>
    </Card>
  );
}
```

- [ ] **Step 2: Build and verify**

```bash
cd /Users/huyduan/projects/blockchain/packages/ui && pnpm build
```

- [ ] **Step 3: Commit**

```bash
git add packages/ui/components/admin/CheckGaslessForm.tsx
git commit -m "feat(ui): add CheckGaslessForm component

Co-Authored-By: Claude Opus 4.6 (1M context) <noreply@anthropic.com>"
```

---

### Task 5: Admin page and nav link

**Files:**
- Create: `packages/ui/app/admin/gasless/page.tsx`
- Modify: `packages/ui/components/layout/Header.tsx`

- [ ] **Step 1: Create admin gasless page**

Create `packages/ui/app/admin/gasless/page.tsx`:

```typescript
"use client";

import { useAccount } from "wagmi";
import { SUDO_ADDRESS } from "@artemis/shared";
import { useGaslessRules } from "~/hooks/useGaslessRules";
import { RulesTable } from "~/components/admin/RulesTable";
import { AddRuleForm } from "~/components/admin/AddRuleForm";
import { CheckGaslessForm } from "~/components/admin/CheckGaslessForm";
import { Badge } from "~/components/ui/badge";

export default function GaslessManagerPage() {
  const { address, isConnected } = useAccount();
  const { data: rules, isLoading } = useGaslessRules();
  const isAdmin = isConnected && address?.toLowerCase() === SUDO_ADDRESS.toLowerCase();

  return (
    <div className="flex flex-col gap-6">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold">Gasless Manager</h1>
          <p className="text-muted-foreground">
            Manage gasless transaction rules on Artemis chain.
          </p>
        </div>
        {isConnected && (
          <Badge variant={isAdmin ? "success" : "secondary"}>
            {isAdmin ? "Admin" : "Read-only"}
          </Badge>
        )}
      </div>

      <section>
        <h2 className="mb-3 text-lg font-medium">Rules</h2>
        {isLoading ? (
          <p className="text-sm text-muted-foreground">Loading rules...</p>
        ) : (
          <RulesTable rules={rules ?? []} isAdmin={isAdmin} />
        )}
      </section>

      {isAdmin && (
        <section>
          <AddRuleForm />
        </section>
      )}

      <section>
        <CheckGaslessForm />
      </section>
    </div>
  );
}
```

- [ ] **Step 2: Add admin nav link to Header**

Read `packages/ui/components/layout/Header.tsx` and add the admin link. The admin link should only show when connected as sudo. Since Header is a client component, add conditional rendering:

Replace the `navItems` array and add account check:

```typescript
"use client";

import Link from "next/link";
import { usePathname } from "next/navigation";
import { useAccount } from "wagmi";
import { SUDO_ADDRESS } from "@artemis/shared";
import { ConnectButtonCustom } from "~/components/scaffold/ConnectButtonCustom";

const navItems = [
  { href: "/", label: "Home" },
  { href: "/transfer", label: "Transfer" },
  { href: "/blockexplorer", label: "Explorer" },
  { href: "/debug", label: "Debug" },
];

export function Header() {
  const pathname = usePathname();
  const { address, isConnected } = useAccount();
  const isAdmin = isConnected && address?.toLowerCase() === SUDO_ADDRESS.toLowerCase();

  const allNavItems = isAdmin
    ? [...navItems, { href: "/admin/gasless", label: "Admin" }]
    : navItems;

  return (
    <header className="border-b border-border bg-background">
      <nav className="mx-auto flex max-w-7xl items-center justify-between px-4 py-3">
        <div className="flex items-center gap-6">
          <Link href="/" className="text-xl font-bold text-primary">
            Artemis
          </Link>
          <div className="flex gap-4">
            {allNavItems.map((item) => (
              <Link
                key={item.href}
                href={item.href}
                className={`text-sm transition-colors ${
                  pathname === item.href
                    ? "font-medium text-foreground"
                    : "text-muted-foreground hover:text-foreground"
                }`}
              >
                {item.label}
              </Link>
            ))}
          </div>
        </div>
        <ConnectButtonCustom />
      </nav>
    </header>
  );
}
```

- [ ] **Step 3: Build and verify**

```bash
cd /Users/huyduan/projects/blockchain/packages/ui && rm -rf .next && pnpm build
```

All routes should appear including `/admin/gasless`.

- [ ] **Step 4: Commit**

```bash
git add packages/ui/app/admin/gasless/page.tsx packages/ui/components/layout/Header.tsx
git commit -m "feat(ui): add gasless manager admin page with sudo nav guard

Co-Authored-By: Claude Opus 4.6 (1M context) <noreply@anthropic.com>"
```
