# 0027 — La marca es un fichero del proyecto, sobre los temas de daisyUI

**Estado:** Aceptada (2026-11-18); el «un solo diseño» lo sustituye la ADR-0028. Especificación:
`PLAN.md` § Fase 25.

## Contexto

Un proyecto nuevo con `web/` se veía como una herramienta interna: un menú horizontal con las
entidades, el login dentro de ese menú, un «Inicio» que era una lista de enlaces y los colores por
defecto de daisyUI. Para que se vea como un producto le falta una marca (nombre, logo, colores) y
unas pantallas base (acceso, panel, perfil). Había que decidir dónde vive la marca y si Puerto
ofrece varias plantillas.

## Decisión

- **La marca son ficheros del usuario, no una sección de `puerto.toml`.** `web/src/marca/marca.ts`
  (nombre, eslogan, logo), `web/src/marca/tema.css` y `web/public/logo.svg`, escritos una vez por
  `puerto nuevo`. Los colores ya tienen un formato, CSS: llevarlos a TOML sería un traductor más
  que mantener, y el generador de temas de daisyUI da el CSS listo para pegar. Ningún generador los
  vuelve a escribir.
- **El tema sobrescribe `light` y `dark` de daisyUI**, en vez de declarar temas con nombre propio:
  la app sigue al sistema (`prefersdark`) sin código, y lo que la marca no fija sale del tema
  original. El `@plugin "daisyui"` vive en `tema.css`, que `index.css` importa: un `@import` detrás
  del `@plugin` lo rechaza Biome, y delante, los temas de daisyUI pisan los de la marca.
- **Un solo diseño**, no plantillas a elegir: cada plantilla más multiplica el compile matrix y se
  queda vieja por su lado; los 35 temas de daisyUI dan la variedad.
- **El panel cuenta con lo que ya hay**: el `total` de la primera página de cada lista
  (`GET /<plural>?limite=1`), sin endpoints nuevos. Cada entrada del menú lleva su función `total`,
  generada en `generado/rutas.tsx`: el cliente tipado no deja pedir una ruta que no conoce.
- **El perfil es de solo lectura**: lo que ya responde `GET /me`.

## Consecuencias

- Cambiar la marca es editar tres ficheros; `puerto generar bootstrap` no los toca.
- Los ajustes de marca en ejecución (un admin que cambia el logo, una marca por cliente) siguen
  fuera: pedirían subir ficheros y separar datos por cliente.
- Con auth, el panel pide sesión cuando alguna de las listas que resume la pide.
