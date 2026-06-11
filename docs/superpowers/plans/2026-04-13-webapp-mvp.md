# Webapp MVP Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Scaffold a Vite + React webapp in `packages/webapp` that connects a wallet via RainbowKit to the custom Artemis chain (ID 322) and displays the connected address + ART balance.

**Architecture:** Single-page app with React Router for future page expansion. RainbowKit handles wallet connection UI, wagmi + viem handle chain interaction, TanStack Query manages caching. Artemis chain definition imports constants from `@betting/shared`. coss ui provides component primitives. E2E tests use Playwright + dappwright for MetaMask automation.

**Tech Stack:** Vite, React 19, TypeScript strict, RainbowKit, wagmi v2, viem, TanStack Query, Zustand, coss ui, Tailwind CSS v4, React Router v7, Playwright, dappwright

---

## File Map

| File | Responsibility |
|------|---------------|
| `packages/webapp/package.json` | Dependencies, scripts |
| `packages/webapp/tsconfig.json` | TypeScript strict config with path aliases |
| `packages/webapp/tsconfig.node.json` | Node config for vite.config.ts |
| `packages/webapp/vite.config.ts` | Vite + React plugin + Tailwind + path alias |
| `packages/webapp/index.html` | HTML entry point |
| `packages/webapp/src/app.css` | Tailwind v4 + coss ui CSS imports |
| `packages/webapp/src/main.tsx` | React root + provider hierarchy |
| `packages/webapp/src/App.tsx` | React Router routes |
| `packages/webapp/src/config/chain.ts` | Artemis chain definition from @betting/shared |
| `packages/webapp/src/config/wagmi.ts` | wagmi + RainbowKit config |
| `packages/webapp/src/components/layout/AppLayout.tsx` | Nav + Outlet |
| `packages/webapp/src/pages/Home.tsx` | Connect wallet + balance display |
| `packages/webapp/src/pages/Rounds.tsx` | Placeholder |
| `packages/webapp/src/pages/RoundDetail.tsx` | Placeholder |
| `packages/webapp/src/pages/Admin.tsx` | Placeholder |
| `packages/webapp/src/stores/.gitkeep` | Zustand store directory (empty for MVP) |
| `packages/webapp/.env.example` | Environment variable template |
| `packages/webapp/e2e/fixtures.ts` | dappwright + Playwright test fixtures |
| `packages/webapp/e2e/connect-wallet.spec.ts` | E2E: connect wallet flow |
| `packages/webapp/playwright.config.ts` | Playwright config for dappwright |

---

### Task 1: Scaffold Vite project and install dependencies

**Files:**
- Create: `packages/webapp/package.json`
- Create: `packages/webapp/tsconfig.json`
- Create: `packages/webapp/tsconfig.node.json`
- Create: `packages/webapp/vite.config.ts`
- Create: `packages/webapp/index.html`
- Create: `packages/webapp/.env.example`

- [ ] **Step 1: Create package.json**

```json
{
  "name": "@betting/webapp",
  "version": "0.0.0",
  "private": true,
  "type": "module",
  "scripts": {
    "dev": "vite",
    "build": "tsc -b && vite build",
    "preview": "vite preview",
    "test:e2e": "playwright test"
  }
}
```

Write this to `packages/webapp/package.json` (overwrite the existing stub).

- [ ] **Step 2: Install runtime dependencies**

Run:
```bash
cd packages/webapp && pnpm add @rainbow-me/rainbowkit wagmi viem @tanstack/react-query react react-dom react-router zustand @betting/shared
```

- [ ] **Step 3: Install dev dependencies**

Run:
```bash
cd packages/webapp && pnpm add -D typescript @types/react @types/react-dom @vitejs/plugin-react vite tailwindcss @tailwindcss/vite @playwright/test @tenkeylabs/dappwright
```

- [ ] **Step 4: Create tsconfig.json**

Write to `packages/webapp/tsconfig.json`:

```json
{
  "compilerOptions": {
    "target": "ES2022",
    "lib": ["ES2022", "DOM", "DOM.Iterable"],
    "module": "ESNext",
    "moduleResolution": "bundler",
    "strict": true,
    "esModuleInterop": true,
    "skipLibCheck": true,
    "forceConsistentCasingInFileNames": true,
    "resolveJsonModule": true,
    "isolatedModules": true,
    "noEmit": true,
    "jsx": "react-jsx",
    "baseUrl": ".",
    "paths": {
      "@/*": ["./src/*"]
    }
  },
  "include": ["src"],
  "references": [{ "path": "./tsconfig.node.json" }]
}
```

