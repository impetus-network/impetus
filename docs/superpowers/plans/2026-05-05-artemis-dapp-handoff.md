# Artemis Dapp Handoff Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Rebuild the existing Artemis Next.js UI so Home, Transfer, and Explorer visually match the `Artemis Dapp.html` handoff while preserving Artemis chain facts from the repo.

**Architecture:** Keep the existing Next.js routes and wagmi/RainbowKit provider. Add focused `components/dapp/*` files for prototype-specific data, live feed, transfer controls, and explorer panels, then update the three route pages plus shared header/footer. Use CSS tokens and Tailwind classes instead of copying inline prototype styles.

**Tech Stack:** Next.js 15, React 19, TypeScript strict, Tailwind v4, wagmi, RainbowKit, viem, lucide-react.

---

## File Structure

- Create `packages/ui/components/dapp/types.ts`
  - Shared typed data contracts for mock transactions, tokens, blocks, validators, and feed state.
- Create `packages/ui/components/dapp/mockData.ts`
  - Deterministic Artemis demo data and pure helpers for hashes, addresses, transaction rows, block rows, and validators.
- Create `packages/ui/components/dapp/LiveFeed.tsx`
  - Client hook and presentational components for the ticking prototype feed.
- Create `packages/ui/components/dapp/DappPrimitives.tsx`
  - Small visual primitives: pulse dot, mono text, stat block, panel frame, overview card, token icon, transaction type badge.
- Create `packages/ui/components/dapp/TokenPicker.tsx`
  - Token selector and token balance list for the Transfer screen.
- Create `packages/ui/components/dapp/ExplorerPanels.tsx`
  - Explorer search frame, overview cards, latest blocks, latest transactions with filters, validator table.
- Modify `packages/ui/app/globals.css`
  - Add dapp animation keyframes, selection styling, and a few utility classes aligned to the handoff.
- Modify `packages/ui/components/layout/AppLayout.tsx`
  - Remove the global centered/narrow `<main>` wrapper so each route can control prototype spacing.
- Modify `packages/ui/components/layout/Header.tsx`
  - Adopt prototype nav structure while keeping Next links, admin/debug access, and real wallet button.
- Modify `packages/ui/components/layout/Footer.tsx`
  - Adopt prototype footer columns and correct Artemis Chain ID `322`.
- Modify `packages/ui/components/scaffold/ConnectButtonCustom.tsx`
  - Restyle the wallet pill to match the prototype; keep real connect/disconnect behavior.
- Modify `packages/ui/app/page.tsx`
  - Replace current compact dashboard with prototype Home structure.
- Modify `packages/ui/app/transfer/page.tsx`
  - Replace current ART-only transfer form with prototype multi-asset demo form and validation states.
- Modify `packages/ui/app/blockexplorer/page.tsx`
  - Replace current chain-data layout with prototype explorer panels.

---

### Task 1: Add Shared Dapp Data Types And Mock Data

**Files:**
- Create: `packages/ui/components/dapp/types.ts`
- Create: `packages/ui/components/dapp/mockData.ts`

- [ ] **Step 1: Create strict shared types**

Create `packages/ui/components/dapp/types.ts`:

```ts
export type TxKind = "transfer" | "swap" | "mint" | "contract";

export type MockTransaction = {
  hash: `0x${string}`;
  from: `0x${string}`;
  to: `0x${string}`;
  value: string;
  type: TxKind;
  age: number;
};

export type LiveFeedState = {
  tps: number;
  block: number;
  gasPrice: 0;
  txs: MockTransaction[];
};

export type DemoToken = {
  sym: string;
  name: string;
  balance: number;
  usd: number;
  color: string;
};

export type MockBlock = {
  number: number;
  hash: `0x${string}`;
  txCount: number;
  proposer: string;
  age: number;
};

export type ValidatorRow = {
  rank: number;
  name: string;
  stake: string;
  blocks: string;
  uptime: string;
};
```

- [ ] **Step 2: Add deterministic demo data helpers**

Create `packages/ui/components/dapp/mockData.ts`:

