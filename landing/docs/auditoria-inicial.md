# squirrelscan Audit Report

**URL:** https://puerto-rs.vercel.app  
**Date:** 2026-10-13T07:23:27.339Z  
**Pages:** 3  
Scan: 3 pages crawled from the CLI (page limit 100).  
**Version:** 0.0.97

## Health Score

| Category | Score |
|----------|-------|
| **Overall** | **58/100 (F)** |
| **SEO** | **70/100** |
| **Performance** | **94/100** |
| **Security** | **86/100** |
| **Agents** | **97/100** |
| Content | 82/100 |
| Crawlability | 89/100 |
| Performance | 94/100 |
| E-E-A-T | 53/100 |
| Core SEO | 87/100 |
| Accessibility | 98/100 |
| Security | 89/100 |
| Legal Compliance | 44/100 |
| Agent Experience | 97/100 |
| Internationalization | 100/100 |
| Images | 100/100 |
| Site Integrity | 100/100 |
| Links | 100/100 |
| Mobile | 100/100 |
| Social Media | 100/100 |
| URL Structure | 100/100 |

## Technologies

_Detected tech stack — informational, not part of the score. No changes since last scan._

**☁️ Hosting**  
<img src="https://squirrelscan.com/tech-icons/vercel.png" alt="Vercel" width="14" height="14" /> Vercel

**🌐 CDN & Edge**  
<img src="https://squirrelscan.com/tech-icons/vercel.png" alt="Vercel Edge Network" width="14" height="14" /> Vercel Edge Network

## Summary

- **Passed:** 345
- **Warnings:** 29
- **Failed:** 3
- **SEO:** 70/100
- **Performance:** 94/100
- **Security:** 86/100
- **Agents:** 97/100

---

## Issues

### Errors

*1 rule(s)*

#### Crawlability

##### Robots.txt **[ERROR]**

`crawl/robots-txt`

> Checks if robots.txt exists and is properly configured

**Solution:**

robots.txt tells search engines which pages to crawl. Place it at the root of your domain (example.com/robots.txt). Include your sitemap URL. Avoid blocking important resources (CSS, JS, images) that search engines need to render pages. Never use 'Disallow: /' unless you want to block all crawling. Use Google Search Console to test your robots.txt.

| Check | Status | Message |
|-------|--------|---------|
| robots-txt-exists | X fail | No robots.txt found |

---

### Recommendations

*3 rule(s)*

#### Legal Compliance

##### Sub-processor Disclosure *[Recommendation]*

`legal/subprocessor-disclosure`

> Checks for a sub-processor / data-processing (DPA) disclosure page or link

**Solution:**

Under GDPR Art. 28, processors must disclose the sub-processors they engage and offer a Data Processing Agreement (DPA). Publish a /subprocessors page listing each third party that handles customer personal data (purpose, location), keep it current, and link a DPA from your legal/trust pages. B2B SaaS and fintech buyers expect this during security review.

| Check | Status | Message |
|-------|--------|---------|
| subprocessor-disclosure | ! warn | No sub-processor / data-processing (DPA) disclosure found |

---

#### Agent Experience

##### llms.txt *[Recommendation]*

`ax/llms-txt`

> Detects /llms.txt (and /llms-full.txt) at the domain root and checks its basic Markdown format — an emerging standard giving AI agents a curated, machine-readable map of your site

**Solution:**

llms.txt (llmstxt.org) is a Markdown file at your domain root that points AI agents and answer engines at your most useful, clean content. Add a /llms.txt with an H1 title, a short summary, and sections of Markdown links to key pages; optionally add /llms-full.txt with that content inlined. This is a recommendation only — it never affects your score.

| Check | Status | Message |
|-------|--------|---------|
| llms-txt-present | ! warn | No /llms.txt found — consider adding one so AI agents can discover your key content |

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

*19 rule(s)*

#### Core SEO

##### Meta Title **[WARN]**

`core/meta-title`

> Validates page title presence and length

**Solution:**

