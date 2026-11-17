# Contextos — notas para agentes de IA

Un directorio por contexto, cuatro crates cada uno (ADR-0023 de Puerto). Un contexto es una entidad
de `puerto.toml`, con su nombre en snake case: `contextos/producto/`.

## Estructura

```
contextos/<c>/
├── dominio/          → dominio_<c>       Rust puro: ni framework web, ni driver de base de datos, ni cliente HTTP
│   └── src/
│       ├── lib.rs                # un `pub mod` por fichero, escrito entero, sin `mod.rs`
│       ├── modelo.rs             # la entidad + sus Props + su tipo de id + los constructores
│       ├── valores.rs            # los objetos valor de esta entidad, si los tiene
│       ├── errores.rs            # el enum Error<Entidad> (thiserror, códigos legibles por máquina)
│       ├── repositorio.rs        # el trait Repositorio<Entidad> (puerto) + `dobles::MockRepositorio<Entidad>`
│       ├── eventos.rs            # GENERADO: snapshot + eventos (proyectos con `eventos`), no lo edites a mano
│       ├── unit_of_work.rs       # el puerto de escritura: begin → guardar → registrar → commit (proyectos con `eventos`)
│       └── pruebas/madres.rs     # la madre (Object Mother), tras la feature `pruebas`
├── aplicacion/       → aplicacion_<c>    un fichero por caso de uso
│   └── src/<accion>.rs           # Params<Accion> + el trait <Accion> + `dobles::Mock<Accion>` + <Accion>Impl + tests
├── infraestructura/  → infra_<c>         los adaptadores de los puertos del dominio
│   └── src/
│       ├── repositorio.rs        # Repositorio<Entidad>EnMemoria, o Repositorio<Entidad>Postgres (--db)
│       ├── fila.rs               # la fila `Fila<Entidad>` + sus conversiones (--db)
│       ├── unit_of_work.rs       # UnitOfWorkFactory<Entidad>Postgres (proyectos con `eventos`)
│       └── pruebas/siembra.rs    # `construir` / `sembrar`: las filas que necesitan los tests de Postgres (--db)
└── rest/             → rest_<c>          HTTP: poem-openapi
    └── src/
        ├── rutas.rs              # Api<Entidad>: impl #[OpenApi] con los endpoints + los tests de ruta
        ├── dto.rs                # structs de petición y respuesta (#[derive(Object)], JSON en camelCase)
        ├── respuestas.rs         # enums #[derive(ApiResponse)]
        └── errores.rs            # `a_respuesta_error(err)`: estado + cuerpo (+ un test por brazo)
```

`contextos/correo/` guarda los correos (`puerto generar correo`); en un proyecto con auth,
`contextos/usuario/` guarda a quien llama como un `Usuario`.

## Dependencias

```
rest_<c> → aplicacion_<c> → dominio_<c> → nucleo, puerto-nucleo
infra_<c> → dominio_<c>
```

Cargo lo hace cumplir: un crate solo ve lo que nombra su `Cargo.toml`. Ningún crate de dominio
depende de uno de aplicación; solo `presentacion/cableado` y los binarios dependen de `infra_*`.
Una referencia (`cliente_id:ref:Cliente`) hace que `dominio_pedido` dependa de `dominio_cliente`,
dueño de `IdCliente`: las referencias no pueden formar un ciclo, y `puerto validar` lo nombra si lo
forman.

Todos los crates del proyecto se declaran en el `Cargo.toml` raíz (`[workspace.dependencies]`) y se
nombran con `{ workspace = true }`. Las ayudas de test (los dobles, las madres, la siembra de
Postgres) están tras la feature `pruebas` de cada crate, que los demás crates activan en sus
`[dev-dependencies]`: `dominio_producto = { workspace = true, features = ["pruebas"] }`.

## Reglas críticas

- **Dos constructores**: `new(props)` valida y devuelve `Result<Self, Error…>`.
  `from_repositorio(datos)` se salta la validación; solo lo llama la infraestructura.
- **Las reglas de los campos viven en `validar(props)`**, en `modelo.rs`: la llaman `new` y
  `actualizar(props)`, así que una regla nueva se escribe una sola vez y vale al crear y al
  actualizar. `actualizar_<entidad>` no comprueba nada por su cuenta: llama a `entidad.actualizar`.
- **Ids tipados**: el id de una entidad es su propio tipo, `IdProducto` (`modelo.rs`):
  `IdProducto::generar()` para uno nuevo, `from_uuid` / `.valor()` solo en los bordes (rutas, DTO,
  filas).
