# DESIGN.md — Puerto

El contrato visual. Escrito el 2026-10-13 con `/design-system`, antes de la primera línea.

## 1. Atmósfera

Un puerto de contenedores a primera hora: hormigón, rotulación pintada, cada caja en su
sitio y con su etiqueta. Quien llega tiene que pensar «esto está ordenado por alguien que
sabe lo que hace», no «esto es otra web oscura de Rust».

La idea que lo sostiene: **el contenedor estándar (ISO 668) hizo rápido el comercio porque
todas las cajas son iguales.** Puerto hace eso con el código: todos los proyectos tienen la
misma forma, así que cualquiera, persona o agente, se mueve por ellos desde el primer día.
*Convention over configuration* contado como estandarización portuaria.

| Puerto | En el muelle |
|---|---|
| `puerto.toml` | el manifiesto de carga |
| `business` / `infrastructure` / `presentation` | tres contenedores apilados, cada uno de su color |
| `puerto generate scaffold Product` | una carga con su etiqueta: ficheros, capas |
| outbox + worker | la cola de descarga |

**Lo que no se hace:** anclas, olas, faros, timones, gaviotas, cuerdas. Nada náutico de
souvenir. Esto es un puerto de mercancías: grúas, hormigón, rotulación y orden.

## 2. Paleta

Contraste medido el 2026-10-13 (WCAG 2.1, script en el historial de la sesión).

### Día — la home y el modo claro de las docs

| Token | Hex | Papel | Contraste |
|---|---|---|---|
| `--color-canvas` | `#E9E5DC` | Fondo: hormigón cálido | — |
| `--color-surface` | `#F4F1EA` | Superficies elevadas: tablas, etiquetas | — |
| `--color-ink` | `#15171A` | Texto, filetes fuertes | 14,3 sobre canvas |
| `--color-ink-muted` | `#5A574F` | Texto secundario | 5,7 sobre canvas · 6,4 sobre surface |
| `--color-rule` | `#C9C3B6` | Filetes finos (decorativo, nunca texto) | — |

### Noche — la terminal y el modo oscuro de las docs

| Token | Hex | Papel | Contraste |
|---|---|---|---|
| `--color-night` | `#101215` | Fondo | — |
| `--color-night-surface` | `#171A1E` | Paneles | — |
| `--color-night-ink` | `#E6E1D6` | Texto | 14,4 |
| `--color-night-muted` | `#9A968C` | Secundario | 6,4 · 5,9 sobre surface |
| `--color-night-rule` | `#2A2E33` | Filetes | — |

### Los contenedores — color con significado, no decoración

Cada capa tiene un color, y ese código se mantiene en **toda** la web: árbol de ficheros,
diagramas, etiquetas de las docs. Nunca se usan para otra cosa.

| Capa | Día | Noche | Contraste (día / noche) | Texto sobre el relleno |
|---|---|---|---|---|
| `business` — azul naviera | `#1D4E89` | `#6FA3E0` | 6,7 / 7,1 | surface 7,4 |
| `infrastructure` — rojo óxido | `#A33B20` | `#E58A6A` | 5,2 / 7,3 | surface 5,8 |
| `presentation` — verde casco | `#2F6B4F` | `#6DBE93` | 5,0 / 8,4 | surface 5,6 |

El color **nunca es el único portador**: cada contenedor lleva su nombre escrito.

### El acento — amarillo grúa

| Token | Hex | Uso |
|---|---|---|
| `--color-accent` | `#F5B700` | CTA principal, franjas de señalización, foco |

**Un solo acento** (candado 1). Los tres colores de capa no son acentos: son un código, y
no aparecen en botones ni en enlaces.

- Sobre claro, **solo como relleno con tinta encima** (tinta sobre amarillo: 10,0). Como
  texto sobre canvas da 1,4: prohibido.
- Sobre noche sí puede ser texto (10,4).
- Las franjas de peligro (`repeating-linear-gradient` 45° amarillo/tinta) marcan **una**
  cosa por pantalla como mucho: el CTA de empezar.

## 3. Tipografía

Las tres autoalojadas con fontsource (sin peticiones a Google).

