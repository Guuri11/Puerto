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

  test("debería pintar sus textos y lo que lleva dentro", () => {
    // Preparar
    const alCancelar = mock(() => {});

    // Actuar
    render(
      <DialogoConfirmar
        abierto
        mensaje="¿Cancelar el pedido?"
        textoConfirmar="Cancelar el pedido"
        textoCancelar="Volver"
        alConfirmar={() => {}}
        alCancelar={alCancelar}
      >
        <p>Ya se ha enviado</p>
      </DialogoConfirmar>,
    );
    fireEvent.click(screen.getByText("Volver"));

    // Comprobar
    expect(screen.getByText("Ya se ha enviado")).toBeTruthy();
    expect(screen.getByText("Cancelar el pedido")).toBeTruthy();
    expect(alCancelar).toHaveBeenCalledTimes(1);
  });
});
