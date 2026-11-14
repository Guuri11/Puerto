# Glosario de lo que genera Puerto

Las reglas, en la [ADR-0024](adr/0024-lo-generado-en-castellano.md): todo en castellano y sin
tildes ni ñ en los identificadores; el concepto delante de la entidad; en inglés solo lo que impone
el lenguaje o una librería, la jerga asentada y las variables de entorno. **Una palabra que no está
aquí se pregunta a Sergio y se añade antes de usarla.**

Los ejemplos usan la entidad `Producto` (`producto`, `productos`) y el caso de uso `crear_producto`.
Lo marcado **pendiente** se decide en su fase, antes de escribir código.

## Estructura

| Hoy | Ahora |
|---|---|
| `business/`, `infrastructure/`, `presentation/` | `contextos/<c>/{dominio,aplicacion,infraestructura,rest}`, `presentacion/` |
| crates `business`, `infrastructure`, `presentation` | `dominio_<c>`, `aplicacion_<c>`, `infra_<c>`, `rest_<c>`, `nucleo`, `migrador`, `cableado`, `api`, `worker` |
| `business/src/domain/shared/` | crate `nucleo` |
| `infrastructure/migrations/`, `MIGRATOR` | `migraciones/`, crate `migrador`, `MIGRADOR` |
| `model.rs`, `errors.rs`, `repository.rs`, `value_objects.rs`, `events.rs` | `modelo.rs`, `errores.rs`, `repositorio.rs`, `valores.rs`, `eventos.rs` |
| `unit_of_work.rs` | `unit_of_work.rs` (jerga) |
| `infrastructure/src/<e>/entity.rs` | `infra_<c>/src/fila.rs` |
| `routes.rs`, `dto.rs`, `responses.rs`, `error_mapper.rs` | `rutas.rs`, `dto.rs`, `respuestas.rs`, `errores.rs` |
| `tests/` (módulo), `mothers/`, mocks, feature `test-utils` | `pruebas/`, `madres/`, `dobles/`, feature `pruebas` |
| `generated/bootstrap.rs` | `generado/bootstrap.rs` |
| `worker/src/generated/registry.rs`, `consumers/` | `worker/src/generado/registro.rs`, `consumidores/` |
| `app.rs` | `app.rs` (jerga) |
| `domain/mail/<n>.rs`, `infrastructure/mail/<n>/` | contexto `correo`: `contextos/correo/{dominio,infraestructura}`; plantillas `asunto.txt`, `cuerpo.html`, `cuerpo.txt` |
| demo `Greeting`, `get_greeting` | `Saludo`, `obtener_saludo` |
| `User` (auth) | contexto `usuario`, entidad `Usuario` |

## Tipos

| Hoy | Ahora |
|---|---|
| `ProductId` | `IdProducto` |
| `ProductProps` | `PropsProducto` |
| `ProductError` | `ErrorProducto` |
| `ProductRepositoryTrait` | `RepositorioProducto` |
| `InMemoryProductRepository`, `PgProductRepository` | `RepositorioProductoEnMemoria`, `RepositorioProductoPostgres` |
| `MockProductRepository` | `MockRepositorioProducto` (el prefijo lo pone mockall) |
| `ProductDb` | `FilaProducto` |
| `CreateProductParams` | `ParamsCrearProducto` |
| `CreateProductUseCaseTrait`, `CreateProductUseCaseImpl` | `CrearProducto`, `CrearProductoImpl` |
| `MockCreateProductUseCase` | `MockCrearProducto` |
| `ProductMother` | `MadreProducto` |
| `ProductEvent`, `ProductSnapshot` | `EventoProducto`, `SnapshotProducto` |
| `ProductUnitOfWork`, `ProductUnitOfWorkFactory`, `PgProductUnitOfWorkFactory` | `UnitOfWorkProducto`, `UnitOfWorkFactoryProducto`, `UnitOfWorkFactoryProductoPostgres` |
| `ProductFilter` | `FiltroProducto` |
| `ProductApi`, `ProductDto`, `ProductPageDto` | `ApiProducto`, `DtoProducto`, `DtoPaginaProducto` |
| `CreateProductRequest`, `CreateProductResponse` | `PeticionCrearProducto`, `RespuestaCrearProducto` |
| `WelcomeMail`, `WelcomeMailer`, `TemplateWelcomeMailer`, `SendWelcomeMail` | **pendiente** (con `Mailer` del runtime) |

