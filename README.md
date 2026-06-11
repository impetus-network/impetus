# Impetus / Impulse

An EVM-compatible Substrate solochain built on Frontier, with precompile extensibility, an on-chain indexer, and a Next.js UI. Ships as a pnpm + Turborepo monorepo so the node, Solidity contracts, shared types, indexer, and frontend can be developed and built together. The node binary embeds two runtime WASM blobs and dispatches by chain spec id.

- **Mainnet:** Impetus — chain id `388266`, token `IPT` (18 decimals), SS58 prefix `11434`, runtime `spec_name="impetus"`.
- **Testnet:** Impulse — chain id `322644`, token `IPL` (18 decimals), SS58 prefix `11348`, runtime `spec_name="impulse"`.
- **Dev mode:** alias of Impulse with manual seal enabled and pre-funded Hardhat dev users (`--chain dev`).
- **Consensus:** Aura + GRANDPA (manual seal available via `--sealing` for dev).
- **EVM compatibility:** Frontier (`pallet-evm`, `pallet-ethereum`).

## Quick start

Bring up a local dev node, compile the contracts, and run the E2E test suite against the live node in under a few minutes.

```bash
# 1. Install JS deps (Node 22, pnpm 10+)
pnpm install

# 2. Build the Substrate node (Rust toolchain with wasm32-unknown-unknown target required)
cd apps/node && cargo build --release && cd ../..

# 3. Start a dev node in one terminal
./scripts/run-dev.sh

# 4. In another terminal, build the TS workspace and run E2E tests against the live node
pnpm turbo build
cd packages/contracts && pnpm test
```

The dev node exposes JSON-RPC at `http://127.0.0.1:9944` and WebSocket at `ws://127.0.0.1:9944`. Any Ethereum wallet configured for chain `322644` can connect directly.

## Repository layout

```
apps/
  node/       Substrate node (Rust) -- Frontier solochain with custom pallets & precompiles
  ui/         Next.js frontend (wagmi + RainbowKit + Tailwind v4)
  indexer/    Ponder indexer (TypeScript) for chain data
packages/
  contracts/  Solidity contracts, precompile interfaces, Hardhat E2E tests
  shared/     Shared constants, TypeScript types, and ABIs consumed by ui + indexer
scripts/
  run-dev.sh      Start a single-validator dev node (instant seal, ephemeral data)
  run-testnet.sh  Start a persistent two-validator local testnet (Alice / Bob)
```

Turborepo orchestrates `build`, `test`, and `lint` across the TypeScript packages; Cargo owns the node build.

## Prerequisites

- **Node.js** 22 (see `.nvmrc`)
- **pnpm** 10.8+ (declared in root `package.json`)
- **Rust** stable toolchain with the `wasm32-unknown-unknown` target for building the runtime
- A standard Substrate build environment (clang, libssl, pkg-config, protobuf)

## Running a node

Two convenience scripts live in `scripts/`. Both auto-build the node binary on first run.

### Dev node — `./scripts/run-dev.sh`

Single-validator, `--dev` chain with instant seal and ephemeral storage. Ideal for wallet wiring, contract testing, and local iteration.

- RPC: `http://127.0.0.1:9944`
- WS: `ws://127.0.0.1:9944`
- Chain ID: `322644`

### Local testnet — `./scripts/run-testnet.sh`

Two-validator local testnet (Alice + Bob) using persistent data under `.chain-data/`.

```bash
./scripts/run-testnet.sh            # Start Alice on :9944
./scripts/run-testnet.sh --bob      # Start Bob on :9945, connecting to Alice
./scripts/run-testnet.sh --purge    # Wipe chain data for the selected validator
```

## Sudo / admin account

The Impetus sudo key is account #0 of a project-specific BIP39 mnemonic. The
mnemonic is consumed by the node, contracts deploys, oz-relayer, oz-monitor
handlers, and any service that needs to sign privileged calls. Account #0 is
pre-funded with 1,000,000 IPL at genesis.

