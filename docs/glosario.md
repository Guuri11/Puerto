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

Decidido el 2026-11-14 para la Fase 19.4. Se aplican las reglas de siempre: el concepto delante, el adaptador
detrás (`OutboxPostgres` como `RepositorioProductoPostgres`), y `from_`, `into_` y `new` en inglés.
La feature `test-utils` pasa a llamarse `pruebas`, y el módulo `mocks`, `dobles`.

| Crate | Hoy | Ahora |
|---|---|---|
| — | `puerto-core`, `puerto-events`, `puerto-mail` | `puerto-nucleo`, `puerto-eventos`, `puerto-correo` |
| — | `puerto-web`, `puerto-sqlx`, `puerto-kafka`, `puerto-auth`, `puerto-macros` | igual |
| nucleo | `RepositoryError::{Duplicate, InvalidData, Unavailable, MissingReference}` | `ErrorRepositorio::{Duplicado, DatosInvalidos, NoDisponible, ReferenciaInexistente}` |
| nucleo | `PageRequest`, `Page { items, total, limit, offset }`, `limit()`, `offset()`, `map` | `PeticionPagina`, `Pagina { elementos, total, limite, desplazamiento }`, `limite()`, `desplazamiento()`, `map` |
| nucleo | `DEFAULT_PAGE_LIMIT`, `MAX_PAGE_LIMIT` | `LIMITE_PAGINA_POR_DEFECTO`, `LIMITE_PAGINA_MAXIMO` |
| nucleo | `DomainEvent`, `EventEnvelope`, `EnvelopeParts`, `DecodeError`, `decode`, `for_event`, `from_parts` | `EventoDominio`, `SobreEvento`, `PartesSobre`, `ErrorDecodificacion`, `decodificar`, `para_evento`, `from_partes` |
| nucleo | `EventHandler`, `HandlerOutcome::{Ack, Retry, DeadLetter}` | `ManejadorEvento`, `ResultadoManejo::{Ack, Reintentar, Descartar}` |
| nucleo | `Delivery`, `decide`, `backoff_for`, `DEFAULT_MAX_DELIVERIES` | `Entrega`, `decidir`, `espera_para`, `ENTREGAS_MAXIMAS_POR_DEFECTO` |
| nucleo | `DeadLetterRecord`, `DeadLetterSink`, `dead_letter`, `unreadable`, `DEAD_LETTER_PAYLOAD_LIMIT` | `RegistroDescartado`, `DestinoDescartados`, `descartar`, `ilegible`, `LIMITE_PAYLOAD_DESCARTADO` |
| nucleo | `REASON_MAX_DELIVERIES_EXCEEDED`, `REASON_PAYLOAD_INVALID`, `REASON_PERMANENT_ERROR` | `MOTIVO_ENTREGAS_MAXIMAS_SUPERADAS`, `MOTIVO_PAYLOAD_INVALIDO`, `MOTIVO_ERROR_PERMANENTE` |
| nucleo | `TransportError`, `Outbox`, `EventPublisher`, `EventSubscription`, `Claimed` | `ErrorTransporte`, `Outbox`, `PublicadorEventos`, `SuscripcionEventos`, `Reservado` |
| nucleo | `Email`, `Mailer`, `SentMails`, `MailError`, `with_text`, `with_header` | `Correo`, `EnviadorCorreo`, `CorreosEnviados`, `ErrorCorreo`, `con_texto`, `con_cabecera` |
| nucleo | `Principal`, `with_user_id`, `with_email`, `with_roles`, `has_role` | `Principal`, `con_id_usuario`, `con_email`, `con_roles`, `tiene_rol` |
| nucleo | `Authenticator`, `AuthError`, `PasswordHasher`, `HashError` | `Autenticador`, `ErrorAuth`, `HasherContrasena`, `ErrorHash` |
| nucleo | `CredentialStore`, `SessionStore`, `Session` | `AlmacenCredenciales`, `AlmacenSesiones`, `Sesion` |
| web | `App`, `Config`, `ConfigError`, `Error`, `from_env`, `from_vars` | `App`, `Config`, `ErrorConfig`, `Error`, `from_env`, `from_variables` |
| web | `port`, `with_port`, `DEFAULT_PORT`, `banner`, `with_banner`, `cors_allowed_origins` | `puerto`, `con_puerto`, `PUERTO_POR_DEFECTO`, `banner`, `con_banner`, `origenes_cors_permitidos` |
| web | `serve`, `try_serve`, `mount`, `routes`, `shutdown_signal`, `SHUTDOWN_DEADLINE`, `init_tracing`, `load_dotenv` | `servir`, `intentar_servir`, `montar`, `rutas`, `senal_apagado`, `PLAZO_APAGADO`, `iniciar_tracing`, `cargar_dotenv` |
| web | `ErrorResponse { name, message }`, `from_code`, `IntoErrorResponse` | `RespuestaError { codigo, mensaje }`, `from_codigo`, `IntoRespuestaError` |
| web | `BearerAuth`, `bearer_token`, `forbidden()`, `rejection` | `BearerAuth`, `bearer_token`, `prohibido()`, `rechazo` |
| web | códigos `auth.forbidden`, `auth.unauthenticated`, `auth.unavailable`, `auth.not_configured` | `auth.prohibido`, `auth.no_autenticado`, `auth.no_disponible`, `auth.no_configurado` |
| web | `ReadinessProbe`, `probe`, `always_ready` | `SondaDisponibilidad`, `sondear`, `siempre_disponible` |
| sqlx | `create_postgres_pool`, `run_migrations`, `ping`, `DatabaseError` | `crear_pool_postgres`, `ejecutar_migraciones`, `ping`, `ErrorBaseDatos` |
| sqlx | `map_sqlx_error`, `map_sqlx_error_with`, `ForeignKey { constraint, field }` | `mapear_error_sqlx`, `mapear_error_sqlx_con`, `ClaveForanea { restriccion, campo }` |
| sqlx | `PgTransaction`, `begin`, `commit`, `tx`, `record`, `record_envelope` | `TransaccionPostgres`, `begin`, `commit`, `tx`, `registrar`, `registrar_sobre` |
| sqlx | `PgOutbox`, `PgDeliveries`, `PgSubscription`, `PgDeadLetters`, `consumers_for`, `route` | `OutboxPostgres`, `EntregasPostgres`, `SuscripcionPostgres`, `DescartadosPostgres`, `consumidores_de`, `enrutar` |
| sqlx | `DEFAULT_CLAIM_LEASE`, `with_lease`, `RELAY_LOCK_KEY`, `with_lock_key` | `RESERVA_POR_DEFECTO`, `con_reserva`, `CLAVE_BLOQUEO_RELAY`, `con_clave_bloqueo` |
| sqlx | `PgSentMails`, `PgSessions`, `PgCredentials` | `CorreosEnviadosPostgres`, `SesionesPostgres`, `CredencialesPostgres` |
| sqlx | `generate_token`, `token_hash`, `session_ttl_from_env`, `parse_ttl`, `DEFAULT_SESSION_TTL` | `generar_token`, `hash_token`, `ttl_sesion_from_env`, `parsear_ttl`, `TTL_SESION_POR_DEFECTO` |
| sqlx | `OUTBOX_MIGRATION`, `USERS_MIGRATION`, `AUTH_MIGRATION`, `SENT_MAILS_MIGRATION` | `MIGRACION_OUTBOX`, `MIGRACION_USUARIOS`, `MIGRACION_AUTH`, `MIGRACION_CORREOS_ENVIADOS` |
| sqlx | tablas `event_deliveries`, `dead_letters`, `sent_mails`, `users`, `sessions`, `credentials` | `entregas_eventos`, `descartados`, `correos_enviados`, `usuarios`, `sesiones`, `credenciales`; `outbox` igual; las columnas y el JSON del sobre también se traducen |
| eventos | `Relay`, `relay_once`, `Consumer`, `consume_once`, `Worker`, `WorkerConfig`, `Loops`, `Heartbeat`, `Progress` | `Relay`, `relay_una_vez`, `Consumidor`, `consumir_una_vez`, `Worker`, `ConfigWorker`, `Bucles`, `Latido`, `Progreso` |
| eventos | `run`, `try_run`, `with_batch_size`, `with_max_deliveries`, `with_max_retry_delay`, `STALL_GRACE` | `ejecutar`, `intentar_ejecutar`, `con_tamano_lote`, `con_entregas_maximas`, `con_espera_maxima_reintento`, `GRACIA_ATASCO` |
| eventos | `DEFAULT_WORKER_PORT`, `DEFAULT_POLL_INTERVAL_MS`, `DEFAULT_RELAY_BATCH_SIZE`, `DEFAULT_CONSUMER_BATCH_SIZE` | `PUERTO_WORKER_POR_DEFECTO`, `INTERVALO_SONDEO_MS_POR_DEFECTO`, `TAMANO_LOTE_RELAY_POR_DEFECTO`, `TAMANO_LOTE_CONSUMIDOR_POR_DEFECTO` |
| eventos | dobles `FakeOutbox`, `FakePublisher`, `FakeSink`, `FakeSubscription` | `OutboxFalso`, `PublicadorFalso`, `DestinoFalso`, `SuscripcionFalsa` |
| kafka | `KafkaConfig`, `KafkaPublisher`, `KafkaSubscription`, `ensure_topics`, `topic`, `group`, `DEFAULT_BROKERS` | `ConfigKafka`, `PublicadorKafka`, `SuscripcionKafka`, `asegurar_topics`, `topic`, `grupo`, `BROKERS_POR_DEFECTO` |
| correo | `MailConfig`, `MailConfigError`, `SmtpMailer`, `Templates`, `render`, `send_once` | `ConfigCorreo`, `ErrorConfigCorreo`, `EnviadorCorreoSmtp`, `Plantillas`, `renderizar`, `enviar_una_vez` |
| correo | dobles `RecordingMailer`, `InMemorySentMails` | `EnviadorCorreoGrabador`, `CorreosEnviadosEnMemoria` |
| auth | `OidcConfig`, `OidcConfigError`, `OidcError`, `OidcAuthenticator`, `discover`, `LEEWAY`, `REFETCH_INTERVAL` | `ConfigOidc`, `ErrorConfigOidc`, `ErrorOidc`, `AutenticadorOidc`, `descubrir`, `MARGEN`, `INTERVALO_RECARGA` |
| auth | `RolesClaim`, `with_roles_claim`, `Argon2Hasher`, `hash_password`, `verify_password` | `ClaimRoles`, `con_claim_roles`, `HasherArgon2`, `hash_contrasena`, `verificar_contrasena` |
| macros | `#[require_role]` | `#[requiere_rol]` |
| todas | variables de entorno (`SMTP_URL`, `WORKER_PORT`, `KAFKA_BROKERS`, `AUTH_ISSUER`…) | igual |

Los correos generados (`generar correo bienvenida`): `CorreoBienvenida`,
`EnviadorCorreoBienvenida`, `EnviadorCorreoBienvenidaPlantilla` y el consumidor `EnviarCorreoBienvenida`.

## Pendiente

Se decide al empezar la fase que lo usa:

- auth: `GET /me` y `/auth/{register,login,logout}` (`login` y `logout` son jerga);
- `--on` de `generar consumidor` y de `generar correo`, y la clave `on` de `[[correo]]`;
- el `Makefile` generado: `test`, `lint`, `format`, `run`, `check`, `audit` y `setup` son jerga;
  `test/domain`, `test/application`, `test/infrastructure`, `generate/*`, `events/*` y `mail/*` se
  traducen.
