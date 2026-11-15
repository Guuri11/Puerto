# 0023 — Un crate por contexto y capa en los proyectos generados

**Estado:** Aceptada (2026-11-14), con las cifras del piloto en canasta (§ Piloto).
Especificación: `PLAN.md` § Fase 19. Los nombres siguen la ADR-0024 y `docs/glosario.md`.
Modifica la ADR-0017 (la forma del proyecto generado, no la de los crates del runtime), la
ADR-0012 (dónde viven los traits de los casos de uso) y la ADR-0022 (las referencias pasan a ser
dependencias entre crates).
**Precedentes:** ADR 0004 de `digitalia-dev` (`ant_backend`) y ADR-0054 de `nexe-document`
(`document-api`), que hicieron este cambio a mano, con scripts y midiendo.

## Contexto

Un proyecto generado tiene tres crates por capa: `business` (dominio y aplicación de todas las
entidades), `infrastructure` y `presentation`. La regla de dependencias solo la impone Cargo
*entre* esas capas. Dentro de `business` nada impide que el dominio de una entidad importe la
aplicación de otra, ni que dos entidades se usen mutuamente. Además, todo cambio recompila
`business` entero, con los `mock!` de todas las entidades.

Los dos proyectos de Sergio que crecieron así acabaron igual:

- **`ant_backend`** (150.000 líneas en `business`). El bucle de TDD tardaba 22 s, 17 de ellos de
  `mockall`. Con un crate por funcionalidad y capa bajó a 1–3 s. Desde cero, un 7 % más lento. Lo
  que más rindió fue sacar la infraestructura de cada funcionalidad a su crate: tocar un
  repositorio pasó de ~8 s a ~2 s.
- **`document-api`** (40.600 líneas). El bucle ya era de 3,1 s, así que el motivo fue otro: que
  Cargo imponga la frontera entre contextos y capas. Quedó en 1,15 s.

Puerto genera proyectos para ese mismo tipo de trabajo. Hoy los genera con la forma que esos dos
proyectos tuvieron que deshacer.

## Decisión

### 1. La unidad es el contexto; en la v1, una entidad es un contexto

Cada `[[entidad]]` de `puerto.toml` es un contexto con sus propios crates, y sus módulos van en la
raíz de cada crate (`dominio_producto::modelo`). Agrupar varias entidades en un mismo contexto,
como `cleaning` en `ant_backend` o `cartera` en `document-api`, queda para una fase posterior: hoy
Puerto no lo necesita, y la forma de los submódulos se decide entonces.

### 2. La estructura

```
Cargo.toml                 members = ["nucleo", "migrador", "contextos/*/*", "presentacion/*"]
nucleo/                    → nucleo            objetos valor compartidos (hoy domain/shared). Sin IO
migraciones/               el SQL de toda la base de datos (hay claves foráneas entre contextos)
migrador/                  → migrador          MIGRADOR + build.rs              (proyectos --db)
contextos/<c>/
  dominio/                 → dominio_<c>       modelo, valores, errores, repositorio, unit_of_work,
                                               eventos; pruebas/ (madres); cada doble, junto a su trait
  aplicacion/              → aplicacion_<c>    un fichero por caso de uso: params, trait, impl,
                                               su doble y sus tests
  infraestructura/         → infra_<c>         en memoria o Postgres: fila, repositorio, unit_of_work
  rest/                    → rest_<c>          rutas, dto, respuestas, errores
presentacion/
  cableado/                → cableado          GENERADO entero, Cargo.toml incluido: repositorios,
                                               unidades de trabajo y casos de uso de cada contexto
  api/                     → api               el binario: main.rs + lib.rs, app.rs (del usuario),
                                               generado/bootstrap.rs
  worker/                  → worker            (proyectos con eventos) generado/registro.rs
web/                       sin cambios en esta ADR
```

- **`members` por glob.** Un contexto nuevo no toca la lista. Los crates internos se declaran en
  `[workspace.dependencies]` y cada `Cargo.toml` los usa con `{ workspace = true }`.
