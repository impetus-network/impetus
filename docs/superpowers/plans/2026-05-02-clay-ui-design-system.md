# Clay UI Design System Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Restyle the entire Artemis UI with a strict Clay-inspired design system while preserving all wallet, contract, indexer, routing, and validation behavior.

**Architecture:** Add a small `components/clay` presentation layer that composes existing `components/ui` primitives without editing them. Map `DESIGN.md` into global Tailwind v4 tokens, then migrate pages and local feature components to Clay wrappers in small commits.

**Tech Stack:** Next.js 15, React 19, TypeScript strict, Tailwind v4, Base UI wrappers, wagmi, RainbowKit, Ponder GraphQL.

---

## File Structure

Create:

- `packages/ui/components/clay/index.ts` exports the Clay layer.
- `packages/ui/components/clay/layout.tsx` owns page, hero, section, panel, and feature-card shells.
- `packages/ui/components/clay/controls.tsx` owns Clay button and badge wrappers.
- `packages/ui/components/clay/data.tsx` owns table and empty-state wrappers.

Modify:

- `packages/ui/app/globals.css` maps Clay tokens and utility classes.
- `packages/ui/components/layout/Header.tsx` uses Clay navigation styling.
- `packages/ui/components/layout/Footer.tsx` uses Clay footer styling.
- `packages/ui/app/page.tsx` adopts Clay hero and stat cards.
- `packages/ui/app/transfer/page.tsx` adopts Clay form surface.
- `packages/ui/app/blockexplorer/page.tsx` adopts Clay block tiles and table frame.
- `packages/ui/app/debug/page.tsx` adopts Clay page/section shells.
- `packages/ui/components/debug/ContractUI.tsx` adopts Clay panel and segmented control.
- `packages/ui/app/admin/gasless/page.tsx` adopts Clay admin shell.
- `packages/ui/components/admin/RulesTable.tsx` adopts Clay table/empty state.
- `packages/ui/components/admin/AddRuleForm.tsx` adopts Clay card treatment.
- `packages/ui/components/admin/CheckGaslessForm.tsx` adopts Clay card treatment.
- `packages/ui/components/blockexplorer/TransactionsTable.tsx` adopts Clay table frame and empty state.

Do not modify:

- `packages/ui/components/ui/*`

## Task 1: Global Clay Tokens

**Files:**
- Modify: `packages/ui/app/globals.css`

- [ ] **Step 1: Update global token values**

Replace the first `@theme` block and `:root` values with Clay-aligned tokens while keeping existing token names:

```css
@theme {
  --color-background: #fffaf0;
  --color-foreground: #0a0a0a;
  --color-muted: #f5f0e0;
  --color-muted-foreground: #6a6a6a;
  --color-border: #e5dcc8;
  --color-primary: #0a0a0a;
  --color-primary-foreground: #ffffff;
  --color-destructive: #ff6b5a;
  --color-success: #22c55e;
}
```

Set the `:root` semantic tokens to this exact palette:

```css
:root {
  --destructive-foreground: #8f1d14;
  --info: #1a3a3a;
  --info-foreground: #1a3a3a;
  --success: #22c55e;
  --success-foreground: #166534;
  --warning: #e8b94a;
  --warning-foreground: #7a4f00;
  --accent: #f5f0e0;
  --accent-foreground: #0a0a0a;
  --background: #fffaf0;
  --border: #e5dcc8;
  --card: #fffaf0;
  --card-foreground: #0a0a0a;
  --destructive: #ff6b5a;
  --foreground: #0a0a0a;
  --input: #d9cfb8;
  --muted: #f5f0e0;
  --muted-foreground: #6a6a6a;
  --popover: #fffaf0;
  --popover-foreground: #0a0a0a;
  --primary: #0a0a0a;
  --primary-foreground: #ffffff;
  --ring: #1a3a3a;
  --secondary: #f5f0e0;
  --secondary-foreground: #0a0a0a;
  --chart-1: #ff4d8b;
  --chart-2: #1a3a3a;
  --chart-3: #b8a4ed;
  --chart-4: #ffb084;
  --chart-5: #e8b94a;
  --code: #ffffff;
  --code-foreground: #0a0a0a;
  --code-highlight: #f5f0e0;
  --radius: 1rem;
  --sidebar: #faf5e8;
  --sidebar-accent: #f5f0e0;
  --sidebar-accent-foreground: #0a0a0a;
  --sidebar-border: #e5dcc8;
  --sidebar-foreground: #3a3a3a;
  --sidebar-primary: #0a0a0a;
  --sidebar-primary-foreground: #ffffff;
  --sidebar-ring: #1a3a3a;
}
```

