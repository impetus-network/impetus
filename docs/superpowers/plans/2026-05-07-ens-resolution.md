# ENS Domain Resolution Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Allow users to enter ENS names (e.g. `vitalik.eth`) in the transfer page recipient field and resolve them to Ethereum addresses server-side.

**Architecture:** Next.js API route calls viem's `getEnsAddress` against ETH mainnet via Alchemy RPC (server-side env). A TanStack Query hook with debouncing calls this route from the transfer page. The transfer page shows resolved address inline and uses it as the transaction target.

**Tech Stack:** viem (ENS utils + publicClient), TanStack Query v5, Next.js App Router API routes

---

## File Map

| File | Action | Responsibility |
|------|--------|----------------|
| `packages/ui/app/api/ens/resolve/route.ts` | Create | API route: validate ENS name, resolve via mainnet, return address |
| `packages/ui/hooks/useDebounce.ts` | Create | Generic debounce hook (no existing one in codebase) |
| `packages/ui/hooks/useEnsResolve.ts` | Create | ENS resolution hook: detect `.eth`, debounce, call API, return state |
| `packages/ui/app/transfer/page.tsx` | Modify | Integrate ENS hook, update validation, show resolved address |

---

### Task 1: API Route — `GET /api/ens/resolve`

**Files:**
- Create: `packages/ui/app/api/ens/resolve/route.ts`

- [ ] **Step 1: Create the API route**

```ts
// packages/ui/app/api/ens/resolve/route.ts
import { NextRequest, NextResponse } from "next/server";
import { createPublicClient, http } from "viem";
import { mainnet } from "viem/chains";
import { normalize } from "viem/ens";

const rpcUrl = process.env.ETH_MAINNET_RPC_URL;

const ethClient = createPublicClient({
  chain: mainnet,
  transport: http(rpcUrl),
});

export async function GET(request: NextRequest): Promise<NextResponse> {
  const name = request.nextUrl.searchParams.get("name");

  if (!name) {
    return NextResponse.json({ error: "Invalid ENS name" }, { status: 400 });
  }

  let normalized: string;
  try {
    normalized = normalize(name);
  } catch {
    return NextResponse.json({ error: "Invalid ENS name" }, { status: 400 });
  }

  try {
    const address = await ethClient.getEnsAddress({ name: normalized });

    if (!address) {
      return NextResponse.json(
        { error: "ENS name not found" },
        { status: 404 },
      );
    }

    return NextResponse.json({ address });
  } catch {
    return NextResponse.json(
      { error: "Failed to resolve ENS name" },
      { status: 500 },
    );
  }
}
```

- [ ] **Step 2: Verify the route responds**

Run from project root:

```bash
cd packages/ui && pnpm dev &
sleep 5
curl -s "http://localhost:3001/api/ens/resolve?name=vitalik.eth" | python3 -m json.tool
```

Expected: `{ "address": "0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045" }`

Test error cases:

```bash
curl -s "http://localhost:3001/api/ens/resolve" | python3 -m json.tool
# Expected: { "error": "Invalid ENS name" } with 400

curl -s "http://localhost:3001/api/ens/resolve?name=thisdoesnotexist12345.eth" | python3 -m json.tool
# Expected: { "error": "ENS name not found" } with 404
```

- [ ] **Step 3: Commit**

```bash
git add packages/ui/app/api/ens/resolve/route.ts
git commit -m "feat(ui): add ENS resolution API route"
```

---

### Task 2: Debounce Hook

**Files:**
- Create: `packages/ui/hooks/useDebounce.ts`

- [ ] **Step 1: Create the debounce hook**

```ts
// packages/ui/hooks/useDebounce.ts
import { useEffect, useState } from "react";

export function useDebounce<T>(value: T, delay: number): T {
  const [debouncedValue, setDebouncedValue] = useState<T>(value);

  useEffect(() => {
    const timer = window.setTimeout(() => setDebouncedValue(value), delay);
    return () => window.clearTimeout(timer);
  }, [value, delay]);

  return debouncedValue;
}
```

