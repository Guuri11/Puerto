#!/usr/bin/env node
/**
 * Runs the real `puerto` CLI and stores what it writes, so the site never shows output that the
 * published version would not produce.
 *
 *   pnpm harvest                       # uses ../target/release/puerto
 *   PUERTO_BIN=$(which puerto) pnpm harvest
 *
 * Writes (all committed — Vercel has no `puerto` to run):
 *   src/data/harvest/meta.json         version and the matrix
 *   src/data/harvest/cli.json          `--help` of every command
 *   public/cargo/<combo>.json          one file per (project flags × entity): tree, stdout
 *   public/cargo/blobs/<hash>.txt      file contents, content-addressed (shared across combos)
 *
 * Run it after every release: `meta.json` carries the version, and a test fails when it differs
 * from `[workspace.package] version` in ../Cargo.toml.
 */
import { execFileSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { mkdirSync, mkdtempSync, readdirSync, readFileSync, rmSync, statSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, relative, resolve } from 'node:path';

const LANDING = resolve(import.meta.dirname, '..');
const REPO = resolve(LANDING, '..');
const BIN = process.env.PUERTO_BIN ?? join(REPO, 'target/release/puerto');
const OUT_DATA = join(LANDING, 'src/data/harvest');
const OUT_PUBLIC = join(LANDING, 'public/cargo');

const run = (args, cwd) =>
  execFileSync(BIN, args, { cwd, encoding: 'utf8', stdio: ['ignore', 'pipe', 'pipe'] });

// ── Version guard ──────────────────────────────────────────────────────────────────────────
const workspaceVersion = readFileSync(join(REPO, 'Cargo.toml'), 'utf8').match(
  /\[workspace\.package\][^[]*?version\s*=\s*"([^"]+)"/s,
)?.[1];
const binVersion = run(['--version']).trim().replace(/^puerto\s+/, '');
if (binVersion !== workspaceVersion) {
  console.error(`puerto at ${BIN} is ${binVersion}, the workspace is ${workspaceVersion}.`);
  console.error('Build it first: cargo build --release -p puerto');
  process.exit(1);
}

// ── CLI reference ──────────────────────────────────────────────────────────────────────────
const COMMANDS = [
  ['new'],
  ['generate', 'scaffold'],
  ['generate', 'use-case'],
  ['generate', 'value-object'],
  ['generate', 'migration'],
  ['generate', 'event'],
  ['generate', 'consumer'],
  ['generate', 'mail'],
  ['generate', 'web'],
  ['generate', 'domain'],
  ['generate', 'application'],
  ['generate', 'repository'],
  ['generate', 'presentation'],
  ['generate', 'bootstrap'],
  ['generate', 'snippets'],
  ['list'],
  ['validate'],
  ['completions'],
];
const cli = {
  root: run(['--help']),
  generate: run(['generate', '--help']),
  commands: COMMANDS.map((c) => ({ command: c.join(' '), help: run([...c, '--help']) })),
};

// ── The cargo matrix ───────────────────────────────────────────────────────────────────────
/**
 * The entities a visitor can load. Each shows something the others do not: a unique field,
 * value objects with an enum, an optional and a vector.
 */
const ENTITIES = [
  { id: 'product', name: 'Product', fields: ['name:String', 'price:i64', 'sku:String!'] },
  {
    id: 'order',
    name: 'Order',
    fields: ['amount:Amount:f64', 'status:Status:enum:Pending/Confirmed/Cancelled', 'note:opt:String'],
  },
  {
    id: 'customer',
    name: 'Customer',
    fields: ['email:Email:String!', 'name:String', 'tags:Tag:vec:String'],
  },
];

/** Every valid combination of `puerto new` flags. `--events` and `--auth local` need `--db`. */
function combos() {
  const out = [];
  for (const db of [false, true])
    for (const events of db ? [null, 'db', 'kafka'] : [null])
      for (const auth of db ? [null, 'oidc', 'local'] : [null, 'oidc'])
        for (const web of [true, false]) out.push({ db, events, auth, web });
  return out;
}

const comboId = ({ db, events, auth, web }) =>
  [db ? 'db' : 'nodb', events ? `ev-${events}` : 'noev', auth ? `auth-${auth}` : 'noauth', web ? 'web' : 'noweb'].join('_');

function newArgs({ db, events, auth, web }) {
  const args = ['new', '--name', 'shop', db ? '--db' : '--no-db', '--no-demo'];
  if (!web) args.push('--no-web');
  if (events) args.push('--events', events);
  if (auth) args.push('--auth', auth);
  return args;
}

