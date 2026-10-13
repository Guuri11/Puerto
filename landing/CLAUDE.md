# Puerto — la web (landing + docs)

Proyecto propio de Sergio Gurillo, construido con el carril A de web-consultors el 2026-10-13.
El framework vive en `..` (Harbor). Esta carpeta es su web: home y documentación.

- **Web:** https://puerto-rs.vercel.app (sin dominio propio: `PENDIENTE.md`)
- **Brief:** `brief.md` · **Contrato visual:** `DESIGN.md` · **Lo que falta:** `PENDIENTE.md`
- **Baseline:** `docs/auditoria-inicial.md` (58/100, 2026-10-13, la landing de abril)

## Stack

Astro 7 + **Starlight** (las docs) + Tailwind 4 (solo la home) + GSAP. pnpm. Playwright en el
puerto **4347**: `pnpm test:e2e`, nunca `playwright test` a pelo.

## La regla de esta web: nada se escribe dos veces

Todo lo que se puede leer del repo o del CLI se lee. Si un dato aparece escrito a mano en un
`.astro` o un `.mdx` y existe en otro sitio, está mal.

| Dato | Fuente | Cómo llega |
|---|---|---|
| Versión, Rust mínimo, ADRs | `../Cargo.toml`, `../docs/adr/` | `src/lib/repo.ts`, en build |
| Páginas de ADRs, changelog | `../docs/adr/*.md`, `../CHANGELOG.md` | `scripts/sync-repo-docs.mjs` (antes de `dev`/`build`) |
| Referencia del CLI y de `make` | `--help` real, Makefiles generados | `sync-repo-docs.mjs` desde la cosecha |
| La terminal de la home | 66 proyectos generados de verdad | `pnpm harvest` → `public/cargo/`, `src/data/harvest/` |
| El error de compilador (agentes) | `cargo check` real | `pnpm capture:compiler` |

Lo que escribe `sync-repo-docs.mjs` está en `.gitignore`: **no se edita**, se edita la fuente.

La marca del repositorio (`../assets/brand/`, la del README) también sale de aquí:
`pnpm brand:repo`. La imagen social de la web, `pnpm brand:images`.

**Tras cada release de Puerto:** `make site/data` desde `..` (compila el CLI, cosecha y captura).
Un test (`accesibilidad.spec.ts › La verdad de los datos`) falla mientras la cosecha sea de
otra versión.

## Decisiones (con motivo)

- **Identidad «puerto industrial»** (Sergio, 2026-10-13): contenedores, hormigón, rotulación.
  Fuera el naranja y el oscuro de todo el ecosistema de Rust. Detalle en `DESIGN.md`.
- **Starlight para las docs** (Sergio): búsqueda, navegación y TOC de serie. Se cambian tokens
  y tipografía, no la estructura, para que actualizar Starlight no rompa nada.
- **La terminal enseña salidas reales precalculadas** (Sergio): ni plantillas en JS (dos
  generadores que divergen) ni WASM (los generadores escriben en disco y pasan `cargo fmt`).
- **Resaltado en build** (`src/pages/cargo/hl/[hash].html.ts`): el navegador no carga Shiki.
- **Los estilos de la terminal son globales** (`is:global`): el árbol y el visor se pintan con
  `innerHTML`, y el CSS con scope de Astro no llega a ese HTML. Todo va prefijado `term__`.
- **El dominio es `puerto-rs.vercel.app`**, por decisión de Sergio (2026-10-13), aunque
  `rules/astro.md` pide dominio propio. Es el definitivo, no uno provisional.
  Si algún día cambia: `astro.config.mjs`, `src/config/site.ts`, `public/robots.txt`,
  `public/llms.txt`, los `@id` de `site.schema` y `DOMINIO` en los tests.
- **`vercel.json` sin el `noindex` para `*.vercel.app`** que trae el starter: aquí `.vercel.app`
  ES producción, y esa cabecera sacaría la web de Google.
- **Sin páginas legales ni cookies**: no se recoge ningún dato.
- **Un solo tema de código** (noche) en toda la web, también en el modo claro de las docs.

## Lo que la auditoría marca y se acepta

Medido en local el 2026-10-13: **79/100** frente al 58 del baseline.
- `token-weight` y «Agents 50»: el HTML de cada página de Starlight lleva la barra lateral
  entera. Es de Starlight; `llms.txt` es la vía para agentes.
- `placeholder-content` (`[[entity]]`): es sintaxis TOML dentro de un bloque de código.
- `label-content-name-mismatch`: el botón de búsqueda de Starlight («Search» / «Search Ctrl K»).
- `broken-external-links` a crates.io: crates.io responde 404 a clientes sin JS; el crate existe.
- `csp`, `x-frame-options`, `sitemap-domain`: artefactos de auditar `localhost`; en Vercel los pone
  `vercel.json`.