- [ ] **Step 2: Commit**

```bash
git add packages/ui/hooks/useDebounce.ts
git commit -m "feat(ui): add useDebounce hook"
```

---

### Task 3: ENS Resolution Hook

**Files:**
- Create: `packages/ui/hooks/useEnsResolve.ts`

- [ ] **Step 1: Create the hook**

```ts
// packages/ui/hooks/useEnsResolve.ts
import { useQuery } from "@tanstack/react-query";
import { useDebounce } from "./useDebounce";

interface EnsResolveResult {
  resolvedAddress: `0x${string}` | null;
  isResolving: boolean;
  ensError: string | null;
  isEnsInput: boolean;
}

function isEnsName(value: string): boolean {
  return value.length > 4 && value.endsWith(".eth");
}

async function fetchEnsAddress(
  name: string,
): Promise<`0x${string}`> {
  const response = await fetch(
    `/api/ens/resolve?name=${encodeURIComponent(name)}`,
  );
  const data: { address?: string; error?: string } = await response.json();

  if (!response.ok) {
    throw new Error(data.error ?? "Failed to resolve ENS name");
  }

  return data.address as `0x${string}`;
}

export function useEnsResolve(input: string): EnsResolveResult {
  const isEns = isEnsName(input.trim());
  const debouncedName = useDebounce(input.trim(), 500);
  const enabled = isEns && isEnsName(debouncedName);

  const { data, isLoading, error } = useQuery({
    queryKey: ["ens", debouncedName],
    queryFn: () => fetchEnsAddress(debouncedName),
    enabled,
    staleTime: 5 * 60 * 1000,
    retry: false,
  });

  const isStale = isEns && debouncedName !== input.trim();

  return {
    resolvedAddress: enabled ? (data ?? null) : null,
    isResolving: isStale || (enabled && isLoading),
    ensError: enabled && error instanceof Error ? error.message : null,
    isEnsInput: isEns,
  };
}
```

Key details:
- `isStale` handles the gap between typing and debounce firing — shows loading during that window.
- `retry: false` prevents hammering the API on ENS names that genuinely don't exist.
- `enabled` prevents queries for raw addresses or empty input.

- [ ] **Step 2: Commit**

```bash
git add packages/ui/hooks/useEnsResolve.ts
git commit -m "feat(ui): add useEnsResolve hook"
```

---

### Task 4: Integrate ENS into Transfer Page

**Files:**
- Modify: `packages/ui/app/transfer/page.tsx`

This task modifies the existing transfer page. The current file has these key sections:
- **Line 6:** imports including `isAddress`
- **Line 56-61:** `getButtonLabel` function with validation logic
- **Line 80-96:** component state
- **Line 101-116:** validation derivations (`numericAmount`, `validAmount`, `amountError`, `recipientError`, `valid`)
- **Line 125-156:** gas estimation `useEffect`
- **Line 158-183:** `handleSend` function
- **Line 270-298:** recipient input JSX

- [ ] **Step 1: Add import**

At line 7 (after the `useTransactor` import), add:

```ts
import { useEnsResolve } from "~/hooks/useEnsResolve";
```

- [ ] **Step 2: Wire up the hook**

After `const [gasEstimate, setGasEstimate] = useState<string | null>(null);` (line 97), add:

```ts
const { resolvedAddress, isResolving, ensError, isEnsInput } =
  useEnsResolve(to);
```

- [ ] **Step 3: Add effectiveRecipient and update validation**

Replace the current validation block (lines 101-123):

```ts
  const numericAmount = parseDecimalAmount(amount);
  const validAmount = isValidDecimalAmount(amount) && numericAmount > 0;
  const amountError =
    amount && !isValidDecimalAmount(amount)
      ? "Enter a decimal amount using digits and one optional dot."
      : amount && numericAmount <= 0
        ? "Enter an amount greater than 0."
        : validAmount && numericAmount > artBalance
          ? "Insufficient balance."
          : "";
  const recipientError = to && !isAddress(to) ? "Enter a valid address." : "";
  const valid =
    isConnected &&
    validAmount &&
    numericAmount <= artBalance &&
    isAddress(to);
  const buttonLabel = getButtonLabel({
    amount: numericAmount,
    balance: artBalance,
    connected: isConnected,
    sending,
    to,
  });
```

