#!/usr/bin/env node
/**
 * Builds the block showcase (`/bloques`, ADR-0028) from the real `puerto` CLI: a project with
 * local accounts and a portada, every block installed with `puerto generar web-bloque --forzar`,
 * and a preview app built from those very files. Nothing is drawn by hand: what the page shows is
 * what the command installs.
 *
 *   pnpm bloques                        # uses ../target/release/puerto; needs bun on PATH
 *   PUERTO_BIN=$(which puerto) pnpm bloques
 *
 * Writes (committed — Vercel has neither `puerto` nor `bun`):
 *   src/data/bloques.json        the catalog: each block, what it needs and the code of its files
 *   public/bloques/vista/        the preview app: vista.html?bloque=<hueco>/<variante>&tema=light|dark
 */
import { execFileSync } from 'node:child_process';
import { cpSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { basename, dirname, extname, join, resolve } from 'node:path';

const LANDING = resolve(import.meta.dirname, '..');
const REPO = resolve(LANDING, '..');
const BIN = resolve(process.env.PUERTO_BIN ?? join(REPO, 'target/release/puerto'));
const OUT_DATA = join(LANDING, 'src/data/bloques.json');
const OUT_VISTA = join(LANDING, 'public/bloques/vista');

const puerto = (args, cwd) =>
  execFileSync(BIN, args, { cwd, encoding: 'utf8', stdio: ['ignore', 'pipe', 'pipe'] });
const bun = (args, cwd) => execFileSync('bun', args, { cwd, stdio: ['ignore', 'ignore', 'inherit'] });

// A block of the escaparate shows an entity (27.20 de Puerto): the project's public Producto.
const entidadDe = (hueco) => (hueco === 'escaparate' ? ['Producto'] : []);

// ── The catalog, as `puerto generar web-bloque` prints it ─────────────────────────────────────
const LINEA = /^ {2}(\S+\/\S+)\s{2,}(.+?)(?: \((auth|portada|portada\/producto)\))?$/;
const catalogo = puerto(['generar', 'web-bloque'])
  .split('\n')
  .map((linea) => linea.match(LINEA))
  .filter(Boolean)
  .map(([, id, texto, pide]) => {
    // A block that takes arguments ends with its usage: "…, en tarjetas: `escaparate/tarjetas <Entidad>`".
    const uso = texto.match(/^(.*): `\S+ (.+)`$/);
    return {
      id,
      descripcion: uso ? uso[1] : texto,
      argumentos: uso ? uso[2] : null,
      pide: pide ?? null,
    };
  });
if (catalogo.length === 0) {
  console.error(`${BIN} no lista ningún bloque: ¿es anterior a la 0.27?`);
  process.exit(1);
}

// ── A project, and every block installed in it ────────────────────────────────────────────────
function construir() {
  const trabajo = mkdtempSync(join(tmpdir(), 'puerto-bloques-'));
  try {
    puerto(
      ['nuevo', '--nombre', 'acme', '--db', '--auth', 'local', '--portada', '--sin-demo', '--destino', trabajo],
      trabajo,
    );
    const proyecto = join(trabajo, 'acme');
    // Its list is public: the escaparate (`escaparate/<variante> Producto`) shows it on the portada.
    puerto(
      ['generar', 'scaffold', 'Producto', '--publico', 'listar_productos,obtener_producto', '--', 'nombre:String', 'precio:i64'],
      proyecto,
    );
    puerto(['generar', 'scaffold', 'Cliente', '--', 'nombre:String', 'email:String'], proyecto);
    // The sections of the portada are installed over `portada/producto`.
    puerto(['generar', 'web-bloque', 'portada/producto', '--forzar'], proyecto);

    const FICHERO = /^[✓=] (\S+)/;
    const vistos = new Set();
    const bloques = catalogo.map((bloque) => {
      const [hueco, variante] = bloque.id.split('/');
      const rutas = puerto(['generar', 'web-bloque', bloque.id, ...entidadDe(hueco), '--forzar'], proyecto)
        .split('\n')
        .map((linea) => linea.match(FICHERO)?.[1])
        .filter(Boolean);
      const ficheros = rutas.map((ruta) => ({ ruta, codigo: readFileSync(join(proyecto, ruta), 'utf8') }));
      // The slot's file next to itself, the variant as a suffix: same directory, same imports.
      const propio = join(proyecto, rutas[0]);
      const ext = extname(propio);
      cpSync(propio, join(dirname(propio), `${basename(propio, ext)}--${variante}${ext}`));
      const porDefecto = !vistos.has(hueco);
      vistos.add(hueco);
      return {
        ...bloque,
        hueco,
        variante,
        porDefecto,
        exporta: ficheros[0].codigo.match(/export function (\w+)/)?.[1] ?? null,
        ficheros,
      };
    });

    // Back to the default of every slot: the previews wrap a variant in the others' defaults.
    for (const bloque of bloques.filter((b) => b.porDefecto && b.hueco !== 'portada')) {
      puerto(['generar', 'web-bloque', bloque.id, ...entidadDe(bloque.hueco), '--forzar'], proyecto);
    }

    const web = join(proyecto, 'web');
    writeFileSync(join(web, 'vista.html'), VISTA_HTML);
    writeFileSync(join(web, 'src/vista.css'), vistaCss(bloques));
    writeFileSync(join(web, 'src/vista.tsx'), vistaTsx(bloques));
    writeFileSync(join(web, 'vite.vista.config.ts'), VITE_CONFIG);
    bun(['install'], web);
    bun(['--bun', 'vite', 'build', '--config', 'vite.vista.config.ts'], web);

    rmSync(OUT_VISTA, { recursive: true, force: true });
    cpSync(join(web, 'dist-vista'), OUT_VISTA, { recursive: true });
    writeFileSync(OUT_DATA, `${JSON.stringify(bloques, null, 2)}\n`);
    console.log(`✓ ${bloques.length} bloques → src/data/bloques.json, public/bloques/vista/`);
  } finally {
    rmSync(trabajo, { recursive: true, force: true });
  }
}

// ── The preview app ───────────────────────────────────────────────────────────────────────────

function vistaCss(bloques) {
  const temas = bloques
    .filter((b) => b.hueco === 'tema')
    .flatMap((b) =>
      [...b.ficheros[0].codigo.matchAll(/name: "(light|dark)";\n {2}(?:default|prefersdark): true;\n([^}]*)\}/g)].map(
        ([, modo, cuerpo]) => `\n:root[data-tema="${b.variante}"][data-theme="${modo}"] {\n${cuerpo}}\n`,
      ),
    );
  return `@import "tailwindcss";\n@import "./marca/tema.css";\n${temas.join('')}`;
}

