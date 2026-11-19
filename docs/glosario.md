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
| `WelcomeMail`, `WelcomeMailer`, `TemplateWelcomeMailer`, `SendWelcomeMail` | `CorreoBienvenida`, `EnviadorCorreoBienvenida`, `EnviadorCorreoBienvenidaPlantilla`, `EnviarCorreoBienvenida` |

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
| `Created`, `Updated`, `Deleted`; `product_created`; `PRODUCT_CREATED` | `Creado`, `Actualizado`, `Borrado`; `producto_creado`; `PRODUCTO_CREADO` |
| `created_at`, `updated_at`, `deleted`, `deleted_at` (`createdAt`…) | `creado_en`, `actualizado_en`, `borrado`, `borrado_en` (`creadoEn`…); `id` igual |
| tabla `products`, ruta `/products`, `?limit=&offset=` | tabla `productos`, ruta `/productos`, `?limite=&desplazamiento=` |
| `Page { items, total, limit, offset }` (JSON) | `{ elementos, total, limite, desplazamiento }` |

**Plural:** se flexiona la primera palabra, el núcleo de un nombre castellano (`linea_pedido` →
`lineas_pedido`). Si termina en vocal, se añade `s` (`productos`); si termina en `z`, `ces`
(`lapices`); si termina en otra consonante, incluida la `y`, `es` (`ciudades`, `leyes`, `paises`).
Hay una tabla de invariables, con tests: `analisis`, `crisis`, `tesis`, `lunes`…

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

Desde la Fase 24 (ADR-0026) un caso de uso no escribe esas líneas: su `ejecutar` lleva
`#[tracing::instrument(name = "crear_producto", …)]`, y el span se llama como el caso de uso.

## Lo que la Fase 19.7 decidió por el camino

| Hoy | Ahora |
|---|---|
| `shared.value_object.<vo>.invalid` (objetos valor compartidos) | `nucleo.<vo>_invalido` |
| `from_entity` (el snapshot de un evento) | `from_entidad` |
| variantes de respuesta `Ok`, `Created`, `NoContent`, `UnprocessableEntity`, `Unauthorized`, `Forbidden`, `NotFound`, `Conflict`, `Unavailable`, `InternalError` | `Ok`, `Creado`, `SinContenido`, `NoProcesable`, `NoAutorizado`, `Prohibido`, `NoEncontrado`, `Conflicto`, `NoDisponible`, `ErrorInterno` |
| handlers `create`, `get_by_id`, `list`, `update`, `delete` | `crear`, `obtener`, `listar`, `actualizar`, `borrar` |
| `RecordingUnitOfWorkFactory`, `Recorded { saved, events, committed }`, `recorded()` | `UnitOfWorkFactoryGrabadora`, `Grabado { guardados, eventos, confirmado }`, `grabado()` |
| `FailAt::{Begin, Save, Commit}`, `failing_at` | `FallarEn::{Begin, Guardar, Commit}`, `fallando_en` |
| `build_app`, `build_worker`, `openapi_service`, `configure(routes)` (`app.rs`) | `construir_app`, `construir_worker`, `servicio_openapi`, `configurar(rutas)` |
| locales y campos: `result`, `response`, `entity`, `repository`, `mock_repo`, `use_case`, `page`, `filter`, `row`, `body`, `client` | `resultado`, `respuesta`, `entidad`, `repositorio`, `repositorio_doble`, `caso_de_uso`, `pagina`, `filtro`, `fila`, `cuerpo`, `cliente` |
| `make db-up`, `db-down`, `reset-db`, `generate/entity`, `generate/use-case`, `generate/migration` | `make db/arrancar`, `db/parar`, `db/reiniciar`, `generar/entidad`, `generar/caso-de-uso`, `generar/migracion` |
| variables de make `NAME`, `ENTITY`, `ACTION`, `ROLE` | `NOMBRE`, `ENTIDAD`, `ACCION`, `ROL` (`EMAIL` igual) |
| el campo por defecto de una entidad o un correo sin campos, `name: String` | `nombre: String` |
| las APIs en `bootstrap.rs`, `product_api`, `me_api`, `auth_api` | `api_producto`, `api_me`, `api_auth` |
| migraciones `create_<e>_table`, `create_users_table`, `create_auth_tables` | `crear_tabla_<e>`, `crear_tabla_usuarios`, `crear_tablas_auth` |
| auth: `UserId`, `UserError`, `ProvisioningAuthenticator`, `RegisterUserParams`, `LoginParams` | `IdUsuario`, `ErrorUsuario`, `AutenticadorAprovisionador`, `ParamsRegistrarUsuario`, `ParamsLogin` |
| auth: `RegisterRequest`, `LoginRequest`, `SessionDto`, `MeDto`, `MeApi`, `AuthApi`, `InvalidCredentials` | `PeticionRegistro`, `PeticionLogin`, `DtoSesion`, `DtoMe`, `ApiMe`, `ApiAuth`, `CredencialesInvalidas` |
| auth, JSON del API: `password`, `name`, `expiresAt` | `contrasena`, `nombre`, `expiraEn` |
| auth, códigos: `user.*`, `auth.invalid_credentials`, `password_too_short`, `email_invalid` | `usuario.*`, `auth.credenciales_invalidas`, `contrasena_demasiado_corta`, `email_invalido` |
| correo: el campo `to`, el consumidor `mail_<nombre>`, su constante `MAIL`, `Context::of`, `TEMPLATES` | `para`, `correo_<nombre>`, `CORREO`, `Contexto::de`, `PLANTILLAS` |
| los mensajes de arranque del proyecto generado (`DATABASE_URL is not set…`) | en castellano (`DATABASE_URL no está definida…`) |

