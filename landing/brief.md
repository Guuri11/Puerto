# Brief — Puerto

> Carril A, 2026-10-13. Sustituye al brief de abril (v1 de la landing, en el historial de git).

## Qué es

Puerto es un framework full-stack de Rust, open source (MIT), de Sergio Gurillo. Lleva la
experiencia de Rails y Laravel —un comando y tienes la pieza entera— a un proyecto con
**DDD y Clean Architecture**. Es un generador (`puerto` CLI) más unos crates de runtime
(`puerto-core`, `-web`, `-sqlx`, `-events`, `-kafka`, `-mail`, `-auth`, `-macros`) que
se actualizan con `cargo update`.

Estado el 2026-10-13: **v0.19.0**, 155 commits en seis meses, 16 fases cerradas, 21 ADRs.
De abril para acá ha pasado de «genera tres capas» a tener eventos de dominio con outbox
(Postgres o Kafka), mail, auth OIDC o local, frontend React tipado desde OpenAPI, ids
tipados, paginación y crates de runtime. La web de abril no cuenta casi nada de eso.

## Para quién

Desarrolladores de Rust de nivel medio-alto que vienen de Rails, Laravel o Django y echan
de menos la productividad. Y, cada vez más, **gente que programa con agentes**: Puerto
entrega cada proyecto con `AGENTS.md` y `CLAUDE.md` testados, y su tesis es que el
compilador de Rust es el mejor bucle de corrección que puede tener un agente.

Lo que buscan al llegar, en orden:
1. ¿Qué es y en qué se diferencia de Loco o de montar Axum a mano?
2. ¿Qué genera de verdad? (y que no sea un juguete)
3. ¿Aguanta un proyecto real? (eventos, auth, mail, tests)
4. ¿Puedo fiarme? (decisiones escritas, versionado, licencia)
5. ¿Cómo empiezo?

## Competencia y referentes

| Proyecto | Qué hace bien | Qué hace mal / dónde nos separamos |
|---|---|---|
| **Loco** (loco.rs) | El referente directo: «Rails para Rust». Docs extensas, buen quick start | Estética de terminal oscura y naranja, la de todo el ecosistema. MVC con modelos tipo ActiveRecord (SeaORM): no hay capa de dominio |
| **Rocket** (rocket.rs) | La guía como libro, de lo mejor del ecosistema | Un framework web, no un generador de arquitectura |
| **Pavex** | Rigor técnico, docs muy cuidadas | Requiere nightly; no genera dominio |
| **Laravel / Rails** | Docs de referencia en todo el sector: cada tarea, su página | — es el listón de las docs |

Todos los proyectos de Rust se parecen: oscuro, naranja, mono. **Ese es el argumento para
no hacerlo** (`/design-system`, pregunta 2).

## Forma del sitio

**Multipágina.** Home + documentación (Starlight).

- La home **presenta y enlaza**: qué es, la terminal con salida real, la regla de
  dependencias, el manifiesto de capacidades (una línea cada una, enlazando a su página),
  el argumento de agentes, las decisiones y cómo empezar.
- Cada tema tiene su página en `/docs/`. Ningún bloque de la home repite una página interior:
  el manifiesto da una línea por capacidad y el detalle vive en su guía.

## Contenido y fuentes de verdad

Nada se escribe dos veces si se puede leer del repo:

| Qué | De dónde |
|---|---|
| Versión, nº de ADRs | `../Cargo.toml`, `../docs/adr/` en build |
| ADRs (`/docs/decisions/…`) | `../docs/adr/*.md`, renderizados tal cual |
| Changelog | `../CHANGELOG.md` |
| Referencia del CLI | `--help` real de cada comando (`pnpm harvest`) |
| La terminal de la home | 66 proyectos generados por el CLI real (`pnpm harvest`) |
| Error del compilador (sección agentes) | capturado de `cargo check` sobre un proyecto generado |

## Decisiones cerradas con Sergio (2026-10-13)

1. Docs con **Starlight**, personalizado con la identidad (no plantilla).
2. **Reemplaza `Harbor/landing`**: mismo proyecto de Vercel.
3. **Identidad nueva**, dirección «puerto industrial» (ver `DESIGN.md`).
4. Pieza estrella: **terminal interactiva** con salidas reales precalculadas.
5. Salidas **reales en build** (no plantillas en JS, no WASM por ahora).

## Idioma

Solo inglés: la comunidad de Rust es global. Los documentos internos (`brief`, `DESIGN`,
`CLAUDE`, `PENDIENTE`, `CAMBIOS`) en español, como el resto del marco.

## Módulos del kit

Ninguno. Sin `i18n` (un idioma), sin `panel` (no hay datos), sin Relieve (no hay escena 3D:
la terminal es la pieza). Sin formulario, sin cookies, sin analítica. Sin páginas legales:
no se recoge ningún dato.

## Keywords

Intención informativa y de comparación: `rust web framework`, `rust rails`,
`rust ddd framework`, `rust clean architecture`, `rust scaffold generator`,
`loco alternative`. Las guías de docs posicionan por cola larga
(`rust outbox pattern`, `rust domain events`, `poem openapi crud`).

## Baseline

`docs/auditoria-inicial.md` — squirrel sobre `puerto-rs.vercel.app`, 2026-10-13:
**58/100 (F)**. SEO 70, E-E-A-T 53, legal 44, rendimiento 94, accesibilidad 98.