- **Los correos** son el contexto `correo` (dominio e infraestructura, sin aplicación), con sus
  plantillas junto a su adaptador. **El usuario de auth** es el contexto `usuario`.
- **El `lib.rs` de cada crate es plano** y se escribe entero. Desaparece el parcheador del árbol de
  módulos inline de `business/src/lib.rs`. `generar caso-de-uso` solo añade una línea `pub mod`.

### 3. Las dependencias

`aplicacion_<c>` → `dominio_<c>` → `nucleo` y `puerto-nucleo`; `infra_<c>` → `dominio_<c>`;
`rest_<c>` → `aplicacion_<c>` + `dominio_<c>`. Solo `cableado` y los binarios dependen de los
`infra_*`. Ningún crate de dominio depende de un crate de aplicación.

**Las referencias son dependencias.** `cliente_id:ref:Cliente` hace que `dominio_pedido` dependa de
`dominio_cliente` por `IdCliente`, que vive con su dueño. Cargo no admite ciclos, así que
`scaffold` y `validar` rechazan una referencia que cerraría uno, nombrando el ciclo. Eso es el mapa
de contextos: lo impone el compilador y no hace falta escribirlo aparte.

### 4. El caso de uso, entero en aplicación

Params, trait, implementación, su doble (`mock!`) y sus tests, en
`aplicacion_<c>/src/<caso>.rs`. El dominio se queda sin `use_cases/`: la entrada y la salida de un
caso de uso son de la aplicación. El trait se mantiene en todos: las rutas guardan
`Arc<dyn CrearProducto>` y se prueban con dobles (la ADR-0012 sigue en pie). Es el criterio de los
dos precedentes, "solo con trait si hay doble", aplicado a Puerto, donde todos tienen uno.

### 5. Dobles y madres, en cada crate

En el módulo `pruebas` del crate dueño del tipo, detrás de la feature `pruebas`, que activa
`mockall` como dependencia opcional. Sin reexports: se importan por su ruta
(`dominio_producto::pruebas::madres::MadreProducto`). Quien los usa solo en tests los declara en
`[dev-dependencies]` con `features = ["pruebas"]`. Se sigue usando `mockall`: partir en crates ya
aísla su coste.

**Los helpers de siembra de los tests de Postgres** (`construir`, `sembrar`) siguen la misma regla,
y van en cadena por las referencias: los de `stat_line` siembran un `player`, que siembra un
`team`. Van en su propio módulo, `infra_<c>::pruebas::siembra`, público detrás de `pruebas`, y no
dentro del módulo de los `#[sqlx::test]`. Si ese módulo se compila fuera de `cfg(test)`, deja
helpers sin usar, y el `make lint` (`-D warnings`) los rechaza. `infra_<c>` declara
`infra_<objetivo>` como dependencia opcional, activada por su feature
(`pruebas = ["dominio_<c>/pruebas", "dep:infra_<objetivo>", "infra_<objetivo>/pruebas"]`). Lo
encontró el piloto.

### 6. Un solo migrador

`migrador` expone el `MIGRADOR` estático (`sqlx::migrate!("../migraciones")`) y tiene un `build.rs`
con `cargo:rerun-if-changed`. Sin él, `migrate!` no ve una migración nueva si no cambia ningún
`.rs`. Los tests de Postgres usan `#[sqlx::test(migrator = "migrador::MIGRADOR")]`. Hoy cada test
lleva `migrations = "./migrations"`, que copia todas las migraciones dentro de cada test: en
`ant_backend` quitarlo bajó la recompilación de 27 s a 7 s.

### 7. La raíz de composición

- **`cableado`** construye, a partir del pool, lo que no depende de cada binario: los repositorios,
  las unidades de trabajo y los casos de uso de cada contexto. No conoce poem. Lo generan los
  mismos comandos que hoy generan `bootstrap.rs`, y su `Cargo.toml` también: así ningún fichero del
  usuario se parchea para añadir un `infra_*`.
- **`api/generado/bootstrap.rs`** monta los `Api*` sobre `cableado`, el autenticador, el OpenAPI y
  las rutas, y termina en `app::configure`. El `Cargo.toml` de `api` recibe los `rest_*` con el
  mismo `add_workspace_dependency` que hoy añade `puerto-mail`.
