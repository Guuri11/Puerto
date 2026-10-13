import { Link } from "wouter";
import { EmptyState } from "../shared/ui/empty-state";
import type { NavEntry } from "../shared/ui/layout";

/** The API serves its Swagger UI at its root (`make run`). */
const API_DOCS_URL = "http://localhost:8080/";

export function HomePage({ entries }: { entries: NavEntry[] }) {
  return (
    <section className="space-y-4">
      <h1 className="text-2xl font-bold">Home</h1>
      {entries.length === 0 ? (
        <EmptyState message="No entities yet. Add one with `puerto generate scaffold <Name>`." />
      ) : (
        <ul className="menu rounded-box bg-base-100">
          {entries.map((entry) => (
            <li key={entry.href}>
              <Link href={entry.href}>{entry.label}</Link>
            </li>
          ))}
        </ul>
      )}
      <a className="link" href={API_DOCS_URL}>
        API docs (Swagger UI)
      </a>
    </section>
  );
}
