# Betting Frontend Implementation Plan (Plan B)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the rounds betting UI, admin panel, and wagmi contract hooks for the Artemis betting webapp, with indexer updates and E2E tests.

**Architecture:** Custom wagmi hooks wrap precompile reads/writes. Pages fetch historical data from Ponder indexer GraphQL API and on-chain data via wagmi. Indexer handles `BetUpdated` event for bet edits/removals. E2E tests use dappwright for full betting flows.

**Tech Stack:** React 19, TypeScript, wagmi v2, viem, TanStack Query, React Router v7, coss ui, Ponder (indexer), Playwright + dappwright

---

## File Map

| File | Responsibility |
|------|---------------|
| `packages/webapp/src/config/contract.ts` | Precompile address + ABI constant for wagmi |
| `packages/webapp/src/hooks/useCurrentRound.ts` | Read `getCurrentRound` with auto-refetch |
| `packages/webapp/src/hooks/useUserBets.ts` | Read `getBets` for connected user |
| `packages/webapp/src/hooks/usePlaceBets.ts` | Write `placeBets` / `placeBet` |
| `packages/webapp/src/hooks/useUpdateBet.ts` | Write `updateBet` |
| `packages/webapp/src/hooks/useClaimWinnings.ts` | Write `claimWinnings` |
| `packages/webapp/src/hooks/useAdminActions.ts` | Write `submitResult`, `adminClaimPool`, `forceCloseRound` |
| `packages/webapp/src/lib/graphql.ts` | GraphQL client for Ponder indexer |
| `packages/webapp/src/hooks/useRounds.ts` | Fetch rounds list from indexer |
| `packages/webapp/src/hooks/useRoundBets.ts` | Fetch bets for a round from indexer |
| `packages/webapp/src/components/betting/NumberGrid.tsx` | 10x10 number picker (0-99) |
| `packages/webapp/src/components/betting/BetList.tsx` | Display user's bets with edit/remove |
| `packages/webapp/src/components/betting/PlaceBetForm.tsx` | Selected numbers + amounts form |
| `packages/webapp/src/components/round/RoundCard.tsx` | Round status card with countdown |
| `packages/webapp/src/components/round/RoundHistory.tsx` | Past rounds table |
| `packages/webapp/src/pages/Rounds.tsx` | Rounds list page (replace placeholder) |
| `packages/webapp/src/pages/RoundDetail.tsx` | Round detail + bet form (replace placeholder) |
| `packages/webapp/src/pages/Admin.tsx` | Admin panel (replace placeholder) |
| `packages/indexer/src/index.ts` | Add BetUpdated handler, update bet ID format |
| `packages/indexer/ponder.schema.ts` | No changes needed (schema already supports multi-bet) |
| `packages/webapp/e2e/place-bet.spec.ts` | E2E: place single bet flow |
| `packages/webapp/e2e/place-bets.spec.ts` | E2E: place multiple bets flow |
| `packages/webapp/e2e/update-bet.spec.ts` | E2E: update/remove bet flow |
| `packages/webapp/e2e/admin.spec.ts` | E2E: admin submit result flow |
| `packages/webapp/e2e/claim-winnings.spec.ts` | E2E: claim winnings flow |

---

### Task 1: Contract config and base hooks

**Files:**
- Create: `packages/webapp/src/config/contract.ts`
- Create: `packages/webapp/src/hooks/useCurrentRound.ts`
- Create: `packages/webapp/src/hooks/useUserBets.ts`

- [ ] **Step 1: Create contract config**

Write to `packages/webapp/src/config/contract.ts`:

```ts
import { BETTING_PRECOMPILE_ADDRESS, IBettingPrecompileAbi } from "@betting/shared";

export const bettingContract = {
  address: BETTING_PRECOMPILE_ADDRESS as `0x${string}`,
  abi: IBettingPrecompileAbi,
} as const;
```

- [ ] **Step 2: Create useCurrentRound hook**

Write to `packages/webapp/src/hooks/useCurrentRound.ts`:

```ts
import { useReadContract } from "wagmi";
import { bettingContract } from "@/config/contract";

export function useCurrentRound() {
  return useReadContract({
    ...bettingContract,
    functionName: "getCurrentRound",
    query: {
      refetchInterval: 6_000,
    },
  });
}
```

- [ ] **Step 3: Create useUserBets hook**

Write to `packages/webapp/src/hooks/useUserBets.ts`:

```ts
import { useReadContract, useAccount } from "wagmi";
import { bettingContract } from "@/config/contract";

export function useUserBets(roundId: bigint | undefined) {
  const { address } = useAccount();

  return useReadContract({
    ...bettingContract,
    functionName: "getBets",
    args: roundId !== undefined && address ? [roundId, address] : undefined,
    query: {
      enabled: roundId !== undefined && !!address,
      refetchInterval: 6_000,
    },
  });
}
```

- [ ] **Step 4: Verify TypeScript compiles**

Run:
```bash
cd packages/webapp && npx tsc --noEmit
```

- [ ] **Step 5: Commit**

```bash
git add packages/webapp/src/config/contract.ts packages/webapp/src/hooks/useCurrentRound.ts packages/webapp/src/hooks/useUserBets.ts
git commit -m "feat(webapp): add contract config and read hooks"
```

---

### Task 2: Write hooks (place, update, claim, admin)

**Files:**
- Create: `packages/webapp/src/hooks/usePlaceBets.ts`
- Create: `packages/webapp/src/hooks/useUpdateBet.ts`
- Create: `packages/webapp/src/hooks/useClaimWinnings.ts`
- Create: `packages/webapp/src/hooks/useAdminActions.ts`

- [ ] **Step 1: Create usePlaceBets hook**

Write to `packages/webapp/src/hooks/usePlaceBets.ts`:

```ts
import { useWriteContract, useWaitForTransactionReceipt } from "wagmi";
import { bettingContract } from "@/config/contract";

export function usePlaceBets() {
  const { data: hash, writeContract, isPending, error } = useWriteContract();
  const { isLoading: isConfirming, isSuccess } = useWaitForTransactionReceipt({ hash });

  function placeBet(number: number, token: `0x${string}`, amount: bigint) {
    writeContract({
      ...bettingContract,
      functionName: "placeBet",
      args: [number, token, amount],
    });
  }

  function placeBets(
    numbers: readonly number[],
    amounts: readonly bigint[],
    tokens: readonly `0x${string}`[],
  ) {
    writeContract({
      ...bettingContract,
      functionName: "placeBets",
      args: [numbers, amounts, tokens],
    });
  }

  return { placeBet, placeBets, hash, isPending, isConfirming, isSuccess, error };
}
```

- [ ] **Step 2: Create useUpdateBet hook**

