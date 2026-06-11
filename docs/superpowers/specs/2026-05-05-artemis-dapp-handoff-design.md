# Artemis Dapp Handoff Design

## Context

The design handoff at
`https://api.anthropic.com/v1/design/h/Of9LE3B-9daUNZN2rrzdvQ?open_file=Artemis+Dapp.html`
exports a prototype for an Artemis dapp with Home, Transfer, and Explorer
screens. The README requires reading the chat transcript and the active
`Artemis Dapp.html` file before implementation.

The transcript establishes the target:

- Hybrid landing page and connected dapp experience.
- Audience: institutional users and DeFi power users.
- Navigation: Home, Transfer, Explorer.
- Transfer: multi-asset, gasless-fee presentation.
- Explorer: block and transaction explorer similar to Etherscan.
- Visual direction: full Clay treatment with cream canvas, saturated cards,
  modern crypto details, and polished dapp affordances.

The current repo already has a Next.js UI under `packages/ui` with wagmi,
RainbowKit, Tailwind, existing routes for `/`, `/transfer`, `/blockexplorer`,
and Clay design tokens/components. The prototype is a visual source, not
production code.

## Decision

Implement the handoff with maximum visual fidelity to `Artemis Dapp.html`,
while letting repository facts override prototype facts when they conflict.

The main known conflict is chain identity:

- Repo: Artemis Chain ID `322`, token `ART`, 18 decimals.
- Prototype: uses demo values such as Chain ID `8442` and mock RPC URLs.

Implementation must use Artemis facts from the repo for network identity.
Demo metrics and mock token balances may remain where they support
pixel-matching the prototype.

## Architecture

Keep the existing Next.js route structure:

- `/`: Home screen matching the prototype hero, live network feed, stats strip,
  feature cards, and developer band.
- `/transfer`: Transfer screen matching the prototype multi-asset send form,
  token picker, summary, success state, portfolio value, and asset list.
- `/blockexplorer`: Explorer screen matching the prototype search, overview
  cards, latest block list, transaction list with filters, and validator table.

Update shared layout:

- `components/layout/Header.tsx` should adopt the prototype top-nav visual
  language: Artemis mark/wordmark, Mainnet pill, route tabs, utility links,
  and wallet control area.
- `components/layout/Footer.tsx` should adopt the prototype footer structure
  and update chain facts to Chain ID `322`.

Do not remove existing routes such as `/debug` or admin-only navigation. They
may remain accessible, but the handoff scope is Home, Transfer, and Explorer.

## Components

Add small, focused dapp components instead of placing the entire prototype into
page files:

- `components/dapp/LiveFeed.tsx`
  - Owns the mock ticking network feed.
  - Exposes block, TPS, and recent transaction values.
  - Cleans up timers in `useEffect`.
- `components/dapp/TokenPicker.tsx`
  - Owns token metadata, token icon display, picker list, and selection.
  - Keeps token data typed with no `any`.
- `components/dapp/ExplorerPanels.tsx`
  - Owns overview cards, latest blocks, latest transactions, filters, and
    validator table.
- `components/dapp/DappPanel.tsx` or equivalent small primitives
  - Provides panel/card treatments that match the prototype more closely than
    the current broad Clay hero/card components where needed.

Reuse existing UI primitives and Clay tokens where they fit. Prefer Tailwind
classes and CSS tokens in `globals.css` over copying inline style objects from
the prototype.

## Data Flow

Use wagmi/RainbowKit for real wallet connection state:

- Header wallet UI should reflect the connected state from wagmi.
- Transfer CTA should require a connected wallet before sending.
- Connected address/balance should be real where the existing hooks provide it.

Use prototype-style demo data for visual fidelity:

- Home live feed increments block and varies TPS on an interval.
- Recent transactions are generated mock rows for the feed and explorer.
- Explorer validator table uses the prototype's static demo validator list.
- Transfer token list and portfolio values use static demo token balances.

Transfer behavior:

- Validate recipient with `viem/isAddress`.
- Validate positive numeric amount.
- Validate amount against the selected demo token balance.
- Submit produces a success state and demo transaction hash.
- Do not perform real multi-token transfers in this scope.

Explorer search:

- Render the prototype search bar and accept input.
- The Search button is visual/demo in this scope and does not need detail-page
  navigation.

## Visual Requirements

Match the handoff's visible structure:

- Cream canvas background.
- Sticky 64px top navigation.
- Saturated single-color cards: pink, teal, lavender, peach, ochre, mint.
- Large display headlines with Inter substitute treatment.
- Mono details for chain data, addresses, hashes, RPC/code snippets.
- Rounded cards and panels matching the prototype's radii.
- Live pulse dots and subtle feed row animation.
- Footer with Network, Build, Use, and Company columns.

Responsive behavior must be production-safe:

- Desktop layouts should match the prototype grid structure.
- Tablet/mobile layouts should collapse grids without horizontal overflow.
- Long addresses and hashes should truncate or wrap intentionally.
- Text must not overlap or escape controls.

## Error Handling

Transfer must expose clear button states:

- `Connect wallet to send`
- `Enter recipient`
- `Invalid address`
- `Enter amount`
- `Insufficient balance`
- `Submitting...`
- `Send <amount> <token>`

Inputs should keep local error state simple and visible. Timers must clean up.
No production `console.log` statements.

## Testing And Verification

Run at minimum:

```bash
pnpm --filter @artemis/ui build
```

If additional UI test or lint scripts become available, run them as well.

After implementation, start the local Next.js dev server and provide the URL so
the UI can be inspected. Verify `/`, `/transfer`, and `/blockexplorer` render
without runtime errors.

Because this is a meaningful TypeScript/UI change, invoke `typescript-reviewer`
after implementation. Invoke `security-reviewer` if the final patch touches real
transaction submission, wallet authorization, or external request handling.

## Acceptance Criteria

- `/` visually matches the prototype Home screen structure.
- `/transfer` visually matches the prototype Transfer screen structure.
- `/blockexplorer` visually matches the prototype Explorer screen structure.
- Header and footer match the prototype direction.
- Artemis Chain ID is `322` wherever chain facts are shown.
- TypeScript remains strict with no `any`.
- Build passes for `@artemis/ui`.
