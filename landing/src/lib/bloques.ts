/**
 * El catálogo de bloques (ADR-0028), tal como lo instala la CLI: `src/data/bloques.json` lo
 * escribe `pnpm bloques` con `puerto generar web-bloque`. Aquí solo se agrupa y se cuenta.
 */
import datos from '@/data/bloques.json';

export interface Fichero {
  ruta: string;
  codigo: string;
}

export interface Bloque {
  id: string;
  descripcion: string;
  pide: 'auth' | 'portada' | 'portada/producto' | null;
  hueco: string;
  variante: string;
  porDefecto: boolean;
  /** El componente del hueco: lo que toda variante conserva. `null` en un tema. */
  exporta: string | null;
  ficheros: Fichero[];
}

export const bloques = datos as Bloque[];

export interface Hueco {
  hueco: string;
  /** El fichero del hueco, en `web/`. */
  fichero: string;
  pide: Bloque['pide'];
  variantes: Bloque[];
}

/** Los huecos en el orden del catálogo. */
export const huecos: Hueco[] = [...new Set(bloques.map((b) => b.hueco))].map((hueco) => {
  const variantes = bloques.filter((b) => b.hueco === hueco);
  return { hueco, fichero: variantes[0].ficheros[0].ruta, pide: variantes[0].pide, variantes };
});

export const grupos = [
  {
    id: 'app',
    titulo: 'Las pantallas de la app',
    texto:
      'El acceso, el marco, el panel y el perfil que escribe puerto nuevo, y otra forma de cada uno.',
    huecos: huecos.filter((h) => h.hueco !== 'tema' && h.hueco !== 'portada' && h.pide !== 'portada/producto'),
  },
  {
    id: 'portada',
    titulo: 'La portada',
    texto:
      'Para un SaaS con portada pública (puerto nuevo --portada): la bienvenida, una web de producto y otra variante de cada sección.',
    huecos: huecos.filter((h) => h.hueco === 'portada' || h.pide === 'portada/producto'),
  },
  {
    id: 'temas',
    titulo: 'Los temas',
    texto: 'Los colores y los radios de web/src/marca/tema.css, en claro y en oscuro.',
    huecos: huecos.filter((h) => h.hueco === 'tema'),
  },
];

export const quePide: Record<NonNullable<Bloque['pide']>, string> = {
  auth: 'un proyecto con auth (puerto nuevo --auth)',
  portada: 'un proyecto con portada (puerto nuevo --portada)',
  'portada/producto': 'la portada de producto instalada (portada/producto)',
};

export const comando = (b: Bloque) => `puerto generar web-bloque ${b.id} --forzar`;

/** El bloque en markdown, para pegárselo a un agente de código. */
export function paraTuAgente(b: Bloque): string {
  const requisito = b.pide ? `, en ${quePide[b.pide]}` : '';
  const contrato = b.exporta
    ? `\`${b.exporta}\` conserva su nombre y sus props, así que nada más cambia.`
    : 'Solo cambian variables de CSS: nada más cambia.';
  return [
    `# Bloque \`${b.id}\` de Puerto`,
    '',
    `${b.descripcion[0].toUpperCase()}${b.descripcion.slice(1)}.`,
    '',
    `Instálalo desde la raíz de un proyecto de Puerto con web/${requisito}:`,
    '',
    `    ${comando(b)}`,
    '',
    `Escribe ${b.ficheros.length === 1 ? 'este fichero' : 'estos ficheros'}:`,
    ...b.ficheros.map((f) => `- ${f.ruta}`),
    '',
    `Sustituye a la variante que tenga el hueco \`${b.hueco}\`. ${contrato}`,
    '`--forzar` pisa lo cambiado a mano en esos ficheros; sin él, el comando se niega.',
    '',
    'Después: `make web/lint` y `make web/test`.',
  ].join('\n');
}
