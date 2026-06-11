# Marketing Site UI Kit

Hi-fi recreation of the Clay marketing homepage — top nav, hero, feature card grid, pricing tiers, CTA band, and footer. Includes a working signup modal.

## Files
- `index.html` — interactive page; click "Try free" to see the signup flow.
- `components.jsx` — all components: `TopNav`, `Hero`, `FeatureCard`, `FeatureGrid`, `ProductMockup`, `PricingTier`, `PricingBand`, `CTABand`, `Footer`, `PrimaryButton`, `SecondaryButton`, `SignupModal`.

## Coverage
| Component | Notes |
|---|---|
| `TopNav` | 64px cream nav, logo + 5 menu items + Sign in + primary CTA |
| `Hero` | 7/5 split: h1 + sub + CTAs left, 3D illustration card right |
| `FeatureCard` | All 6 variants: pink, teal, lavender, peach, ochre, cream |
| `ProductMockup` | Small product UI fragment shown inside feature cards |
| `PricingTier` | Standard + featured (teal) variants |
| `CTABand` | Cream band with mascot illustration + CTAs |
| `Footer` | Cream footer (NOT dark), 4-column links |
| `SignupModal` | 2-step flow demonstrating modal patterns |

## Caveats
- 3D claymation illustrations are placeholders (SVG approximations). Replace `assets/illustrations/*.svg` with the real rendered assets.
- "Plain Black" display face substituted with Inter weight 500 + negative letter-spacing.
- No customer-logo strip, no testimonial carousel, no integration grid — out of scope for the first kit pass. Easy to add following the same component patterns.
