# squirrelscan Audit Report

**URL:** https://puerto-rs.vercel.app  
**Date:** 2026-10-13T10:48:27.640Z  
**Pages:** 48  
Scan: 48 pages crawled from the CLI (page limit 100).  
Audit level: Surface.  
**Version:** 0.0.111

## Health Score

| Category | Score |
|----------|-------|
| **Overall** | **83/100 (B)** |
| **SEO** | **84/100** |
| **Performance** | **80/100** |
| **Security** | **95/100** |
| **Agents** | **50/100** |
| Accessibility | 79/100 |
| Content | 87/100 |
| Performance | 80/100 |
| Agent Experience | 50/100 |
| Core SEO | 84/100 |
| E-E-A-T | 52/100 |
| Links | 90/100 |
| Legal Compliance | 44/100 |
| Social Media | 85/100 |
| Security | 98/100 |
| Crawlability | 100/100 |
| Internationalization | 100/100 |
| Images | 100/100 |
| Site Integrity | 100/100 |
| Mobile | 100/100 |
| Structured Data | 100/100 |
| URL Structure | 100/100 |

## Technologies

_Detected tech stack — informational, not part of the score. No changes since last scan._

**🧩 Frameworks & Libraries**  
<img src="https://squirrelscan.com/tech-icons/astro.png" alt="Astro" width="14" height="14" /> Astro `7.3.8`

**☁️ Hosting**  
<img src="https://squirrelscan.com/tech-icons/vercel.png" alt="Vercel" width="14" height="14" /> Vercel

**🌐 CDN & Edge**  
<img src="https://squirrelscan.com/tech-icons/vercel.png" alt="Vercel Edge Network" width="14" height="14" /> Vercel Edge Network

## Summary

- **Passed:** 5331
- **Warnings:** 163
- **Failed:** 48
- **SEO:** 84/100
- **Performance:** 80/100
- **Security:** 95/100
- **Agents:** 50/100

---

## Issues

### Errors

*1 rule(s)*

#### Accessibility

##### Label Content Name Mismatch **[ERROR]**

`a11y/label-content-name-mismatch`

> Checks that visible label text is part of accessible name

**Solution:**

For controls with visible labels, the accessible name should contain the visible text. Voice control users say what they see - if the accessible name doesn't include the visible label, voice commands won't work. Example: A button showing 'Search' should not have aria-label='Find products'.

| Check | Status | Message |
|-------|--------|---------|
| label-content-name-mismatch | X fail | 1 element(s) where visible text doesn't match accessible name |

<details><summary><strong>label-content-name-mismatch:</strong> 47 page(s) affected</summary>

