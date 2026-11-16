# Guía de tests (TDD)

## Principio

**Los tests se escriben PRIMERO, antes de cualquier implementación. Siempre.**

El TDD no es opcional. Es el flujo de trabajo. Los tests definen el contrato; la implementación lo
cumple.

---

## El ciclo

```
ROJO      → Escribe un test que falla y describe un requisito de negocio
VERDE     → Escribe el código mínimo que lo hace pasar
REFACTOR  → Mejora la calidad sin cambiar el comportamiento (ejecuta los tests tras cada cambio)
```

Repite. Nunca te saltes el ROJO.

---

## Nombres de los tests

**Patrón (código generado):** `deberia_<EXPECTATIVA_DE_NEGOCIO>_cuando_<ESCENARIO_DE_NEGOCIO>`

Los nombres describen **comportamiento de negocio**, no detalles de implementación.

```rust
// Bien — centrado en el negocio
deberia_crear_saludo_cuando_el_nombre_es_valido
deberia_rechazar_saludo_cuando_el_nombre_esta_vacio
deberia_devolver_el_saludo_guardado_cuando_esta_en_el_repositorio

// Mal — centrado en la implementación
deberia_devolver_ok
deberia_llamar_al_repositorio_una_vez
deberia_rellenar_el_campo_mensaje
```

Los tests de Puerto (`crates/cli/src/tests/`, en inglés como el resto de su código) siguen nombrando
por comportamiento: `use_case_generator_creates_trait_file`.

---

## Dónde van los tests

Los tests viven **dentro del fichero que prueban**, en un módulo `#[cfg(test)]`:

```rust
// contextos/saludo/aplicacion/src/obtener_saludo.rs
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn deberia_crear_saludo_cuando_el_nombre_es_valido() {
        // ...
    }
}
```

Las excepciones, también por crate: las madres en `dominio_<c>::pruebas::madres` y la siembra de
los tests de Postgres en `infra_<c>::pruebas::siembra`, públicas tras la feature `pruebas` para
los tests de otros crates.

---

## Patrón AAA (Preparar–Actuar–Comprobar)

Siempre con comentarios explícitos. Comunican la intención a personas y agentes.

```rust
#[tokio::test]
async fn deberia_devolver_error_cuando_el_nombre_esta_vacio() {
    // Preparar
    let mut repositorio_doble = MockRepositorioSaludo::new();
    repositorio_doble.expect_buscar_por_nombre().times(0);

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
```

---

## Dobles

El doble de un trait va **junto al trait**, en el mismo fichero, tras una feature (ADR-0023 § 5):

```rust
#[cfg(any(test, feature = "pruebas"))]
pub mod dobles {
    use mockall::mock;
    use super::*;

    mock! {
        pub RepositorioSaludo {}
        #[async_trait]
        impl RepositorioSaludo for RepositorioSaludo {
            async fn buscar_por_nombre(&self, nombre: &str) -> Result<Option<Saludo>, ErrorRepositorio>;
        }
    }
}
```

Se importa por su ruta, sin reexports:

```rust
use dominio_saludo::repositorio::dobles::MockRepositorioSaludo;   // el repositorio, en el dominio
use aplicacion_saludo::obtener_saludo::dobles::MockObtenerSaludo; // un caso de uso, en la aplicación
```

Cada crate que tiene dobles declara `mockall` como dependencia opcional (feature `pruebas`) **y**
como dev-dependency — así sus propios tests funcionan sin la feature. Quien los usa solo en tests
los declara en `[dev-dependencies]` con `features = ["pruebas"]`.

---

## Categorías de tests (por prioridad)

| Prioridad | Categoría | Descripción |
|-----------|-----------|-------------|
| CRÍTICA | Regla de negocio | Invariantes del dominio — escríbelos siempre |
| ALTA | Caso límite | Condiciones de borde: cadenas vacías, límites, transiciones de estado |
| ALTA | Regresión | Un test por bug corregido — evita que vuelva |
| MEDIA | Integración | Compilación + pila completa (ver `make test/full`) |

---

## Qué sí y qué no

### Sí
- Escribe los tests antes que la implementación (ROJO primero)
- Prueba comportamiento de negocio, no estado interno
- Usa datos realistas (`"Ana"`, no `"test"`)
- Compara con el código de error (`"saludo.no_encontrado"`)
- Cubre el camino feliz Y los casos límite
- Usa `#[tokio::test]` en todos los tests asíncronos

### No
- Probar detalles de implementación (métodos privados, cuántas veces se llama a un doble salvo que
  tenga significado)
- Escribir tests solo por la cobertura
- Doblarlo todo — mejor objetos de dominio reales cuando se pueda
- Hacer que los tests dependan del orden de ejecución
- Usar `unwrap()` en las aserciones — usa `assert!` / `assert_eq!` / `match` explícitos

---

## Ejecutar los tests

En Puerto:

```bash
make test        # Rápido — tests estructurales (estructura de lo generado, salida de cargo-generate, snapshots)
make test/full   # Lento — genera proyectos reales, ejecuta su clippy (-D warnings) y sus tests, y los
                 #         checks del frontend generado (necesita `bun` en el PATH)
make test/db     # Lento — un proyecto --db generado contra un Postgres + Mailpit desechables (Docker)
make test/auth   # Lento — el adaptador OIDC + un proyecto OIDC generado contra Keycloak + Postgres
make test/mutation  # Lento — cargo-mutants sobre la lógica pura de Puerto (validation, migrations, types, patchers)
```

En un proyecto generado, por capa (un glob de paquetes cada uno):

```bash
make test/dominio          # dominio_*
make test/aplicacion       # aplicacion_*
make test/infraestructura  # infra_* (con --db, arranca la base de datos)
```

Un solo test con su salida:

```bash
cargo test deberia_crear_saludo_cuando_el_nombre_es_valido -- --nocapture
```