```ts
import type {
  DemoToken,
  MockBlock,
  MockTransaction,
  TxKind,
  ValidatorRow,
} from "./types";

const txTypes: TxKind[] = [
  "transfer",
  "swap",
  "mint",
  "contract",
  "transfer",
  "transfer",
];

export const demoTokens: DemoToken[] = [
  { sym: "ART", name: "Artemis", balance: 1248.42, usd: 1.84, color: "#ffb084" },
  { sym: "USDC", name: "USD Coin", balance: 5430, usd: 1, color: "#2775ca" },
  { sym: "USDT", name: "Tether", balance: 200.5, usd: 1, color: "#26a17b" },
  { sym: "WETH", name: "Wrapped ETH", balance: 2.4831, usd: 3210, color: "#627eea" },
  { sym: "WBTC", name: "Wrapped BTC", balance: 0.1422, usd: 68420, color: "#f7931a" },
  { sym: "AGT", name: "Argent", balance: 89000, usd: 0.012, color: "#b8a4ed" },
];

export const validators: ValidatorRow[] = [
  { rank: 1, name: "Selene Labs", stake: "4.82M", blocks: "218,492", uptime: "100.00%" },
  { rank: 2, name: "Phoebe Stake", stake: "3.91M", blocks: "177,210", uptime: "99.99%" },
  { rank: 3, name: "Nyx Validators", stake: "3.44M", blocks: "156,003", uptime: "99.98%" },
  { rank: 4, name: "Hecate Node", stake: "2.87M", blocks: "129,477", uptime: "100.00%" },
  { rank: 5, name: "Cynthia Capital", stake: "2.51M", blocks: "113,290", uptime: "99.97%" },
];

function hexFromSeed(seed: number, length: number): string {
  const alphabet = "0123456789abcdef";
  return Array.from({ length }, (_, index) => {
    const cursor = (seed * 17 + index * 31 + seed * index) % alphabet.length;
    return alphabet[cursor];
  }).join("");
}

export function makeAddress(seed: number): `0x${string}` {
  return `0x${hexFromSeed(seed, 40)}`;
}

export function makeHash(seed: number): `0x${string}` {
  return `0x${hexFromSeed(seed + 1000, 64)}`;
}

export function shortHash(value: string): string {
  return `${value.slice(0, 6)}...${value.slice(-4)}`;
}

export function makeTransaction(seed: number, age: number): MockTransaction {
  const type = txTypes[seed % txTypes.length];
  const value = ((seed * 37.119 + 12.48) % 500).toFixed(3);

  return {
    hash: makeHash(seed),
    from: makeAddress(seed + 20),
    to: makeAddress(seed + 40),
    value,
    type,
    age,
  };
}

export function seedTransactions(count: number): MockTransaction[] {
  return Array.from({ length: count }, (_, index) =>
    makeTransaction(index + 1, index * 2),
  );
}

export function makeBlocks(latestBlock: number): MockBlock[] {
  return Array.from({ length: 8 }, (_, index) => ({
    number: latestBlock - index,
    hash: makeHash(latestBlock - index),
    txCount: 50 + ((latestBlock + index * 29) % 250),
    proposer: `${makeAddress(latestBlock + index).slice(0, 10)}...`,
    age: index * 0.4,
  }));
}

export function formatUsd(value: number): string {
  return value.toLocaleString(undefined, { maximumFractionDigits: 0 });
}
```

- [ ] **Step 3: Run a focused type check**

Run:

```bash
pnpm --filter @artemis/ui build
```

Expected: Build may still pass or fail on unrelated existing UI, but it must not fail because of missing exports from the two new files.

- [ ] **Step 4: Commit**

```bash
git add packages/ui/components/dapp/types.ts packages/ui/components/dapp/mockData.ts
git commit -m "feat(ui): add dapp demo data"
```

---

### Task 2: Add Dapp Visual Primitives And Live Feed

**Files:**
- Create: `packages/ui/components/dapp/DappPrimitives.tsx`
- Create: `packages/ui/components/dapp/LiveFeed.tsx`
- Modify: `packages/ui/app/globals.css`

- [ ] **Step 1: Add animation and utility CSS**

Append to `packages/ui/app/globals.css`:

```css
@keyframes artPulse {
  0% {
    opacity: 0.6;
    transform: scale(1);
  }

  100% {
    opacity: 0;
    transform: scale(2.6);
  }
}

@keyframes artFadeIn {
  from {
    opacity: 0;
    transform: translateY(-4px);
  }

  to {
    opacity: 1;
    transform: translateY(0);
  }
}

::selection {
  background: #ffb084;
  color: #0a0a0a;
}

@layer utilities {
  .art-display {
    font-family: var(--font-sans), system-ui, sans-serif;
    font-weight: 500;
    letter-spacing: 0;
  }

  .art-caption {
    font-size: 0.75rem;
    font-weight: 700;
    letter-spacing: 0.14em;
    line-height: 1.4;
    text-transform: uppercase;
  }

  .art-panel {
    border: 1px solid #e5e5e5;
    border-radius: 1.5rem;
    background: #fffaf0;
    overflow: hidden;
  }
}
```

- [ ] **Step 2: Create visual primitives**

Create `packages/ui/components/dapp/DappPrimitives.tsx`:

