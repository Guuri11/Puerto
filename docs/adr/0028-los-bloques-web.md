# 0028 — Los bloques web: variantes de un hueco, instaladas por la CLI

**Estado:** Aceptada (2026-11-19). Sustituye el «un solo diseño» de la ADR-0027. Especificación:
`PLAN.md` § Fase 26.

## Contexto

La Fase 25 dio a `web/` una marca y unas pantallas base, con un solo diseño: cada plantilla más
multiplicaba el compile matrix y se quedaba vieja por su lado. Sergio quiere lo contrario de un solo
diseño: un escaparate al estilo de shadcnblocks (logins, paneles, perfiles, secciones de marketing,
temas) del que cada proyecto elige lo que quiere, y una portada pública para los proyectos que son
un SaaS. Había que decidir qué es un bloque, cómo llega al proyecto, cómo no se queda viejo y dónde
vive la portada.

## Decisión

- **Un bloque es una variante de un hueco.** Un hueco es un fichero del proyecto con un export y
  unas props fijos (`paginas/panel.tsx` → `PaginaPanel({ entradas })`), y sus variantes cumplen ese
  contrato y los textos que buscan los tests de la plantilla. Instalar una cambia ese fichero y nada
  más: ni lo generado, ni los tests, ni `puerto.toml`.
- **Los instala la CLI**: `puerto generar web-bloque <hueco>/<variante>`. Van dentro del binario,
  en la misma versión que lo que generan, y la CLI sabe si el proyecto tiene auth o portada. Un
  registro de shadcn pediría `components.json` y la CLI de shadcn en un proyecto con daisyUI, y no
  sabría nada del proyecto. Un fichero cambiado a mano se protege con la guarda de `scaffold`.
- **La variante por defecto es el fichero de la plantilla**, no una copia: no hay dos versiones que
  mantener de lo que escribe `puerto nuevo`.
- **Lo que temía la ADR-0027 tiene un tope.** Un escenario del compile matrix instala todas las
  variantes que no son la de por defecto (que ya prueban los demás), así que el matrix crece en uno o
  dos escenarios, no en uno por plantilla. El escaparate se construye instalando cada bloque con la
  CLI: lo que se ve en la web es lo que se instala, y no se puede desfasar.
- **Los temas son bloques**: el hueco `tema` es `marca/tema.css`.
- **La portada se elige en `puerto nuevo`** (Sergio): si el proyecto es un SaaS con portada pública,
  la portada va en `/` y la app debajo de `ruta_app` (`/panel` por defecto), montada con el `nest` de
  wouter: dentro, las rutas son relativas y ninguna pantalla cambia. `portada` y `ruta_app` van en
  `puerto.toml` porque los necesitan lo generado (`rutaApp` en `generado/rutas.tsx`) y los bloques
  que se instalen después.

## Consecuencias

- Una variante nueva es un fichero en `templates/web/bloques/` y una línea del catálogo; el
  escenario de los bloques y el escaparate la recogen solos.
- Una variante que cambia el contrato de su hueco (otras props, otros textos) es un hueco nuevo, no
  una variante.
- La portada la pinta el navegador (es una SPA): si el SEO importa, la portada va en una web
  estática aparte, y el proyecto se crea sin ella.
