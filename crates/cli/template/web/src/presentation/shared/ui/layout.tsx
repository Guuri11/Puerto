import type { ReactNode } from "react";
import { Link } from "wouter";

export interface NavEntry {
  label: string;
  href: string;
}

export function Layout({
  title,
  nav,
  children,
}: {
  title: string;
  nav: NavEntry[];
  children: ReactNode;
}) {
  return (
    <div className="min-h-screen bg-base-200">
      <header className="navbar bg-base-100 shadow-sm">
        <Link href="/" className="btn btn-ghost text-xl">
          {title}
        </Link>
        <nav>
          <ul className="menu menu-horizontal">
            {nav.map((entry) => (
              <li key={entry.href}>
                <Link href={entry.href}>{entry.label}</Link>
              </li>
            ))}
          </ul>
        </nav>
      </header>
      <main className="container mx-auto max-w-5xl p-4">{children}</main>
    </div>
  );
}