- [ ] **Step 2: Add Clay utility classes**

Append this exact block before the final `@layer base` block:

```css
@layer utilities {
  .clay-canvas {
    background:
      radial-gradient(circle at top left, rgb(255 77 139 / 12%), transparent 28rem),
      radial-gradient(circle at top right, rgb(184 164 237 / 18%), transparent 30rem),
      linear-gradient(180deg, #fffaf0 0%, #faf5e8 100%);
  }

  .clay-shadow {
    box-shadow: 0 18px 45px rgb(10 10 10 / 10%), inset 0 1px 0 rgb(255 255 255 / 70%);
  }

  .clay-border {
    border-color: #e5dcc8;
  }

  .clay-ink {
    color: #0a0a0a;
  }

  .clay-muted {
    color: #6a6a6a;
  }

  .clay-code {
    border: 1px solid #e5dcc8;
    background: #ffffff;
    color: #1a3a3a;
  }
}
```

- [ ] **Step 3: Update body base style**

Change the `body` rule to:

```css
body {
  font-family: var(--font-sans), system-ui, -apple-system, sans-serif;
}
```

In `@layer base`, change the body apply rule to:

```css
body {
  @apply clay-canvas text-foreground;
}
```

- [ ] **Step 4: Verify no primitive files changed**

Run:

```bash
git diff --name-only -- packages/ui/components/ui
```

Expected: no output.

- [ ] **Step 5: Commit**

Run:

```bash
git add packages/ui/app/globals.css
git commit -m "style(ui): add clay theme tokens"
```

## Task 2: Clay Component Layer

**Files:**
- Create: `packages/ui/components/clay/layout.tsx`
- Create: `packages/ui/components/clay/controls.tsx`
- Create: `packages/ui/components/clay/data.tsx`
- Create: `packages/ui/components/clay/index.ts`

- [ ] **Step 1: Create layout primitives**

Create `packages/ui/components/clay/layout.tsx`:

```tsx
import type React from "react";
import { cn } from "~/lib/utils";
import { Card, CardContent, CardHeader, CardTitle } from "~/components/ui/card";

export function ClayPage({
  children,
  className,
}: {
  children: React.ReactNode;
  className?: string;
}) {
  return <div className={cn("flex flex-col gap-8", className)}>{children}</div>;
}

export function ClayHero({
  eyebrow,
  title,
  description,
  children,
  className,
}: {
  eyebrow?: string;
  title: string;
  description?: string;
  children?: React.ReactNode;
  className?: string;
}) {
  return (
    <section
      className={cn(
        "relative overflow-hidden rounded-[2rem] border clay-border bg-[#fffaf0] p-6 clay-shadow sm:p-8",
        className,
      )}
    >
      <div className="absolute right-6 top-6 h-24 w-24 rounded-full bg-[#ff4d8b] opacity-90" />
      <div className="absolute bottom-6 right-24 h-16 w-16 rounded-[1.25rem] bg-[#b8a4ed]" />
      <div className="relative max-w-3xl">
        {eyebrow && (
          <p className="mb-3 text-xs font-semibold uppercase tracking-[0.18em] text-[#1a3a3a]">
            {eyebrow}
          </p>
        )}
        <h1 className="text-4xl font-black leading-none clay-ink sm:text-6xl">
          {title}
        </h1>
        {description && (
          <p className="mt-4 max-w-2xl text-base leading-7 clay-muted sm:text-lg">
            {description}
          </p>
        )}
      </div>
      {children && <div className="relative mt-6">{children}</div>}
    </section>
  );
}

export function ClaySection({
  title,
  description,
  children,
  action,
  className,
}: {
  title: string;
  description?: string;
  children: React.ReactNode;
  action?: React.ReactNode;
  className?: string;
}) {
  return (
    <section className={cn("flex flex-col gap-4", className)}>
      <div className="flex flex-col gap-2 sm:flex-row sm:items-end sm:justify-between">
        <div>
          <h2 className="text-2xl font-black leading-tight clay-ink">{title}</h2>
          {description && <p className="text-sm clay-muted">{description}</p>}
        </div>
        {action}
      </div>
      {children}
    </section>
  );
}

export function ClayPanel({
  children,
  className,
}: {
  children: React.ReactNode;
  className?: string;
}) {
  return (
    <div
      className={cn(
        "rounded-[1.5rem] border clay-border bg-[#fffaf0] p-5 clay-shadow",
        className,
      )}
    >
      {children}
    </div>
  );
}

export function ClayCard({
  title,
  description,
  children,
  className,
}: {
  title?: string;
  description?: string;
  children: React.ReactNode;
  className?: string;
}) {
  return (
    <Card className={cn("rounded-[1.5rem] border-[#e5dcc8] bg-[#fffaf0] clay-shadow", className)}>
      {(title || description) && (
        <CardHeader>
          {title && <CardTitle className="text-xl font-black">{title}</CardTitle>}
          {description && <p className="text-sm clay-muted">{description}</p>}
        </CardHeader>
      )}
      <CardContent>{children}</CardContent>
    </Card>
  );
}

export function ClayFeatureCard({
  label,
  value,
  tone = "cream",
  children,
}: {
  label: string;
  value: string;
  tone?: "pink" | "teal" | "lavender" | "peach" | "ochre" | "cream";
  children?: React.ReactNode;
}) {
  const tones = {
    cream: "bg-[#fffaf0] text-[#0a0a0a]",
    lavender: "bg-[#b8a4ed] text-[#0a0a0a]",
    ochre: "bg-[#e8b94a] text-[#0a0a0a]",
    peach: "bg-[#ffb084] text-[#0a0a0a]",
    pink: "bg-[#ff4d8b] text-white",
    teal: "bg-[#1a3a3a] text-white",
  };

  return (
    <div className={cn("rounded-[1.5rem] border border-[#0a0a0a]/10 p-5 clay-shadow", tones[tone])}>
      <p className="text-sm font-semibold opacity-75">{label}</p>
      <p className="mt-2 text-3xl font-black leading-none">{value}</p>
      {children && <div className="mt-3 text-sm opacity-80">{children}</div>}
    </div>
  );
}
```

