# Guía de arquitectura

Lo que Puerto genera: un crate por contexto y capa (ADR-0023), todo en castellano (ADR-0024). Los
nombres salen de `docs/glosario.md`; los ejemplos usan la entidad `Producto` y el caso de uso
`crear_producto`.

## Regla de dependencias (NUNCA romperla)

```
rest_<c> → aplicacion_<c> → dominio_<c> → nucleo, puerto-nucleo
infra_<c> → dominio_<c>
cableado, api, worker → todo lo anterior (solo ellos dependen de infra_*)
```

- **Dominio** no depende de NADA más que `std`, `async-trait`, `thiserror`, `nucleo` y el núcleo
  del runtime `puerto-nucleo` (sin driver, sin runtime, sin E/S: `thiserror`, `async-trait`,
  `serde_json`, `uuid`, `chrono` — ADR-0017, ADR-0018)
- **Aplicación** depende solo de su dominio — los casos de uso (más `tracing` para los logs)
- **Infraestructura** depende de su dominio — adaptadores (en memoria, Postgres)
- **REST** depende de su aplicación y su dominio — rutas, DTO, respuestas, errores
- **Presentación** (`cableado`, `api`, `worker`) depende de todo, por inyección manual
- Ningún crate de dominio depende de un crate de aplicación. **Las referencias son
  dependencias**: `cliente_id:ref:Cliente` hace que `dominio_pedido` dependa de `dominio_cliente`
  (por `IdCliente`, que vive con su dueño). Cargo no admite ciclos: `scaffold` y `validar` rechazan
  la referencia que cerraría uno, nombrándolo

Cargo impone la regla: el `Cargo.toml` de cada crate solo nombra lo que la ADR-0023 § 3 permite
(hay un test por regla).

---

## Estructura del proyecto

```
Cargo.toml                members = ["nucleo", "migrador", "contextos/*/*", "presentacion/*"]
puerto.toml
nucleo/                   → nucleo            objetos valor compartidos (valores.rs, errores.rs). Sin E/S
migraciones/              el SQL de toda la base de datos                  (proyectos --db)
migrador/                 → migrador          MIGRADOR + build.rs              (proyectos --db)
contextos/<c>/
  dominio/                → dominio_<c>       modelo, valores, errores, repositorio, unit_of_work,
                                              eventos; pruebas/madres.rs
  aplicacion/             → aplicacion_<c>    un fichero por caso de uso
  infraestructura/        → infra_<c>         en memoria o Postgres: fila, repositorio, unit_of_work;
                                              pruebas/siembra.rs
  rest/                   → rest_<c>          rutas, dto, respuestas, errores
presentacion/
  cableado/               → cableado          GENERADO entero, Cargo.toml incluido
  api/                    → api               el binario: main.rs + lib.rs, app.rs, generado/bootstrap.rs
  worker/                 → worker            (proyectos con eventos) generado/registro.rs, consumidores/
web/                      el frontend: solo presentación (ADR-0025), en castellano (`docs/glosario.md` § Fase 22)
```

- En la v1 **una entidad es un contexto**: `[[entidad]] nombre = "LineaPedido"` →
  `contextos/linea_pedido/`, crates `dominio_linea_pedido`, … Sus módulos van en la raíz de cada
  crate (`dominio_producto::modelo`).
- `members` va por glob: un contexto nuevo no toca la lista. Los crates internos se declaran en
  `[workspace.dependencies]` y cada `Cargo.toml` los usa con `{ workspace = true }`.
- Los correos son el contexto `correo` (dominio e infraestructura, sin aplicación); el usuario de
  auth, el contexto `usuario`.

### `lib.rs` plano, sin `mod.rs`

El `lib.rs` de cada crate se escribe entero: una línea `pub mod` por fichero. Un módulo con hijos
se declara en línea:

```rust
// contextos/producto/dominio/src/lib.rs
pub mod errores;
pub mod modelo;
pub mod repositorio;

#[cfg(any(test, feature = "pruebas"))]
pub mod pruebas {
    pub mod madres;
}
```

`puerto generar caso-de-uso` solo añade una línea `pub mod <caso>;` al `lib.rs` de `aplicacion_<c>`.

---

## Dominio (`contextos/<c>/dominio/` → `dominio_<c>`)

**Lógica de negocio pura. Sin dependencias de infraestructura ni de presentación.**

