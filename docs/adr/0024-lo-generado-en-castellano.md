# 0024 — Lo que genera Puerto, en castellano

**Estado:** Aceptada (2026-11-14). Especificación: `PLAN.md` § Fase 19. El vocabulario, en
`docs/glosario.md`. Modifica la ADR-0007 (el JSON de error pasa a `{ codigo, mensaje }`) y la
ADR-0017 (cambian de nombre `puerto-core`, `puerto-events` y `puerto-mail`).
**Precedentes:** ADR 0005 de `digitalia-dev` (el repositorio en castellano) y ADR-0054 de
`nexe-document` (la estructura en castellano).

## Contexto

Puerto es la herramienta de Sergio para sus propios proyectos, no un producto internacional. Esos
proyectos ya se escriben en castellano: se habla de «transportista» y no de «carrier», y cada
término en inglés es una traducción que se hace de cabeza al leer (ADR 0005 de digitalia). Un
proyecto generado en inglés choca con ellos desde el primer fichero.

## Decisión

**Todo lo que genera Puerto va en castellano y sin tildes ni ñ en los identificadores** (`anio`,
`Autorizacion`): estructura, tipos, funciones, campos, tablas, rutas REST, JSON, códigos de error,
tipos de evento, tests, logs, comentarios y el `AGENTS.md` de cada proyecto. También la API y los
nombres de los crates del runtime, el esquema de `puerto.toml` y los comandos de la CLI. Cargo solo
admite ASCII en el nombre de un crate, y la regla se aplica igual a todo para que no haya dos.

Es más estricta que la ADR 0005 de digitalia: los patrones también van en castellano
(`RepositorioProducto`, `CrearProducto`, `MadreProducto`). Solo quedan en inglés estas
excepciones:

1. **Lo que impone el lenguaje o una librería:** `src`, `lib.rs`, `main.rs` y `build.rs`; los traits
   de std (`From`, `TryFrom`, `FromStr`, `Display`, `Default`) y sus métodos; los prefijos de
   convención de Rust (`new`, `from_`, `to_`, `as_`, `into_`); el prefijo `Mock` que pone mockall;
   el prefijo `use` de los hooks de React; los sufijos `_pkey`, `_key`, `_fkey` e `_idx` de
   Postgres; `DATABASE_URL`, que lee sqlx.
2. **La jerga del oficio bien asentada:** `id`, `uuid`, `Dto`, `Api`, `rest`, `sql`, `outbox`,
   `worker`, `token`, `hash`, `health`, `Params`, `Props`, `Impl`, `Snapshot`, `scaffold`,
   `bootstrap`, `auth`, `db`, `snippets`, y la unidad de trabajo con su vocabulario
   (`UnitOfWork`, `begin`, `commit`, `rollback`).
3. **Las variables de entorno:** son la convención de operaciones y del compose, y `DATABASE_URL`
   viene impuesta.
4. **La sintaxis de los campos en la CLI** (`opt:`, `vec:`, `ref:`, `enum:`, `map`, `!`) y los
   tipos del registro (`String`, `i64`, `DateTime<Utc>`…), que son tipos de Rust.

**El orden de las palabras es el natural:** primero el concepto y después la entidad
(`RepositorioProducto`, `IdProducto`, `ErrorProducto`, `EventoProducto`). **El trait lleva el
nombre del concepto, y la implementación dice cómo lo hace:** `RepositorioProducto`,
`RepositorioProductoPostgres` y `RepositorioProductoEnMemoria`. En los casos de uso, el trait es
`CrearProducto` y la implementación `CrearProductoImpl`. Desaparece el sufijo `Trait`.

**Lo serializado también sale en castellano**: tablas, columnas, rutas, campos JSON, códigos de
error y tipos de evento. La ADR 0005 de digitalia los deja en inglés porque ya hay apps y partners
que dependen de ellos; un proyecto recién generado no tiene contratos previos. Salen del nombre de
la entidad, que escribe el usuario, y del glosario. Por eso Puerto necesita pluralizar en
castellano (`ciudad` → `ciudades`, `lapiz` → `lapices`).

**Una palabra nueva se decide antes de usarla.** Si no está en `docs/glosario.md`, se pregunta a
Sergio, se añade al glosario y solo después se usa. El glosario es el que se mantiene vivo; esta
ADR no se reescribe.

## Lo que no cambia

- **El código fuente de Puerto** (`generators/`, `patchers/`…) sigue en inglés. Traducirlo es mucho
  trabajo y no se nota en lo que se genera.
- **La documentación de Puerto, la web, el README y los mensajes de la CLI** pasan a castellano en
  una pasada aparte, que no depende de esta. Las ADR nuevas ya se escriben en castellano.
- **El frontend** (`web/`) también irá en castellano, pero en su propia fase: hay dudas de diseño
  que resolver allí antes.

## Alternativas descartadas

- **Patrones en inglés**, como la ADR 0005 de digitalia. Sergio lo descartó: el criterio es todo en
  castellano salvo la jerga asentada.
- **Traducir más adelante.** El cambio de estructura (ADR-0023) ya reescribe todas las rutas;
  hacerlo después obligaría a un segundo cambio incompatible y a regrabar todos los snapshots otra vez.
- **Un ajuste de idioma por proyecto** (`[proyecto] idioma`). Duplica las plantillas para un
  público que no existe.

## Consecuencias

- Los crates `puerto-nucleo`, `puerto-eventos` y `puerto-correo` se publican como crates nuevos. Las
  versiones de `puerto-core`, `puerto-events` y `puerto-mail` que ya están en crates.io se quedan
  como están: no se borran ni se retiran.
- Las migraciones del runtime (`outbox.sql`, `sent_mails.sql`, `users.sql`, `auth.sql`) cambian sus
  tablas y siguen coincidiendo byte a byte con las de las plantillas.
- No hay proyectos generados que mantener (2026-11-14): ni compatibilidad con el esquema antiguo de
  `puerto.toml` ni guía de migración.
