import { test, expect } from '@playwright/test';
import { readFileSync } from 'node:fs';

/**
 * El escaparate de bloques (ADR-0028). Todo sale de `pnpm bloques`: si el catálogo de la CLI
 * cambia y no se regenera, estos tests son los que avisan.
 */

interface Bloque {
  id: string;
  hueco: string;
  porDefecto: boolean;
  ficheros: { ruta: string }[];
}

const bloques: Bloque[] = JSON.parse(readFileSync('src/data/bloques.json', 'utf8'));

test('el escaparate enseña cada bloque del catálogo y enlaza su página', async ({ page }) => {
  await page.goto('/bloques/');
  for (const bloque of bloques) {
    const enlace = page.getByRole('link', { name: bloque.id, exact: true });
    await expect(enlace).toHaveAttribute('href', `/bloques/${bloque.id}/`);
  }
  // Una miniatura por bloque, que no se puede tocar ni enfocar.
  await expect(page.locator('.vista--miniatura iframe')).toHaveCount(bloques.length);
  await expect(page.locator('.vista--miniatura iframe').first()).toHaveAttribute('tabindex', '-1');
});

test('cada hueco tiene una sola variante por defecto', () => {
  const huecos = new Set(bloques.map((b) => b.hueco));
  for (const hueco of huecos) {
    expect(bloques.filter((b) => b.hueco === hueco && b.porDefecto), hueco).toHaveLength(1);
  }
});

test('la página de un bloque da su comando, sus ficheros y el texto para un agente', async ({ page }) => {
  const bloque = bloques.find((b) => b.id === 'portada/producto')!;
  await page.goto(`/bloques/${bloque.id}/`);

  await expect(page.getByText(`puerto generar web-bloque ${bloque.id} --forzar`).first()).toBeVisible();
  for (const fichero of bloque.ficheros) {
    await expect(page.locator('figcaption', { hasText: fichero.ruta })).toHaveCount(1);
  }
  await expect(page.locator('.bl-agente')).toContainText(`# Bloque \`${bloque.id}\` de Puerto`);
});

test('un bloque que pide una entidad la da en su comando', async ({ page }) => {
  await page.goto('/bloques/escaparate/tarjetas/');

  await expect(
    page.getByText('puerto generar web-bloque escaparate/tarjetas <Entidad> --forzar').first(),
  ).toBeVisible();
});

test('la vista previa cambia de tema y de ancho', async ({ page }) => {
  await page.goto('/bloques/marco/superior/');
  const vista = page.locator('[data-vista]');
  const marco = vista.locator('iframe');
  await expect(marco).toHaveAttribute('src', /bloque=marco%2Fsuperior&tema=light/);

  await page.getByRole('button', { name: 'Oscuro' }).click();
  await expect(marco).toHaveAttribute('src', /tema=dark/);
  await expect(page.getByRole('button', { name: 'Oscuro' })).toHaveAttribute('aria-pressed', 'true');

  await page.getByRole('button', { name: 'Móvil' }).click();
  await expect(vista).toHaveAttribute('data-ancho', '390');
  await expect(marco).toHaveCSS('width', '390px');
});

test('la vista previa pinta el bloque con datos de ejemplo', async ({ page }) => {
  await page.goto('/bloques/vista/vista.html?bloque=panel/resumen&tema=light');
  await expect(page.getByRole('heading', { name: 'Panel' })).toBeVisible();
  await expect(page.getByText('128')).toBeVisible();
});

test('la página de cada bloque cabe en la SERP', async ({ page }) => {
  for (const bloque of bloques) {
    await page.goto(`/bloques/${bloque.id}/`);
    const titulo = await page.title();
    const descripcion = await page.locator('meta[name=description]').getAttribute('content');
    expect(titulo.length, `título de ${bloque.id}`).toBeLessThanOrEqual(60);
    expect(descripcion!.length, `description de ${bloque.id}: ${descripcion}`).toBeGreaterThanOrEqual(110);
    expect(descripcion!.length, `description de ${bloque.id}: ${descripcion}`).toBeLessThanOrEqual(160);
  }
});