function scaffoldArgs(entity, { auth }) {
  const args = ['generate', 'scaffold', entity.name];
  // In auth projects, show the two flags that shape the routes: reads stay open, deletes need admin.
  if (auth) {
    const plural = `${entity.id}s`;
    args.push('--public', `list_${plural},get_${entity.id}`, '--role', `delete_${entity.id}=admin`);
  }
  return [...args, '--', ...entity.fields];
}

const SKIP_DIRS = new Set(['.git', 'node_modules', 'target']);
const SKIP_FILES = new Set(['bun.lock', 'bun.lockb', 'Cargo.lock']);

function walk(dir, root = dir, acc = new Map()) {
  for (const name of readdirSync(dir)) {
    const full = join(dir, name);
    const st = statSync(full);
    if (st.isDirectory()) {
      if (!SKIP_DIRS.has(name)) walk(full, root, acc);
    } else if (!SKIP_FILES.has(name)) {
      acc.set(relative(root, full), readFileSync(full));
    }
  }
  return acc;
}

/**
 * Migrations are named after the second they were written, so two runs never agree. Fold the
 * timestamp to a fixed one per file order: the site shows a stable tree, and the harvest diff
 * after a release shows real changes only.
 */
function normalizeMigrations(files) {
  const out = new Map();
  const migrations = [...files.keys()].filter((p) => /migrations\/\d{14}_/.test(p)).sort();
  const renamed = new Map(
    migrations.map((p, i) => [p, p.replace(/\d{14}_/, `2026010100${String(i).padStart(4, '0')}_`)]),
  );
  for (const [p, buf] of files) out.set(renamed.get(p) ?? p, buf);
  return out;
}

const hash = (buf) => createHash('sha256').update(buf).digest('hex').slice(0, 16);
const isText = (buf) => !buf.subarray(0, 8000).includes(0);

rmSync(OUT_PUBLIC, { recursive: true, force: true });
mkdirSync(join(OUT_PUBLIC, 'blobs'), { recursive: true });
mkdirSync(OUT_DATA, { recursive: true });

const blobs = new Set();
const matrix = [];
const scratch = mkdtempSync(join(tmpdir(), 'puerto-harvest-'));

for (const flags of combos()) {
  for (const entity of ENTITIES) {
    const id = `${comboId(flags)}__${entity.id}`;
    const dir = join(scratch, id);
    mkdirSync(dir);
    const nArgs = newArgs(flags);
    const newOut = run([...nArgs, '--destination', '.'], dir);
    const project = join(dir, 'shop');
    const before = normalizeMigrations(walk(project));
    const sArgs = scaffoldArgs(entity, flags);
    const scaffoldOut = run(sArgs, project);
    const after = normalizeMigrations(walk(project));

    const files = [];
    for (const [path, buf] of [...after].sort(([a], [b]) => a.localeCompare(b))) {
      if (!isText(buf)) continue;
      const h = hash(buf);
      if (!blobs.has(h)) {
        writeFileSync(join(OUT_PUBLIC, 'blobs', `${h}.txt`), buf);
        blobs.add(h);
      }
      const prev = before.get(path);
      const status = !prev ? 'added' : prev.equals(buf) ? 'kept' : 'changed';
      files.push({ path, hash: h, status, lines: buf.toString('utf8').split('\n').length });
    }

    writeFileSync(
      join(OUT_PUBLIC, `${id}.json`),
      JSON.stringify({
        id,
        commands: [`puerto ${nArgs.join(' ')}`, `puerto ${sArgs.join(' ')}`],
        stdout: { new: newOut, scaffold: scaffoldOut },
        files,
      }),
    );
    matrix.push({
      id,
      flags,
      entity: entity.id,
      total: files.length,
      added: files.filter((f) => f.status === 'added').length,
      changed: files.filter((f) => f.status === 'changed').length,
    });
    process.stdout.write('.');
  }
}
rmSync(scratch, { recursive: true, force: true });

writeFileSync(
  join(OUT_DATA, 'meta.json'),
  JSON.stringify(
    {
      version: binVersion,
      entities: ENTITIES.map(({ id, name, fields }) => ({ id, name, fields })),
      matrix,
    },
    null,
    2,
  ),
);
writeFileSync(join(OUT_DATA, 'cli.json'), JSON.stringify({ version: binVersion, ...cli }, null, 2));

console.log(`\npuerto ${binVersion}: ${matrix.length} projects, ${blobs.size} distinct files.`);
