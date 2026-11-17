import { Link } from "wouter";
import type { EntradaMenu } from "../compartido/ui/marco";
import { SinElementos } from "../compartido/ui/sin-elementos";

/** La API sirve su Swagger UI en su raíz (`make run`). */
const URL_DOCS_API = "http://localhost:8080/";

export function PaginaInicio({ entradas }: { entradas: EntradaMenu[] }) {
  return (
    <section className="space-y-4">
      <h1 className="text-2xl font-bold">Inicio</h1>
      {entradas.length === 0 ? (
        <SinElementos mensaje="Todavía no hay entidades. Añade una con `puerto generar scaffold <Nombre>`." />
      ) : (
        <ul className="menu rounded-box bg-base-100">
          {entradas.map((entrada) => (
            <li key={entrada.href}>
              <Link href={entrada.href}>{entrada.etiqueta}</Link>
            </li>
          ))}
        </ul>
      )}
      <a className="link" href={URL_DOCS_API}>
        Documentación de la API (Swagger UI)
      </a>
    </section>
  );
}