| Index | Address                                      | Role          |
|-------|----------------------------------------------|---------------|
| #0    | `0xd2aE0A2139dC83Cb920e3cd7B9F640922D14b872` | sudo / admin  |

The mnemonic itself is **not** committed. Provide it via env var (`ADMIN_MNEMONIC`)
or per-service key file. Generate a fresh one with:

```bash
cast wallet new-mnemonic --words 24 --accounts 1
```

Copy `.env.example` to `.env` and populate `ADMIN_MNEMONIC`, `ADMIN_ADDRESS`,
`ADMIN_PRIVATE_KEY` before running anything that signs admin transactions.

## Dev users

Pre-funded helper accounts derived from the Hardhat mnemonic. They have no
privileged role and exist purely so wallets seeded from the canonical Hardhat
mnemonic and the contracts E2E suite have funded balances on the dev chain.

Mnemonic: `test test test test test test test test test test test junk`

| Index | Address                                      | Role         |
|-------|----------------------------------------------|--------------|
| #0    | `0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266` | dev user     |
| #1    | `0x70997970C51812dc3A010C7d01b50e0d17dc79C8` | dev user     |
| #2    | `0x3C44CdDdB6a900fa2b585dd299e03d12FA4293BC` | dev user     |
| #3    | `0x90F79bf6EB2c4f870365E785982E1f101E93b906` | dev user     |
| #4    | `0x15d34AAf54267DB7D7c367839AAf71A00a2C6A65` | dev user     |

## Packages

### `apps/node` — Substrate node

Frontier-based solochain runtime with custom pallets and precompiles.

- `pallet-gasless-registry` — admin-managed registry of EVM function selectors that are exempt from gas fees
- `pallet-evm` + `pallet-ethereum` — Ethereum compatibility layer
- `pallet-assets` — fungible token support
- `precompiles/gasless-registry` — Solidity-callable interface to the gasless registry

See `apps/node/README.md` for node-specific build and runtime notes.

### `packages/contracts` — Solidity + Hardhat

Solidity interfaces, a `TestToken.sol` fixture, and a TypeScript/Hardhat E2E suite that exercises the live dev node over JSON-RPC.

```bash
cd packages/contracts
pnpm build    # hardhat compile (produces artifacts + typechain)
pnpm test     # hardhat test (requires dev node running)
```

### `packages/shared` — Shared TypeScript package

Chain constants, types, and precompile ABIs consumed by the UI and indexer. Built to `dist/` via `tsc` and linked via `workspace:*`.

### `apps/ui` — Next.js frontend

Next.js 15 + React 19 + wagmi + RainbowKit + Base UI + Tailwind v4.

```bash
cd apps/ui
pnpm dev          # Next dev server (port 3001)
pnpm build        # Production build
pnpm start        # Run production server
```

### `apps/indexer` — Ponder indexer

Indexes Artemis chain events into a queryable store and exposes a Hono API surface.

```bash
cd apps/indexer
pnpm dev    # ponder dev (watches contracts + config)
pnpm start  # ponder start (production mode)
```

## Common tasks

From the repo root:

```bash
pnpm install             # Install all JS deps
pnpm turbo build         # Build all TS packages (shared -> contracts -> ui -> indexer)
pnpm turbo test          # Run tests across the workspace
pnpm turbo lint          # Lint everything
```

Node builds are driven by Cargo, not Turbo:

```bash
cd apps/node && cargo build --release
```

## Conventions

- **Languages:** Rust for node/pallets/precompiles; TypeScript (strict) everywhere else
- **TypeScript:** no `any`, named imports, 2-space indent
- **Data:** prefer immutable structures; no `console.log` in production code
- **Organization:** feature/domain first, not type first
- **Commits:** `<type>(<scope>): <subject>` (`feat`, `fix`, `chore`, `docs`, …); keep commits atomic and split by concern

## Connecting a wallet

Point any EVM wallet (MetaMask, Rabby, etc.) at:

- RPC URL: `http://127.0.0.1:9944`
- Chain ID: `322644`
- Currency symbol: `IPL`
- Decimals: `18`

