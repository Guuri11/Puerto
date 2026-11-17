/** Una llamada que la API rechazó. `codigo` es su `RespuestaError.codigo`: `producto.no_encontrado`. */
export class ErrorApi extends Error {
  readonly codigo: string;

  constructor(codigo: string) {
    super(codigo);
    this.name = "ErrorApi";
    this.codigo = codigo;
  }
}

/**
 * El código legible por máquina de una llamada que falló: el `RespuestaError.codigo` que respondió
 * la API, o `http.<estado>` si la respuesta no es uno (un error del proxy, el `400` del propio poem).
 */
export function codigoErrorApi(error: unknown, respuesta: Response): string {
  if (typeof error === "object" && error !== null && "codigo" in error) {
    const { codigo } = error;
    if (typeof codigo === "string") return codigo;
  }
  return `http.${respuesta.status}`;
}

/** Lo que resuelve una llamada de openapi-fetch. */
interface Respuesta<T> {
  data?: T;
  error?: unknown;
  response: Response;
}

/** Los datos de una llamada que salió bien; si no, rechaza con su `ErrorApi`. */
export async function desenvolver<T>(llamada: Promise<Respuesta<T>>): Promise<T> {
  const { data, error, response } = await llamada;
  if (!response.ok) throw new ErrorApi(codigoErrorApi(error, response));
  // Un `204` no tiene cuerpo: `data` solo falta donde el tipo de la llamada dice que no lo hay.
  return data as T;
}

/** El tamaño de página que piden las pantallas de lista. */
export const TAMANO_PAGINA = 20;

/**
 * La página más grande que responde la API (el dominio acota `limite`): lo que ofrece el select de
 * una referencia.
 */
export const LIMITE_PAGINA_MAXIMO = 100;
