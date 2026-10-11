# Frontend (`web/`) — Agent Instructions

React 19 + daisyUI 5 (Tailwind v4) on Bun. Routing with wouter, server state with `useAsync`,
native `<form>` and `<dialog>`. The backend's OpenAPI spec is the contract between the two halves.

## Layers

```
presentation → infrastructure → application → domain
```

| Directory | Holds | Never |
|---|---|---|
| `src/domain/<entity>/` | types, the error (`code` = the API's `ErrorResponse.name`), the repository port | imports an npm package or another layer |
| `src/application/` | use cases — **only when there is logic** (see its README) | imports an adapter or a screen |
| `src/infrastructure/` | adapters implementing the domain ports (HTTP) | imports a screen or a use case |
| `src/presentation/` | pages, containers (state + calls), views (props only), shared UI | imports `infrastructure/` (only `core/generated/dependencies.ts` does) |

`bun run lint:arch` (dependency-cruiser) fails on any import that breaks this.

Screens reach adapters through `useDependencies()`, never by importing them: a test renders a
screen inside `<DependenciesProvider value={{ ...fakes }}>` and needs no network.

## Generated files

- **Never hand-edit `src/presentation/core/generated/`** — `dependencies.ts` (the composition
  root) and `routes.tsx` (the route table) are rewritten from `puerto.toml` by
  `puerto generate scaffold` and `puerto generate bootstrap`.
- Everything else is yours once written.

## Conventions

- **No client-side copy of domain validation.** The server is the authority: show the error code
  it answers (`product.validation_error.name_empty`) with `ErrorAlert`. HTML `required` / `type`
  only.
- **TDD with `bun test`** (happy-dom + Testing Library). Test names say what the user sees:
  `should show the error code when the error carries one`, with `// Arrange` / `// Act` /
  `// Assert`.
- Styling is daisyUI classes (`btn`, `card`, `alert`, `modal`); no component library.

## Check before you are done

```bash
make web/test   # bun test
make web/lint   # tsc + Biome + dependency-cruiser
```

`make test` and `make lint` at the project root run these too.