## `puerto.toml`

| Hoy | Ahora |
|---|---|
| `[project] name web db events auth` | `[proyecto] nombre web db eventos auth` |
| `[[entity]] name use_cases db events public roles` | `[[entidad]] nombre casos_de_uso db eventos publicos roles` |
| `[[entity.fields]] name type unique value_object value_object_kind enum_variants references` | `[[entidad.campos]] nombre tipo unico objeto_valor clase_objeto_valor variantes referencia` |
| `[[value_object]] name type` | `[[objeto_valor]] nombre tipo` |
| `[[consumer]] name events` | `[[consumidor]] nombre eventos` |
| `[[mail]] name on` | `[[correo]] nombre tras` |
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
| `--on` | `--tras` |
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

Lo que la Fase 19.4 decidió por el camino, con las mismas reglas:

| Crate | Hoy | Ahora |
|---|---|---|
| nucleo | campos y métodos: `event_type`, `aggregate_id`, `occurred_at`, `headers`, `decode`, `handle`, `send`, `authenticate`, `user_id`, `subject` (de `Principal`), `subject`/`to`/`text` (de `Correo`) | `tipo_evento`, `id_agregado`, `ocurrido_en`, `cabeceras`, `decodificar`, `manejar`, `enviar`, `autenticar`, `id_usuario`, `sujeto`, `asunto`/`para`/`texto` |
| nucleo | `Claimed { envelope, attempt }`, `settle`, `next_batch`, `try_lead`, `fetch_unpublished`, `mark_published` | `Reservado { sobre, intento }`, `resolver`, `siguiente_lote`, `intentar_liderar`, `buscar_sin_publicar`, `marcar_publicados` |
| sqlx | `DatabaseError::{ConnectionError, MigrationError} { target, cause }` | `ErrorBaseDatos::{ErrorConexion, ErrorMigracion} { destino, causa }` |
| sqlx | `outbox` (`seq`, `published_at`…), `event_deliveries` (`status` `pending` \| `done` \| `dead`, `next_attempt_at`, `last_error`), `dead_letters` (`reason`, `detail`, `recorded_at`) | `outbox` (`secuencia`, `publicado_en`…), `entregas_eventos` (`estado` `pendiente` \| `hecho` \| `descartado`, `siguiente_intento_en`, `ultimo_error`), `descartados` (`motivo`, `detalle`, `registrado_en`) |
| sqlx | `users` (`issuer`, `subject`, `name`, `created_at`, `updated_at`), `credentials` (`user_id`, `password_hash`), `sessions` (`token_hash`, `expires_at`) | `usuarios` (`emisor`, `sujeto`, `nombre`, `creado_en`, `actualizado_en`), `credenciales` (`id_usuario`, `hash_contrasena`), `sesiones` (`hash_token`, `expira_en`) |
| eventos | `Progress`, `Pass`, `Heartbeat`, `drive`, `Scripted`, `Fake*` (`with`, `marked`, `published`…) | `Progreso`, `Pasada`, `Latido`, `conducir`, `Guion`, `*Falso` (`con`, `marcados`, `publicados`…) |
| kafka | cabeceras de cada registro `event_id`, `event_type`, `aggregate_type`, `aggregate_id`, `schema_version`, `occurred_at` | `id_evento`, `tipo_evento`, `tipo_agregado`, `id_agregado`, `version_esquema`, `ocurrido_en` |
| correo | `Templates { subject, html, text }`, `render`, `MailConfig { smtp_url, from }`, `failing_with`, `sent` | `Plantillas { asunto, html, texto }`, `renderizar`, `ConfigCorreo { url_smtp, remitente }`, `fallando_con`, `enviados` |
| auth | `OidcConfig { issuer, audience, roles_claim }`, `RolesClaim::{DEFAULT, read}`, `LEEWAY`, `REFETCH_INTERVAL` | `ConfigOidc { emisor, audiencia, claim_roles }`, `ClaimRoles::{POR_DEFECTO, leer}`, `MARGEN`, `INTERVALO_RECARGA` |
| todas | los mensajes para personas (errores de arranque y de configuración, logs) | en castellano; los códigos (`auth.no_autenticado`…) siguen siendo códigos |

