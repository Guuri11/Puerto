import { Link } from "wouter";
import { useCarga } from "../use-carga";

/**
 * Una referencia en una pantalla: un enlace a la página del elemento `id` bajo `ruta`, o «—». El
 * enlace nombra el elemento con su etiqueta: la de `etiquetas` (una lista, que carga los objetivos
 * de una vez) o lo que resuelve `cargar` (un detalle; se lee una vez por `id`); con su id hasta
 * entonces, o si no se puede cargar. Una referencia opcional ausente llega de la API como `null`.
 */
export function EnlaceReferencia({
  ruta,
  id,
  cargar,
  etiquetas,
}: {
  ruta: string;
  id: string | null | undefined;
  cargar?: (id: string) => Promise<string>;
  etiquetas?: Map<string, string>;
}) {
  const etiqueta = useCarga(
    () => (id == null || cargar === undefined ? Promise.resolve(null) : cargar(id)),
    [id],
  );
  if (id == null) return "—";
  const cargada = etiqueta.estado === "listo" ? etiqueta.datos : null;
  return (
    <Link href={`${ruta}/${id}`} className="link">
      {etiquetas?.get(id) ?? cargada ?? id}
    </Link>
  );
}
