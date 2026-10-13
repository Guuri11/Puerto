import { test, expect } from '@playwright/test';

/**
 * Lo que tiene que cumplir toda página del sitio. Cada página nueva se añade a `PAGINAS`: si
 * no está aquí, nadie comprueba que siga viva.
 *
 * El `h1` es el fragmento que **tiene que** decir. Si alguien cambia un titular y esto se cae,
 * esa es la conversación que el test existe para provocar.
 */

const DOMINIO = 'https://puerto-rs.vercel.app';

const PAGINAS = [
  { ruta: '/', h1: 'Scaffold. Structure. Ship.' },
  { ruta: '/docs/', h1: 'What Puerto is' },
  { ruta: '/docs/start/installation/', h1: 'Installation' },
  { ruta: '/docs/start/quick-start/', h1: 'Quick start' },
  { ruta: '/docs/start/anatomy/', h1: 'Anatomy of a project' },
  { ruta: '/docs/start/architecture/', h1: 'The architecture' },
  { ruta: '/docs/guides/entities/', h1: 'Entities and fields' },
  { ruta: '/docs/guides/value-objects/', h1: 'Value objects' },
  { ruta: '/docs/guides/use-cases/', h1: 'Use cases' },
  { ruta: '/docs/guides/database/', h1: 'Database and migrations' },
  { ruta: '/docs/guides/events/', h1: 'Domain events' },
  { ruta: '/docs/guides/mail/', h1: 'Mail' },
  { ruta: '/docs/guides/auth/', h1: 'Authentication' },
  { ruta: '/docs/guides/web/', h1: 'The web frontend' },
  { ruta: '/docs/guides/layer-by-layer/', h1: 'Layer by layer' },
  { ruta: '/docs/guides/testing/', h1: 'Testing' },
  { ruta: '/docs/guides/agents/', h1: 'Working with agents' },
  { ruta: '/docs/guides/configuration/', h1: 'Configuration and running' },
  { ruta: '/docs/guides/upgrading/', h1: 'Upgrading' },
  { ruta: '/docs/reference/cli/', h1: 'CLI' },
  { ruta: '/docs/reference/puerto-toml/', h1: 'The puerto.toml manifest' },
  { ruta: '/docs/reference/field-types/', h1: 'Field types' },
  { ruta: '/docs/reference/errors/', h1: 'Error contract' },
  { ruta: '/docs/reference/runtime-crates/', h1: 'Runtime crates' },
  { ruta: '/docs/reference/make/', h1: 'Make targets' },
  { ruta: '/docs/decisions/', h1: 'Why decisions are written down' },
  { ruta: '/docs/decisions/adr/0018-domain-events-outbox-pluggable-transport/', h1: '0018' },
  { ruta: '/docs/changelog/', h1: 'Changelog' },
];

test.describe('Todas las páginas', () => {
  for (const { ruta, h1 } of PAGINAS) {
    test(`${ruta} responde y tiene un solo h1`, async ({ page }) => {
      const respuesta = await page.goto(ruta);
      expect(respuesta?.status()).toBe(200);
      const encabezados = page.locator('h1');
      await expect(encabezados).toHaveCount(1);
      // Starlight y la home ponen el titular en mayúsculas por CSS: se compara el texto real.
      const texto = (await encabezados.textContent())!.replace(/\s+/g, ' ').trim();
      expect(texto).toContain(h1);
    });

    test(`${ruta} no desborda en horizontal`, async ({ page }) => {
      await page.goto(ruta);
      const desborda = await page.evaluate(
        () => document.documentElement.scrollWidth > document.documentElement.clientWidth + 1,
      );
      expect(desborda).toBe(false);
    });

    test(`${ruta} tiene canonical con barra final`, async ({ page }) => {
      await page.goto(ruta);
      const canonical = await page.locator('link[rel=canonical]').getAttribute('href');
      expect(canonical).toBe(`${DOMINIO}${ruta}`);
    });
  }
});

test('el título y la descripción caben en la SERP', async ({ page }) => {
  for (const { ruta } of PAGINAS.filter((p) => !p.ruta.includes('/adr/'))) {
    await page.goto(ruta);
    const titulo = await page.title();
    const descripcion = await page.locator('meta[name=description]').getAttribute('content');
    expect(titulo.length, `título de ${ruta}: "${titulo}"`).toBeLessThanOrEqual(60);
    expect(descripcion!.length, `description de ${ruta}`).toBeGreaterThanOrEqual(110);
    expect(descripcion!.length, `description de ${ruta}`).toBeLessThanOrEqual(160);
  }
});

test('ningún enlace interno de la home ni de las docs está roto', async ({ page }) => {
  const destinos = new Set<string>();
  for (const ruta of ['/', '/docs/', '/docs/start/architecture/', '/docs/guides/events/', '/docs/reference/cli/']) {
    await page.goto(ruta);
    const enlaces = await page
      .locator('a[href^="/"]')
      .evaluateAll((as) => as.map((a) => a.getAttribute('href')!.split('#')[0]));
    enlaces.filter(Boolean).forEach((e) => destinos.add(e));
  }
  expect(destinos.size).toBeGreaterThan(20);
  for (const destino of destinos) {
    const respuesta = await page.request.get(destino);
    expect(respuesta.status(), `enlace roto: ${destino}`).toBe(200);
  }
});

test('una ruta que no existe responde 404 con salidas', async ({ page }) => {
  const respuesta = await page.goto('/no-such-berth/');
  expect(respuesta?.status()).toBe(404);
  await expect(page.locator('h1')).toContainText('docked');
  await expect(page.locator('main a[href="/docs/start/quick-start/"]')).toBeVisible();
});

test('el sitemap no lista los fragmentos de la terminal', async ({ request }) => {
  const indice = await (await request.get('/sitemap-index.xml')).text();
  const mapa = indice.match(/<loc>([^<]+)<\/loc>/)![1].replace(DOMINIO, '');
  const urls = await (await request.get(mapa)).text();
  expect(urls).toContain(`${DOMINIO}/docs/guides/events/`);
  expect(urls).not.toContain('/cargo/');
});
