# Custom ConnectButton Design Spec

## Overview

Restyle RainbowKit ConnectButton to match Artemis theme. Keep all default
behavior and layout (avatar, address, balance, chain icon).

## Approach

Use RainbowKit `Theme` API — `lightTheme()` as base with overrides for
Artemis palette. No custom render markup needed.

## Files

- Create: `packages/ui/config/rainbowTheme.ts` — custom theme
- Modify: `packages/ui/components/providers/Web3Provider.tsx` — pass theme to RainbowKitProvider

## Theme Overrides

Override these RainbowKit theme tokens to match Artemis `globals.css`:

| RainbowKit token | Artemis value |
|------------------|---------------|
| accentColor | primary (oklch 55% 0.2 260) |
| accentColorForeground | primary-foreground |
| connectButtonBackground | background |
| connectButtonText | foreground |
| modalBackground | background |
| fontStack | system-ui, -apple-system, sans-serif |
| borderRadius | medium |

## Display Config

Keep default ConnectButton props (no overrides needed):
- `showBalance={true}`
- `chainStatus="icon"`
- `accountStatus="full"`

## Non-Goals

- Custom render markup (ConnectButton.Custom)
- Dark mode theme variant
- Mobile-specific layout changes
