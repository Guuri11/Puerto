# Flujos de trabajo de Puerto

Hay dos flujos distintos: uno para desarrollar Puerto y otro para desarrollar los proyectos que
genera Puerto.

---

## 1. Desarrollo de la CLI de Puerto (Spec-Driven)

Puerto es un framework pensado para agentes. Se desarrolla con **Spec-Driven Development**: las
especificaciones viven en `PLAN.md` como fases estructuradas. Los tests se escriben a partir de la
especificación, no leyendo el código. La documentación solo se actualiza cuando cambia el
comportamiento.

### El bucle

```
Especificación (PLAN.md) → Tests ROJO → Implementación VERDE → Refactor → Docs si hace falta
```

Nunca implementes algo sin especificación. Nunca actualices docs por si acaso.

### Paso a paso

#### 1. Leer / escribir la especificación

Busca o escribe la especificación en `PLAN.md`, bajo su fase. Una buena especificación tiene:

- La firma del comando (`puerto generar caso-de-uso <Entidad> <accion>`)
- Los ficheros que crea (rutas exactas)
- Los parches que aplica
- Las reglas de comportamiento (idempotencia, errores, normalización)
- **Los escenarios de test, listados explícitamente**

Si le falta algo de esto, escríbelo antes de seguir. Una palabra nueva en lo generado se decide
antes de usarla: se pregunta a Sergio y se añade a `docs/glosario.md` (ADR-0024).

#### 2. Escribir tests que fallan — ROJO

Los tests de la CLI viven en `crates/cli/src/tests/`, un módulo por área (`scaffolding.rs`,
`manifest_validation.rs`, `compile_matrix.rs`, …), como funciones `#[test]`.
`crates/cli/src/tests.rs` declara los módulos y reimporta los helpers que comparten.

Tests estructurales (existencia de ficheros, aserciones sobre su contenido):

```rust
#[test]
fn use_case_generator_creates_use_case_file() {
    let dir = temp_dir("uc_file");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();
    // ... preparar el proyecto ...
    scaffold::run_use_case("Producto", "publicar_producto", &dir).unwrap();
    assert!(dir.join("contextos/producto/aplicacion/src/publicar_producto.rs").exists());
    cleanup(&dir);
}
```

En los tests, las rutas del proyecto generado también pueden pedirse a `generators/layout.rs`.

Ejecuta `make test` → lo esperado: un error de compilación o una aserción que falla. Es lo
correcto. No sigas hasta que los tests existan y fallen.

#### 3. Implementar — VERDE

Escribe el código mínimo que los hace pasar. Nada especulativo.

Ficheros clave:

- `crates/cli/src/generators/` — escritores de ficheros y generadores por capa (`domain.rs`,
  `application.rs`, `infrastructure.rs`, `presentation.rs`, `scaffold.rs`, `bootstrap.rs`,
  `crates.rs`, `layout.rs`)
- `crates/cli/src/templates/` — las plantillas `.tera` que renderizan los generadores
- `crates/cli/src/patchers/` — `routes_rs.rs`
- `crates/cli/src/puerto_toml.rs` — E/S de puerto.toml + análisis de campos de la CLI
- `crates/cli/src/validation.rs` — predicados de nombres que comparten el parser y `puerto validar`
- `crates/cli/src/main.rs` — despacho de la CLI

#### 4. Refactor

Mejora la estructura sin cambiar el comportamiento. Ejecuta `make test` tras cada cambio.

#### 5. Comprobación final

```bash
make test        # Pasan todos los tests estructurales
make test/full   # Los proyectos generados lintean limpios (clippy -D warnings) y pasan sus tests (necesita bun)
make test/db     # Si tocas la generación --db: lo mismo, contra un Postgres desechable
make lint        # Cero warnings de clippy
make format      # Código formateado
```

#### 6. Commit — una pieza, un commit

Cada arreglo o funcionalidad va en su propio commit, en cuanto sus tests y el lint están en
verde, antes de empezar la siguiente — Conventional Commits, `!` cuando cambia la forma de lo
generado (`release.md`). Una regla como "pasos pequeños" que no está atada a nada comprobable no
se cumple (no se cumplió: una sesión de revisión acumuló veinte arreglos en un solo árbol sin
commitear); un commit por pieza se ve en `git log`.

#### 7. Actualizar docs — solo si hace falta

- **PLAN.md**: marca la fase como hecha (✅), corrige texto desfasado
- **AGENTS.md**: si hay comandos o reglas nuevas
- **`rules/`**: si cambió la arquitectura, los nombres o el flujo
- **`docs/glosario.md`**: cada palabra nueva de lo generado, antes de usarla
- **No** actualices docs para describir lo que hace el código — para eso está el código
- **ADR** (`docs/adr/`): cuando un cambio se apoya en una decisión de diseño que no se ve en el
  código — un compromiso, una convención elegida frente a otra, una medida — se escribe ahí. Así
  la siguiente sesión hereda el motivo en lugar de volver a deducirlo o deshacerlo. Las ADR nuevas
  se escriben en castellano

