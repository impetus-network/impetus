# Packages UI Phase 1 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Create `packages/ui` — a Next.js frontend with coss UI, connected to Artemis chain, with block explorer and debug contracts features inspired by scaffold-eth-2.

**Architecture:** Fresh Next.js 15 app with coss UI primitives (Base UI + Tailwind v4). wagmi v2 + RainbowKit for wallet. Custom hooks for contract interaction. Reads chain data via viem public client.

**Tech Stack:** Next.js 15, React 19, coss UI, Tailwind CSS v4, wagmi v2, RainbowKit, viem, pnpm workspace

---

### Task 1: Scaffold Next.js project and workspace config

**Files:**
- Create: `packages/ui/package.json`
- Create: `packages/ui/next.config.ts`
- Create: `packages/ui/tsconfig.json`
- Create: `packages/ui/postcss.config.mjs`
- Create: `packages/ui/app/layout.tsx`
- Create: `packages/ui/app/page.tsx`
- Create: `packages/ui/app/globals.css`
- Modify: `pnpm-workspace.yaml`

- [ ] **Step 1: Create packages/ui directory**

```bash
mkdir -p packages/ui/app packages/ui/public
```

- [ ] **Step 2: Create package.json**

Create `packages/ui/package.json`:

```json
{
  "name": "@artemis/ui",
  "version": "0.0.0",
  "private": true,
  "scripts": {
    "dev": "next dev --port 3001",
    "build": "next build",
    "start": "next start"
  },
  "dependencies": {
    "@artemis/shared": "workspace:*",
    "@base-ui-components/react": "^1.0.0-alpha",
    "@rainbow-me/rainbowkit": "^2.2.10",
    "@tanstack/react-query": "^5.99.0",
    "next": "^15.2.0",
    "react": "^19.2.0",
    "react-dom": "^19.2.0",
    "viem": "^2.47.0",
    "wagmi": "^2.9.0"
  },
  "devDependencies": {
    "@tailwindcss/postcss": "^4.2.0",
    "@types/node": "^22.0.0",
    "@types/react": "^19.2.0",
    "@types/react-dom": "^19.2.0",
    "tailwindcss": "^4.2.0",
    "typescript": "^5.5.0"
  }
}
```

- [ ] **Step 3: Create tsconfig.json**

Create `packages/ui/tsconfig.json`:

```json
{
  "compilerOptions": {
    "target": "ES2022",
    "lib": ["dom", "dom.iterable", "esnext"],
    "allowJs": true,
    "skipLibCheck": true,
    "strict": true,
    "noEmit": true,
    "esModuleInterop": true,
    "module": "esnext",
    "moduleResolution": "bundler",
    "resolveJsonModule": true,
    "isolatedModules": true,
    "jsx": "preserve",
    "incremental": true,
    "plugins": [{ "name": "next" }],
    "paths": {
      "~/*": ["./*"]
    }
  },
  "include": ["next-env.d.ts", "**/*.ts", "**/*.tsx", ".next/types/**/*.ts"],
  "exclude": ["node_modules"]
}
```

- [ ] **Step 4: Create next.config.ts**

Create `packages/ui/next.config.ts`:

```typescript
import type { NextConfig } from "next";

const nextConfig: NextConfig = {
  reactStrictMode: true,
  transpilePackages: ["@artemis/shared"],
};

export default nextConfig;
```

- [ ] **Step 5: Create postcss.config.mjs**

Create `packages/ui/postcss.config.mjs`:

```javascript
const config = {
  plugins: {
    "@tailwindcss/postcss": {},
  },
};

export default config;
```

- [ ] **Step 6: Create globals.css (Tailwind v4)**

Create `packages/ui/app/globals.css`:

```css
@import "tailwindcss";

@theme {
  --color-background: oklch(100% 0 0);
  --color-foreground: oklch(14.5% 0 0);
  --color-muted: oklch(96.1% 0 0);
  --color-muted-foreground: oklch(55.6% 0 0);
  --color-border: oklch(91.4% 0 0);
  --color-primary: oklch(55% 0.2 260);
  --color-primary-foreground: oklch(98.5% 0 0);
  --color-destructive: oklch(55% 0.2 25);
  --color-success: oklch(55% 0.2 145);
}

body {
  font-family: system-ui, -apple-system, sans-serif;
}
```

- [ ] **Step 7: Create root layout**

Create `packages/ui/app/layout.tsx`:

```typescript
import "./globals.css";
import type { Metadata } from "next";

export const metadata: Metadata = {
  title: "Artemis Explorer",
  description: "Block explorer and contract debugger for Artemis chain",
};

export default function RootLayout({ children }: { children: React.ReactNode }) {
  return (
    <html lang="en">
      <body className="min-h-screen bg-background text-foreground">{children}</body>
    </html>
  );
}
```

