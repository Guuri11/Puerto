import { describe, expect, test } from "bun:test";
import { render, screen } from "@testing-library/react";
import { ErrorCampo } from "./error-campo";

describe("ErrorCampo", () => {
  test("debería mostrar el mensaje cuando el problema es de su campo", () => {
    // Actuar
    render(
      <ErrorCampo problema={{ campo: "nombre", texto: "Nombre: obligatorio" }} campo="nombre" />,
    );

    // Comprobar
    expect(screen.getByText("Nombre: obligatorio")).toBeTruthy();
  });

  test("no debería mostrar nada cuando el problema es de otro campo", () => {
    // Actuar
    const { container } = render(
      <ErrorCampo problema={{ campo: "sku", texto: "Sku: no válido" }} campo="nombre" />,
    );

    // Comprobar
    expect(container.textContent).toBe("");
  });
});
