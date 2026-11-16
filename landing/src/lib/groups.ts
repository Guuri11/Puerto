/**
 * Cómo agrupa la terminal los ficheros de un proyecto generado: un grupo por directorio real
 * (`contextos/producto/dominio/`, `nucleo/`, `presentacion/api/`…), coloreado por su anillo.
 * Lo usan el render del servidor y el script del navegador, así que no toca `node:fs`.
 *
 * Los tres colores de capa (DESIGN.md) son los tres anillos de la regla de dependencias: el
 * núcleo (dominio, aplicación, `nucleo`), los adaptadores (infraestructura, migraciones) y las
 * entradas (rest, presentación).
 */
export type Ring = 'business' | 'infrastructure' | 'presentation' | 'none';

export interface Group<F extends { path: string }> {
  /** El directorio del grupo, sin barra final; `''` para la raíz del proyecto. */
  dir: string;
  label: string;
  ring: Ring;
  files: F[];
}

const CONTEXT_LAYERS: Record<string, [number, Ring]> = {
  dominio: [0, 'business'],
  aplicacion: [1, 'business'],
  infraestructura: [2, 'infrastructure'],
  rest: [3, 'presentation'],
};
const PRESENTATION = ['cableado', 'api', 'worker'];

function place(path: string): { dir: string; ring: Ring; order: string } {
  const [top, second, third] = path.split('/');
  const depth = path.split('/').length;
  if (top === 'contextos' && depth > 3 && third in CONTEXT_LAYERS) {
    const [rank, ring] = CONTEXT_LAYERS[third];
    return { dir: `contextos/${second}/${third}`, ring, order: `0 ${second} ${rank}` };
  }
  if (top === 'nucleo' && depth > 1) return { dir: 'nucleo', ring: 'business', order: '1' };
  if ((top === 'migraciones' || top === 'migrador') && depth > 1) {
    return { dir: top, ring: 'infrastructure', order: `2 ${top}` };
  }
  if (top === 'presentacion' && depth > 2) {
    return { dir: `presentacion/${second}`, ring: 'presentation', order: `3 ${PRESENTATION.indexOf(second)} ${second}` };
  }
  if (top === 'web' && depth > 1) return { dir: 'web', ring: 'none', order: '4' };
  return { dir: '', ring: 'none', order: '5' };
}

/** Los ficheros, en grupos ordenados de dentro hacia fuera; cada fichero conserva su orden. */
export function groupFiles<F extends { path: string }>(files: F[]): Group<F>[] {
  const groups = new Map<string, Group<F> & { order: string }>();
  for (const f of files) {
    const { dir, ring, order } = place(f.path);
    if (!groups.has(dir)) groups.set(dir, { dir, label: dir ? `${dir}/` : 'project root', ring, order, files: [] });
    groups.get(dir)!.files.push(f);
  }
  return [...groups.values()]
    .sort((a, b) => a.order.localeCompare(b.order))
    .map(({ order: _, ...g }) => g);
}

/** La ruta de un fichero dentro de su grupo. */
export const inGroup = (path: string, dir: string) => (dir ? path.slice(dir.length + 1) : path);