| Token | Familia | Uso |
|---|---|---|
| `--font-display` | **Big Shoulders** (variable, 100–900) | Titulares, en mayúsculas. Nació de la señalética industrial de Chicago: condensada, alta, de muelle |
| `--font-stencil` | **Big Shoulders Stencil** (variable) | Etiquetas de contenedor, el wordmark, códigos de capa. Nunca en párrafos |
| `--font-sans` | **Archivo** (variable, con eje `wdth`) | Texto corrido, navegación, UI |
| `--font-mono` | **Martian Mono** (variable, con eje `wdth`) | Código, comandos, rutas de fichero. `wdth` 87,5 en bloques para que quepan 80 columnas |

Escala (fluida, `clamp`):

| Nivel | Tamaño | Familia | Notas |
|---|---|---|---|
| Display | `clamp(3.5rem, 11vw, 10rem)` / 0,82 | Big Shoulders 800 | Solo el titular de la home |
| H2 | `clamp(2.5rem, 6vw, 5rem)` / 0,9 | Big Shoulders 800 | Mayúsculas |
| H3 | `1.5rem` / 1,1 | Big Shoulders 700 | Mayúsculas, `letter-spacing: .01em` |
| Etiqueta | `0.8125rem` | Stencil 700 | Mayúsculas, `letter-spacing: .06em` |
| Cuerpo | `1.0625rem` / 1,6 | Archivo 400 | Máximo 68 caracteres de ancho |
| Código | `0.875rem` / 1,65 | Martian Mono 400 | |

**Prohibido:** Inter, system-ui como elección, JetBrains Mono (la de todo el ecosistema de
Rust y la de la marca vieja).

## 4. El logotipo en la web

Símbolo: **tres contenedores idénticos apilados**, cada uno desplazado unos píxeles como los
deja la grúa: business abajo, infrastructure en medio y presentation arriba, cada uno de su
color. Que sean **idénticos** es la tesis: el estándar. Wordmark: `PUERTO` en Big Shoulders
Stencil 800.

> Propuesta descartada, 2026-10-13: los contenedores formando la silueta de una P. Con tres
> barras horizontales no hay contraforma, y se lee una F. Forzar la P exigía un contenedor
> en vertical, que deja de ser un contenedor apilado.

- En la cabecera: símbolo de 28 px de alto + wordmark. Altura de cabecera 64 px.
- Favicon: solo el símbolo, simplificado (sin las ranuras de las puertas), legible a 16 px.
- Lo que falla si se encoge: por debajo de 20 px las ranuras se empastan. El favicon
  existe para eso; no se reutiliza el símbolo completo.

## 5. Componentes

- **Botón primario:** relleno amarillo, texto tinta en Archivo 600, sin radio, borde de 2 px
  tinta. Hover: se desplaza 2 px arriba-izquierda y deja una sombra dura de tinta
  (`4px 4px 0`). Foco: contorno de 3 px tinta con 2 px de separación.
- **Botón secundario:** transparente, borde de 2 px tinta, mismo hover.
- **Comando copiable:** bloque mono sobre noche, prompt `$` en muted, botón «Copy» que
  cambia a «Copied» durante 1,6 s (`aria-live`).
- **Etiqueta de contenedor:** rectángulo con el color de la capa, texto stencil en surface,
  y a la derecha un código mono (`BUS`, `INF`, `PRE`).
- **Tabla de manifiesto:** filas separadas por filete de 1 px `--color-rule`, cabecera en
  stencil. Sin tarjetas.

## 6. Layout

- Rejilla de 12 columnas, ancho máximo de 1280 px y margen lateral de 16 px en móvil y
  32 px en escritorio.
- **Disciplina de hero:** titular en un máximo de tres líneas cortas (`SCAFFOLD.` /
  `STRUCTURE.` / `SHIP.`): es el lema y ya es una secuencia. Subtítulo de 20 palabras como
  mucho. Comando de instalación y CTA visibles sin scroll.
- El aire es el del muelle: secciones separadas por mucho espacio (`clamp(5rem, 12vw, 10rem)`)
  y un filete de tinta de 2 px a todo el ancho, como la línea pintada del muelle.
