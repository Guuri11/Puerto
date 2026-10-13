#!/usr/bin/env node
/**
 * La marca del repositorio (../assets/brand/), con la identidad de la web: el README de GitHub,
 * la imagen social del repo y los iconos. → DESIGN.md § 4
 *
 *   pnpm brand:repo
 *
 * GitHub enseña los SVG como <img>, y un <img> no carga fuentes externas: cada SVG lleva
 * incrustadas, en base64, solo las letras que usa (pyftsubset, de fonttools). Los PNG se
 * fotografían de esos mismos SVG con Playwright.
 */
import { chromium } from '@playwright/test';
import { execFileSync } from 'node:child_process';
import { copyFileSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';

const LANDING = resolve(import.meta.dirname, '..');
const OUT = resolve(LANDING, '../assets/brand');
const FONTS = join(LANDING, 'node_modules/@fontsource-variable');
const tmp = mkdtempSync(join(tmpdir(), 'puerto-brand-'));

const C = {
  canvas: '#e9e5dc',
  ink: '#15171a',
  muted: '#5a574f',
  night: '#101215',
  nightInk: '#e6e1d6',
  nightMuted: '#9a968c',
  accent: '#f5b700',
  day: { presentation: '#2f6b4f', infrastructure: '#a33b20', business: '#1d4e89' },
  dark: { presentation: '#6dbe93', infrastructure: '#e58a6a', business: '#6fa3e0' },
};

const FILES = {
  display: 'big-shoulders/files/big-shoulders-latin-wght-normal.woff2',
  stencil: 'big-shoulders-stencil/files/big-shoulders-stencil-latin-wght-normal.woff2',
  sans: 'archivo/files/archivo-latin-wdth-normal.woff2',
  mono: 'martian-mono/files/martian-mono-latin-wdth-normal.woff2',
};

/** @font-face con solo los glifos de `text`, para incrustar en un SVG. */
function fontFace(family, key, text) {
  const out = join(tmp, `${key}-${Math.random().toString(36).slice(2)}.woff2`);
  execFileSync('pyftsubset', [
    join(FONTS, FILES[key]),
    `--text=${text}`,
    '--flavor=woff2',
    '--layout-features=*',
    `--output-file=${out}`,
  ]);
  const data = readFileSync(out).toString('base64');
  return `@font-face{font-family:'${family}';src:url(data:font/woff2;base64,${data}) format('woff2');font-weight:100 900;font-stretch:62% 125%}`;
}

/** Los tres contenedores idénticos, como src/components/Logo.astro. */
function symbol({ x = 0, y = 0, scale = 1, colors = C.day, ribs = true, ribColor = C.canvas }) {
  const boxes = [
    ['presentation', 4, 3],
    ['infrastructure', 8, 12],
    ['business', 2, 21],
  ];
  return `<g transform="translate(${x} ${y}) scale(${scale})">${boxes
    .map(([layer, bx, by]) => {
      const rib = ribs
        ? [4, 7, 10, 13, 16]
            .map((r) => `<rect x="${bx + r}" y="${by + 1.5}" width="1" height="5" fill="${ribColor}" opacity=".3"/>`)
            .join('')
        : '';
      return `<rect x="${bx}" y="${by}" width="22" height="8" fill="${colors[layer]}"/>${rib}`;
    })
    .join('')}</g>`;
}

const svg = (w, h, body, style = '') =>
  `<svg xmlns="http://www.w3.org/2000/svg" width="${w}" height="${h}" viewBox="0 0 ${w} ${h}">` +
  (style ? `<style>${style}</style>` : '') +
  body +
  '</svg>\n';

// ── Iconos ──────────────────────────────────────────────────────────────────────────────────
const icon = svg(64, 64, `<rect width="64" height="64" fill="${C.canvas}"/>${symbol({ scale: 2 })}`);
writeFileSync(join(OUT, 'logo-icon.svg'), icon);
copyFileSync(join(LANDING, 'public/favicon.svg'), join(OUT, 'favicon.svg'));

// ── Logo completo, claro y oscuro (312×80: símbolo + wordmark, sin aire sobrante) ──────────
const word = 'PUERTO';
const logoStyle = (fill) =>
  fontFace('Puerto Stencil', 'stencil', word) +
  `.w{font-family:'Puerto Stencil';font-weight:800;font-size:64px;letter-spacing:.04em;fill:${fill}}`;
const logo = (colors, fill, ribColor) =>
  svg(
    312,
    80,
    `${symbol({ x: 4, y: 4, scale: 2.25, colors, ribColor })}<text class="w" x="96" y="63">${word}</text>`,
    logoStyle(fill),
  );
writeFileSync(join(OUT, 'logo-full.svg'), logo(C.day, C.ink, C.canvas));
writeFileSync(join(OUT, 'logo-full-dark.svg'), logo(C.dark, C.nightInk, C.night));

// ── Imagen social del repositorio (1280×640): la misma composición que el og.png de la web ──
const lede = ['The Rust framework that generates', 'Domain-Driven Design backends.'];
const cmd = '$ cargo install puerto';
const hazard = Array.from({ length: 70 }, (_, i) => `<path d="M${i * 40 - 40} 640 l40 -40 h20 l-40 40z" fill="${C.ink}"/>`).join('');
const social = svg(
  1280,
  640,
  `<rect width="1280" height="640" fill="${C.canvas}"/>` +
    symbol({ x: 76, y: 52, scale: 2.25 }) +
    `<text class="w" x="160" y="104">${word}</text>` +
    `<text class="h"><tspan x="70" y="268">SCAFFOLD.</tspan><tspan x="70" y="380">STRUCTURE.</tspan><tspan x="70" y="492">SHIP.</tspan></text>` +
    `<text class="l"><tspan x="800" y="374">${lede[0]}</tspan><tspan x="800" y="412">${lede[1]}</tspan></text>` +
    `<rect x="800" y="446" width="410" height="62" fill="${C.night}" stroke="${C.ink}" stroke-width="3"/>` +
    `<text class="c" x="822" y="486"><tspan fill="${C.nightMuted}">$</tspan> cargo install puerto</text>` +
    `<rect y="597" width="1280" height="43" fill="${C.accent}"/>${hazard}<rect y="596" width="1280" height="3" fill="${C.ink}"/>`,
  fontFace('Puerto Stencil', 'stencil', word) +
    fontFace('Puerto Display', 'display', 'SCAFFOLD.STRUCTURE.SHIP') +
    fontFace('Puerto Sans', 'sans', lede.join(' ')) +
    fontFace('Puerto Mono', 'mono', cmd) +
    `.w{font-family:'Puerto Stencil';font-weight:800;font-size:50px;letter-spacing:.04em;fill:${C.ink}}` +
    `.h{font-family:'Puerto Display';font-weight:800;font-size:138px;fill:${C.ink}}` +
    `.l{font-family:'Puerto Sans';font-size:27px;fill:${C.muted}}` +
    `.c{font-family:'Puerto Mono';font-stretch:87.5%;font-size:22px;fill:${C.nightInk}}`,
);
writeFileSync(join(OUT, 'github-social.svg'), social);

// ── PNG, fotografiados de los SVG ───────────────────────────────────────────────────────────
const browser = await chromium.launch();
for (const [file, w, h, out, scale = 1] of [
  ['github-social.svg', 1280, 640, 'github-social.png'],
  ['logo-icon.svg', 64, 64, 'logo-icon-64.png'],
  ['logo-icon.svg', 64, 64, 'logo-icon-128.png', 2],
  ['favicon.svg', 32, 32, 'favicon.png'],
]) {
  const page = await browser.newPage({ viewport: { width: w, height: h }, deviceScaleFactor: scale });
  await page.goto(`file://${join(OUT, file)}`);
  await page.evaluate(() => document.fonts.ready);
  await page.screenshot({ path: join(OUT, out), omitBackground: file === 'favicon.svg' });
  await page.close();
}
await browser.close();
rmSync(tmp, { recursive: true, force: true });
console.log('repo-brand: ../assets/brand rewritten.');
