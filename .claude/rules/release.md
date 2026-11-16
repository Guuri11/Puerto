# Flujo de release

## Ramas

**Trunk-based development.** `main` siempre se puede publicar.

- El trabajo va en ramas cortas: `feat/scaffold-use-case`, `fix/bootstrap-patch`
- Squash merge a `main` — historial limpio y bisecable
- Sin ramas `develop`, `release/*` ni `hotfix/*`
- Las etiquetas de release (`v0.2.0`) son las únicas marcas de larga vida

---

## Convención de commits

Todos los commits siguen **Conventional Commits** (el tipo en inglés; la descripción puede ir en
castellano, como en el historial reciente):

```
feat: add puerto generar migracion command
fix: bootstrap overwrites entity block on second run
docs: update puerto-toml derivation table
chore: upgrade cargo-generate to 0.23
refactor: extract patch_lib_block into own module
test: add idempotency test for use-case generator
feat!: un crate por contexto y capa en los proyectos generados — Fase 19.6
```

**Tipos:** `feat` · `fix` · `docs` · `chore` · `refactor` · `test` · `perf`

`CHANGELOG.md` sale de estos commits con **git-cliff**. No se escribe a mano.

---

## Versionado

Puerto sigue **SemVer**. La API pública no son solo los flags de la CLI — incluye:

- La estructura del proyecto generado (rutas de ficheros, crates, módulos)
- El esquema de `puerto.toml`
- El contrato de generación de `cableado` y `bootstrap.rs`
- Los nombres de lo generado (`docs/glosario.md`) y la API de los crates del runtime

| Subida            | Cuándo                                                                                              |
| ----------------- | --------------------------------------------------------------------------------------------------- |
| `patch` 0.1.**x** | Arreglo en la generación, errata en una plantilla, arreglo del bootstrap. Sin cambiar la forma de lo generado. |
| `minor` 0.**x**.0 | Comando nuevo, flag nuevo, ficheros generados nuevos, campo nuevo y aditivo en puerto.toml. Compatible. |
| `major` **x**.0.0 | Cambio de estructura de lo generado, renombrado de módulos, cambio incompatible del esquema de puerto.toml, comando eliminado. |

**Mientras estemos en `0.x.y`**: una minor puede traer cambios incompatibles — es la convención de
los crates anteriores a 1.0. Todo cambio incompatible se documenta en `CHANGELOG.md` bajo
`### Breaking`.

---

## Changelog

Lo genera **git-cliff** a partir de los conventional commits. Nunca se edita a mano.

```bash
git cliff --unreleased --prepend CHANGELOG.md
```

Estructura:

```markdown
## [Unreleased]

## [0.2.0] - 2026-05-01

### Added

### Fixed

### Changed

### Breaking

### Migration Guide ← obligatoria en cualquier cambio incompatible
```

La sección `## Migration Guide` es **obligatoria** en cualquier release que cambie:

- La estructura de lo generado
- El esquema de puerto.toml
- Los nombres de módulos visibles en el código generado

**Excepción (decidida el 2026-11-14):** la 0.21 (Fase 19: un crate por contexto y capa, y todo lo
generado en castellano) lleva `### Breaking` sin guía de migración: no hay proyectos generados que
migrar (ADR-0023, ADR-0024).

---

## Proceso de release

Empujar una etiqueta `v*` publica: `release.yml` comprueba que la etiqueta coincide con la versión
de **cada** crate publicado (un solo `[workspace.package] version` en el `Cargo.toml` raíz —
ADR-0017), ejecuta `make test/full` y luego `cargo publish --locked`, primero los crates del
runtime, en orden de dependencias, y después `puerto`:

```
puerto-nucleo → puerto-macros → puerto-web → puerto-sqlx → puerto-eventos → puerto-kafka → puerto-correo → puerto-auth → puerto
```

Un crate cuya versión ya está en crates.io se salta, así que publicar antes a mano no pone la
ejecución en rojo. Un proyecto generado por `puerto` 0.x pide a crates.io los crates del runtime en
0.x: la CLI nunca debe estar en crates.io sin ellos. `puerto-nucleo`, `puerto-eventos` y
`puerto-correo` son crates nuevos (ADR-0024); las versiones de `puerto-core`, `puerto-events` y
`puerto-mail` que ya están en crates.io se quedan como están.

