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
    await expect(terminal).toContainText('puerto generar scaffold Producto');
    await expect(terminal).toContainText('Sin cablear nada a mano');
    await contexto.close();
  });

  test('cambiar la carga y los flags cambia los comandos y el árbol', async ({ page }) => {
    await page.goto('/');
    await page.locator('label.term__cargo-option', { hasText: 'Pedido' }).click();
    await page.locator('.term__pill', { hasText: 'kafka' }).click();

    const comandos = page.locator('[data-commands]');
    await expect(comandos).toContainText('--eventos kafka');
    await expect(comandos).toContainText('puerto generar scaffold Pedido');

    if (await page.locator('[data-tab="tree"]').isVisible()) await page.locator('[data-tab="tree"]').click();
    await expect(page.locator('[data-tree]')).toContainText('unit_of_work.rs');
    await expect(page.locator('[data-tree]')).toContainText('worker');
  });

  test('--eventos y --auth local se desactivan sin --db, como en el CLI', async ({ page }) => {
    await page.goto('/');
    await page.locator('.term__pill', { hasText: 'db' }).first().click(); // --eventos db
    await page.locator('label.term__flag', { hasText: '--db' }).click(); // quita --db
    await expect(page.locator('input[name="events"][value="db"]')).toBeDisabled();
    await expect(page.locator('input[name="auth"][value="local"]')).toBeDisabled();
    await expect(page.locator('input[name="events"][value=""]')).toBeChecked();
    await expect(page.locator('[data-commands]')).toContainText('--sin-db');
  });

  test('abrir un fichero enseña su código resaltado', async ({ page }) => {
    await page.goto('/');
    if (await page.locator('[data-tab="tree"]').isVisible()) await page.locator('[data-tab="tree"]').click();
    await page.locator('.term__row', { hasText: 'modelo.rs' }).first().click();
    await expect(page.locator('[data-file-path]')).toHaveText('contextos/producto/dominio/src/modelo.rs');
    await expect(page.locator('[data-file-body] pre.shiki')).toContainText('pub struct IdProducto(Uuid);');
  });

  test('«Whole project» enseña también lo que escribió puerto nuevo', async ({ page }) => {
    await page.goto('/');
    if (await page.locator('[data-tab="tree"]').isVisible()) await page.locator('[data-tab="tree"]').click();
    await expect(page.locator('[data-tree]')).not.toContainText('rust-toolchain.toml');
    await page.locator('.term__pill', { hasText: 'Todo el proyecto' }).click();
    await expect(page.locator('[data-tree]')).toContainText('rust-toolchain.toml');
  });

  test('la combinación vive en la URL: se puede enlazar y se restaura', async ({ page }) => {
    await page.goto('/?cargo=cliente&events=db&auth=local');
    await expect(page.locator('input[name="entity"][value="cliente"]')).toBeChecked();
    await expect(page.locator('[data-commands]')).toContainText('--eventos db --auth local');
    await expect(page.locator('[data-commands]')).toContainText('puerto generar scaffold Cliente');

    await page.locator('label.term__cargo-option', { hasText: 'Pedido' }).click();
    await expect(page).toHaveURL(/cargo=pedido/);
  });
});