With:

```ts
  const numericAmount = parseDecimalAmount(amount);
  const validAmount = isValidDecimalAmount(amount) && numericAmount > 0;
  const amountError =
    amount && !isValidDecimalAmount(amount)
      ? "Enter a decimal amount using digits and one optional dot."
      : amount && numericAmount <= 0
        ? "Enter an amount greater than 0."
        : validAmount && numericAmount > artBalance
          ? "Insufficient balance."
          : "";

  const effectiveRecipient: `0x${string}` | null = isEnsInput
    ? resolvedAddress
    : isAddress(to)
      ? (to as `0x${string}`)
      : null;

  const recipientError =
    isEnsInput && !isResolving && ensError
      ? ensError
      : to && !isEnsInput && !isAddress(to)
        ? "Enter a valid address."
        : "";

  const valid =
    isConnected &&
    validAmount &&
    numericAmount <= artBalance &&
    effectiveRecipient !== null;

  const buttonLabel = getButtonLabel({
    amount: numericAmount,
    balance: artBalance,
    connected: isConnected,
    sending,
    to,
    isEnsInput,
    isResolving,
    ensError,
  });
```

- [ ] **Step 4: Update getButtonLabel**

Replace the current `getButtonLabel` function (lines 41-61):

```ts
function getButtonLabel({
  amount,
  balance,
  connected,
  sending,
  to,
}: {
  amount: number;
  balance: number;
  connected: boolean;
  sending: boolean;
  to: string;
}): string {
  if (!connected) return "Connect wallet to send";
  if (sending) return "Submitting...";
  if (!to) return "Enter recipient";
  if (!isAddress(to)) return "Invalid address";
  if (!Number.isFinite(amount) || amount <= 0) return "Enter amount";
  if (amount > balance) return "Insufficient balance";
  return `Send ${amount.toFixed(4)} ART`;
}
```

With:

```ts
function getButtonLabel({
  amount,
  balance,
  connected,
  sending,
  to,
  isEnsInput,
  isResolving,
  ensError,
}: {
  amount: number;
  balance: number;
  connected: boolean;
  sending: boolean;
  to: string;
  isEnsInput: boolean;
  isResolving: boolean;
  ensError: string | null;
}): string {
  if (!connected) return "Connect wallet to send";
  if (sending) return "Submitting...";
  if (!to) return "Enter recipient";
  if (isEnsInput && isResolving) return "Resolving ENS...";
  if (isEnsInput && ensError) return "Invalid ENS name";
  if (!isEnsInput && !isAddress(to)) return "Invalid address";
  if (!Number.isFinite(amount) || amount <= 0) return "Enter amount";
  if (amount > balance) return "Insufficient balance";
  return `Send ${amount.toFixed(4)} ART`;
}
```

- [ ] **Step 5: Update gas estimation useEffect**

Replace the current gas estimation `useEffect` (lines 125-156). The key change is using `effectiveRecipient` instead of `to as 0x${string}`:

```ts
  useEffect(() => {
    if (estimateTimerRef.current) {
      window.clearTimeout(estimateTimerRef.current);
      estimateTimerRef.current = null;
    }

    if (!valid || !publicClient || !address || !effectiveRecipient) {
      setGasEstimate(null);
      return;
    }

    estimateTimerRef.current = window.setTimeout(async () => {
      try {
        const gas = await publicClient.estimateGas({
          account: address,
          to: effectiveRecipient,
          value: parseEther(amount),
        });
        const gasPrice = await publicClient.getGasPrice();
        const fee = gas * gasPrice;
        setGasEstimate(`${formatEther(fee)} ART`);
      } catch {
        setGasEstimate(null);
      }
    }, 500);

    return () => {
      if (estimateTimerRef.current) {
        window.clearTimeout(estimateTimerRef.current);
      }
    };
  }, [valid, effectiveRecipient, amount, publicClient, address]);
```

