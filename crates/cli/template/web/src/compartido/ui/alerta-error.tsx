/**
 * El código legible por máquina de un error: el `RespuestaError.codigo` de la API
 * (`producto.validacion.nombre_vacio`), que llevan `ErrorApi` y `ErrorAuth` en `codigo`.
 */
export function codigoError(error: unknown): string {
  if (typeof error === "object" && error !== null && "codigo" in error) {
    const { codigo } = error;
    if (typeof codigo === "string") return codigo;
  }
  return error instanceof Error ? error.message : "desconocido";
}

/** La alerta de una llamada que falló: `mensaje` si lo hay; si no, el código del error. */
export function AlertaError({ error, mensaje }: { error: unknown; mensaje?: string }) {
  return (
    <div role="alert" className="alert alert-error">
      <span>{mensaje ?? codigoError(error)}</span>
    </div>
  );
}