Los correos generados (`generar correo bienvenida`): `CorreoBienvenida`,
`EnviadorCorreoBienvenida`, `EnviadorCorreoBienvenidaPlantilla` y el consumidor `EnviarCorreoBienvenida`.

## Auth y Makefile

| Hoy | Ahora |
|---|---|
| `GET /me`, `POST /auth/register`, `/auth/login`, `/auth/logout` | `GET /me`, `POST /auth/registro`, `/auth/login`, `/auth/logout` (`me`, `login` y `logout` son jerga) |
| `make test/domain`, `test/application`, `test/infrastructure` | `make test/dominio`, `test/aplicacion`, `test/infraestructura` |
| `make generate/*`, `events/dead-letters`, `mail/ui`, `auth/grant` | `make generar/*`, `eventos/descartados`, `correo/ui`, `auth/conceder` |
| `make test`, `lint`, `format`, `run`, `check`, `audit`, `setup`, `docker-compose/*`, `sqlx/*`, `web/*`, `auth/token`, `auth/ui` | igual (jerga y nombres de herramientas) |

## Fase 20 (2026-11-17)

Lo que añadió la Fase 20 (lo que encontró canasta); no hay un nombre anterior.

| Dónde | Nombre |
|---|---|
| modelo | `Producto::actualizar(&mut self, props)` (las mismas reglas que `new`), `Producto::validar(props)` (privada: la llaman `new` y `actualizar`) |
| consumidor del worker | `AvisarProductoNuevo::construir(pool: &PgPool) -> Self` (lo llama `registro.rs`) |
| `puerto.toml` | `[[entidad]] orden unicos`, `[[entidad.campos]] privado` |
| CLI | `--orden`, `--unico`, `--privado` |
| runtime (`puerto-web`) | `BearerAuthOpcional(pub Option<Principal>)` |
| DTO | `DtoProducto::from_dominio_publico(&entidad)` (los campos privados a `None`) |
| SQL | el índice de un único compuesto: `<tabla>_<campo>_<campo>_key` |

## Fase 24 (decidido el 2026-11-18)

Los casos de uso de escritura (ADR-0026). No hay un nombre anterior.

| Dónde | Nombre |
|---|---|
| modelo (proyectos con eventos) | `Producto::crear(props) -> Result<(Producto, EventoProducto), ErrorProducto>` (`new` + el evento `Creado`) |
| modelo | `Producto::borrar(&mut self)` (el borrado lógico; con eventos devuelve `EventoProducto`); con eventos, `actualizar` devuelve `EventoProducto` |
| unidad de trabajo | `UnitOfWorkFactoryProducto::guardar(&self, entidad, evento)` (`begin` + `guardar` + `registrar` + `commit`) |

