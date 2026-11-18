// @ts-check
import { readFileSync } from 'node:fs';
import { defineConfig } from 'astro/config';
import starlight from '@astrojs/starlight';
import sitemap from '@astrojs/sitemap';
import tailwindcss from '@tailwindcss/vite';

const nightTheme = JSON.parse(readFileSync(new URL('./src/themes/puerto-night.json', import.meta.url), 'utf8'));

// `.claude/rules/astro.md` pide un dominio propio, nunca .vercel.app. Aquí .vercel.app ES el
// dominio definitivo, por decisión de Sergio (2026-10-13; CLAUDE.md). Si cambia: aquí y en
// src/config/site.ts.
export default defineConfig({
  site: 'https://puerto-rs.vercel.app',
  trailingSlash: 'always',
  integrations: [
    // Declarado aquí, Starlight no añade el suyo. Fuera del sitemap: los fragmentos resaltados
    // de la terminal (/cargo/), que no son páginas, y el 404.
    sitemap({
      filter: (page) => !page.includes('/cargo/') && !page.endsWith('/404/'),
    }),
    starlight({
      title: 'Puerto',
      description:
        'La documentación de Puerto, el framework en Rust que genera proyectos Domain-Driven Design.',
      // Un solo idioma, en la raíz (sin /es/): los textos propios de Starlight (buscar, en esta
      // página, siguiente…) salen en castellano.
      defaultLocale: 'root',
      locales: { root: { label: 'Español', lang: 'es' } },
      disable404Route: true,
      lastUpdated: true,
      pagination: true,
      favicon: '/favicon.svg',
      customCss: ['./src/styles/starlight.css'],
      components: {
        SiteTitle: './src/components/docs/SiteTitle.astro',
        Head: './src/components/docs/Head.astro',
      },
      social: [{ icon: 'github', label: 'GitHub', href: 'https://github.com/Guuri11/Puerto' }],
      editLink: {
        baseUrl: 'https://github.com/Guuri11/Puerto/edit/main/landing/',
      },
      expressiveCode: {
        // Un solo tema: el código va en noche en toda la web, también en el modo claro de
        // las docs. → DESIGN.md § 8
        themes: [nightTheme],
        useStarlightUiThemeColors: false,
        // Un comando suelto no necesita la barra de título vacía del marco de terminal.
        defaultProps: { overridesByLang: { 'bash,sh,shell': { frame: 'none' } } },
        styleOverrides: {
          borderRadius: '0',
          borderColor: '#2A2E33',
          codeFontFamily: "'Martian Mono Variable', ui-monospace, monospace",
          codeFontSize: '0.8125rem',
          uiFontFamily: "'Archivo Variable', sans-serif",
          frames: {
            shadowColor: 'transparent',
            editorActiveTabIndicatorTopColor: '#F5B700',
            terminalTitlebarDotsOpacity: '0',
            tooltipSuccessBackground: '#F5B700',
            tooltipSuccessForeground: '#15171A',
          },
        },
      },
      sidebar: [
        {
          label: 'Empieza aquí',
          items: [
            { label: 'Qué es Puerto', slug: 'docs' },
            { label: 'Instalación', slug: 'docs/start/installation' },
            { label: 'Inicio rápido', slug: 'docs/start/quick-start' },
            { label: 'Anatomía de un proyecto', slug: 'docs/start/anatomy' },
            { label: 'La arquitectura', slug: 'docs/start/architecture' },
          ],
        },
        {
          label: 'Guías',
          items: [
            { label: 'Entidades y campos', slug: 'docs/guides/entities' },
            { label: 'Objetos valor', slug: 'docs/guides/value-objects' },
            { label: 'Casos de uso', slug: 'docs/guides/use-cases' },
            { label: 'Base de datos y migraciones', slug: 'docs/guides/database' },
            { label: 'Eventos de dominio', slug: 'docs/guides/events' },
            { label: 'Correo', slug: 'docs/guides/mail' },
            { label: 'Autenticación', slug: 'docs/guides/auth' },
            { label: 'El frontend web', slug: 'docs/guides/web' },
            { label: 'Capa a capa', slug: 'docs/guides/layer-by-layer' },
            { label: 'Tests', slug: 'docs/guides/testing' },
            { label: 'Trabajar con agentes', slug: 'docs/guides/agents' },
            { label: 'Configuración y arranque', slug: 'docs/guides/configuration' },
            { label: 'Actualizar', slug: 'docs/guides/upgrading' },
          ],
        },
        {
          label: 'Referencia',
          items: [
            { label: 'CLI', slug: 'docs/reference/cli' },
            { label: 'puerto.toml', slug: 'docs/reference/puerto-toml' },
            { label: 'Tipos de campo', slug: 'docs/reference/field-types' },
            { label: 'El contrato de errores', slug: 'docs/reference/errors' },
            { label: 'Crates del runtime', slug: 'docs/reference/runtime-crates' },
            { label: 'Targets de make', slug: 'docs/reference/make' },
          ],
        },
        {
          label: 'Decisiones',
          collapsed: true,
          items: [
            { label: 'Por qué se escriben', slug: 'docs/decisions' },
            { autogenerate: { directory: 'docs/decisions/adr' } },
          ],
        },
        { label: 'Changelog', slug: 'docs/changelog' },
      ],
    }),
  ],
  vite: {
    plugins: [tailwindcss()],
    preview: {
      allowedHosts: ['.ngrok-free.app', '.ngrok.app'],
    },
  },
});