```tsx
import type { ComponentPropsWithoutRef, ReactElement, ReactNode } from "react";
import type { DemoToken, TxKind } from "./types";
import { cn } from "~/lib/utils";

type DivProps = ComponentPropsWithoutRef<"div">;

const txBadgeClasses: Record<TxKind, string> = {
  transfer: "bg-[#a4d4c5] text-[#0a0a0a]",
  swap: "bg-[#ff4d8b] text-white",
  mint: "bg-[#b8a4ed] text-[#0a0a0a]",
  contract: "bg-[#e8b94a] text-[#0a0a0a]",
};

const cardToneClasses = {
  cream: "bg-[#f5f0e0] text-[#0a0a0a]",
  lavender: "bg-[#b8a4ed] text-[#0a0a0a]",
  ochre: "bg-[#e8b94a] text-[#0a0a0a]",
  peach: "bg-[#ffb084] text-[#0a0a0a]",
  pink: "bg-[#ff4d8b] text-white",
  teal: "bg-[#1a3a3a] text-white",
} as const;

export type CardTone = keyof typeof cardToneClasses;

export function PulseDot({
  color = "#22c55e",
}: {
  color?: string;
}): ReactElement {
  return (
    <span className="relative inline-flex size-2">
      <span
        aria-hidden="true"
        className="absolute inset-0 rounded-full"
        style={{ animation: "artPulse 1.6s ease-out infinite", background: color }}
      />
      <span className="relative size-2 rounded-full" style={{ background: color }} />
    </span>
  );
}

export function Mono({
  children,
  className,
}: {
  children: ReactNode;
  className?: string;
}): ReactElement {
  return <span className={cn("font-mono text-[13px]", className)}>{children}</span>;
}

export function DappPanel({
  className,
  children,
  ...props
}: DivProps): ReactElement {
  return (
    <div {...props} className={cn("art-panel", className)}>
      {children}
    </div>
  );
}

export function OverviewCard({
  label,
  value,
  sub,
  tone,
}: {
  label: string;
  value: ReactNode;
  sub: string;
  tone: CardTone;
}): ReactElement {
  return (
    <div className={cn("rounded-3xl p-5", cardToneClasses[tone])}>
      <div className="art-caption mb-3 opacity-70">{label}</div>
      <div className="art-display text-[32px] leading-none">{value}</div>
      <div className="mt-2 text-xs opacity-70">{sub}</div>
    </div>
  );
}

export function TransactionTypeBadge({ type }: { type: TxKind }): ReactElement {
  return (
    <span className={cn("art-caption rounded-md px-2 py-1 text-[9px]", txBadgeClasses[type])}>
      {type}
    </span>
  );
}

export function TokenIcon({
  token,
  size = 32,
}: {
  token: DemoToken;
  size?: number;
}): ReactElement {
  return (
    <span
      className="inline-flex shrink-0 items-center justify-center rounded-full font-bold text-white"
      style={{ background: token.color, fontSize: size * 0.36, height: size, width: size }}
    >
      {token.sym.slice(0, 2)}
    </span>
  );
}
```

- [ ] **Step 3: Create live feed hook and card**

Create `packages/ui/components/dapp/LiveFeed.tsx`:

```tsx
"use client";

import { useEffect, useState, type ReactElement } from "react";
import {
  makeTransaction,
  seedTransactions,
  shortHash,
} from "./mockData";
import type { LiveFeedState } from "./types";
import { PulseDot } from "./DappPrimitives";

const initialFeed: LiveFeedState = {
  block: 18_472_913,
  gasPrice: 0,
  tps: 2847,
  txs: seedTransactions(8),
};

export function useLiveFeed(): LiveFeedState {
  const [feed, setFeed] = useState<LiveFeedState>(initialFeed);

  useEffect(() => {
    const id = window.setInterval(() => {
      setFeed((current) => {
        const nextTps = Math.max(
          1800,
          Math.min(4200, current.tps + ((current.block % 9) - 4) * 37),
        );
        const nextBlock = current.block + 1;

        return {
          ...current,
          block: nextBlock,
          tps: nextTps,
          txs: [
            makeTransaction(nextBlock, 0),
            ...current.txs.slice(0, 7).map((tx) => ({ ...tx, age: tx.age + 2 })),
          ],
        };
      });
    }, 1800);

    return () => window.clearInterval(id);
  }, []);

  return feed;
}

export function NetworkFeedCard({ feed }: { feed: LiveFeedState }): ReactElement {
  return (
    <div className="flex min-h-[420px] flex-col gap-5 rounded-3xl bg-[#1a3a3a] p-7 text-white">
      <div className="flex items-center justify-between">
        <div className="art-caption opacity-70">Network feed</div>
        <div className="flex items-center gap-2 text-xs text-[#a4d4c5]">
          <PulseDot color="#a4d4c5" />
          <span>Synced</span>
        </div>
      </div>

      <div className="grid grid-cols-2 gap-4">
        <div>
          <div className="mb-1 text-xs opacity-60">Block</div>
          <div className="font-mono text-[22px] font-medium">
            #{feed.block.toLocaleString()}
          </div>
        </div>
        <div>
          <div className="mb-1 text-xs opacity-60">Throughput</div>
          <div className="font-mono text-[22px] font-medium">
            {feed.tps.toLocaleString()} <span className="text-[13px] opacity-60">tps</span>
          </div>
        </div>
      </div>

      <div className="h-px bg-white/10" />

      <div className="flex flex-1 flex-col gap-2">
        <div className="art-caption mb-1 text-[10px] opacity-50">Recent transactions</div>
        {feed.txs.slice(0, 5).map((tx, index) => (
          <div
            key={tx.hash}
            className="grid grid-cols-[12px_minmax(0,1fr)_auto] items-center gap-2 text-xs"
            style={{
              animation: index === 0 ? "artFadeIn 600ms ease-out" : undefined,
              opacity: 1 - index * 0.12,
            }}
          >
            <span
              className="size-1.5 rounded-full"
              style={{
                background:
                  tx.type === "swap"
                    ? "#ff4d8b"
                    : tx.type === "mint"
                      ? "#b8a4ed"
                      : tx.type === "contract"
                        ? "#e8b94a"
                        : "#a4d4c5",
              }}
            />
            <span className="truncate font-mono">{shortHash(tx.hash)}</span>
            <span className="font-mono opacity-70">{tx.value} ART</span>
          </div>
        ))}
      </div>

      <div className="mt-auto flex items-center justify-between border-t border-white/10 pt-3 text-xs text-[#a4d4c5]">
        <span>Gas fee</span>
        <span className="font-mono font-medium">0.000 ART · gasless</span>
      </div>
    </div>
  );
}
```

