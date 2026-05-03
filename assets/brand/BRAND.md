# Puerto — Brand Assets

## Files

| File                 | Size     | Use                                    |
| -------------------- | -------- | -------------------------------------- |
| `logo-icon.svg`      | 64×64    | Favicon, GitHub profile, app icon      |
| `logo-full.svg`      | 420×80   | README, docs (light background)        |
| `logo-full-dark.svg` | 420×80   | README dark mode, website header       |
| `github-social.svg`  | 1280×640 | GitHub social preview (convert to PNG) |
| `favicon.svg`        | 32×32    | Browser tab, simplified icon           |

## Color Palette

| Token                 | Hex       | Use                                 |
| --------------------- | --------- | ----------------------------------- |
| `puerto-deep`         | `#0D1B2A` | Main background (dark)              |
| `puerto-navy`         | `#1E3A5F` | Ship hull, secondary bg, cards      |
| `puerto-steel`        | `#2D5986` | Borders, deck, UI chrome            |
| `puerto-teal`         | `#0891B2` | Business layer, primary accent      |
| `puerto-indigo`       | `#6366F1` | Infrastructure layer, links         |
| `puerto-indigo-light` | `#818CF8` | Infrastructure (on dark bg)         |
| `puerto-rust`         | `#F97316` | Presentation layer, CTA, accent dot |
| `puerto-ice`          | `#F1F5F9` | Main text (on dark bg)              |
| `puerto-mist`         | `#94A3B8` | Secondary text, badges              |

## Container Color Meaning

The 3 container colors map directly to Puerto's 3 workspace crates:

- **Teal `#0891B2`** → `business/` (domain + application)
- **Indigo `#6366F1`** → `infrastructure/`
- **Orange `#F97316`** → `presentation/`

## Typography

- **Logo / Code elements**: JetBrains Mono Bold — `font-family: 'JetBrains Mono', 'Fira Code', ui-monospace, monospace`
- **Body / Prose**: System UI — `font-family: system-ui, -apple-system, sans-serif`

## Converting SVG → PNG

For GitHub social preview (requires `rsvg-convert` or `sharp`):

```bash
# Using rsvg-convert (apt install librsvg2-bin)
rsvg-convert -w 1280 -h 640 github-social.svg > github-social.png

# Using Inkscape
inkscape --export-png=github-social.png -w 1280 -h 640 github-social.svg

# Using sharp-cli (npx)
npx sharp-cli --input github-social.svg --output github-social.png
```

## Usage in README

```markdown
<p align="center">
  <img src="assets/brand/logo-full.svg" alt="Puerto" width="300"/>
</p>
```

For dark/light mode aware README (GitHub supports `#gh-dark-mode-only` / `#gh-light-mode-only`):

```markdown
<p align="center">
  <img src="assets/brand/logo-full.svg#gh-light-mode-only" alt="Puerto" width="300"/>
  <img src="assets/brand/logo-full-dark.svg#gh-dark-mode-only" alt="Puerto" width="300"/>
</p>
```

## Tagline

> **Scaffold. Structure. Ship.**

## Do's and Don'ts

- **Do** maintain clear space around the logo equal to the icon height
- **Do** use the dark variant on dark backgrounds
- **Don't** change the container colors (they have semantic meaning)
- **Don't** use the logo smaller than 120px wide (full) or 24px (icon only)
- **Don't** add effects, gradients, or drop shadows to the logo