## Fase 22 — el frontend (decidido el 2026-11-17)

`web/` pasa a castellano: estructura, nombres y textos. Mismas reglas que el backend (el concepto
delante, sin tildes ni ñ en los identificadores, la jerga igual); en inglés también lo que imponen
React, Vite, Bun y las librerías (el prefijo `use` de los hooks, `main.tsx`, `index.css`, `*.test.tsx`,
los scripts de `package.json`, `happy-dom.ts`, `testing-library.ts`). Los textos que ve el usuario y
los nombres de los tests son frases: con tildes.

### Carpetas y ficheros

| Hoy | Ahora |
|---|---|
| `api/`, `auth/` | igual (jerga) |
| `features/<e>/` | `contextos/<e>/` (el mismo nombre que en el backend) |
| `containers/`, `views/` | `contenedores/`, `vistas/` |
| `pages/`, `shared/`, `shared/ui/`, `generated/`, `test-support/` | `paginas/`, `compartido/`, `compartido/ui/`, `generado/`, `pruebas/` |
| `routes.tsx`, `dependencies.tsx` | `rutas.tsx`, `dependencias.tsx` |
| `api/client.ts`, `api/schema.d.ts`, `api/http.ts` | `api/cliente.ts`, `api/esquema.d.ts`, `api/http.ts` |
| `containers/producto-list-container.tsx` (`-detail-`, `-form-`) | `contenedores/lista-producto.tsx` (`detalle-`, `formulario-`) |
| `views/producto-list-view.tsx` | `vistas/lista-producto.tsx` |
| `messages.ts`, `shared/forms.ts`, `shared/use-async.ts` | `mensajes.ts`, `compartido/formularios.ts`, `compartido/use-carga.ts` |
| `test-support/producto.ts`, `fake-api.ts`, `render.tsx`, `auth.ts` | `pruebas/producto.ts`, `api-falsa.ts`, `renderizar.tsx`, `auth.ts` |
| `shared/ui/error-alert.tsx`, `field-error`, `confirm-dialog`, `empty-state`, `layout`, `pagination`, `reference-link`, `user-menu` | `compartido/ui/alerta-error.tsx`, `error-campo`, `dialogo-confirmar`, `sin-elementos`, `marco`, `paginacion`, `enlace-referencia`, `menu-usuario` |
| `pages/home.tsx`, `not-found`, `login`, `register`, `auth-callback` | `paginas/inicio.tsx`, `no-encontrada`, `login`, `registro`, `auth-callback` |
| `auth/session.ts`, `accounts`, `messages`, `auth.tsx`, `oidc-auth-gateway`, `local-auth-gateway`, `bearer-middleware` | `auth/sesion.ts`, `cuentas`, `mensajes`, `auth.tsx`, `pasarela-oidc`, `pasarela-local`, `middleware-bearer` |

### Nombres

