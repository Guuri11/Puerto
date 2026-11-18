#!/usr/bin/env node
/**
 * Copia a las docs lo que ya está escrito en el repo, para no escribirlo dos veces:
 *
 *   ../docs/adr/NNNN-*.md  →  src/content/docs/docs/decisions/adr/NNNN-*.md
 *   ../CHANGELOG.md        →  src/content/docs/docs/changelog.md
 *
 * Corre antes de cada `dev` y `build`. Lo que escribe está en .gitignore: la fuente es el
 * repo, y estos ficheros se regeneran.
 *
 * Reescribe los enlaces entre ADRs (`0012-….md` → `/docs/decisions/adr/0012-…/`) y los que
 * apuntan a ficheros del repo (`PLAN.md`) hacia GitHub.
 */
import { mkdirSync, readdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { join, resolve } from 'node:path';

const LANDING = resolve(import.meta.dirname, '..');
const ROOT = resolve(LANDING, '..');
const GITHUB = 'https://github.com/Guuri11/Puerto/blob/main';
const OUT = join(LANDING, 'src/content/docs/docs');
const ADR_OUT = join(OUT, 'decisions/adr');

const yaml = (s) => JSON.stringify(s);

/** Enlaces relativos del Markdown del repo → rutas de la web o de GitHub. */
function rewriteLinks(md, fromDir) {
  return md.replace(/\]\(([^)\s]+)\)/g, (whole, target) => {
    if (/^(https?:|mailto:|#)/.test(target)) return whole;
    const [path, anchor = ''] = target.split('#');
    const hash = anchor ? `#${anchor}` : '';
    const adr = path.match(/(?:^|\/)(\d{4}-[^/]+)\.md$/);
    if (adr) return `](/docs/decisions/adr/${adr[1]}/${hash})`;
    if (path === 'README.md' && fromDir === 'docs/adr') return `](/docs/decisions/${hash})`;
    const repoPath = join(fromDir, path).replace(/^\.\//, '');
    return `](${GITHUB}/${repoPath}${hash})`;
  });
}

// ── ADRs ─────────────────────────────────────────────────────────────────────────────────
rmSync(ADR_OUT, { recursive: true, force: true });
mkdirSync(ADR_OUT, { recursive: true });

const files = readdirSync(join(ROOT, 'docs/adr')).filter((f) => /^\d{4}-.+\.md$/.test(f)).sort();
for (const file of files) {
  const text = readFileSync(join(ROOT, 'docs/adr', file), 'utf8');
  const heading = text.match(/^#\s+(\d{4})\s+[—-]\s+(.+)$/m);
  const number = heading?.[1] ?? file.slice(0, 4);
  // Los títulos de los ADR llevan `código`; en la barra lateral y en <title> sobran.
  const title = (heading?.[2] ?? file).replace(/`/g, '').trim();
  // Las ADR en castellano (0023 en adelante) dicen **Estado:** y ## Contexto.
  const status = text.match(/^\*\*(?:Status|Estado):\*\*\s+(.+)$/m)?.[1] ?? '';
  const body = rewriteLinks(text.replace(/^#\s+.+\n/, ''), 'docs/adr');
  // La description sale del primer párrafo del contexto: dice el problema, que es lo que se busca.
  const context = (text.split(/^## Context(?:o)?\s*$/m)[1] ?? '')
    .split(/\n\s*\n/)
    .map((p) => p.replace(/\s+/g, ' ').replace(/[`*]/g, '').replace(/\[([^\]]+)\]\([^)]+\)/g, '$1').trim())
    .find((p) => p && !p.startsWith('|') && !p.startsWith('#'));
  const clip = (s, max) => (s.length <= max ? s : `${s.slice(0, s.lastIndexOf(' ', max - 1))}…`);
  const description = clip(`ADR ${number}. ${context ?? title}`, 158);
  // El <title> lleva « | Puerto» detrás y no debe pasar de 60: los títulos largos se recortan
  // allí, y la página sigue mostrando el título entero como h1.
  const shortTitle = clip(`ADR ${number}: ${title}`, 51);

  writeFileSync(
    join(ADR_OUT, file),
    [
      '---',
      `title: ${yaml(`${number} · ${title}`)}`,
      `description: ${yaml(description)}`,
      ...(shortTitle !== `ADR ${number}: ${title}` ? ['head:', '  - tag: title', `    content: ${yaml(`${shortTitle} | Puerto`)}`] : []),
      `sidebar:`,
      `  label: ${yaml(`${number} ${title}`)}`,
      `editUrl: ${yaml(`https://github.com/Guuri11/Puerto/edit/main/docs/adr/${file}`)}`,
      '---',
      '',
      body.trim(),
      '',
    ].join('\n'),
  );
}

