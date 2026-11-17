# Frontend (`web/`) — instrucciones para agentes

React 19 + daisyUI 5 (Tailwind v4) sobre Bun. Enrutado con wouter, estado del servidor con
`useAsync`, `<form>` y `<dialog>` nativos. La spec OpenAPI del backend es el contrato entre las dos
mitades: las pantallas usan sus tipos y su cliente, sin modelo propio.

## Qué hay en `src/`

El frontend es la capa de presentación: los contenedores llaman a la API y las vistas pintan lo que
reciben. No hay dominio, casos de uso ni adaptadores propios; las reglas viven en el backend.

| Directorio | Contiene |
|---|---|
| `api/` | `client.ts` (`createApiClient`, `ApiClient`, `Schemas`: los DTO por nombre), `http.ts` (`unwrap`, `ApiError`, `PAGE_SIZE`) y `schema.d.ts` (generado) |
| `features/<entity>/` | `containers/` (estado y llamadas a la API, con sus tests), `views/` (solo props) y `messages.ts` |
| `pages/` | las páginas que no son de una entidad (inicio, 404) |
| `shared/` | `forms.ts` (texto del formulario ↔ valores de la API), `use-async.ts` y `ui/` (componentes compartidos) |
| `generated/` | la raíz de composición y la tabla de rutas: **las reescribe Puerto** |
| `test-support/` | `renderScreen`, la API falsa y una madre por entidad: solo para los tests |

Un contenedor recibe el cliente con `useDependencies()` y lo desenvuelve con `unwrap`, que rechaza
con un `ApiError` cuyo `code` es el `RespuestaError.codigo` de la API:

```tsx
const { api } = useDependencies();
const producto = useAsync(
  () => unwrap(api.GET("/productos/{id}", { params: { path: { id } } })),
  [api, id],
);
```

Una vista recibe los DTO como props (`Schemas["DtoProducto"]`) y nunca llama a la API.
`bun run lint:arch` (dependency-cruiser) falla si una vista importa el cliente, `useDependencies`
o `useAsync`, si algo fuera de `generated/dependencies.ts` y `test-support/` construye el cliente,
o si el código de la app importa `test-support/`.

## Las pantallas de una entidad

`puerto generar scaffold <Nombre>` escribe, junto a las capas de Rust, las pantallas de una entidad
CRUD (`puerto generar web <Nombre>` las escribe solas, para una entidad que ya está en
`puerto.toml`):

```
src/features/<entity>/
  messages.ts                 código de la API → texto + campo del formulario
  containers/                 list · detail · form (crear + editar): estado y llamadas a la API (+ tests)
  views/                      list · detail · form: solo props
src/test-support/<entity>.ts  <Entity>Mother (request / random) + <entity>Collection para la API falsa
```

Los campos del formulario guardan texto; `src/shared/forms.ts` lo convierte a lo que manda la API
y de vuelta: un campo opcional vacío no se manda (`optionalText`…), y uno ausente llega como `null`
(`textOf`, `?? "—"`); listas separadas por comas, líneas `clave=valor`, `datetime-local` en hora
local. Un objeto valor enum es el `string` de la API: el `<select>` ofrece sus variantes.

Las listas van paginadas: el contenedor pide `{ limite: PAGE_SIZE, desplazamiento }`, guarda la
página en la URL (`?page=2`) y pinta `shared/ui/pagination.tsx` con lo que respondió la API
(`elementos`, `total`, `limite`, `desplazamiento`). La API limita el tamaño de página a 100, pida lo
que pida la pantalla.

Rutas: `/<plural>`, `/<plural>/new`, `/<plural>/:id`, `/<plural>/:id/edit`.

## Ficheros generados

- **Los mensajes de error** viven en `src/features/<entity>/messages.ts`, generado a partir de la
  lista de errores del backend: `toProblem(code)` responde el texto y el campo del formulario al que
  se refiere. El formulario lo muestra bajo ese campo (`FieldError`, `aria-invalid`), y cualquier
  otro en la alerta; un código desconocido se muestra tal cual. Están en inglés: ese fichero es el
  sitio donde traducirlos (un re-scaffold forzado lo reescribe).
- **Referencias** (`cliente_id:ref:Cliente`): el campo del formulario es un `<select>` con los 100
  primeros elementos del destino (`MAX_PAGE_LIMIT`), que carga el contenedor del formulario y que se
  etiquetan con su primer campo de texto obligatorio; el detalle enlaza con la página del destino
  (`shared/ui/reference-link.tsx`), con la etiqueta que carga su contenedor. Un destino que no existe
  responde `pedido.validacion.cliente_id_no_encontrado`, que se muestra bajo el select.
- **No edites nunca a mano `src/generated/`**: `dependencies.ts` (la raíz de composición) y
  `routes.tsx` (la tabla de rutas) se reescriben a partir de `puerto.toml` con
  `puerto generar scaffold`, `puerto generar web` y `puerto generar bootstrap`. Una pantalla propia
  lleva su ruta en **`src/routes.tsx`** (`customRoutes`, cada `<Route>` con su `key`): es tuyo, se
  comprueba antes que las rutas generadas y nunca se reescribe.
- **No edites nunca a mano `src/api/schema.d.ts`**: son los tipos de red, generados por
  `openapi-typescript` a partir de `../docs/api/openapi.json`. **`make web/sync`** (en la raíz del
  proyecto) vuelve a exportar la spec desde el código Rust, los regenera y formatea `web/` con
  Biome; ejecútalo tras un scaffold y tras cualquier cambio de DTO o de ruta. Hasta entonces
  `bun run typecheck` falla en la pantalla que ya no coincide con la API.
- Todo lo demás es tuyo una vez escrito, incluidos `src/api/client.ts` y
  `src/test-support/fake-api.ts`, que se escriben con la primera entidad.

## Convenciones

- **Nada de copiar en el cliente la validación del dominio.** La autoridad es el servidor: muestra
  el código de error que responde (`producto.validacion.nombre_vacio`) con `ErrorAlert`. En HTML,
  solo `required` / `type`.
- **TDD con `bun test`** (happy-dom + Testing Library). Un test de pantalla la renderiza con
  `renderScreen(<Pantalla />, { api: fakeApi([productoCollection([...])]) })`: la API falsa responde
  como la generada (páginas, `404` con `<entidad>.no_encontrado`, `201`/`200`/`204`), y su segundo
  argumento (`{ "POST /productos": "producto.duplicado" }`) responde ese código. Los nombres de test
  dicen lo que ve el usuario y, como el resto del frontend, siguen en inglés:
  `should show the error code when the error carries one`, con `// Arrange` / `// Act` /
  `// Assert`.
- Los estilos son clases de daisyUI (`btn`, `card`, `alert`, `modal`); sin librería de componentes.

## Comprueba antes de terminar

```bash
make web/test   # bun test
make web/lint   # tsc + Biome + dependency-cruiser
```

`make test` y `make lint` en la raíz del proyecto también los ejecutan.
