# Webapp MVP Design -- Connect Wallet on Artemis

## Overview

Minimal viable frontend for the Artemis betting blockchain. MVP scope: connect wallet via RainbowKit, display address and ART balance. Routing and state management scaffolded for future betting/admin features.

Location: `packages/webapp` within the existing Turborepo monorepo.

## Stack

| Layer | Choice |
|-------|--------|
| Build | Vite + React 19 + TypeScript strict |
| Wallet | RainbowKit + wagmi v2 + viem |
| Server state | TanStack Query (via wagmi hooks) |
| Client state | Zustand (scaffolded, unused in MVP) |
| UI | coss ui + Tailwind CSS v4 |
| Routing | React Router v7 |
| E2E testing | Playwright + dappwright (MetaMask automation) |
| Shared | `@betting/shared` (ABI, addresses, chain config, types) |

## Custom Chain Definition

Define Artemis chain using constants from `@betting/shared`:

- Chain ID: 322
- Token: ART (18 decimals)
- RPC: `http://127.0.0.1:9944`

No hardcoded values in webapp -- import from shared package.

## Pages

| Route | Component | MVP Status |
|-------|-----------|------------|
| `/` | Home -- connect wallet, show address + ART balance | Built |
| `/rounds` | Betting rounds list | Placeholder |
| `/rounds/:id` | Round detail + place bet | Placeholder |
| `/admin` | Admin panel (submit result, claim pool) | Placeholder |

## Project Structure

```
packages/webapp/
  src/
    main.tsx              # Entry point + provider hierarchy
    App.tsx               # React Router setup
    config/
      wagmi.ts            # wagmi + RainbowKit config (getDefaultConfig)
      chain.ts            # Artemis chain definition from @betting/shared
    pages/
      Home.tsx            # Connect wallet + balance display
      Rounds.tsx          # Placeholder
      RoundDetail.tsx     # Placeholder
      Admin.tsx           # Placeholder
    components/
      layout/
        AppLayout.tsx     # Navigation + <Outlet />
    stores/               # Zustand stores (empty for MVP)
    lib/                  # Utilities
  e2e/
    setup.ts              # dappwright bootstrap (install MetaMask, import account)
    connect-wallet.spec.ts
  index.html
  vite.config.ts
  tailwind.config.ts
  tsconfig.json
  playwright.config.ts
  package.json
```

## Provider Hierarchy

```
WagmiProvider
  QueryClientProvider
    RainbowKitProvider
      BrowserRouter
        AppLayout
          <Outlet />
```

Order matters: WagmiProvider outermost, RainbowKitProvider innermost before router.

## Dependencies

### Runtime

- `@rainbow-me/rainbowkit`
- `wagmi`
- `viem`
- `@tanstack/react-query`
- `react-router`
- `zustand`
- `@betting/shared` (workspace dependency)

### Dev

- `vite`
- `@vitejs/plugin-react`
- `typescript`
- `tailwindcss`
- `@tailwindcss/vite`
- `@playwright/test`
- `dappwright`

### UI

coss ui installed via `npx shadcn@latest add @coss/style`, then individual components as needed.

## Environment Variables

| Variable | Purpose |
|----------|---------|
| `VITE_WALLETCONNECT_PROJECT_ID` | WalletConnect Cloud project ID (required for RainbowKit) |

## E2E Testing

### Framework

Playwright with dappwright for MetaMask wallet automation in browser tests.

### MVP Test Scope

Single test: connect wallet flow.

1. dappwright installs MetaMask extension in Playwright browser
2. Import dev account #1 (`0x70997970C51812dc3A010C7d01b50e0d17dc79C8`) using mnemonic `test test test test test test test test test test test junk`
3. Add Artemis network (chain ID 322, RPC localhost:9944)
4. Navigate to app, click connect, approve in MetaMask
5. Assert: address and ART balance displayed correctly

### Test Structure

```
e2e/
  setup.ts                  # dappwright MetaMask bootstrap
  connect-wallet.spec.ts    # Connect + verify address/balance
playwright.config.ts        # Config with dappwright setup
```

### Prerequisites

Local Artemis dev node must be running (`frontier-template-node --dev`) before E2E tests execute. Dev accounts are pre-funded with 1,000,000 ART at genesis.

## Out of Scope (Future Iterations)

- Betting UI (place bet, view rounds)
- Admin panel (submit result, claim pool, admin claim)
- Real-time event subscriptions
- Transaction history
- Responsive/mobile optimization beyond basic layout
- Production deployment configuration
