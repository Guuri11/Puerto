import { Link } from "wouter";

export function PaginaNoEncontrada() {
  return (
    <section className="space-y-4">
      <h1 className="text-2xl font-bold">Página no encontrada</h1>
      <Link href="/" className="link">
        Volver al inicio
      </Link>
    </section>
  );
}
