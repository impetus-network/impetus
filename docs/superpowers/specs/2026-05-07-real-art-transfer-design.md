# Real ART Transfer

Replace the mock transfer flow with actual native ART token transfers using existing `useTransactor` hook.

## Context

The transfer page (`packages/ui/app/transfer/page.tsx`) currently fakes transactions with a `setTimeout` + `makeHash()`. It also displays mock tokens (USDC, USDT, WETH, WBTC, AGT) with hardcoded balances and USD prices. Only ART balance is real (via wagmi `useBalance`).

The codebase already has a production-ready `useTransactor` hook that handles gas estimation, confirmation dialogs, wallet signing, receipt tracking, and toast notifications.

## Goal

Wire the transfer page to send real native ART transactions. Remove all mock token data. Show real gas estimates.

## Scope

One file changes: `packages/ui/app/transfer/page.tsx`. No new files.

Hooks used as-is (no modifications):
- `useTransactor` — transaction lifecycle
- `useBalance` (wagmi) — ART balance
- `useAccount` (wagmi) — wallet connection
- `usePublicClient` (wagmi) — gas estimation
- `TxConfirmProvider` — already wrapping the app

## Design

### 1. Replace handleSend with real transaction

Current:
```ts
setSending(true);
timeoutRef.current = window.setTimeout(() => {
  const hash = makeHash(Date.now());
  setSuccess({ amount, token: "ART", hash });
  // ...
}, 900);
```

New:
```ts
setSending(true);
try {
  const hash = await transact({ to, value: parseEther(amount) });
  if (hash) {
    setSuccess({ amount: numericAmount, token: "ART", hash });
    setAmount("");
    setTo("");
    refetchBalance();
  }
} catch {
  // useTransactor already handles errors via toasts
} finally {
  setSending(false);
}
```

### 2. Remove mock tokens

- Remove `demoTokens`, `TokenPicker`, `DemoToken` imports
- Single token: ART, from `useBalance`
- Sidebar "Your assets" shows only ART with real balance
- Remove portfolio USD section (no price feed) — replace with ART balance display
- Remove `formatUsd`, `makeHash`, `shortHash` mock imports (keep `shortHash` if needed for tx hash display, or inline it)

### 3. Real gas estimation in summary

- Use `usePublicClient` to call `estimateGas` when form is valid (valid address + valid amount + sufficient balance)
- Debounce the estimate call with ~500ms delay to avoid spamming RPC on every keystroke
- Display in summary row: `0.00021 ART` or `--` when not yet estimatable
- Gas estimate updates live as user changes amount/recipient

### 4. Unchanged

- Page layout and visual design
- Amount input with MAX button
- Address input with validation
- `TxConfirmProvider` confirmation dialog (triggered by `useTransactor`)
- Toast notification flow
- Validation logic (isAddress, balance check, amount > 0)

## Edge Cases

| Case | Behavior |
|------|----------|
| Wallet not connected | Button shows "Connect wallet to send", click opens RainbowKit modal |
| Amount > balance | Button shows "Insufficient balance", disabled |
| Invalid address | Error text below input, button disabled |
| Transaction rejected by user | Toast from useTransactor, form stays filled |
| Transaction reverts | Toast from useTransactor, form stays filled |
| Gas estimation fails | Summary shows `--` for fee, send still allowed (wallet handles gas) |
| Zero balance | MAX button sets "0", send button disabled |
