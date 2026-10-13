/**
 * Grafo de entidades de schema.org, no schemas sueltos por página. → .claude/rules/seo.md
 *
 * Puerto no es un negocio local: el `@type` concreto es `SoftwareApplication` (el CLI que se
 * instala) con su código fuente como `SoftwareSourceCode`, y la persona que lo hace como
 * autor. Todo enlazado por `@id`; las páginas referencian, no redeclaran.
 */
import { site } from '@/config/site';

export interface GraphOptions {
  url: string;
  name: string;
  version: string;
}

export function buildGraph({ url, name, version }: GraphOptions) {
  return {
    '@context': 'https://schema.org',
    '@graph': [
      {
        '@type': 'SoftwareApplication',
        '@id': site.schema.softwareId,
        name: site.name,
        alternateName: ['Puerto framework', 'Puerto Rust', 'puerto-rs'],
        description: site.description,
        url: site.url,
        applicationCategory: 'DeveloperApplication',
        applicationSubCategory: 'Web framework',
        operatingSystem: 'Linux, macOS, Windows',
        softwareVersion: version,
        license: 'https://opensource.org/licenses/MIT',
        offers: { '@type': 'Offer', price: '0', priceCurrency: 'USD' },
        keywords: site.schema.keywords.join(', '),
        author: { '@id': site.schema.authorId },
        sameAs: [site.repo, site.crate],
        installUrl: site.crate,
      },
      {
        '@type': 'SoftwareSourceCode',
        '@id': `${site.repo}#source`,
        name: site.name,
        codeRepository: site.repo,
        programmingLanguage: 'Rust',
        license: 'https://opensource.org/licenses/MIT',
        author: { '@id': site.schema.authorId },
        targetProduct: { '@id': site.schema.softwareId },
      },
      {
        '@type': 'Person',
        '@id': site.schema.authorId,
        name: site.author.name,
        url: site.author.url,
        sameAs: [site.author.url],
      },
      {
        '@type': 'WebSite',
        '@id': site.schema.webId,
        url: site.url,
        name: site.name,
        inLanguage: site.language,
        about: { '@id': site.schema.softwareId },
        publisher: { '@id': site.schema.authorId },
      },
      {
        '@type': 'WebPage',
        '@id': `${url}#page`,
        url,
        name,
        isPartOf: { '@id': site.schema.webId },
        about: { '@id': site.schema.softwareId },
        inLanguage: site.language,
      },
    ],
  };
}