```
dominio/src/
  modelo.rs          # la entidad, su id, PropsProducto, constructores
  errores.rs         # ErrorProducto (thiserror) + From<ErrorRepositorio>
  repositorio.rs     # el puerto RepositorioProducto + su doble (dobles::MockRepositorioProducto)
  valores.rs         # objetos valor locales (si la entidad los tiene)
  eventos.rs         # proyectos con eventos, entidades CRUD (GENERADO)
  unit_of_work.rs    # proyectos con eventos, entidades CRUD
  pruebas/madres.rs  # MadreProducto, tras la feature `pruebas`
```

El dominio no tiene `use_cases/`: la entrada y la salida de un caso de uso son de la aplicación.

### Constructores del modelo

Todo modelo de dominio tiene dos:

```rust
impl Producto {
    /// Crea una entidad nueva — valida todos los campos.
    pub fn new(props: PropsProducto) -> Result<Self, ErrorProducto> { ... }

    /// La reconstruye desde la persistencia — sin validar. SOLO la usa la infraestructura.
    pub fn from_repositorio(data: Producto) -> Self { ... }
}
```

Y dos reglas de negocio: `actualizar(&mut self, props)` (las mismas reglas que `new`) y
`borrar(&mut self)` (el borrado lógico: `borrado`, `borrado_en`, `actualizado_en`). En proyectos con
eventos la entidad emite sus eventos (ADR-0026): `crear(props) -> Result<(Producto, EventoProducto),
_>` (`new` + `Creado`; `new` sigue para la madre y los tests), y `actualizar` / `borrar` devuelven
su `EventoProducto`. `EventoProducto` es `#[must_use]`: un evento que nadie guarda es un aviso.

Con `[[entidad.campos]]` en `puerto.toml`, los structs llevan los campos propios:

```rust
#[derive(Debug, Clone)]
pub struct PropsProducto {
    pub nombre: String,
    pub precio: i64,
    pub sku: String,
    pub descripcion: Option<String>,
    pub etiquetas: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct Producto {
    pub id: IdProducto,
    pub creado_en: DateTime<Utc>,
    pub actualizado_en: DateTime<Utc>,
    pub borrado: bool,
    pub borrado_en: Option<DateTime<Utc>>,
    // Campos propios de puerto.toml
    pub nombre: String,
    pub precio: i64,
    pub sku: String,
    pub descripcion: Option<String>,
    pub etiquetas: Vec<String>,
}
```

Sin `campos` (o vacíos), la entidad lleva un único campo `nombre: String`.

### Ids tipados

El id de cada entidad es su propio newtype, declarado en `modelo.rs` encima de la entidad — pasar
un `IdCliente` donde se espera un `IdProducto` no compila:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct IdProducto(Uuid);

impl IdProducto {
    pub fn generar() -> Self { Self(Uuid::new_v4()) }       // Producto::new
    pub fn from_uuid(value: Uuid) -> Self { Self(value) }   // un parámetro de ruta, una fila
    pub fn valor(&self) -> Uuid { self.0 }                  // un DTO, un bind
}
// + Display, para que `tracing::info!(id = %entidad.id, …)` funcione
```

El id es un `Uuid` solo en los bordes: `Path<Uuid>` en las rutas (envuelto con `from_uuid`),
`id: Uuid` en los DTO y en las filas `Fila<Entidad>` (`.valor()`). Los Params, el puerto del
repositorio y los tests usan el newtype.

### Validación en `new()`

- Los campos `String` se validan con `.trim().is_empty()` → `Validacion("<campo>_vacio")`
- Los `Option<String>` no se validan (pueden ser `None`)
- Los numéricos y demás tipos pasan sin validación de dominio

### Puerto del repositorio

```rust
#[async_trait]
pub trait RepositorioProducto: Send + Sync {
    async fn buscar_todos(&self, pagina: PeticionPagina) -> Result<Pagina<Producto>, ErrorRepositorio>;
    async fn buscar_por_id(&self, id: IdProducto) -> Result<Option<Producto>, ErrorRepositorio>;
    async fn guardar(&self, entidad: &Producto) -> Result<(), ErrorRepositorio>;
}

#[cfg(any(test, feature = "pruebas"))]
pub mod dobles {
    use mockall::mock;
    use super::*;

