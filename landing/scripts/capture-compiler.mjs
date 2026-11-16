#!/usr/bin/env node
/**
 * Captura el error real que da `cargo check` cuando un caso de uso se salta el outbox, para la
 * sección «Built for agents» de la home. La web enseña ese error tal cual: no se escribe a mano.
 *
 *   pnpm capture:compiler            # tarda lo que tarde compilar `aplicacion_producto` (~1 min con caché)
 *
 * Escribe src/data/harvest/compiler.json. Correr tras cada release, como `pnpm harvest`.
 */
import { execFileSync, spawnSync } from 'node:child_process';
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';

const LANDING = resolve(import.meta.dirname, '..');
const BIN = process.env.PUERTO_BIN ?? resolve(LANDING, '../target/release/puerto');
const puerto = (args, cwd) => execFileSync(BIN, args, { cwd, encoding: 'utf8' });

const dir = mkdtempSync(join(tmpdir(), 'puerto-compiler-'));
puerto(['nuevo', '--nombre', 'tienda', '--db', '--eventos', 'db', '--sin-web', '--sin-demo', '--destino', '.'], dir);
const project = join(dir, 'tienda');
puerto(['generar', 'scaffold', 'Producto', '--', 'nombre:String', 'precio:i64', 'sku:String!'], project);
puerto(['generar', 'caso-de-uso', 'Producto', 'publicar_producto'], project);

// Lo que escribiría alguien que conoce el patrón de repositorio de otros frameworks: leer,
// cambiar y guardar. En un proyecto con eventos el repositorio no tiene `guardar`.
const FILE = 'contextos/producto/aplicacion/src/publicar_producto.rs';
const body = `    async fn ejecutar(&self, params: ParamsPublicarProducto) -> Result<Producto, ErrorProducto> {
        tracing::info!(id = %params.id, "publicar_producto: inicio");
        let mut producto = self
            .repositorio
            .buscar_por_id(params.id)
            .await?
            .ok_or(ErrorProducto::NoEncontrado)?;
        producto.actualizado_en = chrono::Utc::now();
        self.repositorio.guardar(&producto).await?;
        Ok(producto)
    }`;
const path = join(project, FILE);
const generated = readFileSync(path, 'utf8');
const patched = generated
  .replace('use dominio_producto::modelo::Producto;', 'use dominio_producto::modelo::{IdProducto, Producto};')
  .replace(/(pub struct ParamsPublicarProducto \{\n)    pub nombre: String,/, '$1    pub id: IdProducto,')
  .replace(/    async fn ejecutar\(&self, _params[\s\S]*?\n    }\n/, `${body}\n`);
if (!patched.includes('pub id: IdProducto,') || !patched.includes('.guardar(&producto)')) {
  throw new Error('the generated use case changed shape: update this script');
}
writeFileSync(path, patched);

const check = spawnSync('cargo', ['check', '-p', 'aplicacion_producto', '--color', 'never'], {
  cwd: project,
  encoding: 'utf8',
});
const error = check.stderr
  .split('\n')
  .filter((l) => !/^\s*(Compiling|Checking|Blocking|Downloaded|Downloading|Updating|Locking|Adding)\b/.test(l))
  .join('\n')
  .trim();
if (check.status === 0 || !error.includes('E0599')) {
  throw new Error(`expected E0599 from cargo check, got:\n${error}`);
}

const version = puerto(['--version'], project).trim().replace(/^puerto\s+/, '');
writeFileSync(
  join(LANDING, 'src/data/harvest/compiler.json'),
  JSON.stringify(
    {
      version,
      file: FILE,
      // Sin la sangría del `impl`: en la web se lee suelto.
      code: body.replace(/^ {4}/gm, ''),
      error,
    },
    null,
    2,
  ),
);
rmSync(dir, { recursive: true, force: true });
console.log(`capture-compiler: puerto ${version}, E0599 captured.`);