- [/docs/](https://puerto-rs.vercel.app/docs/)
- [/docs/changelog/](https://puerto-rs.vercel.app/docs/changelog/)
- [/docs/decisions/](https://puerto-rs.vercel.app/docs/decisions/)
- [/docs/decisions/adr/0001-enum-value-objects-implement-fromstr/](https://puerto-rs.vercel.app/docs/decisions/adr/0001-enum-value-objects-implement-fromstr/)
- [/docs/decisions/adr/0002-response-enums-allow-large-enum-variant/](https://puerto-rs.vercel.app/docs/decisions/adr/0002-response-enums-allow-large-enum-variant/)
- [/docs/decisions/adr/0003-unique-indexes-are-partial/](https://puerto-rs.vercel.app/docs/decisions/adr/0003-unique-indexes-are-partial/)
- [/docs/decisions/adr/0004-generate-use-case-never-overwrites/](https://puerto-rs.vercel.app/docs/decisions/adr/0004-generate-use-case-never-overwrites/)
- [/docs/decisions/adr/0005-sql-reserved-words-only-in-db-projects/](https://puerto-rs.vercel.app/docs/decisions/adr/0005-sql-reserved-words-only-in-db-projects/)
- [/docs/decisions/adr/0006-logging-through-tracing-not-a-logger-port/](https://puerto-rs.vercel.app/docs/decisions/adr/0006-logging-through-tracing-not-a-logger-port/)
- [/docs/decisions/adr/0007-error-contract-code-in-name-422/](https://puerto-rs.vercel.app/docs/decisions/adr/0007-error-contract-code-in-name-422/)
- [/docs/decisions/adr/0008-liveness-readiness-graceful-shutdown/](https://puerto-rs.vercel.app/docs/decisions/adr/0008-liveness-readiness-graceful-shutdown/)
- [/docs/decisions/adr/0009-generated-projects-pin-their-toolchain/](https://puerto-rs.vercel.app/docs/decisions/adr/0009-generated-projects-pin-their-toolchain/)
- [/docs/decisions/adr/0010-generated-agent-rules-are-tested/](https://puerto-rs.vercel.app/docs/decisions/adr/0010-generated-agent-rules-are-tested/)
- [/docs/decisions/adr/0011-templates-own-shape-types-own-decisions/](https://puerto-rs.vercel.app/docs/decisions/adr/0011-templates-own-shape-types-own-decisions/)
- [/docs/decisions/adr/0012-presentation-holds-use-case-ports/](https://puerto-rs.vercel.app/docs/decisions/adr/0012-presentation-holds-use-case-ports/)
- [/docs/decisions/adr/0013-web-frontend-react-daisyui-bun/](https://puerto-rs.vercel.app/docs/decisions/adr/0013-web-frontend-react-daisyui-bun/)
- [/docs/decisions/adr/0014-entity-ids-are-generated-newtypes/](https://puerto-rs.vercel.app/docs/decisions/adr/0014-entity-ids-are-generated-newtypes/)
- [/docs/decisions/adr/0015-repositories-return-one-repository-error/](https://puerto-rs.vercel.app/docs/decisions/adr/0015-repositories-return-one-repository-error/)
- [/docs/decisions/adr/0016-lists-are-offset-pages/](https://puerto-rs.vercel.app/docs/decisions/adr/0016-lists-are-offset-pages/)
- [/docs/decisions/adr/0017-runtime-crates-next-to-the-generator/](https://puerto-rs.vercel.app/docs/decisions/adr/0017-runtime-crates-next-to-the-generator/)
- [/docs/decisions/adr/0018-domain-events-outbox-pluggable-transport/](https://puerto-rs.vercel.app/docs/decisions/adr/0018-domain-events-outbox-pluggable-transport/)
- [/docs/decisions/adr/0019-generated-code-is-formatted-by-cargo-fmt/](https://puerto-rs.vercel.app/docs/decisions/adr/0019-generated-code-is-formatted-by-cargo-fmt/)
- [/docs/decisions/adr/0020-mail-port-per-mail-sent-from-the-worker/](https://puerto-rs.vercel.app/docs/decisions/adr/0020-mail-port-per-mail-sent-from-the-worker/)
- [/docs/decisions/adr/0021-auth-one-authenticator-port-oidc-or-local/](https://puerto-rs.vercel.app/docs/decisions/adr/0021-auth-one-authenticator-port-oidc-or-local/)
- [/docs/guides/agents/](https://puerto-rs.vercel.app/docs/guides/agents/)
- [/docs/guides/auth/](https://puerto-rs.vercel.app/docs/guides/auth/)
- [/docs/guides/configuration/](https://puerto-rs.vercel.app/docs/guides/configuration/)
- [/docs/guides/database/](https://puerto-rs.vercel.app/docs/guides/database/)
- [/docs/guides/entities/](https://puerto-rs.vercel.app/docs/guides/entities/)
- [/docs/guides/events/](https://puerto-rs.vercel.app/docs/guides/events/)
- [/docs/guides/layer-by-layer/](https://puerto-rs.vercel.app/docs/guides/layer-by-layer/)
- [/docs/guides/mail/](https://puerto-rs.vercel.app/docs/guides/mail/)
- [/docs/guides/testing/](https://puerto-rs.vercel.app/docs/guides/testing/)
- [/docs/guides/upgrading/](https://puerto-rs.vercel.app/docs/guides/upgrading/)
- [/docs/guides/use-cases/](https://puerto-rs.vercel.app/docs/guides/use-cases/)
- [/docs/guides/value-objects/](https://puerto-rs.vercel.app/docs/guides/value-objects/)
- [/docs/guides/web/](https://puerto-rs.vercel.app/docs/guides/web/)
- [/docs/reference/cli/](https://puerto-rs.vercel.app/docs/reference/cli/)
- [/docs/reference/errors/](https://puerto-rs.vercel.app/docs/reference/errors/)
- [/docs/reference/field-types/](https://puerto-rs.vercel.app/docs/reference/field-types/)
- [/docs/reference/make/](https://puerto-rs.vercel.app/docs/reference/make/)
- [/docs/reference/puerto-toml/](https://puerto-rs.vercel.app/docs/reference/puerto-toml/)
- [/docs/reference/runtime-crates/](https://puerto-rs.vercel.app/docs/reference/runtime-crates/)
- [/docs/start/anatomy/](https://puerto-rs.vercel.app/docs/start/anatomy/)
- [/docs/start/architecture/](https://puerto-rs.vercel.app/docs/start/architecture/)
- [/docs/start/installation/](https://puerto-rs.vercel.app/docs/start/installation/)
- [/docs/start/quick-start/](https://puerto-rs.vercel.app/docs/start/quick-start/)

</details>

<details><summary><strong>label-content-name-mismatch:</strong> 1 item(s)</summary>

- button: visible="ctrlk" vs aria-label="search"
  - from: [/docs/](https://puerto-rs.vercel.app/docs/)
  - from: [/docs/changelog/](https://puerto-rs.vercel.app/docs/changelog/)
  - from: [/docs/decisions/](https://puerto-rs.vercel.app/docs/decisions/)
  - +7 more (see full list above)

</details>

---

### Recommendations

*3 rule(s)*

#### Images

##### Inline SVG Size *[Recommendation]*

`images/svg-inline`

> Checks for large inline SVGs bloating HTML

**Solution:**

Large inline SVGs increase HTML size and block rendering. Move SVGs >4KB to external files and reference with <img> or CSS background. Inline small, critical SVGs (icons, logos) only. Use SVGO to optimize. Consider SVG sprites for icon sets. Inline SVGs can't be cached separately from HTML.

| Check | Status | Message |
|-------|--------|---------|
| svg-inline | ! warn | 1 to 15 large inline SVG(s) (>4KB each) |

<details><summary><strong>svg-inline:</strong> 4 page(s) affected</summary>

- [/docs/guides/entities/](https://puerto-rs.vercel.app/docs/guides/entities/)
- [/docs/guides/mail/](https://puerto-rs.vercel.app/docs/guides/mail/)
- [/docs/guides/web/](https://puerto-rs.vercel.app/docs/guides/web/)
- [/docs/start/anatomy/](https://puerto-rs.vercel.app/docs/start/anatomy/)

</details>

---

#### Agent Experience

##### AGENTS.md *[Recommendation]*

`ax/agents-md`

> Detects /AGENTS.md (and variants) — plain-Markdown instructions for coding agents working against the site's repository

**Solution:**

If the site has an associated codebase, publish an AGENTS.md at the repository root covering setup, testing, and conventions a coding agent needs to be productive. This is a recommendation only — it never affects your score.

| Check | Status | Message |
|-------|--------|---------|
| agents-md-present | ! warn | No AGENTS.md found — this site publishes llms.txt, so consider an AGENTS.md for coding agents too |

---

##### Markdown Response *[Recommendation]*

`ax/markdown-response`

> Checks whether the site serves text/markdown via content negotiation (Accept: text/markdown) or exposes a .md variant of the homepage — agents increasingly prefer clean Markdown over rendered HTML

**Solution:**

Agents and answer engines parse Markdown more reliably than rendered HTML. Serve a Markdown representation of your key pages: honor `Accept: text/markdown` via content negotiation, and/or publish a `.md` variant (e.g. /index.md). This is a recommendation only — it never affects your score.

| Check | Status | Message |
|-------|--------|---------|
| markdown-response | ! warn | No Markdown response — consider honoring Accept: text/markdown or publishing a .md variant so agents get clean content |

---

### Warnings

*20 rule(s)*

#### Core SEO

##### Meta Title **[WARN]**

`core/meta-title`

> Validates page title presence and length

**Solution:**

Every page needs a unique, descriptive title tag between 30-75 characters. Titles appear in browser tabs, search results, and social shares. Write titles that accurately describe the page content while including your primary keyword near the beginning. If your title is too short, add more descriptive context. If too long, prioritize the most important information first and trim secondary details. Avoid keyword stuffing or duplicate titles across pages.

| Check | Status | Message |
|-------|--------|---------|
| meta-title | ! warn | Title too short |

<details><summary><strong>meta-title:</strong> 22 page(s) affected</summary>

- [/docs/](https://puerto-rs.vercel.app/docs/)
- [/docs/changelog/](https://puerto-rs.vercel.app/docs/changelog/)
- [/docs/guides/agents/](https://puerto-rs.vercel.app/docs/guides/agents/)
- [/docs/guides/auth/](https://puerto-rs.vercel.app/docs/guides/auth/)
- [/docs/guides/entities/](https://puerto-rs.vercel.app/docs/guides/entities/)
- [/docs/guides/events/](https://puerto-rs.vercel.app/docs/guides/events/)
- [/docs/guides/layer-by-layer/](https://puerto-rs.vercel.app/docs/guides/layer-by-layer/)
- [/docs/guides/mail/](https://puerto-rs.vercel.app/docs/guides/mail/)
- [/docs/guides/testing/](https://puerto-rs.vercel.app/docs/guides/testing/)
- [/docs/guides/upgrading/](https://puerto-rs.vercel.app/docs/guides/upgrading/)
- [/docs/guides/use-cases/](https://puerto-rs.vercel.app/docs/guides/use-cases/)
- [/docs/guides/value-objects/](https://puerto-rs.vercel.app/docs/guides/value-objects/)
- [/docs/guides/web/](https://puerto-rs.vercel.app/docs/guides/web/)
- [/docs/reference/cli/](https://puerto-rs.vercel.app/docs/reference/cli/)
- [/docs/reference/errors/](https://puerto-rs.vercel.app/docs/reference/errors/)
- [/docs/reference/field-types/](https://puerto-rs.vercel.app/docs/reference/field-types/)
- [/docs/reference/make/](https://puerto-rs.vercel.app/docs/reference/make/)
- [/docs/reference/runtime-crates/](https://puerto-rs.vercel.app/docs/reference/runtime-crates/)
- [/docs/start/anatomy/](https://puerto-rs.vercel.app/docs/start/anatomy/)
- [/docs/start/architecture/](https://puerto-rs.vercel.app/docs/start/architecture/)
- [/docs/start/installation/](https://puerto-rs.vercel.app/docs/start/installation/)
- [/docs/start/quick-start/](https://puerto-rs.vercel.app/docs/start/quick-start/)

</details>

<details><summary><strong>meta-title:</strong> 22 item(s)</summary>

- [What Puerto is | Puerto (23 chars)](https://puerto-rs.vercel.app/docs/)
  - from: [/docs/](https://puerto-rs.vercel.app/docs/)
- [Changelog | Puerto (18 chars)](https://puerto-rs.vercel.app/docs/changelog/)
  - from: [/docs/changelog/](https://puerto-rs.vercel.app/docs/changelog/)
- [Working with agents | Puerto (28 chars)](https://puerto-rs.vercel.app/docs/guides/agents/)
  - from: [/docs/guides/agents/](https://puerto-rs.vercel.app/docs/guides/agents/)
- [Authentication | Puerto (23 chars)](https://puerto-rs.vercel.app/docs/guides/auth/)
  - from: [/docs/guides/auth/](https://puerto-rs.vercel.app/docs/guides/auth/)
- [Entities and fields | Puerto (28 chars)](https://puerto-rs.vercel.app/docs/guides/entities/)
  - from: [/docs/guides/entities/](https://puerto-rs.vercel.app/docs/guides/entities/)
- [Domain events | Puerto (22 chars)](https://puerto-rs.vercel.app/docs/guides/events/)
  - from: [/docs/guides/events/](https://puerto-rs.vercel.app/docs/guides/events/)
- [Layer by layer | Puerto (23 chars)](https://puerto-rs.vercel.app/docs/guides/layer-by-layer/)
  - from: [/docs/guides/layer-by-layer/](https://puerto-rs.vercel.app/docs/guides/layer-by-layer/)
- [Mail | Puerto (13 chars)](https://puerto-rs.vercel.app/docs/guides/mail/)
  - from: [/docs/guides/mail/](https://puerto-rs.vercel.app/docs/guides/mail/)
- [Testing | Puerto (16 chars)](https://puerto-rs.vercel.app/docs/guides/testing/)
  - from: [/docs/guides/testing/](https://puerto-rs.vercel.app/docs/guides/testing/)
- [Upgrading | Puerto (18 chars)](https://puerto-rs.vercel.app/docs/guides/upgrading/)
  - from: [/docs/guides/upgrading/](https://puerto-rs.vercel.app/docs/guides/upgrading/)
- [Use cases | Puerto (18 chars)](https://puerto-rs.vercel.app/docs/guides/use-cases/)
  - from: [/docs/guides/use-cases/](https://puerto-rs.vercel.app/docs/guides/use-cases/)
- [Value objects | Puerto (22 chars)](https://puerto-rs.vercel.app/docs/guides/value-objects/)
  - from: [/docs/guides/value-objects/](https://puerto-rs.vercel.app/docs/guides/value-objects/)
- [The web frontend | Puerto (25 chars)](https://puerto-rs.vercel.app/docs/guides/web/)
  - from: [/docs/guides/web/](https://puerto-rs.vercel.app/docs/guides/web/)
- [CLI | Puerto (12 chars)](https://puerto-rs.vercel.app/docs/reference/cli/)
  - from: [/docs/reference/cli/](https://puerto-rs.vercel.app/docs/reference/cli/)
- [Error contract | Puerto (23 chars)](https://puerto-rs.vercel.app/docs/reference/errors/)
  - from: [/docs/reference/errors/](https://puerto-rs.vercel.app/docs/reference/errors/)
- [Field types | Puerto (20 chars)](https://puerto-rs.vercel.app/docs/reference/field-types/)
  - from: [/docs/reference/field-types/](https://puerto-rs.vercel.app/docs/reference/field-types/)
- [Make targets | Puerto (21 chars)](https://puerto-rs.vercel.app/docs/reference/make/)
  - from: [/docs/reference/make/](https://puerto-rs.vercel.app/docs/reference/make/)
- [Runtime crates | Puerto (23 chars)](https://puerto-rs.vercel.app/docs/reference/runtime-crates/)
  - from: [/docs/reference/runtime-crates/](https://puerto-rs.vercel.app/docs/reference/runtime-crates/)
- [Anatomy of a project | Puerto (29 chars)](https://puerto-rs.vercel.app/docs/start/anatomy/)
  - from: [/docs/start/anatomy/](https://puerto-rs.vercel.app/docs/start/anatomy/)
- [The architecture | Puerto (25 chars)](https://puerto-rs.vercel.app/docs/start/architecture/)
  - from: [/docs/start/architecture/](https://puerto-rs.vercel.app/docs/start/architecture/)
- [Installation | Puerto (21 chars)](https://puerto-rs.vercel.app/docs/start/installation/)
  - from: [/docs/start/installation/](https://puerto-rs.vercel.app/docs/start/installation/)
- [Quick start | Puerto (20 chars)](https://puerto-rs.vercel.app/docs/start/quick-start/)
  - from: [/docs/start/quick-start/](https://puerto-rs.vercel.app/docs/start/quick-start/)

</details>

---

##### Meta Description **[WARN]**

`core/meta-description`

> Validates meta description presence and length

**Solution:**

Meta descriptions should be 120-160 characters and provide a compelling summary of the page. While not a direct ranking factor, good descriptions improve click-through rates from search results. Write unique descriptions for each page that accurately preview the content. Include a call-to-action when appropriate. If missing, search engines will auto-generate snippets which may not represent your page optimally.

| Check | Status | Message |
|-------|--------|---------|
| meta-description | ! warn | Description too short |

<details><summary><strong>meta-description:</strong> 1 page(s) affected</summary>

- [/docs/start/installation/](https://puerto-rs.vercel.app/docs/start/installation/)

</details>

<details><summary><strong>meta-description:</strong> 1 item(s)</summary>

- [Install the Puerto CLI with cargo, and the tools a (119 chars)](https://puerto-rs.vercel.app/docs/start/installation/)
  - from: [/docs/start/installation/](https://puerto-rs.vercel.app/docs/start/installation/)

</details>

---

#### Security

##### Content Security Policy **[WARN]**

`security/csp`

> Checks for Content-Security-Policy header and validates directives

**Solution:**

CSP prevents XSS attacks by restricting which resources can load. Start with a report-only policy to identify issues. Key directives: default-src 'self', script-src (avoid 'unsafe-inline'), img-src, style-src, frame-ancestors. Use nonces or hashes instead of 'unsafe-inline' for scripts. Test thoroughly as strict CSP can break functionality.

| Check | Status | Message |
|-------|--------|---------|
| csp-no-script-control | ! warn | CSP has no script-src or default-src directive |

---

#### Links

##### Broken External Links **[WARN]**

`links/broken-external-links`

> Detects external links returning 4xx/5xx errors or timeouts

**Solution:**

Broken external links hurt user experience and credibility. Regularly audit external links using automated tools. Remove or replace broken links with working alternatives. Consider using archived versions (archive.org) if the original content is gone. For important resources, consider hosting your own copies of critical documentation or linking to more stable sources.

| Check | Status | Message |
|-------|--------|---------|
| broken-external-links | ! warn | 3 broken external link(s): 2 failed, 1 with 404 |

<details><summary><strong>broken-external-links:</strong> 2 page(s) affected</summary>

- [/docs/reference/make/](https://puerto-rs.vercel.app/docs/reference/make/)
- [/](https://puerto-rs.vercel.app/)

</details>

<details><summary><strong>broken-external-links:</strong> 3 item(s)</summary>

- [http://localhost:5173/ (Error: Unable to connect. Is the computer able to access the url?)](http://localhost:5173/)
  - from: [/docs/reference/make/](https://puerto-rs.vercel.app/docs/reference/make/)
- [http://localhost:8080/ (Error: Unable to connect. Is the computer able to access the url?)](http://localhost:8080/)
  - from: [/docs/reference/make/](https://puerto-rs.vercel.app/docs/reference/make/)
- [https://crates.io/crates/puerto (404)](https://crates.io/crates/puerto)
  - from: [/](https://puerto-rs.vercel.app/)

</details>

---

##### HTTPS Downgrade **[WARN]**

`links/https-downgrade`

> Detects links from HTTPS pages to HTTP destinations

**Solution:**

Links from HTTPS to HTTP pages create security warnings and break the trust chain. Users may see 'not secure' warnings. Update all links to use HTTPS. If the target site doesn't support HTTPS, consider if you really need to link there. For internal links, ensure your entire site uses HTTPS.

| Check | Status | Message |
|-------|--------|---------|
| https-downgrade | ! warn | 2 link(s) downgrade to HTTP |

<details><summary><strong>https-downgrade:</strong> 1 page(s) affected</summary>

- [/docs/reference/make/](https://puerto-rs.vercel.app/docs/reference/make/)

</details>

<details><summary><strong>https-downgrade:</strong> 2 item(s)</summary>

- [http://localhost:5173](http://localhost:5173)
  - from: [/docs/reference/make/](https://puerto-rs.vercel.app/docs/reference/make/)
- [http://localhost:8080](http://localhost:8080)
  - from: [/docs/reference/make/](https://puerto-rs.vercel.app/docs/reference/make/)

</details>

---

##### No Contextual Inbound Links **[WARN]**

`links/no-contextual-inbound`

> Detects pages whose only internal inbound links come from sitewide chrome (nav, header, footer, sidebar)

**Solution:**

These pages look well-linked but receive no editorial support: every link pointing at them is repeated on every page of the site, so it carries no signal about what the page is about or how important it is. Add links from the body copy of related articles, hub pages, or category descriptions using descriptive anchor text. Sitewide navigation links help discovery, but contextual links are what pass topical relevance and internal link equity.

| Check | Status | Message |
|-------|--------|---------|
| no-contextual-inbound | ! warn | 5 page(s) are linked only from sitewide chrome |

<details><summary><strong>no-contextual-inbound:</strong> 5 page(s) affected</summary>

- [/docs/](https://puerto-rs.vercel.app/docs/)
- [/docs/guides/testing/](https://puerto-rs.vercel.app/docs/guides/testing/)
- [/docs/reference/cli/](https://puerto-rs.vercel.app/docs/reference/cli/)
- [/docs/reference/make/](https://puerto-rs.vercel.app/docs/reference/make/)
- [/docs/start/anatomy/](https://puerto-rs.vercel.app/docs/start/anatomy/)

</details>

---

#### Content

##### Placeholder Text **[WARN]**

`content/placeholder-text`

> Detects template leftovers and filler copy that shipped to production

**Solution:**

Each family points at a different break in the publishing pipeline. Lorem ipsum and theme boilerplate mean a page was published before its copy was written: replace the text, and add the affected fields to whatever check gates publishing. Unrendered template syntax means the templating engine never ran over that string, usually because the value was interpolated into an already-escaped fragment or the template was served as static HTML: render it server-side, or delete the stale copy. A visible undefined, NaN, null or [object Object] means the value was missing and the code concatenated it into the copy anyway: guard the field at the point of render rather than in CSS. TODO and FIXME markers mean a draft shipped: finish or remove the note, since search engines and readers see it exactly as written.

| Check | Status | Message |
|-------|--------|---------|
| placeholder-content | X fail | 15 placeholder occurrence(s) in visible text: unrendered template syntax ([[entity]]) |

<details><summary><strong>placeholder-content:</strong> 1 page(s) affected</summary>

- [/docs/reference/puerto-toml/](https://puerto-rs.vercel.app/docs/reference/puerto-toml/)

</details>

<details><summary><strong>placeholder-content:</strong> 1 item(s)</summary>

- unrendered template syntax
  - from: [/docs/reference/puerto-toml/](https://puerto-rs.vercel.app/docs/reference/puerto-toml/)

</details>

---

##### Keyword Stuffing **[WARN]**

`content/keyword-stuffing`

> Detects excessive keyword repetition in content

**Solution:**

Keyword stuffing is repeating words unnaturally to manipulate rankings. Search engines penalize this practice. Write naturally for users first. Use keywords where they fit naturally. Aim for 1-2% keyword density at most. Use synonyms and related terms instead of repeating the exact same phrase. Focus on providing value, not gaming algorithms.

| Check | Status | Message |
|-------|--------|---------|
| keyword-stuffing | ! warn | 1 to 3 word(s) may be overused |

<details><summary><strong>keyword-stuffing:</strong> 17 page(s) affected</summary>

- [/](https://puerto-rs.vercel.app/)
- [/docs/changelog/](https://puerto-rs.vercel.app/docs/changelog/)
- [/docs/decisions/](https://puerto-rs.vercel.app/docs/decisions/)
- [/docs/decisions/adr/0014-entity-ids-are-generated-newtypes/](https://puerto-rs.vercel.app/docs/decisions/adr/0014-entity-ids-are-generated-newtypes/)
- [/docs/decisions/adr/0020-mail-port-per-mail-sent-from-the-worker/](https://puerto-rs.vercel.app/docs/decisions/adr/0020-mail-port-per-mail-sent-from-the-worker/)
- [/docs/guides/auth/](https://puerto-rs.vercel.app/docs/guides/auth/)
- [/docs/guides/database/](https://puerto-rs.vercel.app/docs/guides/database/)
- [/docs/guides/layer-by-layer/](https://puerto-rs.vercel.app/docs/guides/layer-by-layer/)
- [/docs/guides/mail/](https://puerto-rs.vercel.app/docs/guides/mail/)
- [/docs/guides/upgrading/](https://puerto-rs.vercel.app/docs/guides/upgrading/)
- [/docs/guides/use-cases/](https://puerto-rs.vercel.app/docs/guides/use-cases/)
- [/docs/guides/value-objects/](https://puerto-rs.vercel.app/docs/guides/value-objects/)
- [/docs/reference/cli/](https://puerto-rs.vercel.app/docs/reference/cli/)
- [/docs/reference/errors/](https://puerto-rs.vercel.app/docs/reference/errors/)
- [/docs/reference/field-types/](https://puerto-rs.vercel.app/docs/reference/field-types/)
- [/docs/reference/make/](https://puerto-rs.vercel.app/docs/reference/make/)
- [/docs/reference/runtime-crates/](https://puerto-rs.vercel.app/docs/reference/runtime-crates/)

</details>

<details><summary><strong>keyword-stuffing:</strong> 16 item(s)</summary>

- "are" (3.5%)
  - from: [/docs/decisions/](https://puerto-rs.vercel.app/docs/decisions/)
- "auth" (3.5%)
  - from: [/docs/guides/auth/](https://puerto-rs.vercel.app/docs/guides/auth/)
- "entity" (3.8%)
  - from: [/docs/reference/errors/](https://puerto-rs.vercel.app/docs/reference/errors/)
- "error" (3.8%)
  - from: [/docs/reference/errors/](https://puerto-rs.vercel.app/docs/reference/errors/)
- "every" (3.6%)
  - from: [/docs/reference/make/](https://puerto-rs.vercel.app/docs/reference/make/)
- "generate" (3.9%)
  - from: [/docs/reference/cli/](https://puerto-rs.vercel.app/docs/reference/cli/)
- "mail" (4.0%)
  - from: [/docs/decisions/adr/0020-mail-port-per-mail-sent-from-the-worker/](https://puerto-rs.vercel.app/docs/decisions/adr/0020-mail-port-per-mail-sent-from-the-worker/)
  - from: [/docs/guides/mail/](https://puerto-rs.vercel.app/docs/guides/mail/)
- "name" (3.5%)
  - from: [/docs/reference/cli/](https://puerto-rs.vercel.app/docs/reference/cli/)
- "product" (5.8%)
  - from: [/](https://puerto-rs.vercel.app/)
  - from: [/docs/guides/use-cases/](https://puerto-rs.vercel.app/docs/guides/use-cases/)
- "project" (3.7%)
  - from: [/docs/reference/make/](https://puerto-rs.vercel.app/docs/reference/make/)
- "puerto" (3.1%)
  - from: [/docs/changelog/](https://puerto-rs.vercel.app/docs/changelog/)
  - from: [/docs/guides/layer-by-layer/](https://puerto-rs.vercel.app/docs/guides/layer-by-layer/)
  - from: [/docs/guides/upgrading/](https://puerto-rs.vercel.app/docs/guides/upgrading/)
  - +2 more (see full list above)
- "sqlx" (3.2%)
  - from: [/docs/guides/database/](https://puerto-rs.vercel.app/docs/guides/database/)
- "src" (3.8%)
  - from: [/](https://puerto-rs.vercel.app/)
- "string" (4.1%)
  - from: [/docs/reference/field-types/](https://puerto-rs.vercel.app/docs/reference/field-types/)
- "uuid" (3.3%)
  - from: [/docs/decisions/adr/0014-entity-ids-are-generated-newtypes/](https://puerto-rs.vercel.app/docs/decisions/adr/0014-entity-ids-are-generated-newtypes/)
- "value" (5.0%)
  - from: [/docs/guides/value-objects/](https://puerto-rs.vercel.app/docs/guides/value-objects/)

</details>

---

#### Performance

##### DOM Size **[WARN]**

`perf/dom-size`

> Detects excessive DOM complexity that impacts performance

**Solution:**

Large DOMs slow page rendering, increase memory usage, and harm mobile performance. Google recommends keeping total nodes under 1500.

Fixes for large DOMs:
- Use virtualization for long lists (e.g., react-window)
- Lazy-load off-screen content
- Reduce unnecessary wrapper elements
- Use CSS instead of DOM for visual effects
- Paginate large content sections

| Check | Status | Message |
|-------|--------|---------|
| dom-max-children | ! warn | Element with 95 to 192 children found |
| dom-total-nodes | ! warn | Large DOM (1690 nodes) |
| dom-total-nodes | ! warn | Large DOM (2643 nodes) |

<details><summary><strong>dom-max-children:</strong> 2 page(s) affected</summary>

- [/docs/changelog/](https://puerto-rs.vercel.app/docs/changelog/)
- [/docs/reference/cli/](https://puerto-rs.vercel.app/docs/reference/cli/)

</details>

<details><summary><strong>dom-total-nodes:</strong> 1 page(s) affected</summary>

- [/docs/guides/use-cases/](https://puerto-rs.vercel.app/docs/guides/use-cases/)

</details>

<details><summary><strong>dom-total-nodes:</strong> 1 page(s) affected</summary>

- [/docs/changelog/](https://puerto-rs.vercel.app/docs/changelog/)

</details>

---

##### Critical Request Chains **[WARN]**

`perf/critical-request-chains`

> Identifies chains of dependent resources that delay rendering

**Solution:**

Critical request chains are sequences of dependent network requests that must complete before the page can render. Reduce chain depth by: 1) Inlining critical CSS instead of linking external files. 2) Adding async or defer to non-critical scripts. 3) Avoiding CSS @import — use <link> tags instead. 4) Using <link rel='preload'> for critical resources. 5) Reducing the number of render-blocking resources in <head>.

| Check | Status | Message |
|-------|--------|---------|
| critical-request-chains | ! warn | 1 to 2 critical request chain(s) found |

<details><summary><strong>critical-request-chains:</strong> 48 page(s) affected</summary>

- [/](https://puerto-rs.vercel.app/)
- [/docs/](https://puerto-rs.vercel.app/docs/)
- [/docs/changelog/](https://puerto-rs.vercel.app/docs/changelog/)
- [/docs/decisions/](https://puerto-rs.vercel.app/docs/decisions/)
- [/docs/decisions/adr/0001-enum-value-objects-implement-fromstr/](https://puerto-rs.vercel.app/docs/decisions/adr/0001-enum-value-objects-implement-fromstr/)
- [/docs/decisions/adr/0002-response-enums-allow-large-enum-variant/](https://puerto-rs.vercel.app/docs/decisions/adr/0002-response-enums-allow-large-enum-variant/)
- [/docs/decisions/adr/0003-unique-indexes-are-partial/](https://puerto-rs.vercel.app/docs/decisions/adr/0003-unique-indexes-are-partial/)
- [/docs/decisions/adr/0004-generate-use-case-never-overwrites/](https://puerto-rs.vercel.app/docs/decisions/adr/0004-generate-use-case-never-overwrites/)
- [/docs/decisions/adr/0005-sql-reserved-words-only-in-db-projects/](https://puerto-rs.vercel.app/docs/decisions/adr/0005-sql-reserved-words-only-in-db-projects/)
- [/docs/decisions/adr/0006-logging-through-tracing-not-a-logger-port/](https://puerto-rs.vercel.app/docs/decisions/adr/0006-logging-through-tracing-not-a-logger-port/)
- [/docs/decisions/adr/0007-error-contract-code-in-name-422/](https://puerto-rs.vercel.app/docs/decisions/adr/0007-error-contract-code-in-name-422/)
- [/docs/decisions/adr/0008-liveness-readiness-graceful-shutdown/](https://puerto-rs.vercel.app/docs/decisions/adr/0008-liveness-readiness-graceful-shutdown/)
- [/docs/decisions/adr/0009-generated-projects-pin-their-toolchain/](https://puerto-rs.vercel.app/docs/decisions/adr/0009-generated-projects-pin-their-toolchain/)
- [/docs/decisions/adr/0010-generated-agent-rules-are-tested/](https://puerto-rs.vercel.app/docs/decisions/adr/0010-generated-agent-rules-are-tested/)
- [/docs/decisions/adr/0011-templates-own-shape-types-own-decisions/](https://puerto-rs.vercel.app/docs/decisions/adr/0011-templates-own-shape-types-own-decisions/)
- [/docs/decisions/adr/0012-presentation-holds-use-case-ports/](https://puerto-rs.vercel.app/docs/decisions/adr/0012-presentation-holds-use-case-ports/)
- [/docs/decisions/adr/0013-web-frontend-react-daisyui-bun/](https://puerto-rs.vercel.app/docs/decisions/adr/0013-web-frontend-react-daisyui-bun/)
- [/docs/decisions/adr/0014-entity-ids-are-generated-newtypes/](https://puerto-rs.vercel.app/docs/decisions/adr/0014-entity-ids-are-generated-newtypes/)
- [/docs/decisions/adr/0015-repositories-return-one-repository-error/](https://puerto-rs.vercel.app/docs/decisions/adr/0015-repositories-return-one-repository-error/)
- [/docs/decisions/adr/0016-lists-are-offset-pages/](https://puerto-rs.vercel.app/docs/decisions/adr/0016-lists-are-offset-pages/)
- [/docs/decisions/adr/0017-runtime-crates-next-to-the-generator/](https://puerto-rs.vercel.app/docs/decisions/adr/0017-runtime-crates-next-to-the-generator/)
- [/docs/decisions/adr/0018-domain-events-outbox-pluggable-transport/](https://puerto-rs.vercel.app/docs/decisions/adr/0018-domain-events-outbox-pluggable-transport/)
- [/docs/decisions/adr/0019-generated-code-is-formatted-by-cargo-fmt/](https://puerto-rs.vercel.app/docs/decisions/adr/0019-generated-code-is-formatted-by-cargo-fmt/)
- [/docs/decisions/adr/0020-mail-port-per-mail-sent-from-the-worker/](https://puerto-rs.vercel.app/docs/decisions/adr/0020-mail-port-per-mail-sent-from-the-worker/)
- [/docs/decisions/adr/0021-auth-one-authenticator-port-oidc-or-local/](https://puerto-rs.vercel.app/docs/decisions/adr/0021-auth-one-authenticator-port-oidc-or-local/)
- [/docs/guides/agents/](https://puerto-rs.vercel.app/docs/guides/agents/)
- [/docs/guides/auth/](https://puerto-rs.vercel.app/docs/guides/auth/)
- [/docs/guides/configuration/](https://puerto-rs.vercel.app/docs/guides/configuration/)
- [/docs/guides/database/](https://puerto-rs.vercel.app/docs/guides/database/)
- [/docs/guides/entities/](https://puerto-rs.vercel.app/docs/guides/entities/)
- [/docs/guides/events/](https://puerto-rs.vercel.app/docs/guides/events/)
- [/docs/guides/layer-by-layer/](https://puerto-rs.vercel.app/docs/guides/layer-by-layer/)
- [/docs/guides/mail/](https://puerto-rs.vercel.app/docs/guides/mail/)
- [/docs/guides/testing/](https://puerto-rs.vercel.app/docs/guides/testing/)
- [/docs/guides/upgrading/](https://puerto-rs.vercel.app/docs/guides/upgrading/)
- [/docs/guides/use-cases/](https://puerto-rs.vercel.app/docs/guides/use-cases/)
- [/docs/guides/value-objects/](https://puerto-rs.vercel.app/docs/guides/value-objects/)
- [/docs/guides/web/](https://puerto-rs.vercel.app/docs/guides/web/)
- [/docs/reference/cli/](https://puerto-rs.vercel.app/docs/reference/cli/)
- [/docs/reference/errors/](https://puerto-rs.vercel.app/docs/reference/errors/)
- [/docs/reference/field-types/](https://puerto-rs.vercel.app/docs/reference/field-types/)
- [/docs/reference/make/](https://puerto-rs.vercel.app/docs/reference/make/)
- [/docs/reference/puerto-toml/](https://puerto-rs.vercel.app/docs/reference/puerto-toml/)
- [/docs/reference/runtime-crates/](https://puerto-rs.vercel.app/docs/reference/runtime-crates/)
- [/docs/start/anatomy/](https://puerto-rs.vercel.app/docs/start/anatomy/)
- [/docs/start/architecture/](https://puerto-rs.vercel.app/docs/start/architecture/)
- [/docs/start/installation/](https://puerto-rs.vercel.app/docs/start/installation/)
- [/docs/start/quick-start/](https://puerto-rs.vercel.app/docs/start/quick-start/)

</details>

<details><summary><strong>critical-request-chains:</strong> 3 item(s)</summary>

- CSS: /_astro/BaseLayout.C7NmqqhH.css
  - from: [/](https://puerto-rs.vercel.app/)
- CSS: /_astro/index.BgsnL72l.css
  - from: [/](https://puerto-rs.vercel.app/)
- CSS: /_astro/index.C7B8cZ5E.css
  - from: [/docs/](https://puerto-rs.vercel.app/docs/)
  - from: [/docs/changelog/](https://puerto-rs.vercel.app/docs/changelog/)
  - from: [/docs/decisions/](https://puerto-rs.vercel.app/docs/decisions/)
  - +7 more (see full list above)

</details>

---

##### Duplicate JavaScript **[WARN]**

`perf/duplicate-js`

> Detects duplicate JavaScript libraries loaded multiple times

**Solution:**

Remove duplicate JavaScript library loads to reduce page weight and avoid conflicts. Check for the same library loaded from different CDNs or versions. Use a single source for each dependency. Consider using a module bundler to deduplicate shared dependencies.

| Check | Status | Message |
|-------|--------|---------|
| duplicate-js-same-version | ! warn | 1 library(s) loaded multiple times |

<details><summary><strong>duplicate-js-same-version:</strong> 1 page(s) affected</summary>

- [/docs/guides/agents/](https://puerto-rs.vercel.app/docs/guides/agents/)

</details>

<details><summary><strong>duplicate-js-same-version:</strong> 1 item(s)</summary>

- ec (2x)
  - from: [/docs/guides/agents/](https://puerto-rs.vercel.app/docs/guides/agents/)

</details>

---

#### Social Media

##### Site Asset Consistency **[WARN]**

`social/asset-divergence`

> Finds pages whose favicon, theme-color or default OG image differs from the rest of the site

**Solution:**

Your favicon, theme-color and fallback share image come from one layout, so every page should carry the same three. Pages that carry different ones are usually running an older template: a section that missed a redesign, a cached shell, or a second layout nobody remembers owning. The visible cost is inconsistent link previews and a tab icon that changes as visitors move around the site. Fix it where the assets are declared (the shared head partial or theme config) rather than page by page. Per-page share images on articles and products are correct and are not reported here: what this rule compares is the fallback the rest of the site shares.

| Check | Status | Message |
|-------|--------|---------|
| asset-divergence | ! warn | 1 of 48 page(s) diverge from the site's own favicon, theme-color and default OG image (favicon 100%, theme-color 98%, default OG image 100%) |

<details><summary><strong>asset-divergence:</strong> 1 page(s) affected</summary>

- [/](https://puerto-rs.vercel.app/)

</details>

<details><summary><strong>asset-divergence:</strong> 1 item(s)</summary>

- 1 of 48 page(s) declare theme-color "#e9e5dc" but the site norm is "#101215"
  - from: [/](https://puerto-rs.vercel.app/)

</details>

---

#### Accessibility

##### Identical Links Same Purpose **[WARN]**

`a11y/identical-links-same-purpose`

> Checks that links with identical text go to the same destination

**Solution:**

Links with the same visible text should go to the same URL. When identical link text leads to different destinations, it confuses screen reader users who navigate by listing links. Make link text unique or more descriptive to differentiate destinations.

| Check | Status | Message |
|-------|--------|---------|
| identical-links-same-purpose | ! warn | 1 link text(s) lead to different destinations |

<details><summary><strong>identical-links-same-purpose:</strong> 2 page(s) affected</summary>

- [/docs/guides/agents/](https://puerto-rs.vercel.app/docs/guides/agents/)
- [/docs/guides/configuration/](https://puerto-rs.vercel.app/docs/guides/configuration/)

</details>

<details><summary><strong>identical-links-same-purpose:</strong> 2 item(s)</summary>

- "mail" → 2 different URLs
  - from: [/docs/guides/configuration/](https://puerto-rs.vercel.app/docs/guides/configuration/)
- "the architecture" → 2 different URLs
  - from: [/docs/guides/agents/](https://puerto-rs.vercel.app/docs/guides/agents/)

</details>

---

##### Table Duplicate Name **[WARN]**

`a11y/table-duplicate-name`

> Checks that data tables have unique accessible names

**Solution:**

When a page has multiple data tables, each should have a unique accessible name to help users distinguish between them. Use <caption>, aria-label, or aria-labelledby with unique text for each table.

| Check | Status | Message |
|-------|--------|---------|
| tables-without-names | ! warn | 2 to 20 table(s) without accessible names |

<details><summary><strong>tables-without-names:</strong> 10 page(s) affected</summary>

- [/docs/decisions/adr/0017-runtime-crates-next-to-the-generator/](https://puerto-rs.vercel.app/docs/decisions/adr/0017-runtime-crates-next-to-the-generator/)
- [/docs/decisions/adr/0020-mail-port-per-mail-sent-from-the-worker/](https://puerto-rs.vercel.app/docs/decisions/adr/0020-mail-port-per-mail-sent-from-the-worker/)
- [/docs/guides/auth/](https://puerto-rs.vercel.app/docs/guides/auth/)
- [/docs/guides/configuration/](https://puerto-rs.vercel.app/docs/guides/configuration/)
- [/docs/guides/database/](https://puerto-rs.vercel.app/docs/guides/database/)
- [/docs/guides/events/](https://puerto-rs.vercel.app/docs/guides/events/)
- [/docs/reference/cli/](https://puerto-rs.vercel.app/docs/reference/cli/)
- [/docs/reference/errors/](https://puerto-rs.vercel.app/docs/reference/errors/)
- [/docs/reference/field-types/](https://puerto-rs.vercel.app/docs/reference/field-types/)
- [/docs/reference/puerto-toml/](https://puerto-rs.vercel.app/docs/reference/puerto-toml/)

</details>

---

#### E-E-A-T

##### About Page **[WARN]**

`eeat/about-page`

> Checks for an about/company page with content

**Solution:**

An About page establishes credibility and trust. Include company history, mission, team overview, and credentials. Link from main navigation or footer. For E-E-A-T, explain your expertise and why visitors should trust you. Include contact information and physical location if applicable.

| Check | Status | Message |
|-------|--------|---------|
| about-page | ! warn | No About page found |

---

##### Author Bylines **[WARN]**

`eeat/author-byline`

> Checks for visible author names on content pages

**Solution:**

Author bylines demonstrate experience and accountability. Show author names prominently on articles, blog posts, and expert content. Include author credentials where relevant. Link author names to bio pages. For YMYL content (health, finance), author transparency is especially important for Google's E-E-A-T assessment.

| Check | Status | Message |
|-------|--------|---------|
| author-byline | ! warn | No content pages have author attribution |

---

##### Contact Page **[WARN]**

`eeat/contact-page`

> Checks for contact page with multiple contact methods

**Solution:**

A contact page with multiple contact methods builds trust. Include: email address or contact form, phone number (if applicable), physical address, and social media links. Make contact information easy to find from any page. For local businesses, include business hours. Response time expectations are also helpful.

| Check | Status | Message |
|-------|--------|---------|
| contact-page | ! warn | No Contact page found |

---

##### Privacy Policy **[WARN]**

`eeat/privacy-policy`

> Checks for privacy policy page linked from footer

**Solution:**

A privacy policy is required by law in many jurisdictions (GDPR, CCPA) and signals trustworthiness. Link it from your footer on every page. Cover: what data you collect, how you use it, third-party sharing, user rights, and contact for privacy concerns. Keep it updated when practices change.

| Check | Status | Message |
|-------|--------|---------|
| privacy-policy | ! warn | No Privacy Policy page found |

---

#### Legal Compliance

##### Privacy Policy **[WARN]**

`legal/privacy-policy`

> Checks for privacy policy link presence

**Solution:**

A privacy policy is legally required in many jurisdictions (GDPR, CCPA). Link to your privacy policy from every page, typically in the footer. The policy should explain what data you collect, how it's used, and user rights. Consider using schema.org markup to identify the policy page.

| Check | Status | Message |
|-------|--------|---------|
| privacy-policy | ! warn | No privacy policy link found across site |

---

#### Agent Experience

##### Token Weight **[WARN]**

`ax/token-weight`

> Estimates the LLM token cost of a page's raw HTML and reports the text-to-HTML ratio — how much of that cost is actual content versus markup, scripts, and styles

**Solution:**

Strip or externalize inline <script>/<style> blocks, avoid deeply nested wrapper divs and long utility-class strings on content-bearing elements, and serve lean server-rendered markup rather than a client framework's verbose hydration output — especially on content pages. Consider Markdown content negotiation, which sidesteps the ratio problem by removing HTML markup from the response entirely.

| Check | Status | Message |
|-------|--------|---------|
| token-weight-ratio | ! warn | Visible text is under 15% of the page HTML — agents pay token cost mostly for markup, scripts, and styles |

<details><summary><strong>token-weight-ratio:</strong> 42 page(s) affected</summary>

- [/](https://puerto-rs.vercel.app/)
- [/docs/](https://puerto-rs.vercel.app/docs/)
- [/docs/decisions/](https://puerto-rs.vercel.app/docs/decisions/)
- [/docs/decisions/adr/0001-enum-value-objects-implement-fromstr/](https://puerto-rs.vercel.app/docs/decisions/adr/0001-enum-value-objects-implement-fromstr/)
- [/docs/decisions/adr/0002-response-enums-allow-large-enum-variant/](https://puerto-rs.vercel.app/docs/decisions/adr/0002-response-enums-allow-large-enum-variant/)
- [/docs/decisions/adr/0003-unique-indexes-are-partial/](https://puerto-rs.vercel.app/docs/decisions/adr/0003-unique-indexes-are-partial/)
- [/docs/decisions/adr/0004-generate-use-case-never-overwrites/](https://puerto-rs.vercel.app/docs/decisions/adr/0004-generate-use-case-never-overwrites/)
- [/docs/decisions/adr/0005-sql-reserved-words-only-in-db-projects/](https://puerto-rs.vercel.app/docs/decisions/adr/0005-sql-reserved-words-only-in-db-projects/)
- [/docs/decisions/adr/0006-logging-through-tracing-not-a-logger-port/](https://puerto-rs.vercel.app/docs/decisions/adr/0006-logging-through-tracing-not-a-logger-port/)
- [/docs/decisions/adr/0007-error-contract-code-in-name-422/](https://puerto-rs.vercel.app/docs/decisions/adr/0007-error-contract-code-in-name-422/)
- [/docs/decisions/adr/0008-liveness-readiness-graceful-shutdown/](https://puerto-rs.vercel.app/docs/decisions/adr/0008-liveness-readiness-graceful-shutdown/)
- [/docs/decisions/adr/0009-generated-projects-pin-their-toolchain/](https://puerto-rs.vercel.app/docs/decisions/adr/0009-generated-projects-pin-their-toolchain/)
- [/docs/decisions/adr/0010-generated-agent-rules-are-tested/](https://puerto-rs.vercel.app/docs/decisions/adr/0010-generated-agent-rules-are-tested/)
- [/docs/decisions/adr/0011-templates-own-shape-types-own-decisions/](https://puerto-rs.vercel.app/docs/decisions/adr/0011-templates-own-shape-types-own-decisions/)
- [/docs/decisions/adr/0012-presentation-holds-use-case-ports/](https://puerto-rs.vercel.app/docs/decisions/adr/0012-presentation-holds-use-case-ports/)
- [/docs/decisions/adr/0014-entity-ids-are-generated-newtypes/](https://puerto-rs.vercel.app/docs/decisions/adr/0014-entity-ids-are-generated-newtypes/)
- [/docs/decisions/adr/0015-repositories-return-one-repository-error/](https://puerto-rs.vercel.app/docs/decisions/adr/0015-repositories-return-one-repository-error/)
- [/docs/decisions/adr/0016-lists-are-offset-pages/](https://puerto-rs.vercel.app/docs/decisions/adr/0016-lists-are-offset-pages/)
- [/docs/decisions/adr/0019-generated-code-is-formatted-by-cargo-fmt/](https://puerto-rs.vercel.app/docs/decisions/adr/0019-generated-code-is-formatted-by-cargo-fmt/)
- [/docs/guides/agents/](https://puerto-rs.vercel.app/docs/guides/agents/)
- [/docs/guides/auth/](https://puerto-rs.vercel.app/docs/guides/auth/)
- [/docs/guides/configuration/](https://puerto-rs.vercel.app/docs/guides/configuration/)
- [/docs/guides/database/](https://puerto-rs.vercel.app/docs/guides/database/)
- [/docs/guides/entities/](https://puerto-rs.vercel.app/docs/guides/entities/)
- [/docs/guides/events/](https://puerto-rs.vercel.app/docs/guides/events/)
- [/docs/guides/layer-by-layer/](https://puerto-rs.vercel.app/docs/guides/layer-by-layer/)
- [/docs/guides/mail/](https://puerto-rs.vercel.app/docs/guides/mail/)
- [/docs/guides/testing/](https://puerto-rs.vercel.app/docs/guides/testing/)
- [/docs/guides/upgrading/](https://puerto-rs.vercel.app/docs/guides/upgrading/)
- [/docs/guides/use-cases/](https://puerto-rs.vercel.app/docs/guides/use-cases/)
- [/docs/guides/value-objects/](https://puerto-rs.vercel.app/docs/guides/value-objects/)
- [/docs/guides/web/](https://puerto-rs.vercel.app/docs/guides/web/)
- [/docs/reference/cli/](https://puerto-rs.vercel.app/docs/reference/cli/)
- [/docs/reference/errors/](https://puerto-rs.vercel.app/docs/reference/errors/)
- [/docs/reference/field-types/](https://puerto-rs.vercel.app/docs/reference/field-types/)
- [/docs/reference/make/](https://puerto-rs.vercel.app/docs/reference/make/)
- [/docs/reference/puerto-toml/](https://puerto-rs.vercel.app/docs/reference/puerto-toml/)
- [/docs/reference/runtime-crates/](https://puerto-rs.vercel.app/docs/reference/runtime-crates/)
- [/docs/start/anatomy/](https://puerto-rs.vercel.app/docs/start/anatomy/)
- [/docs/start/architecture/](https://puerto-rs.vercel.app/docs/start/architecture/)
- [/docs/start/installation/](https://puerto-rs.vercel.app/docs/start/installation/)
- [/docs/start/quick-start/](https://puerto-rs.vercel.app/docs/start/quick-start/)

</details>

<details><summary><strong>token-weight-ratio:</strong> 42 item(s)</summary>

- [~13% of HTML is visible text (~19,333 est. tokens)](https://puerto-rs.vercel.app/)
  - from: [/](https://puerto-rs.vercel.app/)
- [~10% of HTML is visible text (~13,126 est. tokens)](https://puerto-rs.vercel.app/docs/)
  - from: [/docs/](https://puerto-rs.vercel.app/docs/)
- [~10% of HTML is visible text (~10,441 est. tokens)](https://puerto-rs.vercel.app/docs/decisions/)
  - from: [/docs/decisions/](https://puerto-rs.vercel.app/docs/decisions/)
- [~7% of HTML is visible text (~9,959 est. tokens)](https://puerto-rs.vercel.app/docs/decisions/adr/0001-enum-value-objects-implement-fromstr/)
  - from: [/docs/decisions/adr/0001-enum-value-objects-implement-fromstr/](https://puerto-rs.vercel.app/docs/decisions/adr/0001-enum-value-objects-implement-fromstr/)
- [~7% of HTML is visible text (~9,904 est. tokens)](https://puerto-rs.vercel.app/docs/decisions/adr/0002-response-enums-allow-large-enum-variant/)
  - from: [/docs/decisions/adr/0002-response-enums-allow-large-enum-variant/](https://puerto-rs.vercel.app/docs/decisions/adr/0002-response-enums-allow-large-enum-variant/)
- [~7% of HTML is visible text (~9,898 est. tokens)](https://puerto-rs.vercel.app/docs/decisions/adr/0003-unique-indexes-are-partial/)
  - from: [/docs/decisions/adr/0003-unique-indexes-are-partial/](https://puerto-rs.vercel.app/docs/decisions/adr/0003-unique-indexes-are-partial/)
- [~8% of HTML is visible text (~10,095 est. tokens)](https://puerto-rs.vercel.app/docs/decisions/adr/0004-generate-use-case-never-overwrites/)
  - from: [/docs/decisions/adr/0004-generate-use-case-never-overwrites/](https://puerto-rs.vercel.app/docs/decisions/adr/0004-generate-use-case-never-overwrites/)
- [~7% of HTML is visible text (~9,942 est. tokens)](https://puerto-rs.vercel.app/docs/decisions/adr/0005-sql-reserved-words-only-in-db-projects/)
  - from: [/docs/decisions/adr/0005-sql-reserved-words-only-in-db-projects/](https://puerto-rs.vercel.app/docs/decisions/adr/0005-sql-reserved-words-only-in-db-projects/)
- [~8% of HTML is visible text (~10,196 est. tokens)](https://puerto-rs.vercel.app/docs/decisions/adr/0006-logging-through-tracing-not-a-logger-port/)
  - from: [/docs/decisions/adr/0006-logging-through-tracing-not-a-logger-port/](https://puerto-rs.vercel.app/docs/decisions/adr/0006-logging-through-tracing-not-a-logger-port/)
- [~8% of HTML is visible text (~10,099 est. tokens)](https://puerto-rs.vercel.app/docs/decisions/adr/0007-error-contract-code-in-name-422/)
  - from: [/docs/decisions/adr/0007-error-contract-code-in-name-422/](https://puerto-rs.vercel.app/docs/decisions/adr/0007-error-contract-code-in-name-422/)
- [~7% of HTML is visible text (~10,016 est. tokens)](https://puerto-rs.vercel.app/docs/decisions/adr/0008-liveness-readiness-graceful-shutdown/)
  - from: [/docs/decisions/adr/0008-liveness-readiness-graceful-shutdown/](https://puerto-rs.vercel.app/docs/decisions/adr/0008-liveness-readiness-graceful-shutdown/)
- [~8% of HTML is visible text (~10,042 est. tokens)](https://puerto-rs.vercel.app/docs/decisions/adr/0009-generated-projects-pin-their-toolchain/)
  - from: [/docs/decisions/adr/0009-generated-projects-pin-their-toolchain/](https://puerto-rs.vercel.app/docs/decisions/adr/0009-generated-projects-pin-their-toolchain/)
- [~8% of HTML is visible text (~10,095 est. tokens)](https://puerto-rs.vercel.app/docs/decisions/adr/0010-generated-agent-rules-are-tested/)
  - from: [/docs/decisions/adr/0010-generated-agent-rules-are-tested/](https://puerto-rs.vercel.app/docs/decisions/adr/0010-generated-agent-rules-are-tested/)
- [~11% of HTML is visible text (~10,575 est. tokens)](https://puerto-rs.vercel.app/docs/decisions/adr/0011-templates-own-shape-types-own-decisions/)
  - from: [/docs/decisions/adr/0011-templates-own-shape-types-own-decisions/](https://puerto-rs.vercel.app/docs/decisions/adr/0011-templates-own-shape-types-own-decisions/)
- [~11% of HTML is visible text (~10,585 est. tokens)](https://puerto-rs.vercel.app/docs/decisions/adr/0012-presentation-holds-use-case-ports/)
  - from: [/docs/decisions/adr/0012-presentation-holds-use-case-ports/](https://puerto-rs.vercel.app/docs/decisions/adr/0012-presentation-holds-use-case-ports/)
- [~9% of HTML is visible text (~10,397 est. tokens)](https://puerto-rs.vercel.app/docs/decisions/adr/0014-entity-ids-are-generated-newtypes/)
  - from: [/docs/decisions/adr/0014-entity-ids-are-generated-newtypes/](https://puerto-rs.vercel.app/docs/decisions/adr/0014-entity-ids-are-generated-newtypes/)
- [~10% of HTML is visible text (~10,508 est. tokens)](https://puerto-rs.vercel.app/docs/decisions/adr/0015-repositories-return-one-repository-error/)
  - from: [/docs/decisions/adr/0015-repositories-return-one-repository-error/](https://puerto-rs.vercel.app/docs/decisions/adr/0015-repositories-return-one-repository-error/)
- [~10% of HTML is visible text (~10,400 est. tokens)](https://puerto-rs.vercel.app/docs/decisions/adr/0016-lists-are-offset-pages/)
  - from: [/docs/decisions/adr/0016-lists-are-offset-pages/](https://puerto-rs.vercel.app/docs/decisions/adr/0016-lists-are-offset-pages/)
- [~11% of HTML is visible text (~10,607 est. tokens)](https://puerto-rs.vercel.app/docs/decisions/adr/0019-generated-code-is-formatted-by-cargo-fmt/)
  - from: [/docs/decisions/adr/0019-generated-code-is-formatted-by-cargo-fmt/](https://puerto-rs.vercel.app/docs/decisions/adr/0019-generated-code-is-formatted-by-cargo-fmt/)
- [~12% of HTML is visible text (~15,971 est. tokens)](https://puerto-rs.vercel.app/docs/guides/agents/)
  - from: [/docs/guides/agents/](https://puerto-rs.vercel.app/docs/guides/agents/)
- [~10% of HTML is visible text (~17,777 est. tokens)](https://puerto-rs.vercel.app/docs/guides/auth/)
  - from: [/docs/guides/auth/](https://puerto-rs.vercel.app/docs/guides/auth/)
- [~9% of HTML is visible text (~15,135 est. tokens)](https://puerto-rs.vercel.app/docs/guides/configuration/)
  - from: [/docs/guides/configuration/](https://puerto-rs.vercel.app/docs/guides/configuration/)
- [~9% of HTML is visible text (~18,011 est. tokens)](https://puerto-rs.vercel.app/docs/guides/database/)
  - from: [/docs/guides/database/](https://puerto-rs.vercel.app/docs/guides/database/)
- [~4% of HTML is visible text (~48,003 est. tokens)](https://puerto-rs.vercel.app/docs/guides/entities/)
  - from: [/docs/guides/entities/](https://puerto-rs.vercel.app/docs/guides/entities/)
- [~12% of HTML is visible text (~22,551 est. tokens)](https://puerto-rs.vercel.app/docs/guides/events/)
  - from: [/docs/guides/events/](https://puerto-rs.vercel.app/docs/guides/events/)
- [~7% of HTML is visible text (~12,302 est. tokens)](https://puerto-rs.vercel.app/docs/guides/layer-by-layer/)
  - from: [/docs/guides/layer-by-layer/](https://puerto-rs.vercel.app/docs/guides/layer-by-layer/)
- [~7% of HTML is visible text (~22,261 est. tokens)](https://puerto-rs.vercel.app/docs/guides/mail/)
  - from: [/docs/guides/mail/](https://puerto-rs.vercel.app/docs/guides/mail/)
- [~9% of HTML is visible text (~14,810 est. tokens)](https://puerto-rs.vercel.app/docs/guides/testing/)
  - from: [/docs/guides/testing/](https://puerto-rs.vercel.app/docs/guides/testing/)
- [~8% of HTML is visible text (~13,062 est. tokens)](https://puerto-rs.vercel.app/docs/guides/upgrading/)
  - from: [/docs/guides/upgrading/](https://puerto-rs.vercel.app/docs/guides/upgrading/)
- [~9% of HTML is visible text (~23,825 est. tokens)](https://puerto-rs.vercel.app/docs/guides/use-cases/)
  - from: [/docs/guides/use-cases/](https://puerto-rs.vercel.app/docs/guides/use-cases/)
- [~8% of HTML is visible text (~16,944 est. tokens)](https://puerto-rs.vercel.app/docs/guides/value-objects/)
  - from: [/docs/guides/value-objects/](https://puerto-rs.vercel.app/docs/guides/value-objects/)
- [~8% of HTML is visible text (~20,279 est. tokens)](https://puerto-rs.vercel.app/docs/guides/web/)
  - from: [/docs/guides/web/](https://puerto-rs.vercel.app/docs/guides/web/)
- [~13% of HTML is visible text (~19,341 est. tokens)](https://puerto-rs.vercel.app/docs/reference/cli/)
  - from: [/docs/reference/cli/](https://puerto-rs.vercel.app/docs/reference/cli/)
- [~10% of HTML is visible text (~12,416 est. tokens)](https://puerto-rs.vercel.app/docs/reference/errors/)
  - from: [/docs/reference/errors/](https://puerto-rs.vercel.app/docs/reference/errors/)
- [~11% of HTML is visible text (~12,784 est. tokens)](https://puerto-rs.vercel.app/docs/reference/field-types/)
  - from: [/docs/reference/field-types/](https://puerto-rs.vercel.app/docs/reference/field-types/)
- [~12% of HTML is visible text (~10,311 est. tokens)](https://puerto-rs.vercel.app/docs/reference/make/)
  - from: [/docs/reference/make/](https://puerto-rs.vercel.app/docs/reference/make/)
- [~9% of HTML is visible text (~15,697 est. tokens)](https://puerto-rs.vercel.app/docs/reference/puerto-toml/)
  - from: [/docs/reference/puerto-toml/](https://puerto-rs.vercel.app/docs/reference/puerto-toml/)
- [~10% of HTML is visible text (~11,607 est. tokens)](https://puerto-rs.vercel.app/docs/reference/runtime-crates/)
  - from: [/docs/reference/runtime-crates/](https://puerto-rs.vercel.app/docs/reference/runtime-crates/)
- [~3% of HTML is visible text (~60,295 est. tokens)](https://puerto-rs.vercel.app/docs/start/anatomy/)
  - from: [/docs/start/anatomy/](https://puerto-rs.vercel.app/docs/start/anatomy/)
- [~12% of HTML is visible text (~16,557 est. tokens)](https://puerto-rs.vercel.app/docs/start/architecture/)
  - from: [/docs/start/architecture/](https://puerto-rs.vercel.app/docs/start/architecture/)
- [~7% of HTML is visible text (~12,741 est. tokens)](https://puerto-rs.vercel.app/docs/start/installation/)
  - from: [/docs/start/installation/](https://puerto-rs.vercel.app/docs/start/installation/)
- [~9% of HTML is visible text (~16,221 est. tokens)](https://puerto-rs.vercel.app/docs/start/quick-start/)
  - from: [/docs/start/quick-start/](https://puerto-rs.vercel.app/docs/start/quick-start/)

</details>

---

## Entities

_The site's own JSON-LD, collapsed into one graph. Informational, not part of the score._

| Metric | Value |
| --- | ---: |
| Entities | 6 |
| References | 7 |
| Dangling references | 0 |
| Pages crawled | 48 |
| Pages with no entities | 47 |
| Entities with a stable @id | 5 (83%) |

### Largest entities

| Type | Name | @id | Occurrences | Pages |
| --- | --- | --- | ---: | ---: |
| Offer | (unnamed Offer) | _none_ | 1 | 1 |
| SoftwareSourceCode | Puerto | https://github.com/Guuri11/Puerto#source | 1 | 1 |
| Person | Sergio Gurillo | https://puerto-rs.vercel.app/#author | 1 | 1 |
| SoftwareApplication | Puerto | https://puerto-rs.vercel.app/#software | 1 | 1 |
| WebSite | Puerto | https://puerto-rs.vercel.app/#web | 1 | 1 |

### Conflicting properties (0)

No entity disagrees with itself across pages.

### Dangling references (0)

Every `@id` reference points at an entity some page declares.

### Entities without an @id (0)

Every entity declared on more than one page carries an `@id`.

---

*Generated by [squirrelscan](https://squirrelscan.com) v0.0.111*