| Hoy | Ahora |
|---|---|
| `ProductoListContainer`, `ProductoDetailContainer`, `ProductoFormContainer`, `ProductoForm` | `ContenedorListaProducto`, `ContenedorDetalleProducto`, `ContenedorFormularioProducto`, `FormularioProducto` |
| `ProductoListView`, `ProductoDetailView`, `ProductoFormView` | `VistaListaProducto`, `VistaDetalleProducto`, `VistaFormularioProducto` |
| `ProductoFormValues`, `ProductoFormOptions`, `toProductoForm`, `toProductoRequest` | `ValoresFormularioProducto`, `OpcionesFormularioProducto`, `aFormularioProducto`, `aPeticionProducto` |
| `ProductoProblem`, `toProblem`, `TEXTS`, `FIELDS` | `ProblemaProducto`, `aProblema`, `TEXTOS`, `CAMPOS` |
| `clienteLabel` (el detalle de una referencia) | `etiquetaCliente` |
| `ApiClient`, `createApiClient`, `Schemas` | `ClienteApi`, `crearClienteApi`, `Esquemas` |
| `ApiError`, `apiErrorCode`, `unwrap`, `PAGE_SIZE`, `MAX_PAGE_LIMIT` | `ErrorApi`, `codigoErrorApi`, `desenvolver`, `TAMANO_PAGINA`, `LIMITE_PAGINA_MAXIMO` |
| `Dependencies`, `createDependencies`, `DependenciesProvider`, `useDependencies` | `Dependencias`, `crearDependencias`, `ProveedorDependencias`, `useDependencias` |
| `AppRoutes`, `customRoutes`, `navEntries`, `NavEntry` | `RutasApp`, `rutasPropias`, `entradasMenu`, `EntradaMenu` |
| `useAsync`, `AsyncState { status: "loading" \| "error" \| "ok", data, error }` | `useCarga`, `EstadoCarga { estado: "cargando" \| "error" \| "listo", datos, error }` |
| `ErrorAlert`, `errorCode`, `FieldError`, `ConfirmDialog`, `EmptyState`, `Layout`, `Pagination`, `ReferenceLink` | `AlertaError`, `codigoError`, `ErrorCampo`, `DialogoConfirmar`, `SinElementos`, `Marco`, `Paginacion`, `EnlaceReferencia` |
| `offsetFromSearch`, `pageSearch`, `HomePage`, `NotFoundPage` | `desplazamientoDeUrl`, `busquedaPagina`, `PaginaInicio`, `PaginaNoEncontrada` |
| `textOf`, `optionalText`, `optionalNumber`, `optionalBoolean`, `optionalDateTime` | `textoDe`, `textoOpcional`, `numeroOpcional`, `booleanoOpcional`, `fechaHoraOpcional` |
| `splitList`, `splitNumbers`, `formatEntries`, `parseEntries` | `separarLista`, `separarNumeros`, `formatearEntradas`, `leerEntradas` |
| `toDateTimeLocal`, `fromDateTimeLocal`, `formatDateTime`, `ReferenceOption`, `referenceOptions` | `aFechaHoraLocal`, `deFechaHoraLocal`, `formatearFechaHora`, `OpcionReferencia`, `opcionesReferencia` |
| `ProductoMother.request()`, `.random()`, `productoCollection` | `MadreProducto.peticion()`, `.aleatorio()`, `coleccionProducto` |
| `fakeApi`, `collection`, `Collection { path, notFound, items }` | `apiFalsa`, `coleccion`, `Coleccion { ruta, noEncontrado, elementos }` |
| `renderScreen`, `referencedBy`, `fillIn`, `choose`, `type` | `renderizarPantalla`, `referenciadosPor`, `rellenar`, `elegir`, `escribir` |
| el sufijo de una variable que choca con una palabra reservada o un local (`caseItem`, `seriesList`) | `caseElemento`, `seriesLista` |
| locales: `seeded`, `stored`, `failures`, `request`, `values`, `saving`, `failure`, `problem`, `existing`, `navigate`, `location`, `goTo`, `offset`, `query`, `confirming`, `remove`, `save`, `saved`, `found`, `target`, `owner`, `options`, `history`, `user`, `code`, `created`, `input` | `sembrado`, `guardados`, `fallos`, `peticion`, `valores`, `guardando`, `fallo`, `problema`, `existente`, `navegar`, `ubicacion`, `irA`, `desplazamiento`, `consulta`, `confirmando`, `borrar`, `guardar`, `guardado`, `encontrado`, `objetivo`, `dueno`, `opciones`, `historial`, `usuario`, `codigo`, `creado`, `campo` |
| auth: `AuthGateway`, `CurrentUser`, `Accounts`, `Registration`, `AuthError`, `AuthProvider`, `RequireAuth`, `useAuth` | `PasarelaAuth`, `UsuarioActual`, `Cuentas`, `Registro`, `ErrorAuth`, `ProveedorAuth`, `RequiereAuth`, `useAuth` |
| auth: `currentUser`, `accessToken`, `login`, `completeLogin`, `logout`, `refresh`, `can(role)`, `register`, `signIn` | `usuarioActual`, `tokenAcceso`, `login`, `completarLogin`, `logout`, `refrescar`, `puede(rol)`, `registrar`, `iniciarSesion` |
| auth: `oidcAuthGateway`, `localAuthGateway`, `bearerMiddleware`, `OidcSettings`, `oidcSettingsFromEnv`, `TOKEN_KEY` | `pasarelaOidc`, `pasarelaLocal`, `middlewareBearer`, `AjustesOidc`, `ajustesOidcDeEntorno`, `CLAVE_TOKEN` |
| auth: `CurrentUserMother`, `fakeAuthGateway`, `fakeAccounts`, `UserMenu`, `LoginPage`, `RegisterPage`, `AuthCallbackPage`, `nextPath`, `AccountProblem`, `toAccountProblem` | `MadreUsuarioActual`, `pasarelaAuthFalsa`, `cuentasFalsas`, `MenuUsuario`, `PaginaLogin`, `PaginaRegistro`, `PaginaAuthCallback`, `rutaSiguiente`, `ProblemaCuenta`, `aProblemaCuenta` |
| reglas de dependency-cruiser | `vistas-solo-props`, `solo-generado-construye-el-cliente`, `solo-los-tests-importan-pruebas`, `sin-ciclos`, `solo-la-pasarela-importa-oidc-client-ts` |
| decididos por el camino: la primera página de las entidades referenciadas, los cambios de una madre, la carga de la etiqueta de un objetivo | `PRIMERA_PAGINA`, `cambios`, `etiquetaCliente` (`objetivo`, `encontrado`) |