- [ ] **Step 8: Create home page placeholder**

Create `packages/ui/app/page.tsx`:

```typescript
export default function Home() {
  return (
    <main className="flex min-h-screen flex-col items-center justify-center">
      <h1 className="text-4xl font-bold">Artemis Explorer</h1>
      <p className="mt-4 text-muted-foreground">Block explorer and contract debugger</p>
    </main>
  );
}
```

- [ ] **Step 9: Add to pnpm workspace**

Edit `pnpm-workspace.yaml` — add `"packages/ui"` to the packages list.

- [ ] **Step 10: Install dependencies and verify**

```bash
cd /Users/huyduan/projects/blockchain && pnpm install
cd packages/ui && pnpm build
```

Expected: Next.js builds successfully.

- [ ] **Step 11: Commit**

```bash
git add packages/ui pnpm-workspace.yaml pnpm-lock.yaml
git commit -m "feat(ui): scaffold Next.js project with Tailwind v4"
```

---

### Task 2: Configure wagmi + RainbowKit for Artemis

**Files:**
- Create: `packages/ui/config/wagmi.ts`
- Create: `packages/ui/config/chains.ts`
- Create: `packages/ui/components/providers/Web3Provider.tsx`
- Modify: `packages/ui/app/layout.tsx`
- Create: `packages/ui/.env.local`

- [ ] **Step 1: Create chain definition**

Create `packages/ui/config/chains.ts`:

```typescript
import { defineChain } from "viem";

export const artemis = defineChain({
  id: 322,
  name: "Artemis",
  nativeCurrency: { name: "Artemis Token", symbol: "ART", decimals: 18 },
  rpcUrls: {
    default: { http: ["http://127.0.0.1:9944"] },
  },
});
```

- [ ] **Step 2: Create wagmi config**

Create `packages/ui/config/wagmi.ts`:

```typescript
"use client";

import { getDefaultConfig } from "@rainbow-me/rainbowkit";
import { artemis } from "./chains";

export const wagmiConfig = getDefaultConfig({
  appName: "Artemis Explorer",
  projectId: process.env.NEXT_PUBLIC_WALLET_CONNECT_PROJECT_ID || "3a8170812b534d0ff9d794f19a901d64",
  chains: [artemis],
  ssr: true,
});
```

- [ ] **Step 3: Create Web3Provider**

Create `packages/ui/components/providers/Web3Provider.tsx`:

```typescript
"use client";

import { RainbowKitProvider } from "@rainbow-me/rainbowkit";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { WagmiProvider } from "wagmi";
import { wagmiConfig } from "~/config/wagmi";
import "@rainbow-me/rainbowkit/styles.css";

const queryClient = new QueryClient();

export function Web3Provider({ children }: { children: React.ReactNode }) {
  return (
    <WagmiProvider config={wagmiConfig}>
      <QueryClientProvider client={queryClient}>
        <RainbowKitProvider>{children}</RainbowKitProvider>
      </QueryClientProvider>
    </WagmiProvider>
  );
}
```

- [ ] **Step 4: Create .env.local**

Create `packages/ui/.env.local`:

```
NEXT_PUBLIC_WALLET_CONNECT_PROJECT_ID=3a8170812b534d0ff9d794f19a901d64
```

- [ ] **Step 5: Update root layout with provider**

Replace `packages/ui/app/layout.tsx`:

```typescript
import "./globals.css";
import type { Metadata } from "next";
import { Web3Provider } from "~/components/providers/Web3Provider";

export const metadata: Metadata = {
  title: "Artemis Explorer",
  description: "Block explorer and contract debugger for Artemis chain",
};

export default function RootLayout({ children }: { children: React.ReactNode }) {
  return (
    <html lang="en">
      <body className="min-h-screen bg-background text-foreground">
        <Web3Provider>{children}</Web3Provider>
      </body>
    </html>
  );
}
```

- [ ] **Step 6: Build and verify**

```bash
cd packages/ui && pnpm build
```

- [ ] **Step 7: Commit**

```bash
git add packages/ui/config packages/ui/components/providers packages/ui/app/layout.tsx packages/ui/.env.local
git commit -m "feat(ui): configure wagmi and RainbowKit for Artemis chain"
```

---

### Task 3: Core layout — Header and Footer with coss

**Files:**
- Create: `packages/ui/components/layout/Header.tsx`
- Create: `packages/ui/components/layout/Footer.tsx`
- Create: `packages/ui/components/layout/AppLayout.tsx`
- Modify: `packages/ui/app/layout.tsx`
- Modify: `packages/ui/app/page.tsx`

- [ ] **Step 1: Install coss button and navigation primitives**

```bash
cd packages/ui && npx coss add button tabs badge
```