// ── Changelog ────────────────────────────────────────────────────────────────────────────
const changelog = readFileSync(join(ROOT, 'CHANGELOG.md'), 'utf8')
  // El título y la cabecera de keep-a-changelog los pone la página.
  .replace(/^#\s+Changelog.*\n/m, '');
writeFileSync(
  join(OUT, 'changelog.md'),
  [
    '---',
    'title: Changelog',
    `description: ${yaml('Cada release de Puerto, de la más nueva a la más antigua: lo añadido, cambiado y arreglado en la CLI y el runtime, y cómo migrar un proyecto generado.')}`,
    `editUrl: ${yaml('https://github.com/Guuri11/Puerto/edit/main/CHANGELOG.md')}`,
    'tableOfContents:',
    '  maxHeadingLevel: 2',
    '---',
    '',
    rewriteLinks(changelog, '.').trim(),
    '',
  ].join('\n'),
);

// ── Referencia del CLI, desde la ayuda real (src/data/harvest/cli.json, `pnpm harvest`) ─────
const cli = JSON.parse(readFileSync(join(LANDING, 'src/data/harvest/cli.json'), 'utf8'));

/** La ayuda de clap (en castellano): descripción, `Uso:` y las secciones `Argumentos:` / `Opciones:`. */
function parseHelp(text) {
  const lines = text.split('\n');
  const usageAt = lines.findIndex((l) => l.startsWith('Uso: '));
  const about = lines.slice(0, usageAt).join(' ').replace(/\s+/g, ' ').trim();
  const usage = lines[usageAt].replace('Uso: ', '');
  const sections = {};
  let current = null;
  for (const line of lines.slice(usageAt + 1)) {
    const header = line.match(/^(\w[\w ]*):$/);
    if (header) {
      current = sections[header[1]] = [];
      continue;
    }
    if (!current || !line.trim()) continue;
    // Un elemento empieza con 2 a 6 espacios (`-h, --help` o `      --name`); más es continuación.
    const item = line.match(/^ {2,6}(\S.*?)(?:\s{2,}(.*))?$/);
    if (item) current.push({ name: item[1].trim(), text: (item[2] ?? '').trim() });
    else if (current.length) current[current.length - 1].text += ` ${line.trim()}`;
  }
  return { about, usage, sections };
}

const cell = (s) => s.replace(/\|/g, '\\|');
/** Lo que en la ayuda son flags y valores posibles, como código en la tabla. */
const codeify = (s) =>
  cell(s)
    // `name:String`, `opt:<type>`: lo que se escribe en la terminal, como código (y sin que el
    // `<type>` se lea como HTML).
    .replace(/(^|[\s(])([a-z_]+:[\w:<>/!]+|--[a-z][a-z-]*)(?=[\s.,;)]|$)/g, '$1`$2`')
    .replace(/\s*\[valores posibles: ([^\]]+)\]/, (_, v) =>
      `. Valores posibles: ${v.split(', ').map((x) => `\`${x}\``).join(', ')}.`,
    )
    .replace(/\.\. Valores/, '. Valores');

const GUIDES = {
  nuevo: ['Inicio rápido', '/docs/start/quick-start/'],
  'generar scaffold': ['Entidades y campos', '/docs/guides/entities/'],
  'generar caso-de-uso': ['Casos de uso', '/docs/guides/use-cases/'],
  'generar objeto-valor': ['Objetos valor', '/docs/guides/value-objects/'],
  'generar migracion': ['Base de datos y migraciones', '/docs/guides/database/'],
  'generar evento': ['Eventos de dominio', '/docs/guides/events/'],
  'generar consumidor': ['Consumidores', '/docs/guides/events/#consumidores'],
  'generar correo': ['Correo', '/docs/guides/mail/'],
  'generar web': ['El frontend web', '/docs/guides/web/'],
  'generar dominio': ['Capa a capa', '/docs/guides/layer-by-layer/'],
  'generar aplicacion': ['Capa a capa', '/docs/guides/layer-by-layer/'],
  'generar repositorio': ['Capa a capa', '/docs/guides/layer-by-layer/'],
  'generar presentacion': ['Capa a capa', '/docs/guides/layer-by-layer/'],
  'generar bootstrap': ['Regenerar el cableado', '/docs/guides/layer-by-layer/#regenerar-el-cableado'],
  'generar snippets': ['Snippets del IDE', '/docs/guides/configuration/#snippets-del-ide'],
  listar: ['puerto.toml', '/docs/reference/puerto-toml/'],
  validar: ['Lo que comprueba puerto validar', '/docs/reference/puerto-toml/#lo-que-comprueba-puerto-validar'],
};

const commandDoc = ({ command, help }) => {
  const { about, usage, sections } = parseHelp(help);
  const out = [`## \`puerto ${command}\``, '', about.endsWith('.') ? about : `${about}.`, ''];
  out.push('```bash', usage, '```', '');
  const args = sections.Argumentos ?? [];
  if (args.length) {
    out.push('| Argumento | |', '|---|---|');
    for (const a of args) out.push(`| \`${cell(a.name)}\` | ${codeify(a.text)} |`);
    out.push('');
  }
  const opts = (sections.Opciones ?? []).filter((o) => !o.name.startsWith('-h'));
  if (opts.length) {
    out.push('| Opción | |', '|---|---|');
    for (const o of opts) out.push(`| \`${cell(o.name)}\` | ${codeify(o.text)} |`);
    out.push('');
  }
  if (GUIDES[command]) out.push(`Ver la guía: [${GUIDES[command][0]}](${GUIDES[command][1]}).`, '');
  return out.join('\n');
};

