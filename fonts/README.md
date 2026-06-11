# Fonts

## Inter (body + UI)
Loaded via Google Fonts in `colors_and_type.css`:
```css
@import url('https://fonts.googleapis.com/css2?family=Inter:wght@300;400;500;600;700&display=swap');
```
No local files needed — Inter ships from Google Fonts.

## Plain Black (display) — SUBSTITUTED
Plain Black is licensed exclusively to Clay and not available as a public web font.

**Substitute in use:** Inter weight 500 with negative letter-spacing (-2.5px at 72px → -0.5px at 32px). This is documented in the spec's "Note on Font Substitutes" and gives the closest visual match without licensing.

**To restore the real face:**
1. Drop `PlainBlack.woff2` into this folder.
2. Add to the top of `colors_and_type.css`:
   ```css
   @font-face {
     font-family: 'Plain Black';
     src: url('fonts/PlainBlack.woff2') format('woff2');
     font-weight: 500;
     font-display: swap;
   }
   ```
3. Update `--font-display` in `:root`:
   ```css
   --font-display: 'Plain Black', 'Inter', -apple-system, sans-serif;
   ```

Other near-substitutes (if licensed): Söhne Breit (Buch), Recoleta (500).