- **Los repositorios devuelven `ErrorRepositorio`** (`puerto_nucleo`): `Duplicado`,
  `DatosInvalidos`, `NoDisponible`, `ReferenciaInexistente`. Cada error de entidad tiene
  `From<ErrorRepositorio>`, así que un caso de uso usa `?`. Es `#[non_exhaustive]`: el `From`
  conserva un brazo `_`.
- **Las listas paginadas** reciben un `puerto_nucleo::PeticionPagina` y responden un
  `puerto_nucleo::Pagina<T>`.
- **Un caso de uso es un fichero** del crate de aplicación: sus `Params`, su trait, el doble del
  trait y la `Impl` con sus tests. Las rutas guardan un `Arc<dyn …>` del trait.
- **Proyectos con eventos**: un puerto de repositorio solo lee. Una escritura pasa por la unidad de
  trabajo de la entidad (`uow_factory.begin()`, `guardar`, `registrar(Evento<Entidad>::…)`,
  `commit`), y sus tests usan `unit_of_work::dobles::UnitOfWorkFactoryGrabadora`.
- **Dependencias inyectadas como `Arc<dyn Trait>`**, nunca tipos concretos.
- **Nada de `unwrap()`/`expect()`** fuera de los tests (los módulos `pruebas` son código de test).
- **Logs con `tracing` directamente**, sin puerto de logger. Un caso de uso registra
  `<caso_de_uso>: inicio` y `<caso_de_uso>: fin`; nunca el contenido.
- **Objetos valor**: valor interno privado, `new()` que devuelve `Result`; los objetos valor enum
  implementan `std::str::FromStr` (`"Activo".parse::<Estado>()`) y `as_str()`.
- **Los códigos de error son legibles por máquina**: `"producto.no_encontrado"`,
  `"producto.validacion.nombre_vacio"`, nunca frases.
- **La infraestructura no tiene lógica de negocio**: `from_repositorio()` es la única puerta del
  almacenamiento al dominio. Registra el error del driver con su contexto y conviértelo después con
  `puerto_sqlx::mapear_error_sqlx` (`23505` → `Duplicado`); una fila que el dominio rechaza es
  `DatosInvalidos`. Una entidad con referencias escribe a través de `mapear_error_sqlx_con`, con sus
  `ClaveForanea`.
- **REST**: los DTO nunca exponen modelos de dominio (`Dto<Entidad>::from_dominio(&entidad)`); los
  mapeos de errores son exhaustivos y cada brazo tiene un test; cada enum `ApiResponse` tiene
  `from_status`.

## Tests

- Los tests viven **dentro del fichero que prueban**, en `#[cfg(test)] mod tests { ... }`, y se
  escriben antes que la implementación. Nombres: `deberia_<resultado>_cuando_<escenario>`, con
  `// Preparar`, `// Actuar`, `// Comprobar`.
- El test de un caso de uso usa un doble del repositorio:
  `dominio_<c>::repositorio::dobles::MockRepositorio<Entidad>`.
- Los tests de ruta conducen el `OpenApiService` real con `poem::test::TestClient`, con todos los
  casos de uso sustituidos por dobles (`aplicacion_<c>::<accion>::dobles::Mock<Accion>`). Un test
  por cada código de estado.
- Los tests de Postgres son `#[sqlx::test(migrator = "migrador::MIGRADOR")]`: una base de datos
  desechable por test, migrada por el único migrador del proyecto. Siembran filas con
  `pruebas::siembra::sembrar`, y las filas a las que se refiere una entidad con la del destino
  (`infra_cliente::pruebas::siembra::sembrar`).
- `cargo test -p dominio_producto`, o todos los dominios a la vez: `make test/dominio`.

## Madres (Object Mother)

`dominio_<c>::pruebas::madres::Madre<Entidad>`:

```rust
let producto = MadreProducto::aleatorio();
let invalidas = MadreProducto::new().con_nombre_vacio().construir_props();
let lote = MadreProducto::aleatorios(5);
```

Cada campo tiene `.con_<campo>(valor)`; los campos `String`, también `.con_<campo>_vacio()`.

Los tests generados sacan sus valores válidos de la madre (`construir_props()`, `aleatorio()`),
también la siembra de los repositorios. **Si una regla nueva rechaza un valor por defecto de la
madre, cámbialo en la madre**, no en cada test.

## Añadir una entidad

`puerto generar scaffold <Nombre> [-- campos…]` escribe los cuatro crates a la vez. Una capa cada
vez:

```bash
puerto generar dominio <Nombre>        # dominio_<c> + la madre    → siguiente: aplicacion
puerto generar aplicacion <Nombre>     # aplicacion_<c>            → siguiente: repositorio
puerto generar repositorio <Nombre>    # infra_<c>                 → siguiente: presentacion
puerto generar presentacion <Nombre>   # rest_<c> + el cableado
```