If `coss` CLI is not available, manually create the components using Base UI. Alternatively install:
```bash
pnpm add @heroicons/react
```

- [ ] **Step 2: Create Header**

Create `packages/ui/components/layout/Header.tsx`:

```typescript
"use client";

import Link from "next/link";
import { usePathname } from "next/navigation";
import { ConnectButton } from "@rainbow-me/rainbowkit";

const navItems = [
  { href: "/", label: "Home" },
  { href: "/blockexplorer", label: "Explorer" },
  { href: "/debug", label: "Debug" },
];

export function Header() {
  const pathname = usePathname();

  return (
    <header className="border-b border-border bg-background">
      <nav className="mx-auto flex max-w-7xl items-center justify-between px-4 py-3">
        <div className="flex items-center gap-6">
          <Link href="/" className="text-xl font-bold text-primary">
            Artemis
          </Link>
          <div className="flex gap-4">
            {navItems.map((item) => (
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
        <ConnectButton />
      </nav>
    </header>
  );
}
```

- [ ] **Step 3: Create Footer**

Create `packages/ui/components/layout/Footer.tsx`:

```typescript
export function Footer() {
  return (
    <footer className="border-t border-border py-6">
      <div className="mx-auto max-w-7xl px-4 text-center text-sm text-muted-foreground">
        <p>Artemis Chain — ID 322 — ART Token</p>
      </div>
    </footer>
  );
}
```

- [ ] **Step 4: Create AppLayout**

Create `packages/ui/components/layout/AppLayout.tsx`:

```typescript
import { Header } from "./Header";
import { Footer } from "./Footer";

export function AppLayout({ children }: { children: React.ReactNode }) {
  return (
    <div className="flex min-h-screen flex-col">
      <Header />
      <main className="mx-auto w-full max-w-7xl flex-1 px-4 py-8">{children}</main>
      <Footer />
    </div>
  );
}
```

- [ ] **Step 5: Update root layout**

Replace `packages/ui/app/layout.tsx`:

```typescript
import "./globals.css";
import type { Metadata } from "next";
import { Web3Provider } from "~/components/providers/Web3Provider";
import { AppLayout } from "~/components/layout/AppLayout";

export const metadata: Metadata = {
  title: "Artemis Explorer",
  description: "Block explorer and contract debugger for Artemis chain",
};

export default function RootLayout({ children }: { children: React.ReactNode }) {
  return (
    <html lang="en">
      <body className="min-h-screen bg-background text-foreground">
        <Web3Provider>
          <AppLayout>{children}</AppLayout>
        </Web3Provider>
      </body>
    </html>
  );
}
```

- [ ] **Step 6: Update home page**

Replace `packages/ui/app/page.tsx`:

```typescript
"use client";

import { useAccount, useBalance, useBlockNumber } from "wagmi";
import { formatEther } from "viem";

export default function Home() {
  const { address, isConnected } = useAccount();
  const { data: balance } = useBalance({ address, query: { enabled: isConnected } });
  const { data: blockNumber } = useBlockNumber({ watch: true });

  return (
    <div className="flex flex-col gap-8">
      <section className="text-center">
        <h1 className="text-4xl font-bold">Artemis Explorer</h1>
        <p className="mt-2 text-muted-foreground">
          Block explorer and contract debugger for Artemis chain (ID 322)
        </p>
      </section>

      <section className="grid gap-4 sm:grid-cols-3">
        <div className="rounded-lg border border-border p-4">
          <p className="text-sm text-muted-foreground">Latest Block</p>
          <p className="text-2xl font-bold">{blockNumber?.toString() ?? "—"}</p>
        </div>
        <div className="rounded-lg border border-border p-4">
          <p className="text-sm text-muted-foreground">Connected</p>
          <p className="text-2xl font-bold">{isConnected ? "Yes" : "No"}</p>
        </div>
        <div className="rounded-lg border border-border p-4">
          <p className="text-sm text-muted-foreground">Balance</p>
          <p className="text-2xl font-bold">
            {balance ? `${Number(formatEther(balance.value)).toFixed(2)} ART` : "—"}
          </p>
        </div>
      </section>
    </div>
  );
}
```

- [ ] **Step 7: Build and verify**

```bash
cd packages/ui && pnpm build
```

- [ ] **Step 8: Commit**

```bash
git add packages/ui/components/layout packages/ui/app
git commit -m "feat(ui): add header, footer, home page with chain stats"
```

---

### Task 4: Utility components — Address and Balance

**Files:**
- Create: `packages/ui/components/scaffold/Address.tsx`
- Create: `packages/ui/components/scaffold/Balance.tsx`
- Create: `packages/ui/components/scaffold/BlockieAvatar.tsx`
- Create: `packages/ui/hooks/useCopyToClipboard.ts`

- [ ] **Step 1: Create copy hook**