- [ ] **Step 2: Create controls wrappers**

Create `packages/ui/components/clay/controls.tsx`:

```tsx
import type React from "react";
import { cn } from "~/lib/utils";
import { Badge, type BadgeProps } from "~/components/ui/badge";
import { Button, type ButtonProps } from "~/components/ui/button";

export function ClayButton({ className, ...props }: ButtonProps): React.ReactElement {
  return (
    <Button
      className={cn(
        "rounded-xl border-[#0a0a0a] bg-[#0a0a0a] font-bold text-white shadow-[0_7px_0_#d9cfb8] transition-transform hover:-translate-y-0.5 active:translate-y-0 active:shadow-[0_3px_0_#d9cfb8]",
        className,
      )}
      {...props}
    />
  );
}

export function ClayBadge({ className, ...props }: BadgeProps): React.ReactElement {
  return (
    <Badge
      className={cn("rounded-full border border-[#0a0a0a]/10 px-3 py-1 font-bold", className)}
      {...props}
    />
  );
}
```

- [ ] **Step 3: Create data wrappers**

Create `packages/ui/components/clay/data.tsx`:

```tsx
import type React from "react";
import { cn } from "~/lib/utils";

export function ClayTableFrame({
  children,
  className,
}: {
  children: React.ReactNode;
  className?: string;
}) {
  return (
    <div
      className={cn(
        "overflow-x-auto rounded-[1.25rem] border clay-border bg-white shadow-[0_10px_30px_rgb(10_10_10/8%)]",
        className,
      )}
    >
      {children}
    </div>
  );
}

export function ClayEmptyState({
  title,
  description,
}: {
  title: string;
  description?: string;
}) {
  return (
    <div className="rounded-[1.25rem] border clay-border bg-[#faf5e8] p-5 text-sm">
      <p className="font-black clay-ink">{title}</p>
      {description && <p className="mt-1 clay-muted">{description}</p>}
    </div>
  );
}
```

- [ ] **Step 4: Create barrel export**

Create `packages/ui/components/clay/index.ts`:

```ts
export {
  ClayCard,
  ClayFeatureCard,
  ClayHero,
  ClayPage,
  ClayPanel,
  ClaySection,
} from "./layout";
export { ClayBadge, ClayButton } from "./controls";
export { ClayEmptyState, ClayTableFrame } from "./data";
```

- [ ] **Step 5: Type-check the new layer**

Run:

```bash
pnpm --filter @artemis/ui build
```

Expected: build reaches Next compilation. If unrelated route errors appear, fix only errors caused by the new Clay files.

- [ ] **Step 6: Commit**

Run:

```bash
git add packages/ui/components/clay
git commit -m "feat(ui): add clay component layer"
```

## Task 3: Layout Shell

**Files:**
- Modify: `packages/ui/components/layout/Header.tsx`
- Modify: `packages/ui/components/layout/Footer.tsx`

