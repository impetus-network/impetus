# Packages UI Phase 1 Design Spec

## Overview

Create `packages/ui` in the Artemis monorepo — a Next.js frontend forked
from scaffold-eth-2, with DaisyUI replaced by coss UI (Base UI + Tailwind
v4). Configured for Artemis chain (ID 322). Includes block explorer and
debug contracts pages from scaffold-eth-2.

## Goals

- Scaffold-eth-2 fork adapted for Artemis chain
- coss UI replaces DaisyUI for all components
- Tailwind CSS v4 (required by coss)
- pnpm workspace member (not Yarn)
- Block explorer and debug contracts functional on day one
- Reuse existing `packages/contracts` for Hardhat/Solidity (do not duplicate)

## Non-Goals

- DApp frontend (Phase 2-3)
- Admin panel / gasless management (Phase 4)
- Custom contract deployment UI
- Mobile-specific layouts
- Production deployment configuration

## Architecture

```
packages/ui/
├── app/                    Next.js app router
│   ├── layout.tsx          Root layout (providers, header, footer)
│   ├── page.tsx            Home page
│   ├── blockexplorer/      Block explorer pages
│   └── debug/              Debug contracts pages
├── components/
│   ├── scaffold-eth/       Scaffold-eth components (Address, Balance, etc.)
│   └── ui/                 coss primitives (installed via coss CLI)
├── contracts/              Generated contract types + deployed addresses
├── hooks/                  Custom hooks (scaffold-eth pattern)
├── services/               Store, web3 services
├── utils/                  Scaffold utilities
├── public/
├── next.config.ts
├── tailwind.config.ts      Tailwind v4 config
├── package.json
└── tsconfig.json
```

## Tech Stack

| Layer | Technology |
|-------|-----------|
| Framework | Next.js 14+ (app router) |
| UI | coss UI (Base UI + Tailwind CSS v4) |
| Wallet | wagmi v2 + RainbowKit |
| Chain interaction | viem |
| Package manager | pnpm (workspace member) |
| TypeScript | strict mode |

## Chain Configuration

```typescript
const artemis = {
  id: 322,
  name: "Artemis",
  nativeCurrency: { name: "Artemis Token", symbol: "ART", decimals: 18 },
  rpcUrls: { default: { http: ["http://127.0.0.1:9944"] } },
};
```

Target network: Artemis only. Remove all default Ethereum/Sepolia configs
from scaffold-eth-2.

## DaisyUI to coss Migration

Scaffold-eth-2 uses DaisyUI classes extensively (`btn`, `card`, `badge`,
`dropdown`, `modal`, `tabs`, `table`, etc.). Each must be replaced with
coss equivalents:

| DaisyUI | coss primitive |
|---------|---------------|
| `btn` | `Button` |
| `card` | `Card` |
| `badge` | `Badge` |
| `modal` | `Dialog` |
| `dropdown` | `Menu` |
| `tabs` | `Tabs` |
| `table` | `Table` |
| `input` | `Input` |
| `select` | `Select` |
| `tooltip` | `Tooltip` |
| `toast` | `Toast` |
| `skeleton` | `Skeleton` |
| `spinner` | `Spinner` |

Install coss primitives via:
```bash
npx coss add button card badge dialog menu tabs table input select tooltip toast skeleton spinner
```

## Tailwind v3 to v4 Migration

- Remove `tailwind.config.js` (v3 style)
- Use CSS-first configuration with `@theme` directive
- Import via `@import "tailwindcss"` in global CSS
- Custom colors defined as CSS custom properties with `oklch()`
- DaisyUI theme variables removed entirely

## Scaffold-eth-2 Components to Adapt

These scaffold-eth-2 components need restyling with coss:

**Core (must have):**
- `Header` — Navigation bar with wallet connect
- `Footer` — Links and info
- `Address` — Display/copy EVM address
- `Balance` — Show token balance
- `BlockieAvatar` — Address avatar

**Block Explorer:**
- `TransactionsTable` — List of transactions
- `AddressComponent` — Address detail page
- `SearchBar` — Search by address/tx/block

**Debug Contracts:**
- `ContractUI` — Contract interface panel
- `ContractReadMethods` — Read functions list
- `ContractWriteMethods` — Write functions list
- `ContractInput` — Typed input fields
- `DisplayVariable` — Show return values

## Pages

### Home (`/`)
- Artemis branding
- Wallet connection via RainbowKit
- Quick stats (block number, gas price)
- Navigation to explorer and debug

### Block Explorer (`/blockexplorer`)
- Latest blocks and transactions
- Search by address, tx hash, or block number
- Address detail page with balance and transaction history
- Transaction detail page

### Debug Contracts (`/debug`)
- Auto-detect deployed contracts from `packages/contracts`
- List all read/write functions
- Input fields typed per ABI
- Execute and display results
- Show events

## Contract Integration

Scaffold-eth-2 generates contract types from deployments. Instead of its own
`packages/hardhat`, we point to `packages/contracts`:

- Read deployed contract addresses from hardhat deployment artifacts
- Generate TypeScript types from ABI
- Use `@artemis/shared` for precompile addresses and ABIs

A `scripts/generate-contracts.ts` script (or Next.js plugin) reads hardhat
artifacts and produces typed hooks.

## Scaffold-eth-2 Removal List

Remove from fork:
- `packages/hardhat/` (use existing `packages/contracts`)
- Yarn lock, `.yarnrc.yml`
- DaisyUI dependency and all DaisyUI theme config
- Tailwind v3 config format
- All default Ethereum network configs (mainnet, sepolia, etc.)
- Example contracts (keep only deploy scripts pattern)
- GitHub-specific files (.github from scaffold-eth-2)

## Dependencies

```json
{
  "dependencies": {
    "@artemis/shared": "workspace:*",
    "@base-ui/react": "latest",
    "@rainbow-me/rainbowkit": "^2.2",
    "@tanstack/react-query": "^5",
    "next": "^14",
    "react": "^19",
    "react-dom": "^19",
    "viem": "^2",
    "wagmi": "^2"
  },
  "devDependencies": {
    "tailwindcss": "^4",
    "@tailwindcss/postcss": "^4",
    "typescript": "^5"
  }
}
```

## pnpm Workspace Integration

Add to root `pnpm-workspace.yaml`:
```yaml
packages:
  - "packages/ui"
```

Add to `turbo.json` tasks as needed.

## Verification

Phase 1 is complete when:
- `pnpm dev` in `packages/ui` starts Next.js dev server
- Home page loads with Artemis branding and wallet connect
- Wallet connects to local Artemis node (chain ID 322)
- Block explorer shows blocks from dev node
- Debug page shows deployed contracts (if any)
- All DaisyUI references removed
- All components use coss primitives
- Tailwind v4 CSS-first config works

## Open Risks

- Scaffold-eth-2 components are tightly coupled to DaisyUI class names.
  Migration requires touching every component file. Estimate: 20-30 files.
- Some scaffold-eth-2 hooks assume specific chain behavior (e.g., ENS
  resolution) that may not apply to Artemis. These need to be disabled or
  stubbed.
- coss does not have all DaisyUI utility classes (e.g., `loading` animation).
  May need custom Tailwind utilities for some edge cases.