Write to `packages/webapp/src/hooks/useUpdateBet.ts`:

```ts
import { useWriteContract, useWaitForTransactionReceipt } from "wagmi";
import { bettingContract } from "@/config/contract";

export function useUpdateBet() {
  const { data: hash, writeContract, isPending, error } = useWriteContract();
  const { isLoading: isConfirming, isSuccess } = useWaitForTransactionReceipt({ hash });

  function updateBet(
    roundId: bigint,
    number: number,
    newAmount: bigint,
    token: `0x${string}`,
  ) {
    writeContract({
      ...bettingContract,
      functionName: "updateBet",
      args: [roundId, number, newAmount, token],
    });
  }

  return { updateBet, hash, isPending, isConfirming, isSuccess, error };
}
```

- [ ] **Step 3: Create useClaimWinnings hook**

Write to `packages/webapp/src/hooks/useClaimWinnings.ts`:

```ts
import { useWriteContract, useWaitForTransactionReceipt } from "wagmi";
import { bettingContract } from "@/config/contract";

export function useClaimWinnings() {
  const { data: hash, writeContract, isPending, error } = useWriteContract();
  const { isLoading: isConfirming, isSuccess } = useWaitForTransactionReceipt({ hash });

  function claimWinnings(roundId: bigint, number: number) {
    writeContract({
      ...bettingContract,
      functionName: "claimWinnings",
      args: [roundId, number],
    });
  }

  return { claimWinnings, hash, isPending, isConfirming, isSuccess, error };
}
```

- [ ] **Step 4: Create useAdminActions hook**

Write to `packages/webapp/src/hooks/useAdminActions.ts`:

```ts
import { useWriteContract, useWaitForTransactionReceipt } from "wagmi";
import { bettingContract } from "@/config/contract";

export function useAdminActions() {
  const { data: hash, writeContract, isPending, error } = useWriteContract();
  const { isLoading: isConfirming, isSuccess } = useWaitForTransactionReceipt({ hash });

  function submitResult(roundId: bigint, number: number) {
    writeContract({
      ...bettingContract,
      functionName: "submitResult",
      args: [roundId, number],
    });
  }

  function adminClaimPool(roundId: bigint) {
    writeContract({
      ...bettingContract,
      functionName: "adminClaimPool",
      args: [roundId],
    });
  }

  function forceCloseRound(roundId: bigint) {
    writeContract({
      ...bettingContract,
      functionName: "forceCloseRound",
      args: [roundId],
    });
  }

  return { submitResult, adminClaimPool, forceCloseRound, hash, isPending, isConfirming, isSuccess, error };
}
```

- [ ] **Step 5: Verify TypeScript compiles**

Run:
```bash
cd packages/webapp && npx tsc --noEmit
```

- [ ] **Step 6: Commit**

```bash
git add packages/webapp/src/hooks/
git commit -m "feat(webapp): add write hooks for betting and admin"
```

---

### Task 3: GraphQL client and indexer query hooks

**Files:**
- Create: `packages/webapp/src/lib/graphql.ts`
- Create: `packages/webapp/src/hooks/useRounds.ts`
- Create: `packages/webapp/src/hooks/useRoundBets.ts`

- [ ] **Step 1: Create GraphQL client**

Write to `packages/webapp/src/lib/graphql.ts`:

```ts
const INDEXER_URL = import.meta.env.VITE_INDEXER_URL ?? "http://localhost:42069/graphql";

export async function gqlQuery<T>(query: string, variables?: Record<string, unknown>): Promise<T> {
  const response = await fetch(INDEXER_URL, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ query, variables }),
  });

  if (!response.ok) {
    throw new Error(`GraphQL request failed: ${response.status}`);
  }

  const json = await response.json();
  if (json.errors) {
    throw new Error(json.errors[0].message);
  }

  return json.data as T;
}
```

- [ ] **Step 2: Create useRounds hook**

Write to `packages/webapp/src/hooks/useRounds.ts`:

```ts
import { useQuery } from "@tanstack/react-query";
import { gqlQuery } from "@/lib/graphql";

interface RoundRow {
  id: string;
  status: number;
  winningNumber: number | null;
  totalBets: number;
  totalVolume: string;
  totalPayout: string;
  resolvedAt: string | null;
}

interface RoundsResponse {
  rounds: { items: RoundRow[] };
}

const ROUNDS_QUERY = `
  query Rounds($limit: Int, $orderBy: String, $orderDirection: String) {
    rounds(limit: $limit, orderBy: $orderBy, orderDirection: $orderDirection) {
      items {
        id
        status
        winningNumber
        totalBets
        totalVolume
        totalPayout
        resolvedAt
      }
    }
  }
`;

export function useRounds(limit = 20) {
  return useQuery({
    queryKey: ["rounds", limit],
    queryFn: () =>
      gqlQuery<RoundsResponse>(ROUNDS_QUERY, {
        limit,
        orderBy: "id",
        orderDirection: "desc",
      }),
    select: (data) => data.rounds.items,
    refetchInterval: 10_000,
  });
}
```

- [ ] **Step 3: Create useRoundBets hook**

Write to `packages/webapp/src/hooks/useRoundBets.ts`:

```ts
import { useQuery } from "@tanstack/react-query";
import { gqlQuery } from "@/lib/graphql";

interface BetRow {
  id: string;
  roundId: string;
  user: string;
  number: number;
  token: string;
  amount: string;
  claimed: boolean;
  payout: string;
  timestamp: string;
}

interface BetsResponse {
  bets: { items: BetRow[] };
}

const BETS_QUERY = `
  query BetsByRound($roundId: BigInt!) {
    bets(where: { roundId: $roundId }, orderBy: "number", orderDirection: "asc") {
      items {
        id
        roundId
        user
        number
        token
        amount
        claimed
        payout
        timestamp
      }
    }
  }
`;

export function useRoundBets(roundId: string | undefined) {
  return useQuery({
    queryKey: ["roundBets", roundId],
    queryFn: () =>
      gqlQuery<BetsResponse>(BETS_QUERY, { roundId }),
    select: (data) => data.bets.items,
    enabled: !!roundId,
    refetchInterval: 10_000,
  });
}
```

- [ ] **Step 4: Add env var to .env.example**

Append to `packages/webapp/.env.example`:

```
VITE_INDEXER_URL=http://localhost:42069/graphql
```

- [ ] **Step 5: Verify TypeScript compiles**

Run:
```bash
cd packages/webapp && npx tsc --noEmit
```

- [ ] **Step 6: Commit**

```bash
git add packages/webapp/src/lib/graphql.ts packages/webapp/src/hooks/useRounds.ts packages/webapp/src/hooks/useRoundBets.ts packages/webapp/.env.example
git commit -m "feat(webapp): add GraphQL client and indexer query hooks"
```

