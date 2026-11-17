import { describe, expect, test } from "bun:test";
import { act, renderHook, waitFor } from "@testing-library/react";
import { useCarga } from "./use-carga";

describe("useCarga", () => {
  test("debería dar los datos cuando la llamada se resuelve", async () => {
    // Preparar
    const cargar = () => Promise.resolve(["Ana", "Luis"]);

    // Actuar
    const { result } = renderHook(() => useCarga(cargar, []));

    // Comprobar
    expect(result.current.estado).toBe("cargando");
    await waitFor(() => expect(result.current.estado).toBe("listo"));
    expect(result.current).toMatchObject({ datos: ["Ana", "Luis"] });
  });

  test("debería dar el error cuando la llamada se rechaza", async () => {
    // Preparar
    const fallo = { code: "producto.repositorio" };
    const cargar = () => Promise.reject(fallo);

    // Actuar
    const { result } = renderHook(() => useCarga(cargar, []));

    // Comprobar
    await waitFor(() => expect(result.current.estado).toBe("error"));
    expect(result.current).toMatchObject({ error: fallo });
  });

  test("debería volver a llamar cuando se recarga", async () => {
    // Preparar
    let llamadas = 0;
    const cargar = () => Promise.resolve(++llamadas);
    const { result } = renderHook(() => useCarga(cargar, []));
    await waitFor(() => expect(result.current).toMatchObject({ datos: 1 }));

    // Actuar
    act(() => result.current.recargar());

    // Comprobar
    await waitFor(() => expect(result.current).toMatchObject({ datos: 2 }));
  });
});