const project = cli.commands.filter((c) => !c.command.startsWith('generar '));
const generators = cli.commands.filter((c) => c.command.startsWith('generar '));
writeFileSync(
  join(OUT, 'reference/cli.md'),
  [
    '---',
    'title: CLI',
    `description: ${yaml('Todos los comandos y opciones de puerto, generados desde la ayuda de la CLI publicada: nuevo, los generadores, listar, validar y completions.')}`,
    'tableOfContents:',
    '  maxHeadingLevel: 2',
    '---',
    '',
    `Generada desde el \`--help\` de \`puerto ${cli.version}\`, así que dice exactamente lo que dice la CLI.`,
    'Todo comando que escribe Rust termina ejecutando `cargo fmt` en el proyecto.',
    '',
    '```bash',
    'cargo install puerto --locked',
    'puerto --help',
    '```',
    '',
    ...project.map(commandDoc),
    '## Generadores',
    '',
    'Todo generador lee y actualiza `puerto.toml`, se niega a sobrescribir ficheros que ya',
    'existen (`scaffold --forzar` y `web --forzar` son las excepciones explícitas) e imprime el',
    'siguiente paso.',
    '',
    ...generators.map((c) => commandDoc(c).replace(/^## /, '### ')),
  ].join('\n'),
);

// ── Referencia de `make`, desde los Makefiles de los 66 proyectos de la cosecha ────────────
// Cada target dice en qué proyectos existe: se deduce de en qué combinaciones de flags aparece.
const meta = JSON.parse(readFileSync(join(LANDING, 'src/data/harvest/meta.json'), 'utf8'));
const targetsIn = new Map(); // target → { help, combos: Set }
for (const m of meta.matrix) {
  const combo = JSON.parse(readFileSync(join(LANDING, 'public/cargo', `${m.id}.json`), 'utf8'));
  const makefile = combo.files.find((f) => f.path === 'Makefile');
  const text = readFileSync(join(LANDING, 'public/cargo/blobs', `${makefile.hash}.txt`), 'utf8');
  for (const [, target, help] of text.matchAll(/^([a-z][\w/.-]*):[^\n]*?##\s*(.+)$/gm)) {
    if (!targetsIn.has(target)) targetsIn.set(target, { help, combos: new Set() });
    targetsIn.get(target).combos.add(m.id);
  }
}
const CONDITIONS = [
  ['todo proyecto', () => true],
  ['`web/`', (f) => f.web],
  ['`--db`', (f) => f.db],
  ['`--eventos`', (f) => !!f.events],
  ['`--auth`', (f) => !!f.auth],
  ['`--auth oidc`', (f) => f.auth === 'oidc'],
  ['`--auth local`', (f) => f.auth === 'local'],
  ['`--db` o `--auth oidc`', (f) => f.db || f.auth === 'oidc'],
];
const when = ({ combos }) => {
  for (const [label, test] of CONDITIONS) {
    const expected = meta.matrix.filter((m) => test(m.flags)).map((m) => m.id);
    if (expected.length === combos.size && expected.every((id) => combos.has(id))) return label;
  }
  return 'algunos proyectos';
};
const makeRows = [...targetsIn]
  .filter(([t]) => t !== 'help')
  .map(([t, info]) => `| \`make ${t}\` | ${cell(info.help.trim())} | ${when(info)} |`);
writeFileSync(
  join(OUT, 'reference/make.md'),
  [
    '---',
    'title: Targets de make',
    `description: ${yaml('Todos los targets de make de un proyecto Puerto generado, qué hacen y qué proyectos los tienen, leídos de los Makefiles que escribe la CLI.')}`,
    '---',
    '',
    'El `Makefile` de un proyecto generado es su línea de comandos. `make help` lista los targets de',
    'ese proyecto; esta página los lista todos, leídos de los Makefiles de cada combinación de flags',
    `de \`puerto nuevo\` que genera \`puerto ${meta.version}\`.`,
    '',
    '| Target | Qué hace | En |',
    '|---|---|---|',
    ...makeRows,
    '| `make correo/ui` | Dónde llegan los correos enviados en desarrollo (Mailpit) | tras `puerto generar correo` |',
    '',
    'Los targets que necesitan un servicio (`sqlx/migrate`, `worker`, `test/infraestructura`…)',
    'arrancan antes `docker compose`. `make test` y `make lint` incluyen `web/` siempre que exista el directorio.',
    '',
  ].join('\n'),
);

console.log(`sync-repo-docs: ${files.length} ADR, el changelog, las referencias de la CLI y de make.`);
