/**
 * The machine-readable code of an error: `ErrorResponse.name` from the API
 * (`product.validation_error.name_empty`), carried by the domain errors as `code`.
 */
export function errorCode(error: unknown): string {
  if (typeof error === "object" && error !== null && "code" in error) {
    const { code } = error;
    if (typeof code === "string") return code;
  }
  return error instanceof Error ? error.message : "unknown";
}

export function ErrorAlert({ error }: { error: unknown }) {
  return (
    <div role="alert" className="alert alert-error">
      <span>{errorCode(error)}</span>
    </div>
  );
}
