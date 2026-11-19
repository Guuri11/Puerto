# puerto.toml — Esquema y derivación de identificadores

## Esquema

```toml
[proyecto]
nombre = "mi-app"        # nombre del proyecto (con guiones; es el nombre del binario)
web = true               # hay frontend web/ — lo pone `puerto nuevo`, ausente con --sin-web
db = true                # SQLx/Postgres — lo pone `puerto nuevo --db`
eventos = "db"           # eventos de dominio: "db" | "kafka"; ausente = ninguno. Exige db = true
auth = "oidc"            # auth (ADR-0021): "oidc" | "local"; ausente = ninguna. "local" exige db = true
portada = true           # un SaaS con portada pública en / (ADR-0028): `puerto nuevo --portada`. Exige web
ruta_app = "/panel"      # dónde va la app bajo la portada (`--ruta-app`); solo con portada

[[entidad]]
nombre = "Producto"      # nombre de la entidad en PascalCase
casos_de_uso = ["crear_producto", "listar_productos"]   # acciones en snake_case
db = true                # opcional — omitido: en memoria; true: SQLx/Postgres
eventos = ["producto_publicado"]   # opcional, proyectos con eventos: eventos propios además de los
                                   # CRUD creado/actualizado/borrado — snake_case, únicos, no CRUD
transiciones = ["cancelar"]      # opcional (27.12, `generar transicion`): verbos; cada uno es el método
                                 # dominio/src/<verbo>.rs, el caso de uso <verbo>_<entidad> (también en
                                 # casos_de_uso) y la ruta rest/src/<verbo>_<entidad>.rs con su Api<Caso>,
                                 # que bootstrap.rs monta junto a Api<Entidad> (sin campo en ella)
internos = ["agotar_producto"]   # opcional (27.13, `--interno`): casos de uso sin ruta — sus Params sin
                                 # actor, sin campo en Api<Entidad>, una transición sin Api<Caso> ni
                                 # botón; cableado los construye. Ni CRUD, ni publicos, ni roles
unicos = [["jugador_id", "partido_id"]]   # opcional: un único sobre varios campos (`--unico a,b`) →
                                 # CREATE UNIQUE INDEX <tabla>_<a>_<b>_key … WHERE borrado = false; el
                                 # repositorio en memoria también lo comprueba. Dos o más campos propios,
                                 # sin Option/Vec/HashMap. Un re-scaffold sin --unico conserva el del bloque
orden = ["-puntos", "nombre"]    # opcional: el orden de la lista (`--orden=-puntos,nombre`; `-`, descendente),
                                 # después `creado_en DESC, id`. Campos propios sin Option/Vec/HashMap.
                                 # En memoria, el mismo orden (los textos, por bytes)
publicos = ["listar_productos"]    # opcional, proyectos con auth: rutas sin BearerAuth
roles = { crear_producto = ["admin"] }   # opcional, proyectos con auth: #[requiere_rol] por ruta.
                                 # Las claves son casos de uso de la entidad, nunca también
                                 # publicos; los roles casan con [a-z][a-z0-9_:-]*; dos roles en un
                                 # caso de uso: se exigen los dos. Es lo que escribió scaffold:
                                 # rutas.rs es del usuario desde entonces (`puerto validar` lo
                                 # comprueba de forma textual)

[[consumidor]]                   # proyectos con eventos: un consumidor del worker (ADR-0018 § 5)
nombre = "avisar_producto_nuevo" # snake_case, único → presentacion/worker/src/consumidores/avisar_producto_nuevo.rs
eventos = ["producto_creado", "producto_publicado"]   # cada uno declarado por alguna entidad (CRUD o propio)

[[correo]]                       # un correo (ADR-0020) — solo el cableado, su contenido está en sus ficheros
nombre = "bienvenida"            # snake_case, único → contextos/correo/{dominio,infraestructura}/src/bienvenida.rs
tras = ["cliente_creado"]        # opcional, proyectos con eventos: el worker lo envía tras estos eventos

[[entidad.campos]]
nombre = "nombre"        # nombre del campo en snake_case
tipo = "String"          # tipo Rust del registro de tipos

[[entidad.campos]]
nombre = "precio"
tipo = "i64"

[[entidad.campos]]
nombre = "sku"
tipo = "String"
unico = true             # índice único en la base de datos
# privado = true         # proyectos con auth, campos de texto (String, Option<String>, su objeto valor
                         # no enum): `Option` en Dto<Entidad>; una ruta pública recibe
                         # `auth: BearerAuthOpcional` y sin sesión responde from_dominio_publico (el
                         # campo, ausente). En la CLI, --privado email,telefono

[[entidad.campos]]
nombre = "descripcion"
tipo = "Option<String>"  # campo nullable
# etiqueta = "Resumen"   # opcional (27.5): lo que dicen las pantallas; sin ella, el nombre con sus
                         # tildes (`naming::etiqueta`: anio → Año, codigo_postal → Código postal).
                         # Sin comillas, `\` ni saltos de línea
# textos = { AlaVenta = "A la venta" }   # opcional, objetos valor enum (27.9): lo que se lee de cada
                         # variante; sin él, su nombre en palabras (EnRevision → En revisión)

[[entidad.campos]]
nombre = "etiquetas"
tipo = "Vec<String>"     # campo array

[[entidad.campos]]
nombre = "cliente_id"
tipo = "Uuid"            # "Option<Uuid>" para una referencia opcional
referencia = "Cliente"   # ADR-0022: otra entidad CRUD declarada, nunca esta (ni Usuario/Me/Auth en
                         # proyectos con auth), sin objeto_valor; en la CLI cliente_id:ref:Cliente,
                         # revisor_id:ref:opt:Cliente. scaffold la rechaza hasta que el objetivo tiene
                         # código, y si cierra un ciclo entre contextos
```