    mock! {
        pub RepositorioProducto {}
        #[async_trait]
        impl RepositorioProducto for RepositorioProducto {
            async fn buscar_todos(&self, pagina: PeticionPagina) -> Result<Pagina<Producto>, ErrorRepositorio>;
            async fn buscar_por_id(&self, id: IdProducto) -> Result<Option<Producto>, ErrorRepositorio>;
            async fn guardar(&self, entidad: &Producto) -> Result<(), ErrorRepositorio>;
        }
    }
}
```

El doble va **junto a su trait**, en el mismo fichero, tras la feature `pruebas`. El trait lleva el
nombre del concepto; la implementación dice cómo lo hace (`RepositorioProductoEnMemoria`,
`RepositorioProductoPostgres`). Sin sufijo `Trait`.

### Patrón de error

```rust
#[derive(Debug, thiserror::Error)]
pub enum ErrorProducto {
    #[error("producto.validacion.{0}")]
    Validacion(String),   // "nombre_vacio", "cliente_id_no_encontrado"…
    #[error("producto.no_encontrado")]
    NoEncontrado,
    #[error("producto.duplicado")]
    Duplicado,
    #[error("producto.repositorio")]
    Repositorio,
    #[error("producto.desconocido")]
    Desconocido,
    // un objeto valor inválido: <Vo>Invalido → "producto.<vo>_invalido"
    // proyectos con auth: Prohibido → "producto.prohibido" (403)
}
```

Los códigos de error son **identificadores legibles por máquina**, nunca mensajes para personas.

### Error de repositorio

Los puertos de repositorio devuelven un único error compartido, sea cual sea la entidad y el
adaptador (ADR-0015):

```rust
// puerto_nucleo::ErrorRepositorio — crates/nucleo en el workspace de Puerto (ADR-0017), #[non_exhaustive]
pub enum ErrorRepositorio {
    Duplicado,                           // una restricción única rechazó la escritura
    DatosInvalidos,                      // una fila guardada incumple las reglas del dominio
    NoDisponible,                        // el almacén falló o no se alcanza
    ReferenciaInexistente(&'static str), // una clave foránea rechazó la escritura (ADR-0022)
}
```

Cada error de entidad lo convierte — `impl From<ErrorRepositorio> for ErrorProducto` (`Duplicado →
Duplicado`, el resto → `Repositorio`, más un brazo `_` porque el enum es `#[non_exhaustive]`;
con referencias, `ReferenciaInexistente(campo) → Validacion("<campo>_no_encontrado")`) — para que
los casos de uso lo propaguen con `?` y la API siga respondiendo con los códigos de la entidad.

### Listas paginadas

`puerto_nucleo::{PeticionPagina, Pagina}` (ADR-0016): `PeticionPagina::new(limite, desplazamiento)`
— campos privados; `limite` vale 20 por defecto (`LIMITE_PAGINA_POR_DEFECTO`) y se acota a
`1..=100` (`LIMITE_PAGINA_MAXIMO`) — y `Pagina<T> { elementos, total, limite, desplazamiento }`. El
puerto CRUD lee `buscar_todos(&self, pagina: PeticionPagina) -> Result<Pagina<Producto>,
ErrorRepositorio>`, `ParamsListarProductos { pagina }` devuelve una `Pagina` y
`GET /productos?limite=&desplazamiento=` responde un `DtoPaginaProducto`
(`{ elementos, total, limite, desplazamiento }`). Postgres:
`ORDER BY creado_en DESC, id LIMIT $1 OFFSET $2` + `COUNT(*)`.

### Referencias (ADR-0022)

`cliente_id:ref:Cliente` — un agregado guarda el **id** de otro, nunca el agregado: sin
navegación ni carga perezosa; un caso de uso que necesita el cliente se lo pide a su repositorio.

- Dominio, Params, puerto del repositorio, madre y tests: `IdCliente` (importado de
  `dominio_cliente::modelo`). Filas, DTO y el snapshot de eventos: `Uuid` (`from_uuid` /
  `.valor()` en los bordes, como el id propio de la entidad)
- La existencia la comprueba la base de datos, no el caso de uso:
  `CONSTRAINT pedidos_cliente_id_fkey REFERENCES clientes (id)` + `pedidos_cliente_id_idx`. La
  escritura que la incumple se traduce, con las `REFERENCES: &[ClaveForanea]` del repositorio (o de
  la unidad de trabajo), en `ErrorRepositorio::ReferenciaInexistente("cliente_id")` →
  `ErrorPedido::Validacion("cliente_id_no_encontrado")` → 422
- El borrado lógico deja las referencias apuntando a la fila borrada; los proyectos en memoria no
  comprueban nada
- La lista filtra por cada referencia: `ParamsListarPedidos { pagina, cliente_id:
  Option<IdCliente>, … }` → `buscar_todos(pagina, FiltroPedido { cliente_id, … })` —
  `FiltroPedido` se declara junto al puerto, solo para una entidad con referencias. Postgres: una
  sola consulta estática, `AND ($3::uuid IS NULL OR cliente_id = $3)` en la página y en su
  `COUNT`. `GET /pedidos?clienteId=&limite=&desplazamiento=`
