export const siteConfig = {
  name: 'Harbor',
  tagline: 'Scaffold. Structure. Ship.',
  description: 'The Rust DDD framework that brings Rails-like productivity to production-grade backends. Zero boilerplate, clean architecture, AI-ready.',
  url: 'https://harbor-framework.dev',
  github: 'https://github.com/sergio-gurillo/harbor',

  logo: {
    src: '/images/logo.svg',
    alt: 'Harbor',
    width: 120,
    height: 28,
  },

  nav: [
    { label: 'Docs', href: '/docs' },
    { label: 'GitHub', href: 'https://github.com/sergio-gurillo/harbor', external: true },
  ],
} as const;
