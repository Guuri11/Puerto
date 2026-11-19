# Objetos valor — reglas de generación de Puerto

## Sintaxis

Los argumentos de campo de la CLI usan una sintaxis con dos puntos — sin comillas en ninguna shell:

```bash
# Primitivo (sin objeto valor)
puerto generar scaffold Pedido -- importe:f64 activo:bool

# Primitivos Option / Vec (atajo seguro para la shell)
puerto generar scaffold Pedido -- descripcion:opt:String etiquetas:vec:String

# Atajos DateTime / HashMap
puerto generar scaffold Evento -- ocurrido_en:DateTime meta:map

# Objeto valor normal (envuelve un primitivo)
puerto generar scaffold Persona -- nombre:Nombre:String edad:Edad:i64

# Objeto valor único
puerto generar scaffold Producto -- sku:Sku:String!

# Option<VO> (nullable)
puerto generar scaffold Persona -- segundo_nombre:SegundoNombre:opt:String

# Vec<VO> (array)
puerto generar scaffold Articulo -- etiquetas:Etiqueta:vec:String

# Objeto valor enum (variantes separadas por /)
puerto generar scaffold Pedido -- estado:Estado:enum:Pendiente/Confirmado/Cancelado

# Mezcla
puerto generar scaffold Persona -- nombre:Nombre:String edad:Edad:i64 activo:bool
```

| Formato | Resultado |
|---------|-----------|
| `campo:TipoPrimitivo` | Campo primitivo, sin objeto valor |
| `campo:opt:Tipo` | Primitivo `Option<Tipo>` |
| `campo:vec:Tipo` | Primitivo `Vec<Tipo>` |
| `campo:DateTime` | Primitivo `DateTime<Utc>` (atajo) |
| `campo:map` | Primitivo `HashMap<String, String>` (atajo) |
| `campo:NombreVo:Tipo` | Objeto valor que envuelve un primitivo |
| `campo:NombreVo:opt:Tipo` | Objeto valor nullable (`Option<NombreVo>`) |
| `campo:NombreVo:vec:Tipo` | Objeto valor array (`Vec<NombreVo>`) |
| `campo:NombreVo:enum:V1/V2/...` | Objeto valor enum (sobre un String) |
| `campo:NombreVo` | Objeto valor compartido ya declarado: el tipo se deduce de `[[objeto_valor]]` |
| `!` al final del argumento | `unico = true` (no vale en objetos valor Option/Vec) |

### Objetos valor compartidos

Se declaran antes del scaffold:

```bash
puerto generar objeto-valor Email String
puerto generar objeto-valor Dinero i64
```

Esto añade bloques `[[objeto_valor]]` a `puerto.toml`. Cuando el objeto valor de un campo coincide
con uno compartido declarado, el import pasa de local (`crate::valores`) a `nucleo::valores`.

`--ejemplo <valor>` (27.15, `[[objeto_valor]] ejemplo`; solo `String` no vacío, `i64`, `f64` y
`bool`, leído como su tipo) es el valor válido que usan los tests de sus campos en vez de
`"example"`: la madre, los cuerpos de los tests de rutas y de casos de uso, la siembra de un campo
no único y la madre de la web. Un campo no lo guarda: `puerto_toml::read` y
`apply_shared_vo_inference` se lo dan (`Field::example`, `#[serde(skip)]`).

## Esquema de puerto.toml