- [ ] **Step 4: Build**

Run:

```bash
pnpm --filter @artemis/ui build
```

Expected: no TypeScript errors from `DappPrimitives.tsx` or `LiveFeed.tsx`.

- [ ] **Step 5: Commit**

```bash
git add packages/ui/app/globals.css packages/ui/components/dapp/DappPrimitives.tsx packages/ui/components/dapp/LiveFeed.tsx
git commit -m "feat(ui): add dapp visual primitives"
```

---

### Task 3: Update Layout, Header, Footer, And Wallet Pill

**Files:**
- Modify: `packages/ui/components/layout/AppLayout.tsx`
- Modify: `packages/ui/components/layout/Header.tsx`
- Modify: `packages/ui/components/layout/Footer.tsx`
- Modify: `packages/ui/components/scaffold/ConnectButtonCustom.tsx`

- [ ] **Step 1: Let pages own their own width**

Replace `packages/ui/components/layout/AppLayout.tsx` with:

```tsx
import { Footer } from "./Footer";
import { Header } from "./Header";

export function AppLayout({ children }: { children: React.ReactNode }) {
  return (
    <div className="flex min-h-screen flex-col bg-[#fffaf0]">
      <Header />
      <main className="w-full flex-1">{children}</main>
      <Footer />
    </div>
  );
}
```

- [ ] **Step 2: Restyle wallet button without changing behavior**

In `packages/ui/components/scaffold/ConnectButtonCustom.tsx`, keep existing imports and behavior, then update button class names:

```tsx
function truncateAddress(address: string): string {
  return `${address.slice(0, 6)}...${address.slice(-4)}`;
}
```

Use this trigger in `AddressInfoDropdown`:

```tsx
<MenuTrigger
  render={
    <Button
      variant="outline"
      size="sm"
      className="h-11 gap-2 rounded-full border-[#e5e5e5] bg-[#f5f0e0] pl-1.5 pr-3 font-semibold text-[#0a0a0a]"
    />
  }
>
```

Use this disconnected button:

```tsx
<Button
  size="sm"
  onClick={openConnectModal}
  className="h-11 rounded-xl bg-[#0a0a0a] px-5 font-semibold text-white hover:bg-[#1f1f1f]"
>
  Connect wallet
</Button>
```

Use this connected wrapper:

```tsx
<div className="flex items-center gap-2">
  <div className="hidden flex-col items-end sm:flex">
    <span className="font-mono text-xs text-[#0a0a0a]">
      <Balance address={account.address as AddressType} />
    </span>
    <span className="text-[10px] font-medium text-[#6a6a6a]">{chain.name}</span>
  </div>
  <AddressInfoDropdown
    address={account.address as AddressType}
    displayName={account.displayName}
  />
</div>
```

- [ ] **Step 3: Replace header visual structure**

Replace the returned JSX in `packages/ui/components/layout/Header.tsx` with this structure while preserving `navItems`, `isActivePath`, admin logic, and `ConnectButtonCustom`:

```tsx
return (
  <header className="sticky top-0 z-40 border-b border-[#f0f0f0] bg-[#fffaf0]">
    <nav className="mx-auto flex min-h-16 max-w-7xl flex-col gap-3 px-4 py-3 sm:flex-row sm:items-center sm:gap-8 sm:px-8">
      <div className="flex items-center justify-between gap-4">
        <Link href="/" className="flex items-center gap-2">
          <span className="flex size-7 items-center justify-center rounded-lg bg-[#0a0a0a] text-sm font-bold text-white">
            A
          </span>
          <span className="text-lg font-semibold tracking-normal text-[#0a0a0a]">
            Artemis
          </span>
          <span className="art-caption rounded-md bg-[#f5f0e0] px-2 py-1 text-[10px] text-[#6a6a6a]">
            Mainnet
          </span>
        </Link>
        <div className="shrink-0 sm:hidden">
          <ConnectButtonCustom />
        </div>
      </div>

      <div className="flex gap-1 overflow-x-auto sm:ml-4">
        {allNavItems.map((item) => {
          const isActive = isActivePath(pathname, item.href);

          return (
            <Link
              key={item.href}
              href={item.href}
              aria-current={isActive ? "page" : undefined}
              className={cn(
                "rounded-lg px-3.5 py-2 text-sm font-medium transition-colors",
                isActive
                  ? "bg-[#f5f0e0] text-[#0a0a0a]"
                  : "text-[#6a6a6a] hover:bg-[#f5f0e0] hover:text-[#0a0a0a]",
              )}
            >
              {item.label}
            </Link>
          );
        })}
      </div>

      <div className="ml-auto hidden items-center gap-4 sm:flex">
        <Link href="/debug" className="text-sm font-medium text-[#6a6a6a] hover:text-[#0a0a0a]">
          Docs
        </Link>
        <Link href="/transfer" className="text-sm font-medium text-[#6a6a6a] hover:text-[#0a0a0a]">
          Bridge
        </Link>
        <ConnectButtonCustom />
      </div>
    </nav>
  </header>
);
```