- Sin tres tarjetas iguales en fila: los contenidos repetidos van en manifiesto (tabla) o en
  zig-zag.

## 7. Profundidad

Plana. **Ni sombras difusas ni blur.** La única elevación es la sombra dura de tinta
(`4px 4px 0 var(--color-ink)`), y solo en hover de botones y en el panel de la terminal
(`8px 8px 0`), que es el objeto que se manipula.

Radios: **cero en todo** (candado 2). También en Starlight.

## 8. Do's & don'ts

**Sí**
- El código de color de capas, siempre igual, en toda la web.
- Salida real del CLI, capturada. El contenido técnico es la imagen de esta web.
- Los números que se leen del repo (versión, ADRs) en build. Nunca escritos a mano.
- Copy corto, declarativo, con verbos. Como el de los commits de Puerto: «a write that
  skips the outbox does not compile».

**No**
- Naranja. Es el color de Rust y de todos los proyectos de Rust.
- Iconos de línea genéricos para capacidades. La capacidad se explica con su comando.
- Gradientes, glow, glassmorphism, mesh. Nada brilla.
- Estadísticas de vanidad (estrellas, descargas) mientras no sean significativas.
- «Blazingly fast», «AI-powered», «next-gen». Puerto no vende velocidad, vende forma.

### Excepciones documentadas a `design/ANTISLOP.md`

- **Versión en la página** (`v0.19`). El antislop la prohíbe en webs de negocio porque es
  decoración; en un framework es **información**: dice qué instala `cargo install`. Va solo
  junto al comando de instalación y en el pie, leída de `Cargo.toml`. Nunca como adorno.
- **UI de producto hecha con divs.** La terminal es HTML, pero **lo que muestra es real**:
  árbol, comandos, stdout y ficheros salen del CLI publicado. La regla prohíbe inventar
  producto, y esto no lo inventa.
- **Un tema por página.** La home es de día y la terminal es un panel de noche dentro de
  ella. No es un salto de tema: es un bloque de código grande, y el código va en noche en
  toda la web. Las docs siguen la preferencia del sistema, con los dos temas diseñados.

## 9. Responsive

- Hasta 640 px: el titular baja a 3,5 rem, el manifiesto pasa de tabla a lista apilada
  (comando arriba, descripción debajo) y la terminal se queda en **pestañas**: Compose ·
  Tree · File.
- De 640 a 1024 px: la terminal pasa a dos columnas (compositor arriba, árbol y fichero
  abajo).
- Desde 1024 px: las tres columnas de la terminal a la vez.
- Objetivos táctiles de 44 px en móvil. Los chips de flags cumplen esa medida.

## 10. Movimiento

Tono **moderno / tech**: 0,6 s, `power2.out`, stagger 0,08 s, desplazamiento de 28 px.

| Qué | Decisión |
|---|---|
| Titular de la home | Entrada en cascada por línea (`gsap.set` antes) |
| Árbol de la terminal al cambiar de carga o de flags | **Estiba**: los ficheros añadidos bajan 12 px y aparecen con stagger de 0,012 s; los que se van se desvanecen. Comunica qué cambió, así que tiene función |
| Contenedores de la regla de dependencias | Se apilan al entrar en pantalla (`once: true`) |
| Filas del manifiesto | No (tabla larga) |
| Código | Nunca |
| Cabecera | Ocultar al bajar y mostrar al subir, con CSS + rAF |
| Pie | Nunca |

Todo dentro de `prefers-reduced-motion: no-preference`. ScrollTrigger con `once: true`.

## 11. Relieve

No lleva. **¿Necesita WebGL? No:** los contenedores son formas planas en 2D y SVG con GSAP
las mueve sin coste. La pieza de la home es la terminal, no una escena.

## 12. Accesibilidad

- Contrastes de la sección 2, medidos.
- La terminal funciona con teclado: los flags son `<input type="checkbox|radio">` reales,
  el árbol es una lista de botones con `aria-current`, y el fichero abierto se anuncia.
- `prefers-reduced-motion` respetado: sin estiba, cambio directo.
- Foco visible en amarillo con contorno de tinta (sobre claro) y amarillo (sobre noche).
- El código de color de capas va siempre acompañado de texto.