Every page needs a unique, descriptive title tag between 30-75 characters. Titles appear in browser tabs, search results, and social shares. Write titles that accurately describe the page content while including your primary keyword near the beginning. If your title is too short, add more descriptive context. If too long, prioritize the most important information first and trim secondary details. Avoid keyword stuffing or duplicate titles across pages.

| Check | Status | Message |
|-------|--------|---------|
| meta-title | ! warn | Title too short |

<details><summary><strong>meta-title:</strong> 2 page(s) affected</summary>

- [/docs](https://puerto-rs.vercel.app/docs)
- [/docs/](https://puerto-rs.vercel.app/docs/)

</details>

<details><summary><strong>meta-title:</strong> 2 item(s)</summary>

- [Getting Started — Puerto (24 chars)](https://puerto-rs.vercel.app/docs)
  - from: [/docs](https://puerto-rs.vercel.app/docs)
- [Getting Started — Puerto (24 chars)](https://puerto-rs.vercel.app/docs/)
  - from: [/docs/](https://puerto-rs.vercel.app/docs/)

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

<details><summary><strong>meta-description:</strong> 2 page(s) affected</summary>

- [/docs](https://puerto-rs.vercel.app/docs)
- [/docs/](https://puerto-rs.vercel.app/docs/)

</details>

<details><summary><strong>meta-description:</strong> 2 item(s)</summary>

- [Install Puerto and scaffold your first Rust DDD pr (74 chars)](https://puerto-rs.vercel.app/docs)
  - from: [/docs](https://puerto-rs.vercel.app/docs)
- [Install Puerto and scaffold your first Rust DDD pr (74 chars)](https://puerto-rs.vercel.app/docs/)
  - from: [/docs/](https://puerto-rs.vercel.app/docs/)

</details>

---

##### Title Uniqueness **[WARN]**

`core/title-unique`

> Checks that page titles are unique across the site

**Solution:**

Each page should have a unique title that accurately describes its content. Duplicate titles confuse search engines and users about which page to display. Use a pattern like 'Page Topic | Brand Name' to ensure uniqueness. CMS often generate duplicate titles - audit and customize them.

| Check | Status | Message |
|-------|--------|---------|
| title-unique | ! warn | 1 duplicate title(s) affecting 2 pages |

<details><summary><strong>title-unique:</strong> 2 page(s) affected</summary>

- [/docs](https://puerto-rs.vercel.app/docs)
- [/docs/](https://puerto-rs.vercel.app/docs/)

</details>

<details><summary><strong>title-unique:</strong> 1 item(s)</summary>

- "getting started — puerto..." (2 pages)
  - from: [/docs](https://puerto-rs.vercel.app/docs)
  - from: [/docs/](https://puerto-rs.vercel.app/docs/)

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
| csp-missing | ! warn | No Content-Security-Policy header |

---

##### X-Frame-Options **[WARN]**

`security/x-frame-options`

> Checks for clickjacking protection header

**Solution:**

X-Frame-Options prevents your site from being embedded in iframes, protecting against clickjacking attacks. Set: X-Frame-Options: DENY (no framing) or SAMEORIGIN (same origin only). For modern browsers, CSP frame-ancestors is preferred: Content-Security-Policy: frame-ancestors 'self'. Use both for maximum compatibility.

| Check | Status | Message |
|-------|--------|---------|
| x-frame-options | ! warn | No clickjacking protection |

---

#### Content

##### Duplicate Title **[WARN]**

`content/duplicate-title`

> Checks for duplicate title tags across the site

**Solution:**

Each page should have a unique title tag that accurately describes its content. Duplicate titles confuse search engines about which page to rank and make your pages less distinguishable in search results. Use unique, descriptive titles that include relevant keywords. For similar pages (e.g., pagination), add differentiating elements like page numbers or category names.

| Check | Status | Message |
|-------|--------|---------|
| duplicate-title | ! warn | 1 duplicate title(s) found across 2 pages |

<details><summary><strong>duplicate-title:</strong> 2 page(s) affected</summary>

- [/docs](https://puerto-rs.vercel.app/docs)
- [/docs/](https://puerto-rs.vercel.app/docs/)

</details>

<details><summary><strong>duplicate-title:</strong> 1 item(s)</summary>

- "getting started — puerto..." (2 pages)
  - from: [/docs](https://puerto-rs.vercel.app/docs)
  - from: [/docs/](https://puerto-rs.vercel.app/docs/)

</details>

---

##### Placeholder Text **[WARN]**

`content/placeholder-text`

> Detects template leftovers and filler copy that shipped to production

**Solution:**

Each family points at a different break in the publishing pipeline. Lorem ipsum and theme boilerplate mean a page was published before its copy was written: replace the text, and add the affected fields to whatever check gates publishing. Unrendered template syntax means the templating engine never ran over that string, usually because the value was interpolated into an already-escaped fragment or the template was served as static HTML: render it server-side, or delete the stale copy. A visible undefined, NaN, null or [object Object] means the value was missing and the code concatenated it into the copy anyway: guard the field at the point of render rather than in CSS. TODO and FIXME markers mean a draft shipped: finish or remove the note, since search engines and readers see it exactly as written.

| Check | Status | Message |
|-------|--------|---------|
| placeholder-content | X fail | 4 placeholder occurrence(s) in visible text: unrendered template syntax ([[entity]]) |

<details><summary><strong>placeholder-content:</strong> 2 page(s) affected</summary>

- [/docs](https://puerto-rs.vercel.app/docs)
- [/docs/](https://puerto-rs.vercel.app/docs/)

</details>

<details><summary><strong>placeholder-content:</strong> 1 item(s)</summary>

- unrendered template syntax
  - from: [/docs](https://puerto-rs.vercel.app/docs)
  - from: [/docs/](https://puerto-rs.vercel.app/docs/)

</details>

---

##### Duplicate Description **[WARN]**

`content/duplicate-description`

> Checks for duplicate meta descriptions across the site

**Solution:**

Each page should have a unique meta description that summarizes its specific content. Duplicate descriptions reduce click-through rates and provide poor user experience in search results. Write unique, compelling descriptions for each page. For pages without unique content (like paginated results), consider using no description rather than a duplicate.

| Check | Status | Message |
|-------|--------|---------|
| duplicate-description | ! warn | 1 duplicate description(s) found across 2 pages |

<details><summary><strong>duplicate-description:</strong> 2 page(s) affected</summary>

- [/docs](https://puerto-rs.vercel.app/docs)
- [/docs/](https://puerto-rs.vercel.app/docs/)

</details>

<details><summary><strong>duplicate-description:</strong> 1 item(s)</summary>

- "install puerto and scaffold your first r..." (2 pages)
  - from: [/docs](https://puerto-rs.vercel.app/docs)
  - from: [/docs/](https://puerto-rs.vercel.app/docs/)

</details>

---

#### Performance

##### LCP Optimization Hints **[WARN]**

`perf/lcp-hints`

> Checks for Largest Contentful Paint optimization opportunities

**Solution:**

LCP measures when the largest content element becomes visible. Optimize by: 1) Preload your LCP image with <link rel='preload' as='image'>. 2) Don't use loading='lazy' on above-fold images as it delays loading. 3) Minimize render-blocking CSS/JS in <head>. 4) Use modern image formats (WebP/AVIF) for faster loading. 5) Consider using fetchpriority='high' on the LCP image.

| Check | Status | Message |
|-------|--------|---------|
| lcp-preload | ! warn | 2 likely-LCP images loaded without preload |

<details><summary><strong>lcp-preload:</strong> 3 page(s) affected</summary>

- [/](https://puerto-rs.vercel.app/)
- [/docs](https://puerto-rs.vercel.app/docs)
- [/docs/](https://puerto-rs.vercel.app/docs/)

</details>

---

##### Critical Request Chains **[WARN]**

`perf/critical-request-chains`

> Identifies chains of dependent resources that delay rendering

**Solution:**

Critical request chains are sequences of dependent network requests that must complete before the page can render. Reduce chain depth by: 1) Inlining critical CSS instead of linking external files. 2) Adding async or defer to non-critical scripts. 3) Avoiding CSS @import — use <link> tags instead. 4) Using <link rel='preload'> for critical resources. 5) Reducing the number of render-blocking resources in <head>.

| Check | Status | Message |
|-------|--------|---------|
| critical-request-chains | ! warn | 1 critical request chain(s) found |

<details><summary><strong>critical-request-chains:</strong> 3 page(s) affected</summary>

- [/](https://puerto-rs.vercel.app/)
- [/docs](https://puerto-rs.vercel.app/docs)
- [/docs/](https://puerto-rs.vercel.app/docs/)

</details>

<details><summary><strong>critical-request-chains:</strong> 1 item(s)</summary>

- CSS: /_astro/BaseLayout.oA0fIT-p.css
  - from: [/](https://puerto-rs.vercel.app/)
  - from: [/docs](https://puerto-rs.vercel.app/docs)
  - from: [/docs/](https://puerto-rs.vercel.app/docs/)

</details>

---

#### Accessibility

##### Table Headers **[WARN]**

`a11y/table-headers`

> Checks that data tables have proper headers

**Solution:**

Data tables need proper headers for screen reader users to understand relationships. Use <th> for header cells, not styled <td>. Add scope='col' or scope='row' to clarify header direction. For complex tables, use id and headers attributes to associate data cells with headers. Include a <caption> to describe the table's purpose. Layout tables should have role='presentation'.

| Check | Status | Message |
|-------|--------|---------|
| table-headers | ! warn | 2 table(s) without <th> headers |

<details><summary><strong>table-headers:</strong> 2 page(s) affected</summary>

- [/docs](https://puerto-rs.vercel.app/docs)
- [/docs/](https://puerto-rs.vercel.app/docs/)

</details>

<details><summary><strong>table-headers:</strong> 2 item(s)</summary>

- Table 4
  - from: [/docs](https://puerto-rs.vercel.app/docs)
  - from: [/docs/](https://puerto-rs.vercel.app/docs/)
- Table 6
  - from: [/docs](https://puerto-rs.vercel.app/docs)
  - from: [/docs/](https://puerto-rs.vercel.app/docs/)

</details>

---

##### Table Duplicate Name **[WARN]**

`a11y/table-duplicate-name`

> Checks that data tables have unique accessible names

**Solution:**

When a page has multiple data tables, each should have a unique accessible name to help users distinguish between them. Use <caption>, aria-label, or aria-labelledby with unique text for each table.

| Check | Status | Message |
|-------|--------|---------|
| tables-without-names | ! warn | 6 table(s) without accessible names |

<details><summary><strong>tables-without-names:</strong> 2 page(s) affected</summary>

- [/docs](https://puerto-rs.vercel.app/docs)
- [/docs/](https://puerto-rs.vercel.app/docs/)

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

##### Content Dates **[WARN]**

`eeat/content-dates`

> Checks for published and modified dates on content

**Solution:**

Visible dates show content freshness and help users assess relevance. Include datePublished and dateModified in Article schema. Show human-readable dates on pages. Update dateModified when making significant changes. Fresh content signals ongoing maintenance and expertise. Stale dates may hurt rankings for time-sensitive topics.

| Check | Status | Message |
|-------|--------|---------|
| date-published | ! warn | No content pages have datePublished |

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

<details><summary><strong>token-weight-ratio:</strong> 1 page(s) affected</summary>

- [/](https://puerto-rs.vercel.app/)

</details>

<details><summary><strong>token-weight-ratio:</strong> 1 item(s)</summary>

- [~15% of HTML is visible text (~7,710 est. tokens)](https://puerto-rs.vercel.app/)
  - from: [/](https://puerto-rs.vercel.app/)

</details>

---

## Entities

This site declares no JSON-LD entities, so search engines have nothing to reconcile it into.

---

*Generated by [squirrelscan](https://squirrelscan.com) v0.0.97*