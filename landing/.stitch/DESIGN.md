# Design System — Puerto

## Identidad visual

Puerto es un framework CLI para Rust con arquitectura DDD. El diseño habla el idioma de los desarrolladores: dark mode puro, terminal vibes, estructura visible. No es un producto de consumo — es una herramienta de precisión para ingenieros.

## Paleta de colores

- Fondo principal: Puerto Deep (#0D1B2A) — navy oscuro casi negro
- Fondo secundario: Puerto Navy (#1E3A5F) — cards, secciones alternas
- Bordes / UI chrome: Puerto Steel (#2D5986)
- Primary accent: Puerto Teal (#0891B2) — links, highlights, capa business
- Secondary accent: Puerto Indigo (#6366F1) — badges, capa infrastructure
- CTA / botón principal: Puerto Rust (#F97316) — naranja vibrante, capa presentation
- Texto principal: Puerto Ice (#F1F5F9) — blanco frío
- Texto secundario: Puerto Mist (#94A3B8) — gris azulado

## Tipografía

- Código / comandos CLI: JetBrains Mono (monospace, bold)
- Body / prose / navegación: IBM Plex Sans
- Escala de títulos: H1 72px bold, H2 48px semibold, H3 32px semibold
- Body: 18px, line-height 1.75

## Estilo de componentes

- Bordes: sutiles, 1px solid #2D5986 (puerto-steel)
- Radius: 8px en cards, 6px en botones, 4px en badges
- Sombras: difusas y oscuras (box-shadow con navy)
- Espaciado: amplio — secciones con 120px vertical padding
- Code blocks: fondo #1E3A5F, border-left 3px solid teal, monospace

## Los 3 colores semánticos de Puerto (importante en diseño)

- Teal #0891B2 → capa "business/" (domain + application)
- Indigo #6366F1 → capa "infrastructure/"
- Orange #F97316 → capa "presentation/"
  Estos 3 colores deben aparecer juntos en el diagrama de arquitectura.

## Atmósfera

Sientes que estás en una terminal de alta gama. Dark, preciso, sin distracciones. El código es protagonista — bloques de comandos grandes, con resaltado, copiables. La estructura DDD se visualiza con color. Confías en la herramienta antes de usarla.