---

### Task 4: Betting UI components

**Files:**
- Create: `packages/webapp/src/components/betting/NumberGrid.tsx`
- Create: `packages/webapp/src/components/betting/BetList.tsx`
- Create: `packages/webapp/src/components/betting/PlaceBetForm.tsx`
- Create: `packages/webapp/src/components/round/RoundCard.tsx`
- Create: `packages/webapp/src/components/round/RoundHistory.tsx`

- [ ] **Step 1: Create NumberGrid component**

Write to `packages/webapp/src/components/betting/NumberGrid.tsx`:

```tsx
interface NumberGridProps {
  selected: Set<number>;
  onToggle: (n: number) => void;
  disabled?: boolean;
}

export function NumberGrid({ selected, onToggle, disabled }: NumberGridProps) {
  return (
    <div className="grid grid-cols-10 gap-1" data-testid="number-grid">
      {Array.from({ length: 100 }, (_, i) => (
        <button
          key={i}
          type="button"
          disabled={disabled}
          onClick={() => onToggle(i)}
          className={`h-10 w-full rounded text-sm font-mono transition-colors ${
            selected.has(i)
              ? "bg-primary text-primary-foreground"
              : "bg-muted hover:bg-muted-foreground/20"
          } disabled:opacity-50 disabled:cursor-not-allowed`}
          data-testid={`number-${i}`}
        >
          {i}
        </button>
      ))}
    </div>
  );
}
```

- [ ] **Step 2: Create BetList component**

Write to `packages/webapp/src/components/betting/BetList.tsx`:

```tsx
import { formatEther } from "viem";

interface Bet {
  number: number;
  amount: bigint;
  token: `0x${string}`;
  claimed: boolean;
}

interface BetListProps {
  bets: Bet[];
  onUpdate?: (number: number, newAmount: bigint, token: `0x${string}`) => void;
  onRemove?: (number: number) => void;
  editable?: boolean;
}

export function BetList({ bets, onUpdate, onRemove, editable }: BetListProps) {
  if (bets.length === 0) {
    return <p className="text-sm text-muted-foreground">No bets placed</p>;
  }

  return (
    <div className="space-y-2" data-testid="bet-list">
      {bets.map((bet) => (
        <div
          key={bet.number}
          className="flex items-center justify-between rounded border border-border p-3"
          data-testid={`bet-item-${bet.number}`}
        >
          <div className="flex items-center gap-4">
            <span className="font-mono text-lg font-bold" data-testid={`bet-number-${bet.number}`}>
              #{bet.number}
            </span>
            <span className="text-sm" data-testid={`bet-amount-${bet.number}`}>
              {formatEther(bet.amount)} ART
            </span>
            {bet.claimed && (
              <span className="text-xs text-muted-foreground">Claimed</span>
            )}
          </div>
          {editable && !bet.claimed && (
            <div className="flex gap-2">
              {onRemove && (
                <button
                  type="button"
                  onClick={() => onRemove(bet.number)}
                  className="text-xs text-muted-foreground hover:text-foreground"
                  data-testid={`remove-bet-${bet.number}`}
                >
                  Remove
                </button>
              )}
            </div>
          )}
        </div>
      ))}
    </div>
  );
}
```

- [ ] **Step 3: Create PlaceBetForm component**

Write to `packages/webapp/src/components/betting/PlaceBetForm.tsx`:

```tsx
import { useState } from "react";
import { parseEther } from "viem";
import { NumberGrid } from "./NumberGrid";
import { Button } from "@/components/ui/button";

const NATIVE_TOKEN = "0x0000000000000000000000000000000000000000" as `0x${string}`;

interface PlaceBetFormProps {
  onSubmit: (numbers: number[], amounts: bigint[], tokens: `0x${string}`[]) => void;
  isPending: boolean;
  disabled?: boolean;
}

export function PlaceBetForm({ onSubmit, isPending, disabled }: PlaceBetFormProps) {
  const [selected, setSelected] = useState<Set<number>>(new Set());
  const [amount, setAmount] = useState("");

  function handleToggle(n: number) {
    setSelected((prev) => {
      const next = new Set(prev);
      if (next.has(n)) {
        next.delete(n);
      } else {
        next.add(n);
      }
      return next;
    });
  }

  function handleSubmit() {
    if (selected.size === 0 || !amount) return;

    const parsedAmount = parseEther(amount);
    const numbers = Array.from(selected).sort((a, b) => a - b);
    const amounts = numbers.map(() => parsedAmount);
    const tokens = numbers.map(() => NATIVE_TOKEN);

    onSubmit(numbers, amounts, tokens);
    setSelected(new Set());
    setAmount("");
  }

  return (
    <div className="space-y-4">
      <NumberGrid
        selected={selected}
        onToggle={handleToggle}
        disabled={disabled || isPending}
      />

      {selected.size > 0 && (
        <div className="space-y-3">
          <div className="flex items-center gap-2">
            <span className="text-sm text-muted-foreground">
              Selected: {Array.from(selected).sort((a, b) => a - b).join(", ")}
            </span>
          </div>

          <div className="flex items-center gap-3">
            <label htmlFor="bet-amount" className="text-sm">
              Amount per number (ART):
            </label>
            <input
              id="bet-amount"
              type="number"
              min="0"
              step="0.01"
              value={amount}
              onChange={(e) => setAmount(e.target.value)}
              placeholder="0.0"
              className="w-32 rounded border border-border bg-background px-3 py-1.5 text-sm"
              data-testid="bet-amount-input"
            />
          </div>

          <Button
            onClick={handleSubmit}
            disabled={isPending || !amount || selected.size === 0}
            data-testid="submit-bets"
          >
            {isPending ? "Confirming..." : `Place ${selected.size} Bet${selected.size > 1 ? "s" : ""}`}
          </Button>
        </div>
      )}
    </div>
  );
}
```

- [ ] **Step 4: Create RoundCard component**

Write to `packages/webapp/src/components/round/RoundCard.tsx`:

```tsx
import { useEffect, useState } from "react";

const STATUS_LABELS: Record<number, string> = {
  0: "Open",
  1: "Closed",
  2: "Resolved",
  3: "Settled",
};

const STATUS_COLORS: Record<number, string> = {
  0: "bg-green-500/20 text-green-400",
  1: "bg-yellow-500/20 text-yellow-400",
  2: "bg-blue-500/20 text-blue-400",
  3: "bg-muted text-muted-foreground",
};

interface RoundCardProps {
  roundId: bigint;
  closeTimestamp: bigint;
  status: number;
}

export function RoundCard({ roundId, closeTimestamp, status }: RoundCardProps) {
  const [countdown, setCountdown] = useState("");

  useEffect(() => {
    function update() {
      const now = Math.floor(Date.now() / 1000);
      const diff = Number(closeTimestamp) - now;
      if (diff <= 0) {
        setCountdown("Closed");
        return;
      }
      const hours = Math.floor(diff / 3600);
      const mins = Math.floor((diff % 3600) / 60);
      const secs = diff % 60;
      setCountdown(`${hours}h ${mins}m ${secs}s`);
    }

    update();
    const interval = setInterval(update, 1000);
    return () => clearInterval(interval);
  }, [closeTimestamp]);

  return (
    <div className="rounded-lg border border-border p-6" data-testid="round-card">
      <div className="flex items-center justify-between">
        <h2 className="text-xl font-bold" data-testid="round-id">
          Round #{roundId.toString()}
        </h2>
        <span
          className={`rounded-full px-3 py-1 text-xs font-medium ${STATUS_COLORS[status] ?? ""}`}
          data-testid="round-status"
        >
          {STATUS_LABELS[status] ?? "Unknown"}
        </span>
      </div>
      {status === 0 && (
        <p className="mt-2 text-sm text-muted-foreground" data-testid="round-countdown">
          Closes in: {countdown}
        </p>
      )}
    </div>
  );
}
```

- [ ] **Step 5: Create RoundHistory component**

Write to `packages/webapp/src/components/round/RoundHistory.tsx`:

```tsx
import { Link } from "react-router";
import { formatEther } from "viem";

const STATUS_LABELS: Record<number, string> = {
  0: "Open",
  1: "Closed",
  2: "Resolved",
  3: "Settled",
};

interface RoundRow {
  id: string;
  status: number;
  winningNumber: number | null;
  totalBets: number;
  totalVolume: string;
}

interface RoundHistoryProps {
  rounds: RoundRow[];
  isLoading: boolean;
}

export function RoundHistory({ rounds, isLoading }: RoundHistoryProps) {
  if (isLoading) {
    return <p className="text-sm text-muted-foreground">Loading rounds...</p>;
  }

  if (rounds.length === 0) {
    return <p className="text-sm text-muted-foreground">No rounds yet</p>;
  }

  return (
    <div className="overflow-x-auto" data-testid="round-history">
      <table className="w-full text-sm">
        <thead>
          <tr className="border-b border-border text-left text-muted-foreground">
            <th className="pb-2 pr-4">Round</th>
            <th className="pb-2 pr-4">Status</th>
            <th className="pb-2 pr-4">Winner</th>
            <th className="pb-2 pr-4">Bets</th>
            <th className="pb-2">Volume</th>
          </tr>
        </thead>
        <tbody>
          {rounds.map((r) => (
            <tr key={r.id} className="border-b border-border/50">
              <td className="py-2 pr-4">
                <Link to={`/rounds/${r.id}`} className="text-primary hover:underline">
                  #{r.id}
                </Link>
              </td>
              <td className="py-2 pr-4">{STATUS_LABELS[r.status] ?? "Unknown"}</td>
              <td className="py-2 pr-4 font-mono">
                {r.winningNumber !== null ? r.winningNumber : "-"}
              </td>
              <td className="py-2 pr-4">{r.totalBets}</td>
              <td className="py-2">{formatEther(BigInt(r.totalVolume))} ART</td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}
```

- [ ] **Step 6: Verify TypeScript compiles**

Run:
```bash
cd packages/webapp && npx tsc --noEmit
```

- [ ] **Step 7: Commit**

```bash
git add packages/webapp/src/components/betting/ packages/webapp/src/components/round/
git commit -m "feat(webapp): add betting and round UI components"
```

---

### Task 5: Rounds page

**Files:**
- Modify: `packages/webapp/src/pages/Rounds.tsx`

- [ ] **Step 1: Replace placeholder Rounds page**

Write to `packages/webapp/src/pages/Rounds.tsx`:

```tsx
import { useAccount } from "wagmi";
import { formatEther } from "viem";
import { Link } from "react-router";
import { useCurrentRound } from "@/hooks/useCurrentRound";
import { useUserBets } from "@/hooks/useUserBets";
import { useRounds } from "@/hooks/useRounds";
import { RoundCard } from "@/components/round/RoundCard";
import { BetList } from "@/components/betting/BetList";
import { RoundHistory } from "@/components/round/RoundHistory";

export function Rounds() {
  const { isConnected } = useAccount();
  const { data: currentRound } = useCurrentRound();
  const roundId = currentRound ? currentRound[0] : undefined;
  const { data: userBets } = useUserBets(roundId);
  const { data: pastRounds, isLoading: roundsLoading } = useRounds();

  const bets = userBets
    ? userBets[0].map((number, i) => ({
        number: Number(number),
        amount: userBets[2][i],
        token: userBets[1][i] as `0x${string}`,
        claimed: userBets[3][i],
      }))
    : [];

  return (
    <div className="space-y-8">
      <h1 className="text-2xl font-bold">Rounds</h1>

      {currentRound && (
        <section className="space-y-4">
          <div className="flex items-center justify-between">
            <h2 className="text-lg font-semibold">Current Round</h2>
            <Link
              to={`/rounds/${currentRound[0].toString()}`}
              className="text-sm text-primary hover:underline"
            >
              View Details
            </Link>
          </div>
          <RoundCard
            roundId={currentRound[0]}
            closeTimestamp={currentRound[1]}
            status={Number(currentRound[2])}
          />
        </section>
      )}

      {isConnected && bets.length > 0 && (
        <section className="space-y-4">
          <h2 className="text-lg font-semibold">Your Bets This Round</h2>
          <BetList bets={bets} />
        </section>
      )}

      <section className="space-y-4">
        <h2 className="text-lg font-semibold">Past Rounds</h2>
        <RoundHistory rounds={pastRounds ?? []} isLoading={roundsLoading} />
      </section>
    </div>
  );
}
```

- [ ] **Step 2: Verify TypeScript compiles**

Run:
```bash
cd packages/webapp && npx tsc --noEmit
```

- [ ] **Step 3: Commit**

```bash
git add packages/webapp/src/pages/Rounds.tsx
git commit -m "feat(webapp): implement Rounds page with current round and history"
```

---

### Task 6: Round Detail page

**Files:**
- Modify: `packages/webapp/src/pages/RoundDetail.tsx`

- [ ] **Step 1: Replace placeholder RoundDetail page**

Write to `packages/webapp/src/pages/RoundDetail.tsx`:

```tsx
import { useParams } from "react-router";
import { useAccount } from "wagmi";
import { useCurrentRound } from "@/hooks/useCurrentRound";
import { useUserBets } from "@/hooks/useUserBets";
import { usePlaceBets } from "@/hooks/usePlaceBets";
import { useUpdateBet } from "@/hooks/useUpdateBet";
import { useClaimWinnings } from "@/hooks/useClaimWinnings";
import { RoundCard } from "@/components/round/RoundCard";
import { PlaceBetForm } from "@/components/betting/PlaceBetForm";
import { BetList } from "@/components/betting/BetList";

const NATIVE_TOKEN = "0x0000000000000000000000000000000000000000" as `0x${string}`;

export function RoundDetail() {
  const { id } = useParams<{ id: string }>();
  const roundId = id ? BigInt(id) : undefined;
  const { isConnected } = useAccount();
  const { data: currentRound } = useCurrentRound();
  const { data: userBets, refetch: refetchBets } = useUserBets(roundId);
  const { placeBet, placeBets, isPending: isPlacing } = usePlaceBets();
  const { updateBet, isPending: isUpdating } = useUpdateBet();
  const { claimWinnings, isPending: isClaiming } = useClaimWinnings();

  const isCurrentRound = currentRound && roundId !== undefined && currentRound[0] === roundId;
  const roundStatus = currentRound && isCurrentRound ? Number(currentRound[2]) : undefined;
  const closeTimestamp = currentRound && isCurrentRound ? currentRound[1] : 0n;
  const isOpen = roundStatus === 0;

  const bets = userBets
    ? userBets[0].map((number, i) => ({
        number: Number(number),
        amount: userBets[2][i],
        token: userBets[1][i] as `0x${string}`,
        claimed: userBets[3][i],
      }))
    : [];

  const winningNumber = currentRound && isCurrentRound && roundStatus === 2
    ? undefined
    : undefined;

  function handlePlaceBets(numbers: number[], amounts: bigint[], tokens: `0x${string}`[]) {
    if (numbers.length === 1) {
      placeBet(numbers[0], tokens[0], amounts[0]);
    } else {
      placeBets(numbers, amounts, tokens);
    }
  }

  function handleRemove(number: number) {
    if (roundId === undefined) return;
    updateBet(roundId, number, 0n, NATIVE_TOKEN);
  }

  function handleClaim(number: number) {
    if (roundId === undefined) return;
    claimWinnings(roundId, number);
  }

  return (
    <div className="space-y-8">
      <h1 className="text-2xl font-bold">Round #{id}</h1>

      {isCurrentRound && currentRound && (
        <RoundCard
          roundId={currentRound[0]}
          closeTimestamp={closeTimestamp}
          status={roundStatus ?? 0}
        />
      )}

      {isConnected && isOpen && (
        <section className="space-y-4">
          <h2 className="text-lg font-semibold">Place Bets</h2>
          <PlaceBetForm
            onSubmit={handlePlaceBets}
            isPending={isPlacing}
            disabled={!isOpen}
          />
        </section>
      )}

      {isConnected && bets.length > 0 && (
        <section className="space-y-4">
          <h2 className="text-lg font-semibold">Your Bets</h2>
          <BetList
            bets={bets}
            editable={isOpen}
            onRemove={handleRemove}
          />

          {roundStatus === 2 && bets.some((b) => !b.claimed) && (
            <div className="space-y-2">
              <h3 className="text-sm font-medium">Claim Winnings</h3>
              {bets
                .filter((b) => !b.claimed)
                .map((b) => (
                  <button
                    key={b.number}
                    onClick={() => handleClaim(b.number)}
                    disabled={isClaiming}
                    className="mr-2 rounded bg-primary px-3 py-1.5 text-sm text-primary-foreground hover:opacity-90 disabled:opacity-50"
                    data-testid={`claim-${b.number}`}
                  >
                    Claim #{b.number}
                  </button>
                ))}
            </div>
          )}
        </section>
      )}
    </div>
  );
}
```

- [ ] **Step 2: Verify TypeScript compiles**

Run:
```bash
cd packages/webapp && npx tsc --noEmit
```

- [ ] **Step 3: Commit**

```bash
git add packages/webapp/src/pages/RoundDetail.tsx
git commit -m "feat(webapp): implement RoundDetail page with bet form and management"
```

---

### Task 7: Admin page

**Files:**
- Modify: `packages/webapp/src/pages/Admin.tsx`

- [ ] **Step 1: Replace placeholder Admin page**

Write to `packages/webapp/src/pages/Admin.tsx`:

```tsx
import { useState } from "react";
import { useAccount } from "wagmi";
import { ConnectButton } from "@rainbow-me/rainbowkit";
import { useCurrentRound } from "@/hooks/useCurrentRound";
import { useAdminActions } from "@/hooks/useAdminActions";
import { Button } from "@/components/ui/button";

const ADMIN_ADDRESS = "0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266";

export function Admin() {
  const { address, isConnected } = useAccount();
  const { data: currentRound } = useCurrentRound();
  const { submitResult, adminClaimPool, forceCloseRound, isPending, isSuccess, error } =
    useAdminActions();

  const [roundIdInput, setRoundIdInput] = useState("");
  const [numberInput, setNumberInput] = useState("");

  const isAdmin = isConnected && address?.toLowerCase() === ADMIN_ADDRESS.toLowerCase();

  const defaultRoundId = currentRound ? currentRound[0].toString() : "";

  if (!isConnected) {
    return (
      <div className="flex flex-col items-center gap-6 pt-24">
        <h1 className="text-2xl font-bold">Admin Panel</h1>
        <p className="text-muted-foreground">Connect admin wallet to continue</p>
        <ConnectButton />
      </div>
    );
  }

  if (!isAdmin) {
    return (
      <div className="flex flex-col items-center gap-6 pt-24">
        <h1 className="text-2xl font-bold">Admin Panel</h1>
        <p className="text-muted-foreground">
          Connected wallet is not the admin account
        </p>
        <p className="font-mono text-xs text-muted-foreground">{address}</p>
      </div>
    );
  }

  return (
    <div className="space-y-8">
      <h1 className="text-2xl font-bold">Admin Panel</h1>

      {error && (
        <div className="rounded border border-red-500/50 bg-red-500/10 p-3 text-sm text-red-400">
          {error.message}
        </div>
      )}

      {isSuccess && (
        <div className="rounded border border-green-500/50 bg-green-500/10 p-3 text-sm text-green-400">
          Transaction confirmed
        </div>
      )}

      <section className="space-y-4 rounded-lg border border-border p-6">
        <h2 className="text-lg font-semibold">Submit Result</h2>
        <div className="flex items-end gap-3">
          <div>
            <label htmlFor="result-round" className="text-sm text-muted-foreground">
              Round ID
            </label>
            <input
              id="result-round"
              type="number"
              value={roundIdInput || defaultRoundId}
              onChange={(e) => setRoundIdInput(e.target.value)}
              className="mt-1 block w-32 rounded border border-border bg-background px-3 py-1.5 text-sm"
              data-testid="admin-round-id"
            />
          </div>
          <div>
            <label htmlFor="result-number" className="text-sm text-muted-foreground">
              Winning Number (0-99)
            </label>
            <input
              id="result-number"
              type="number"
              min="0"
              max="99"
              value={numberInput}
              onChange={(e) => setNumberInput(e.target.value)}
              className="mt-1 block w-24 rounded border border-border bg-background px-3 py-1.5 text-sm"
              data-testid="admin-winning-number"
            />
          </div>
          <Button
            onClick={() => {
              const rid = roundIdInput || defaultRoundId;
              if (rid && numberInput) {
                submitResult(BigInt(rid), Number(numberInput));
              }
            }}
            disabled={isPending}
            data-testid="admin-submit-result"
          >
            {isPending ? "Submitting..." : "Submit Result"}
          </Button>
        </div>
      </section>

      <section className="space-y-4 rounded-lg border border-border p-6">
        <h2 className="text-lg font-semibold">Admin Claim Pool</h2>
        <div className="flex items-end gap-3">
          <div>
            <label htmlFor="claim-round" className="text-sm text-muted-foreground">
              Round ID
            </label>
            <input
              id="claim-round"
              type="number"
              value={roundIdInput || defaultRoundId}
              onChange={(e) => setRoundIdInput(e.target.value)}
              className="mt-1 block w-32 rounded border border-border bg-background px-3 py-1.5 text-sm"
            />
          </div>
          <Button
            onClick={() => {
              const rid = roundIdInput || defaultRoundId;
              if (rid) adminClaimPool(BigInt(rid));
            }}
            disabled={isPending}
            data-testid="admin-claim-pool"
          >
            {isPending ? "Claiming..." : "Claim Pool"}
          </Button>
        </div>
      </section>

      <section className="space-y-4 rounded-lg border border-border p-6">
        <h2 className="text-lg font-semibold">Force Close Round</h2>
        <div className="flex items-end gap-3">
          <div>
            <label htmlFor="close-round" className="text-sm text-muted-foreground">
              Round ID
            </label>
            <input
              id="close-round"
              type="number"
              value={roundIdInput || defaultRoundId}
              onChange={(e) => setRoundIdInput(e.target.value)}
              className="mt-1 block w-32 rounded border border-border bg-background px-3 py-1.5 text-sm"
            />
          </div>
          <Button
            onClick={() => {
              const rid = roundIdInput || defaultRoundId;
              if (rid) forceCloseRound(BigInt(rid));
            }}
            disabled={isPending}
            variant="outline"
            data-testid="admin-force-close"
          >
            {isPending ? "Closing..." : "Force Close"}
          </Button>
        </div>
      </section>
    </div>
  );
}
```

- [ ] **Step 2: Verify TypeScript compiles**

Run:
```bash
cd packages/webapp && npx tsc --noEmit
```

- [ ] **Step 3: Commit**

```bash
git add packages/webapp/src/pages/Admin.tsx
git commit -m "feat(webapp): implement Admin page with submit result, claim pool, force close"
```

---

### Task 8: Indexer -- handle BetUpdated event

**Files:**
- Modify: `packages/indexer/src/index.ts`

- [ ] **Step 1: Update bet ID format in BetPlaced handler**

In `packages/indexer/src/index.ts`, the current `betId` is `${roundId}-${user}`. Change to `${roundId}-${user}-${number}` to support multi-number bets.

Change:
```ts
const betId = `${roundId}-${user}`;
```
to:
```ts
const betId = `${roundId}-${user}-${number}`;
```

Also remove the `existingBet` lookup (was used for unique user counting -- replace with a simpler approach or keep the logic but use new ID).

- [ ] **Step 2: Add BetUpdated event handler**

Add after the `PoolClaimed` handler:

```ts
ponder.on("BettingPrecompile:BetUpdated", async ({ event, context }) => {
  const { db } = context;
  const roundId = event.args.roundId;
  const user = event.args.user;
  const number = event.args.number;
  const oldAmount = event.args.oldAmount;
  const newAmount = event.args.newAmount;
  const betId = `${roundId}-${user}-${number}`;
  const dayId = roundId.toString();
  const amountDiff = newAmount - oldAmount;

  if (newAmount === 0n) {
    // Bet removed
    await db.delete(bet, { id: betId });

    await db
      .insert(round)
      .values({
        id: roundId,
        status: RoundStatus.Open,
        totalBets: 0,
        totalVolume: 0n,
        totalPayout: 0n,
        poolClaimed: 0n,
      })
      .onConflictDoUpdate((row) => ({
        totalBets: row.totalBets - 1,
        totalVolume: row.totalVolume - oldAmount,
      }));
  } else {
    // Bet updated
    await db.update(bet, { id: betId }).set({
      amount: newAmount,
    });

    await db
      .insert(round)
      .values({
        id: roundId,
        status: RoundStatus.Open,
        totalBets: 0,
        totalVolume: 0n,
        totalPayout: 0n,
        poolClaimed: 0n,
      })
      .onConflictDoUpdate((row) => ({
        totalVolume: row.totalVolume + amountDiff,
      }));
  }

  // Update user stats
  await db
    .insert(userStats)
    .values({
      address: user,
      totalBets: 0,
      totalWagered: newAmount > oldAmount ? newAmount - oldAmount : 0n,
      totalWon: 0n,
      totalClaimed: 0n,
      winCount: 0,
    })
    .onConflictDoUpdate((row) => ({
      totalWagered: row.totalWagered + amountDiff,
    }));

  // Update daily stats
  await db
    .insert(dailyStats)
    .values({
      id: dayId,
      totalBets: 0,
      totalVolume: amountDiff,
      totalPayout: 0n,
      uniqueUsers: 0,
    })
    .onConflictDoUpdate((row) => ({
      totalVolume: row.totalVolume + amountDiff,
    }));

  // Update protocol stats
  await db
    .insert(protocolStats)
    .values({
      id: "global",
      totalRounds: 0,
      totalBets: newAmount === 0n ? -1 : 0,
      totalVolume: amountDiff,
      totalPayout: 0n,
      totalPoolClaimed: 0n,
    })
    .onConflictDoUpdate((row) => ({
      totalBets: row.totalBets + (newAmount === 0n ? -1 : 0),
      totalVolume: row.totalVolume + amountDiff,
    }));
});
```

- [ ] **Step 3: Verify indexer builds**