- **`worker/generado/registro.rs`** monta el relevo y los consumidores sobre `cableado`, con sus
  adaptadores (SMTP). Lo que pide configuración propia (el proveedor OIDC, el SMTP) se queda en el
  binario que lo usa. Es la lección de `document-api`: un contenedor que lo construye todo obliga a
  cada binario a arrancar con la configuración de todos.
- **`api` tiene `lib.rs`:** `main.rs` solo lo arranca. En `ant_backend` un binario sin `lib.rs`
  tardaba 0,6 s más en compilar, porque Cargo no encadena su compilación con la de sus
  dependencias.

### 8. Los errores en los crates REST

Un crate `rest_<c>` no puede escribir `impl IntoErrorResponse for ErrorProducto`: ni el trait
(`puerto-web`) ni el tipo (`dominio_producto`) son suyos (la orphan rule). Cada `rest_<c>/errores.rs`
exporta una función libre, `a_respuesta_error(err) -> (StatusCode, Json<RespuestaError>)`, con un
test por rama. El contrato de la ADR-0007 no cambia. En el piloto también valía un trait local en
cada crate REST: un trait propio sí se puede implementar para un tipo ajeno. Se queda la función
libre, que no repite la definición del trait en cada crate.

## Piloto (2026-11-14)

canasta (`--db --events db --auth local`; 6 entidades con referencias, un consumidor, un correo;
23.000 líneas de Rust) pasó de 4 crates a 37 con un script determinista. El script mueve los
ficheros sin tocar sus `use`: cada crate reexporta sus dependencias en las rutas antiguas, como en
`ant_backend` (A.1). Las fachadas no cuestan compilación (A.15). Está en la rama
`piloto/crates-por-contexto` de canasta, junto con los scripts y las medidas. Pasan los mismos 472
tests.

Cada medida es la mediana de 3 ejecuciones tras una de calentamiento, después de un `touch`, en el
portátil de desarrollo (16 hilos, 15 GB), con rustc 1.95.0 y `SQLX_OFFLINE=true`:

| Qué toco → qué compilo | Antes (4 crates) | Después (37 crates) |
|---|---|---|
| un caso de uso → los tests de su crate | 0,95 s · 0,38 GB (`business`) | **0,27 s · 0,16 GB** |
| el modelo → los tests de dominio y aplicación | 0,95 s (`business`) | **0,36 s** |
| un repositorio de Postgres → sus tests | 0,85 s · 0,61 GB (`infrastructure`) | **0,42 s · 0,39 GB** |
| un caso de uso → `check -p presentation` | 0,95 s | **0,34 s** |
| un caso de uso → `build -p presentation` | 1,48 s | **0,88 s** |
| desde cero (solo los crates propios): `test --workspace --no-run` | 10,9 s | 10,1 s |
| desde cero (solo los crates propios): `check --workspace --all-targets` | 6,1 s | **2,95 s** |
| ejecutar los tests unitarios (210) | 0,28 s | 0,30 s |
| ejecutar todos los tests con BD (472), con un Postgres nuevo, alternando dos tandas | 12,7–13,1 s | 7,1–7,3 s |

- **Nada empeora, ni siquiera desde cero.** Con 16 hilos, 37 crates pequeños se compilan en paralelo
  mejor que 4 grandes.
- **A este tamaño, el bucle ya era rápido** (~1 s). Lo que se gana de verdad es la frontera, como
  en `document-api`; el tiempo es un extra que crece con el proyecto.
- **El tiempo con BD depende del estado de Postgres más que de la estructura:** la primera medida
  del "antes" dio 7,3 s y no se repitió. Cada test de BD tarda ~0,5 s en serie y ~3 s con 16 en
  paralelo, por la contención al crear las bases de prueba. Con el servidor recién arrancado en
  los dos casos, el "después" sale más rápido en todas las tandas.
- `cargo test -p 'dominio_*' -p 'aplicacion_*'` y `cargo nextest run -p …` aceptan globs, y la
  caché `.sqlx` de la raíz sigue valiendo.
