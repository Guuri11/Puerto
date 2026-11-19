# Convenciones y patrones de tests

> Complementa `testing.md` con patrones concretos para los datos de prueba, las madres y los
> dobles de los proyectos generados.

---

## Object Mother (madres)

Las madres son factorías que crean objetos de dominio de prueba con valores por defecto válidos.
Quitan repetición y dejan los tests centrados en el escenario, no en preparar datos.

### Dónde van (en los proyectos generados)

En el crate de dominio dueño del tipo, tras la feature `pruebas` (ADR-0023 § 5):

```
contextos/producto/dominio/src/
├── lib.rs              # #[cfg(any(test, feature = "pruebas"))] pub mod pruebas { pub mod madres; }
└── pruebas/
    └── madres.rs       # MadreProducto
```

Se importan por su ruta: `dominio_producto::pruebas::madres::MadreProducto`. La madre de
`Principal` es del runtime: `puerto_nucleo::auth::dobles::MadrePrincipal` (feature `pruebas` de
`puerto-nucleo`).

### API

```rust
// Lo más habitual — una entidad válida
let producto = MadreProducto::aleatorio();

// Un campo concreto — cadena de constructor
let producto = MadreProducto::new()
    .con_nombre("Ana")
    .construir();

// Solo los props — para probar el constructor
let props = MadreProducto::new()
    .con_nombre_vacio()
    .construir_props();

// Una colección
let productos = MadreProducto::aleatorios(5);
```

### Lo que se genera

```rust
pub struct MadreProducto {
    nombre: Option<String>,
}

impl MadreProducto {
    pub fn new() -> Self {
        Self { nombre: None }
    }

    // Constructores por campo
    pub fn con_nombre(mut self, nombre: &str) -> Self {
        self.nombre = Some(nombre.to_string());
        self
    }

    pub fn con_nombre_vacio(mut self) -> Self {
        self.nombre = Some(String::new());
        self
    }

    // Construcción
    pub fn construir(self) -> Producto {
        Producto::new(PropsProducto {
            nombre: self.nombre.unwrap_or_else(|| "example".to_string()),
        })
        .expect("MadreProducto: Producto::new rechazó los props")
    }

    pub fn construir_props(self) -> PropsProducto {
        // Devuelve los props sin construir la entidad — útil para probar new()
        ...
    }

    // Atajos
    pub fn aleatorio() -> Producto {
        Self::new().construir()
    }

    pub fn aleatorios(n: usize) -> Vec<Producto> {
        (0..n).map(|_| Self::aleatorio()).collect()
    }
}

impl Default for MadreProducto { /* Self::new() */ }
```

### Campos con tipo (`[[entidad.campos]]`)

La madre genera un constructor por **cada campo propio**:

```rust
pub struct MadreProducto {
    nombre: Option<String>,
    precio: Option<i64>,
    descripcion: Option<String>,     // un Option<String>: sin fijar, None
    etiquetas: Option<Vec<String>>,
    cliente_id: Option<IdCliente>,
}

impl MadreProducto {
    pub fn con_nombre(mut self, nombre: &str) -> Self { self.nombre = Some(nombre.to_string()); self }
    pub fn con_precio(mut self, precio: i64) -> Self { self.precio = Some(precio); self }
    pub fn con_nombre_vacio(mut self) -> Self { self.nombre = Some(String::new()); self }
    // ...
}
```

- Todo campo tiene `.con_<campo>(valor)` (los `String` y `Option<String>`, con `&str`)
- Los `String` obligatorios tienen además `.con_<campo>_vacio()`
- Los valores por defecto salen del registro de tipos (`i64` → `42`, `String` → `"example"`); una
  referencia, `Id<Objetivo>::generar()`
- Un campo con objeto valor guarda el objeto valor (ver `value_objects.md`)

---

## Nombres

| Método | Devuelve | Para qué |
|--------|----------|----------|
| `aleatorio()` | `Entidad` | Entidad válida |
| `aleatorios(n)` | `Vec<Entidad>` | Varias entidades válidas |
| `.con_<campo>(valor)` | `Self` | Constructor: fija un campo |
| `.con_<campo>_vacio()` | `Self` | Constructor: campo vacío (escenarios de error) |
| `.construir()` | `Entidad` | Crea la entidad (hace panic si es inválida — a propósito en tests) |
| `.construir_props()` | `PropsEntidad` | Crea los props sin construir la entidad |
| `<campo>_unico()` | el primitivo | Campos `unico`: un valor nuevo por llamada (12 hexadecimales en un `String`), el que usan las siembras de los tests de los repositorios |

---

## Dobles

### El `Cargo.toml` de un crate con dobles

`mockall` va **a la vez** en `[dependencies]` (opcional) y en `[dev-dependencies]`:

```toml
# contextos/producto/dominio/Cargo.toml
[dependencies]
mockall = { workspace = true, optional = true }

[features]
# Los dobles (`mock!`) y la madre, para los tests de otros crates (ADR-0023 § 5 de Puerto).
pruebas = ["dep:mockall"]

[dev-dependencies]
mockall = { workspace = true }   # para los tests del propio crate
```

`aplicacion_<c>` hace lo mismo (`pruebas = ["dep:mockall", "dominio_<c>/pruebas"]`). Quien usa los
dobles de otro crate solo en tests lo declara en `[dev-dependencies]` con
`features = ["pruebas"]`: `rest_<c>` los de `aplicacion_<c>` y `dominio_<c>`; `api` los de cada
`aplicacion_*` (el test que exporta la especificación OpenAPI).

### La puerta del doble, junto a su trait

