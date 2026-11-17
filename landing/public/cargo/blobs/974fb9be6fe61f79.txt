import { describe, expect, test } from "bun:test";
import { render, screen } from "@testing-library/react";
import { EnlaceReferencia } from "./enlace-referencia";

describe("EnlaceReferencia", () => {
  test("debería enlazar con la página del elemento referenciado", () => {
    // Actuar
    render(<EnlaceReferencia ruta="/clientes" id="c1" />);

    // Comprobar
    expect(screen.getByRole("link", { name: "c1" }).getAttribute("href")).toBe("/clientes/c1");
  });

  test("debería nombrar el elemento referenciado con su etiqueta en cuanto se carga", async () => {
    // Preparar
    const cargar = async (id: string) => (id === "c1" ? "Ana" : id);

    // Actuar
    render(<EnlaceReferencia ruta="/clientes" id="c1" cargar={cargar} />);

    // Comprobar
    const enlace = await screen.findByRole("link", { name: "Ana" });
    expect(enlace.getAttribute("href")).toBe("/clientes/c1");
  });

  test("debería dejar el id cuando el elemento referenciado no se puede cargar", async () => {
    // Preparar
    const cargar = () => Promise.reject(new Error("no está"));

    // Actuar
    render(<EnlaceReferencia ruta="/clientes" id="c1" cargar={cargar} />);

    // Comprobar
    expect((await screen.findByRole("link")).textContent).toBe("c1");
  });

  test("debería mostrar una raya cuando no hay referencia", () => {
    // Actuar
    const { container } = render(<EnlaceReferencia ruta="/clientes" id={undefined} />);

    // Comprobar
    expect(container.textContent).toBe("—");
  });
});
