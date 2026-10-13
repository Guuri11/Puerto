/**
 * La cosecha del CLI (`pnpm harvest`) leída en build: qué proyectos hay, qué ficheros tiene cada
 * uno y en qué lenguaje resaltar cada fichero. La terminal de la home y el endpoint de resaltado
 * la usan; el navegador solo descarga el JSON de la combinación que se elige.
 */
import { readdirSync, readFileSync } from 'node:fs';
import { join } from 'node:path';
import meta from '@/data/harvest/meta.json';

const CARGO_DIR = join(process.cwd(), 'public/cargo');

export interface CargoFile {
  path: string;
  hash: string;
  status: 'added' | 'changed' | 'kept';
  lines: number;
}

export interface Cargo {
  id: string;
  commands: string[];
  stdout: { new: string; scaffold: string };
  files: CargoFile[];
}

export type Flags = (typeof meta.matrix)[number]['flags'];

export const entities = meta.entities;
export const matrix = meta.matrix;
export const harvestVersion = meta.version;

export function readCargo(id: string): Cargo {
  return JSON.parse(readFileSync(join(CARGO_DIR, `${id}.json`), 'utf8'));
}

export function readBlob(hash: string): string {
  return readFileSync(join(CARGO_DIR, 'blobs', `${hash}.txt`), 'utf8');
}

/** Primera ruta con la que aparece cada contenido: decide su lenguaje. */
export function blobPaths(): Map<string, string> {
  const paths = new Map<string, string>();
  for (const file of readdirSync(CARGO_DIR).filter((f) => f.endsWith('.json'))) {
    for (const f of readCargo(file.replace(/\.json$/, '')).files) {
      if (!paths.has(f.hash)) paths.set(f.hash, f.path);
    }
  }
  return paths;
}

const BY_EXTENSION: Record<string, string> = {
  rs: 'rust',
  ts: 'typescript',
  tsx: 'tsx',
  toml: 'toml',
  md: 'markdown',
  json: 'json',
  sql: 'sql',
  yml: 'yaml',
  yaml: 'yaml',
  cjs: 'javascript',
  js: 'javascript',
  html: 'html',
  css: 'css',
  'code-snippets': 'json',
};

export function languageOf(path: string): string {
  const name = path.split('/').pop() ?? path;
  if (name === 'Makefile') return 'makefile';
  if (name.startsWith('.env')) return 'dotenv';
  if (name === '.gitignore') return 'txt';
  const ext = name.split('.').pop() ?? '';
  return BY_EXTENSION[ext] ?? 'txt';
}

/** La capa a la que pertenece un fichero, para su color. */
export type Layer = 'business' | 'infrastructure' | 'presentation' | 'web' | 'worker' | 'root';

export function layerOf(path: string): Layer {
  const top = path.split('/')[0];
  if (top === 'business' || top === 'infrastructure' || top === 'presentation') return top;
  if (top === 'web' || top === 'worker') return top;
  return 'root';
}
