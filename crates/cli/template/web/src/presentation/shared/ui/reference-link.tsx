import { Link } from "wouter";

/** A reference on a detail screen: a link to the page of the item `id` under `path`, or "—". */
export function ReferenceLink({ path, id }: { path: string; id: string | null }) {
  if (id === null) return "—";
  return (
    <Link href={`${path}/${id}`} className="link">
      {id}
    </Link>
  );
}