- [ ] **Step 1: Restyle header without changing nav logic**

Replace `Header.tsx` with:

```tsx
"use client";

import Link from "next/link";
import { usePathname } from "next/navigation";
import { useAccount } from "wagmi";
import { SUDO_ADDRESS } from "@artemis/shared";
import { ConnectButtonCustom } from "~/components/scaffold/ConnectButtonCustom";
import { cn } from "~/lib/utils";

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
    <header className="sticky top-0 z-40 border-b border-[#e5dcc8] bg-[#fffaf0]/90 backdrop-blur">
      <nav className="mx-auto flex max-w-7xl items-center justify-between gap-4 px-4 py-4">
        <div className="flex min-w-0 items-center gap-5">
          <Link href="/" className="shrink-0 text-2xl font-black tracking-tight clay-ink">
            Artemis
          </Link>
          <div className="flex min-w-0 gap-1 overflow-x-auto rounded-full border border-[#e5dcc8] bg-[#faf5e8] p-1">
            {allNavItems.map((item) => (
              <Link
                key={item.href}
                href={item.href}
                className={cn(
                  "rounded-full px-3 py-1.5 text-sm font-bold transition-colors",
                  pathname === item.href
                    ? "bg-[#0a0a0a] text-white"
                    : "text-[#3a3a3a] hover:bg-white",
                )}
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

- [ ] **Step 2: Restyle footer**

Replace `Footer.tsx` with:

```tsx
export function Footer() {
  return (
    <footer className="border-t border-[#e5dcc8]">
      <div className="mx-auto flex max-w-7xl flex-col gap-1 px-4 py-6 text-sm text-[#6a6a6a] sm:flex-row sm:items-center sm:justify-between">
        <p className="font-bold text-[#0a0a0a]">Artemis Chain</p>
        <p>ID 322 · ART Token · Frontier EVM</p>
      </div>
    </footer>
  );
}
```

- [ ] **Step 3: Build-check shell changes**

Run:

```bash
pnpm --filter @artemis/ui build
```

Expected: PASS.

- [ ] **Step 4: Commit**

Run:

```bash
git add packages/ui/components/layout/Header.tsx packages/ui/components/layout/Footer.tsx
git commit -m "style(ui): apply clay layout shell"
```

## Task 4: Core Routes

**Files:**
- Modify: `packages/ui/app/page.tsx`
- Modify: `packages/ui/app/transfer/page.tsx`
- Modify: `packages/ui/app/blockexplorer/page.tsx`

- [ ] **Step 1: Restyle home route**

Use `ClayPage`, `ClayHero`, and `ClayFeatureCard`. Keep the existing wagmi hooks and displayed values:

```tsx
"use client";

import { useAccount, useBalance, useBlockNumber } from "wagmi";
import { formatEther } from "viem";
import { ClayFeatureCard, ClayHero, ClayPage } from "~/components/clay";

export default function Home() {
  const { address, isConnected } = useAccount();
  const { data: balance } = useBalance({ address, query: { enabled: isConnected } });
  const { data: blockNumber } = useBlockNumber({ watch: true });

  return (
    <ClayPage>
      <ClayHero
        eyebrow="Artemis · Chain ID 322"
        title="Artemis Explorer"
        description="Block explorer, transfer console, and contract debugger for the ART chain."
      />
      <section className="grid gap-4 sm:grid-cols-3">
        <ClayFeatureCard label="Latest Block" value={blockNumber?.toString() ?? "—"} tone="pink" />
        <ClayFeatureCard label="Wallet Connected" value={isConnected ? "Yes" : "No"} tone="teal" />
        <ClayFeatureCard
          label="Balance"
          value={balance ? `${Number(formatEther(balance.value)).toFixed(2)} ART` : "—"}
          tone="lavender"
        />
      </section>
    </ClayPage>
  );
}
```

- [ ] **Step 2: Restyle transfer route**

Replace only presentation wrappers. Keep `validate`, `getTxParams`, `handleEstimate`, and `handleSubmit` behavior unchanged. Use `ClayPage`, `ClayHero`, `ClayCard`, `ClayBadge`, and `ClayButton`.

- [ ] **Step 3: Restyle explorer route**

Use `ClayPage`, `ClayHero`, `ClaySection`, `ClayFeatureCard`, and `ClayTableFrame`. Keep `useFetchBlocks(10)`, `SearchBar`, and `TransactionsTable` unchanged at this step.

- [ ] **Step 4: Build-check core routes**

Run:

```bash
pnpm --filter @artemis/ui build
```

Expected: PASS.

- [ ] **Step 5: Commit**

Run:

```bash
git add packages/ui/app/page.tsx packages/ui/app/transfer/page.tsx packages/ui/app/blockexplorer/page.tsx
git commit -m "style(ui): apply clay core routes"
```

## Task 5: Admin, Debug, And Tables

**Files:**
- Modify: `packages/ui/app/debug/page.tsx`
- Modify: `packages/ui/components/debug/ContractUI.tsx`
- Modify: `packages/ui/app/admin/gasless/page.tsx`
- Modify: `packages/ui/components/admin/RulesTable.tsx`
- Modify: `packages/ui/components/admin/AddRuleForm.tsx`
- Modify: `packages/ui/components/admin/CheckGaslessForm.tsx`
- Modify: `packages/ui/components/blockexplorer/TransactionsTable.tsx`

- [ ] **Step 1: Restyle debug route**

Wrap the page with `ClayPage` and `ClayHero`. Empty contract state must use:

```tsx
<ClayEmptyState
  title="No contracts configured"
  description="Add contract configs before using the debugger."
