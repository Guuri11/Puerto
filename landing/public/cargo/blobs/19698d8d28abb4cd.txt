import { describe, expect, test } from "bun:test";
import { render, screen } from "@testing-library/react";
import { PaginaInicio } from "./inicio";

describe("PaginaInicio", () => {
  test("debería enlazar cada entidad cuando hay alguna", () => {
    // Preparar
    const entradas = [{ etiqueta: "Productos", href: "/productos" }];

    // Actuar
    render(<PaginaInicio entradas={entradas} />);

    // Comprobar
    expect(screen.getByText("Productos").getAttribute("href")).toBe("/productos");
  });

  test("debería explicar cómo añadir una entidad cuando no hay ninguna", () => {
    // Actuar
    render(<PaginaInicio entradas={[]} />);

    // Comprobar
    expect(screen.getByText(/puerto generar scaffold/)).toBeTruthy();
  });
});
