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
    'Puerto is a Rust framework that generates a Domain-Driven Design project: API, ' +
    'database, events, auth and a typed React frontend, from one manifest.',
  tagline: 'Scaffold. Structure. Ship.',
  language: 'en',

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
    { text: 'Guides', href: '/docs/guides/entities/' },
    { text: 'Reference', href: '/docs/reference/cli/' },
    { text: 'Decisions', href: '/docs/decisions/' },
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
      'Rust web framework',
      'Domain-Driven Design',
      'Clean Architecture',
      'code generator',
      'poem-openapi',
      'SQLx',
      'transactional outbox',
    ],
  },
} as const;

export type NavLink = (typeof site.nav)[number];