/>
```

- [ ] **Step 2: Restyle contract panel**

In `ContractUI.tsx`, keep `const [tab, setTab] = useState<"read" | "write">("read");`. Replace the outer border wrapper with `ClayPanel`. Style the tab buttons as a rounded segmented control. Do not change `ReadMethods` or `WriteMethods`.

- [ ] **Step 3: Restyle admin page**

Use `ClayPage`, `ClayHero`, `ClaySection`, and `ClayBadge`. Preserve the `isAdmin` expression:

```tsx
const isAdmin = isConnected && address?.toLowerCase() === SUDO_ADDRESS.toLowerCase();
```

- [ ] **Step 4: Restyle rules table**

Wrap the existing `Table` in `ClayTableFrame`. Replace the empty rules state with:

```tsx
<ClayEmptyState
  title="No gasless rules configured"
  description="Rules created by the sudo account will appear here."
/>
```

Keep `handleToggle`, `handleRemove`, `writeAsync`, and `isMining` unchanged.

- [ ] **Step 5: Restyle admin forms**

Use `ClayCard` around `AddRuleForm` and `CheckGaslessForm`. Keep all field state, validation, `writeAsync`, and read logic unchanged.

- [ ] **Step 6: Restyle transaction table**

Wrap the existing HTML table in `ClayTableFrame`. Use `ClayEmptyState` for loading and empty transaction states. Keep `formatEther`, `Address`, and transaction cell values unchanged.

- [ ] **Step 7: Build-check feature routes**

Run:

```bash
pnpm --filter @artemis/ui build
```

Expected: PASS.

- [ ] **Step 8: Commit**

Run:

```bash
git add packages/ui/app/debug/page.tsx packages/ui/components/debug/ContractUI.tsx packages/ui/app/admin/gasless/page.tsx packages/ui/components/admin/RulesTable.tsx packages/ui/components/admin/AddRuleForm.tsx packages/ui/components/admin/CheckGaslessForm.tsx packages/ui/components/blockexplorer/TransactionsTable.tsx
git commit -m "style(ui): apply clay feature surfaces"
```

## Task 6: Final Verification

**Files:**
- Inspect: `packages/ui/components/ui/*`
- Inspect: all changed files from previous tasks

- [ ] **Step 1: Confirm primitives stayed untouched**

Run:

```bash
git diff --name-only HEAD~4..HEAD -- packages/ui/components/ui
```

Expected: no output.

- [ ] **Step 2: Run UI build**

Run:

```bash
pnpm --filter @artemis/ui build
```

Expected: PASS.

- [ ] **Step 3: Start UI dev server**

Run:

```bash
pnpm --filter @artemis/ui dev
```

Expected: Next dev server starts on `http://localhost:3001`.

- [ ] **Step 4: Manual browser inspection**

Open and inspect these routes:

```text
http://localhost:3001/
http://localhost:3001/transfer
http://localhost:3001/blockexplorer
http://localhost:3001/debug
http://localhost:3001/admin/gasless
```

Expected:

- Cream canvas and dark ink are visible across all routes.
- Header and footer match the Clay surface.
- Home stat cards use saturated colors.
- Transfer, debug, explorer, and admin flows still render.
- Text does not overlap at mobile width.
- Data tables remain readable and horizontally scroll on narrow screens.

- [ ] **Step 5: Final status**

Run:

```bash
git status --short
```

Expected: no unstaged implementation changes except pre-existing unrelated files such as `.env.example` and `DESIGN.md`.