Import any dev account by private key derived from the Hardhat mnemonic above
to get 1,000,000 IPL at genesis. To act as sudo, import account #0 of your
`ADMIN_MNEMONIC` instead (`0xd2aE0A2139dC83Cb920e3cd7B9F640922D14b872` for the
default genesis pinning).


# Clay Design System

A vibrant claymation-meets-data interface for **Clay.com** — a GTM (go-to-market) data-orchestration platform. Designs anchor on cream-tinted white canvas with dark-navy primary CTAs, custom rounded display type, and saturated single-color feature cards (hot pink, deep teal, lavender, peach, ochre) that punctuate long-scroll explainer pages. Brand voltage comes from 3D-rendered claymation illustrations (mountains, characters, mascots) used as full-bleed hero artifacts.

> **Note on the brief.** The chat brief named the company "Dapp", but the only attached design specification (`uploads/DESIGN.md`) describes Clay.com. We've followed the attached spec. If you intended a different company, re-attach the relevant design materials and I'll rebuild.

---

## Sources

- **`uploads/DESIGN.md`** — Authoritative design tokens, components, and usage rules. Imported as the basis for `colors_and_type.css`, all preview cards, and the marketing UI kit.
- **No codebase, Figma URL, or slide deck was provided.** The recreations in `ui_kits/` are based on the spec's documented component anatomy + general knowledge of the Clay marketing surface.

---

## Index

| Path | What it is |
|---|---|
| `README.md` | This file. Visual foundations, content tone, iconography, manifest. |
| `SKILL.md` | Agent Skill descriptor — read by Claude Code if downloaded as a skill. |
| `colors_and_type.css` | All design tokens (CSS variables) + typography utility classes + element defaults. Import this into any HTML you build for the brand. |
| `fonts/` | Font notes. Inter ships from Google Fonts (loaded via `@import` in the CSS); Plain Black is a Clay-licensed face — substituted with Inter weight 500 + negative letter-spacing. |
| `assets/` | Logos, illustration placeholders, mascot SVGs. |
| `preview/` | Card files that populate the Design System tab. One concept per card. |
| `ui_kits/marketing-site/` | Hi-fi recreation of the Clay marketing homepage. JSX components + interactive `index.html`. |

---

## Visual Foundations

