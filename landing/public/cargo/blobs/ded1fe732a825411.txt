import { describe, expect, mock, test } from "bun:test";
import { fireEvent, render, screen } from "@testing-library/react";
import { busquedaPagina, desplazamientoDeUrl, Paginacion } from "./paginacion";

function boton(nombre: string): HTMLButtonElement {
  return screen.getByRole("button", { name: nombre }) as HTMLButtonElement;
}

describe("Paginacion", () => {
  test("debería mostrar el rango y desactivar Anterior en la primera página", () => {
    // Preparar
    const alCambiar = mock((_: number) => {});

    // Actuar
    render(
      <Paginacion total={45} limite={20} desplazamiento={0} alCambiarDesplazamiento={alCambiar} />,
    );

    // Comprobar
    expect(screen.getByText("1–20 de 45")).toBeTruthy();
    expect(boton("Anterior").disabled).toBe(true);
    expect(boton("Siguiente").disabled).toBe(false);
  });

  test("debería desactivar Siguiente en la última página", () => {
    // Actuar
    render(
      <Paginacion total={45} limite={20} desplazamiento={40} alCambiarDesplazamiento={() => {}} />,
    );

    // Comprobar
    expect(screen.getByText("41–45 de 45")).toBeTruthy();
    expect(boton("Siguiente").disabled).toBe(true);
  });

  test("debería pedir el siguiente desplazamiento cuando se pulsa Siguiente", () => {
    // Preparar
    const alCambiar = mock((_: number) => {});
    render(
      <Paginacion total={45} limite={20} desplazamiento={20} alCambiarDesplazamiento={alCambiar} />,
    );

    // Actuar
    fireEvent.click(boton("Siguiente"));

    // Comprobar
    expect(alCambiar).toHaveBeenCalledWith(40);
  });

  test("no debería pintar nada cuando todo cabe en una página", () => {
    // Actuar
    const { container } = render(
      <Paginacion total={3} limite={20} desplazamiento={0} alCambiarDesplazamiento={() => {}} />,
    );

    // Comprobar
    expect(container.textContent).toBe("");
  });

  test("debería leer la página de la URL como un desplazamiento", () => {
    expect(desplazamientoDeUrl("?pagina=3", 20)).toBe(40);
    expect(desplazamientoDeUrl("", 20)).toBe(0);
    expect(desplazamientoDeUrl("?pagina=abc", 20)).toBe(0);
    expect(desplazamientoDeUrl("?pagina=0", 20)).toBe(0);
  });

  test("debería escribir un desplazamiento como la página de la URL, ninguna para la primera", () => {
    expect(busquedaPagina(40, 20)).toBe("?pagina=3");
    expect(busquedaPagina(0, 20)).toBe("");
  });
});
