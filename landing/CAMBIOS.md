# Cambios — web de Puerto

Log de modificaciones. Lo más reciente arriba.

## 2026-11-16 — Puerto 0.21: un crate por contexto, lo generado en castellano

- La cosecha (`harvest.mjs`) y la captura del compilador (`capture-compiler.mjs`) usan el CLI en
  castellano (`puerto nuevo`, `generar scaffold`, `--eventos`…) y las cargas `Producto`, `Pedido`,
  `Cliente` en el proyecto `tienda`.
- La terminal agrupa los ficheros por directorio real (`contextos/producto/dominio/`, `nucleo/`,
  `presentacion/api/`…, `src/lib/groups.ts`, compartido por el servidor y el navegador), con el
  color del anillo. El `LayerStack` enseña los cuatro crates de un contexto.
- `sync-repo-docs.mjs` y `repo.ts` leen también las ADR en castellano (`**Estado:**`,
  `## Contexto`).
- La home: una línea nueva en el manifiesto («In Spanish»); los comandos, en castellano.
- Las 22 páginas de docs, al vocabulario de la 0.21; la prosa sigue en inglés.

## 2026-10-13 — Lanzamiento

- Desplegada en producción (`1904402`): squirrel 83/100 (B) frente a 58 (F) del baseline.
- Marca del README y de `assets/brand/` rehecha con la identidad nueva (`pnpm brand:repo`).
- El dominio se queda en `puerto-rs.vercel.app` (decisión de Sergio).
- Corregidos dos textos pegados a un enlace o un `<code>` («written by», «built by»): Astro se
  come el salto de línea entre texto y elemento, así que el espacio va explícito con `{' '}`.

## 2026-10-13 — Web nueva (carril A, primera versión)

Sustituye entera a la landing de abril (Astro 6, dos páginas). Lo que queda en el historial de
git es esa versión.

**Hecho**
- Identidad nueva: símbolo de tres contenedores idénticos, wordmark en Big Shoulders Stencil,
  paleta hormigón + colores de capa + amarillo grúa, Archivo y Martian Mono. `DESIGN.md`.
- Home: hero, terminal interactiva (66 combinaciones reales del CLI, resaltadas en build), regla
  de dependencias, manifiesto de capacidades, sección de agentes con un error real de `rustc`,
  los ADRs, y cómo empezar.
- Docs con Starlight: 5 páginas de inicio, 13 guías, 6 referencias (CLI y `make` generadas),
  los 21 ADRs y el changelog leídos del repo.
- Cada ejemplo de código de las guías que no sale de la cosecha se ha compilado y testeado en un
  proyecto generado (`reprice_product` en *Use cases*, la unit of work en *Domain events*).
- Scripts: `harvest`, `capture:compiler`, `brand:images`, `sync-repo-docs`. En Harbor,
  `make site/data` y su línea en el checklist de release.
- 216 E2E en verde (escritorio y móvil). Auditoría local 79/100 frente a 58 del baseline.

**Decidido en la marcha (y por qué)**
- El logo iba a formar una **P** con los contenedores: con barras horizontales se lee F. Se
  cambió a tres contenedores idénticos apilados, que además cuenta la tesis (el estándar).
- Fuera los códigos de muelle decorativos (`A-01`…) que se propusieron: el antislop prohíbe los
  eyebrows numerados sin secuencia real.
- La versión aparece junto al comando de instalación y en el pie, y en ningún sitio más. El
  antislop la prohíbe como decoración; aquí es información (excepción escrita en `DESIGN.md`).

**Para la siguiente vuelta (tramo 4)**: enseñarla desplegada y preguntar qué chirría.
