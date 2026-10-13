import { defineConfig, devices } from '@playwright/test';

/**
 * Un puerto propio por proyecto, y no el 4321 de todo el mundo.
 *
 * En Astro 7 `astro preview` **siempre** arranca en segundo plano y el comando vuelve
 * enseguida, así que Playwright no puede tenerlo de hijo: `reuseExistingServer` tiene que
 * estar en `true` por narices. Y ahí está la trampa, porque Playwright solo comprueba que
 * *algo* responda en la URL:
 *
 * - Si hay un preview de OTRO proyecto en ese puerto, la suite entera se ejecuta contra
 *   la web equivocada. Pasó montando este starter: 70 tests en verde contra Lysis, sin un
 *   solo aviso. Lo arregla el puerto propio.
 * - Si hay un preview viejo de ESTE proyecto, se testea un `dist/` obsoleto. Lo arregla
 *   `test:e2e` en `package.json`, que reconstruye y reinicia el demonio antes de empezar.
 *
 * Las dos cosas van juntas. Corre siempre `pnpm test:e2e`, no `playwright test` a pelo.
 *
 * ── QUÉ SE CAMBIA POR PROYECTO ────────────────────────────────────────────────
 * `PUERTO`, aquí y en el script `test:e2e`. Elige uno y no lo repitas entre clientes.
 */
const PUERTO = 4347;

export default defineConfig({
  testDir: './tests/e2e',
  fullyParallel: true,
  forbidOnly: !!process.env.CI,
  retries: process.env.CI ? 2 : 0,
  reporter: process.env.CI ? 'github' : 'list',
  use: {
    baseURL: `http://localhost:${PUERTO}`,
    trace: 'on-first-retry',
  },
  projects: [
    {
      name: 'escritorio',
      use: {
        ...devices['Desktop Chrome'],
        // Solo hace falta si el proyecto lleva Relieve: sin GPU, WebGL no arranca y los
        // tests de la escena darían un falso negativo. No estorba si no lo lleva.
        // → kit/packages/motion/
        launchOptions: {
          args: ['--use-gl=angle', '--use-angle=swiftshader', '--enable-unsafe-swiftshader'],
        },
      },
    },
    // El móvil no es un caso secundario: en la mayoría de nuestros clientes es de donde
    // llega el grueso del tráfico. Se testea siempre, no cuando sobra tiempo.
    { name: 'movil', use: { ...devices['Pixel 7'] } },
  ],
  webServer: {
    command: `pnpm preview --port ${PUERTO}`,
    url: `http://localhost:${PUERTO}`,
    // Obligatorio en Astro 7: el preview se demoniza. Ver la nota de PUERTO.
    reuseExistingServer: true,
    timeout: 60_000,
  },
});