function vistaTsx(bloques) {
  const pascal = (s) => s[0].toUpperCase() + s.slice(1);
  const modulo = (b) => b.ficheros[0].ruta.replace(/^web\/src\//, './').replace(/\.tsx$/, `--${b.variante}`);
  const pintar = (b, m) => {
    switch (b.hueco) {
      case 'acceso':
        return `<${m}.MarcoAcceso><PaginaLogin /></${m}.MarcoAcceso>`;
      case 'marco':
        return `<${m}.Marco menu={entradasMenu}><PaginaPanel entradas={entradasMenu} /></${m}.Marco>`;
      case 'panel':
        return `<Marco menu={entradasMenu}><${m}.PaginaPanel entradas={entradasMenu} /></Marco>`;
      case 'perfil':
        return `<Marco menu={entradasMenu}><${m}.PaginaPerfil /></Marco>`;
      case 'portada':
        return `<${m}.PaginaPortada />`;
      case 'tema':
        return '<Muestra />';
      default:
        return `<div className="min-h-screen bg-base-100"><${m}.Seccion${pascal(b.hueco)} /></div>`;
    }
  };
  const conModulo = bloques.map((b, i) => ({ ...b, m: b.hueco === 'tema' ? null : `b${i}` }));
  const importaciones = conModulo
    .filter((b) => b.m)
    .map((b) => `import * as ${b.m} from "${modulo(b)}";`)
    .join('\n');
  const entradas = conModulo.map((b) => `  "${b.id}": () => ${pintar(b, b.m)},`).join('\n');
  const temaPorDefecto = bloques.find((b) => b.hueco === 'tema' && b.porDefecto).variante;
  return `${VISTA_TSX_CABECERA}${importaciones}

const bloques: Record<string, () => ReactNode> = {
${entradas}
};

/** El tema del proyecto: daisyUI pinta su \`light\` de serie sobre el de la marca con \`data-theme\`. */
const TEMA_POR_DEFECTO = "${temaPorDefecto}";
${VISTA_TSX_CUERPO}`;
}

const VISTA_HTML = `<!doctype html>
<html lang="es">
  <head>
    <meta charset="UTF-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <meta name="robots" content="noindex" />
    <title>Vista previa de un bloque de Puerto</title>
  </head>
  <body>
    <div id="root"></div>
    <script type="module" src="/src/vista.tsx"></script>
  </body>
</html>
`;

// The project's config is a function of the mode (it reads SERVICE_PORT from .env, 27.1 de Puerto).
const VITE_CONFIG = `import { defineConfig, mergeConfig } from "vite";
import base from "./vite.config.ts";

export default defineConfig((entorno) => mergeConfig(base(entorno), {
  base: "/bloques/vista/",
  build: {
    outDir: "dist-vista",
    emptyOutDir: true,
    rollupOptions: {
      input: "vista.html",
      output: {
        entryFileNames: "vista.js",
        chunkFileNames: "[name].js",
        assetFileNames: "[name][extname]",
      },
    },
  },
}));
`;

const VISTA_TSX_CABECERA = `import "./vista.css";

import { type ReactNode, StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { Router } from "wouter";
import { memoryLocation } from "wouter/memory-location";
import { ProveedorAuth } from "./auth/auth";
import { Marco } from "./compartido/ui/marco";
import { ProveedorDependencias } from "./dependencias";
import type { Dependencias } from "./generado/dependencias";
import { entradasMenu } from "./generado/rutas";
import { marca } from "./marca/marca";
import { PaginaLogin } from "./paginas/login";
import { PaginaPanel } from "./paginas/panel";
import { apiFalsa } from "./pruebas/api-falsa";
import { cuentasFalsas, MadreUsuarioActual, pasarelaAuthFalsa } from "./pruebas/auth";
import { coleccionCliente, MadreCliente } from "./pruebas/cliente";
import { coleccionProducto, MadreProducto } from "./pruebas/producto";

/** Un tema, sobre el panel y unos cuantos componentes. */
function Muestra() {
  return (
    <Marco menu={entradasMenu}>
      <div className="space-y-8">
        <PaginaPanel entradas={entradasMenu} />
        <div className="card bg-base-100 shadow-sm">
          <div className="card-body gap-4">
            <h2 className="card-title">Producto nuevo</h2>
            <label className="fieldset">
              <span className="fieldset-legend">Nombre</span>
              <input className="input w-full" defaultValue="Cafetera de filtro" />
            </label>
            <div className="flex flex-wrap items-center gap-2">
              <span className="badge badge-primary">Nuevo</span>
              <span className="badge badge-accent">Oferta</span>
              <span className="badge badge-outline">Borrador</span>
              <input type="checkbox" className="toggle toggle-primary" defaultChecked />
            </div>
            <progress className="progress progress-primary" value={64} max={100} />
            <div className="card-actions justify-end">
              <button type="button" className="btn">Cancelar</button>
              <button type="button" className="btn btn-accent">Guardar borrador</button>
              <button type="button" className="btn btn-primary">Crear producto</button>
            </div>
          </div>
        </div>
      </div>
    </Marco>
  );
}

`;

const VISTA_TSX_CUERPO = `
const parametros = new URLSearchParams(window.location.search);
const bloque = parametros.get("bloque") ?? "";
document.documentElement.dataset.theme = parametros.get("tema") === "dark" ? "dark" : "light";
document.documentElement.dataset.tema = bloque.startsWith("tema/") ? bloque.slice(5) : TEMA_POR_DEFECTO;
// El logo del proyecto se sirve aquí, no en la raíz de la web de Puerto.
marca.logo = \`\${import.meta.env.BASE_URL}logo.svg\`;

const dependencias: Dependencias = {
  api: apiFalsa([
    coleccionProducto(Array.from({ length: 128 }, () => MadreProducto.aleatorio())),
    coleccionCliente(Array.from({ length: 37 }, () => MadreCliente.aleatorio())),
  ]),
  auth: pasarelaAuthFalsa({
    usuario: MadreUsuarioActual.aleatorio({
      nombre: "Ana Lozano",
      email: "ana@example.com",
      roles: ["admin", "ventas"],
    }),
  }),
  cuentas: cuentasFalsas(),
};

const raiz = document.getElementById("root");
if (raiz === null) throw new Error("vista.html no tiene el elemento #root");
const pintar = bloques[bloque] ?? (() => <p className="p-8">No hay ningún bloque «{bloque}».</p>);

createRoot(raiz).render(
  <StrictMode>
    <ProveedorDependencias valor={dependencias}>
      <Router hook={memoryLocation({ path: bloque.startsWith("perfil/") ? "/perfil" : "/" }).hook}>
        <ProveedorAuth>{pintar()}</ProveedorAuth>
      </Router>
    </ProveedorDependencias>
  </StrictMode>,
);
`;

construir();
