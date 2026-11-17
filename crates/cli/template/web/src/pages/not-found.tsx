import { Link } from "wouter";

export function NotFoundPage() {
  return (
    <section className="space-y-4">
      <h1 className="text-2xl font-bold">Page not found</h1>
      <Link href="/" className="link">
        Back home
      </Link>
    </section>
  );
}
