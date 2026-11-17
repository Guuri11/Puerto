import { Link } from "wouter";
import { useAsync } from "../../core/hooks/use-async";

/**
 * A reference on a detail screen: a link to the page of the item `id` under `path`, or "—". With
 * `load`, the link names the item by what `load` resolves to — by its id until then, or if it
 * cannot be loaded. `load` is read once per `id`.
 */
export function ReferenceLink({
  path,
  id,
  load,
}: {
  path: string;
  id: string | null;
  load?: (id: string) => Promise<string>;
}) {
  const label = useAsync(
    () => (id === null || load === undefined ? Promise.resolve(null) : load(id)),
    [id],
  );
  if (id === null) return "—";
  return (
    <Link href={`${path}/${id}`} className="link">
      {label.status === "ok" && label.data !== null ? label.data : id}
    </Link>
  );
}

/** A `ReferenceLink`'s `load`: the item's label, read through its repository. */
export function labelOf<T>(
  repository: { get(id: string): Promise<T> },
  label: (item: T) => string,
): (id: string) => Promise<string> {
  return (id) => repository.get(id).then(label);
}