## Funciones y métodos

| Hoy | Ahora |
|---|---|
| `new`, `from_uuid`, `as_str`, `fmt`, `from`, `try_from` | igual (convención de Rust) |
| `from_repository`, `from_domain`, `to_domain` | `from_repositorio`, `from_dominio`, `to_dominio` (el prefijo, convención de Rust) |
| `value`, `generate` | `valor`, `generar` |
| `execute` | `ejecutar` |
| `find_by_id`, `find_all`, `save` | `buscar_por_id`, `buscar_todos`, `guardar` |
| `begin`, `commit`, `rollback` | igual (unidad de trabajo) |
| `record` (registrar un evento en la unidad de trabajo) | `registrar` |
| `build`, `build_props`, `random`, `random_vec` | `construir`, `construir_props`, `aleatorio`, `aleatorios` |
| `with_<campo>`, `with_empty_<campo>` | `con_<campo>`, `con_<campo>_vacio` |
| `seed` (tests de Postgres) | `sembrar` |
| `into_error_response` | `a_respuesta_error` (función libre, ADR-0023 § 8) |
| `send` | `enviar` |

## Casos de uso, eventos y campos

| Hoy | Ahora |
|---|---|
| `create_<e>`, `list_<plural>`, `get_<e>`, `update_<e>`, `delete_<e>` | `crear_<e>`, `listar_<plural>`, `obtener_<e>`, `actualizar_<e>`, `borrar_<e>` |
| `Created`, `Updated`, `Deleted`; `product.created`; `PRODUCT_CREATED` | `Creado`, `Actualizado`, `Borrado`; `producto.creado`; `PRODUCTO_CREADO` |
| `created_at`, `updated_at`, `deleted`, `deleted_at` (`createdAt`…) | `creado_en`, `actualizado_en`, `borrado`, `borrado_en` (`creadoEn`…); `id` igual |
| tabla `products`, ruta `/products`, `?limit=&offset=` | tabla `productos`, ruta `/productos`, `?limite=&desplazamiento=` |
| `Page { items, total, limit, offset }` (JSON) | `{ elementos, total, limite, desplazamiento }` |

**Plural:** si termina en vocal, se añade `s` (`productos`); si termina en `z`, `ces` (`lapices`); si
termina en otra consonante, incluida la `y`, `es` (`ciudades`, `leyes`). Hay una tabla de
excepciones, con tests: `pais` → `paises`; invariables como `analisis`, `crisis`, `tesis` y `lunes`.

## Errores

| Hoy | Ahora |
|---|---|
| `ValidationError(String)`, `NotFound`, `Duplicate` | `Validacion(String)`, `NoEncontrado`, `Duplicado` |
| `RepositoryError`, `Unknown`, `Forbidden`, `Unavailable` | `Repositorio`, `Desconocido`, `Prohibido`, `NoDisponible` |
| `Invalid<Vo>` | `<Vo>Invalido` |
| `product.validation_error.name_empty` | `producto.validacion.nombre_vacio` |
| `product.validation_error.customer_id_not_found` | `producto.validacion.cliente_id_no_encontrado` |
| `product.not_found`, `product.duplicate`, `product.invalid_status` | `producto.no_encontrado`, `producto.duplicado`, `producto.estado_invalido` |
| `ErrorResponse { name, message }` | `RespuestaError { codigo, mensaje }` |

## Tests y logs

| Hoy | Ahora |
|---|---|
| `should_<resultado>_when_<escenario>` | `deberia_<resultado>_cuando_<escenario>` |
| `// Arrange`, `// Act`, `// Assert` | `// Preparar`, `// Actuar`, `// Comprobar` |
| `create_product: start`, `done`, `rejected`, `repository failed` | `crear_producto: inicio`, `fin`, `rechazado`, `fallo del repositorio` |

## `puerto.toml`