```toml
[proyecto]
nombre = "mi-app"

# Objetos valor compartidos — reutilizables entre entidades
[[objeto_valor]]
nombre = "Email"
tipo = "String"

[[objeto_valor]]
nombre = "Dinero"
tipo = "i64"
ejemplo = "1250"                # opcional (27.15): lo que usan sus tests en vez del valor por defecto

[[entidad]]
nombre = "Cliente"
casos_de_uso = ["crear_cliente"]

[[entidad.campos]]
nombre = "email"
tipo = "String"
objeto_valor = "Email"          # coincide con un [[objeto_valor]] → compartido

[[entidad.campos]]
nombre = "nombre"
tipo = "String"
objeto_valor = "Nombre"         # sin [[objeto_valor]] → local

[[entidad.campos]]
nombre = "segundo_nombre"
tipo = "Option<String>"
objeto_valor = "SegundoNombre"  # objeto valor nullable

[[entidad.campos]]
nombre = "etiquetas"
tipo = "Vec<String>"
objeto_valor = "Etiqueta"       # objeto valor Vec

[[entidad.campos]]
nombre = "estado"
tipo = "String"
objeto_valor = "Estado"
clase_objeto_valor = "enum"
variantes = ["Activo", "Inactivo", "Suspendido"]

[[entidad.campos]]
nombre = "activo"
tipo = "bool"
# sin objeto_valor → campo primitivo
```

## El struct `Field` (`puerto_toml.rs`)

El código de Puerto sigue en inglés; las claves en castellano son renombres de serde:

```rust
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Field {
    #[serde(rename = "nombre")]
    pub name: String,
    #[serde(rename = "tipo")]
    pub field_type: String,
    #[serde(default, skip_serializing_if = "std::ops::Not::not", rename = "unico")]
    pub unique: bool,
    #[serde(default, skip_serializing_if = "Option::is_none", rename = "objeto_valor")]
    pub value_object: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none", rename = "clase_objeto_valor")]
    pub value_object_kind: Option<String>,   // "enum" para los objetos valor enum
    #[serde(default, skip_serializing_if = "Option::is_none", rename = "variantes")]
    pub enum_variants: Option<Vec<String>>,  // ["Activo", "Inactivo"]
    #[serde(default, skip_serializing_if = "Option::is_none", rename = "referencia")]
    pub references: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ValueObjectDefinition {
    #[serde(rename = "nombre")]
    pub name: String,
    #[serde(rename = "tipo")]
    pub inner_type: String,
}
```

### Ejemplos de análisis

```
parse_field_arg("nombre:Nombre:String")
→ Field { name: "nombre", field_type: "String", value_object: Some("Nombre"), .. }

parse_field_arg("segundo_nombre:SegundoNombre:opt:String")
→ Field { name: "segundo_nombre", field_type: "Option<String>", value_object: Some("SegundoNombre"), .. }

parse_field_arg("etiquetas:Etiqueta:vec:String")
→ Field { name: "etiquetas", field_type: "Vec<String>", value_object: Some("Etiqueta"), .. }

parse_field_arg("estado:Estado:enum:Activo/Inactivo")
→ Field { name: "estado", field_type: "String", value_object: Some("Estado"),
          value_object_kind: Some("enum"), enum_variants: Some(["Activo", "Inactivo"]) }

parse_field_arg("sku:Sku:String!")
→ Field { name: "sku", field_type: "String", unique: true, value_object: Some("Sku"), .. }

parse_field_arg("precio:i64")
→ Field { name: "precio", field_type: "i64", value_object: None, .. }
```

Reglas de validación:
- El nombre del objeto valor va en PascalCase
- Objetos valor normales: `field_type` es un primitivo, `Option<primitivo>` o `Vec<primitivo>`
- Primitivos internos admitidos: `String`, `i64`, `bool`, `f64`, `Uuid`, `DateTime<Utc>`
- `Option<T>` y `Vec<T>` de esos valen como tipo base
- Objetos valor enum: `field_type` es `"String"`, `variantes` no está vacío, todas las variantes en
  PascalCase
- `unico = true` no vale en campos con objeto valor `Option<T>` o `Vec<T>`
- Un campo con `referencia` no lleva `objeto_valor`

## Extensiones del registro de tipos (`generators/types.rs`)