- Web: el `<select>` del formulario lista los primeros `LIMITE_PAGINA_MAXIMO` elementos del
  objetivo, cargados una vez por entidad objetivo (`opcionesReferencia(cargados, etiqueta)` en
  `compartido/formularios.ts`); el detalle muestra `<EnlaceReferencia ruta="/clientes" id={…} />`,
  con la etiqueta que carga su contenedor; `mensajes.ts` asocia
  `cliente_id_no_encontrado` a `clienteId`

### Eventos de dominio (`[proyecto] eventos` — ADR-0018)

Un proyecto creado con `puerto nuevo --db --eventos db|kafka`. Cada entidad **CRUD** gana dos
ficheros de dominio; las entidades simples (el demo Saludo) no cambian:

```
dominio/src/
  eventos.rs        # GENERADO desde puerto.toml, nunca a mano: SnapshotProducto (primitivos con la
                    #   forma del DTO, serde camelCase, from_entidad), EventoProducto
                    #   (Creado/Actualizado/Borrado + propios), impl puerto_nucleo::EventoDominio,
                    #   pub const PRODUCTO_CREADO = "producto_creado"…
  unit_of_work.rs   # UnitOfWorkProducto (guardar / registrar / commit(self: Box<Self>)) y
                    #   UnitOfWorkFactoryProducto (begin, guardar(&entidad, evento));
                    #   dobles::UnitOfWorkFactoryGrabadora
```

- El puerto del repositorio **no tiene `guardar`**: solo lee. El evento lo emite la entidad
  (`let evento = entidad.actualizar(props)?;`) y la escritura es
  `uow_factory.guardar(&entidad, evento)` (ADR-0026), un método por defecto que hace `begin()` →
  `guardar` → `registrar` → `commit`. Para escribir más de una vez, esos pasos a mano: soltar la
  unidad de trabajo sin `commit` deshace — un `?` temprano es el rollback
- `crear` usa solo la factoría de unidades de trabajo; `obtener`/`listar`, solo el repositorio;
  `actualizar`/`borrar` y cada caso de uso propio, los dos. `cableado` construye el repositorio y
  la factoría una vez por contexto
- Infraestructura: `unit_of_work.rs` — `UnitOfWorkFactoryProductoPostgres { pool }` sobre
  `puerto_sqlx::uow::TransaccionPostgres`; el upsert vive ahí, no en el repositorio Postgres. Sus
  `#[sqlx::test]` comprueban que la fila y su evento del outbox se confirman (y se deshacen) juntos
- `eventos.rs` lo reescriben `scaffold`, `generar bootstrap` y `generar evento`; los modelos de
  dominio y los objetos valor siguen sin serde. El snapshot es un contrato público: los
  consumidores lo decodifican
- Los tests usan `UnitOfWorkFactoryGrabadora`
  (`fallando_en(FallarEn::Guardar, ErrorRepositorio::Duplicado)`, `grabado()` → `guardados`,
  `eventos`, `confirmado`)

### El worker (`[proyecto] eventos` — ADR-0018 § 5)

Un segundo binario junto a `api`, y una raíz de composición como él:

```
presentacion/worker/src/
  main.rs                    # ConfigWorker::from_env() + Worker::new(config).intentar_ejecutar(construir_worker) — no cambia
  generado/registro.rs       # GENERADO desde [[consumidor]]: el relay + un consumidor cada uno — nunca a mano
  consumidores.rs            # pub mod <consumidor>; (ordenados)
  consumidores/<nombre>.rs   # el ManejadorEvento del usuario, escrito una vez por `puerto generar consumidor`
  consumidores/comun.rs      # lo que comparten varios consumidores (27.14), del usuario; no es un consumidor
```

- Un consumidor compara `event.tipo_evento` con las constantes `<ENTIDAD>_<VERBO>` de la entidad,
  decodifica `SnapshotProducto` (un error de decodificación →
  `ResultadoManejo::descartar(MOTIVO_PAYLOAD_INVALIDO, …)`) y actúa. Tiene que ser idempotente: la
  entrega es al menos una vez, con `event.id` como clave
- `Ack` / `Reintentar` (se reentrega con la espera de la política hasta las entregas máximas) /
  `Descartar` — nada se retira sin una fila en `descartados`