- [ ] **Step 5: Create tsconfig.node.json**

Write to `packages/webapp/tsconfig.node.json`:

```json
{
  "compilerOptions": {
    "target": "ES2022",
    "lib": ["ES2022"],
    "module": "ESNext",
    "moduleResolution": "bundler",
    "strict": true,
    "esModuleInterop": true,
    "skipLibCheck": true,
    "noEmit": true,
    "isolatedModules": true
  },
  "include": ["vite.config.ts"]
}
```

- [ ] **Step 6: Create vite.config.ts**

Write to `packages/webapp/vite.config.ts`:

```ts
import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";
import path from "path";

export default defineConfig({
  plugins: [react(), tailwindcss()],
  resolve: {
    alias: {
      "@": path.resolve(__dirname, "./src"),
    },
  },
  server: {
    port: 3000,
  },
});
```

- [ ] **Step 7: Create index.html**

Write to `packages/webapp/index.html`:

```html
<!DOCTYPE html>
<html lang="en">
  <head>
    <meta charset="UTF-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <title>Artemis Betting</title>
  </head>
  <body>
    <div id="root"></div>
    <script type="module" src="/src/main.tsx"></script>
  </body>
</html>
```

- [ ] **Step 8: Create .env.example**

Write to `packages/webapp/.env.example`:

```
VITE_WALLETCONNECT_PROJECT_ID=your_project_id_here
```

- [ ] **Step 9: Commit**

```bash
git add packages/webapp/package.json packages/webapp/tsconfig.json packages/webapp/tsconfig.node.json packages/webapp/vite.config.ts packages/webapp/index.html packages/webapp/.env.example pnpm-lock.yaml
git commit -m "chore(webapp): scaffold Vite project with dependencies"
```

---

### Task 2: Set up Tailwind CSS v4 + coss ui

**Files:**
- Create: `packages/webapp/src/app.css`
- Create: `packages/webapp/components.json` (via shadcn CLI)

- [ ] **Step 1: Initialize coss ui**

Run:
```bash
cd packages/webapp && npx shadcn@latest init @coss/style
```

Follow prompts: select Vite framework, confirm defaults. This creates `components.json` and wires up CSS imports.

- [ ] **Step 2: Verify or create src/app.css**

Ensure `packages/webapp/src/app.css` contains (add if not created by CLI):

```css
@import "tailwindcss";
@import "@coss/ui/styles";
```

- [ ] **Step 3: Install button component for MVP**

Run:
```bash
cd packages/webapp && npx shadcn@latest add @coss/button
```

- [ ] **Step 4: Verify Tailwind works**

Run:
```bash
cd packages/webapp && pnpm build
```

Expected: Build succeeds (may warn about missing entry point -- that is fine, we add it next task).

- [ ] **Step 5: Commit**

```bash
git add packages/webapp/src/app.css packages/webapp/components.json packages/webapp/src/components/
git commit -m "chore(webapp): set up Tailwind CSS v4 and coss ui"
```

---

### Task 3: Artemis chain config + wagmi setup

**Files:**
- Create: `packages/webapp/src/config/chain.ts`
- Create: `packages/webapp/src/config/wagmi.ts`

- [ ] **Step 1: Create chain definition**

Write to `packages/webapp/src/config/chain.ts`:

```ts
import { type Chain } from "@rainbow-me/rainbowkit";
import { CHAIN_CONFIG } from "@betting/shared";

export const artemis = {
  id: CHAIN_CONFIG.chainId,
  name: "Artemis",
  nativeCurrency: {
    name: "Artemis Token",
    symbol: CHAIN_CONFIG.tokenSymbol,
    decimals: CHAIN_CONFIG.tokenDecimals,
  },
  rpcUrls: {
    default: {
      http: [CHAIN_CONFIG.rpcUrl],
    },
  },
} as const satisfies Chain;
```

- [ ] **Step 2: Create wagmi config**

Write to `packages/webapp/src/config/wagmi.ts`:

```ts
import { getDefaultConfig } from "@rainbow-me/rainbowkit";
import { artemis } from "./chain";

export const wagmiConfig = getDefaultConfig({
  appName: "Artemis Betting",
  projectId: import.meta.env.VITE_WALLETCONNECT_PROJECT_ID ?? "",
  chains: [artemis],
});
```