```rust
is_value_object(field)                          // true si field.value_object es Some (de cualquier clase)
is_option_vo(field)                             // true si field_type empieza por "Option<"
is_vec_vo(field)                                // true si field_type empieza por "Vec<"
is_enum_vo(field)                               // true si value_object_kind == "enum"
is_shared_vo(field, shared_vos)                 // true si el nombre coincide con un [[objeto_valor]]
vo_inner_type(field)                            // el primitivo interno: "String" de Option<String>, Vec<String>
field_rust_type(field)                          // el tipo Rust en los structs: el VO, Option<VO>, Vec<VO> o el primitivo
field_vo_conversion(field, source, pascal, shared_vos) // primitivo → valor de dominio (`Nombre::new(params.nombre)?`)
field_value_accessor(field, owner)              // valor de dominio → primitivo (`entidad.nombre.valor().to_string()`)
props_literal(field) / seed_literal(field)      // un valor de test válido / un valor para la siembra de Postgres, con la forma del VO
field_ctx(field, pascal, shared_vos)            // FieldCtx: todo lo que una plantilla sabe de un campo (ADR-0011)
vo_import_path(field, snake, shared_vos)        // ruta del import: local o compartida
literal_import(field)                           // el import que necesita un literal (chrono::Utc, uuid::Uuid…)
literal_names_vo(field)                         // si un literal nombra el VO (false para los VO Option/Vec)
```

## Arquitectura: los objetos valor en cada capa

### Dominio: `valores.rs`

Un fichero por contexto en `contextos/<c>/dominio/src/valores.rs`. Los compartidos viven en el
crate `nucleo`: `nucleo/src/valores.rs`.

#### Objeto valor String (recorte + validación de vacío)

```rust
#[derive(Debug, Clone, PartialEq)]
pub struct Nombre {
    value: String,
}

impl Nombre {
    pub fn new(value: String) -> Result<Self, ErrorPersona> {
        let trimmed = value.trim().to_string();
        if trimmed.is_empty() {
            return Err(ErrorPersona::NombreInvalido);
        }
        Ok(Self { value: trimmed })
    }

    pub fn valor(&self) -> &str {
        &self.value
    }
}
```

#### Objeto valor numérico (`i64`, `f64`) — pasa sin validar

```rust
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Edad(i64);

impl Edad {
    pub fn new(value: i64) -> Result<Self, ErrorPersona> {
        Ok(Self(value))
    }

    pub fn valor(&self) -> i64 {
        self.0
    }
}
```

#### Objetos valor bool, Uuid, DateTime<Utc> — el mismo patrón de tuple struct

#### Objeto valor Option

```rust
// El struct del VO es idéntico al de la variante primitiva.
// El Option se pone en el modelo y en los props — Option<SegundoNombre>.
// Construcción: let segundo_nombre = params.segundo_nombre.map(SegundoNombre::new).transpose()?;
```

#### Objeto valor Vec

```rust
// El struct del VO es idéntico al de la variante primitiva.
// El Vec se pone en el modelo y en los props — Vec<Etiqueta>.
// Construcción: let etiquetas = params.etiquetas.into_iter().map(Etiqueta::new).collect::<Result<Vec<_>, _>>()?;
```

#### Objeto valor enum

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum Estado {
    Pendiente,
    Confirmado,
    Cancelado,
}

impl std::str::FromStr for Estado {
    type Err = ErrorPedido;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Pendiente" => Ok(Self::Pendiente),
            "Confirmado" => Ok(Self::Confirmado),
            "Cancelado" => Ok(Self::Cancelado),
            _ => Err(ErrorPedido::EstadoInvalido),
        }
    }
}

