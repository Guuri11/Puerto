#!/usr/bin/env node
/**
 * Captura el error real que da `cargo check` cuando un use case se salta el outbox, para la
 * sección «Built for agents» de la home. La web enseña ese error tal cual: no se escribe a mano.
 *
 *   pnpm capture:compiler            # tarda lo que tarde compilar `business` (~1 min con caché)
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
puerto(['new', '--name', 'shop', '--db', '--events', 'db', '--no-web', '--no-demo', '--destination', '.'], dir);
const project = join(dir, 'shop');
puerto(['generate', 'scaffold', 'Product', '--', 'name:String', 'price:i64', 'sku:String!'], project);
puerto(['generate', 'use-case', 'Product', 'publish_product'], project);

// Lo que escribiría alguien que conoce el patrón de repositorio de otros frameworks: leer,
// cambiar y guardar. En un proyecto con eventos el repositorio no tiene `save`.
const paramsPath = join(project, 'business/src/domain/product/use_cases/publish_product.rs');
writeFileSync(
  paramsPath,
  readFileSync(paramsPath, 'utf8')
    .replace(
      'use crate::domain::product::{errors::ProductError, model::Product};',
      'use crate::domain::product::{\n    errors::ProductError,\n    model::{Product, ProductId},\n};',
    )
    .replace('    pub name: String,', '    pub id: ProductId,'),
);

const body = `    async fn execute(&self, params: PublishProductParams) -> Result<Product, ProductError> {
        tracing::info!(id = %params.id, "publish_product: start");
        let mut product = self
            .repository
            .find_by_id(params.id)
            .await?
            .ok_or(ProductError::NotFound)?;
        product.updated_at = chrono::Utc::now();
        self.repository.save(&product).await?;
        Ok(product)
    }`;
const implPath = join(project, 'business/src/application/product/publish_product.rs');
const impl = readFileSync(implPath, 'utf8');
const patched = impl.replace(/    async fn execute\(&self, _params[\s\S]*?\n    }\n/, `${body}\n`);
if (patched === impl) throw new Error('the generated use case changed shape: update this script');
writeFileSync(implPath, patched);

const check = spawnSync('cargo', ['check', '-p', 'business', '--color', 'never'], {
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
      file: 'business/src/application/product/publish_product.rs',
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