Run:
```bash
cd packages/indexer && pnpm build 2>&1 || echo "Build check done"
```

Note: Ponder may not have a traditional build step. Just verify no TypeScript errors.

- [ ] **Step 4: Commit**

```bash
git add packages/indexer/src/index.ts
git commit -m "feat(indexer): handle BetUpdated event, update bet ID for multi-number"
```

---

### Task 9: Build verification

- [ ] **Step 1: TypeScript check webapp**

Run:
```bash
cd packages/webapp && npx tsc --noEmit
```

Expected: No errors.

- [ ] **Step 2: Production build webapp**

Run:
```bash
cd packages/webapp && pnpm build
```

Expected: Build succeeds.

- [ ] **Step 3: Full turbo build**

Run:
```bash
cd /Users/huyduan/projects/blockchain && pnpm turbo build
```

Expected: All packages build.

---

### Task 10: E2E test -- place single bet

**Files:**
- Create: `packages/webapp/e2e/place-bet.spec.ts`

- [ ] **Step 1: Write the test**

Write to `packages/webapp/e2e/place-bet.spec.ts`:

```ts
import { test, expect } from "./fixtures";

test.describe("Place Single Bet", () => {
  test("places a bet on a number and sees it in bet list", async ({
    wallet,
    context,
  }) => {
    await wallet.addNetwork({
      networkName: "Artemis",
      rpc: "http://localhost:9944",
      chainId: 322,
      symbol: "ART",
    });

    const appPage = await context.newPage();
    await appPage.bringToFront();
    await appPage.goto("http://localhost:3000", { waitUntil: "networkidle" });

    // Connect wallet
    await appPage.getByRole("main").getByTestId("rk-connect-button").click();
    await appPage.getByRole("button", { name: /metamask/i }).click();
    await wallet.approve();
    await appPage.bringToFront();

    // Wait for wallet to connect
    await expect(appPage.getByTestId("wallet-address")).toBeVisible({ timeout: 10_000 });

    // Navigate to rounds
    await appPage.getByRole("link", { name: "Rounds" }).click();
    await expect(appPage.getByText("Current Round")).toBeVisible({ timeout: 10_000 });

    // Click on current round detail link
    await appPage.getByText("View Details").click();

    // Wait for round detail page
    await expect(appPage.getByTestId("number-grid")).toBeVisible({ timeout: 10_000 });

    // Select number 42
    await appPage.getByTestId("number-42").click();

    // Enter amount
    await appPage.getByTestId("bet-amount-input").fill("1");

    // Submit bet
    await appPage.getByTestId("submit-bets").click();

    // Approve transaction in MetaMask
    await wallet.confirmTransaction({ priority: 0 });
    await appPage.bringToFront();

    // Verify bet appears in bet list
    await expect(appPage.getByTestId("bet-item-42")).toBeVisible({ timeout: 15_000 });
    await expect(appPage.getByTestId("bet-number-42")).toContainText("#42");
  });
});
```

- [ ] **Step 2: Commit**

```bash
git add packages/webapp/e2e/place-bet.spec.ts
git commit -m "test(webapp): add E2E test for placing single bet"
```

---

### Task 11: E2E test -- place multiple bets

**Files:**
- Create: `packages/webapp/e2e/place-bets.spec.ts`

- [ ] **Step 1: Write the test**

Write to `packages/webapp/e2e/place-bets.spec.ts`:

```ts
import { test, expect } from "./fixtures";

test.describe("Place Multiple Bets", () => {
  test("places bets on multiple numbers in one transaction", async ({
    wallet,
    context,
  }) => {
    await wallet.addNetwork({
      networkName: "Artemis",
      rpc: "http://localhost:9944",
      chainId: 322,
      symbol: "ART",
    });

    const appPage = await context.newPage();
    await appPage.bringToFront();
    await appPage.goto("http://localhost:3000", { waitUntil: "networkidle" });

    // Connect wallet
    await appPage.getByRole("main").getByTestId("rk-connect-button").click();
    await appPage.getByRole("button", { name: /metamask/i }).click();
    await wallet.approve();
    await appPage.bringToFront();
    await expect(appPage.getByTestId("wallet-address")).toBeVisible({ timeout: 10_000 });

    // Navigate to current round detail
    await appPage.getByRole("link", { name: "Rounds" }).click();
    await appPage.getByText("View Details").click();
    await expect(appPage.getByTestId("number-grid")).toBeVisible({ timeout: 10_000 });

    // Select numbers 10, 20, 30
    await appPage.getByTestId("number-10").click();
    await appPage.getByTestId("number-20").click();
    await appPage.getByTestId("number-30").click();

    // Enter amount
    await appPage.getByTestId("bet-amount-input").fill("0.5");

    // Submit
    await appPage.getByTestId("submit-bets").click();
    await wallet.confirmTransaction({ priority: 0 });
    await appPage.bringToFront();

    // Verify all 3 bets appear
    await expect(appPage.getByTestId("bet-item-10")).toBeVisible({ timeout: 15_000 });
    await expect(appPage.getByTestId("bet-item-20")).toBeVisible();
    await expect(appPage.getByTestId("bet-item-30")).toBeVisible();
  });
});
```

- [ ] **Step 2: Commit**

```bash
git add packages/webapp/e2e/place-bets.spec.ts
git commit -m "test(webapp): add E2E test for placing multiple bets"
```

---

### Task 12: E2E test -- admin submit result

**Files:**
- Create: `packages/webapp/e2e/admin.spec.ts`

- [ ] **Step 1: Write the test**

Write to `packages/webapp/e2e/admin.spec.ts`:

```ts
import { test, expect } from "./fixtures";

test.describe("Admin Panel", () => {
  test("admin submits result for a round", async ({
    wallet,
    context,
  }) => {
    await wallet.addNetwork({
      networkName: "Artemis",
      rpc: "http://localhost:9944",
      chainId: 322,
      symbol: "ART",
    });

    const appPage = await context.newPage();
    await appPage.bringToFront();
    await appPage.goto("http://localhost:3000", { waitUntil: "networkidle" });

    // Connect wallet (account #0 is admin)
    await appPage.getByRole("main").getByTestId("rk-connect-button").click();
    await appPage.getByRole("button", { name: /metamask/i }).click();
    await wallet.approve();
    await appPage.bringToFront();
    await expect(appPage.getByTestId("wallet-address")).toBeVisible({ timeout: 10_000 });

    // First place a bet so there is a round
    await appPage.getByRole("link", { name: "Rounds" }).click();
    await appPage.getByText("View Details").click();
    await expect(appPage.getByTestId("number-grid")).toBeVisible({ timeout: 10_000 });
    await appPage.getByTestId("number-7").click();
    await appPage.getByTestId("bet-amount-input").fill("1");
    await appPage.getByTestId("submit-bets").click();
    await wallet.confirmTransaction({ priority: 0 });
    await appPage.bringToFront();
    await expect(appPage.getByTestId("bet-item-7")).toBeVisible({ timeout: 15_000 });

    // Navigate to admin
    await appPage.getByRole("link", { name: "Admin" }).click();

    // Verify admin panel is visible (not the "not admin" message)
    await expect(appPage.getByTestId("admin-submit-result")).toBeVisible({ timeout: 10_000 });

    // First force close the round so we can submit result
    await appPage.getByTestId("admin-force-close").click();
    await wallet.confirmTransaction({ priority: 0 });
    await appPage.bringToFront();

    // Wait for tx confirmation
    await expect(appPage.getByText("Transaction confirmed")).toBeVisible({ timeout: 15_000 });

    // Submit result: winning number 7
    await appPage.getByTestId("admin-winning-number").fill("7");
    await appPage.getByTestId("admin-submit-result").click();
    await wallet.confirmTransaction({ priority: 0 });
    await appPage.bringToFront();

    // Verify success
    await expect(appPage.getByText("Transaction confirmed")).toBeVisible({ timeout: 15_000 });
  });
});
```

- [ ] **Step 2: Commit**

```bash
git add packages/webapp/e2e/admin.spec.ts
git commit -m "test(webapp): add E2E test for admin submit result"
```

---

### Task 13: E2E test -- update/remove bet and claim winnings

**Files:**
- Create: `packages/webapp/e2e/update-bet.spec.ts`
- Create: `packages/webapp/e2e/claim-winnings.spec.ts`

- [ ] **Step 1: Write update bet test**

Write to `packages/webapp/e2e/update-bet.spec.ts`:

```ts
import { test, expect } from "./fixtures";

test.describe("Update Bet", () => {
  test("removes a bet by clicking remove button", async ({
    wallet,
    context,
  }) => {
    await wallet.addNetwork({
      networkName: "Artemis",
      rpc: "http://localhost:9944",
      chainId: 322,
      symbol: "ART",
    });

    const appPage = await context.newPage();
    await appPage.bringToFront();
    await appPage.goto("http://localhost:3000", { waitUntil: "networkidle" });

    // Connect wallet
    await appPage.getByRole("main").getByTestId("rk-connect-button").click();
    await appPage.getByRole("button", { name: /metamask/i }).click();
    await wallet.approve();
    await appPage.bringToFront();
    await expect(appPage.getByTestId("wallet-address")).toBeVisible({ timeout: 10_000 });

    // Place a bet first
    await appPage.getByRole("link", { name: "Rounds" }).click();
    await appPage.getByText("View Details").click();
    await expect(appPage.getByTestId("number-grid")).toBeVisible({ timeout: 10_000 });
    await appPage.getByTestId("number-55").click();
    await appPage.getByTestId("bet-amount-input").fill("1");
    await appPage.getByTestId("submit-bets").click();
    await wallet.confirmTransaction({ priority: 0 });
    await appPage.bringToFront();
    await expect(appPage.getByTestId("bet-item-55")).toBeVisible({ timeout: 15_000 });

    // Remove the bet
    await appPage.getByTestId("remove-bet-55").click();
    await wallet.confirmTransaction({ priority: 0 });
    await appPage.bringToFront();

    // Verify bet is removed
    await expect(appPage.getByTestId("bet-item-55")).not.toBeVisible({ timeout: 15_000 });
  });
});
```

- [ ] **Step 2: Write claim winnings test**

Write to `packages/webapp/e2e/claim-winnings.spec.ts`:

```ts
import { test, expect } from "./fixtures";

test.describe("Claim Winnings", () => {
  test("places winning bet, admin submits result, claims winnings", async ({
    wallet,
    context,
  }) => {
    await wallet.addNetwork({
      networkName: "Artemis",
      rpc: "http://localhost:9944",
      chainId: 322,
      symbol: "ART",
    });

    const appPage = await context.newPage();
    await appPage.bringToFront();
    await appPage.goto("http://localhost:3000", { waitUntil: "networkidle" });

    // Connect wallet (account #0 is admin and bettor)
    await appPage.getByRole("main").getByTestId("rk-connect-button").click();
    await appPage.getByRole("button", { name: /metamask/i }).click();
    await wallet.approve();
    await appPage.bringToFront();
    await expect(appPage.getByTestId("wallet-address")).toBeVisible({ timeout: 10_000 });

    // Place a bet on number 33
    await appPage.getByRole("link", { name: "Rounds" }).click();
    await appPage.getByText("View Details").click();
    await expect(appPage.getByTestId("number-grid")).toBeVisible({ timeout: 10_000 });
    await appPage.getByTestId("number-33").click();
    await appPage.getByTestId("bet-amount-input").fill("1");
    await appPage.getByTestId("submit-bets").click();
    await wallet.confirmTransaction({ priority: 0 });
    await appPage.bringToFront();
    await expect(appPage.getByTestId("bet-item-33")).toBeVisible({ timeout: 15_000 });

    // Go to admin, force close round, submit result 33
    await appPage.getByRole("link", { name: "Admin" }).click();
    await expect(appPage.getByTestId("admin-force-close")).toBeVisible({ timeout: 10_000 });

    await appPage.getByTestId("admin-force-close").click();
    await wallet.confirmTransaction({ priority: 0 });
    await appPage.bringToFront();
    await expect(appPage.getByText("Transaction confirmed")).toBeVisible({ timeout: 15_000 });

    await appPage.getByTestId("admin-winning-number").fill("33");
    await appPage.getByTestId("admin-submit-result").click();
    await wallet.confirmTransaction({ priority: 0 });
    await appPage.bringToFront();
    await expect(appPage.getByText("Transaction confirmed")).toBeVisible({ timeout: 15_000 });

    // Go back to round detail and claim
    await appPage.getByRole("link", { name: "Rounds" }).click();
    await appPage.getByText("View Details").click();

    // Look for claim button
    await expect(appPage.getByTestId("claim-33")).toBeVisible({ timeout: 15_000 });
    await appPage.getByTestId("claim-33").click();
    await wallet.confirmTransaction({ priority: 0 });
    await appPage.bringToFront();

    // Verify claim succeeded (bet should show as claimed)
    await expect(appPage.getByText("Claimed")).toBeVisible({ timeout: 15_000 });
  });
});
```

- [ ] **Step 3: Commit**

```bash
git add packages/webapp/e2e/update-bet.spec.ts packages/webapp/e2e/claim-winnings.spec.ts
git commit -m "test(webapp): add E2E tests for update bet and claim winnings"
```