- [ ] **Step 3: Verify TypeScript compiles**

Run:
```bash
cd packages/webapp && npx tsc --noEmit
```

Expected: No errors (or only errors from missing main.tsx/App.tsx which we create next).

- [ ] **Step 4: Commit**

```bash
git add packages/webapp/src/config/
git commit -m "feat(webapp): add Artemis chain definition and wagmi config"
```

---

### Task 4: Provider hierarchy + entry point

**Files:**
- Create: `packages/webapp/src/main.tsx`
- Create: `packages/webapp/src/App.tsx`

- [ ] **Step 1: Create main.tsx**

Write to `packages/webapp/src/main.tsx`:

```tsx
import "@rainbow-me/rainbowkit/styles.css";
import "@/app.css";

import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { WagmiProvider } from "wagmi";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { RainbowKitProvider } from "@rainbow-me/rainbowkit";
import { BrowserRouter } from "react-router";

import { wagmiConfig } from "@/config/wagmi";
import App from "@/App";

const queryClient = new QueryClient();

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <WagmiProvider config={wagmiConfig}>
      <QueryClientProvider client={queryClient}>
        <RainbowKitProvider>
          <BrowserRouter>
            <App />
          </BrowserRouter>
        </RainbowKitProvider>
      </QueryClientProvider>
    </WagmiProvider>
  </StrictMode>,
);
```

- [ ] **Step 2: Create App.tsx with routes**

Write to `packages/webapp/src/App.tsx`:

```tsx
import { Routes, Route } from "react-router";
import { AppLayout } from "@/components/layout/AppLayout";
import { Home } from "@/pages/Home";
import { Rounds } from "@/pages/Rounds";
import { RoundDetail } from "@/pages/RoundDetail";
import { Admin } from "@/pages/Admin";

export default function App() {
  return (
    <Routes>
      <Route element={<AppLayout />}>
        <Route index element={<Home />} />
        <Route path="rounds" element={<Rounds />} />
        <Route path="rounds/:id" element={<RoundDetail />} />
        <Route path="admin" element={<Admin />} />
      </Route>
    </Routes>
  );
}
```

- [ ] **Step 3: Commit**

```bash
git add packages/webapp/src/main.tsx packages/webapp/src/App.tsx
git commit -m "feat(webapp): add provider hierarchy and router setup"
```

---

### Task 5: Layout component

**Files:**
- Create: `packages/webapp/src/components/layout/AppLayout.tsx`

- [ ] **Step 1: Create AppLayout**

Write to `packages/webapp/src/components/layout/AppLayout.tsx`:

```tsx
import { ConnectButton } from "@rainbow-me/rainbowkit";
import { Link, Outlet } from "react-router";

export function AppLayout() {
  return (
    <div className="min-h-screen bg-background text-foreground">
      <header className="border-b border-border">
        <nav className="mx-auto flex max-w-5xl items-center justify-between px-4 py-3">
          <div className="flex items-center gap-6">
            <Link to="/" className="text-lg font-bold">
              Artemis
            </Link>
            <Link to="/rounds" className="text-sm text-muted-foreground hover:text-foreground">
              Rounds
            </Link>
            <Link to="/admin" className="text-sm text-muted-foreground hover:text-foreground">
              Admin
            </Link>
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

- [ ] **Step 2: Commit**

```bash
git add packages/webapp/src/components/layout/
git commit -m "feat(webapp): add AppLayout with nav and ConnectButton"
```

---

### Task 6: Home page -- connect wallet + balance

**Files:**
- Create: `packages/webapp/src/pages/Home.tsx`

- [ ] **Step 1: Create Home page**

Write to `packages/webapp/src/pages/Home.tsx`:

```tsx
import { ConnectButton } from "@rainbow-me/rainbowkit";
import { useAccount, useBalance } from "wagmi";
import { formatEther } from "viem";

