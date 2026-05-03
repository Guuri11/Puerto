export const siteConfig = {
  name: 'Puerto',
  tagline: 'Scaffold. Structure. Ship.',
  description: 'The Rust DDD framework that brings Rails-like productivity to production-grade backends. Zero boilerplate, clean architecture, AI-ready.',
  url: 'https://puerto-framework.dev',
  github: 'https://github.com/Guuri11/Puerto',

  logo: {
    src: '/images/logo.svg',
    alt: 'Puerto',
    width: 120,
    height: 28,
  },

  nav: [
    { label: 'Docs', href: '/docs' },
    { label: 'GitHub', href: 'https://github.com/Guuri11/Puerto', external: true },
  ],
} as const;