impl Estado {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Pendiente => "Pendiente",
            Self::Confirmado => "Confirmado",
            Self::Cancelado => "Cancelado",
        }
    }
}
```

**Reglas:**
- Valor interno privado (tuple struct para numéricos/bool/Uuid, campo con nombre para String,
  variantes para enum)
- `new()` devuelve siempre `Result<Self, Error<Entidad>>` — también los que no validan
- `valor()` devuelve `&str` (String), el primitivo por valor (numéricos/bool/Uuid), con propiedad
  (DateTime)
- Los objetos valor enum implementan `std::str::FromStr` (se construyen con `s.parse::<Estado>()`,
  sin import) y exponen `as_str()`, en lugar de `new()` / `valor()`. Un `from_str` inherente es lo
  que rechaza el `should_implement_trait` de clippy en el `make lint` del proyecto generado
- Sin `Serialize`/`Deserialize` en los objetos valor — son solo del dominio; los DTO usan primitivos
- Los compartidos tienen su propio tipo de error en `nucleo/src/errores.rs`

### Dominio: `errores.rs`

Cuando la entidad tiene campos con objeto valor, cada uno añade una variante `<Vo>Invalido`, con el
código `<entidad>.<vo en snake_case>_invalido`:

```rust
#[derive(Debug, Error)]
pub enum ErrorPersona {
    #[error("persona.validacion.{0}")]
    Validacion(String),
    #[error("persona.no_encontrado")]
    NoEncontrado,
    #[error("persona.duplicado")]
    Duplicado,
    #[error("persona.repositorio")]
    Repositorio,
    #[error("persona.desconocido")]
    Desconocido,
    // Objetos valor
    #[error("persona.nombre_invalido")]
    NombreInvalido,
    #[error("persona.estado_invalido")]
    EstadoInvalido,
}
```

`rest_<c>/errores.rs` responde 422 a cada una, con su test.

### Dominio: `modelo.rs`

Los campos con objeto valor usan el tipo del VO; los primitivos siguen siendo primitivos:

```rust
pub struct PropsPersona {
    pub nombre: Nombre,                        // VO
    pub segundo_nombre: Option<SegundoNombre>, // VO Option
    pub etiquetas: Vec<Etiqueta>,              // VO Vec
    pub estado: Estado,                        // VO enum
    pub activo: bool,                          // primitivo
}
```

Los campos con VO **no** se validan en `modelo::new()` — ya los validó `Vo::new()`. Los campos
`String` primitivos siguen con la comprobación de vacío.

### Dominio: `lib.rs` y `Cargo.toml`

`pub mod valores;` va en el `lib.rs` del dominio cuando el contexto tiene objetos valor locales.
Con objetos valor compartidos, `dominio_<c>` (y los crates que los nombran) dependen de `nucleo`, y
`nucleo/src/lib.rs` declara `pub mod errores; pub mod valores;`.

### Aplicación: los casos de uso

Los Params usan siempre **tipos primitivos**. La implementación construye los objetos valor antes
de `Persona::new()`:

```rust
pub struct ParamsCrearPersona {
    pub nombre: String,          // primitivo
    pub segundo_nombre: Option<String>,
    pub etiquetas: Vec<String>,
    pub estado: String,
    pub activo: bool,
}

async fn ejecutar(&self, params: ParamsCrearPersona) -> Result<Persona, ErrorPersona> {
    let nombre = Nombre::new(params.nombre)?;
    let segundo_nombre = params.segundo_nombre.map(SegundoNombre::new).transpose()?;
    let etiquetas = params.etiquetas.into_iter().map(Etiqueta::new).collect::<Result<Vec<_>, _>>()?;
    let estado = params.estado.parse::<Estado>()?;

    let entidad = Persona::new(PropsPersona { nombre, segundo_nombre, etiquetas, estado, activo: params.activo })?;
    self.repositorio.guardar(&entidad).await?;
    Ok(entidad)
}
```

**Los compartidos** usan `.map_err(|_| ErrorPersona::EmailInvalido)?`, porque su `new()` devuelve
el error de `nucleo` (`ErrorEmail::Invalido`, código `nucleo.email_invalido`).

### Infraestructura: `fila.rs`

`FilaPersona` usa **tipos primitivos** — la definición del struct no cambia. Las conversiones
reconstruyen los objetos valor:

```rust
// TryFrom: fila → dominio (reconstruye los VO)
impl TryFrom<FilaPersona> for Persona {
    type Error = ErrorPersona;