- [ ] **Step 4: Replace footer**

Replace `packages/ui/components/layout/Footer.tsx` with:

```tsx
const footerColumns = [
  { heading: "Network", items: ["Stats", "Validators", "Governance", "Status"] },
  { heading: "Build", items: ["Docs", "RPC endpoints", "SDK", "GitHub"] },
  { heading: "Use", items: ["Bridge", "Wallet", "Explorer", "Faucet"] },
  { heading: "Company", items: ["About", "Blog", "Brand", "Contact"] },
];

export function Footer() {
  return (
    <footer className="bg-[#faf5e8] px-4 py-10 sm:px-8 sm:py-16">
      <div className="mx-auto grid max-w-7xl gap-8 md:grid-cols-[1.5fr_repeat(4,1fr)]">
        <div>
          <div className="flex items-center gap-2">
            <span className="flex size-7 items-center justify-center rounded-lg bg-[#0a0a0a] text-sm font-bold text-white">
              A
            </span>
            <span className="text-lg font-semibold tracking-normal text-[#0a0a0a]">
              Artemis
            </span>
          </div>
          <p className="mt-3 max-w-60 text-sm text-[#6a6a6a]">
            The gasless EVM Layer 1.
          </p>
        </div>

        {footerColumns.map((column) => (
          <div key={column.heading}>
            <div className="art-caption mb-3 text-[#6a6a6a]">{column.heading}</div>
            <ul className="flex list-none flex-col gap-2 p-0 text-sm text-[#3a3a3a]">
              {column.items.map((item) => (
                <li key={item}>{item}</li>
              ))}
            </ul>
          </div>
        ))}
      </div>

      <div className="mx-auto mt-12 flex max-w-7xl flex-col gap-2 border-t border-[#e5e5e5] pt-5 text-xs text-[#6a6a6a] sm:flex-row sm:justify-between">
        <span>© 2026 Artemis Labs · chainId 322</span>
        <span className="font-mono">RPC: localhost:9944 · WS: localhost:9944</span>
      </div>
    </footer>
  );
}
```

- [ ] **Step 5: Build and commit**

Run:

```bash
pnpm --filter @artemis/ui build
```

Expected: layout/header/footer compile with no JSX or import errors.

Commit:

```bash
git add packages/ui/components/layout/AppLayout.tsx packages/ui/components/layout/Header.tsx packages/ui/components/layout/Footer.tsx packages/ui/components/scaffold/ConnectButtonCustom.tsx
git commit -m "feat(ui): restyle dapp shell"
```

---

### Task 4: Implement Home Route

**Files:**
- Modify: `packages/ui/app/page.tsx`

- [ ] **Step 1: Replace Home page with prototype structure**

Replace `packages/ui/app/page.tsx` with a client component that imports:

```tsx
"use client";

import Link from "next/link";
import { useAccount } from "wagmi";
import { NetworkFeedCard, useLiveFeed } from "~/components/dapp/LiveFeed";
import { Mono, PulseDot } from "~/components/dapp/DappPrimitives";
```

The page must render:

- Hero eyebrow `Mainnet · Live` with `PulseDot`.
- H1 `The gasless EVM chain.`
- Copy describing Artemis as EVM-compatible Layer 1 with zero gas fees.
- Primary CTA to `/transfer`, text `Open transfer` when connected and `Connect wallet` when disconnected.
- Secondary CTA to `/debug`, text `Read the docs`.
- Compatibility row: MetaMask, Hardhat, Foundry, Viem.
- `<NetworkFeedCard feed={feed} />`.
- Stats strip with `24h transactions`, `Avg. block time`, `Active addresses`, `Gas paid by users`.
- Feature grid with cards for Gasless, EVM compatible, and Sub-second finality.
- Developer band with config snippet showing `chainId: 322`.

Use these exact facts in the developer snippet:

```tsx
const code = `import { defineChain } from "viem";

export const artemis = defineChain({
  id: 322,
  name: "Artemis",
  nativeCurrency: { name: "ART", symbol: "ART", decimals: 18 },
  rpcUrls: { default: { http: ["http://127.0.0.1:9944"] } },
});`;
```

