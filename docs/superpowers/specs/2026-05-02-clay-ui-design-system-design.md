# Clay UI Design System Design Spec

## Overview

Restyle the Artemis UI using the existing `DESIGN.md` direction in a strict
Clay-inspired style. The app should feel like a bright tactile product surface:
cream canvas, dark ink, saturated accent cards, large rounded geometry, and
clear operational hierarchy.

The implementation keeps the app behavior unchanged. Wallet connection,
contract calls, Ponder reads, explorer fetching, validation, and routing remain
as they are today.

## Decisions

- Apply the style to the whole app.
- Follow `DESIGN.md` closely for colors, surfaces, radius, and component tone.
- Keep the existing Inter and Geist fonts. Do not add font files or font
  dependencies.
- Do not edit `packages/ui/components/ui`. Treat those components as reusable
  primitives.
- Create a Clay presentation layer that composes the existing primitives.

## Architecture

Styling will be organized in three layers.

### Global Tokens

Update `packages/ui/app/globals.css` to map `DESIGN.md` into Tailwind v4 theme
tokens:

- Cream canvas background.
- Dark ink foreground.
- Warm muted and card surfaces.
- Saturated accents: pink, teal, lavender, peach, ochre, mint, and coral.
- Larger radius values.
- Tactile border, focus ring, and shadow defaults.

Global tokens may affect existing primitives, but component-specific Clay
styling should not be placed inside `components/ui`.

### Clay Components

Add `packages/ui/components/clay/` as a small design-system layer. These
components wrap or compose the current headless/base UI primitives:

- `ClayPage`
- `ClayHero`
- `ClaySection`
- `ClayPanel`
- `ClayCard`
- `ClayFeatureCard`
- `ClayButton`
- `ClayBadge`
- `ClayTableFrame`
- Form helpers if they remove repeated class strings.

The layer should stay small. Add a component only when it removes repeated
styling or gives a clear semantic role.

### Page Composition

Update routes and local feature components to consume the Clay layer:

- Home: Clay hero and colored stat cards.
- Transfer: tactile form panel and clearer connected-wallet balance treatment.
- Block explorer: Clay page header, search/table frame, and block feature
  tiles.
- Debug: contract panels and segmented read/write control styled through the
  Clay layer.
- Admin gasless: admin status, rules table, add/check forms, and empty/loading
  states styled consistently.
- Layout: header/footer should use the cream canvas and dark ink navigation
  treatment.

## Data Flow

No data flow changes.

- Wallet state remains through wagmi and RainbowKit.
- Contract reads and writes remain through existing scaffold hooks.
- Ponder GraphQL remains the source for gasless rules.
- Explorer data remains through `useFetchBlocks`.
- Form state and validation remain local to the existing components.

## Error And Loading States

Existing error and loading behavior remains. The visual treatment changes:

- Empty states become small Clay panels instead of plain text.
- Loading states use styled muted panels or existing spinners where available.
- Validation errors stay attached to fields.
- Transaction states such as mining, sending, and estimating keep their current
  labels and disabled behavior.

No errors should be hidden by the styling layer.

## File Scope

Expected additions:

- `packages/ui/components/clay/*`

Expected updates:

- `packages/ui/app/globals.css`
- `packages/ui/components/layout/Header.tsx`
- `packages/ui/components/layout/Footer.tsx`
- `packages/ui/app/page.tsx`
- `packages/ui/app/transfer/page.tsx`
- `packages/ui/app/blockexplorer/page.tsx`
- `packages/ui/app/debug/page.tsx`
- `packages/ui/app/admin/gasless/page.tsx`
- Local feature presentation components where needed, such as admin forms,
  admin tables, debug panels, and explorer tables.

Files that should remain untouched:

- `packages/ui/components/ui/*`

## Non-Goals

- No changes to wallet behavior.
- No changes to contract calls, ABIs, or chain configuration.
- No changes to Ponder queries or indexer behavior.
- No new font assets.
- No new design dependency.
- No routing changes.
- No broad refactor outside the UI presentation layer.

## Risks

- Global token changes affect all existing primitives. Keep token names
  compatible with current Tailwind usage.
- Strict Clay radius and color can reduce density in admin views. Use compact
  Clay table frames for data-heavy surfaces.
- Existing raw `rounded-lg border` page classes may clash with the new style.
  Migrate obvious old-style wrappers during page touch-up.
- `DESIGN.md` references `Plain Black`, but the implementation keeps Inter and
  Geist. Typography should rely on weight, scale, and layout rather than a new
  font.

## Verification

Run:

```bash
pnpm --filter @artemis/ui build
```

Then manually inspect:

- `/`
- `/transfer`
- `/blockexplorer`
- `/debug`
- `/admin/gasless`

Check both mobile and desktop widths. Confirm no files under
`packages/ui/components/ui` changed.