- Los bucles, `/health` + `/health/ready` en `WORKER_PORT` y el drenaje con SIGTERM son de
  `puerto-eventos`; los adaptadores del transporte `db`, de `puerto_sqlx::eventos`
- `registro.rs` construye cada consumidor con `<Consumidor>::construir(&pool)`, que es del usuario:
  ahí saca de `cableado::<c>::construir(pool)` los casos de uso que necesita y los guarda en sus
  campos. `generar consumidor … --usa <caso_de_uso>,…` (27.14) lo escribe, con `cableado` y los
  `aplicacion_<c>` en el `Cargo.toml` del worker y un doble por caso de uso en sus tests. El worker
  no tiene usuario: el `actor` de un caso de uso protegido es `Principal::sistema("<consumidor>")`,
  y una lista entera se lee con `puerto_nucleo::todas_las_paginas`
- `kafka` (`puerto-kafka`): el relay sigue leyendo `OutboxPostgres` y los descartados siguen en
  `DescartadosPostgres`; los eventos van a `<KAFKA_TOPIC_PREFIX>.<agregado>` con el id del agregado
  como clave, y un consumidor es el grupo `<prefijo>.<consumidor>`. Reparte un registro cada vez:
  un `Reintentar` se reintenta en proceso y retiene su partición, nunca se reordena. Un registro que
  no es un sobre se descarta (`id_evento` NULL)

### Correo (ADR-0020)

`puerto generar correo <nombre>` escribe, una vez, en el contexto `correo` — del usuario desde
entonces:

```
contextos/correo/dominio/src/<nombre>.rs          # Correo<Nombre> (destinatario + campos primitivos) + el
                                                  #   puerto EnviadorCorreo<Nombre> + dobles::MockEnviadorCorreo<Nombre>
                                                  #   — sin motor de plantillas, sin SMTP
contextos/correo/infraestructura/src/<nombre>.rs  # EnviadorCorreo<Nombre>Plantilla: renderiza las plantillas
                                                  #   (puerto_correo::Plantillas) y envía el Correo
contextos/correo/infraestructura/plantillas/<nombre>/
                                                  # asunto.txt, cuerpo.html, cuerpo.txt — minijinja, estricto,
                                                  #   embebidas con include_str!; tests de renderizado en el fichero de arriba
```

- Ninguna entidad puede llamarse `Correo`: el contexto es de los correos
- El código que envía un correo depende de `EnviadorCorreo<Nombre>`; en la v1, un consumidor del
  worker (`--tras`, proyectos con eventos): `presentacion/worker/src/consumidores/correo_<nombre>.rs`
  (`EnviarCorreo<Nombre>`) decodifica el snapshot, construye `Correo<Nombre>` y llama a
  `puerto_correo::enviar_una_vez` — mira el registro `correos_enviados`, envía, marca. El registro
  lo monta como el consumidor `correo_<nombre>` con un `EnviadorCorreoSmtp` (`SMTP_URL`,
  `MAIL_FROM`) y un `CorreosEnviadosPostgres` que comparten todos los correos

---

## Aplicación (`contextos/<c>/aplicacion/` → `aplicacion_<c>`)

**Orquesta la lógica de dominio. Un fichero por caso de uso, con todo lo suyo** (ADR-0023 § 4):

```rust
// contextos/producto/aplicacion/src/crear_producto.rs
pub struct ParamsCrearProducto { pub nombre: String }          // 1. la entrada

#[async_trait]
pub trait CrearProducto: Send + Sync {                         // 2. el contrato
    async fn ejecutar(&self, params: ParamsCrearProducto) -> Result<Producto, ErrorProducto>;
}

#[cfg(any(test, feature = "pruebas"))]
pub mod dobles { /* mock! { pub CrearProducto {} … } → MockCrearProducto */ }   // 3. su doble

pub struct CrearProductoImpl {                                 // 4. la implementación
    pub repositorio: Arc<dyn RepositorioProducto>,
}

#[async_trait]
impl CrearProducto for CrearProductoImpl {
    #[tracing::instrument(name = "crear_producto", skip_all, err(level = "warn"))]
    async fn ejecutar(&self, params: ParamsCrearProducto) -> Result<Producto, ErrorProducto> {
        let entidad = Producto::new(PropsProducto { nombre: params.nombre })?;
        self.repositorio.guardar(&entidad).await?;
        Ok(entidad)
    }
}

#[cfg(test)]
mod tests { /* 5. sus tests */ }
```

- El trait lleva el nombre del caso de uso (`CrearProducto`), la implementación `…Impl`. Se
  mantiene el trait en todos: las rutas guardan `Arc<dyn CrearProducto>` y se prueban con dobles
