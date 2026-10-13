import { test, expect } from '@playwright/test';

/**
 * La terminal de la home: el flujo propio de este proyecto. Si se rompe, la home pierde su
 * argumento principal, así que se prueba como un formulario que cuesta dinero.
 */

test.describe('La terminal', () => {
  test('sin JavaScript enseña la salida real de la combinación por defecto', async ({ browser, baseURL }) => {
    const contexto = await browser.newContext({ javaScriptEnabled: false, baseURL });
    const pagina = await contexto.newPage();
    await pagina.goto('/');
    const terminal = pagina.locator('[data-terminal]');
    await expect(terminal).toContainText('puerto generate scaffold Product');
    await expect(terminal).toContainText('Zero manual wiring');
    await contexto.close();
  });

  test('cambiar la carga y los flags cambia los comandos y el árbol', async ({ page }) => {
    await page.goto('/');
    await page.locator('label.term__cargo-option', { hasText: 'Order' }).click();
    await page.locator('.term__pill', { hasText: 'kafka' }).click();

    const comandos = page.locator('[data-commands]');
    await expect(comandos).toContainText('--events kafka');
    await expect(comandos).toContainText('puerto generate scaffold Order');

    if (await page.locator('[data-tab="tree"]').isVisible()) await page.locator('[data-tab="tree"]').click();
    await expect(page.locator('[data-tree]')).toContainText('unit_of_work.rs');
    await expect(page.locator('[data-tree]')).toContainText('worker');
  });

  test('--events y --auth local se desactivan sin --db, como en el CLI', async ({ page }) => {
    await page.goto('/');
    await page.locator('.term__pill', { hasText: 'db' }).first().click(); // --events db
    await page.locator('label.term__flag', { hasText: '--db' }).click(); // quita --db
    await expect(page.locator('input[name="events"][value="db"]')).toBeDisabled();
    await expect(page.locator('input[name="auth"][value="local"]')).toBeDisabled();
    await expect(page.locator('input[name="events"][value=""]')).toBeChecked();
    await expect(page.locator('[data-commands]')).toContainText('--no-db');
  });

  test('abrir un fichero enseña su código resaltado', async ({ page }) => {
    await page.goto('/');
    if (await page.locator('[data-tab="tree"]').isVisible()) await page.locator('[data-tab="tree"]').click();
    await page.locator('.term__row', { hasText: 'model.rs' }).first().click();
    await expect(page.locator('[data-file-path]')).toHaveText('business/src/domain/product/model.rs');
    await expect(page.locator('[data-file-body] pre.shiki')).toContainText('pub struct ProductId(Uuid);');
  });

  test('«Whole project» enseña también lo que escribió puerto new', async ({ page }) => {
    await page.goto('/');
    if (await page.locator('[data-tab="tree"]').isVisible()) await page.locator('[data-tab="tree"]').click();
    await expect(page.locator('[data-tree]')).not.toContainText('rust-toolchain.toml');
    await page.locator('.term__pill', { hasText: 'Whole project' }).click();
    await expect(page.locator('[data-tree]')).toContainText('rust-toolchain.toml');
  });

  test('la combinación vive en la URL: se puede enlazar y se restaura', async ({ page }) => {
    await page.goto('/?cargo=customer&events=db&auth=local');
    await expect(page.locator('input[name="entity"][value="customer"]')).toBeChecked();
    await expect(page.locator('[data-commands]')).toContainText('--events db --auth local');
    await expect(page.locator('[data-commands]')).toContainText('puerto generate scaffold Customer');

    await page.locator('label.term__cargo-option', { hasText: 'Order' }).click();
    await expect(page).toHaveURL(/cargo=order/);
  });
});
