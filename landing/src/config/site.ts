/**
 * Única fuente de verdad de los datos del sitio.
 *
 * Lo que se puede leer del repo (versión, ADRs, changelog, la ayuda del CLI) no se escribe
 * aquí: lo lee `src/lib/repo.ts` en build, o `pnpm harvest`. Aquí va solo lo que no está en
 * ningún otro sitio.
 */

export const site = {
  name: 'Puerto',
  /** El dominio definitivo (CLAUDE.md). Si cambia, cambia aquí y en astro.config.mjs. */
  url: 'https://puerto-rs.vercel.app',
  description:
    'Puerto es un framework en Rust que genera un proyecto Domain-Driven Design: API, ' +
    'base de datos, eventos, auth y frontend React tipado, desde un manifiesto.',
  tagline: 'Genera. Ordena. Zarpa.',
  language: 'es',

  repo: 'https://github.com/Guuri11/Puerto',
  crate: 'https://crates.io/crates/puerto',
  license: 'MIT',
  install: 'cargo install puerto --locked',

  author: {
    name: 'Sergio Gurillo',
    url: 'https://github.com/Guuri11',
  },

  nav: [
    { text: 'Docs', href: '/docs/' },
    { text: 'Guías', href: '/docs/guides/entities/' },
    { text: 'Referencia', href: '/docs/reference/cli/' },
    { text: 'Bloques', href: '/bloques/' },
    { text: 'Decisiones', href: '/docs/decisions/' },
  ],

  /**
   * El grafo de schema.org. Lo consume `src/lib/schema.ts`.
   * Los `@id` se escriben una vez y no se tocan: cambiarlos rompe el grafo en silencio.
   */
  schema: {
    softwareId: 'https://puerto-rs.vercel.app/#software',
    webId: 'https://puerto-rs.vercel.app/#web',
    authorId: 'https://puerto-rs.vercel.app/#author',
    keywords: [
      'framework web en Rust',
      'Domain-Driven Design',
      'Clean Architecture',
      'generador de código',
      'poem-openapi',
      'SQLx',
      'transactional outbox',
    ],
  },
} as const;

export type NavLink = (typeof site.nav)[number];
