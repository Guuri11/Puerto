# Puerto — Instrucciones para agentes

Puerto es una CLI en Rust que genera proyectos full-stack DDD. Usa `cargo-generate` para crear
workspaces con un crate por contexto y capa (ADR-0023), siguiendo Clean Architecture (hexagonal,
puertos y adaptadores). Todo lo que genera va en castellano (ADR-0024, `docs/glosario.md`); el
código fuente de Puerto sigue en inglés.

## Stack

- Rust (edition 2024) · cargo-generate · poem-openapi · tokio
- mockall (tests) · thiserror (errores) · async-trait

## La estructura de Puerto

```
crates/
  nucleo/           — `puerto-nucleo`, crate del runtime (ADR-0017): `ErrorRepositorio`, `PeticionPagina`,
                      `Pagina<T>`; `eventos` (ADR-0018): `EventoDominio`, `SobreEvento`, `ManejadorEvento`,
                      la política de entrega y los puertos de transporte; `correo` (ADR-0020): `Correo`,
                      `EnviadorCorreo`, `CorreosEnviados`; `auth` (ADR-0021): `Principal`, `Autenticador`,
                      `ErrorAuth`, `auth::dobles::{MockAutenticador, MadrePrincipal}` (feature `pruebas`).
                      Los proyectos generados dependen de él en la minor de la CLI; misma versión de
                      workspace que `cli/`. El compile matrix compila contra este checkout
                      (`[patch.crates-io]`, que solo añade el arnés de tests)
  macros/           — `puerto-macros` (proc-macro, ADR-0021 § 4): `#[requiere_rol("<rol>")]`, usado como
                      `puerto_web::requiere_rol` — un 403 al principio de un handler que recibe
                      `auth: BearerAuth`; si se usa mal, emite el método sin tocar junto al error
  web/              — `puerto-web`, el runtime de presentación: `App` (tracing, CORS, apagado ordenado),
                      `Config::from_env`, las sondas de salud, `RespuestaError { codigo, mensaje }`;
                      `auth`: `BearerAuth` (pregunta al `Arc<dyn Autenticador>` que las rutas llevan
                      como data; 401 `auth.no_autenticado`, 503 `auth.no_disponible`), `prohibido()`
  sqlx/             — `puerto-sqlx`, el runtime de Postgres (proyectos `--db`): `crear_pool_postgres`,
                      `ejecutar_migraciones(pool, &MIGRADOR)`, `ping`, `mapear_error_sqlx` (`_con` las
                      `ClaveForanea` de una entidad: `23503` → `ReferenciaInexistente(campo)`, ADR-0022);
                      para proyectos con eventos, `uow::TransaccionPostgres`,
                      `eventos::{OutboxPostgres, EntregasPostgres, SuscripcionPostgres, DescartadosPostgres}`
                      y `migrations/outbox.sql` (el `templates/infrastructure/outbox.sql` de la CLI debe
                      coincidir byte a byte — hay test); para correo (ADR-0020)
                      `correo::CorreosEnviadosPostgres` y `migrations/sent_mails.sql` (misma regla con
                      `templates/infrastructure/sent_mails.sql`); para cuentas locales (ADR-0021)
                      `auth::{SesionesPostgres, CredencialesPostgres}`, el token de sesión (256 bits; la
                      fila guarda su SHA-256), `ttl_sesion_from_env` y `migrations/{users,auth}.sql`
                      (byte a byte con `templates/auth/users.sql` y `templates/auth/local/auth.sql`)
  eventos/          — `puerto-eventos`, el runtime del worker (proyectos `--eventos`, ADR-0018): `Relay`
                      (outbox → `PublicadorEventos`), `Consumidor` (suscripción → `ManejadorEvento`,
                      `decidir`, descarte antes de retirar), `Worker` + `Bucles` (salud en `WORKER_PORT`,
                      drenaje con SIGTERM), `ConfigWorker::from_env`
  kafka/            — `puerto-kafka`, el transporte Kafka (`--eventos kafka`, ADR-0018): `ConfigKafka`
                      (`KAFKA_BROKERS`, `KAFKA_TOPIC_PREFIX`, `asegurar_topics`), `PublicadorKafka`,
                      `SuscripcionKafka` (un registro por lote, reintento en proceso, commit tras el
                      resultado). `make test/kafka` lo prueba contra Redpanda. Compilarlo pide `cmake` y
                      las cabeceras de libcurl (librdkafka 2.12 incluye `curl/curl.h` aunque no use curl)
  correo/           — `puerto-correo`, el correo (ADR-0020): `ConfigCorreo` (`SMTP_URL`, `MAIL_FROM`),
                      `EnviadorCorreoSmtp` (lettre, rustls; 5xx → `Permanente`, el resto → `Transitorio`),
                      `Plantillas` (minijinja, estricto, el asunto en una línea), `enviar_una_vez` (el
                      registro de enviados, luego el envío, luego la marca) y, tras la feature `pruebas`,
                      `EnviadorCorreoGrabador` / `CorreosEnviadosEnMemoria`. Los puertos (`Correo`,
                      `EnviadorCorreo`, `CorreosEnviados`) son de `puerto-nucleo`. `make test/mail` lo
                      prueba contra Mailpit
  auth/             — `puerto-auth`, adaptadores de auth (ADR-0021). Feature `oidc`: `ConfigOidc`
                      (`AUTH_ISSUER`, `AUTH_AUDIENCE`, `AUTH_ROLES_CLAIM`), `ClaimRoles` (un nombre de
                      primer nivel tal cual; si no, una ruta con puntos), `AutenticadorOidc::descubrir`
                      (solo RS256/ES256, `iss`/`aud`/`exp`/`nbf` ± 60 s, un `kid` desconocido vuelve a
                      pedir el JWKS como mucho una vez por minuto; el `id_usuario` del `Principal` es
                      nil). Feature `local`: `HasherArgon2`. Sus tests activan la feature con una
                      dev-dependency sobre sí mismo. `make test/auth` lo prueba contra Keycloak
  cli/              — El binario `puerto`
    src/main.rs     — Definición de la CLI y despacho
    src/commands/   — Handlers de comandos (new, list, validate)
    src/generators/ — Generadores por capa + bootstrap (`cableado` y `bootstrap.rs`), crates (el
                      `Cargo.toml` y el `lib.rs` plano de cada crate de un contexto), merge (contrato e
                      implementación de un caso de uso en un fichero), conflict (guarda de
                      sobrescritura), consistency (deriva, incluida la deriva textual de auth en
                      `rutas.rs`), auth (`nuevo --auth`: el contexto `usuario`, aprovisionamiento,
                      `GET /me`, Keycloak), web (el slice del frontend +
                      web/src/presentation/core/generated/*), events (`eventos.rs`, los puertos de
                      unidad de trabajo, `generar evento`), worker (`presentacion/worker`,
                      `registro.rs`, `generar consumidor`), mail (el contexto `correo`, `generar correo`),
                      naming (pascal/snake, el plural en castellano, los verbos CRUD), layout (dónde va
                      cada pieza de un proyecto generado: toda ruta del proyecto se le pide, nunca se
                      escribe a mano)
    src/templates/  — Las plantillas .tera que renderizan los generadores (embebidas con include_dir)
    src/patchers/   — `routes_rs.rs`: añade a `Api<Entidad>` el campo de cada caso de uso nuevo
    src/runtime.rs  — De qué crates del runtime depende un proyecto y en qué versión (se renderiza con
                      la de la CLI; `puerto validar` avisa si la minor no coincide)
    src/puerto_toml.rs — Structs serde de puerto.toml + read/write/add_entity + parser de campos
    src/validation.rs  — Predicados de nombres que comparten el parser de campos y `puerto validar`
    src/tests.rs    — Raíz de los tests: helpers compartidos + un módulo por área en src/tests/
    src/tests/      — Tests estructurales + compile matrix `#[ignore]` (`make test/full`) + snapshots
    template/       — La plantilla base (se edita aquí directamente)
      Cargo.toml.liquid   — Manifiesto del workspace (solo workspace, sin [package]; edition 2024,
                            resolver 3; `members` por glob; los crates internos en
                            `[workspace.dependencies]`)
      cargo-generate.toml — Configuración de la plantilla (qué ficheros llevan sustitución)
      rust-toolchain.toml — Fija la toolchain con la que lintea el matrix (ADR-0009)
      AGENTS.md, CLAUDE.md, contextos/AGENTS.md, presentacion/AGENTS.md
                          — Reglas para agentes que lleva cada proyecto (ADR-0010; con tests)
      web/                — Frontend React + daisyUI sobre Bun (ADR-0013); `--sin-web` lo quita.
                            Nunca `bun install` aquí: la plantilla va embebida (hay test)
      puerto.toml         — Fuente de verdad de las entidades ({{project-name}} sustituido)
      nucleo/             — Crate `nucleo`: los objetos valor compartidos
      contextos/<c>/{dominio,aplicacion,infraestructura,rest}/ — El demo `Saludo`
      presentacion/
        cableado/         — GENERADO entero, `Cargo.toml.liquid` incluido
        api/src/
          main.rs         — Punto de entrada: `puerto_web::Config::from_env()` +
                            `App::new(config).intentar_servir(…)`
          lib.rs          — `mod app; pub mod generado { pub mod bootstrap; }`
          app.rs          — El hook del proyecto: `configurar(rutas)`, se escribe una vez, nunca se
                            regenera
          generado/bootstrap.rs — GENERADO, monta las `Api*` sobre `cableado`
        (la copia de `bootstrap.rs` y de `cableado/` de la plantilla debe coincidir byte a byte con
        el generador — hay test)
docs/adr/           — Architecture Decision Records: el *porqué* de las convenciones (empieza aquí)
docs/glosario.md    — Los nombres de todo lo generado. Una palabra nueva se pregunta y se añade antes
```

## puerto.toml — Fuente de verdad

Cada proyecto generado tiene un `puerto.toml` en la raíz:

```toml
[proyecto]
nombre = "mi-app"

[[entidad]]
nombre = "Saludo"                    # PascalCase
casos_de_uso = ["obtener_saludo"]    # acciones en snake_case

[[entidad]]
nombre = "Producto"
casos_de_uso = ["crear_producto", "listar_productos"]

[[entidad.campos]]
nombre = "nombre"                    # nombre del campo en snake_case
tipo = "String"                      # tipo Rust del registro de tipos

[[entidad.campos]]
nombre = "precio"
tipo = "i64"

[[entidad.campos]]
nombre = "sku"
tipo = "String"
unico = true                         # CREATE UNIQUE INDEX productos_sku_key … WHERE borrado = false (solo SQL)

[[entidad.campos]]
nombre = "descripcion"
tipo = "Option<String>"              # campo nullable

[[entidad.campos]]
nombre = "etiquetas"
tipo = "Vec<String>"                 # campo array
```

**Reglas:**

- `puerto generar scaffold <Nombre>` añade el bloque `[[entidad]]` automáticamente
- Volver a hacer scaffold de una entidad que ya tiene ficheros generados se **rechaza** salvo con
  `--forzar` (en modo interactivo, pregunta). Con `--forzar`, el bloque `[[entidad]]` se actualiza
  en su sitio para que el manifiesto siga describiendo el código en disco — los casos de uso
  añadidos con `generar caso-de-uso` se conservan
- `presentacion/cableado/` y `presentacion/api/src/generado/bootstrap.rs` se regeneran desde
  puerto.toml en cada scaffold
- **Nunca edites a mano `cableado/` ni `generado/bootstrap.rs`** — ejecuta `puerto generar bootstrap`
- Todos los identificadores derivados salen de `nombre` (PascalCase) y de las entradas de
  `casos_de_uso` (snake_case)
- Si `campos` está vacío o no existe, la entidad lleva por defecto `nombre: String` (también un
  correo sin campos)
- Los tipos de los campos deben estar en el registro de tipos — `puerto validar` lo comprueba
- **Nombres reservados** (los rechazan la CLI y `puerto validar`): los campos de sistema `id`,
  `creado_en`, `actualizado_en`, `borrado`, `borrado_en`; las palabras clave de Rust (`type`,
  `match`, `move`, …); nombres de entidad que chocan con el código generado (`Error`, `String`,
  `Option`, `Uuid`, …) o con un contexto que ya existe (`Correo`; `Usuario`, `Me`, `Auth` en
  proyectos con auth); en proyectos `--db`, palabras reservadas de PostgreSQL como nombre de campo
  (`desc`, `order`, `user`, …)
- Se rechazan los nombres de campo duplicados; las acciones de los casos de uso van en snake_case
- `generar caso-de-uso` rechaza una acción que ya existe (incluidas las CRUD del scaffold), escribe
  un solo fichero en `aplicacion_<c>` con su `pub mod`, y añade el campo del caso de uso a
  `Api<Entidad>` en `rutas.rs`, para que `bootstrap.rs` siga compilando
- `generar objeto-valor` valida el nombre (PascalCase), el tipo interno y los duplicados antes de
  escribir puerto.toml
- `puerto nuevo --nombre` admite minúsculas, dígitos, `-` y `_`, y empieza por letra
- `puerto validar` también comprueba que cada `[[entidad]]` declarada tiene código en disco — una
  entidad declarada sin scaffold hace que `cableado` y `bootstrap.rs` importen crates que no
  existen — y que ninguna referencia cierra un ciclo entre contextos

### Registro de tipos

| Tipo Rust | Tipo SQL | OpenAPI | Valor por defecto (tests) |
|-----------|----------|---------|---------------------------|
| `String` | `TEXT` | `string` | `"example"` |
| `i64` | `BIGINT` | `integer(int64)` | `42` |
| `bool` | `BOOLEAN` | `boolean` | `true` |
| `f64` | `DOUBLE` | `number(double)` | `1.5` |
| `Uuid` | `UUID` | `string(uuid)` | `Uuid::new_v4()` |
| `DateTime<Utc>` | `TIMESTAMPTZ` | `string(date-time)` | `Utc::now()` |
| `Option<T>` (`T` = `String`, `i64`, `bool`, `f64`, `Uuid`, `DateTime<Utc>`) | SQL nullable | `T?` | `None` |
| `Vec<String>` | `TEXT[]` | `array[string]` | `vec![]` |
| `Vec<i64>` | `BIGINT[]` | `array[integer]` | `vec![]` |
| `HashMap<String, String>` | `JSONB` (campo de la fila `sqlx::types::Json<…>`) | `object` | `HashMap::new()` |

La tabla completa de derivación y las reglas de los campos, en `.claude/rules/puerto-toml.md`.

## Arquitectura del proyecto generado

Cada proyecto generado es un workspace de Cargo con un crate por contexto y capa (ADR-0023); en la
v1, cada entidad es un contexto. La regla de dependencias (solo hacia dentro) la impone Cargo:

```
rest_<c> → aplicacion_<c> → dominio_<c> → nucleo, puerto-nucleo
infra_<c> → dominio_<c>                    (solo cableado y los binarios dependen de infra_*)
```

- **Dominio** (`dominio_<c>`) — Rust puro, sin más crates externos que std, async-trait, thiserror
  y `puerto-nucleo`
- **Aplicación** (`aplicacion_<c>`) — un fichero por caso de uso: params, trait, doble,
  implementación y tests; importa solo el dominio
- **Infraestructura** (`infra_<c>`) — adaptadores (en memoria, Postgres); importa el dominio
- **REST** (`rest_<c>`) — API REST con poem-openapi; importa aplicación y dominio
- **Presentación** (`presentacion/`) — `cableado` (la raíz de composición), `api` y `worker` (los
  binarios)

Una referencia `cliente_id:ref:Cliente` hace que `dominio_pedido` dependa de `dominio_cliente`;
Cargo no admite ciclos, así que el mapa de contextos lo vigila el compilador.

## Reglas críticas

- **Variables de la plantilla**: `{{project-name}}`, solo en los ficheros que lista el `include` de
  `cargo-generate.toml` (`puerto.toml`, `presentacion/api/Cargo.toml`, `Makefile`,
  `web/index.html`). Los ficheros que no están en la lista se copian tal cual.
- **Los `Cargo.toml` de la plantilla se llaman `Cargo.toml.liquid`**: cargo-generate les quita la
  extensión. El raíz lleva `PUERTO_RUNTIME_VERSION`, que `commands/new.rs` sustituye por la versión
  de la CLI (`runtime::render_manifest`).
- **`no_workspace: true`** tiene que ir en `GenerateArgs` — evita que cargo-generate intente añadir
  el proyecto generado como miembro del workspace de Puerto.
- **Nunca añadas `[package]` al `Cargo.toml.liquid` raíz de la plantilla** — es solo un manifiesto
  de workspace.
- **`members` va por glob** (`contextos/*/*`, `presentacion/*`): un glob que no casa con nada,
  Cargo lo lee como una ruta y no carga el workspace. Un proyecto `--sin-demo` no lista
  `contextos/*/*` hasta el primer contexto; `migrador` solo aparece en los proyectos `--db`.
- **Toda ruta de un proyecto generado se pide a `generators/layout.rs`**, nunca se escribe literal.
- **`cableado/` y `generado/bootstrap.rs` son generados** — nunca se editan a mano ni se commitean
  ediciones a mano en los proyectos generados.

## Comandos de la CLI

```bash
puerto nuevo                                   # Interactivo: pide el nombre y si lleva base de datos
puerto nuevo --nombre <nombre>                 # Sin preguntar el nombre
puerto nuevo --db                              # Sin preguntar por la base de datos (SQLx/Postgres)
puerto nuevo --nombre <nombre> --db            # Sin preguntas, con base de datos
puerto nuevo --nombre <nombre> --sin-db        # Sin preguntas, sin base de datos
puerto nuevo --nombre <nombre> --sin-db --destino /tmp/proyectos  # Elige el directorio de salida
puerto nuevo --sin-web                         # Sin frontend (web/ se incluye por defecto)
puerto nuevo --sin-demo                        # Sin el demo Saludo (proyecto vacío)
puerto nuevo --db --eventos db                 # Eventos de dominio (ADR-0018): unidades de trabajo + outbox (db | kafka)
puerto nuevo --auth oidc                       # Auth (ADR-0021): tokens de un proveedor OIDC (Auth0, Keycloak); Keycloak en compose
puerto nuevo --db --auth local                 # Auth: cuentas y sesiones en esta base de datos; /auth/registro|login|logout, pantallas /login y /register

puerto generar scaffold <Nombre>                                            # Entidad DDD nueva — todas las capas (db, deducida de puerto.toml)
puerto generar scaffold <Nombre> --forzar                                   # Regenera una entidad existente, pisando los cambios a mano
puerto generar scaffold <Nombre> -- nombre:String precio:i64!              # Campos primitivos (! = único)
puerto generar scaffold <Nombre> -- descripcion:opt:String etiquetas:vec:String   # Primitivos Option / Vec
puerto generar scaffold <Nombre> -- nombre:Nombre:String edad:Edad:i64     # Campos con objeto valor
puerto generar scaffold Producto --publico listar_productos --rol borrar_producto=admin   # Proyectos con auth: rutas abiertas, roles exigidos
puerto generar scaffold <Nombre> -- estado:Estado:enum:Activo/Inactivo     # Objeto valor enum
puerto generar scaffold <Nombre> -- segundo_nombre:SegundoNombre:opt:String   # Objeto valor Option
puerto generar scaffold <Nombre> -- etiquetas:Etiqueta:vec:String          # Objeto valor Vec
puerto generar scaffold Pedido -- cliente_id:ref:Cliente revisor_id:ref:opt:Cliente   # Referencias (ADR-0022): el id del objetivo, una clave foránea
puerto generar objeto-valor <Nombre> <tipo>         # Declara un objeto valor compartido en puerto.toml (p. ej. Email String; vale DateTime)
puerto generar evento <Entidad> <nombre>            # Proyectos con eventos: un evento propio (producto_publicado) + regenera eventos.rs
puerto generar consumidor <nombre> --tras producto_creado,producto_publicado   # Proyectos con eventos: un consumidor del worker + regenera registro.rs
puerto generar correo <nombre> [-- nombre:String plazas:i64 plan:opt:String]  # Un correo: puerto, enviador con plantillas, plantillas; el primero añade Mailpit
puerto generar correo <nombre> --tras cliente_creado --para email   # Proyectos con eventos: + el consumidor correo_<nombre> que lo envía una vez por evento
puerto generar dominio <Nombre>                     # Solo el crate de dominio: modelo, errores, repositorio, madre
puerto generar dominio <Nombre> -- precio:i64 estado:Estado:enum:Borrador/Pagado   # Misma sintaxis que scaffold — los campos van a puerto.toml
puerto generar aplicacion <Nombre>                  # Solo el crate de aplicación: los casos de uso
puerto generar repositorio <Nombre>                 # Solo el crate de infraestructura: en memoria o Postgres (deducido de puerto.toml)
puerto generar presentacion <Nombre>                # Solo el crate REST + regenera el cableado; se niega hasta que existe la aplicación
puerto generar web <Nombre>                         # Proyectos con web: solo el slice del frontend (+ core/generated/*); --forzar sobrescribe
puerto generar caso-de-uso <Entidad> <accion>       # Añade un caso de uso (rechaza acciones existentes; añade el campo a Api<Entidad>)
puerto generar migracion <nombre>                   # Nueva migración SQLx en migraciones/ (solo proyectos --db; las versiones nunca chocan)
puerto generar bootstrap                            # Regenera cableado y bootstrap.rs desde puerto.toml (se niega si hay deriva manifiesto/disco)
puerto generar bootstrap --permitir-faltantes       # Los escribe igualmente, aunque una entidad declarada no tenga código
puerto generar snippets                             # Ficheros de snippets para el IDE (Zed + VS Code)
puerto generar snippets --ide zed                   # Solo Zed  (.zed/snippets/rust.json)
puerto generar snippets --ide vscode                # Solo VS Code (.vscode/puerto.code-snippets)
puerto validar                                      # Valida puerto.toml (tipos, nombres, reservados, ciclos, deriva manifiesto/disco)
puerto listar                                       # Lista las entidades y casos de uso de puerto.toml
```

## Comandos de make

```bash
make run           # Ejecuta la CLI puerto (pide el nombre del proyecto)
make test          # Tests estructurales rápidos (estructura de lo generado y snapshots)
make test/full     # Lento: genera proyectos reales, ejecuta su `clippy --all-targets -D warnings` y sus tests
                   #        y los checks del frontend generado (necesita `bun` en el PATH)
make test/db       # Lento: un proyecto --db generado contra un Postgres + Mailpit desechables (Docker + sqlx-cli);
                   #        también el e2e de la API de un proyecto con auth local (registro → login → /me → logout)
make test/kafka    # Lento: el transporte Kafka contra un Redpanda desechable (+ Postgres, Mailpit; Docker, cmake)
make test/mail     # Lento: el adaptador SMTP contra un Mailpit desechable (Docker)
make test/auth     # Lento: el adaptador OIDC + la API de un proyecto OIDC generado contra un Keycloak 26.8
                   #        y un Postgres desechables (Docker + sqlx-cli)
make test/mutation # Lento: cargo-mutants sobre validation / migrations / types / patchers
make test/snapshots # Regraba los snapshots de lo generado tras un cambio de salida intencionado
make lint          # cargo clippy --workspace --all-targets -- -D warnings
make check         # cargo check --workspace
make format        # cargo fmt --all -- --check
make format/fix    # cargo fmt --all
```

## Lista de comprobación al terminar

- [ ] `make test` pasa (tests estructurales — rápidos)
- [ ] `make test/full` pasa (los proyectos generados lintean limpios y pasan sus tests — lento;
      necesita `bun` en el `PATH` para los escenarios web)
- [ ] `make test/db` pasa cuando el cambio toca la generación `--db` (repositorio Postgres,
      migraciones, SQL)
- [ ] Cambio de Tier 1 (plantillas, esquema, bootstrap, migraciones, patchers): diff leído entero —
      ver `.claude/rules/workflow.md` § Niveles de riesgo
- [ ] Un commit por pieza; una decisión de diseño sin rastro en el código lleva una ADR
- [ ] `make lint` limpio (cero warnings)
- [ ] `make format/fix` ejecutado

## El contrato del proyecto generado (lo que recibe cada proyecto)

- Un crate por contexto y capa (ADR-0023): `contextos/<c>/{dominio,aplicacion,infraestructura,rest}`
  → `dominio_<c>`, `aplicacion_<c>`, `infra_<c>`, `rest_<c>`; `nucleo` (objetos valor compartidos,
  sin E/S); `presentacion/{cableado,api}` (+ `worker` con eventos); con `--db`, `migraciones/` (el
  SQL de toda la base de datos) y `migrador` (`MIGRADOR` + `build.rs` con `rerun-if-changed`).
  El `lib.rs` de cada crate es plano y sin `mod.rs`. Los crates internos van en
  `[workspace.dependencies]` y cada miembro los usa con `{ workspace = true }`
- Crates del runtime en vez de copias (ADR-0017): `puerto-nucleo = "<minor de la cli>"` en el
  `[workspace.dependencies]` raíz; el dominio importa
  `puerto_nucleo::{ErrorRepositorio, Pagina, PeticionPagina}`. Los `rest_*` y `api` dependen de
  `puerto-web`: `main.rs` es `Config::from_env()` + `App::new(config).intentar_servir(construir_app)`,
  y `bootstrap.rs` entrega su API y su sonda de disponibilidad a `puerto_web::rutas`, y luego al
  `app::configurar` del usuario. En un proyecto `--db`, los `infra_*` y `api` dependen de
  `puerto-sqlx`; `api` arranca con `crear_pool_postgres` + `ejecutar_migraciones(&pool,
  &migrador::MIGRADOR)`
- `cableado` (GENERADO, `Cargo.toml` incluido) construye, a partir del pool, los repositorios, las
  unidades de trabajo y los casos de uso de cada contexto: `cableado::<c>::construir(…)` devuelve
  sus `CasosDeUso`. No conoce poem. Lo que pide configuración propia (el proveedor OIDC, la
  duración de las sesiones, el SMTP) se queda en el binario que lo usa
- Formateado: todo comando que escribe Rust termina con `cargo fmt --all` en el proyecto, así que
  su `make format` pasa; si el formateo falla, solo avisa (ADR-0019)
- Un fallo de arranque es un mensaje, no un panic: `construir_app` / `construir_worker` devuelven
  `Result<_, String>` (`?`, una pista por causa: `.env`, `make docker-compose/up`), y `main` los
  pasa a `intentar_servir` / `intentar_ejecutar`, que registran el error y salen con 1.
  `puerto_sqlx::ErrorBaseDatos` dice `host:puerto/base` y la causa del driver, nunca la contraseña
- Logs con `tracing` directamente — sin puerto de logger (ADR-0006)
- Errores: `RespuestaError { codigo, mensaje }` con el código en `codigo`; validación → 422; cada
  `rest_<c>/errores.rs` exporta la función libre `a_respuesta_error(err)`, con un test por rama
  (ADR-0007, ADR-0023 § 8)
- `/health`, `/health/ready`, apagado ordenado con SIGTERM (ADR-0008)
- Toolchain fijada + edition 2024 (ADR-0009); `AGENTS.md` + `CLAUDE.md` en la raíz, y
  `contextos/AGENTS.md`, `presentacion/AGENTS.md` (ADR-0010)
- Targets de make: `test/dominio`, `test/aplicacion`, `test/infraestructura` (un glob de paquetes
  cada uno), `generar/entidad`, `generar/caso-de-uso`, `generar/bootstrap`, `clean/incremental`;
  con `--db`, `db/arrancar`, `db/parar`, `db/reiniciar`, `sqlx/*`, `generar/migracion`
- `web/` salvo con `--sin-web`: `[proyecto] web = true`, targets `make web/*` (ADR-0013);
  `scaffold` escribe las pantallas de una entidad CRUD (dominio, repositorio HTTP,
  lista/detalle/formulario, tests) y regenera
  `web/src/presentation/core/generated/{dependencies.ts,routes.tsx}`; `bootstrap.rs` exporta la
  especificación OpenAPI a `docs/api/openapi.json` desde los dobles de los casos de uso, y
  `make web/sync` la convierte en los tipos del frontend. El frontend sigue en inglés (su
  traducción es una fase aparte)
- Los formularios web dicen qué falló junto al campo: `domain/<entity>/messages.ts` (generado de
  los mismos códigos que `errores.rs`) asocia un código a un mensaje y un campo; `FieldError` lo
  muestra bajo ese input, los demás códigos van a la alerta, los desconocidos se muestran tal cual
- `--eventos db|kafka` (necesita `--db`, ADR-0018): `[proyecto] eventos`, la migración del outbox
  (el `templates/infrastructure/outbox.sql` de la CLI), por cada entidad CRUD `eventos.rs`
  (GENERADO) y `unit_of_work.rs` en `dominio_<c>`, un puerto de repositorio de solo lectura,
  escrituras por `UnitOfWorkFactory<Entidad>Postgres` (`infra_<c>/src/unit_of_work.rs`), y una
  sección del `AGENTS.md` raíz que dice que `eventos.rs` nunca se edita a mano
- `--eventos` también escribe `presentacion/worker` (ADR-0018 § 5): `main.rs` es
  `ConfigWorker::from_env()` + `Worker::new(config).intentar_ejecutar(construir_worker)`
  (`puerto-eventos`), `generado/registro.rs` (GENERADO desde `[[consumidor]]`; todavía no usa
  `cableado`: los consumidores no reciben casos de uso), `consumidores/<nombre>.rs` (del usuario),
  `make worker`, `make eventos/descartados`, `WORKER_PORT`. Con `db`, el registro pasa
  `OutboxPostgres` → `EntregasPostgres`, con una `SuscripcionPostgres` por consumidor. Con `kafka`,
  pasa a `PublicadorKafka` (los topics se crean al arrancar), con una `SuscripcionKafka` por
  consumidor (`puerto-kafka`); la migración del outbox no tiene `entregas_eventos`,
  `docker-compose.yml` gana Redpanda y `.env`, `KAFKA_BROKERS`
- `--auth oidc` (ADR-0021): `[proyecto] auth`, escrito una vez: el contexto `usuario` con la
  entidad `Usuario` (`contextos/usuario/`, en memoria o en Postgres con consultas comprobadas en
  ejecución, la migración de `usuarios` con `UNIQUE (emisor, sujeto)`),
  `contextos/usuario/infraestructura/src/aprovisionamiento.rs` (el autenticador del proyecto:
  pregunta al proveedor y busca o crea el usuario, con una caché de 60 s) y `GET /me`
  (`contextos/usuario/rest/src/me/`). `bootstrap.rs` descubre el proveedor
  (`puerto_auth::AutenticadorOidc::descubrir`; si está caído, un error de arranque con pista) y da
  a las rutas `.data(Arc<dyn Autenticador>)`. Keycloak 26.8 en `docker-compose.yml` con
  `keycloak/realm.json` (cliente `web`, mapper de audiencia `api`, usuario
  `admin@<proyecto>.localhost`; un cliente solo de desarrollo `dev-cli` con password grant y un
  usuario `user@<proyecto>.localhost` sin roles), `AUTH_*` en `.env`, `make auth/ui`,
  `make -s auth/token [EMAIL=…]` (un access token de `dev-cli`), una sección del `AGENTS.md` raíz.
  El demo Saludo es público. `scaffold` en un proyecto con auth: cada handler CRUD recibe
  `auth: BearerAuth` salvo los `--publico`, `--rol` añade `#[requiere_rol]` (los dos quedan en
  `[[entidad]] publicos` / `roles`); los params protegidos llevan `actor: Principal` y sus
  respuestas un 503 `NoDisponible`; `Error<Entidad>::Prohibido` (403, `<entidad>.prohibido`) con
  su test; los tests de rutas inician sesión con un `MockAutenticador` (un llamante con todos los
  roles y otro sin ninguno) y añaden un 401 por ruta protegida y un 403 por ruta con rol. Los tests
  construyen el actor con `puerto_nucleo::auth::dobles::MadrePrincipal`
- `--auth local` (necesita `--db`): el mismo `Usuario` (uno local tiene emisor `local` y como
  sujeto su email en minúsculas) en Postgres, sin aprovisionamiento ni Keycloak; `registrar_usuario`
  / `login` / `logout` (`contextos/usuario/aplicacion/`, sobre `HasherContrasena`,
  `AlmacenCredenciales` y `AlmacenSesiones` de `puerto-nucleo`) y
  `POST /auth/{registro,login,logout}` (`contextos/usuario/rest/src/auth/`, 422/401/409/500, tests
  de rutas). `bootstrap.rs` construye `HasherArgon2`, `CredencialesPostgres` y `SesionesPostgres` —
  el autenticador. Las migraciones de `usuarios` y de `auth`, `AUTH_SESSION_TTL=30d` en `.env`,
  `make auth/conceder EMAIL= ROL=`, su propia sección Auth en el `AGENTS.md` raíz
- `--auth local` con `web/`: los mismos ficheros compartidos que OIDC (`session.ts`, el middleware
  bearer, `auth.tsx`, el menú de usuario, `renderScreen`), más `domain/auth/{accounts,messages}.ts`
  (el puerto `Accounts`, los códigos de la API como mensajes de formulario),
  `infrastructure/auth/local-auth-gateway.ts` (el token en `localStorage`; `login(returnTo)` lo
  borra y va a `/login?next=`) y las páginas `/login` y `/register`. `dependencies.ts` construye un
  solo `localAuthGateway()` como `auth` y `accounts`; `routes.tsx` tiene `/login` y `/register` en
  vez de `/auth/callback`. Sin `oidc-client-ts`, sin `web/.env`
- `--auth oidc` con `web/`: escrito una vez — `domain/auth/session.ts` (`AuthGateway`,
  `CurrentUser`), `infrastructure/auth/{oidc-auth-gateway,bearer-middleware}.ts` (`oidc-client-ts`,
  fijado, en memoria; dependency-cruiser lo mantiene ahí), `presentation/core/auth.tsx`
  (`AuthProvider`, `useAuth`, `RequireAuth`), `/auth/callback`, un menú de usuario en el layout,
  `test-support/auth.ts` y un `renderScreen` que inicia sesión con un llamante con todos los roles;
  `VITE_AUTH_*` en `web/.env`. Regenerados: `dependencies.ts` construye el gateway y firma el
  cliente, `routes.tsx` envuelve en `<RequireAuth>` cada pantalla cuyo caso de uso está protegido,
  `roles.ts` lista los roles. Las pantallas del scaffold ocultan Nuevo/Editar/Borrar tras
  `can(role)` cuando el caso de uso tiene rol, con un test cada uno, y un test de 403 en un borrado
  protegido
- Referencias (ADR-0022): `cliente_id:ref:Cliente` (`[[entidad.campos]] referencia`) — el dominio,
  los Params y la madre guardan `IdCliente` (`IdCliente::generar()` en los tests), las filas y los
  DTO un `Uuid`; `dominio_pedido` depende de `dominio_cliente`. `--db`:
  `CONSTRAINT pedidos_cliente_id_fkey REFERENCES clientes (id)` + un índice; el repositorio
  Postgres (la unidad de trabajo en proyectos con eventos) traduce `23503` con sus `REFERENCES` a
  `ErrorRepositorio::ReferenciaInexistente(campo)`, que `errores.rs` convierte en
  `Validacion("cliente_id_no_encontrado")` (422). Los tests de Postgres siembran el objetivo con
  `infra_cliente::pruebas::siembra::sembrar` (dependencia opcional activada por la feature
  `pruebas`). La lista filtra por cada referencia: `GET /pedidos?clienteId=` →
  `ParamsListarPedidos { cliente_id }` → `buscar_todos(pagina, FiltroPedido)` (Postgres:
  `$3::uuid IS NULL OR cliente_id = $3`). Web: un `<select>` con los primeros `MAX_PAGE_LIMIT`
  elementos del objetivo (`referenceOptions`, etiquetados por su primer campo `String` obligatorio
  o, si no, por su id), `<ReferenceLink>` en el detalle, `cliente_id_no_encontrado` bajo el select
  (`messages.ts`). `scaffold` rechaza un objetivo que aún no tiene código, y `scaffold` y `validar`,
  una referencia que cierra un ciclo; todavía no hay autorreferencias, `vec:ref:` ni `ref:Usuario`
- `puerto generar correo <nombre>` (cualquier proyecto, ADR-0020): el contexto `correo`
  (`contextos/correo/{dominio,infraestructura}`, sin aplicación): `dominio_correo::<nombre>`
  (`Correo<Nombre>` + el puerto `EnviadorCorreo<Nombre>` + su doble),
  `infra_correo::<nombre>` (`EnviadorCorreo<Nombre>Plantilla` sobre `puerto-correo`, con tests de
  renderizado) y `contextos/correo/infraestructura/plantillas/<nombre>/{asunto.txt,cuerpo.html,
  cuerpo.txt}` — escritos una vez, del usuario. El primer correo añade `puerto-correo`, Mailpit a
  `docker-compose.yml` (que se crea si no hay `--db`), `SMTP_URL` / `MAIL_FROM`, `make correo/ui` y
  una sección del `AGENTS.md` raíz. Una entidad no puede llamarse `Correo`. Con
  `--tras <tipo_evento> --para <campo>` (proyectos con eventos), también
  `presentacion/worker/src/consumidores/correo_<nombre>.rs` (`EnviarCorreo<Nombre>`: snapshot →
  correo → `puerto_correo::enviar_una_vez`), `[[correo]] tras`, el consumidor `correo_<nombre>` en
  `registro.rs` (un `EnviadorCorreoSmtp` + un `CorreosEnviadosPostgres`) y, la primera vez, la
  migración de `correos_enviados` (el `templates/infrastructure/sent_mails.sql` de la CLI) y
  `puerto-correo` en `worker`

## Reglas detalladas

- `.claude/rules/architecture.md` — Reglas de capas, convenciones de nombres, patrones de error
- `.claude/rules/puerto-toml.md` — Esquema de puerto.toml + tabla de derivación de identificadores
- `.claude/rules/testing.md` — Ciclo TDD, nombres de tests, patrón AAA, qué sí y qué no
- `.claude/rules/testing-conventions.md` — Object Mother, dobles, tabla de nombres
- `.claude/rules/workflow.md` — Flujo de desarrollo de Puerto (Spec-Driven) + TDD del proyecto generado
- `.claude/rules/value_objects.md` — Objetos valor: sintaxis, esquema y su paso por las capas
- `.claude/rules/release.md` — Versionado, changelog, proceso de release, cambios incompatibles, crates.io
- `docs/adr/` — Architecture Decision Records
- `docs/glosario.md` — El vocabulario de lo generado
