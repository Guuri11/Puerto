#!/usr/bin/env node
/**
 * La imagen social (public/og.png, 1200×630) y el icono de iOS (public/apple-touch-icon.png),
 * fotografiados con Playwright a partir de HTML con las fuentes de la marca. Versionado para
 * poder rehacerlos cuando cambie la marca o el lema. → .claude/rules/motion.md (el fotograma
 * se fotografía, no se exporta)
 *
 *   pnpm brand:images
 */
import { chromium } from '@playwright/test';
import { readFileSync } from 'node:fs';
import { join, resolve } from 'node:path';

const LANDING = resolve(import.meta.dirname, '..');
const font = (pkg, file) =>
  `data:font/woff2;base64,${readFileSync(join(LANDING, 'node_modules/@fontsource-variable', pkg, 'files', file)).toString('base64')}`;

const FONTS = `
  @font-face { font-family: 'BS'; src: url(${font('big-shoulders', 'big-shoulders-latin-wght-normal.woff2')}); font-weight: 100 900; }
  @font-face { font-family: 'BSS'; src: url(${font('big-shoulders-stencil', 'big-shoulders-stencil-latin-wght-normal.woff2')}); font-weight: 100 900; }
  @font-face { font-family: 'MM'; src: url(${font('martian-mono', 'martian-mono-latin-wdth-normal.woff2')}); font-weight: 100 800; font-stretch: 75% 112.5%; }
  @font-face { font-family: 'AR'; src: url(${font('archivo', 'archivo-latin-wdth-normal.woff2')}); font-weight: 100 900; font-stretch: 62% 125%; }
`;

/** El símbolo: tres contenedores idénticos, como src/components/Logo.astro. */
const symbol = (size) => `
  <svg viewBox="0 0 32 32" width="${size}" height="${size}">
    ${[
      ['#2f6b4f', 4, 3],
      ['#a33b20', 8, 12],
      ['#1d4e89', 2, 21],
    ]
      .map(
        ([c, x, y]) =>
          `<rect x="${x}" y="${y}" width="22" height="8" fill="${c}"/>` +
          [4, 7, 10, 13, 16].map((r) => `<rect x="${x + r}" y="${y + 1.5}" width="1" height="5" fill="#e9e5dc" opacity=".28"/>`).join(''),
      )
      .join('')}
  </svg>`;

const OG = `<!doctype html><html><head><style>${FONTS}
  * { margin: 0; box-sizing: border-box; }
  body { width: 1200px; height: 630px; background: #e9e5dc; color: #15171a; font-family: 'AR'; position: relative; overflow: hidden; }
  .brand { position: absolute; top: 56px; left: 72px; display: flex; align-items: center; gap: 16px; }
  .word { font-family: 'BSS'; font-weight: 800; font-size: 44px; letter-spacing: .04em; }
  h1 { position: absolute; left: 68px; top: 150px; font-family: 'BS'; font-weight: 800; font-size: 132px; line-height: .82; text-transform: uppercase; }
  .side { position: absolute; right: 72px; bottom: 112px; width: 400px; }
  .side p { font-size: 27px; line-height: 1.35; color: #5a574f; }
  .cmd { margin-top: 26px; background: #101215; color: #e6e1d6; border: 3px solid #15171a; padding: 16px 20px; font-family: 'MM'; font-stretch: 87.5%; font-size: 22px; }
  .cmd span { color: #9a968c; }
  .hazard { position: absolute; left: 0; right: 0; bottom: 0; height: 40px; border-top: 3px solid #15171a;
    background: repeating-linear-gradient(-45deg, #f5b700 0 20px, #15171a 20px 40px); }
</style></head><body>
  <div class="brand">${symbol(64)}<span class="word">PUERTO</span></div>
  <h1>Scaffold.<br>Structure.<br>Ship.</h1>
  <div class="side">
    <p>The Rust framework that generates Domain-Driven Design backends.</p>
    <div class="cmd"><span>$</span> cargo install puerto</div>
  </div>
  <div class="hazard"></div>
</body></html>`;

const ICON = `<!doctype html><html><head><style>
  * { margin: 0; } body { width: 180px; height: 180px; background: #e9e5dc; display: grid; place-items: center; }
</style></head><body>${symbol(132)}</body></html>`;

const browser = await chromium.launch();
for (const [html, w, h, out] of [
  [OG, 1200, 630, 'public/og.png'],
  [ICON, 180, 180, 'public/apple-touch-icon.png'],
]) {
  const page = await browser.newPage({ viewport: { width: w, height: h } });
  await page.setContent(html);
  await page.evaluate(() => document.fonts.ready);
  await page.screenshot({ path: join(LANDING, out) });
  await page.close();
}
await browser.close();
console.log('brand-images: public/og.png, public/apple-touch-icon.png');
