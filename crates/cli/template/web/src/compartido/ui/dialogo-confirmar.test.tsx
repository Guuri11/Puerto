import { describe, expect, mock, test } from "bun:test";
import { fireEvent, render, screen } from "@testing-library/react";
import { DialogoConfirmar } from "./dialogo-confirmar";

describe("DialogoConfirmar", () => {
  test("debería confirmar cuando se pulsa el botón de confirmar", () => {
    // Preparar
    const alConfirmar = mock(() => {});
    const alCancelar = mock(() => {});
    render(
      <DialogoConfirmar
        abierto
        mensaje="¿Borrar a Ana?"
        alConfirmar={alConfirmar}
        alCancelar={alCancelar}
      />,
    );

    // Actuar
    fireEvent.click(screen.getByText("Borrar"));

    // Comprobar
    expect(alConfirmar).toHaveBeenCalledTimes(1);
    expect(alCancelar).not.toHaveBeenCalled();
  });

  test("debería cancelar cuando se pulsa el botón de cancelar", () => {
    // Preparar
    const alConfirmar = mock(() => {});
    const alCancelar = mock(() => {});
    render(
      <DialogoConfirmar
        abierto
        mensaje="¿Borrar a Ana?"
        alConfirmar={alConfirmar}
        alCancelar={alCancelar}
      />,
    );

    // Actuar
    fireEvent.click(screen.getByText("Cancelar"));

    // Comprobar
    expect(alCancelar).toHaveBeenCalledTimes(1);
    expect(alConfirmar).not.toHaveBeenCalled();
  });
});
