import { Link } from "wouter";
import { useCarga } from "../use-carga";

/**
 * Una referencia en una pantalla de detalle: un enlace a la página del elemento `id` bajo `ruta`,
 * o «—». Con `cargar`, el enlace nombra el elemento con lo que resuelve `cargar` — con su id hasta
 * entonces, o si no se puede cargar. `cargar` se lee una vez por `id`. Una referencia opcional
 * ausente llega de la API como `null`.
 */
export function EnlaceReferencia({
  ruta,
  id,
  cargar,
}: {
  ruta: string;
  id: string | null | undefined;
  cargar?: (id: string) => Promise<string>;
}) {
  const etiqueta = useCarga(
    () => (id == null || cargar === undefined ? Promise.resolve(null) : cargar(id)),
    [id],
  );
  if (id == null) return "—";
  return (
    <Link href={`${ruta}/${id}`} className="link">
      {etiqueta.estado === "listo" && etiqueta.datos !== null ? etiqueta.datos : id}
    </Link>
  );
}