Los objetos valor (`objeto_valor`, `clase_objeto_valor`, `variantes`, `[[objeto_valor]]`), en
`value_objects.md`.

- Un bloque `[[entidad]]` por entidad DDD; en la v1, cada una es un contexto (`contextos/<snake>/`)
- `nombre` es el PascalCase canónico — todos los demás identificadores salen de él
- Las entradas de `casos_de_uso` son acciones en snake_case — coinciden con el nombre del fichero y
  del módulo
- `db = true` → `RepositorioProductoPostgres` + `fila.rs`; ausente/false →
  `RepositorioProductoEnMemoria`
- `[[entidad.campos]]` es una lista opcional de campos con tipo. Si falta o está vacía, la entidad
  lleva por defecto `nombre: String`
- El `nombre` de un campo es snake_case válido (minúsculas, dígitos y guiones bajos; no empieza por
  dígito)
- El `tipo` de un campo tiene que estar en el registro de tipos — `puerto validar` lo comprueba
- `unico = true` genera un índice único **parcial** (`… WHERE borrado = false`, solo SQL): las
  filas se borran de forma lógica, así que una fila borrada no retiene su valor. No se combina con
  `Option<T>` (`puerto validar` avisa)
- En proyectos `--db` se rechazan los nombres de campo que son palabras reservadas de PostgreSQL
  (`desc`, `order`, `user`, `select`, …): son nombres de columna sin comillas. También los nombres
  de campo duplicados
- `HashMap<String, String>` es una columna `JSONB`; el campo de `Fila<Entidad>` es
  `sqlx::types::Json<HashMap<String, String>>` y el `SELECT` del repositorio sobrescribe su tipo
- Lo gestiona la CLI de Puerto — no añadas entidades a mano salvo que luego ejecutes
  `puerto generar bootstrap`

### Sintaxis de los campos en la CLI

```bash
puerto generar scaffold Producto -- nombre:String precio:i64! sku:String
```

- Los campos van como argumentos tras `--`, en formato `nombre:Tipo`
- Un `!` tras el tipo marca el campo como `unico = true` (p. ej. `sku:String!`)
- Los analiza `parse_field_arg()` en `puerto_toml.rs`
- Se validan contra el registro de tipos antes de que scaffold siga

---

## Registro de tipos