### Rutas y textos

| Hoy | Ahora |
|---|---|
| `/productos/new`, `/productos/:id/edit`, `?page=2` | `/productos/nuevo`, `/productos/:id/editar`, `?pagina=2` |
| `/register`, `/login?next=`, `/login`, `/auth/callback` | `/registro`, `/login?siguiente=`; `/login` y `/auth/callback` igual (el callback lo registra Keycloak) |
| `New producto`, `Edit producto`, `Delete this producto?` | `Crear producto`, `Editar producto`, `¿Borrar producto?` (frases neutras: sin artículo ni adjetivo que concuerde, porque el género de la entidad no se sabe) |
| `View`, `Edit`, `Delete`, `Save`, `Cancel`, `Yes`, `No`, `Previous`, `Next`, `1–20 of 45` | `Ver`, `Editar`, `Borrar`, `Guardar`, `Cancelar`, `Sí`, `No`, `Anterior`, `Siguiente`, `1–20 de 45` |
| `Back to productos`, `No productos yet.`, `Choose a cliente` / `Choose…`, `None` | `Volver a productos`, `Todavía no hay productos.`, `Elige una opción`, `—` |
| `Home`, `Page not found`, `key=value, one per line` | `Inicio`, `Página no encontrada`, `clave=valor, una por línea` |
| mensajes: `Title is required`, `Sku is not valid`, `Cliente does not exist` | `Título: obligatorio`, `Sku: no válido`, `Cliente: no existe` (con dos puntos: concuerdan con cualquier campo) |
| mensajes: `A producto with these values already exists`, `This producto no longer exists`, `… could not be saved, try again later`, `Something went wrong` | `Estos valores ya están en uso`, `Ya no existe`, `No se ha podido guardar; inténtalo más tarde`, `Algo ha ido mal` |
| auth: `Sign in`, `Sign out`, `Create account`, `Create an account`, `Email`, `Password`, `Name`, `Wrong email or password`, `An account with this email already exists`, `This is not an email address` | `Iniciar sesión`, `Cerrar sesión`, `Crear cuenta`, `Crea una cuenta`, `Email`, `Contraseña`, `Nombre`, `Email o contraseña incorrectos`, `Ya hay una cuenta con este email`, `No es una dirección de email` |
| tests: `should show … when …`, `// Arrange`, `// Act`, `// Assert` | `debería mostrar … cuando …`, `// Preparar`, `// Actuar`, `// Comprobar` |

## Fase 25 — la marca y las pantallas base (decidido el 2026-11-18)

No hay un nombre anterior. Sergio aprobó `marca`, `tema`, `panel`, `perfil` y `acceso`; el resto se
decidió por el camino, con las mismas reglas.

