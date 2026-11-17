import type { ReactNode } from "react";
import { Link } from "wouter";

export interface EntradaMenu {
  etiqueta: string;
  href: string;
}

export function Marco({
  titulo,
  menu,
  children,
}: {
  titulo: string;
  menu: EntradaMenu[];
  children: ReactNode;
}) {
  return (
    <div className="min-h-screen bg-base-200">
      <header className="navbar bg-base-100 shadow-sm">
        <Link href="/" className="btn btn-ghost text-xl">
          {titulo}
        </Link>
        <nav>
          <ul className="menu menu-horizontal">
            {menu.map((entrada) => (
              <li key={entrada.href}>
                <Link href={entrada.href}>{entrada.etiqueta}</Link>
              </li>
            ))}
          </ul>
        </nav>
      </header>
      <main className="container mx-auto max-w-5xl p-4">{children}</main>
    </div>
  );
}