- Los casos de uso CRUD: `crear_<e>`, `obtener_<e>`, `listar_<plural>`, `actualizar_<e>`,
  `borrar_<e>`
- `ParamsCrearProducto` lleva todos los campos propios (sin los de sistema);
  `ParamsActualizarProducto`, además, `id: IdProducto`. Los Params usan tipos primitivos; la
  implementación construye los objetos valor antes de `Producto::new()`
- Las dependencias se inyectan como `Arc<dyn Trait>` — nunca tipos concretos
- **Logs con `tracing` directamente** — sin logger inyectado ni puerto `Logger` (ADR-0006).
  `tracing` ya es una fachada: el subscriber lo instala una vez `puerto_web::App`. El `ejecutar`
  de un caso de uso lleva `#[tracing::instrument(name = "<caso>", skip_all, err(level = "warn"))]`
  (con `fields(id = %params.id)` si hay id) y no escribe líneas propias (ADR-0026): el span da
  nombre a los logs del adaptador, que registra cada fallo con su causa, y `err` registra una vez
  el código del error. `skip_all`: nunca se registran los datos de la petición, que son del usuario
- El caso de uso se renderiza con dos plantillas (contrato e implementación) que el generador une
  bajo un solo bloque de `use` (`generators/merge.rs`): dos `use` del mismo nombre son E0252

---

## Infraestructura (`contextos/<c>/infraestructura/` → `infra_<c>`)

**Adaptadores de sistemas externos. Sin lógica de negocio.**

Cuál se genera se deduce de `[proyecto] db` en puerto.toml — se fija una vez con
`puerto nuevo --db`. `generar scaffold` no tiene `--db` propio.

### En memoria (por defecto — `db` ausente o false)

```
infraestructura/src/
  repositorio.rs   # RepositorioProductoEnMemoria — implementa el puerto del dominio
```

### SQLx/Postgres (`db = true`)

```
infraestructura/src/
  fila.rs              # FilaProducto (#[derive(FromRow)]) + TryFrom + From
  repositorio.rs       # RepositorioProductoPostgres { pool: PgPool } — implementa el puerto
  unit_of_work.rs      # proyectos con eventos
  pruebas/siembra.rs   # construir / sembrar, para los tests de Postgres (feature `pruebas`)
migraciones/           # en la raíz: el SQL de todos los contextos
migrador/              # en la raíz: pub static MIGRADOR = sqlx::migrate!("../migraciones")
```

**Reglas:**

- `from_repositorio()` es la **única** entrada desde la persistencia al dominio — nunca se llama a
  `Producto::new()` dentro de un repositorio
- `TryFrom<FilaProducto> for Producto` convierte fila → dominio (puede fallar)
- `From<&Producto> for FilaProducto` convierte dominio → fila (no falla)
- No hay `db.rs`: el pool, `ejecutar_migraciones`, `ping` y `mapear_error_sqlx` son de
  `puerto-sqlx` (ADR-0017) y se importan de ahí; el proyecto solo aporta `migrador::MIGRADOR`
  (`sqlx::migrate!` embebe `migraciones/` al compilar; el `build.rs` de `migrador`, con
  `cargo:rerun-if-changed`, hace que vea una migración nueva aunque no cambie ningún `.rs`)
- Los tests de Postgres usan `#[sqlx::test(migrator = "migrador::MIGRADOR")]`: ningún test lleva
  su propia copia de las migraciones (ADR-0023 § 6)
- Los helpers de siembra (`construir`, `sembrar`) van en `infra_<c>::pruebas::siembra`, públicos
  tras `pruebas`, y van en cadena por las referencias: `infra_pedido` declara `infra_cliente` como
  dependencia opcional (`pruebas = ["dominio_pedido/pruebas", "dep:infra_cliente",
  "infra_cliente/pruebas"]`). Dentro del módulo de los `#[sqlx::test]` quedarían helpers sin usar,
  y el `make lint` (`-D warnings`) los rechaza
- Un campo `HashMap<String, String>` es una columna `JSONB`: el campo de la fila es
  `sqlx::types::Json<HashMap<String, String>>`, el `SELECT` sobrescribe el tipo de la columna
  (`meta as "meta: Json<HashMap<String, String>>"`) y el bind lo envuelve en `Json(…)`
- Los campos `unico` tienen un índice único parcial (`productos_sku_key … WHERE borrado = false`):
  las filas se borran de forma lógica