| Tipo Rust | Tipo SQL | OpenAPI | Por defecto (tests) |
|-----------|----------|---------|---------------------|
| `String` | `TEXT` | `string` | `"example"` |
| `i64` | `BIGINT` | `integer(int64)` | `42` |
| `bool` | `BOOLEAN` | `boolean` | `true` |
| `f64` | `DOUBLE` | `number(double)` | `1.5` |
| `Uuid` | `UUID` | `string(uuid)` | `Uuid::new_v4()` |
| `DateTime<Utc>` | `TIMESTAMPTZ` | `string(date-time)` | `Utc::now()` |
| `Option<String>` | `TEXT` (nullable) | `string?` | `None` |
| `Option<i64>` | `BIGINT` (nullable) | `integer(int64)?` | `None` |
| `Option<bool>` | `BOOLEAN` (nullable) | `boolean?` | `None` |
| `Option<f64>` | `DOUBLE` (nullable) | `number(double)?` | `None` |
| `Option<Uuid>` | `UUID` (nullable) | `string(uuid)?` | `None` |
| `Option<DateTime<Utc>>` | `TIMESTAMPTZ` (nullable) | `string(date-time)?` | `None` |
| `Vec<String>` | `TEXT[]` | `array[string]` | `vec![]` |
| `Vec<i64>` | `BIGINT[]` | `array[integer]` | `vec![]` |
| `HashMap<String, String>` | `JSONB` | `object` | `HashMap::new()` |

### Campos de sistema (siempre presentes)

Toda entidad los lleva — **no** se añaden a `campos`:

```rust
pub id: IdProducto,        // el newtype del id de la entidad sobre un Uuid — ver architecture.md § Ids tipados
pub creado_en: DateTime<Utc>,
pub actualizado_en: DateTime<Utc>,
pub borrado: bool,
pub borrado_en: Option<DateTime<Utc>>,
```

En el JSON, `creadoEn`, `actualizadoEn`…

### Resolución de tipos

`resolve_type(field_type: &str) -> Result<&TypeMapping, Error>` en `generators/types.rs`:
- Busca el tipo en el registro
- Devuelve un `TypeMapping` con `rust_type`, `sql_type`, `sql_nullable`, `openapi_type`,
  `openapi_format`, `default_expr`, `needs_import`
- Un tipo desconocido da un error que lista todos los tipos válidos

`validate_fields(fields: &[Field]) -> Result<(), Vec<String>>` valida todos los campos de una vez.

`literal_import(field) -> Option<&'static str>` devuelve el import que necesita un *literal* del
campo (`chrono::Utc` para `Utc::now()`), a diferencia de `TypeMapping::needs_import`, que es lo que
necesita nombrar el *tipo* del campo.

---

## Tabla de derivación

Con `nombre = "LineaPedido"` y `casos_de_uso = ["crear_linea_pedido", "listar_lineas_pedido"]`:

| Identificador     | Valor                               | Dónde se usa                               |
| ----------------- | ----------------------------------- | ------------------------------------------ |
| `pascal`          | `LineaPedido`                       | Nombres de structs                         |
| `snake`           | `linea_pedido`                      | Contexto, ficheros, módulos, variables     |
| `plural`          | `lineas_pedido`                     | Tabla SQL, rutas REST (`/lineas_pedido`)   |
| `plural_pascal`   | `LineasPedido`                      | Comentarios, tipos `ListarLineasPedido…`   |
| contexto          | `contextos/linea_pedido/`           | Los cuatro crates del contexto             |
| crates            | `dominio_linea_pedido`, `aplicacion_linea_pedido`, `infra_linea_pedido`, `rest_linea_pedido` | `[workspace.dependencies]`, `use` |
| `uc`              | `crear_linea_pedido`                | Campo del caso de uso, nombre del fichero  |
| `uc_pascal`       | `CrearLineaPedido`                  | El trait del caso de uso                   |
| impl              | `CrearLineaPedidoImpl`              | Implementación (aplicación)                |
| params            | `ParamsCrearLineaPedido`            | Struct de entrada                          |
| doble             | `MockCrearLineaPedido`              | `aplicacion_<c>::<uc>::dobles`             |
| id                | `IdLineaPedido`                     | El id tipado                               |
| props / error     | `PropsLineaPedido`, `ErrorLineaPedido` | Modelo, errores                         |
| repositorio       | `RepositorioLineaPedido`            | Puerto del dominio                         |
| adaptadores       | `RepositorioLineaPedidoEnMemoria`, `RepositorioLineaPedidoPostgres` | Infraestructura |
| fila              | `FilaLineaPedido`                   | `infra_<c>/src/fila.rs`                    |
| madre             | `MadreLineaPedido`                  | `dominio_<c>::pruebas::madres`             |
| api / dto         | `ApiLineaPedido`, `DtoLineaPedido`, `DtoPaginaLineaPedido` | REST              |
| petición / respuesta | `PeticionCrearLineaPedido`, `RespuestaCrearLineaPedido` | REST              |
| eventos           | `EventoLineaPedido`, `SnapshotLineaPedido`, `linea_pedido_creado`, `LINEA_PEDIDO_CREADO` | Proyectos con eventos |