export function Home() {
  const { address, isConnected } = useAccount();
  const { data: balance } = useBalance({ address });

  if (!isConnected) {
    return (
      <div className="flex flex-col items-center gap-6 pt-24">
        <h1 className="text-4xl font-bold">Artemis Betting</h1>
        <p className="text-muted-foreground">Connect your wallet to get started</p>
        <ConnectButton />
      </div>
    );
  }

  return (
    <div className="flex flex-col gap-6">
      <h1 className="text-2xl font-bold">Dashboard</h1>
      <div className="rounded-lg border border-border p-6">
        <dl className="grid gap-4">
          <div>
            <dt className="text-sm text-muted-foreground">Address</dt>
            <dd className="font-mono text-sm" data-testid="wallet-address">
              {address}
            </dd>
          </div>
          <div>
            <dt className="text-sm text-muted-foreground">Balance</dt>
            <dd className="text-2xl font-bold" data-testid="wallet-balance">
              {balance ? formatEther(balance.value) : "0"} ART
            </dd>
          </div>
        </dl>
      </div>
    </div>
  );
}
```

- [ ] **Step 2: Commit**

```bash
git add packages/webapp/src/pages/Home.tsx
git commit -m "feat(webapp): add Home page with wallet connection and balance"
```

---

### Task 7: Placeholder pages

**Files:**
- Create: `packages/webapp/src/pages/Rounds.tsx`
- Create: `packages/webapp/src/pages/RoundDetail.tsx`
- Create: `packages/webapp/src/pages/Admin.tsx`
- Create: `packages/webapp/src/stores/.gitkeep`

- [ ] **Step 1: Create Rounds placeholder**

Write to `packages/webapp/src/pages/Rounds.tsx`:

```tsx
export function Rounds() {
  return (
    <div>
      <h1 className="text-2xl font-bold">Rounds</h1>
      <p className="mt-2 text-muted-foreground">Coming soon</p>
    </div>
  );
}
```

- [ ] **Step 2: Create RoundDetail placeholder**

Write to `packages/webapp/src/pages/RoundDetail.tsx`:

```tsx
import { useParams } from "react-router";

export function RoundDetail() {
  const { id } = useParams<{ id: string }>();

  return (
    <div>
      <h1 className="text-2xl font-bold">Round #{id}</h1>
      <p className="mt-2 text-muted-foreground">Coming soon</p>
    </div>
  );
}
```

- [ ] **Step 3: Create Admin placeholder**

Write to `packages/webapp/src/pages/Admin.tsx`:

```tsx
export function Admin() {
  return (
    <div>
      <h1 className="text-2xl font-bold">Admin</h1>
      <p className="mt-2 text-muted-foreground">Coming soon</p>
    </div>
  );
}
```

- [ ] **Step 4: Create stores directory**

Run:
```bash
touch packages/webapp/src/stores/.gitkeep
```

- [ ] **Step 5: Commit**

```bash
git add packages/webapp/src/pages/Rounds.tsx packages/webapp/src/pages/RoundDetail.tsx packages/webapp/src/pages/Admin.tsx packages/webapp/src/stores/.gitkeep
git commit -m "feat(webapp): add placeholder pages and stores directory"
```

---

### Task 8: Verify dev server runs

- [ ] **Step 1: Create .env for local development**

Run:
```bash
cp packages/webapp/.env.example packages/webapp/.env
```

Edit `packages/webapp/.env` and set a WalletConnect project ID (get one from cloud.walletconnect.com, or use a placeholder for local MetaMask-only testing).

- [ ] **Step 2: Build shared package first**

Run:
```bash
cd packages/shared && pnpm build
```

Expected: Compiles successfully, `dist/` created.

- [ ] **Step 3: Start dev server**

Run:
```bash
cd packages/webapp && pnpm dev
```

Expected: Vite starts on http://localhost:3000. Open in browser -- page loads with "Artemis Betting" heading and Connect Wallet button.

- [ ] **Step 4: Verify TypeScript compiles**

Run:
```bash
cd packages/webapp && npx tsc --noEmit
```

Expected: No errors.

- [ ] **Step 5: Verify production build**

Run:
```bash
cd packages/webapp && pnpm build
```

Expected: Build succeeds, `dist/` created.

- [ ] **Step 6: Commit .env to gitignore if not already**

Verify `packages/webapp/.env` is covered by root `.gitignore` (which already ignores `.env`). No action needed if so.

---

### Task 9: E2E test fixtures with dappwright

**Files:**
- Create: `packages/webapp/playwright.config.ts`
- Create: `packages/webapp/e2e/fixtures.ts`

- [ ] **Step 1: Create playwright.config.ts**

Write to `packages/webapp/playwright.config.ts`:

```ts
import { defineConfig } from "@playwright/test";