- **Hallazgo:** los helpers de siembra de los tests de Postgres van en cadena entre crates (§ 5).

## Implementación (Fase 19.6, 2026-11-14)

Lo que la implementación fijó y la decisión no decía:

- **El doble va junto a su trait**, en el mismo fichero (`repositorio.rs` → `mocks`, cada caso de
  uso → su `mocks`), detrás de la feature `pruebas`. En `pruebas/` quedan las madres y, en los
  crates de infraestructura de Postgres, la siembra. Es la misma regla para el repositorio y para
  los casos de uso.
- **El caso de uso se escribe con dos plantillas** (contrato e implementación) que el generador une
  bajo un solo bloque de `use`, sin repetir ningún elemento (`generators/merge.rs`): dos `use` del
  mismo nombre son E0252.
- **La madre de `Principal` pasa al runtime** (`puerto_nucleo::auth::dobles::MadrePrincipal`): con
  un crate por contexto no hay un `business` común donde ponerla, y la usan los tests de todos.
- **Un glob de `members` que no casa con nada, Cargo lo lee como una ruta** y no carga el
  workspace. Un proyecto sin contextos (`--sin-demo`) no lista `contextos/*/*` hasta el primero;
  `migrador` solo aparece en los proyectos `--db`.
- **Las rutas necesitan la aplicación**: guardan los traits de los casos de uso, que viven en
  `aplicacion_<c>`. `generar presentacion` se niega hasta que existe; la API ya no puede ir antes
  que la aplicación.
- **El cableado de auth se queda en `bootstrap.rs`**: pide configuración (el proveedor OIDC, la
  duración de las sesiones). `cableado` construye los contextos de `puerto.toml`.
- **`registro.rs` no usa `cableado` todavía**: los consumidores no reciben casos de uso. Lo hará
  cuando un consumidor generado llame a uno.
- **Nombres reservados**: una entidad no puede llamarse `Correo` (el contexto de los correos), ni
  `Usuario` en un proyecto con auth.
- Hasta la 19.7 los identificadores siguen en inglés; lo nuevo de la 19.6 ya nace en castellano
  (`CasosDeUso`, `construir`, `a_respuesta_error`, `MadrePrincipal`).

## Alternativas descartadas

- **Módulos por contexto dentro de `business`.** Barato, pero sigue siendo un crate: ni frontera ni
  compilación. `ant_backend` lo descartó tras evaluarlo.
- **Un crate por contexto con todas sus capas.** Sin frontera entre capas; tocar un repositorio
  recompila el dominio y los dobles; los tests unitarios enlazan sqlx (`ant_backend`, A.12).
- **Dos crates por contexto** (dominio y aplicación juntos). Defendible, pero renuncia a la
  frontera dominio → aplicación y a ~1 s por ciclo, y la regla uniforme de cuatro crates es más
  sencilla de generar.
- **Los ids en `nucleo`**, como `document-api`. Las referencias no crearían dependencias, pero el
  mapa de contextos dejaría de estar vigilado y cada entidad nueva recompilaría todo.
- **Imponer la frontera con un linter** (cargo-pup, `disallowed_types`, un test con grep). O exige
  nightly, o es textual, o se configura por crate y no por módulo.

## Consecuencias

- Cambia la forma del proyecto generado y el esquema de `puerto.toml`: es un cambio incompatible
  (0.21). No hay proyectos generados que mantener, así que no hay guía de migración.
- En un proyecto pequeño, más crates puede salir más lento desde cero: hay más binarios de test
  que enlazar. Por eso el piloto se mide antes de tocar los generadores. Si sale peor, la decisión
  se apoya en la frontera, como en `document-api`, y se vuelve a discutir con las cifras.
- En Puerto, las rutas de lo generado pasan a un solo módulo de layout. Hoy hay 102 literales
  repartidos por los generadores. Todos los snapshots cambian.
- Las cachés incrementales crecen más rápido con decenas de crates (`ant_backend` llenó 76 GB en
  cuatro días): el `Makefile` generado gana un objetivo para limpiarlas.