### Reglas de derivación

```
pascal    = PascalCase(nombre)                     // "LineaPedido"
snake     = pascal_to_snake(pascal)                // "linea_pedido"
plural    = pluralize(snake)                       // "lineas_pedido"
uc        = casos_de_uso[i]                        // "crear_linea_pedido"
uc_pascal = PascalCase(uc)                         // "CrearLineaPedido"
```

`PascalCase` parte por `_` y `-`, pone en mayúscula cada palabra y las une:

- `"linea_pedido"` → `"LineaPedido"`
- `"crear-producto"` → `"CrearProducto"`

`pluralize` (`generators/naming.rs`) pluraliza en castellano y flexiona la **primera** palabra, el
núcleo de un nombre castellano (`linea_pedido` → `lineas_pedido`): vocal → `s` (`productos`), `z` →
`ces` (`lapices`), cualquier otra consonante, `y` incluida → `es` (`ciudades`, `leyes`, `paises`);
una tabla de invariables (`analisis`, `crisis`, `lunes`…). `naming::crud_use_case` da nombre a los
casos de uso del scaffold — `listar` con el plural (`listar_lineas_pedido`), el resto con la entidad
(`crear_`, `obtener_`, `actualizar_`, `borrar_linea_pedido`). `naming::entity_ctx` pone `pascal`,
`snake`, `plural` y `plural_pascal` en el contexto de cada plantilla.

`pascal_to_snake` inserta `_` antes de cada mayúscula (salvo la primera) y lo pasa todo a minúsculas:

- `"LineaPedido"` → `"linea_pedido"`
- `"Producto"` → `"producto"`

---

## Rutas que salen de una entidad

Con `nombre = "Producto"` y los casos de uso CRUD (las rutas se piden a `generators/layout.rs`):

```
contextos/producto/dominio/Cargo.toml
contextos/producto/dominio/src/{lib,modelo,errores,repositorio}.rs   (+ valores.rs, eventos.rs, unit_of_work.rs)
contextos/producto/dominio/src/pruebas/madres.rs
contextos/producto/aplicacion/src/{crear_producto,obtener_producto,listar_productos,actualizar_producto,borrar_producto}.rs
contextos/producto/infraestructura/src/repositorio.rs                (+ fila.rs, unit_of_work.rs, pruebas/siembra.rs con --db)
contextos/producto/rest/src/{rutas,dto,respuestas,errores}.rs
migraciones/<versión>_<nombre>.sql                                   (--db)
```

Cada crate tiene su `Cargo.toml` y su `lib.rs` plano; no hay `mod.rs`.

---

## Lógica de generación de `cableado` y `bootstrap.rs`

`presentacion/cableado/` y `presentacion/api/src/generado/bootstrap.rs` se generan desde la lista
entera de entidades:

1. `cableado/Cargo.toml` depende de `dominio_<c>`, `aplicacion_<c>` e `infra_<c>` de cada contexto
2. `cableado/src/lib.rs`, un módulo por contexto: `pub struct CasosDeUso { pub <uc>: Arc<dyn
   <UcPascal>>, … }` y `pub fn construir(…) -> CasosDeUso`, que construye el repositorio una vez
   (`Arc::clone` para cada caso de uso):
   - `db = false` → `RepositorioProductoEnMemoria::new()`, `construir()` sin argumentos
   - `db = true` → `RepositorioProductoPostgres::new(pool.clone())`, `construir(&pool)`
   - proyectos con eventos → además, `UnitOfWorkFactoryProductoPostgres`
3. `bootstrap.rs` importa `rest_<c>::rutas::Api<Entidad>` y monta cada una con
   `cableado::<c>::construir(…)`. `construir_app(config)` es siempre `async` y devuelve
   `Result<impl Endpoint, String>`; con `--db` lee `DATABASE_URL`, crea el pool y ejecuta las
   migraciones de `migrador::MIGRADOR`
