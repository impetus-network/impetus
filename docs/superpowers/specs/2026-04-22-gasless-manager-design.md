# Gasless Manager Admin Page Design Spec

## Overview

Admin page at `/admin/gasless` for managing GaslessRegistry precompile rules.
Reads rules from Ponder GraphQL indexer, writes via `useScaffoldWriteContract`.
Only sudo account can write; others see read-only mode.

## Data Sources

- **Read**: Ponder GraphQL at `NEXT_PUBLIC_PONDER_URL` (default `http://localhost:42069`)
  - Query `gaslessRules` table (id, contract, selector, enabled, minValue, updatedAtBlock)
- **Write**: GaslessRegistry precompile at `0x0000000000000000000000000000000000000800`
  - `setRule(contract, selector, minValue, enabled)` — add or update rule
  - `removeRule(contract, selector)` — remove rule

## Access Control

Connected wallet is checked against `SUDO_ADDRESS` from `@artemis/shared`.
- **Sudo**: Full access — table with actions, add form, check form
- **Others**: Read-only — table without actions, check form only, no add form

## Features

### 1. Rules Table

- Columns: Contract, Selector, Enabled (badge), MinValue, Actions
- Data from Ponder GraphQL query
- Auto-refresh on interval (poll every 5s via TanStack Query)
- Inline actions per row:
  - Toggle: flip enabled via `setRule` (keeps same minValue)
  - Remove: `removeRule` after confirmation dialog

### 2. Add Rule Form

- Fields:
  - Contract address (text, validated with `isAddress`)
  - Function selector (text, bytes4 hex, e.g. `0xa9059cbb`)
  - MinValue (number, in wei)
  - Enabled (switch, default true)
- Submit calls `setRule` via `useScaffoldWriteContract`
- Form resets after success
- Only visible when sudo connected

### 3. Check Gasless Form

- Fields:
  - Contract address
  - Calldata (hex bytes)
  - Value (uint256)
  - Gas limit (uint256)
- Calls `isGasless` view function
- Shows result as badge: true (green) / false (red)
- Available to all connected wallets

## Files

- `app/admin/gasless/page.tsx` — page layout, sudo guard
- `components/admin/RulesTable.tsx` — table with inline actions
- `components/admin/AddRuleForm.tsx` — add/update rule form
- `components/admin/CheckGaslessForm.tsx` — isGasless test form
- `hooks/useGaslessRules.ts` — fetch from Ponder GraphQL
- `config/ponder.ts` — Ponder URL config

## Ponder GraphQL Query

```graphql
query {
  gaslessRuless(orderBy: "updatedAtBlock", orderDirection: "desc") {
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
```

Note: Ponder auto-pluralizes table name — verify actual query name at runtime.

## coss Components Used

- Table (rules list)
- Card (form containers)
- Field, Input, Switch (form fields)
- Button (actions)
- Badge (enabled/disabled status, gasless check result)
- Dialog (remove confirmation — via useTransactor confirm flow)

## Nav

Add "Admin" link to Header nav, only visible when sudo connected.

## Non-Goals

- Role-based access beyond sudo check
- Batch rule operations
- Rule history/audit log (covered by block explorer events)
- Deploy to production Ponder URL (Phase 2+)
