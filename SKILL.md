---
name: clay-design
description: Use this skill to generate well-branded interfaces and assets for Clay (clay.com — GTM data-orchestration platform), either for production or throwaway prototypes/mocks/etc. Contains essential design guidelines, colors, type, fonts, assets, and UI kit components for prototyping.
user-invocable: true
---

Read the README.md file within this skill, and explore the other available files.

If creating visual artifacts (slides, mocks, throwaway prototypes, etc), copy assets out and create static HTML files for the user to view. Always link `colors_and_type.css` and use the CSS variables (`--canvas`, `--primary`, `--brand-pink`, etc.) — never inline raw hex codes.

If working on production code, you can copy assets and read the rules here to become an expert in designing with this brand.

If the user invokes this skill without any other guidance, ask them what they want to build or design, ask some questions, and act as an expert designer who outputs HTML artifacts _or_ production code, depending on the need.

## Quick rules
- Cream canvas (#fffaf0) on every surface — never dark, never cool gray. Cream footer too.
- Display headlines: Inter weight 500 + negative letter-spacing (substitute for Plain Black).
- Saturated feature cards alternate: pink → teal → lavender → peach → ochre → cream. Never repeat in a row.
- 96px between major editorial bands.
- Primary CTAs are near-black (#0a0a0a), 12px radius, 44px tall.
- Feature cards use 24px radius. Content cards 16px. Buttons/inputs 12px.
- No emoji, no gradients, no heavy shadows. Depth comes from saturated color contrast.
- 3D claymation illustrations are the brand voltage — use placeholders if real assets aren't provided and flag the substitution.
