# Puerto

Puerto es una CLI en Rust que genera backends DDD con Clean Architecture (hexagonal, puertos y
adaptadores): un workspace con un crate por contexto y capa, una API REST con poem-openapi, la
inyección de dependencias generada desde un manifiesto y tests en cada capa. Todo lo que genera va
en castellano (`RepositorioProducto`, `crear_producto`, `/productos`). Web y documentación:
<https://puerto-rs.vercel.app>.

## Instalación

```bash
cargo install puerto --locked
```

`--locked` compila con las versiones de dependencias con las que se probó Puerto; sin él, cargo
puede elegir dependencias transitivas más nuevas que pidan un Rust más nuevo que el tuyo.

## Empezar

```bash
puerto nuevo                                  # interactivo: nombre, base de datos, entidad de demo
puerto nuevo --nombre mi-app --db             # sin preguntas, con SQLx/Postgres
cd mi-app
puerto generar scaffold Producto -- titulo:String precio:i64 sku:String!
make run                                      # http://localhost:8080: Swagger UI
```

## Comandos

```bash
puerto generar scaffold <Nombre> [--forzar] -- <campos>    # una entidad en todas las capas
puerto generar caso-de-uso <Entidad> <accion>              # un caso de uso más (snake_case)
puerto generar objeto-valor <Nombre> <tipo>                # un objeto valor compartido
puerto generar dominio|aplicacion|repositorio|presentacion <Nombre>   # una capa cada vez
puerto generar migracion <nombre>                          # migración SQL (proyectos --db)
puerto generar bootstrap                                   # vuelve a cablear desde puerto.toml
puerto generar snippets [--ide zed|vscode]                 # snippets del IDE
puerto validar                                             # comprueba puerto.toml contra el código
```

Sintaxis de los campos: `nombre:String`, `precio:i64!` (único), `nota:opt:String`,
`etiquetas:vec:String`, `nombre:Nombre:String` (objeto valor), `estado:Estado:enum:Activo/Archivado`,
`email:Email` (objeto valor compartido), `cliente_id:ref:Cliente` (una referencia).

## Lo que trae cada proyecto generado

- Un crate por contexto y capa: `contextos/<c>/{dominio,aplicacion,infraestructura,rest}`
  (`dominio_<c>`, `aplicacion_<c>`, `infra_<c>`, `rest_<c>`), `nucleo` para los objetos valor
  compartidos, `presentacion/{cableado,api}`; Cargo impone que las dependencias solo apunten hacia
  dentro
- `puerto.toml` como fuente de verdad; `presentacion/cableado` y
  `presentacion/api/src/generado/bootstrap.rs` se cablean desde él
- Tests unitarios por caso de uso, madres (Object Mother), dobles con `mockall`; tests de
  integración contra Postgres con `--db`
- `GET /health`, `GET /health/ready` (sondea la base de datos), apagado ordenado con SIGTERM
- Errores `{ codigo, mensaje }` con un código legible por máquina (`producto.no_encontrado`); la
  validación es `422`
- Logs con `tracing`; una toolchain fijada (`rust-toolchain.toml`), edition 2024
- Instrucciones `AGENTS.md` + `CLAUDE.md` para agentes
- Un `make lint` (clippy `-D warnings`) que pasa de serie

## Licencia

MIT
