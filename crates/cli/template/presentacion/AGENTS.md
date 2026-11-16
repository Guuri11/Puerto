# Presentación — notas para agentes de IA

Los binarios y su cableado (ADR-0023 § 7 de Puerto). Aquí no vive lógica de negocio: cada crate
es una raíz de composición.

## Estructura

```
presentacion/
├── cableado/                 # crate GENERADO, `Cargo.toml` incluido — no lo edites a mano
│   └── src/lib.rs            # por contexto: su repositorio, su unidad de trabajo y sus casos de uso, construidos
├── api/                      # el binario HTTP (con el nombre del proyecto)
│   └── src/
│       ├── main.rs           # `puerto_web::Config::from_env()` + `App::new(config).intentar_servir(…)` (no cambia)
│       ├── lib.rs            # `mod app; pub mod generado { pub mod bootstrap; }`
│       ├── app.rs            # Tuyo: `configurar(rutas)` — middleware y rutas extra
│       └── generado/bootstrap.rs   # GENERADO: la `Api<Entidad>` de cada contexto sobre `cableado` — no lo edites a mano
└── worker/                   # proyectos con `eventos`: releva el outbox y ejecuta los consumidores (con su propio AGENTS.md)
```

## Reglas críticas

- **El cableado se genera**: `puerto generar bootstrap` (y cada scaffold) reescribe `cableado` y
  `generado/bootstrap.rs` desde `puerto.toml` — nunca cablees a mano. Los mismos comandos añaden
  a `api/Cargo.toml` el crate REST de un contexto nuevo.
- **`cableado` no sabe de HTTP**: construye los casos de uso; `api` monta las rutas sobre ellos.
- **`Api<Entidad>` guarda puertos** — `Arc<dyn CrearProducto>`, nunca el `CrearProductoImpl`.
- **Contrato de errores**: todo cuerpo de error es `RespuestaError { codigo, mensaje }`, construido
  con `RespuestaError::from_codigo(…)`. `codigo` es el código legible por máquina
  (`producto.no_encontrado`). Validación del dominio (objetos valor inválidos incluidos) → `422`;
  no encontrado → `404`; duplicado → `409`; repositorio / desconocido → `500`. Poem responde por su
  cuenta `400` a un cuerpo mal formado. El mapeo lo hace `rest_<c>::errores::a_respuesta_error` de
  cada contexto.
- Tu middleware y tus rutas van en `api/src/app.rs`, que ningún generador toca.

## Salud y apagado

Los sirve el runtime de Puerto, `puerto-web`; `generado/bootstrap.rs` le pasa la comprobación de
disponibilidad.

- `GET /health` — vida: el proceso responde.
- `GET /health/ready` — disponibilidad: `503` hasta que responde cada dependencia (la base de
  datos, en un proyecto `--db`). Apunta aquí las comprobaciones de disponibilidad del orquestador,
  y las de vida a `/health`.
- Con SIGTERM / Ctrl-C el servidor deja de aceptar peticiones y termina las que tiene en curso,
  durante como mucho `puerto_web::app::PLAZO_APAGADO` (8 s).

## Configuración

`puerto_web::Config::from_env()` lee el entorno, después de `.env`: `SERVICE_PORT` (por defecto
`8080`), `CORS_ALLOWED_ORIGINS` (separados por comas; sin ninguno, vale cualquier origen),
`PUERTO_BANNER` (`false` oculta el banner). Un valor definido pero inválido detiene el servidor al
arrancar.

## Patrón de un handler

En `contextos/<c>/rest/src/rutas.rs`:

```rust
#[oai(path = "/productos/:id", method = "get", tag = "ApiTags::Producto")]
async fn obtener(&self, id: Path<Uuid>) -> RespuestaObtenerProducto {
    match self.obtener_producto.ejecutar(ParamsObtenerProducto { id: IdProducto::from_uuid(id.0) }).await {
        Ok(entidad) => RespuestaObtenerProducto::Ok(Json(DtoProducto::from_dominio(&entidad))),
        Err(err) => {
            let (status, error) = a_respuesta_error(err);
            tracing::warn!(codigo = %error.0.codigo, "obtener_producto falló");
            RespuestaObtenerProducto::from_status(status, error)
        }
    }
}
```

Un caso de uso añadido con `puerto generar caso-de-uso` gana un campo en `Api<Entidad>`; exponlo
añadiendo un endpoint a `rutas.rs`, con un test de ruta por estado.
