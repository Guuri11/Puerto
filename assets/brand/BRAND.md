# Puerto — Brand

The full visual contract (colours with measured contrast, type scale, motion, do's and don'ts) is
[`landing/DESIGN.md`](../../landing/DESIGN.md). This page is the short version for the repository.

**Every file here is generated** by `landing/scripts/repo-brand.mjs` from the same tokens as the
website. Change the script, not the files:

```bash
cd landing && pnpm brand:repo
```

## The idea

The standard shipping container made trade fast because every box is the same size. Puerto does
that to code: every project has the same shape. The symbol is **three identical containers**,
stacked the way a crane leaves them, one per layer of a Puerto project.

## Files

| File | Size | Use |
|---|---|---|
| `github-social.svg` / `.png` | 1280×640 | README header; upload the PNG in *Settings → Social preview* |
| `logo-full.svg` | 312×80 | Wordmark + symbol on light backgrounds |
| `logo-full-dark.svg` | 312×80 | The same on dark backgrounds |
| `logo-icon.svg`, `logo-icon-64.png`, `logo-icon-128.png` | 64×64 | Avatar, app icon |
| `favicon.svg`, `favicon.png` | 32×32 | Browser tab: the symbol without ribs, light and dark aware |

The SVGs embed the subset of the fonts they use, so they render the same wherever an `<img>`
shows them, GitHub included.

## Colour

| Token | Hex | Use |
|---|---|---|
| Concrete | `#E9E5DC` | Background |
| Ink | `#15171A` | Text, rules |
| Crane yellow | `#F5B700` | The one accent: calls to action, hazard stripes. Never text on light |
| Night | `#101215` | Code and terminal panels; the dark theme |

The three layer colours mean **which layer**, everywhere, and nothing else:

| Layer | Day | Night |
|---|---|---|
| `presentation` | `#2F6B4F` | `#6DBE93` |
| `infrastructure` | `#A33B20` | `#E58A6A` |
| `business` | `#1D4E89` | `#6FA3E0` |

## Type

- **Big Shoulders Stencil** — the wordmark and labels.
- **Big Shoulders** — headlines, upper case.
- **Archivo** — text.
- **Martian Mono** — code and commands.

All four are on Google Fonts (SIL Open Font License); the website self-hosts them.

## Using the logo in Markdown

```markdown
<p align="center">
  <img src="assets/brand/logo-full.svg#gh-light-mode-only" alt="Puerto" width="300"/>
  <img src="assets/brand/logo-full-dark.svg#gh-dark-mode-only" alt="Puerto" width="300"/>
</p>
```

## Don'ts

- Don't recolour the containers, or use the layer colours for anything but layers.
- Don't add anchors, waves, ships or any nautical decoration. This is a cargo port.
- Don't use the full symbol below 20 px; use `favicon.svg`, which drops the ribs.
- Don't add gradients, glows or shadows.