| Dónde | Nombre |
|---|---|
| ficheros | `web/src/marca/marca.ts`, `web/src/marca/tema.css`, `web/public/logo.svg`, `compartido/ui/marco-acceso.tsx`, `compartido/ui/avatar.tsx`, `paginas/panel.tsx` (antes `inicio.tsx`), `paginas/perfil.tsx` |
| nombres | `marca { nombre, eslogan, logo }`, `MarcoAcceso`, `Avatar`, `iniciales`, `PaginaPanel` (antes `PaginaInicio`), `TarjetaTotal`, `PaginaPerfil`, `EntradaMenu.total`, `contarTotal`, `PAGINA_DE_UNO` |
| rutas | `/perfil` |
| textos | `Panel`, `Perfil`, `Roles`, `Sin roles`, `Sin nombre`, `Sin email`, `Ver la lista`, `Abrir el menú`, `Cerrar el menú`, el eslogan `Todo lo que gestionas, en un solo sitio.` |

## Fase 26 — los bloques web (decidido el 2026-11-19)

Sergio eligió `web-bloque` para el comando y aprobó tal cual los huecos, las variantes y los textos.
`portada/bienvenida` sale de su «algo básico de bienvenida»; `ruta_app` / `rutaApp`, de «en qué
ruta quieres el SaaS».

| Dónde | Nombre |
|---|---|
| comando | `puerto generar web-bloque <hueco>/<variante>`; en `puerto nuevo`, `--portada`, `--sin-portada`, `--ruta-app` |
| conceptos | un **bloque** es una variante de un **hueco**; el **escaparate** es `/bloques` en la web de Puerto |
| huecos | `acceso`, `marco`, `panel`, `perfil`, `tema`, `portada`, `cabecera`, `caracteristicas`, `precios`, `preguntas`, `llamada` (la llamada a la acción), `pie` |
| variantes | `acceso/{dividido,centrado}`, `marco/{lateral,superior}`, `panel/{tarjetas,resumen}`, `perfil/{tarjeta,banda}`, `tema/{puerto,bosque,ciruela,coral,pizarra,miel}`, `portada/{bienvenida,producto}`, `cabecera/{centrada,dividida}`, `caracteristicas/rejilla`, `precios/planes`, `preguntas/acordeon`, `llamada/banda`, `pie/columnas` |
| ficheros | `paginas/portada.tsx`, `web/src/portada/{cabecera,caracteristicas,precios,preguntas,llamada,pie}.tsx` |
| nombres | `PaginaPortada`, `SeccionCabecera`, `SeccionCaracteristicas`, `SeccionPrecios`, `SeccionPreguntas`, `SeccionLlamada`, `SeccionPie`, `rutaApp`; en `puerto.toml`, `[proyecto] portada`, `ruta_app` |
| textos | `Entrar`, `Ir al panel`, `Características`, `Precios`, `Preguntas frecuentes` |

## Fase 27 — lo que encontró Aforo (2026-11-19)

Los de 27.1 y 27.2 vienen del plan. El resto se decidió por el camino, con las mismas reglas
(Sergio puede cambiarlos); las palabras de «Por decidir» de la Fase 27 se preguntan antes.

| Dónde | Nombre |
|---|---|
| `.env` | `SMTP_HOST_PORT`, `MAILPIT_UI_PORT` |
| madre | `Madre<Entidad>::<campo>_unico()` (un campo `unico`) |
| API falsa | `fetchFalso`, `Coleccion.acciones`, el fallo de una acción `"POST /productos/{id}/<accion>"` |
| referencias en una lista | `etiquetasReferencia(cargados, campo)`, `etiquetas<Objetivo>` (`etiquetasCliente`), `EnlaceReferencia etiquetas` |
| logs | `guardar rechazado` (un duplicado o una referencia inexistente, con `warn!`) |
| decididos por Sergio | `[[entidad.campos]] etiqueta` (27.5) y `textos` (27.9: `{ AlaVenta = "A la venta" }`); el mensaje `<Etiqueta>: campo obligatorio`; `CONFIRM=DESTRUIR make db/reiniciar` (27.22, en inglés como toda variable de entorno); tras crear sin poder ver el detalle ni la lista, el panel con el aviso `Guardado` (27.8) |

## Pendiente

Nada por ahora. Una palabra nueva se pregunta antes de usarla.
