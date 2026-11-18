import { describe, expect, test } from "bun:test";
import { screen } from "@testing-library/react";
import type { EntradaMenu } from "../compartido/ui/marco";
import { renderizarPantalla } from "../pruebas/renderizar";
import { PaginaPanel } from "./panel";

function entrada(total: EntradaMenu["total"]): EntradaMenu {
  return { etiqueta: "Productos", href: "/productos", total };
}

describe("PaginaPanel", () => {
  test("debería mostrar cuántos elementos tiene cada sección y enlazar su lista", async () => {
    // Preparar
    const entradas = [entrada(async () => 12)];

    // Actuar
    renderizarPantalla(<PaginaPanel entradas={entradas} />, {});

    // Comprobar
    expect(await screen.findByText("12")).toBeTruthy();
    expect(screen.getByText("Productos").closest("a")?.getAttribute("href")).toBe("/productos");
  });

  test("debería mostrar una raya cuando no se puede contar una sección", async () => {
    // Preparar
    const entradas = [entrada(() => Promise.reject(new Error("http.500")))];

    // Actuar
    renderizarPantalla(<PaginaPanel entradas={entradas} />, {});

    // Comprobar
    expect(await screen.findByText("—")).toBeTruthy();
  });

  test("debería explicar cómo añadir una entidad cuando no hay ninguna", async () => {
    // Actuar
    renderizarPantalla(<PaginaPanel entradas={[]} />, {});

    // Comprobar
    expect(await screen.findByText(/puerto generar scaffold/)).toBeTruthy();
  });
});
