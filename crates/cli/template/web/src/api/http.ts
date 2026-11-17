/** A call the API refused. `code` is its `RespuestaError.codigo`: `producto.no_encontrado`. */
export class ApiError extends Error {
  readonly code: string;

  constructor(code: string) {
    super(code);
    this.name = "ApiError";
    this.code = code;
  }
}

/**
 * The machine-readable code of a failed call: the `RespuestaError.codigo` the API answered, or
 * `http.<status>` for a body that is not one (a proxy error, poem's own `400`).
 */
export function apiErrorCode(error: unknown, response: Response): string {
  if (typeof error === "object" && error !== null && "codigo" in error) {
    const { codigo } = error;
    if (typeof codigo === "string") return codigo;
  }
  return `http.${response.status}`;
}

/** What an openapi-fetch call resolves to. */
interface Answer<T> {
  data?: T;
  error?: unknown;
  response: Response;
}

/** The data of a call that succeeded; else rejects with its `ApiError`. */
export async function unwrap<T>(call: Promise<Answer<T>>): Promise<T> {
  const { data, error, response } = await call;
  if (!response.ok) throw new ApiError(apiErrorCode(error, response));
  // A `204` has no body: `data` is only absent where the call's type says there is none.
  return data as T;
}

/** The page size the list screens ask for. */
export const PAGE_SIZE = 20;

/** The largest page the API answers (the domain caps `limite`): what a reference's select offers. */
export const MAX_PAGE_LIMIT = 100;
