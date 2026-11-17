# 0025 — El frontend generado es presentación: contenedores sobre el cliente tipado

**Estado:** Aceptada (2026-11-17). Especificación: `PLAN.md` § Fase 21. Sustituye la decisión «No
layer without logic» de la ADR-0013 (las capas `domain/`, `application/` e `infrastructure/` del
frontend); el resto de la ADR-0013 sigue en pie.

## Contexto

La ADR-0013 dio al frontend la Regla de Dependencias del backend: `domain/` (el modelo, el error y
el puerto del repositorio de cada entidad), `infrastructure/` (el adaptador HTTP), `application/`
(vacío) y `presentation/`. Leído contra lo que genera (snapshot `web_every_type`, 2026-11-17):

- `model.ts` es el DTO de `schema.d.ts` copiado a mano, con `undefined` cambiado por `null` y el
  enum estrechado de `string` a una unión.
- El adaptador HTTP son ~90 líneas por entidad que traducen esa copia en los dos sentidos y
  convierten la respuesta de error en un `<Entidad>Error` por entidad.
- Cada puerto tiene una implementación, la HTTP; la otra es el falso de los tests.
- `application/` es un README.

Unas 270 líneas por entidad que no hacen trabajo propio. Lo único que compraba el puerto —probar
las pantallas sin red, con un falso— lo da también `openapi-fetch`, que acepta un `fetch` propio.

## Decisión

- **El frontend es la capa de presentación.** Las pantallas trabajan con los tipos de
  `schema.d.ts` (`Schemas["Dto<Entidad>"]`, `Schemas["PeticionCrear<Entidad>"]`), sin modelo
  propio. Un contenedor llama al cliente tipado (`api.GET("/productos/{id}", …)`) por
  `useDependencies()` y lo desenvuelve con `unwrap`, que rechaza con un único `ApiError(code)`.
  Una vista solo recibe props.
- **Sin openapi-generator.** Pide la JVM, genera clientes de clases y sería un segundo generador
  junto a Puerto. Lo útil de ese mundo ya está: `openapi-typescript` (los tipos) y `openapi-fetch`
  (el cliente). Tampoco `openapi-react-query`: mete TanStack Query, y `useAsync` basta.
- **Los tests de las pantallas hablan HTTP con un falso.** `fakeApi` (`test-support/fake-api.ts`)
  es un `ApiClient` sobre colecciones en memoria que responde como la API generada (páginas, 404
  con `<entidad>.no_encontrado`, 201/200/204) y, para los caminos de error, el código que se le
  pida. Uno solo para todas las entidades, en vez de un repositorio falso por entidad; los tests
  ejercitan las rutas y los parámetros reales de la API.
- **El contenedor/vista es la única frontera**, y dependency-cruiser la vigila: una vista no
  importa el cliente, `useDependencies` ni `useAsync` (sí los tipos), y solo
  `generated/dependencies.ts` y `test-support/` construyen el cliente.
- **Una carpeta por lo que es, no por capa:** `api/`, `features/<entidad>/`, `pages/`, `shared/`,
  `auth/`, `generated/` (lo que Puerto reescribe) y `test-support/`. `core/` y `presentation/`
  desaparecen: si todo es presentación, sobran.
- **Auth conserva su puerto** (`AuthGateway`): tiene tres implementaciones (OIDC, local y el falso
  de los tests de pantallas). Solo cambia de sitio, a `auth/`.

## Consecuencias

- Una entidad pasa de 16 ficheros de frontend a 11; las plantillas `model`, `errors`,
  `repository`, `repository_http` y su test desaparecen, y con ellas el estrechamiento de los enums
  (un enum es un `string` y el `<select>` ofrece las variantes de `puerto.toml`).
- Un `Option` del backend es una propiedad opcional en los tipos y puede llegar `null` (poem lo
  serializa así): las expresiones generadas lo tratan con `??` y `== null`, y el formulario manda
  `undefined` para un campo vacío.
- Un cambio de DTO sigue fallando `bun run typecheck` donde la pantalla ya no encaja, ahora en la
  pantalla misma y no en un adaptador.
- La lógica de cliente que aparezca (una regla, una orquestación) se escribe a mano donde haga
  falta. Si la Fase 10 (el dominio de Rust compilado a WASM) arranca algún día, traerá de vuelta un
  `domain/`, con contenido.
- Rompe la forma del frontend generado; las pantallas en castellano son la fase siguiente.
