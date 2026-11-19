# Instrucciones para agentes de IA

Un backend en Rust generado por [Puerto](https://github.com/Guuri11/Puerto): DDD + hexagonal
(puertos y adaptadores), un crate por contexto y capa, API REST con poem-openapi. Este fichero es
la puerta de entrada de cualquier agente de código (`CLAUDE.md` lo importa). El detalle está junto
al código: `contextos/AGENTS.md`, `presentacion/AGENTS.md`.

## Arquitectura

```
nucleo/                 objetos valor compartidos — sin E/S
contextos/<c>/          un contexto por entidad de puerto.toml, cuatro crates cada uno:
  dominio/              → dominio_<c>      modelos, objetos valor, errores, puertos de repositorio
  aplicacion/           → aplicacion_<c>   un fichero por caso de uso: params, trait, impl, tests
  infraestructura/      → infra_<c>        adaptadores: repositorios en memoria o de Postgres
  rest/                 → rest_<c>         HTTP: rutas, DTO, respuestas, mapeo de errores
presentacion/
  cableado/             GENERADO: construye los repositorios y casos de uso de cada contexto
  api/                  el binario: sondas de salud, el bootstrap generado, tu `app.rs`
migraciones/, migrador/ el SQL de toda la base de datos y su único `MIGRADOR` (proyectos `--db`)
```

Las dependencias solo apuntan hacia dentro, y Cargo las hace cumplir: `rest → aplicacion → dominio
→ nucleo`, `infraestructura → dominio`; solo `presentacion/*` depende de `infra_*`. Una referencia
entre entidades es una dependencia entre sus crates de dominio, así que las referencias nunca
forman un ciclo.

| Crate | Contiene | Nunca |
|---|---|---|
| `dominio_<c>` | el modelo, su tipo de id, los objetos valor, los errores, el puerto del repositorio | importa un driver de base de datos, un framework HTTP o un adaptador |
| `aplicacion_<c>` | los casos de uso: el contrato de cada uno y su implementación | sabe de HTTP o de almacenamiento |
| `infra_<c>` | los adaptadores que implementan los puertos del dominio | contiene reglas de negocio |
| `rest_<c>` | rutas, DTO, respuestas, mapeo de errores | expone un modelo de dominio en una respuesta |

**El runtime de Puerto.** Lo que comparten todos los proyectos de Puerto no se copia en ellos: es
un crate, declarado una vez en el `Cargo.toml` raíz (`[workspace.dependencies]`, en la misma minor
que la CLI `puerto`) y que cada miembro hereda con `{ workspace = true }`.

| Crate | Es dueño de | Lo importan |
|---|---|---|
| `puerto-nucleo` | `ErrorRepositorio` (con lo que falla todo puerto de repositorio), `PeticionPagina` / `Pagina<T>` (listas paginadas, límite máximo de 100) | todos los crates: `use puerto_nucleo::{Pagina, PeticionPagina, ErrorRepositorio};` |
| `puerto-web` | el servidor (`App`: tracing, CORS, apagado ordenado), `Config` (leída del entorno y de `.env`), las sondas `/health` y `/health/ready`, `RespuestaError` | `rest_*` y `presentacion/api` |
| `puerto-sqlx` (proyectos `--db`) | el pool de Postgres, las migraciones en el arranque, el `ping` de disponibilidad, `mapear_error_sqlx` (error del driver → `ErrorRepositorio`) | `infra_*` y los binarios |

`puerto-nucleo` solo depende de `thiserror`, así que el dominio puede usarlo; `puerto-web` arrastra
poem y se queda en los crates REST y en el binario; `puerto-sqlx` arrastra sqlx y se queda en los
crates de infraestructura y en los binarios. No copies su código en el proyecto; actualízalos con
la CLI `puerto` (`puerto validar` avisa cuando se desfasan). Tus propios middleware y rutas van en
`presentacion/api/src/app.rs`, que ningún generador toca.

**Las rutas se sirven bajo `/api`**: `#[oai(path = "/productos")]` responde en
`GET /api/productos`. La Swagger UI está en `/` y las sondas en `/health` y `/health/ready`, sin
prefijo.

**El `lib.rs` de cada crate es plano**: un `pub mod` por fichero, sin `mod.rs`. Añadir un caso de
uso es una línea en el `lib.rs` del crate de aplicación más su fichero, y `puerto generar
caso-de-uso` escribe las dos cosas.

## `puerto.toml` es la fuente de verdad

Describe cada entidad, sus campos y casos de uso, y las funcionalidades del proyecto (`db`, `web`,
`eventos`, `auth`). La CLI `puerto` genera a partir de él el código y el cableado; `puerto validar`
comprueba que los dos siguen de acuerdo. `puerto listar` muestra lo que declara, y `puerto
<comando> --help` cuenta con más detalle todo lo que sigue.

| Comando | Qué escribe |
|---|---|
| `puerto generar scaffold <Nombre> [-- campos…]` | todas las capas de una entidad CRUD (crear, obtener, listar, actualizar, borrar) + la migración de su tabla con `db` |
| `puerto generar dominio <Nombre> [-- campos…]` | solo la capa de dominio, y la entidad en `puerto.toml`; después `puerto generar aplicacion <Nombre>`, `puerto generar repositorio <Nombre>`, `puerto generar presentacion <Nombre>`: una capa cada vez, y cada una comprueba la anterior |
| `puerto generar caso-de-uso <Entidad> <accion> [--interno]` | un caso de uso más: su fichero (trait + doble, la impl por rellenar; ver más abajo) y un campo en `Api<Entidad>`; con `--interno`, sin campo ni `actor`: lo llama el worker u otro caso de uso (`[[entidad]] internos`) |
| `puerto generar transicion <Entidad> <verbo> [--evento <tipo>] [--rol <rol>] [--interno]` | un cambio de estado de punta a punta (`cancelar`): el método `<verbo>` de la entidad en `dominio/src/<verbo>.rs` (la regla, por escribir), el caso de uso `<verbo>_<entidad>` con sus tests, `POST /<plural>/{id}/<verbo>` en `rest/src/<verbo>_<entidad>.rs` y, con pantallas, su botón en el detalle; con `--interno`, sin ruta ni botón |
| `puerto generar objeto-valor <Nombre> <tipo>` | un objeto valor compartido (`[[objeto_valor]]`), reutilizable por todas las entidades |
| `puerto generar migracion <nombre>` | una migración SQL vacía (proyectos `db`) |
| `puerto generar bootstrap` | vuelve a cablear `presentacion/cableado` y `generado/bootstrap.rs` a partir de `puerto.toml` |
| `puerto generar evento <Entidad> <nombre>` | un evento de dominio propio (proyectos con `eventos`) |
| `puerto generar consumidor <nombre> --tras <tipo_evento>[,…] [--usa <caso_de_uso>[,…]]` | un consumidor del worker (proyectos con `eventos`); con `--usa`, los casos de uso que llama, sacados de `cableado` |
| `puerto generar correo <nombre> [-- campos…] [--tras <tipo_evento> --para <campo>]` | un correo: su puerto, sus plantillas, su adaptador SMTP; con `--tras`, lo envía el worker |
| `puerto generar snippets` | snippets de editor para los patrones de abajo (Zed, VS Code) |
| `puerto validar` | comprueba nombres, tipos y que el manifiesto y el código están de acuerdo |

Un único sobre varios campos (una línea por jugador y partido) es `--unico jugador_id,partido_id`
en `scaffold` o `dominio` (se puede repetir): un índice único parcial en Postgres, comprobado
también por el repositorio en memoria; queda en `[[entidad]] unicos`. El orden de la lista es
`--orden=-puntos,nombre` (`-`, descendente; después, los más recientes primero): queda en
`[[entidad]] orden`.

En un proyecto con auth, `scaffold` también acepta `--publico <caso_de_uso>[,…]` (sin token),
`--rol <caso_de_uso>=<rol>[,…]` (ver la sección Auth) y `--privado <campo>[,…]`: un campo de texto
que las rutas públicas solo dan a quien tiene sesión (`[[entidad.campos]] privado`; esas rutas
reciben `auth: BearerAuthOpcional` y responden `Dto<Entidad>::from_dominio_publico` sin ella).

<!-- web -->
Con `web/`, `scaffold` también escribe las pantallas de la entidad y `generar bootstrap` también
reescribe los ficheros generados del frontend; `puerto generar web <Nombre>` escribe solo las
pantallas de una entidad.
<!-- /web -->

### Campos

Los campos van después de `--`, un argumento cada uno, sin comillas:

| Argumento | Campo |
|---|---|
| `titulo:String`, `precio:i64`, `activo:bool`, `ratio:f64`, `propietario_id:Uuid` | un primitivo |
| `precio:i64!` | `!` al final: único (un índice único parcial en Postgres; no en `opt`/`vec`) |
| `resumen:opt:String` | `Option<String>`: admite nulo |
| `etiquetas:vec:String` | `Vec<String>` (`String` o `i64`) |
| `publicado_en:DateTime` | `DateTime<Utc>` (también `opt:DateTime`) |
| `meta:map` | `HashMap<String, String>`: una columna `JSONB` |
| `nombre:Nombre:String` | un objeto valor `Nombre` que envuelve un primitivo: se valida una vez, en `Nombre::new` |
| `segundo_nombre:SegundoNombre:opt:String` | un objeto valor opcional |
| `etiqueta:Etiqueta:vec:String` | una lista de objetos valor |
| `estado:Estado:enum:Borrador/Publicado` | un objeto valor enum (`"Borrador".parse::<Estado>()`) |
| `email:Email` | un objeto valor compartido declarado con `generar objeto-valor` (el tipo se infiere) |
| `cliente_id:ref:Cliente` | una referencia: el `IdCliente` de un `Cliente` generado antes (`!`: uno a uno) |
| `revisor_id:ref:opt:Cliente` | una referencia opcional |

Mejor un objeto valor que un primitivo suelto cuando el campo tiene una regla: la regla vive
entonces en un solo sitio, y un valor inválido es un código de error (`producto.nombre_invalido`,
`422`) en todas partes.

Una entidad se refiere a otra por su id, nunca conteniéndola: un caso de uso que necesita el
cliente se lo pide al repositorio de clientes. Con base de datos la columna es una clave foránea, y
una escritura que nombra un cliente que no existe responde `422`
`pedido.validacion.cliente_id_no_encontrado`. Borrar un cliente (un borrado lógico) deja sus pedidos
apuntándole. La lista filtra por cada referencia: `GET /pedidos?clienteId=…` (`ParamsListarPedidos
{ cliente_id }`, `FiltroPedido` en el puerto del repositorio). En la web la referencia es un
`<select>` de clientes y un enlace en el detalle.

### Lo generado y lo tuyo

| | Ficheros | Se cambian |
|---|---|---|
| **Se reescriben en cada ejecución**: no los edites a mano | `presentacion/cableado/` (también su `Cargo.toml`), `presentacion/api/src/generado/bootstrap.rs`, `contextos/<c>/dominio/src/eventos.rs`, `presentacion/worker/src/generado/registro.rs` | editando `puerto.toml` y ejecutando después el comando que los escribe (`generar bootstrap`, `generar evento`, `generar consumidor`) |
| **Los ficheros de la entidad**: son tuyos, pero `scaffold --forzar` los reescribe | los cuatro crates de un contexto: los casos de uso CRUD, el modelo, los errores, el repositorio, las rutas, los DTO, los mapeos, la madre, y sus `Cargo.toml` y `lib.rs` | editándolos; `--forzar` solo mientras sigan siendo lo que escribió el generador |
| **Parcheados**: se añade una línea y se conserva el resto | el `Cargo.toml` raíz (miembros, `[workspace.dependencies]`), `presentacion/api/Cargo.toml` y `presentacion/worker/Cargo.toml` (el crate de un contexto), el `lib.rs` del crate de aplicación, `rutas.rs` (los campos de `Api<Entidad>`), `presentacion/worker/src/consumidores.rs`, `puerto.toml` | editándolos libremente; conserva las líneas que añadió Puerto |
| **Se escriben una vez**: tuyos desde el principio | `presentacion/api/src/app.rs` (tus middleware y rutas), la impl de un `generar caso-de-uso`, los ficheros de una transición (su método, su caso de uso, su ruta, su botón), los consumidores, los correos y sus plantillas, las migraciones, los ficheros de auth | editándolos |

<!-- web -->
Con `web/`: `web/src/generado/*`, `web/src/api/esquema.d.ts` y `docs/api/openapi.json` también se
reescriben (`generar bootstrap`, `make web/sync`); las pantallas de una entidad son ficheros suyos
como los demás; `web/src/rutas.tsx` (tus propias pantallas) es tuyo. El detalle, en
`web/AGENTS.md`.
<!-- /web -->

### Cambiar los campos de una entidad

- **El código sigue siendo lo que escribió el generador:** cambia el bloque `[[entidad]]` de
  `puerto.toml` (o pasa la nueva lista de campos) y ejecuta `puerto generar scaffold <Nombre>
  --forzar`. Los casos de uso añadidos con `generar caso-de-uso` se conservan; cualquier otro cambio
  a mano en los ficheros de la entidad se pierde.
- **En un proyecto `--db` la migración de la tabla nunca se reescribe** (puede estar ya aplicada).
  Si aún no se ha aplicado en ningún sitio: edita esa migración `crear_tabla_<entidad>`. Si ya se
  ha aplicado: añade `puerto generar migracion anadir_<campo>_a_<plural>` con el `ALTER TABLE`, y
  después `make sqlx/migrate && make sqlx/prepare`.
- **El código ya es tuyo:** añade el campo a mano en todas las capas (el modelo y las Props, los
  params, la madre, la fila y sus conversiones, los DTO, cualquier pantalla) con una migración, y
  actualiza `[[entidad.campos]]` para que `puerto.toml` siga describiendo el código.

## Trabajar con Puerto

Primero genera; después escribe las reglas de negocio, empezando por el test. No escribas nunca el
cableado a mano: si el cableado, un `lib.rs` o `Api<Entidad>` necesitan una línea, la escribe un
comando.

1. **Modélalo en el generador.** Una entidad nueva: `puerto generar scaffold <Nombre> -- …` con sus
   campos, objetos valor y enums. Un cambio de estado (cancelar, publicar, archivar):
   `puerto generar transicion <Entidad> <verbo>`, que solo deja por escribir la regla. Cualquier
   otra operación sobre una entidad: `puerto generar caso-de-uso <Entidad> <accion>`.
2. **Enumera los escenarios** antes del código: el camino feliz, la entrada vacía o inválida, los
   límites, los cambios de estado y la regresión que pida un arreglo.
3. **Rojo.** Una entidad generada con `scaffold` ya tiene tests de lo que generó; añade los tuyos al
   lado. `generar caso-de-uso` deja la impl en `todo!()`, unos `Params` de relleno y un test marcado
   `#[ignore = "escribe este test"]` en `contextos/<c>/aplicacion/src/<accion>.rs`: quita el
   `ignore`, escribe el test y míralo fallar.
4. **Verde.** Primero el dominio: los invariantes en el `validar` del modelo (lo llaman `new` y
   `actualizar`) o en un objeto valor (`contextos/<c>/dominio/`); si uno rechaza un valor por
   defecto de la madre, cámbialo en la madre: los tests generados toman de ahí sus valores. Después
   los `Params`, el trait y la impl del caso de uso, en un solo fichero.
5. **El endpoint, API primero.** `generar caso-de-uso` pone el caso de uso en `Api<Entidad>` pero no
   añade ninguna ruta: escribe `dto.rs`, `respuestas.rs`, el brazo de `errores.rs` de cada error
   nuevo (con su test), después el handler en `rutas.rs` y un test de ruta por cada código de estado
   (`contextos/AGENTS.md`).
6. **Comprueba.** Todo lo de *Comprueba antes de terminar*, y después un commit.

## Frontend (`web/`)

Una app React + daisyUI sobre Bun que habla con la API bajo `/api`. Es solo presentación: los
contenedores llaman a la API con su cliente tipado y las vistas pintan lo que reciben; las reglas
viven en el backend. `make web/lint` comprueba que una vista no llama a la API. Sus reglas
(carpetas, ficheros generados, tests) están en `web/AGENTS.md`.

La spec OpenAPI es el contrato entre las dos mitades: `docs/api/openapi.json`, exportada por un test
de `generado/bootstrap.rs`. **Después de un scaffold y después de tocar una ruta o un DTO, ejecuta
`make web/sync`** (vuelve a exportar la spec, regenera los tipos de red del frontend y formatea
`web/`) y haz commit de las dos cosas. Hasta entonces `make test` falla: ese fallo es la
comprobación de desfase. `puerto generar scaffold` también escribe las pantallas de una entidad
CRUD; `puerto generar web <Nombre>` las escribe solas.

Las pantallas base (el acceso, el marco, el panel, el perfil), la portada y el tema tienen otras
variantes: `puerto generar web-bloque` las lista, y `puerto generar web-bloque <hueco>/<variante>
--forzar` cambia el fichero de ese hueco por otra. Cada variante cumple el contrato de su hueco (el
mismo export, las mismas props), así que no toca nada más. Se ven en
https://puerto-rs.vercel.app/bloques.

## Convenciones

- **TDD.** Escribe primero el test que falla, y después el código mínimo. Nombres de test:
  `deberia_<resultado>_cuando_<escenario>`, con `// Preparar` / `// Actuar` / `// Comprobar`.
- **Los errores son códigos legibles por máquina** (`producto.no_encontrado`,
  `producto.validacion.titulo_vacio`), nunca frases. La API los devuelve en
  `RespuestaError.codigo`; los fallos de validación del dominio son `422`.
- **Logs:** `tracing` directamente, sin logger inyectado. El `ejecutar` de un caso de uso lleva
  `#[tracing::instrument(name = "<caso_de_uso>", skip_all, err(level = "warn"))]`: el span da
  nombre a los logs de lo que llama y registra su error una vez. No escribas líneas de inicio ni de
  fin, y nunca registres el contenido de una petición (son datos del usuario: por eso `skip_all`).
- **Nada de `unwrap()` / `expect()`** fuera de los tests.
- **API primero:** cambia `rutas.rs` + `dto.rs` (el contrato) antes que la implementación.

## Comprueba antes de terminar

```bash
puerto validar   # el manifiesto y el código están de acuerdo
make test        # tests unitarios y de integración
make lint        # clippy --all-targets -D warnings: la toolchain está fijada en rust-toolchain.toml
make format      # comprobación de rustfmt; `make format/fix` lo aplica
```

<!-- web -->
- **Con `web/`:** `make web/sync` tras un scaffold o cualquier cambio de ruta o DTO, y commit de lo
  que escribió; hasta entonces `make test` falla por el desfase. `make test` y `make lint` también
  pasan las comprobaciones del frontend.
<!-- /web -->
- **En un proyecto `--db`:** `make docker-compose/up` antes de `make test` (los `#[sqlx::test]`
  necesitan Postgres), y `make sqlx/prepare` tras cambiar una consulta o una migración; haz commit
  de `.sqlx/`.

`rust-toolchain.toml` fija la versión de Rust: clippy gana lints entre versiones, así que una
toolchain fijada es lo que hace que `make lint` dé la misma respuesta en todas las máquinas.

## Cómo se trabaja aquí

**Haz un commit por pieza.** Termina un caso de uso (o un arreglo), deja `make test` y `make lint`
en verde, haz commit y empieza el siguiente. Una regla como «pasos pequeños» que no está atada a
nada comprobable no se cumple; un commit por pieza se ve en `git log`.

**Dónde tiene que leer el diff una persona**: calibra según lo que perdona el sistema cuando el
código está mal:

- **Nivel 1: falla en silencio, despacio o caro.** Un juego de tests en verde no basta; una persona
  lee el diff entero antes del commit: las migraciones SQL (sobre todo las destructivas), todo lo
  que toque dinero u otra aritmética de la que dependa el negocio, las comprobaciones de
  autenticación y autorización, las reglas de unicidad o deduplicación, los datos que cruzan la
  frontera de un inquilino o de un cliente.
- **Nivel 2: falla de forma ruidosa y reversible.** Bastan las comprobaciones de arriba: el mapeo
  DTO ↔ dominio, los adaptadores cubiertos por tests de integración reales, las consultas de solo
  lectura, la documentación.

Un cambio que toca los dos niveles es de Nivel 1.

**Deja las decisiones por escrito.** Cuando una decisión de diseño se apoya en un contexto que no
está en el código, regístralo (una ADR en `docs/adr/` o un cambio en estos ficheros) para que la
siguiente sesión herede el motivo en vez de volver a deducirlo o de deshacerlo.
