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
        'Documentation for Puerto, the Rust framework that generates Domain-Driven Design projects.',
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
          label: 'Start here',
          items: [
            { label: 'What Puerto is', slug: 'docs' },
            { label: 'Installation', slug: 'docs/start/installation' },
            { label: 'Quick start', slug: 'docs/start/quick-start' },
            { label: 'Anatomy of a project', slug: 'docs/start/anatomy' },
            { label: 'The architecture', slug: 'docs/start/architecture' },
          ],
        },
        {
          label: 'Guides',
          items: [
            { label: 'Entities and fields', slug: 'docs/guides/entities' },
            { label: 'Value objects', slug: 'docs/guides/value-objects' },
            { label: 'Use cases', slug: 'docs/guides/use-cases' },
            { label: 'Database and migrations', slug: 'docs/guides/database' },
            { label: 'Domain events', slug: 'docs/guides/events' },
            { label: 'Mail', slug: 'docs/guides/mail' },
            { label: 'Authentication', slug: 'docs/guides/auth' },
            { label: 'The web frontend', slug: 'docs/guides/web' },
            { label: 'Layer by layer', slug: 'docs/guides/layer-by-layer' },
            { label: 'Testing', slug: 'docs/guides/testing' },
            { label: 'Working with agents', slug: 'docs/guides/agents' },
            { label: 'Configuration and running', slug: 'docs/guides/configuration' },
            { label: 'Upgrading', slug: 'docs/guides/upgrading' },
          ],
        },
        {
          label: 'Reference',
          items: [
            { label: 'CLI', slug: 'docs/reference/cli' },
            { label: 'puerto.toml', slug: 'docs/reference/puerto-toml' },
            { label: 'Field types', slug: 'docs/reference/field-types' },
            { label: 'Error contract', slug: 'docs/reference/errors' },
            { label: 'Runtime crates', slug: 'docs/reference/runtime-crates' },
            { label: 'Make targets', slug: 'docs/reference/make' },
          ],
        },
        {
          label: 'Decisions',
          collapsed: true,
          items: [
            { label: 'Why they are written down', slug: 'docs/decisions' },
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
