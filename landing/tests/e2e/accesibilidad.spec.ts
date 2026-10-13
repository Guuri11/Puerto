import { test, expect } from '@playwright/test';
import { execFileSync } from 'node:child_process';
import { readFileSync } from 'node:fs';

/**
 * Accesibilidad, datos estructurados y la verdad de los datos. Cada comprobación es un fallo
 * que se ha cometido alguna vez; se lee entera en un minuto.
 */

const RUTAS = ['/', '/docs/', '/docs/guides/events/'];

test.describe('Accesibilidad y robustez', () => {
  for (const ruta of RUTAS) {
    test(`${ruta}: el contenido se ve aunque el JavaScript no corra`, async ({ browser, baseURL }) => {
      // `baseURL` explícito: browser.newContext() no lo hereda de la configuración.
      const contexto = await browser.newContext({ javaScriptEnabled: false, baseURL });
      const pagina = await contexto.newPage();
      await pagina.goto(ruta);
      await expect(pagina.locator('h1')).toBeVisible();
      expect(await pagina.locator('h1').evaluate((el) => getComputedStyle(el).opacity)).toBe('1');
      await contexto.close();
    });

    test(`${ruta}: los encabezados no saltan niveles`, async ({ page }) => {
      await page.goto(ruta);
      const niveles = await page
        .locator('main h1, main h2, main h3, main h4')
        .evaluateAll((els) => els.map((e) => Number(e.tagName[1])));
      expect(niveles[0]).toBe(1);
      for (let i = 1; i < niveles.length; i++) {
        expect(niveles[i] - niveles[i - 1], `salto de h${niveles[i - 1]} a h${niveles[i]}`).toBeLessThanOrEqual(1);
      }
    });

    test(`${ruta}: las imágenes llevan alt`, async ({ page }) => {
      await page.goto(ruta);
      const sinAlt = await page
        .locator('img')
        .evaluateAll((imgs) => imgs.filter((i) => i.getAttribute('alt') === null).map((i) => i.getAttribute('src')));
      expect(sinAlt).toEqual([]);
    });
  }

  test('el primer tabulador de la home salta al contenido, con foco visible', async ({ page }) => {
    await page.goto('/');
    await page.keyboard.press('Tab');
    await expect(page.locator('.skip-link')).toBeFocused();
    const contorno = await page.evaluate(() => getComputedStyle(document.activeElement!).outlineStyle);
    expect(contorno).not.toBe('none');
  });

  test('los objetivos táctiles de la home llegan a 44px', async ({ page }) => {
    await page.goto('/');
    // Con puntero fino (escritorio) la terminal usa filas de 32px, densas como un editor; con
    // dedo (pointer: coarse) suben a 44. Se mide lo que aplica al dispositivo del proyecto.
    const tactil = await page.evaluate(() => matchMedia('(pointer: coarse)').matches);
    const pequenos = await page
      .locator('a.btn, button, .header a, .footer nav a, .manifest a, .decisions__list a, .term__row, .term__pill span')
      .evaluateAll(
        (els, tactil) =>
          els
            .filter((e) => (e as HTMLElement).offsetParent !== null)
            .filter((e) => tactil || !e.closest('.term'))
            .map((e) => ({ texto: e.textContent?.trim().slice(0, 30), alto: e.getBoundingClientRect().height }))
            .filter((e) => e.alto > 0 && e.alto < 44),
        tactil,
      );
    expect(pequenos).toEqual([]);
  });
});

test.describe('Datos estructurados', () => {
  test('la home declara un solo grafo, con el software y su autor', async ({ page }) => {
    await page.goto('/');
    const bloques = await page.locator('script[type="application/ld+json"]').allTextContents();
    expect(bloques).toHaveLength(1);
    const tipos = JSON.parse(bloques[0])['@graph'].map((n: { '@type': string }) => n['@type']);
    expect(tipos).toEqual(
      expect.arrayContaining(['SoftwareApplication', 'SoftwareSourceCode', 'Person', 'WebSite', 'WebPage']),
    );
  });
});

test.describe('La verdad de los datos', () => {
  const cargo = readFileSync('../Cargo.toml', 'utf8');
  const version = cargo.match(/\[workspace\.package\][^[]*?version\s*=\s*"([^"]+)"/s)![1];

  test('la cosecha del CLI es de la versión del repo (si no: pnpm harvest)', () => {
    const meta = JSON.parse(readFileSync('src/data/harvest/meta.json', 'utf8'));
    const cli = JSON.parse(readFileSync('src/data/harvest/cli.json', 'utf8'));
    const compilador = JSON.parse(readFileSync('src/data/harvest/compiler.json', 'utf8'));
    expect(meta.version).toBe(version);
    expect(cli.version).toBe(version);
    expect(compilador.version).toBe(version);
  });

  test('la home enseña la versión y el número de ADRs del repo', async ({ page }) => {
    const adrs = execFileSync('ls', ['../docs/adr'], { encoding: 'utf8' })
      .split('\n')
      .filter((f) => /^\d{4}-.+\.md$/.test(f)).length;
    await page.goto('/');
    await expect(page.locator('.hero__meta')).toContainText(`puerto ${version}`);
    await expect(page.locator('#decisions-title')).toContainText(`${adrs} decisions`);
  });
});