    fn try_from(fila: FilaPersona) -> Result<Self, ErrorPersona> {
        Ok(Self::from_repositorio(Persona {
            nombre: Nombre::new(fila.nombre)?,
            segundo_nombre: fila.segundo_nombre.map(SegundoNombre::new).transpose()?,
            etiquetas: fila.etiquetas.into_iter().map(Etiqueta::new).collect::<Result<Vec<_>, _>>()?,
            estado: fila.estado.parse::<Estado>()?,
            activo: fila.activo,
            // ... campos de sistema
        }))
    }
}

// From: dominio → fila (saca los primitivos)
impl From<&Persona> for FilaPersona {
    fn from(entidad: &Persona) -> Self {
        Self {
            nombre: entidad.nombre.valor().to_string(),
            segundo_nombre: entidad.segundo_nombre.as_ref().map(|v| v.valor().to_string()),
            etiquetas: entidad.etiquetas.iter().map(|v| v.valor().to_string()).collect(),
            estado: entidad.estado.as_str().to_string(),
            activo: entidad.activo,
            // ... campos de sistema
        }
    }
}
```

### REST: `dto.rs`

Los campos del DTO siguen siendo primitivos. `from_dominio()` saca `.valor()` / `.as_str()` de los
campos con VO:

```rust
impl DtoPersona {
    pub fn from_dominio(entidad: &Persona) -> Self {
        Self {
            id: entidad.id.valor(),
            nombre: entidad.nombre.valor().to_string(),
            segundo_nombre: entidad.segundo_nombre.as_ref().map(|v| v.valor().to_string()),
            etiquetas: entidad.etiquetas.iter().map(|v| v.valor().to_string()).collect(),
            estado: entidad.estado.as_str().to_string(),
            activo: entidad.activo,
        }
    }
}
```

### Dominio: la madre

Los campos con VO guardan el tipo del VO (no el primitivo). Los valores por defecto se envuelven en
`Vo::new(valor_por_defecto).expect(...)`:

```rust
pub struct MadrePersona {
    nombre: Option<Nombre>,
    segundo_nombre: Option<SegundoNombre>,
    etiquetas: Option<Vec<Etiqueta>>,
    estado: Option<Estado>,
    activo: Option<bool>,
}

impl MadrePersona {
    pub fn con_nombre(mut self, nombre: Nombre) -> Self { self.nombre = Some(nombre); self }
    pub fn con_estado(mut self, estado: Estado) -> Self { self.estado = Some(estado); self }