4. El argumento de `servicio_openapi(...)` — el único sitio donde se llama a `OpenApiService::new`:
   - **Una entidad**: `saludo_api`
   - **Varias**: `(saludo_api, producto_api, ...)`
5. **`[proyecto] web = true`** → un test `#[cfg(test)]` `openapi_spec_matches_docs_api_openapi_json`
   construye cada `Api<Entidad>` con los `Mock<Caso>` (todo caso de uso trae uno; `api` declara los
   `aplicacion_*` con `features = ["pruebas"]` en `[dev-dependencies]`) y compara
   `servicio_openapi(...).spec()` con `docs/api/openapi.json`: si falta → se escribe;
   `UPDATE_OPENAPI=1` → se reescribe; distinto → falla (`make web/sync`). Sin servidor ni base de
   datos.

---

## Comandos de la CLI que tocan puerto.toml

| Comando                                         | Efecto                                                                            |
| ----------------------------------------------- | --------------------------------------------------------------------------------- |
| `puerto nuevo [--nombre] [--db\|--sin-db] [--eventos db\|kafka] [--auth oidc\|local] [--sin-demo] [--sin-web] [--portada [--ruta-app <ruta>]\|--sin-portada] [--destino]` | Crea puerto.toml desde la plantilla con la entidad Saludo; `--eventos` (necesita `--db`) escribe `[proyecto] eventos` y la migración del outbox; `--auth` escribe `[proyecto] auth` y lista el caso de uso del Saludo en `publicos` (`local` necesita `--db`); `--portada` (necesita web) escribe `[proyecto] portada` y `ruta_app` (`/panel` por defecto) |
| `puerto generar scaffold <Nombre>`              | Inserta o actualiza el bloque `[[entidad]]`, regenera `cableado` y `bootstrap.rs`. En proyectos con auth, `--publico <uc>[,…]` y `--rol <uc>=<rol>[,…]` fijan `publicos` / `roles` (se comprueban antes de escribir; un re-scaffold sin ellos conserva los del bloque) |
| `puerto generar scaffold <Nombre> -- nombre:Tipo` | Igual, con `[[entidad.campos]]` sacados de los argumentos finales               |
| `puerto generar dominio <Nombre> -- nombre:Tipo` | Registra la entidad **con sus campos**, para que los comandos por capa posteriores los vean |
| `puerto generar bootstrap [--permitir-faltantes]` | Regenera `cableado`, `bootstrap.rs` (y `web/…/core/generated/*`); se niega si una entidad declarada no tiene código en disco |
| `puerto generar caso-de-uso <Entidad> <accion> [--interno]` | Añade la acción a `casos_de_uso`, escribe su fichero en `aplicacion_<c>` con su `pub mod`, añade el campo a `Api<Entidad>`, regenera el cableado. Rechaza una acción que ya existe. `--interno` (27.13): a `internos`, sin `actor` ni campo |
| `puerto generar objeto-valor <Nombre> <tipo> [--ejemplo <valor>]` | Añade un bloque `[[objeto_valor]]` (con `ejemplo`, 27.15; `<tipo>` puede ser una clase, `email` o `dinero`, que escribe su `tipo` y `clase`, 27.16). Rechaza nombres o tipos inválidos, duplicados y un ejemplo que no es de su tipo |
| `puerto generar transicion <Entidad> <verbo> [--evento <tipo>] [--rol <rol>[,…]] [--interno]` | Entidades CRUD con presentación (27.12): añade el verbo a `transiciones`, su caso de uso a `casos_de_uso`, el evento a `eventos` (proyectos con eventos) y los roles a `roles` (proyectos con auth); escribe `dominio/src/<verbo>.rs`, `aplicacion/src/<verbo>_<entidad>.rs`, `rest/src/<verbo>_<entidad>.rs` (sus `pub mod`) y, con pantallas, `contenedores/<verbo>-<entidad>.tsx` + su test, una línea en el detalle y otra en `mensajes.ts`; regenera el cableado. `--interno` (27.13): sin ruta ni botón, el caso de uso en `internos`, sin `actor`; no va con `--rol`. Rechaza un verbo CRUD, `validar`, `new`, un módulo del dominio, una palabra clave, uno que ya existe y una entidad que no es CRUD |
| `puerto generar evento <Entidad> <nombre>`      | Proyectos con eventos, entidades CRUD: añade a `[[entidad]] eventos`, regenera `eventos.rs`. Rechaza un nombre CRUD, un duplicado (también por variante: `publicado` = `producto_publicado`) o un nombre que no es snake_case. No toca ningún caso de uso |
| `puerto generar web-bloque [<hueco>/<variante>] [--forzar]` | Proyectos con web: sin argumento, el catálogo; con uno, escribe el fichero de su hueco (ADR-0028). No toca puerto.toml. Rechaza un bloque desconocido, uno de auth sin `auth` y uno de la portada sin `portada` (las secciones, sin `portada/producto`) |
| `puerto generar web <Nombre> [--forzar]`        | Proyectos con web: el slice del frontend de la entidad + `core/generated/*`. No toca puerto.toml; rechaza una entidad no declarada o no CRUD |
| `puerto generar migracion <nombre>`             | Crea una migración en `migraciones/` (solo proyectos `--db`) — no toca puerto.toml. La versión es única aunque sea en el mismo segundo |
| `puerto generar consumidor <nombre> --tras <tipo>[,…] [--usa <caso_de_uso>[,…]]` | Proyectos con eventos (`db` o `kafka`): añade `[[consumidor]]` (`--usa` no se guarda: un campo por caso de uso en el consumidor, sacado de `cableado`, y `cableado` + `aplicacion_<c>` en el `Cargo.toml` del worker), escribe `presentacion/worker/src/consumidores/<nombre>.rs` (nunca lo sobrescribe), añade su `pub mod` a `consumidores.rs` (ordenado), regenera `generado/registro.rs`. Rechaza un nombre existente, un tipo de evento no declarado, un nombre que no es snake_case o uno cuyo tipo ya usa el worker |
| `puerto generar correo <nombre> [-- campo:Tipo …]` | Cualquier proyecto: añade `[[correo]]`, escribe el puerto en `dominio_correo`, el enviador con plantillas en `infra_correo` y las tres plantillas (`asunto.txt`, `cuerpo.html`, `cuerpo.txt`; nunca las sobrescribe) y sus `pub mod`. El primero añade `puerto-correo`, Mailpit, `SMTP_URL` / `MAIL_FROM`, `make correo/ui`. Campos: solo primitivos y `opt:`, nunca el destinatario (siempre está); sin campos → `nombre:String`. Rechaza un nombre existente, uno que no es snake_case o es palabra clave, un objeto valor, `!`, `vec:`/`map`. `--tras <tipo>[,…] --para <campo>` (proyectos con eventos): escribe `[[correo]] tras` y `presentacion/worker/src/consumidores/correo_<nombre>.rs`, regenera `registro.rs`, añade una vez la migración de `correos_enviados`; `--para` tiene que ser un campo `String` del snapshot de cada evento, y los campos tras `--` tienen que estar en cada snapshot con el mismo tipo (sin ellos → los campos imprimibles del snapshot salvo `--para`) |
| `puerto validar`                                | Valida nombres, tipos, duplicados, y que cada entidad declarada existe en disco; `eventos` es `db`/`kafka` y necesita `db = true`; los eventos propios siguen las reglas de `generar evento`; los consumidores son snake_case, únicos, nombran eventos declarados y tienen su fichero en disco; los correos son snake_case, únicos, tienen sus ficheros en disco, y `tras` (solo proyectos con eventos) nombra eventos declarados; `auth` es `oidc`/`local` (`local` necesita `db`); `publicos`/`roles` solo en proyectos con auth, con casos de uso de la entidad, nunca los dos para uno; `Correo` es un nombre de entidad reservado, y `Usuario`, `Me`, `Auth` en proyectos con auth; una `referencia` nombra otra entidad CRUD declarada, es un `Uuid` / `Option<Uuid>` y no cierra un ciclo; deriva: un handler `publicos` no recibe `BearerAuth`, todos los demás sí, y cada rol tiene su `#[requiere_rol("<rol>")]` |

`db` es un ajuste **del proyecto** (`[proyecto] db`) que se elige una vez con `puerto nuevo --db`.
`generar scaffold` no tiene `--db`: lee el ajuste del proyecto en puerto.toml.
