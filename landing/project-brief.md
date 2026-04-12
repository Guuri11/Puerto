# Project Brief — Harbor

> Fase 1 completada — 2026-04-12

---

## Datos de la empresa

| Campo | Valor |
|-------|-------|
| Nombre de la empresa | Harbor |
| Sector / industria | Developer tooling — Rust framework |
| Año de fundación | 2026 |
| Ubicación | Open source / Global |
| Web actual | Sin web (solo README en GitHub) |
| Teléfono | N/A |
| Email de contacto | N/A |
| Persona de contacto | Sergio Gurillo (autor) |

## Producto

Harbor es un **framework Rust full-stack con arquitectura DDD** que trae la experiencia de Laravel/Rails al ecosistema Rust. Herramienta CLI open source.

1. **`harbor new`**: scaffolding interactivo de workspace Cargo con 3 capas (business, infrastructure, presentation). Con o sin base de datos (--db).
2. **`harbor generate scaffold <Name>`**: genera una entidad DDD completa cruzando todas las capas. Zero boilerplate manual.
3. **`harbor generate use-case <Entity> <action>`**: añade un caso de uso a una entidad existente con tests incluidos.
4. **`harbor generate migration <name>`**: wraps `sqlx migrate add` con convenciones Harbor.
5. **Convención sobre configuración**: estructura estandarizada, patrones de error, DI automática via `bootstrap.rs` generado.

## Público objetivo

**Perfil principal:**
- Desarrolladores Rust de nivel medio-avanzado
- Que vienen de Rails, Laravel, Django y extrañan la productividad
- Que quieren construir backends escalables sin decidir la arquitectura desde cero
- Que usan LLMs para generar código y necesitan feedback rápido del compilador

**Pain points que Harbor resuelve:**
- "Rust tiene el ecosistema fragmentado — no sé qué librería usar ni cómo estructurar el proyecto"
- "Pierdo horas en boilerplate antes de escribir lógica de negocio"
- "El código generado por IA en Rust necesita muchas correcciones antes de compilar"
- "Cada proyecto tiene una estructura diferente, difícil de navegar"

## Branding actual

### Logo
- [x] Tienen logo
- [x] Logo disponible en formato vectorial (SVG)
- Ruta: `~/dev/harbor/assets/brand/`
- Variantes: `logo-full.svg` (light), `logo-full-dark.svg` (dark), `logo-icon.svg` (64px), `favicon.svg`

### Paleta de colores — DEFINITIVA (del brand oficial)

| Token | Hex | Uso semántico |
|-------|-----|---------------|
| harbor-deep | `#0D1B2A` | Fondo principal (dark) |
| harbor-navy | `#1E3A5F` | Cards, secondary bg |
| harbor-steel | `#2D5986` | Bordes, UI chrome |
| harbor-teal | `#0891B2` | Primary accent, capa business |
| harbor-indigo | `#6366F1` | Links, capa infrastructure |
| harbor-indigo-light | `#818CF8` | Indigo en dark bg |
| harbor-rust | `#F97316` | CTA principal, capa presentation |
| harbor-ice | `#F1F5F9` | Texto principal (sobre oscuro) |
| harbor-mist | `#94A3B8` | Texto secundario, badges |

### Tipografía — DEFINITIVA

| Token | Fuente | Uso |
|-------|--------|-----|
| --font-mono | JetBrains Mono | Logo, comandos CLI, bloques de código |
| --font-sans | system-ui, -apple-system | Prose, body, navegación |

### Tono de voz
- [x] Técnico y especializado (sin pedantería)
- Directo, sin palabrería: los devs valoran la concisión
- Tagline oficial: **"Scaffold. Structure. Ship."**

### Materiales gráficos disponibles
- [x] Logo SVG completo (light + dark + icon + favicon)
- [x] GitHub social preview SVG (1280×640)
- [ ] Fotos (no aplica — producto de software)

## Webs de referencia

| Proyecto | URL | Referencia |
|---------|-----|------------|
| Loco.rs | loco.rs | Referente principal: Rust, dark mode, terminal vibes |
| Zed.dev | zed.dev | Dark mode premium, tipografía elegante |
| Astro.build | astro.build | Copy orientado a beneficio, alternating sections, code snippets |

## Páginas a construir (v1)

- [x] **Home** — hero + features + arquitectura visual + quick start + "por qué Harbor" + CTA GitHub
- [x] **Docs / Getting Started** — instalación, `harbor new`, primer scaffold

**Para v1: 2 páginas.** Showcase y blog en fases posteriores.

## Funcionalidades

- [x] Bloques de código con syntax highlighting (Shiki via Astro)
- [x] Copy-to-clipboard en comandos CLI
- [x] Sticky header con scroll hide/show (vanilla JS)
- [x] Anclas suaves a secciones (#features, #quickstart)
- Solo inglés (comunidad Rust global)

## Decisiones de branding (confirmadas)

### Paleta de color definitiva para el sitio

| Token CSS | Hex | Uso |
|-----------|-----|-----|
| --color-bg | `#0D1B2A` | Fondo principal (harbor-deep) |
| --color-bg-alt | `#1E3A5F` | Cards, secciones alternativas (harbor-navy) |
| --color-border | `#2D5986` | Bordes (harbor-steel) |
| --color-primary | `#0891B2` | Primary accent, links (harbor-teal) |
| --color-secondary | `#6366F1` | Secondary links, badges (harbor-indigo) |
| --color-accent | `#F97316` | CTA, botones principales (harbor-rust) |
| --color-text | `#F1F5F9` | Texto principal (harbor-ice) |
| --color-text-muted | `#94A3B8` | Texto secundario (harbor-mist) |

### Tipografía definitiva

| Token | Fuente | Uso |
|-------|--------|-----|
| --font-mono | 'JetBrains Mono', 'Fira Code', ui-monospace | Código, comandos, logo text |
| --font-sans | system-ui, -apple-system, sans-serif | Todo lo demás |

### Estilo visual general

Dark mode puro. Terminal vibes con elegancia de producto premium. El diseño habla el idioma de los desarrolladores: código visible, estructura clara, sin adornos innecesarios. Inspirado en loco.rs y Zed.dev.

## Plan de animación

### Nivel: 2 — Herramienta técnica

Animaciones funcionales, no decorativas. Los desarrolladores aprecian fluidez pero no excesos.

### Mapa de animación

| Qué | Decisión | Notas |
|-----|----------|-------|
| Hero entrance (título, subtítulo, CTA) | Animar | fade-up cascada, 0.6s |
| Feature cards con stagger | Animar | scroll reveal, stagger 0.10s |
| Secciones texto + código | Animar | fade-up suave |
| Bloques de código | No animar | Ya tienen peso visual propio |
| Navegación (hide/show al scroll) | Sí (vanilla JS) | rAF + CSS transition |
| Formularios | Nunca | — |

### Valores de referencia

| Propiedad | Valor |
|-----------|-------|
| Duration base | 0.6s |
| Ease principal | power2.out |
| Stagger entre elementos | 0.10s |
| Y offset de entrada | 30px |

### Plugins necesarios
- [x] ScrollTrigger — scroll reveals

---

## Plazos

| Hito | Fecha |
|------|-------|
| Brief completo | 2026-04-12 ✅ |
| Fase 2 (desarrollo) | 2026-04-12 |

---

*Brief creado el 2026-04-12 · Proyecto Harbor (proyecto propio de Sergio Gurillo)*