Create `packages/ui/hooks/useCopyToClipboard.ts`:

```typescript
"use client";

import { useState } from "react";

export function useCopyToClipboard(timeout = 2000) {
  const [copied, setCopied] = useState(false);

  async function copy(text: string) {
    await navigator.clipboard.writeText(text);
    setCopied(true);
    setTimeout(() => setCopied(false), timeout);
  }

  return { copied, copy };
}
```

- [ ] **Step 2: Create BlockieAvatar**

Create `packages/ui/components/scaffold/BlockieAvatar.tsx`:

```typescript
"use client";

interface BlockieAvatarProps {
  address: string;
  size?: number;
}

export function BlockieAvatar({ address, size = 24 }: BlockieAvatarProps) {
  // Simple gradient avatar based on address
  const hue = parseInt(address.slice(2, 8), 16) % 360;
  return (
    <div
      className="rounded-full"
      style={{
        width: size,
        height: size,
        background: `linear-gradient(135deg, hsl(${hue}, 70%, 60%), hsl(${(hue + 60) % 360}, 70%, 40%))`,
      }}
    />
  );
}
```

- [ ] **Step 3: Create Address component**

Create `packages/ui/components/scaffold/Address.tsx`:

```typescript
"use client";

import { BlockieAvatar } from "./BlockieAvatar";
import { useCopyToClipboard } from "~/hooks/useCopyToClipboard";

interface AddressProps {
  address: string;
  format?: "short" | "full";
}

export function Address({ address, format = "short" }: AddressProps) {
  const { copied, copy } = useCopyToClipboard();
  const display = format === "short" ? `${address.slice(0, 6)}...${address.slice(-4)}` : address;

  return (
    <button
      onClick={() => copy(address)}
      className="inline-flex items-center gap-1.5 rounded px-1 py-0.5 font-mono text-sm hover:bg-muted"
      title={copied ? "Copied!" : "Click to copy"}
    >
      <BlockieAvatar address={address} size={18} />
      <span>{display}</span>
      {copied && <span className="text-xs text-success">✓</span>}
    </button>
  );
}
```

- [ ] **Step 4: Create Balance component**

Create `packages/ui/components/scaffold/Balance.tsx`:

```typescript
"use client";

import { useBalance } from "wagmi";
import { formatEther } from "viem";
import type { Address as AddressType } from "viem";

interface BalanceProps {
  address: AddressType;
}

export function Balance({ address }: BalanceProps) {
  const { data, isLoading } = useBalance({ address });

  if (isLoading) return <span className="text-muted-foreground">...</span>;
  if (!data) return <span className="text-muted-foreground">—</span>;

  return (
    <span className="font-mono">
      {Number(formatEther(data.value)).toFixed(4)} {data.symbol}
    </span>
  );
}
```

- [ ] **Step 5: Build and verify**

```bash
cd packages/ui && pnpm build
```

- [ ] **Step 6: Commit**

```bash
git add packages/ui/components/scaffold packages/ui/hooks
git commit -m "feat(ui): add Address, Balance, BlockieAvatar components"
```

---

### Task 5: Block Explorer page

**Files:**
- Create: `packages/ui/app/blockexplorer/page.tsx`
- Create: `packages/ui/hooks/useFetchBlocks.ts`
- Create: `packages/ui/components/blockexplorer/TransactionsTable.tsx`
- Create: `packages/ui/components/blockexplorer/SearchBar.tsx`

- [ ] **Step 1: Create block fetching hook**

Create `packages/ui/hooks/useFetchBlocks.ts`:

```typescript
"use client";

import { useEffect, useState } from "react";
import { usePublicClient, useBlockNumber } from "wagmi";
import type { Block, Transaction } from "viem";

export function useFetchBlocks(count = 10) {
  const publicClient = usePublicClient();
  const { data: latestBlock } = useBlockNumber({ watch: true });
  const [blocks, setBlocks] = useState<Block[]>([]);
  const [transactions, setTransactions] = useState<Transaction[]>([]);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    if (!publicClient || latestBlock === undefined) return;

    async function fetchBlocks() {
      setLoading(true);
      const blockNumbers = Array.from(
        { length: Math.min(count, Number(latestBlock!) + 1) },
        (_, i) => latestBlock! - BigInt(i),
      );

      const fetchedBlocks = await Promise.all(
        blockNumbers.map((n) => publicClient!.getBlock({ blockNumber: n, includeTransactions: true })),
      );

      setBlocks(fetchedBlocks);

      const allTxs = fetchedBlocks.flatMap((b) => b.transactions as unknown as Transaction[]);
      setTransactions(allTxs.slice(0, 20));
      setLoading(false);
    }

    fetchBlocks();
  }, [publicClient, latestBlock, count]);

  return { blocks, transactions, loading, latestBlock };
}
```

