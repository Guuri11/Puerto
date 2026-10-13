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
  const status = text.match(/^\*\*Status:\*\*\s+(.+)$/m)?.[1] ?? '';
  const body = rewriteLinks(text.replace(/^#\s+.+\n/, ''), 'docs/adr');
  // La description sale del primer párrafo del contexto: dice el problema, que es lo que se busca.
  const context = (text.split(/^## Context\s*$/m)[1] ?? '')
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
    `description: ${yaml('Every Puerto release, newest first: what was added, changed and fixed in the CLI and the runtime crates, and how to migrate a generated project.')}`,
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

/** La ayuda de clap: descripción, `Usage:` y las secciones `Arguments:` / `Options:`. */
function parseHelp(text) {
  const lines = text.split('\n');
  const usageAt = lines.findIndex((l) => l.startsWith('Usage: '));
  const about = lines.slice(0, usageAt).join(' ').replace(/\s+/g, ' ').trim();
  const usage = lines[usageAt].replace('Usage: ', '');
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
    .replace(/\s*\[possible values: ([^\]]+)\]/, (_, v) =>
      `. Possible values: ${v.split(', ').map((x) => `\`${x}\``).join(', ')}.`,
    )
    .replace(/\.\. Possible/, '. Possible');

const GUIDES = {
  new: ['Quick start', '/docs/start/quick-start/'],
  'generate scaffold': ['Entities and fields', '/docs/guides/entities/'],
  'generate use-case': ['Use cases', '/docs/guides/use-cases/'],
  'generate value-object': ['Value objects', '/docs/guides/value-objects/'],
  'generate migration': ['Database and migrations', '/docs/guides/database/'],
  'generate event': ['Domain events', '/docs/guides/events/'],
  'generate consumer': ['Consumers', '/docs/guides/events/#consumers'],
  'generate mail': ['Mail', '/docs/guides/mail/'],
  'generate web': ['The web frontend', '/docs/guides/web/'],
  'generate domain': ['Layer by layer', '/docs/guides/layer-by-layer/'],
  'generate application': ['Layer by layer', '/docs/guides/layer-by-layer/'],
  'generate repository': ['Layer by layer', '/docs/guides/layer-by-layer/'],
  'generate presentation': ['Layer by layer', '/docs/guides/layer-by-layer/'],
  'generate bootstrap': ['When the wiring is out of date', '/docs/guides/layer-by-layer/#when-the-wiring-is-out-of-date'],
  'generate snippets': ['IDE snippets', '/docs/guides/configuration/#ide-snippets'],
  list: ['puerto.toml', '/docs/reference/puerto-toml/'],
  validate: ['What puerto validate checks', '/docs/reference/puerto-toml/#what-puerto-validate-checks'],
};

const commandDoc = ({ command, help }) => {
  const { about, usage, sections } = parseHelp(help);
  const out = [`## \`puerto ${command}\``, '', about.endsWith('.') ? about : `${about}.`, ''];
  out.push('```bash', usage, '```', '');
  const args = sections.Arguments ?? [];
  if (args.length) {
    out.push('| Argument | |', '|---|---|');
    for (const a of args) out.push(`| \`${cell(a.name)}\` | ${codeify(a.text)} |`);
    out.push('');
  }
  const opts = (sections.Options ?? []).filter((o) => !o.name.startsWith('-h'));
  if (opts.length) {
    out.push('| Option | |', '|---|---|');
    for (const o of opts) out.push(`| \`${cell(o.name)}\` | ${codeify(o.text)} |`);
    out.push('');
  }
  if (GUIDES[command]) out.push(`See the guide: [${GUIDES[command][0]}](${GUIDES[command][1]}).`, '');
  return out.join('\n');
};

const project = cli.commands.filter((c) => !c.command.startsWith('generate '));
const generators = cli.commands.filter((c) => c.command.startsWith('generate '));
writeFileSync(
  join(OUT, 'reference/cli.md'),
  [
    '---',
    'title: CLI',
    `description: ${yaml('Every puerto command and option, generated from the help of the released CLI: new, the generators, list, validate and completions.')}`,
    'tableOfContents:',
    '  maxHeadingLevel: 2',
    '---',
    '',
    `Generated from the \`--help\` of \`puerto ${cli.version}\`, so it says exactly what the CLI says.`,
    'Every command that writes Rust ends by running `cargo fmt` on the project.',
    '',
    '```bash',
    'cargo install puerto --locked',
    'puerto --help',
    '```',
    '',
    ...project.map(commandDoc),
    '## Generators',
    '',
    'Every generator reads and updates `puerto.toml`, refuses to overwrite files that already',
    'exist (`scaffold --force` and `web --force` are the explicit exceptions) and prints the next',
    'step.',
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
  ['every project', () => true],
  ['`web/`', (f) => f.web],
  ['`--db`', (f) => f.db],
  ['`--events`', (f) => !!f.events],
  ['`--auth`', (f) => !!f.auth],
  ['`--auth oidc`', (f) => f.auth === 'oidc'],
  ['`--auth local`', (f) => f.auth === 'local'],
  ['`--db` or `--auth oidc`', (f) => f.db || f.auth === 'oidc'],
];
const when = ({ combos }) => {
  for (const [label, test] of CONDITIONS) {
    const expected = meta.matrix.filter((m) => test(m.flags)).map((m) => m.id);
    if (expected.length === combos.size && expected.every((id) => combos.has(id))) return label;
  }
  return 'some projects';
};
const makeRows = [...targetsIn]
  .filter(([t]) => t !== 'help')
  .map(([t, info]) => `| \`make ${t}\` | ${cell(info.help.trim())} | ${when(info)} |`);
writeFileSync(
  join(OUT, 'reference/make.md'),
  [
    '---',
    'title: Make targets',
    `description: ${yaml('Every make target of a generated Puerto project, what it does and which projects have it, read from the Makefiles the CLI writes.')}`,
    '---',
    '',
    "A generated project's `Makefile` is its command line. `make help` lists the targets of that",
    'project; this page lists them all, read from the Makefiles of every combination of',
    `\`puerto new\` flags that \`puerto ${meta.version}\` generates.`,
    '',
    '| Target | Does | In |',
    '|---|---|---|',
    ...makeRows,
    '| `make mail/ui` | Where the mails sent in development land (Mailpit) | after `puerto generate mail` |',
    '',
    'Targets that need a service (`sqlx/migrate`, `worker`, `test/infrastructure`…) start',
    '`docker compose` first. `make test` and `make lint` include `web/` whenever the directory exists.',
    '',
  ].join('\n'),
);

console.log(`sync-repo-docs: ${files.length} ADRs, the changelog, the CLI and make references.`);