### Atmosphere
- **Cream canvas, throughout.** Every surface — page background, hero, footer — sits on `--canvas` (#fffaf0) or close cousins (`--surface-soft` #faf5e8, `--surface-card` #f5f0e0). No dark footer, no cool-gray sections. The warm tint is the brand's most distinctive gesture in a category dominated by cool grays.
- **Saturated feature cards.** Long-scroll pages alternate hot pink → teal → lavender → peach → ochre → cream. Same color twice in a row reads as off-rhythm.
- **3D claymation illustrations.** Mountains, mascot characters, abstract clay shapes rendered with hand-crafted depth. The brand's most-recognized visual element — used as full-bleed hero artifacts and inline figures inside feature cards.

### Color
- **Primary CTAs:** near-black (`--primary` #0a0a0a). Active state nudges to #1f1f1f; disabled flattens to a hairline gray.
- **6 accent cards:** pink, teal, lavender, peach, ochre, cream. Text flips: white on pink/teal; ink-black on lavender/peach/ochre/cream.
- **Semantic** (`--success` / `--warning` / `--error`): tailwind-grade green/amber/red, used sparingly for product UI inside feature cards.

### Type
- **Display:** Plain Black (custom rounded) at weight **500** with **negative letter-spacing** (-2.5px at 72px → -0.5px at 32px). Substituted with Inter 500 + negative tracking — see "Font substitution" below.
- **Body / UI:** Inter at 400 (body), 500 (nav), 600 (titles, buttons, badge labels).
- **Display weight stays at 500.** Going to 700 reads as bombastic; the rounded character of Plain Black gives warmth without bolder weight. Rule: Plain Black for headlines, Inter for everything else, never mix.

### Spacing
- 4px base unit. Tokens at 4 / 8 / 12 / 16 / 24 / 32 / 48 / **96**.
- **96px between major editorial bands.** This is non-negotiable rhythm for marketing pages.
- **32px** card-internal padding for saturated feature cards; **24px** for testimonial / product mockup cards.

### Backgrounds
- **No gradients.** Flat color throughout.
- **No repeating patterns or textures.** The visual interest is in the illustrations and the saturated cards.
- **3D illustrations** as full-bleed artifacts inside `--surface-soft` rounded containers (24px radius).

### Borders
- **1px hairline** (`--hairline` #e5e5e5) on inputs, product mockup cards, secondary buttons. That's the entire border vocabulary.
- **No colored borders, no left-accent stripes, no double-borders.**

### Corners
- **6px** small badges → **8px** small buttons → **12px** standard buttons + inputs → **16px** content cards → **24px** saturated feature cards → **9999px** pills + avatars.
- The bigger radius on feature cards matches Plain Black's rounded character — it's a deliberate echo.

### Shadows / Elevation
- **No heavy shadows.** Depth comes from saturated color contrast (bright card on cream canvas).
- A **soft drop shadow** is permitted on hover-elevated states only, and rarely.
- **3D claymation** illustrations carry their own rendered depth — the system doesn't try to compete with synthetic shadows.

### Animation
- The spec doesn't formalize timings. Convention: **gentle fades and translate-Y entrance** on scroll for feature cards; **subtle parallax** for 3D illustrations; **no bounces, no spring overshoots, no aggressive easing.** Hover transitions use `150ms ease-out`. Press states are a `transform: scale(0.98)` with no color flash.

### Hover & Press
- **Buttons (primary):** background nudges from `--primary` to `--primary-active` (a hair lighter); no scale.
- **Buttons (on-color cards):** white background dims to ~92% opacity.
- **Cards:** lift with `--shadow-hover` (the rare case shadows appear).
- **Press:** `transform: scale(0.98)` for 80ms.

### Layout
- **Max content width** 1280px, centered.
- **Hero** uses a 7/5 split (h1 + sub + CTAs left, illustration right) on desktop; stacks on mobile.
- **Feature card grids** 3-up desktop, 2-up tablet, 1-up mobile.
- **Top nav** is 64px, fixed-cream — it does NOT go transparent over hero.

### Transparency & Blur
- **None used in the system.** No frosted glass, no backdrop-filter, no semi-transparent overlays. The cream canvas is opaque throughout.

### Imagery vibe
- **3D claymation:** warm-leaning palette (peach, ochre, mint, coral) regardless of subject. Soft global illumination, no harsh shadows. Reads as hand-crafted, not photorealistic.
- **Product UI fragments:** shown at small scale inside cards with a 1px hairline border. Real screens (agent runs, sequencer flows, enrichment tables), never wireframes.

---

## Content Fundamentals

### Voice
Clay's copy is **plainly confident, slightly playful, never cute**. Headlines do real work — they make a benefit claim, not a riff. The tone is closer to a sharp PM explaining a product than a copywriter chasing virality.

### Person & Address
- **Second person ("you", "your")** is the default for marketing copy. *"Build your dream GTM stack."*
- **First person plural ("we") rarely** — only in trust passages (about, careers).
- Imperative verbs lead headlines: "Go to market with…", "Build with…", "Start free."

### Casing
- **Sentence case** for headlines and button labels: "Try free", "Start with a template", "Book a demo".
- **UPPERCASE** reserved for the `caption-uppercase` token (12px, +1.5px tracking) — used as section eyebrows ("FEATURED", "INTEGRATIONS").
- **Title Case** is **avoided.** Even product names follow sentence-case patterns ("Claygent", not "ClayGent").

### Length & Density
- **Hero h1:** 5–9 words. *"Go to market with unique data."*
- **Sub-headline:** one sentence, 12–20 words.
- **Feature card body:** 1–3 sentences max. The colored card does the heavy lifting; copy adds specifics.
- **CTAs:** 1–3 words. "Try free", "Book a demo", "See pricing".

### Vibe
- Confident, specific, technically literate. Mentions integrations and product nouns directly ("CRM enrichment", "outbound sequencer", "AI research agent").
- **Light wit** in feature card titles is permitted: *"Tell our AI to do (almost) anything."*
- **No emoji.** No exclamation points outside genuine celebration moments (Done! Welcome aboard!).
- **No buzzword slop.** Avoid "revolutionize", "unleash", "supercharge".

### Examples
- Hero: *"Go to market with unique data."*
- Sub: *"Clay combines 100+ data providers, AI research agents, and the world's best go-to-market experts to help you turn ideas into pipeline."*
- Feature card title: *"Sequence outbound with the data you actually have."*
- CTA: *"Try free"*

---

## Iconography

Clay does **not** ship a recognizable open-source icon system in its marketing surface. The visual heavy-lifting is done by 3D claymation illustrations, not icons.

### What we observe
- **3D claymation illustrations** carry ~80% of the visual load. Mountains, mascot characters, peach/ochre/lavender clay scenes. These are commissioned, not from a library.
- **Small UI icons** (chevrons, checkmarks, arrows in CTAs, logos in integration grids) are **simple line icons at 1.5px stroke**. Style is closest to **Lucide** or **Phosphor** — geometric, rounded line endings, not Material's filled aesthetic.
- **Integration tile icons** are the actual brand logos (Salesforce, HubSpot, Slack, etc.) shown as colored SVG tiles, not generic glyphs.
- **No emoji** in marketing copy.
- **No unicode glyphs** as decorative icons.
- **No icon font.** Icons are inline SVG.

### Substitution
For prototyping where a real icon system isn't established, **use [Lucide](https://lucide.dev)** via CDN at 1.5px stroke. It matches the geometric, rounded-endings style closest. If a designer asks "what icon should I use?", the answer is Lucide unless there's a specific brand-logo case (integration tile, social link).

```html
<!-- Load Lucide via CDN -->
<script src="https://unpkg.com/lucide@latest"></script>
<i data-lucide="arrow-right" style="width:16px;height:16px;stroke-width:1.5;"></i>
<script>lucide.createIcons();</script>
```

### Logo
- The **Clay wordmark** + mark sit in `assets/`. Wordmark is set in Plain Black; mark is a stylized rounded-square clay tile. We've created a clean SVG approximation in `assets/clay-logo.svg`.

### Flagged substitutions
- **Plain Black** (display face) — substituted with **Inter weight 500 + negative letter-spacing**. *Please provide the licensed Plain Black `.woff2` if you have it; drop into `fonts/` and update the `@font-face` block in `colors_and_type.css`.*
- **3D claymation illustrations** — placeholder SVGs in `assets/illustrations/`. *Please provide the actual rendered illustration assets (or Figma access) when available.*
- **Mascot characters** — not provided; we've created neutral abstract placeholders. *Mascot lineage is not formalized in the spec.*

---

## Token Quick-Reference

```css
/* Color */
--canvas: #fffaf0;       --primary: #0a0a0a;
--brand-pink: #ff4d8b;   --brand-teal: #1a3a3a;
--brand-lavender: #b8a4ed;  --brand-peach: #ffb084;
--brand-ochre: #e8b94a;  --surface-card: #f5f0e0;

/* Type — class utilities */
.t-display-xl  /* 72px / 500 / -2.5px */
.t-title-md    /* 18px / 600 */
.t-body-md     /* 16px / 400 / 1.55 */

/* Radius */
--radius-md: 12px;   /* buttons + inputs */
--radius-lg: 16px;   /* content cards */
--radius-xl: 24px;   /* feature cards */

/* Spacing */
--space-section: 96px;  /* between major bands */
--space-xl: 32px;       /* feature card padding */
```

---

## How to use

```html
<link rel="stylesheet" href="colors_and_type.css">
<button style="background:var(--primary); color:var(--on-primary); padding:12px 20px; border-radius:var(--radius-md); border:none;" class="t-button">
  Try free
</button>
```

For React prototypes, see `ui_kits/marketing-site/` for component patterns.