- Los repositorios devuelven `ErrorRepositorio`: registran el error del driver con su contexto y lo
  convierten con `puerto_sqlx::mapear_error_sqlx` (`_con(err, REFERENCES)` si la entidad tiene
  referencias) — el único sitio que conoce los códigos de Postgres (`23505` → `Duplicado`, `23503`
  → `ReferenciaInexistente`); `to_dominio(fila)` convierte una fila que `TryFrom` rechaza en
  `DatosInvalidos`
- Nunca un `map_err` que devuelve su entrada sin cambiarla (clippy `manual_inspect` en el
  `make lint` del proyecto generado)

**Con `[[entidad.campos]]`**, `FilaProducto` lleva las columnas propias junto a las de sistema:

```rust
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct FilaProducto {
    pub id: Uuid,
    pub creado_en: DateTime<Utc>,
    pub actualizado_en: DateTime<Utc>,
    pub borrado: bool,
    pub borrado_en: Option<DateTime<Utc>>,
    // Columnas propias de puerto.toml
    pub nombre: String,
    pub precio: i64,
    pub sku: String,
    pub descripcion: Option<String>,
    pub etiquetas: Vec<String>,
}
```

Los tipos SQL salen del registro de tipos (ver `puerto-toml.md`). La tabla es el plural de la
entidad (`productos`).

---

## REST (`contextos/<c>/rest/` → `rest_<c>`)

**Solo puntos de entrada. Sin lógica de negocio.**

```
rest/src/
  lib.rs           # pub mod dto; pub mod errores; pub mod respuestas; pub mod rutas;
  rutas.rs         # ApiProducto: endpoints, auth, lectura de parámetros, tests de rutas
  dto.rs           # DtoProducto, DtoPaginaProducto, PeticionCrearProducto… (#[derive(Object)])
  respuestas.rs    # RespuestaCrearProducto… (#[derive(ApiResponse)])
  errores.rs       # a_respuesta_error(ErrorProducto) -> (StatusCode, Json<RespuestaError>)
```

### Los errores, con una función libre

Un `rest_<c>` no puede escribir `impl IntoRespuestaError for ErrorProducto`: ni el trait
(`puerto-web`) ni el tipo (`dominio_producto`) son suyos (la orphan rule). `errores.rs` exporta la
función libre `a_respuesta_error(err)`, exhaustiva (sin brazo `_`) y con un test por rama
(ADR-0023 § 8).

### DTO

Los DTO derivan el `Object` de poem-openapi (no serde) y el contrato JSON es **camelCase**: un campo
`publicado_en` es `publicadoEn` en el JSON.

```rust
#[derive(Debug, Object)]
#[oai(rename_all = "camelCase")]
pub struct PeticionCrearProducto {
    pub nombre: String,
    pub precio_unidad: i64,       // JSON: "precioUnidad"
    pub descripcion: Option<String>,
}

#[derive(Debug, Object)]
#[oai(rename_all = "camelCase")]
pub struct DtoProducto {
    pub id: Uuid,
    pub nombre: String,
    // ...
}
```

Cada tipo OpenAPI sale del registro de tipos (`i64` → `integer(int64)`, `Uuid` → `string(uuid)`).

### Reglas

- **API primero**: `rutas.rs` + `dto.rs` antes que cualquier implementación
- Los modelos de dominio **nunca** salen en una respuesta — siempre por
  `DtoProducto::from_dominio(&entidad)`
- Todo enum `ApiResponse` tiene un `from_status(StatusCode, Json<RespuestaError>) -> Self`. Sus
  variantes: `Ok`, `Creado`, `SinContenido`, `NoProcesable`, `NoAutorizado`, `Prohibido`,
  `NoEncontrado`, `Conflicto`, `NoDisponible`, `ErrorInterno`
- Todos los errores usan `puerto_web::RespuestaError { codigo, mensaje }` (ADR-0017), construido con
  `RespuestaError::from_codigo(err.to_string())`: **`codigo` es el código legible por máquina**
  (`producto.no_encontrado`) — ADR-0007
- Estados: validación de dominio y objetos valor inválidos → **422**; no encontrado → 404;
  duplicado → 409; prohibido → 403; repositorio / desconocido → 500. El `400` se deja a poem para
  los cuerpos mal formados
- Los handlers CRUD son `crear`, `obtener`, `listar`, `actualizar`, `borrar`; las rutas,
  `/productos` y `/productos/:id`
- `Api<Entidad>` guarda **puertos**: `Arc<dyn CrearProducto>`. Solo `cableado` (la raíz de
  composición) nombra las `…Impl`