| Hoy | Ahora |
|---|---|
| `[project] name web db events auth` | `[proyecto] nombre web db eventos auth` |
| `[[entity]] name use_cases db events public roles` | `[[entidad]] nombre casos_de_uso db eventos publicos roles` |
| `[[entity.fields]] name type unique value_object value_object_kind enum_variants references` | `[[entidad.campos]] nombre tipo unico objeto_valor clase_objeto_valor variantes referencia` |
| `[[value_object]] name type` | `[[objeto_valor]] nombre tipo` |
| `[[consumer]] name events` | `[[consumidor]] nombre eventos` |
| `[[mail]] name on` | `[[correo]] nombre` + **pendiente** (`on`) |
| valores `"enum"`, `"db"`, `"kafka"`, `"oidc"`, `"local"` | igual |

## CLI

| Hoy | Ahora |
|---|---|
| `puerto new`, `validate`, `list` | `puerto nuevo`, `validar`, `listar` |
| `generate scaffold`, `bootstrap`, `snippets`, `web` | `generar scaffold`, `bootstrap`, `snippets`, `web` |
| `generate domain`, `application`, `repository`, `presentation` | `generar dominio`, `aplicacion`, `repositorio`, `presentacion` |
| `generate use-case`, `value-object`, `migration`, `event`, `consumer`, `mail` | `generar caso-de-uso`, `objeto-valor`, `migracion`, `evento`, `consumidor`, `correo` |
| `--name`, `--destination`, `--force`, `--allow-missing` | `--nombre`, `--destino`, `--forzar`, `--permitir-faltantes` |
| `--no-db`, `--no-web`, `--no-demo` | `--sin-db`, `--sin-web`, `--sin-demo` |
| `--events`, `--public`, `--role`, `--to` | `--eventos`, `--publico`, `--rol`, `--para` |
| `--db`, `--auth`, `--ide` | igual |
| `--on` | **pendiente** (`--tras`) |
| sintaxis de campos `opt:`, `vec:`, `ref:`, `enum:`, `map`, `DateTime`, `!` | igual |

## Runtime

| Hoy | Ahora |
|---|---|
| `puerto-core`, `puerto-events`, `puerto-mail` | `puerto-nucleo`, `puerto-eventos`, `puerto-correo` |
| `puerto-web`, `puerto-sqlx`, `puerto-kafka`, `puerto-auth`, `puerto-macros` | igual |
| `RepositoryError::{Duplicate, InvalidData, Unavailable, MissingReference}` | `ErrorRepositorio::{Duplicado, DatosInvalidos, NoDisponible, ReferenciaInexistente}` |
| `PageRequest`, `Page` | `PeticionPagina`, `Pagina` |
| `DomainEvent`, `Authenticator` | `EventoDominio`, `Autenticador` |
| `#[require_role]`, `BearerAuth` | `#[requiere_rol]`, `BearerAuth` |
| tablas `event_deliveries`, `dead_letters`, `sent_mails`, `users`, `sessions`, `credentials` | `entregas_eventos`, `descartados`, `correos_enviados`, `usuarios`, `sesiones`, `credenciales`; `outbox` igual |
| `EventEnvelope`, `EventHandler`, `HandlerOutcome::{Ack, Retry, DeadLetter}`, `Principal`, `PasswordHasher`, `Mailer`, `Email`, `SentMails`, `Relay`, `Consumer` y el resto | **pendiente**: en la fase del runtime, crate a crate |

## Pendiente

Se decide al empezar la fase que lo usa:

- los correos: `Mailer`, `Email`, `SentMails` y los tipos que genera `generar correo`;
- el resto de la API del runtime (tabla de arriba);
- auth: `GET /me` y `/auth/{register,login,logout}` (`login` y `logout` son jerga);
- `--on` de `generar consumidor` y de `generar correo`, y la clave `on` de `[[correo]]`;
- el `Makefile` generado: `test`, `lint`, `format`, `run`, `check`, `audit` y `setup` son jerga;
  `test/domain`, `test/application`, `test/infrastructure`, `generate/*`, `events/*` y `mail/*` se
  traducen.