### Niveles de riesgo — dónde lee el diff una persona

El gate (`make test`, `make test/full`, `make lint`) da confianza en la mayoría de los cambios. No
ve errores de diseño ni roturas silenciosas, así que la lectura humana va donde Puerto **no
perdona** un error:

- **Tier 1 — lee el diff entero antes del commit, aunque el gate esté en verde.** Todo lo que
  cambia lo que llega al proyecto de un usuario o un contrato público:
  `crates/cli/src/templates/`, `crates/cli/template/`, el esquema de `puerto.toml` y su parser, la
  generación de `cableado` y `bootstrap.rs`, el SQL de las migraciones, y lo que reescribe ficheros
  del usuario (`patchers/routes_rs.rs`, los `pub mod` y las dependencias que `generators/crates.rs`
  añade a un `lib.rs` o a un `Cargo.toml`). Un defecto aquí llega a cada proyecto generado después,
  y parte de él (una migración, un fichero sobrescrito) no se arregla arreglando Puerto.
- **Tier 2 — basta con el gate.** Mensajes de la CLI, docs, snippets, código solo de tests,
  refactors cubiertos por completo por el compile matrix.

Todo cambio en lo generado aparece además como diff en `crates/cli/src/tests/snapshots/*.snap` (un
fichero por escenario del compile matrix: cada fichero que los generadores crean, reescriben o
borran). `make test` falla por él hasta que se regraban los snapshots con `make test/snapshots` — y
ese diff del `.snap` es el diff de Tier 1 que hay que leer. Un refactor de los generadores que no
debe cambiar su salida tiene que dejar los snapshots intactos.

Un cambio que toca los dos es Tier 1. Antes de cerrar una pieza de Tier 1 en lógica pura
(`validation.rs`, `migration.rs`, `types.rs`, `naming.rs`, los patchers), ejecuta
`make test/mutation` y mira los supervivientes — y el **número** de mutantes: cero supervivientes de
cero mutantes no prueba nada.

---

## 2. Desarrollo de un proyecto generado (TDD)

Para las entidades DDD de los proyectos que genera Puerto. Los ejemplos usan el contexto
`producto`.

**CRÍTICO:** los tests se escriben PRIMERO. Nunca escribas implementación sin un test que falla.

### El ciclo

```
ROJO      → Escribe un test que falla y describe un requisito de negocio
VERDE     → Escribe el código mínimo que lo hace pasar
REFACTOR  → Mejora la calidad sin cambiar el comportamiento (ejecuta los tests tras cada cambio)
```

### Paso a paso

#### 1. Analizar — primero los requisitos de negocio

- Identifica la regla de negocio que vas a implementar
- Lista TODOS los escenarios antes de escribir código:
  - Camino feliz
  - Entradas vacías o inválidas
  - Casos límite (valores de borde, transiciones de estado)
  - Regresiones (si arreglas un bug)

#### 2. Escribir tests que fallan — ROJO

NO escribas implementación todavía.

```rust
#[tokio::test]
async fn deberia_crear_producto_cuando_el_nombre_es_valido() {
    // Preparar, Actuar, Comprobar
}

#[tokio::test]
async fn deberia_rechazar_producto_cuando_el_nombre_esta_vacio() {
    // Preparar, Actuar, Comprobar
}
```

Ejecuta `make test` (o `make test/dominio`, `make test/aplicacion`) → lo esperado: los tests
fallan. Es lo correcto.

#### 3. Implementar el modelo de dominio — VERDE

En `contextos/producto/dominio/src/modelo.rs`:

- El struct de la entidad + `PropsProducto`
- `new(props)` con validación
- `from_repositorio()` (sin validación)

#### 4. Definir y implementar el caso de uso — VERDE

En `contextos/producto/aplicacion/src/<accion>.rs` (`puerto generar caso-de-uso Producto <accion>`
escribe el fichero y su `pub mod`), todo junto:

- El struct `Params<Accion>`
- El trait `<Accion>` (`#[async_trait]`) y su doble (`dobles`)
- `<Accion>Impl`, que implementa el trait con `Arc<dyn RepositorioProducto>`
- Sus tests

Ejecuta `make test` → todos los tests deberían pasar.

#### 5. Refactor

#### 6. Infraestructura

`contextos/producto/infraestructura/src/repositorio.rs` — empieza en memoria; con `--db`, Postgres
(`fila.rs`, las migraciones en `migraciones/`).

#### 7. REST — API primero

Actualiza `rutas.rs` + `dto.rs` ANTES de escribir el handler. Después, en orden, en
`contextos/producto/rest/src/`:

1. `dto.rs`
2. `respuestas.rs`
3. `errores.rs`
4. `rutas.rs`

#### 8. Cablear

Ejecuta `puerto generar bootstrap` (o `make generar/bootstrap`) desde la raíz del proyecto si
`cableado` o `bootstrap.rs` no están al día. Si no, ya los regeneró `puerto generar scaffold`.

#### 9. Comprobación final

```bash
make test
make lint
make format
```

---

## Rutas clave

### La CLI de Puerto

| Fichero                                  | Para qué                                                    |
| ---------------------------------------- | ----------------------------------------------------------- |
| `crates/cli/src/main.rs`                 | Definición de la CLI + despacho                             |
| `crates/cli/src/tests/`                  | Tests, un módulo por área (estructurales + compile matrix `#[ignore]`) |
| `crates/cli/src/generators/`             | Generadores por capa, bootstrap, crates, guardas de sobrescritura y deriva |
| `crates/cli/src/generators/layout.rs`    | Dónde va cada pieza de un proyecto generado                 |
| `crates/cli/src/templates/`              | Plantillas `.tera` que renderizan los generadores           |
| `crates/cli/src/patchers/`               | `routes_rs.rs`                                              |
| `crates/cli/src/puerto_toml.rs`          | read/write/add_entity de puerto.toml + `parse_field_arg`    |
| `crates/cli/src/validation.rs`           | Predicados de nombres que comparten el parser y `puerto validar` |
| `crates/cli/src/scaffold.rs`             | Fachada de reexports sobre `generators/`                    |
| `crates/cli/template/`                   | La plantilla que usa `puerto nuevo`                         |
| `docs/glosario.md`                       | Los nombres de lo generado                                  |

### Proyectos generados

| Pieza                    | Ruta                                                                  |
| ------------------------ | --------------------------------------------------------------------- |
| Modelo de dominio        | `contextos/<c>/dominio/src/modelo.rs`                                 |
| Errores de dominio       | `contextos/<c>/dominio/src/errores.rs`                                |
| Puerto del repositorio   | `contextos/<c>/dominio/src/repositorio.rs`                            |
| Objetos valor            | `contextos/<c>/dominio/src/valores.rs` (compartidos: `nucleo/src/valores.rs`) |
| Madre                    | `contextos/<c>/dominio/src/pruebas/madres.rs`                         |
| Caso de uso (entero)     | `contextos/<c>/aplicacion/src/<accion>.rs`                            |
| Repositorio (adaptador)  | `contextos/<c>/infraestructura/src/repositorio.rs`                    |
| Fila (Postgres)          | `contextos/<c>/infraestructura/src/fila.rs`                           |
| Rutas REST               | `contextos/<c>/rest/src/rutas.rs`                                     |
| DTO                      | `contextos/<c>/rest/src/dto.rs`                                       |
| Respuestas               | `contextos/<c>/rest/src/respuestas.rs`                                |
| Errores → HTTP           | `contextos/<c>/rest/src/errores.rs`                                   |
| Migraciones              | `migraciones/` (+ el crate `migrador`)                                |
| Composición              | `presentacion/cableado/src/lib.rs` (generado)                         |
| Bootstrap de la API      | `presentacion/api/src/generado/bootstrap.rs` (generado)               |
| Hook de la API           | `presentacion/api/src/app.rs` (del usuario)                           |

---

## Lista de revisión de código

### Cambios en la CLI de Puerto

- [ ] Hay especificación en PLAN.md antes de implementar
- [ ] Tests escritos antes que la implementación
- [ ] Pasan todos los tests: `make test` + `make test/full`
- [ ] Lint limpio: `make lint`
- [ ] Formateado: `make format`
- [ ] Cambio de Tier 1 (plantillas, esquema, bootstrap, migraciones, patchers): diff leído entero
- [ ] Palabras nuevas de lo generado, en `docs/glosario.md` antes de usarlas
- [ ] Un commit para esta pieza — no acumulado con la siguiente
- [ ] Decisión de diseño sin rastro en el código → ADR en `docs/adr/`
- [ ] Docs actualizadas solo donde cambió el comportamiento

### Cambios en un proyecto generado

- [ ] Tests escritos antes que la implementación
- [ ] Ningún `unwrap()`/`expect()` fuera de los tests
- [ ] El dominio no importa nada de infraestructura ni de REST (Cargo lo impide: no añadas la
      dependencia)
- [ ] Los DTO nunca exponen modelos de dominio
- [ ] Los códigos de error son identificadores legibles por máquina (`"producto.no_encontrado"`)
- [ ] El `lib.rs` de cada crate sigue plano (sin `mod.rs`)
- [ ] `cableado/` y `generado/bootstrap.rs` sin ediciones a mano
