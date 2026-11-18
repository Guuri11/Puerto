import { Link } from "wouter";
import type { EntradaMenu } from "../compartido/ui/marco";
import { SinElementos } from "../compartido/ui/sin-elementos";
import { useCarga } from "../compartido/use-carga";
import { useDependencias } from "../dependencias";
import { marca } from "../marca/marca";

/** La API sirve su Swagger UI en su raíz (`make run`). */
const URL_DOCS_API = "http://localhost:8080/";

/** La página de inicio: una tarjeta por sección, con cuántos elementos tiene. */
export function PaginaPanel({ entradas }: { entradas: EntradaMenu[] }) {
  return (
    <section className="space-y-6">
      <header>
        <h1 className="text-3xl font-bold">Panel</h1>
        <p className="text-base-content/70">{marca.eslogan}</p>
      </header>
      {entradas.length === 0 ? (
        <SinElementos mensaje="Todavía no hay entidades. Añade una con `puerto generar scaffold <Nombre>`." />
      ) : (
        <ul className="grid gap-4 sm:grid-cols-2 xl:grid-cols-3">
          {entradas.map((entrada) => (
            <li key={entrada.href}>
              <TarjetaTotal entrada={entrada} />
            </li>
          ))}
        </ul>
      )}
      <a className="link link-hover text-sm" href={URL_DOCS_API}>
        Documentación de la API (Swagger UI)
      </a>
    </section>
  );
}

function TarjetaTotal({ entrada }: { entrada: EntradaMenu }) {
  const dependencias = useDependencias();
  const total = useCarga(() => entrada.total(dependencias), [entrada, dependencias]);
  return (
    <Link
      href={entrada.href}
      className="stats w-full bg-base-100 shadow-sm transition hover:shadow-md"
    >
      <div className="stat">
        <div className="stat-title">{entrada.etiqueta}</div>
        <div className="stat-value text-primary">
          {total.estado === "listo" ? (
            total.datos
          ) : total.estado === "cargando" ? (
            <span className="loading loading-dots" />
          ) : (
            "—"
          )}
        </div>
        <div className="stat-desc">Ver la lista</div>
      </div>
    </Link>
  );
}
