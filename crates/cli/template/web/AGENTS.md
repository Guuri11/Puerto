# Frontend (`web/`) — instrucciones para agentes

React 19 + daisyUI 5 (Tailwind v4) sobre Bun. Enrutado con wouter, estado del servidor con
`useAsync`, `<form>` y `<dialog>` nativos. La spec OpenAPI del backend es el contrato entre las dos
mitades.

## Capas

```
presentation → infrastructure → application → domain
```

| Directorio | Contiene | Nunca |
|---|---|---|
| `src/domain/<entity>/` | los tipos, el error (`code` = el `RespuestaError.codigo` de la API), los mensajes de esos códigos, el puerto del repositorio | importa un paquete de npm u otra capa |
| `src/application/` | casos de uso, **solo cuando hay lógica** (ver su README) | importa un adaptador o una pantalla |
| `src/infrastructure/` | los adaptadores que implementan los puertos del dominio (HTTP) | importa una pantalla o un caso de uso |
| `src/presentation/` | páginas, contenedores (estado + llamadas), vistas (solo props), UI compartida | importa `infrastructure/` (solo lo hace `core/generated/dependencies.ts`) |

`bun run lint:arch` (dependency-cruiser) falla con cualquier import que rompa esto.

Las pantallas llegan a los adaptadores con `useDependencies()`, nunca importándolos: un test
renderiza una pantalla con `renderScreen(<Screen />, { productRepository: fake })`
(`src/test-support/render.tsx`) y no necesita red.

## Las pantallas de una entidad

`puerto generar scaffold <Nombre>` escribe, junto a las capas de Rust, las pantallas de una entidad
CRUD (`puerto generar web <Nombre>` las escribe solas, para una entidad que ya está en
`puerto.toml`):

```
src/domain/<entity>/            model.ts · errors.ts · messages.ts · repository.ts (el puerto)
src/infrastructure/<entity>/    <entity>-repository-http.ts (+ test): DTO ↔ modelo, RespuestaError → error
src/presentation/features/<entity>/
  containers/                   list · detail · form (crear + editar): estado y llamadas al repositorio (+ tests)
  views/                        list · detail · form: solo props
src/test-support/<entity>.ts    <Entity>Mother + fake<Entity>Repository (en memoria)
```

Los campos del formulario guardan texto; `src/presentation/shared/forms.ts` lo convierte al modelo
y de vuelta (`null` para un campo opcional vacío, listas separadas por comas, líneas `clave=valor`,
`datetime-local` en hora local). Un objeto valor enum es un tipo unión con una función `parse…` en
`model.ts`.

Las listas van paginadas: el `list(page)` del puerto responde un `Page<T>`
(`src/domain/shared/page.ts`: `items`, `total`, el `limit` que aplicó la API, `offset`); el
contenedor de la lista guarda el `offset` y renderiza `shared/ui/pagination.tsx`. La API limita el
tamaño de página a 100, pida lo que pida la pantalla.

Rutas: `/<plural>`, `/<plural>/new`, `/<plural>/:id`, `/<plural>/:id/edit`. Los contenedores llaman
directamente al puerto del repositorio; escribe un caso de uso en `src/application/` cuando haya
lógica que guardar. `src/test-support/` es para los tests: nunca llega a un adaptador, y solo lo
importan ficheros de test.

## Ficheros generados

- **Los mensajes de error** viven en `src/domain/<entity>/messages.ts`, generado a partir de la
  lista de errores del backend: `toProblem(code)` responde el texto y el campo del formulario al que
  se refiere. El formulario lo muestra bajo ese campo (`FieldError`, `aria-invalid`), y cualquier
  otro en la alerta; un código desconocido se muestra tal cual. Están en inglés: ese fichero es el
  sitio donde traducirlos (un re-scaffold forzado lo reescribe).
- **Referencias** (`cliente_id:ref:Cliente`): el campo del formulario es un `<select>` con los 100
  primeros elementos del destino (`MAX_PAGE_LIMIT`), que carga el contenedor del formulario a través
  del repositorio del destino y que se etiquetan con su primer campo de texto obligatorio; el
  detalle enlaza con la página del destino (`shared/ui/reference-link.tsx`). Un destino que no
  existe responde `pedido.validacion.cliente_id_no_encontrado`, que se muestra bajo el select.
- **No edites nunca a mano `src/presentation/core/generated/`**: `dependencies.ts` (la raíz de
  composición) y `routes.tsx` (la tabla de rutas) se reescriben a partir de `puerto.toml` con
  `puerto generar scaffold`, `puerto generar web` y `puerto generar bootstrap`. Una pantalla propia
  lleva su ruta en **`src/presentation/core/routes.tsx`** (`customRoutes`, cada `<Route>` con su
  `key`): es tuyo, se comprueba antes que las rutas generadas y nunca se reescribe.
- **No edites nunca a mano `src/infrastructure/http/schema.d.ts`**: son los tipos de red, generados
  por `openapi-typescript` a partir de `../docs/api/openapi.json`. **`make web/sync`** (en la raíz
  del proyecto) vuelve a exportar la spec desde el código Rust, los regenera y formatea `web/` con
  Biome; ejecútalo tras un scaffold y tras cualquier cambio de DTO o de ruta. Hasta entonces
  `bun run typecheck` falla donde el frontend ya no coincide con la API.
- Todo lo demás es tuyo una vez escrito, incluido `src/infrastructure/http/client.ts`, que se
  escribe con la primera entidad.

## Convenciones

- **Nada de copiar en el cliente la validación del dominio.** La autoridad es el servidor: muestra
  el código de error que responde (`producto.validacion.nombre_vacio`) con `ErrorAlert`. En HTML,
  solo `required` / `type`.
- **TDD con `bun test`** (happy-dom + Testing Library). Los nombres de test dicen lo que ve el
  usuario y, como el resto del frontend, siguen en inglés:
  `should show the error code when the error carries one`, con `// Arrange` / `// Act` /
  `// Assert`.
- Los estilos son clases de daisyUI (`btn`, `card`, `alert`, `modal`); sin librería de componentes.

## Comprueba antes de terminar

```bash
make web/test   # bun test
make web/lint   # tsc + Biome + dependency-cruiser
```

`make test` y `make lint` en la raíz del proyecto también los ejecutan.