- Cada caso de uso trae `dobles::Mock<Caso>` (misma puerta que los dobles del repositorio), y el
  `rutas.rs` CRUD se genera con tests de rutas: `poem::test::TestClient` sobre el
  `OpenApiService` real, un caso de uso doblado por test, un test por camino de estado (ADR-0012).
  `rest_<c>` declara `aplicacion_<c>` con `features = ["pruebas"]` en `[dev-dependencies]`

```rust
pub struct ApiProducto {
    pub crear_producto: Arc<dyn CrearProducto>,
}

#[OpenApi]
impl ApiProducto {
    #[oai(path = "/productos", method = "post")]
    async fn crear(&self, cuerpo: Json<PeticionCrearProducto>) -> RespuestaCrearProducto {
        match self.crear_producto.ejecutar(ParamsCrearProducto { nombre: cuerpo.nombre.clone() }).await {
            Ok(entidad) => RespuestaCrearProducto::Creado(Json(DtoProducto::from_dominio(&entidad))),
            Err(err) => {
                let (status, error) = a_respuesta_error(err);
                RespuestaCrearProducto::from_status(status, error)
            }
        }
    }
}
```

---

## Presentación (`presentacion/`)

```
presentacion/
  cableado/src/lib.rs           # GENERADO: pub mod <c> { pub struct CasosDeUso { … } pub fn construir(…) }
  api/src/
    main.rs                     # puerto_web::Config::from_env() + App::new(config).intentar_servir(…) — no cambia
    lib.rs                      # mod app; pub mod generado { pub mod bootstrap; }
    app.rs                      # el hook del usuario: configurar(rutas) — lo escribe `nuevo` una vez, nunca se regenera
    generado/bootstrap.rs       # GENERADO desde puerto.toml — nunca a mano
  worker/                       # proyectos con eventos (ver § El worker)
```

- **`cableado`** construye, a partir del pool, los repositorios, las unidades de trabajo y los
  casos de uso de cada contexto: `cableado::producto::construir(&pool)` devuelve sus `CasosDeUso`.
  No conoce poem. Su `Cargo.toml` también se genera, así que ningún fichero del usuario se parchea
  para añadir un `infra_*`
- **`generado/bootstrap.rs`** monta las `Api*` sobre `cableado`, el autenticador, el OpenAPI y las
  rutas, y termina en `crate::app::configurar(puerto_web::rutas(servicio_openapi(…), &config, …))`. El `Cargo.toml` de `api` recibe los `rest_*`. El cableado de auth se queda aquí: pide
  configuración (el proveedor OIDC, la duración de las sesiones)
- Los regeneran `puerto generar scaffold <Nombre>` (automático) y `puerto generar bootstrap`
  (manual). **Nunca se editan a mano**
- `app.rs` es del usuario: ningún generador lo escribe
- `api` tiene `lib.rs`: `main.rs` solo lo arranca (un binario sin `lib.rs` compila más lento)
- `/health` es solo vida; `/health/ready` sondea las dependencias (`puerto_sqlx::ping` en proyectos
  `--db`, entregado a `puerto_web::health::sondear`) y responde 503 si una está caída. SIGTERM
  drena las peticiones en curso durante `PLAZO_APAGADO` (ADR-0008) — todo lo sirve `puerto-web`
- `Config::from_env()` (`puerto-web`) lee `SERVICE_PORT`, `CORS_ALLOWED_ORIGINS` y `PUERTO_BANNER`
  después de `.env`; un valor puesto pero inválido es un error de arranque

---

## Convenciones de Rust

- **`snake_case`** — ficheros, módulos, variables, funciones
- **`PascalCase`** — structs, enums, traits
- **`SCREAMING_SNAKE_CASE`** — constantes
- Identificadores generados en castellano y sin tildes ni ñ (`anio`, `Autorizacion`); el concepto
  delante de la entidad (`RepositorioProducto`, `IdProducto`, `ErrorProducto`). En inglés solo lo
  que impone el lenguaje o una librería (`new`, `from_`, `as_`, el prefijo `Mock`), la jerga
  asentada (`id`, `Dto`, `Api`, `Params`, `Props`, `Impl`, `Snapshot`, `UnitOfWork`…) y las
  variables de entorno (ADR-0024). Una palabra que no está en `docs/glosario.md` se pregunta antes
- **Nunca** `unwrap()` ni `expect()` fuera de los tests
- Sin sufijo `Trait`: el trait es el concepto (`RepositorioProducto`, `CrearProducto`)