- [ ] **Step 2: Build**

Run:

```bash
pnpm --filter @artemis/ui build
```

Expected: `/` compiles and no server/client component boundary errors appear.

- [ ] **Step 3: Commit**

```bash
git add packages/ui/app/page.tsx
git commit -m "feat(ui): implement dapp home"
```

---

### Task 5: Implement Transfer Route And Token Picker

**Files:**
- Create: `packages/ui/components/dapp/TokenPicker.tsx`
- Modify: `packages/ui/app/transfer/page.tsx`

- [ ] **Step 1: Create TokenPicker component**

Create `packages/ui/components/dapp/TokenPicker.tsx`:

```tsx
"use client";

import type { ReactElement } from "react";
import type { DemoToken } from "./types";
import { TokenIcon } from "./DappPrimitives";

export function TokenPicker({
  tokens,
  selectedToken,
  open,
  onOpenChange,
  onSelect,
}: {
  tokens: DemoToken[];
  selectedToken: DemoToken;
  open: boolean;
  onOpenChange: (open: boolean) => void;
  onSelect: (token: DemoToken) => void;
}): ReactElement {
  return (
    <div>
      <button
        type="button"
        onClick={() => onOpenChange(!open)}
        className="flex h-11 items-center gap-2 rounded-full border border-[#e5e5e5] bg-[#fffaf0] py-1 pl-1.5 pr-3 font-semibold text-[#0a0a0a]"
      >
        <TokenIcon token={selectedToken} size={32} />
        <span>{selectedToken.sym}</span>
        <span className="text-xs text-[#6a6a6a]">▾</span>
      </button>

      {open && (
        <div className="mt-4 max-h-72 overflow-y-auto rounded-2xl border border-[#e5e5e5] bg-[#fffaf0] p-1.5">
          {tokens.map((token) => (
            <button
              key={token.sym}
              type="button"
              onClick={() => {
                onSelect(token);
                onOpenChange(false);
              }}
              className="flex w-full items-center gap-3 rounded-lg p-3 text-left hover:bg-[#f5f0e0]"
            >
              <TokenIcon token={token} size={32} />
              <span className="min-w-0 flex-1">
                <span className="block text-sm font-semibold">{token.sym}</span>
                <span className="block text-xs text-[#6a6a6a]">{token.name}</span>
              </span>
              <span className="font-mono text-[13px]">{token.balance.toLocaleString()}</span>
            </button>
          ))}
        </div>
      )}
    </div>
  );
}
```

- [ ] **Step 2: Replace Transfer page behavior**

Replace `packages/ui/app/transfer/page.tsx` with a client component that:

- Imports `isAddress` from `viem`.
- Imports `useAccount` from `wagmi`.
- Uses `demoTokens` and `formatUsd` from `mockData`.
- Uses `TokenPicker`, `TokenIcon`, `Mono`, and `DappPanel`.
- Holds local state for selected token, amount, recipient, picker open, sending, and success.
- Computes `valid` as connected wallet, positive amount, amount within demo balance, and valid address.
- Does not call `useTransactor` in this pixel-match scope.

Use this validation helper:

```ts
function getButtonLabel({
  amount,
  connected,
  sending,
  token,
  to,
}: {
  amount: number;
  connected: boolean;
  sending: boolean;
  token: { balance: number; sym: string; usd: number };
  to: string;
}): string {
  if (!connected) return "Connect wallet to send";
  if (sending) return "Submitting...";
  if (!to) return "Enter recipient";
  if (!isAddress(to)) return "Invalid address";
  if (!amount) return "Enter amount";
  if (amount > token.balance) return "Insufficient balance";
  return `Send ${amount.toFixed(token.usd > 100 ? 4 : 2)} ${token.sym}`;
}
```

Use this send handler:

```ts
function handleSend() {
  if (!valid) return;
  setSending(true);
  window.setTimeout(() => {
    const hash = makeHash(Date.now());
    setSuccess({ amount: numericAmount, token: selectedToken.sym, hash });
    setAmount("");
    setTo("");
    setSending(false);
  }, 900);
}
```

The JSX must include:

- Eyebrow `Transfer`.
- H2 `Send any token.` with muted `Pay nothing.`
- Amount/token card.
- Balance row with `MAX` button.
- Token picker.
- Recipient input.
- Summary rows for Network, Network fee, Estimated finality, You send.
- Primary button using the exact labels from the helper.
- Success banner with short hash.
- Portfolio side panel and asset list from `demoTokens`.

- [ ] **Step 3: Build**

Run:

```bash
pnpm --filter @artemis/ui build
```

Expected: no TypeScript errors, no missing imports, and no accidental real transaction submission.

- [ ] **Step 4: Commit**

```bash
git add packages/ui/components/dapp/TokenPicker.tsx packages/ui/app/transfer/page.tsx
git commit -m "feat(ui): implement dapp transfer"
```

---

### Task 6: Implement Explorer Route And Panels