    pub fn construir(self) -> Persona {
        Persona::new(PropsPersona {
            nombre: self.nombre.unwrap_or_else(|| Nombre::new("example".to_string()).expect("Nombre::new rechazó el valor de ejemplo")),
            segundo_nombre: self.segundo_nombre,
            etiquetas: self.etiquetas.unwrap_or_default(),
            estado: self.estado.unwrap_or(Estado::Pendiente),   // la primera variante
            activo: self.activo.unwrap_or(true),
        }).expect("MadrePersona: …")
    }
}
```

### Objetos valor compartidos

Se declaran en el nivel superior de `puerto.toml` como secciones `[[objeto_valor]]`:

```toml
[[objeto_valor]]
nombre = "Email"
tipo = "String"
```

Se generan en `nucleo/src/valores.rs`. Cada uno tiene su propio tipo de error en
`nucleo/src/errores.rs` (`ErrorEmail::Invalido`, `"nucleo.email_invalido"`). Cuando el
`objeto_valor` de un campo coincide con un `[[objeto_valor]]`, el import pasa de local
(`crate::valores::Email`) a compartido (`nucleo::valores::Email`).

## Validación (`validation.rs`, `commands/validate.rs`)

- `objeto_valor`, si está, va en PascalCase
- Con `objeto_valor` y `clase_objeto_valor != "enum"`: `tipo` es un primitivo, `Option<primitivo>`
  o `Vec<primitivo>`
- Tipos internos admitidos: `String`, `i64`, `bool`, `f64`, `Uuid`, `DateTime<Utc>`, y sus
  `Option<*>` y `Vec<*>`
- `unico = true` no se combina con campos con VO `Option<T>` o `Vec<T>`
- Objetos valor enum: `tipo` es `"String"`, `variantes` no está vacío, todas en PascalCase
- `clase_objeto_valor` sin `objeto_valor` → error
- Compartidos: los nombres de `[[objeto_valor]]` van en PascalCase; `tipo` es un primitivo simple

## Ficheros que se generan

### Entidad con objetos valor locales

| Fichero | Acción |
|---------|--------|
| `contextos/<c>/dominio/src/valores.rs` | **NUEVO** — los structs de los VO |
| `contextos/<c>/dominio/src/modelo.rs` | **CAMBIA** — tipos VO en Props/Entidad |
| `contextos/<c>/dominio/src/errores.rs` | **CAMBIA** — variantes `<Vo>Invalido` |
| `contextos/<c>/dominio/src/lib.rs` | **CAMBIA** — `pub mod valores;` |
| `contextos/<c>/dominio/src/pruebas/madres.rs` | **CAMBIA** — tipos VO en los constructores |
| `contextos/<c>/aplicacion/src/crear_<c>.rs` | **CAMBIA** — construcción de los VO |
| `contextos/<c>/aplicacion/src/actualizar_<c>.rs` | **CAMBIA** — construcción de los VO |
| `contextos/<c>/infraestructura/src/fila.rs` | **CAMBIA** — TryFrom/From con VO (`--db`) |
| `contextos/<c>/rest/src/dto.rs` | **CAMBIA** — `.valor()` / `.as_str()` en `from_dominio()` |
| `contextos/<c>/rest/src/errores.rs` | **CAMBIA** — un 422 por variante `<Vo>Invalido` |

### Con objetos valor compartidos

| Fichero | Acción |
|---------|--------|
| `nucleo/src/valores.rs` | **NUEVO** — los structs de los VO compartidos |
| `nucleo/src/errores.rs` | **NUEVO** — sus tipos de error |
| `nucleo/src/lib.rs` | **CAMBIA** — `pub mod errores; pub mod valores;` |
| `contextos/<c>/*/Cargo.toml` | **CAMBIA** — `nucleo = { workspace = true }` donde se nombran |

## Alcance V1 — COMPLETO

- ✅ VO String con recorte y validación de vacío, variante de error `<Vo>Invalido`
- ✅ VO numéricos/bool/Uuid/DateTime que pasan sin validar
- ✅ Recorrido por todas las capas (dominio → aplicación → infraestructura → REST)
- ✅ Integración con la madre
- ✅ `puerto validar` comprueba las restricciones de los campos con VO
- ✅ Análisis en la CLI: `nombre:Nombre:String`, `sku:Sku:String!`

## Alcance V2 — COMPLETO

- ✅ `Option<VO>` (campos VO nullables): `segundo_nombre:SegundoNombre:opt:String`
- ✅ `Vec<VO>` (campos VO array): `etiquetas:Etiqueta:vec:String`
- ✅ VO enum: `estado:Estado:enum:Activo/Inactivo/Suspendido`
- ✅ VO compartidos (`[[objeto_valor]]` en `puerto.toml`, en el crate `nucleo`)

## Alcance V3 — EN CURSO

- ✅ **Deducción del tipo de un VO compartido**: si el VO de un campo coincide con un
  `[[objeto_valor]]` declarado, el tipo interno se deduce — `email:Email` en vez de
  `email:Email:String`

## Más adelante (V3+)

- ✅ Snippets de los patrones de VO (`puerto generar snippets`): `vo-string`, `vo-numeric`,
  `vo-enum`, `vo-option-construct`, `vo-vec-construct`
- Reglas de validación propias en la CLI (p. ej. un modificador `min`/`max` en
  `nombre:Nombre:String`) — el disparador para compartir el dominio con el frontend vía WASM
  (`PLAN.md` § Fase 10)