export default defineConfig({
  testDir: "./e2e",
  testMatch: "**/*.spec.ts",
  retries: process.env.CI ? 1 : 0,
  timeout: 60_000,
  use: {
    headless: false,
    trace: "retain-on-first-failure",
  },
  reporter: [["list"], ["html", { open: "on-failure" }]],
  webServer: {
    command: "pnpm dev",
    url: "http://localhost:3000",
    timeout: 30_000,
    reuseExistingServer: !process.env.CI,
  },
});
```

- [ ] **Step 2: Create test fixtures**

Write to `packages/webapp/e2e/fixtures.ts`:

```ts
import { test as base } from "@playwright/test";
import type { BrowserContext } from "playwright-core";
import {
  bootstrap,
  getWallet,
  MetaMaskWallet,
  type Dappwright,
} from "@tenkeylabs/dappwright";

const SEED = "test test test test test test test test test test test junk";
const PASSWORD = "password1234!@#$";

export const test = base.extend<
  { wallet: Dappwright },
  { walletContext: BrowserContext }
>({
  walletContext: [
    async ({}, use) => {
      const [, , context] = await bootstrap("", {
        wallet: "metamask",
        version: MetaMaskWallet.recommendedVersion,
        seed: SEED,
        password: PASSWORD,
        headless: false,
      });
      await use(context);
      await context.close();
    },
    { scope: "worker" },
  ],
  context: async ({ walletContext }, use) => {
    await use(walletContext);
  },
  wallet: async ({ walletContext }, use) => {
    const wallet = await getWallet("metamask", walletContext);
    await use(wallet);
  },
});

export { expect } from "@playwright/test";
```

- [ ] **Step 3: Commit**

```bash
git add packages/webapp/playwright.config.ts packages/webapp/e2e/fixtures.ts
git commit -m "test(webapp): add Playwright config and dappwright fixtures"
```

---

### Task 10: E2E test -- connect wallet flow

**Files:**
- Create: `packages/webapp/e2e/connect-wallet.spec.ts`

- [ ] **Step 1: Write the connect wallet test**

Write to `packages/webapp/e2e/connect-wallet.spec.ts`:

```ts
import { test, expect } from "./fixtures";

test.describe("Connect Wallet", () => {
  test("connects MetaMask and displays address and balance", async ({
    wallet,
    page,
  }) => {
    // Add Artemis network to MetaMask
    await wallet.addNetwork({
      networkName: "Artemis",
      rpc: "http://localhost:9944",
      chainId: 322,
      symbol: "ART",
    });

    // Navigate to app
    await page.goto("http://localhost:3000");

    // Verify disconnect state
    await expect(page.getByText("Connect your wallet to get started")).toBeVisible();

    // Click RainbowKit connect button
    await page.getByRole("button", { name: /connect wallet/i }).click();

    // Select MetaMask in RainbowKit modal
    await page.getByRole("button", { name: /metamask/i }).click();

    // Approve connection in MetaMask
    await wallet.approve();

    // Verify connected state
    await expect(page.getByTestId("wallet-address")).toBeVisible();
    await expect(page.getByTestId("wallet-address")).toContainText("0x70997970");

    // Verify balance is displayed (dev account has 1,000,000 ART)
    await expect(page.getByTestId("wallet-balance")).toBeVisible();
    await expect(page.getByTestId("wallet-balance")).toContainText("ART");
  });
});
```

- [ ] **Step 2: Run E2E test (requires local Artemis node running)**

Ensure the dev node is running:
```bash
cd packages/node && ./target/release/frontier-template-node --dev
```

Then run:
```bash
cd packages/webapp && pnpm test:e2e
```

Expected: Test passes -- MetaMask connects, address and balance display correctly.

- [ ] **Step 3: Commit**

```bash
git add packages/webapp/e2e/connect-wallet.spec.ts
git commit -m "test(webapp): add E2E test for wallet connection flow"
```

---

### Task 11: Final verification

- [ ] **Step 1: Full TypeScript check**

Run:
```bash
cd packages/webapp && npx tsc --noEmit
```

Expected: No errors.

- [ ] **Step 2: Production build**

Run:
```bash
cd packages/webapp && pnpm build
```

Expected: Build succeeds.

- [ ] **Step 3: Turbo build from root**

Run:
```bash
cd /Users/huyduan/projects/blockchain && pnpm turbo build
```

Expected: All packages build successfully, including `@betting/webapp`.

- [ ] **Step 4: Run E2E test suite**

Run (with dev node running):
```bash
cd packages/webapp && pnpm test:e2e
```

Expected: All tests pass.