```rust
// contextos/producto/dominio/src/repositorio.rs
#[cfg(any(test, feature = "pruebas"))]
pub mod dobles {
    use mockall::mock;
    use super::*;

    mock! {
        pub RepositorioProducto {}

        #[async_trait]
        impl RepositorioProducto for RepositorioProducto {
            async fn buscar_por_id(&self, id: IdProducto) -> Result<Option<Producto>, ErrorRepositorio>;
        }
    }
}
```

`#[cfg(any(test, feature = "pruebas"))]` significa:
- Disponible en todos los tests del propio crate (por `#[cfg(test)]`)
- Disponible para otros crates con `features = ["pruebas"]`

Cada caso de uso lleva el suyo en su fichero (`aplicacion_producto::crear_producto::dobles::MockCrearProducto`).
En proyectos con eventos, `unit_of_work.rs` trae un doble escrito a mano:
`dobles::UnitOfWorkFactoryGrabadora` (`fallando_en(FallarEn::…, …)`, `grabado()`).

---

## Ejemplo completo

```rust
// contextos/saludo/aplicacion/src/obtener_saludo.rs

#[cfg(test)]
mod tests {
    use super::*;
    use dominio_saludo::repositorio::dobles::MockRepositorioSaludo;

    #[tokio::test]
    async fn deberia_devolver_el_saludo_cuando_el_nombre_es_valido() {
        // Preparar
        let mut repositorio_doble = MockRepositorioSaludo::new();
        repositorio_doble.expect_buscar_por_nombre().returning(|_| Ok(None));

        let caso_de_uso = ObtenerSaludoImpl {
            repositorio: Arc::new(repositorio_doble),
        };

        // Actuar
        let resultado = caso_de_uso
            .ejecutar(ParamsObtenerSaludo { nombre: "Mundo".into() })
            .await;

        // Comprobar
        assert!(resultado.is_ok());
        let saludo = resultado.unwrap();
        assert_eq!(saludo.nombre, "Mundo");
        assert!(saludo.mensaje.contains("Mundo"));
    }

    #[tokio::test]
    async fn deberia_devolver_error_cuando_el_nombre_esta_vacio() {
        // Preparar
        let repositorio_doble = MockRepositorioSaludo::new(); // sin expectativas
        let caso_de_uso = ObtenerSaludoImpl {
            repositorio: Arc::new(repositorio_doble),
        };

        // Actuar
        let resultado = caso_de_uso
            .ejecutar(ParamsObtenerSaludo { nombre: "".into() })
            .await;

        // Comprobar
        assert!(resultado.is_err());
        assert_eq!(
            resultado.unwrap_err().to_string(),
            "saludo.validacion.nombre_vacio"
        );
    }
}
```

---

## Dónde está cada test

```
contextos/producto/
├── dominio/src/
│   ├── modelo.rs            # tests del constructor y las reglas de negocio
│   ├── errores.rs           # tests de la conversión desde ErrorRepositorio
│   └── pruebas/madres.rs    # la madre (sin tests propios)
├── aplicacion/src/
│   └── crear_producto.rs    # tests del caso de uso (dobles + AAA)
├── infraestructura/src/
│   ├── repositorio.rs       # en memoria: tests; Postgres: #[sqlx::test(migrator = "migrador::MIGRADOR")]
│   └── pruebas/siembra.rs   # construir / sembrar (--db)
└── rest/src/
    ├── errores.rs           # un test por rama de a_respuesta_error
    └── rutas.rs             # tests de rutas: TestClient + un caso de uso doblado por test
```

Los tests de integración (clippy + compilación + tests de un proyecto entero) viven en
`crates/cli/src/tests/compile_matrix.rs` como tests `#[ignore]` que ejecuta `make test/full` (y
`make test/db` para el escenario de Postgres).

---

## Dobles, falsos y stubs

| Concepto | Qué es | Cuándo |
|----------|--------|--------|
| **Stub** | Devuelve una respuesta fija; no comprueba las llamadas | Un camino feliz donde la respuesta del colaborador da igual |
| **Mock** | Registra llamadas; tiene expectativas programables (`mockall`) | Comprobar que se llamó a un colaborador con ciertos argumentos, o probar los errores que devuelve |
| **Falso** | Una implementación ligera que funciona (`RepositorioProductoEnMemoria`, `UnitOfWorkFactoryGrabadora`) | Tests entre capas donde quieres comportamiento real sin base de datos |

**Regla:** stubs para caminos felices, mocks para caminos de error y aserciones de interacción,
falsos para tests de integración. Nunca dobles lo que es tuyo (los modelos de dominio).

---

## Datos aleatorios con el crate `fake` (opcional)

Puerto no lo añade; si quieres datos realistas en una madre, en el `Cargo.toml` de su dominio:

```toml
[dev-dependencies]
fake = "2"
```

```rust
use fake::{Fake, faker::name::en::Name as NombreFalso, faker::lorem::en::Word};

impl MadreProducto {
    pub fn aleatorio() -> Producto {
        Self::new()
            .con_nombre(&NombreFalso().fake::<String>())
            .con_sku(&Word().fake::<String>())
            .construir()
    }
}
```

Los datos aleatorios encuentran errores que esconden las cadenas fijas como `"test"`: recortes,
límites de longitud, codificación. Una madre usada por otros crates fuera de `cfg(test)` necesita
`fake` en `[dependencies]` tras la feature `pruebas`.

---

## Constantes de test

Nombra los valores límite importantes como constantes en lugar de repetir literales:

```rust
#[cfg(test)]
mod tests {
    const NOMBRE_VALIDO: &str = "Ana";
    const VACIO: &str = "";
    const UUID_VALIDO: &str = "550e8400-e29b-41d4-a716-446655440000";

    // ...
    let resultado = caso_de_uso.ejecutar(ParamsCrearProducto { nombre: VACIO.into() }).await;
    assert!(resultado.is_err());
}
```