- [ ] **Step 2: Create SearchBar**

Create `packages/ui/components/blockexplorer/SearchBar.tsx`:

```typescript
"use client";

import { type FormEvent, useState } from "react";
import { useRouter } from "next/navigation";
import { isAddress, isHex } from "viem";

export function SearchBar() {
  const [query, setQuery] = useState("");
  const router = useRouter();

  function handleSearch(e: FormEvent) {
    e.preventDefault();
    const trimmed = query.trim();
    if (isAddress(trimmed)) {
      router.push(`/blockexplorer/address/${trimmed}`);
    } else if (isHex(trimmed) && trimmed.length === 66) {
      router.push(`/blockexplorer/tx/${trimmed}`);
    }
    setQuery("");
  }

  return (
    <form onSubmit={handleSearch} className="flex gap-2">
      <input
        type="text"
        value={query}
        onChange={(e) => setQuery(e.target.value)}
        placeholder="Search by address or tx hash..."
        className="flex-1 rounded-lg border border-border bg-background px-4 py-2 text-sm focus:border-primary focus:outline-none"
      />
      <button
        type="submit"
        className="rounded-lg bg-primary px-4 py-2 text-sm font-medium text-primary-foreground hover:opacity-90"
      >
        Search
      </button>
    </form>
  );
}
```

- [ ] **Step 3: Create TransactionsTable**

Create `packages/ui/components/blockexplorer/TransactionsTable.tsx`:

```typescript
"use client";

import type { Transaction } from "viem";
import { formatEther } from "viem";
import { Address } from "~/components/scaffold/Address";

interface TransactionsTableProps {
  transactions: Transaction[];
  loading: boolean;
}

export function TransactionsTable({ transactions, loading }: TransactionsTableProps) {
  if (loading) return <p className="text-muted-foreground">Loading transactions...</p>;
  if (transactions.length === 0) return <p className="text-muted-foreground">No transactions found.</p>;

  return (
    <div className="overflow-x-auto rounded-lg border border-border">
      <table className="w-full text-sm">
        <thead>
          <tr className="border-b border-border bg-muted text-left">
            <th className="px-4 py-3">Tx Hash</th>
            <th className="px-4 py-3">From</th>
            <th className="px-4 py-3">To</th>
            <th className="px-4 py-3">Value</th>
          </tr>
        </thead>
        <tbody>
          {transactions.map((tx) => (
            <tr key={tx.hash} className="border-b border-border hover:bg-muted/50">
              <td className="px-4 py-3 font-mono text-xs">
                {tx.hash.slice(0, 10)}...{tx.hash.slice(-6)}
              </td>
              <td className="px-4 py-3">
                <Address address={tx.from} />
              </td>
              <td className="px-4 py-3">
                {tx.to ? <Address address={tx.to} /> : <span className="text-muted-foreground">Contract Create</span>}
              </td>
              <td className="px-4 py-3 font-mono">
                {Number(formatEther(tx.value)).toFixed(4)} ART
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}
```

- [ ] **Step 4: Create block explorer page**

Create `packages/ui/app/blockexplorer/page.tsx`:

```typescript
"use client";

import { useFetchBlocks } from "~/hooks/useFetchBlocks";
import { SearchBar } from "~/components/blockexplorer/SearchBar";
import { TransactionsTable } from "~/components/blockexplorer/TransactionsTable";

export default function BlockExplorer() {
  const { blocks, transactions, loading, latestBlock } = useFetchBlocks(10);

  return (
    <div className="flex flex-col gap-6">
      <div className="flex items-center justify-between">
        <h1 className="text-2xl font-bold">Block Explorer</h1>
        <span className="text-sm text-muted-foreground">
          Latest block: #{latestBlock?.toString() ?? "—"}
        </span>
      </div>

      <SearchBar />

      <section>
        <h2 className="mb-3 text-lg font-medium">Recent Blocks</h2>
        <div className="grid gap-2 sm:grid-cols-5">
          {blocks.slice(0, 5).map((block) => (
            <div key={block.number?.toString()} className="rounded-lg border border-border p-3">
              <p className="text-xs text-muted-foreground">Block</p>
              <p className="font-mono font-bold">#{block.number?.toString()}</p>
              <p className="text-xs text-muted-foreground">
                {block.transactions.length} txs
              </p>
            </div>
          ))}
        </div>
      </section>

      <section>
        <h2 className="mb-3 text-lg font-medium">Recent Transactions</h2>
        <TransactionsTable transactions={transactions} loading={loading} />
      </section>
    </div>
  );
}
```

- [ ] **Step 5: Build and verify**

```bash
cd packages/ui && pnpm build
```

- [ ] **Step 6: Commit**