```bash
# 1. Subir la versión — una vez para todos los crates
#    Edita el Cargo.toml raíz — [workspace.package] version = "0.x.y", y la misma versión en cada
#    entrada de su [workspace.dependencies] (puerto-nucleo, puerto-macros, puerto-web: contra lo
#    que se publican los crates del runtime que dependen de ellos)

# 2. Actualizar el changelog (a mano a partir de los commits desde la última release, mismo formato)
#    `git cliff --unreleased --prepend CHANGELOG.md` lo esboza si git-cliff está instalado

# 3. Verificar
make test
make test/full
make test/db
make test/kafka
make test/mail
make test/auth
make lint
make format
cargo package --list -p puerto       # confirma que van los ficheros de la plantilla
cargo publish --dry-run -p puerto-nucleo
cargo publish --dry-run -p puerto-macros
cargo publish --dry-run -p puerto-web
cargo publish --dry-run -p puerto-sqlx
cargo publish --dry-run -p puerto-eventos
cargo publish --dry-run -p puerto-kafka
cargo publish --dry-run -p puerto-correo
cargo publish --dry-run -p puerto-auth
cargo publish --dry-run -p puerto

# 4. Commit, push, etiqueta — la etiqueta publica
git add -A && git commit -m "chore: release v0.x.y"
git push
git tag -a v0.x.y -m "v0.x.y" && git push origin v0.x.y

# Si el workflow no puede publicar — mismo orden:
#   cargo publish -p puerto-nucleo --locked && cargo publish -p puerto-macros --locked \
#     && cargo publish -p puerto-web --locked \
#     && cargo publish -p puerto-sqlx --locked && cargo publish -p puerto-eventos --locked \
#     && cargo publish -p puerto-kafka --locked && cargo publish -p puerto-correo --locked \
#     && cargo publish -p puerto-auth --locked && cargo publish -p puerto --locked
```

La web (`landing/`) la despliega Vercel en cada push a `main` — actualiza sus docs en el push del
commit de release cuando la release cambia lo que describe.

---

## Lista previa a la release

Toda release — automática o manual — tiene que pasar:

- [ ] `make test` — pasan los tests estructurales
- [ ] `make test/full` — los proyectos generados lintean limpios y pasan sus tests ← **el gate más
      importante** (necesita `bun` en el `PATH`: los escenarios web instalan, lintean y prueban el
      frontend generado)
- [ ] `make test/db` — el escenario `--db` contra Postgres
- [ ] `make test/kafka` — el transporte Kafka contra Redpanda
- [ ] `make test/mail` — el adaptador SMTP contra Mailpit
- [ ] `make test/auth` — el adaptador OIDC y un proyecto OIDC generado contra Keycloak + Postgres
- [ ] `make lint` — cero warnings de clippy
- [ ] `make format` — sin diff de formato
- [ ] `cargo package --list` — los ficheros de la plantilla van en el paquete
- [ ] `cargo publish --dry-run` — sin errores de publicación
- [ ] `CHANGELOG.md` tiene una entrada para esta versión
- [ ] `make site/data` — la terminal, la referencia de la CLI y la captura del compilador de la web
      siguen a la nueva versión (los tests de `landing/` fallan hasta que lo hacen); commitea lo que
      cambie
- [ ] El `Cargo.toml` de cada crate publicado tiene `description`, `license`, `repository`,
      `rust-version`
- [ ] Los cambios incompatibles tienen `## Migration Guide` en el changelog (salvo la excepción de
      la 0.21)

---

## CI/CD

Dos workflows de GitHub Actions:

**`ci.yml`** — en cada push y PR:

```
make test
make lint
cargo fmt --check
```

**`ci.yml`** también ejecuta `make test/full` como job aparte.

**`release.yml`** — al empujar una etiqueta `v*`:

```
etiqueta == versión de los crates; se salta lo que ya está en crates.io
make test/full
cargo publish --locked, crate a crate, en el orden de arriba
```

La distribución de binarios con **cargo-dist** aún no está montada — trabajo futuro.

---

## Política de cambios incompatibles

1. **Deprecación de un ciclo**: se depreca en una `minor` y se quita en la siguiente `minor`. Durante
   una release se imprime un aviso en ejecución:

   ```rust
   eprintln!("aviso: el campo X de puerto.toml está deprecado — usa Y (ver CHANGELOG)");
   ```

2. **Guía de migración**: toda release incompatible lleva una sección `## Migration Guide` en
   `CHANGELOG.md` que explica exactamente qué cambió y cómo migrar un proyecto existente.

3. **Contrato de puerto.toml**: el esquema es el contrato público más frágil. Si cambia de forma
   incompatible, se añade `schema_version` al fichero para que Puerto pueda detectar y migrar los
   proyectos antiguos.

4. **Código generado**: nunca se cambia en silencio el contenido de lo generado en una release
   patch. Un patch solo arregla bugs — nunca reorganiza lo que `puerto generar` escribe en disco.

La Fase 19 (0.21) se salta las reglas 1–3: no hay proyectos generados que mantener (2026-11-14).

---

## Particularidades de Rust / crates.io

- **MSRV**: declara `rust-version` en `Cargo.toml`. Pruébalo en CI con
  `rustup run <version> cargo check`.
- **La plantilla en el paquete**: Puerto embebe su plantilla con `include_dir`. Comprueba con
  `cargo package --list` que van los bytes embebidos.
- **Yank**: `cargo yank --version X.Y.Z` para releases rotas. Nunca hagas yank de una versión de la
  que dependen proyectos que funcionan.
- **`publish = false`**: en cualquier miembro del workspace que no deba llegar a crates.io.
- **Cargo.lock**: se commitea — Puerto es una herramienta binaria, el lockfile va en el repositorio.
