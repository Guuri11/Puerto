/** «1–20 de 45» con Anterior / Siguiente. No pinta nada cuando la lista cabe en una página. */
export function Paginacion({
  total,
  limite,
  desplazamiento,
  alCambiarDesplazamiento,
}: {
  total: number;
  limite: number;
  desplazamiento: number;
  alCambiarDesplazamiento: (desplazamiento: number) => void;
}) {
  if (desplazamiento === 0 && total <= limite) return null;
  const ultimo = Math.min(desplazamiento + limite, total);

  return (
    <nav className="flex items-center justify-between" aria-label="Paginación">
      <span className="text-sm">
        {desplazamiento + 1}–{ultimo} de {total}
      </span>
      <div className="join">
        <button
          type="button"
          className="join-item btn btn-sm"
          disabled={desplazamiento === 0}
          onClick={() => alCambiarDesplazamiento(Math.max(desplazamiento - limite, 0))}
        >
          Anterior
        </button>
        <button
          type="button"
          className="join-item btn btn-sm"
          disabled={ultimo >= total}
          onClick={() => alCambiarDesplazamiento(desplazamiento + limite)}
        >
          Siguiente
        </button>
      </div>
    </nav>
  );
}

/** El desplazamiento que nombra el `?pagina=` de la URL (desde 1); la primera página si no hay. */
export function desplazamientoDeUrl(busqueda: string, limite: number): number {
  const pagina = Number(new URLSearchParams(busqueda).get("pagina"));
  return Number.isInteger(pagina) && pagina > 1 ? (pagina - 1) * limite : 0;
}

/** La búsqueda de la URL para la página en `desplazamiento`: ninguna para la primera. */
export function busquedaPagina(desplazamiento: number, limite: number): string {
  const pagina = Math.floor(desplazamiento / limite) + 1;
  return pagina > 1 ? `?pagina=${pagina}` : "";
}