```bash
git add packages/ui/app/blockexplorer packages/ui/hooks/useFetchBlocks.ts packages/ui/components/blockexplorer
git commit -m "feat(ui): add block explorer with transactions table"
```

---

### Task 6: Debug Contracts page

**Files:**
- Create: `packages/ui/app/debug/page.tsx`
- Create: `packages/ui/components/debug/ContractUI.tsx`
- Create: `packages/ui/components/debug/ReadMethods.tsx`
- Create: `packages/ui/components/debug/WriteMethods.tsx`
- Create: `packages/ui/config/contracts.ts`

- [ ] **Step 1: Create contracts config**

This defines the contracts available for debugging. For now, hardcode the gasless registry precompile. Later tasks can auto-generate from deployments.

Create `packages/ui/config/contracts.ts`:

```typescript
import { GaslessRegistryAbi, GASLESS_REGISTRY_ADDRESS } from "@artemis/shared";

export interface ContractConfig {
  name: string;
  address: `0x${string}`;
  abi: readonly unknown[];
}

export const debugContracts: ContractConfig[] = [
  {
    name: "GaslessRegistry",
    address: GASLESS_REGISTRY_ADDRESS,
    abi: GaslessRegistryAbi,
  },
];
```

- [ ] **Step 2: Create ReadMethods component**

Create `packages/ui/components/debug/ReadMethods.tsx`:

```typescript
"use client";

import { useState } from "react";
import { useReadContract } from "wagmi";
import type { Abi, AbiFunction } from "viem";

interface ReadMethodsProps {
  address: `0x${string}`;
  abi: readonly unknown[];
}

function ReadMethod({ address, abi, fn }: { address: `0x${string}`; abi: readonly unknown[]; fn: AbiFunction }) {
  const [args, setArgs] = useState<string[]>(fn.inputs.map(() => ""));
  const [enabled, setEnabled] = useState(false);

  const { data, isLoading, error } = useReadContract({
    address,
    abi: abi as Abi,
    functionName: fn.name,
    args: args.map((a) => a || undefined),
    query: { enabled },
  });

  return (
    <div className="rounded-lg border border-border p-4">
      <h4 className="font-mono text-sm font-medium">{fn.name}</h4>
      {fn.inputs.length > 0 && (
        <div className="mt-2 flex flex-col gap-2">
          {fn.inputs.map((input, i) => (
            <input
              key={i}
              type="text"
              placeholder={`${input.name || `arg${i}`} (${input.type})`}
              value={args[i]}
              onChange={(e) => {
                const next = [...args];
                next[i] = e.target.value;
                setArgs(next);
              }}
              className="rounded border border-border bg-background px-3 py-1.5 font-mono text-xs"
            />
          ))}
        </div>
      )}
      <button
        onClick={() => setEnabled(true)}
        className="mt-2 rounded bg-primary px-3 py-1 text-xs font-medium text-primary-foreground hover:opacity-90"
      >
        Read
      </button>
      {isLoading && <p className="mt-2 text-xs text-muted-foreground">Loading...</p>}
      {error && <p className="mt-2 text-xs text-destructive">{error.message.slice(0, 100)}</p>}
      {data !== undefined && (
        <pre className="mt-2 overflow-x-auto rounded bg-muted p-2 text-xs">
          {JSON.stringify(data, (_, v) => (typeof v === "bigint" ? v.toString() : v), 2)}
        </pre>
      )}
    </div>
  );
}

export function ReadMethods({ address, abi }: ReadMethodsProps) {
  const readFns = (abi as AbiFunction[]).filter(
    (item) => item.type === "function" && (item.stateMutability === "view" || item.stateMutability === "pure"),
  );

  if (readFns.length === 0) return <p className="text-sm text-muted-foreground">No read methods.</p>;

  return (
    <div className="flex flex-col gap-3">
      {readFns.map((fn) => (
        <ReadMethod key={fn.name} address={address} abi={abi} fn={fn} />
      ))}
    </div>
  );
}
```

- [ ] **Step 3: Create WriteMethods component**

Create `packages/ui/components/debug/WriteMethods.tsx`:

```typescript
"use client";

import { useState } from "react";
import { useWriteContract, useWaitForTransactionReceipt } from "wagmi";
import type { Abi, AbiFunction } from "viem";

interface WriteMethodsProps {
  address: `0x${string}`;
  abi: readonly unknown[];
}

function WriteMethod({ address, abi, fn }: { address: `0x${string}`; abi: readonly unknown[]; fn: AbiFunction }) {
  const [args, setArgs] = useState<string[]>(fn.inputs.map(() => ""));
  const { writeContract, data: hash, isPending, error } = useWriteContract();
  const { isLoading: isConfirming, isSuccess } = useWaitForTransactionReceipt({ hash });

  function handleSubmit() {
    writeContract({
      address,
      abi: abi as Abi,
      functionName: fn.name,
      args: args.map((a) => a || undefined),
    });
  }

  return (
    <div className="rounded-lg border border-border p-4">
      <h4 className="font-mono text-sm font-medium">{fn.name}</h4>
      {fn.inputs.length > 0 && (
        <div className="mt-2 flex flex-col gap-2">
          {fn.inputs.map((input, i) => (
            <input
              key={i}
              type="text"
              placeholder={`${input.name || `arg${i}`} (${input.type})`}
              value={args[i]}
              onChange={(e) => {
                const next = [...args];
                next[i] = e.target.value;
                setArgs(next);
              }}
              className="rounded border border-border bg-background px-3 py-1.5 font-mono text-xs"
            />
          ))}
        </div>
      )}
      <button
        onClick={handleSubmit}
        disabled={isPending || isConfirming}
        className="mt-2 rounded bg-primary px-3 py-1 text-xs font-medium text-primary-foreground hover:opacity-90 disabled:opacity-50"
      >
        {isPending ? "Confirm..." : isConfirming ? "Waiting..." : "Write"}
      </button>
      {error && <p className="mt-2 text-xs text-destructive">{error.message.slice(0, 100)}</p>}
      {isSuccess && <p className="mt-2 text-xs text-success">Transaction confirmed!</p>}
      {hash && <p className="mt-1 font-mono text-xs text-muted-foreground">{hash}</p>}
    </div>
  );
}

export function WriteMethods({ address, abi }: WriteMethodsProps) {
  const writeFns = (abi as AbiFunction[]).filter(
    (item) => item.type === "function" && item.stateMutability === "nonpayable",
  );

  if (writeFns.length === 0) return <p className="text-sm text-muted-foreground">No write methods.</p>;

  return (
    <div className="flex flex-col gap-3">
      {writeFns.map((fn) => (
        <WriteMethod key={fn.name} address={address} abi={abi} fn={fn} />
      ))}
    </div>
  );
}
```

- [ ] **Step 4: Create ContractUI**

Create `packages/ui/components/debug/ContractUI.tsx`:

```typescript
"use client";

import { useState } from "react";
import type { ContractConfig } from "~/config/contracts";
import { ReadMethods } from "./ReadMethods";
import { WriteMethods } from "./WriteMethods";

interface ContractUIProps {
  contract: ContractConfig;
}

export function ContractUI({ contract }: ContractUIProps) {
  const [tab, setTab] = useState<"read" | "write">("read");

  return (
    <div className="rounded-lg border border-border">
      <div className="flex items-center justify-between border-b border-border px-4 py-3">
        <div>
          <h3 className="font-medium">{contract.name}</h3>
          <p className="font-mono text-xs text-muted-foreground">{contract.address}</p>
        </div>
        <div className="flex gap-1 rounded-lg bg-muted p-1">
          <button
            onClick={() => setTab("read")}
            className={`rounded-md px-3 py-1 text-xs font-medium transition-colors ${
              tab === "read" ? "bg-background shadow-sm" : "text-muted-foreground"
            }`}
          >
            Read
          </button>
          <button
            onClick={() => setTab("write")}
            className={`rounded-md px-3 py-1 text-xs font-medium transition-colors ${
              tab === "write" ? "bg-background shadow-sm" : "text-muted-foreground"
            }`}
          >
            Write
          </button>
        </div>
      </div>
      <div className="p-4">
        {tab === "read" && <ReadMethods address={contract.address} abi={contract.abi} />}
        {tab === "write" && <WriteMethods address={contract.address} abi={contract.abi} />}
      </div>
    </div>
  );
}
```

- [ ] **Step 5: Create debug page**

Create `packages/ui/app/debug/page.tsx`:

```typescript
"use client";

import { debugContracts } from "~/config/contracts";
import { ContractUI } from "~/components/debug/ContractUI";

export default function DebugContracts() {
  return (
    <div className="flex flex-col gap-6">
      <h1 className="text-2xl font-bold">Debug Contracts</h1>
      <p className="text-muted-foreground">
        Interact with deployed contracts on Artemis chain.
      </p>
      {debugContracts.length === 0 ? (
        <p className="text-muted-foreground">No contracts configured.</p>
      ) : (
        <div className="flex flex-col gap-6">
          {debugContracts.map((contract) => (
            <ContractUI key={contract.address} contract={contract} />
          ))}
        </div>
      )}
    </div>
  );
}
```

- [ ] **Step 6: Build and verify**

```bash
cd packages/ui && pnpm build
```

- [ ] **Step 7: Commit**

```bash
git add packages/ui/app/debug packages/ui/components/debug packages/ui/config/contracts.ts
git commit -m "feat(ui): add debug contracts page with read/write methods"
```

---

### Task 7: Block Explorer detail pages

