# ENS Domain Resolution

Server-side ENS name resolution for the Artemis transfer page, allowing users to send ART tokens to ENS names (e.g. `vitalik.eth`) instead of raw addresses.

## Architecture

```
Browser (transfer page)
  |
  |  GET /api/ens/resolve?name=vitalik.eth
  v
Next.js API Route (server-side)
  |
  |  publicClient.getEnsAddress({ name })
  v
ETH Mainnet RPC (Alchemy, via ETH_MAINNET_RPC_URL)
```

ENS resolution happens server-side to keep the Alchemy API key out of the browser. The client queries the API route through a TanStack Query hook with debouncing.

## Secret Management

- **Key:** `ETH_MAINNET_RPC_URL`
- **Value:** Alchemy ETH mainnet endpoint
- **Storage:** Infisical (project: Blockchain, env: dev)
- **Exposure:** Server-side only (no `NEXT_PUBLIC_` prefix)

## API Route

**File:** `packages/ui/app/api/ens/resolve/route.ts`

**Endpoint:** `GET /api/ens/resolve?name=<ens-name>`

**Responses:**

| Status | Body | Condition |
|--------|------|-----------|
| 200 | `{ "address": "0x..." }` | Resolved successfully |
| 400 | `{ "error": "Invalid ENS name" }` | Missing or malformed name |
| 404 | `{ "error": "ENS name not found" }` | Name does not resolve |
| 500 | `{ "error": "Failed to resolve ENS name" }` | RPC or server error |

**Implementation details:**
- Create a viem `publicClient` for ETH mainnet using `ETH_MAINNET_RPC_URL`
- Validate and normalize the ENS name with `normalize` from `viem/ens`
- Call `publicClient.getEnsAddress({ name })` to resolve
- No cache headers; caching handled client-side by TanStack Query

## Client Hook

**File:** `packages/ui/hooks/useEnsResolve.ts`

**Input:** `input: string` (raw value from recipient field)

**Behavior:**
1. Detect ENS input: string ends with `.eth`
2. Debounce 500ms before calling API
3. Query via `useQuery` with key `["ens", debouncedName]`
4. `enabled: false` when input is a raw address or empty
5. `staleTime: 5 * 60 * 1000` (5 minutes)

**Return type:**

```ts
{
  resolvedAddress: `0x${string}` | null
  isResolving: boolean
  ensError: string | null
  isEnsInput: boolean
}
```

## Transfer Page Changes

**File:** `packages/ui/app/transfer/page.tsx`

### Recipient input area

Three states below the input:
- **Resolving:** small spinner + "Resolving ENS name..."
- **Resolved:** truncated address (e.g. `0xd8dA...6045`), muted text
- **Error:** red error text matching existing `recipientError` style

### Validation logic

Current: `isAddress(to)`
New: `isAddress(to) || resolvedAddress !== null`

The `recipientError` field expands: if `isEnsInput && !isResolving && ensError`, show `ensError`.

### Transaction target

```ts
const effectiveRecipient = resolvedAddress ?? (to as `0x${string}`);
```

All references to `to as 0x${string}` in gas estimation and transaction sending use `effectiveRecipient` instead.

### Button label

Two new cases:
- `isEnsInput && isResolving` -> "Resolving ENS..."
- `isEnsInput && ensError` -> "Invalid ENS name"

## Files touched

| File | Change |
|------|--------|
| `packages/ui/app/api/ens/resolve/route.ts` | New API route |
| `packages/ui/hooks/useEnsResolve.ts` | New hook |
| `packages/ui/app/transfer/page.tsx` | Integrate hook, update validation and tx target |
| `.env.example` | Document `ETH_MAINNET_RPC_URL` (already done) |

## Out of scope

- ENS avatar or other metadata resolution
- Reverse resolution (address to ENS name)
- Support for non-.eth TLDs
- Client-side ENS resolution