**Files:**
- Create: `packages/ui/components/dapp/ExplorerPanels.tsx`
- Modify: `packages/ui/app/blockexplorer/page.tsx`

- [ ] **Step 1: Create ExplorerPanels component**

Create `packages/ui/components/dapp/ExplorerPanels.tsx` with exports:

```tsx
"use client";

import { useMemo, useState, type ReactElement } from "react";
import { DappPanel, Mono, OverviewCard, TransactionTypeBadge } from "./DappPrimitives";
import { makeBlocks, shortHash, validators } from "./mockData";
import type { LiveFeedState, TxKind } from "./types";

const filters: Array<TxKind | "all"> = ["all", "transfer", "swap", "mint", "contract"];

export function ExplorerPanels({ feed }: { feed: LiveFeedState }): ReactElement {
  const [search, setSearch] = useState("");
  const [filter, setFilter] = useState<TxKind | "all">("all");
  const blocks = useMemo(() => makeBlocks(feed.block), [feed.block]);
  const txs = filter === "all" ? feed.txs : feed.txs.filter((tx) => tx.type === filter);

  return (
    <>
      <div className="mb-8 flex items-center gap-2 rounded-3xl bg-[#f5f0e0] p-2">
        <span className="pl-4 text-lg text-[#6a6a6a]">⌕</span>
        <input
          type="text"
          value={search}
          onChange={(event) => setSearch(event.target.value)}
          placeholder="Search by address, tx hash, block number, or .art name"
          className="min-w-0 flex-1 bg-transparent px-1 py-3 font-mono text-sm text-[#0a0a0a] outline-none"
        />
        <button className="h-11 rounded-xl bg-[#0a0a0a] px-5 text-sm font-semibold text-white">
          Search
        </button>
      </div>

      <div className="mb-8 grid gap-4 sm:grid-cols-2 lg:grid-cols-4">
        <OverviewCard tone="lavender" label="Latest block" value={`#${feed.block.toLocaleString()}`} sub="0.4s ago" />
        <OverviewCard tone="ochre" label="TPS" value={feed.tps.toLocaleString()} sub="↑ trending up" />
        <OverviewCard tone="peach" label="Validators" value="142" sub="100% online" />
        <OverviewCard tone="cream" label="Avg gas paid" value="$0.00" sub="gasless network" />
      </div>

      <div className="grid gap-6 lg:grid-cols-[1fr_1.3fr]">
        <DappPanel>
          <PanelHeader title="Latest blocks" action="View all blocks →" />
          {blocks.map((block, index) => (
            <div key={block.number} className="grid grid-cols-[auto_minmax(0,1fr)_auto] items-center gap-3 border-t border-[#f0f0f0] px-4 py-3 first:border-t-0">
              <div className="flex size-9 items-center justify-center rounded-lg bg-[#f5f0e0] font-mono text-[11px] text-[#6a6a6a]">
                BLK
              </div>
              <div className="min-w-0">
                <div className="font-mono text-[13px] font-semibold">#{block.number.toLocaleString()}</div>
                <div className="text-xs text-[#6a6a6a]">by <Mono>{block.proposer}</Mono></div>
              </div>
              <div className="text-right">
                <div className="text-[13px] font-medium">{block.txCount} txs</div>
                <div className="text-xs text-[#6a6a6a]">{index === 0 ? "just now" : `${block.age.toFixed(1)}s ago`}</div>
              </div>
            </div>
          ))}
        </DappPanel>

        <DappPanel>
          <PanelHeader title="Latest transactions" action="View all txs →" />
          <div className="flex gap-1 border-b border-[#f0f0f0] px-4 py-3">
            {filters.map((item) => (
              <button
                key={item}
                onClick={() => setFilter(item)}
                className={filter === item ? "rounded-full bg-[#0a0a0a] px-3 py-1 text-xs font-semibold capitalize text-white" : "rounded-full px-3 py-1 text-xs font-semibold capitalize text-[#6a6a6a]"}
              >
                {item}
              </button>
            ))}
          </div>
          {txs.slice(0, 8).map((tx) => (
            <div key={tx.hash} className="grid grid-cols-[auto_minmax(0,1fr)_auto] items-center gap-3 border-t border-[#f0f0f0] px-4 py-3 first:border-t-0">
              <TransactionTypeBadge type={tx.type} />
              <div className="min-w-0">
                <Mono className="block truncate">{shortHash(tx.hash)}</Mono>
                <div className="flex gap-1 text-xs text-[#6a6a6a]">
                  <Mono className="text-[11px]">{shortHash(tx.from)}</Mono>
                  <span>→</span>
                  <Mono className="text-[11px]">{shortHash(tx.to)}</Mono>
                </div>
              </div>
              <div className="text-right">
                <Mono className="block font-semibold">{tx.value} ART</Mono>
                <div className="text-xs text-[#6a6a6a]">{tx.age === 0 ? "just now" : `${tx.age}s ago`}</div>
              </div>
            </div>
          ))}
        </DappPanel>
      </div>

      <div className="mt-8">
        <DappPanel>
          <PanelHeader title="Top validators" action="View all 142 →" />
          <div className="overflow-x-auto">
            <table className="w-full min-w-[720px] border-collapse">
              <thead>
                <tr className="border-b border-[#f0f0f0]">
                  {["Rank", "Validator", "Stake", "Blocks proposed", "Uptime", "Status"].map((heading, index) => (
                    <th key={heading} className={index >= 2 ? "art-caption px-4 py-3 text-right text-[10px] text-[#6a6a6a]" : "art-caption px-4 py-3 text-left text-[10px] text-[#6a6a6a]"}>
                      {heading}
                    </th>
                  ))}
                </tr>
              </thead>
              <tbody>
                {validators.map((validator, index) => (
                  <tr key={validator.rank} className="border-t border-[#f0f0f0] first:border-t-0">
                    <td className="px-4 py-3 font-mono text-[13px] text-[#6a6a6a]">#{validator.rank}</td>
                    <td className="px-4 py-3">
                      <div className="flex items-center gap-2">
                        <span className="size-6 rounded-full" style={{ background: ["#ff4d8b", "#1a3a3a", "#b8a4ed", "#ffb084", "#e8b94a"][index] }} />
                        <span className="text-sm font-medium">{validator.name}</span>
                      </div>
                    </td>
                    <td className="px-4 py-3 text-right font-mono text-[13px]">{validator.stake} ART</td>
                    <td className="px-4 py-3 text-right font-mono text-[13px]">{validator.blocks}</td>
                    <td className="px-4 py-3 text-right font-mono text-[13px]">{validator.uptime}</td>
                    <td className="px-4 py-3 text-right">
                      <span className="inline-flex items-center gap-1.5 rounded-full bg-[#f5f0e0] px-3 py-1 text-xs font-semibold text-[#22c55e]">
                        <span className="size-1.5 rounded-full bg-[#22c55e]" />
                        Active
                      </span>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </DappPanel>
      </div>
    </>
  );
}

function PanelHeader({
  title,
  action,
}: {
  title: string;
  action: string;
}): ReactElement {
  return (
    <div className="flex items-center justify-between border-b border-[#f0f0f0] px-5 py-4">
      <span className="text-base font-semibold text-[#0a0a0a]">{title}</span>
      <span className="text-xs text-[#6a6a6a]">{action}</span>
    </div>
  );
}
```

- [ ] **Step 2: Replace Explorer page**

Replace `packages/ui/app/blockexplorer/page.tsx` with:

```tsx
"use client";

import { ExplorerPanels } from "~/components/dapp/ExplorerPanels";
import { useLiveFeed } from "~/components/dapp/LiveFeed";

export default function BlockExplorer() {
  const feed = useLiveFeed();

  return (
    <section className="mx-auto max-w-7xl px-4 py-12 sm:px-8 sm:py-16">
      <div className="art-caption mb-4 text-[#6a6a6a]">Explorer</div>
      <h1 className="art-display mb-8 text-5xl leading-[1.05] text-[#0a0a0a] sm:text-[56px]">
        Search the chain.
      </h1>
      <ExplorerPanels feed={feed} />
    </section>
  );
}
```

- [ ] **Step 3: Build and commit**

Run:

```bash
pnpm --filter @artemis/ui build
```

Expected: explorer compiles and transaction filter type is `TxKind | "all"` with no `any`.

Commit:

```bash
git add packages/ui/components/dapp/ExplorerPanels.tsx packages/ui/app/blockexplorer/page.tsx
git commit -m "feat(ui): implement dapp explorer"
```

---

### Task 7: Final Verification And Browser Run

**Files:**
- No intended source edits unless verification reveals a build or runtime issue.

- [ ] **Step 1: Run production build**

Run:

```bash
pnpm --filter @artemis/ui build
```

Expected: `@artemis/ui` builds successfully.

- [ ] **Step 2: Start local dev server**

Run:

```bash
pnpm --filter @artemis/ui dev
```

Expected: Next.js starts on `http://localhost:3001`. Keep the session running until inspection is complete.

- [ ] **Step 3: Inspect routes**

Open:

```text
http://localhost:3001/
http://localhost:3001/transfer
http://localhost:3001/blockexplorer
```

Expected:

- Home shows hero, live feed, stats strip, feature cards, developer band.
- Transfer shows multi-asset form, portfolio side panel, and validation labels.
- Explorer shows search, overview cards, block/transaction panels, validator table.
- Header and footer appear on all routes.
- No visible `8442`; chain ID text is `322`.

- [ ] **Step 4: Run reviewer pass**

For meaningful TypeScript/UI changes, invoke `typescript-reviewer`.

Invoke `security-reviewer` only if implementation reintroduces real transaction submission or external request handling beyond the approved demo flow.

- [ ] **Step 5: Commit fixes if any**

If verification requires source fixes:

```bash
git add packages/ui
git commit -m "fix(ui): polish dapp handoff"
```

If no fixes are needed, do not create an empty commit.
