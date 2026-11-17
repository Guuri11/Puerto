import { describe, expect, test } from "bun:test";
import { render, screen } from "@testing-library/react";
import { AlertaError } from "./alerta-error";

describe("AlertaError", () => {
  test("debería mostrar el código del error cuando el error lo lleva", () => {
    // Preparar
    const error = { codigo: "producto.validacion.nombre_vacio" };

    // Actuar
    render(<AlertaError error={error} />);

    // Comprobar
    expect(screen.getByRole("alert").textContent).toBe("producto.validacion.nombre_vacio");
  });

  test("debería mostrar el mensaje dado en vez del código", () => {
    // Preparar
    const error = { codigo: "producto.duplicado" };

    // Actuar
    render(<AlertaError error={error} mensaje="Estos valores ya están en uso" />);

    // Comprobar
    expect(screen.getByRole("alert").textContent).toBe("Estos valores ya están en uso");
  });

  test("debería mostrar el mensaje cuando falló la propia petición", () => {
    // Preparar
    const error = new TypeError("Failed to fetch");

    // Actuar
    render(<AlertaError error={error} />);

    // Comprobar
    expect(screen.getByRole("alert").textContent).toBe("Failed to fetch");
  });
});