**Files:**
- Create: `packages/ui/app/blockexplorer/address/[address]/page.tsx`
- Create: `packages/ui/app/blockexplorer/tx/[hash]/page.tsx`

- [ ] **Step 1: Create address detail page**

Create `packages/ui/app/blockexplorer/address/[address]/page.tsx`:

```typescript
"use client";

import { useParams } from "next/navigation";
import { useBalance } from "wagmi";
import { type Address as AddressType } from "viem";
import { Address } from "~/components/scaffold/Address";
import { Balance } from "~/components/scaffold/Balance";

export default function AddressPage() {
  const { address } = useParams<{ address: string }>();
  const addr = address as AddressType;

  return (
    <div className="flex flex-col gap-6">
      <h1 className="text-2xl font-bold">Address</h1>
      <div className="rounded-lg border border-border p-6">
        <dl className="grid gap-4">
          <div>
            <dt className="text-sm text-muted-foreground">Address</dt>
            <dd><Address address={addr} format="full" /></dd>
          </div>
          <div>
            <dt className="text-sm text-muted-foreground">Balance</dt>
            <dd className="text-xl font-bold"><Balance address={addr} /></dd>
          </div>
        </dl>
      </div>
    </div>
  );
}
```

- [ ] **Step 2: Create transaction detail page**

Create `packages/ui/app/blockexplorer/tx/[hash]/page.tsx`:

```typescript
"use client";

import { useParams } from "next/navigation";
import { useTransaction, useTransactionReceipt } from "wagmi";
import { formatEther, type Hash } from "viem";
import { Address } from "~/components/scaffold/Address";

export default function TxPage() {
  const { hash } = useParams<{ hash: string }>();
  const { data: tx, isLoading } = useTransaction({ hash: hash as Hash });
  const { data: receipt } = useTransactionReceipt({ hash: hash as Hash });

  if (isLoading) return <p className="text-muted-foreground">Loading...</p>;
  if (!tx) return <p className="text-muted-foreground">Transaction not found.</p>;

  return (
    <div className="flex flex-col gap-6">
      <h1 className="text-2xl font-bold">Transaction</h1>
      <div className="rounded-lg border border-border p-6">
        <dl className="grid gap-4">
          <div>
            <dt className="text-sm text-muted-foreground">Hash</dt>
            <dd className="break-all font-mono text-sm">{tx.hash}</dd>
          </div>
          <div>
            <dt className="text-sm text-muted-foreground">Status</dt>
            <dd>
              {receipt?.status === "success" ? (
                <span className="text-success font-medium">Success</span>
              ) : receipt?.status === "reverted" ? (
                <span className="text-destructive font-medium">Reverted</span>
              ) : (
                <span className="text-muted-foreground">Pending</span>
              )}
            </dd>
          </div>
          <div>
            <dt className="text-sm text-muted-foreground">From</dt>
            <dd><Address address={tx.from} /></dd>
          </div>
          <div>
            <dt className="text-sm text-muted-foreground">To</dt>
            <dd>{tx.to ? <Address address={tx.to} /> : "Contract Creation"}</dd>
          </div>
          <div>
            <dt className="text-sm text-muted-foreground">Value</dt>
            <dd className="font-mono">{formatEther(tx.value)} ART</dd>
          </div>
          <div>
            <dt className="text-sm text-muted-foreground">Gas Used</dt>
            <dd className="font-mono">{receipt?.gasUsed?.toString() ?? "—"}</dd>
          </div>
          <div>
            <dt className="text-sm text-muted-foreground">Block</dt>
            <dd className="font-mono">{tx.blockNumber?.toString()}</dd>
          </div>
        </dl>
      </div>
    </div>
  );
}
```

- [ ] **Step 3: Build and verify**

```bash
cd packages/ui && pnpm build
```

- [ ] **Step 4: Commit**

```bash
git add packages/ui/app/blockexplorer
git commit -m "feat(ui): add address and transaction detail pages"
```

---

### Task 8: Final integration and turbo config

**Files:**
- Modify: `turbo.json`
- Create: `packages/ui/.gitignore`

- [ ] **Step 1: Create .gitignore**

Create `packages/ui/.gitignore`:

```
.next/
node_modules/
.env.local
```

- [ ] **Step 2: Update turbo.json**

Add `packages/ui` to the build pipeline. Read `turbo.json` first, then add the `@artemis/ui#dev` task if not present. The existing `dev` task config (`cache: false, persistent: true`) already covers it.

- [ ] **Step 3: Final build test**

```bash
cd /Users/huyduan/projects/blockchain && pnpm turbo build
```

All packages must build successfully.

- [ ] **Step 4: Commit**

```bash
git add packages/ui/.gitignore turbo.json
git commit -m "chore(ui): add gitignore and turbo integration"
```
