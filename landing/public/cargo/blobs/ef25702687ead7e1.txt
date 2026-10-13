/**
 * The message under an input, when the API's error is about that field (`problem.field`). Placed
 * after the input's `<label>`, not in it: inside, it would become part of the input's name.
 */
export function FieldError({
  problem,
  field,
}: {
  problem: { field: string | null; text: string } | null;
  field: string;
}) {
  if (problem?.field !== field) return null;
  return <span className="text-error text-sm">{problem.text}</span>;
}
