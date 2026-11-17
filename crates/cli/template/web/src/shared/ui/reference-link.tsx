import { Link } from "wouter";
import { useAsync } from "../use-async";

/**
 * A reference on a detail screen: a link to the page of the item `id` under `path`, or "—". With
 * `load`, the link names the item by what `load` resolves to — by its id until then, or if it
 * cannot be loaded. `load` is read once per `id`. An absent optional reference comes from the API
 * as `null`.
 */
export function ReferenceLink({
  path,
  id,
  load,
}: {
  path: string;
  id: string | null | undefined;
  load?: (id: string) => Promise<string>;
}) {
  const label = useAsync(
    () => (id == null || load === undefined ? Promise.resolve(null) : load(id)),
    [id],
  );
  if (id == null) return "—";
  return (
    <Link href={`${path}/${id}`} className="link">
      {label.status === "ok" && label.data !== null ? label.data : id}
    </Link>
  );
}
