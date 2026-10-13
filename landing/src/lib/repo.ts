/**
 * Lo que la web lee del repositorio de Puerto en build. Así ningún número de la home se
 * escribe a mano: la versión sale de Cargo.toml y las decisiones de docs/adr.
 *
 * Funciona en Vercel porque el proyecto clona el repo entero y el build corre en
 * `landing/`: `..` es la raíz de Puerto.
 */
import { readdirSync, readFileSync } from 'node:fs';
import { join, resolve } from 'node:path';
import meta from '@/data/harvest/meta.json';

const ROOT = resolve(process.cwd(), '..');

const cargoToml = readFileSync(join(ROOT, 'Cargo.toml'), 'utf8');

export const version =
  cargoToml.match(/\[workspace\.package\][^[]*?version\s*=\s*"([^"]+)"/s)?.[1] ?? meta.version;

/** `0.19` — what a project generated today asks for (`puerto-web = "0.19"`). */
export const minor = version.split('.').slice(0, 2).join('.');

export const rustVersion = cargoToml.match(/rust-version\s*=\s*"([^"]+)"/)?.[1] ?? '';

export const runtimeCrates = [...cargoToml.matchAll(/"crates\/(\w+)"/g)]
  .map((m) => m[1])
  .filter((c) => c !== 'cli')
  .map((c) => `puerto-${c}`);

export interface Adr {
  number: string;
  slug: string;
  title: string;
  status: string;
  date: string;
}

export const adrs: Adr[] = readdirSync(join(ROOT, 'docs/adr'))
  .filter((f) => /^\d{4}-.+\.md$/.test(f))
  .sort()
  .map((file) => {
    const text = readFileSync(join(ROOT, 'docs/adr', file), 'utf8');
    const heading = text.match(/^#\s+(\d{4})\s+[—-]\s+(.+)$/m);
    const status = text.match(/^\*\*Status:\*\*\s+(\w+)(?:\s+\((\d{4}-\d{2}-\d{2}))?/m);
    return {
      number: heading?.[1] ?? file.slice(0, 4),
      slug: file.replace(/\.md$/, ''),
      // Sin las comillas invertidas del Markdown: el título se pinta como texto.
      title: (heading?.[2] ?? file).replaceAll('`', '').trim(),
      status: status?.[1] ?? 'Accepted',
      date: status?.[2] ?? '',
    };
  });

/** Lo que la harvest dice frente a lo que dice Cargo.toml. Lo comprueba un test. */
export const harvestVersion = meta.version;