- [ ] **Step 6: Update handleSend**

Replace `handleSend` (lines 158-183). The key change is using `effectiveRecipient`:

```ts
  async function handleSend() {
    if (!isConnected) {
      openConnectModal?.();
      return;
    }

    if (!valid || !effectiveRecipient) return;

    setSending(true);
    try {
      const hash = await transact({
        to: effectiveRecipient,
        value: parseEther(amount),
      });
      if (hash) {
        setSuccess({ amount: numericAmount, token: "ART", hash });
        setAmount("");
        setTo("");
        refetchBalance();
      }
    } catch {
      // useTransactor handles error toasts
    } finally {
      setSending(false);
    }
  }
```

- [ ] **Step 7: Update recipient input JSX**

Replace the recipient input label and error block (lines 270-298). Changes: placeholder text, ENS status display below input.

Find this block:

```tsx
            <label className="mt-5 block" htmlFor={recipientInputId}>
              <span className="art-caption text-[#6a6a6a]">To address</span>
              <input
                aria-describedby={
                  recipientError ? recipientErrorId : undefined
                }
                aria-invalid={!!recipientError}
                className="mt-2 h-14 w-full rounded-2xl border border-transparent bg-[#f5f0e0] px-5 font-mono text-sm text-[#0a0a0a] outline-none transition placeholder:text-[#0a0a0a]/30 focus:border-[#1a3a3a]"
                disabled={sending}
                id={recipientInputId}
                onChange={(event) => {
                  setTo(event.target.value);
                  setSuccess(null);
                }}
                placeholder="0x..."
                spellCheck={false}
                type="text"
                value={to}
              />
            </label>
            {recipientError && (
              <p
                className="mt-2 text-sm font-black text-[#8f1d14]"
                id={recipientErrorId}
              >
                {recipientError}
              </p>
            )}
```

Replace with:

```tsx
            <label className="mt-5 block" htmlFor={recipientInputId}>
              <span className="art-caption text-[#6a6a6a]">To address</span>
              <input
                aria-describedby={
                  recipientError ? recipientErrorId : undefined
                }
                aria-invalid={!!recipientError}
                className="mt-2 h-14 w-full rounded-2xl border border-transparent bg-[#f5f0e0] px-5 font-mono text-sm text-[#0a0a0a] outline-none transition placeholder:text-[#0a0a0a]/30 focus:border-[#1a3a3a]"
                disabled={sending}
                id={recipientInputId}
                onChange={(event) => {
                  setTo(event.target.value);
                  setSuccess(null);
                }}
                placeholder="0x... or ENS name"
                spellCheck={false}
                type="text"
                value={to}
              />
            </label>
            {isEnsInput && isResolving && (
              <p className="mt-2 text-sm text-[#6a6a6a]">
                Resolving ENS name...
              </p>
            )}
            {isEnsInput && resolvedAddress && !isResolving && (
              <p className="mt-2 font-mono text-sm text-[#6a6a6a]">
                {resolvedAddress.slice(0, 6)}...{resolvedAddress.slice(-4)}
              </p>
            )}
            {recipientError && (
              <p
                className="mt-2 text-sm font-black text-[#8f1d14]"
                id={recipientErrorId}
              >
                {recipientError}
              </p>
            )}
```

- [ ] **Step 8: Verify in browser**

Start dev server and test manually:

```bash
cd packages/ui && pnpm dev
```

Open `http://localhost:3001/transfer` and verify:

1. Type `vitalik.eth` in recipient — should show resolving spinner, then truncated address below input
2. Type `thisdoesnotexist12345.eth` — should show "ENS name not found" error
3. Type `0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266` — should work exactly as before
4. Type partial input like `abc` — no ENS resolution triggered, normal invalid address error
5. Button label changes: "Resolving ENS..." while loading, "Invalid ENS name" on error

- [ ] **Step 9: Commit**

```bash
git add packages/ui/app/transfer/page.tsx
git commit -m "feat(ui): integrate ENS resolution into transfer page"
```
