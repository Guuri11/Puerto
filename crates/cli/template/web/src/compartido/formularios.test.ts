import { describe, expect, test } from "bun:test";
import {
  aFechaHoraLocal,
  booleanoOpcional,
  deFechaHoraLocal,
  fechaHoraOpcional,
  formatearEntradas,
  leerEntradas,
  numeroOpcional,
  opcionesReferencia,
  separarLista,
  separarNumeros,
  textoDe,
  textoOpcional,
} from "./formularios";

describe("formularios", () => {
  test("debería dejar vacío un input opcional cuando falta el valor", () => {
    expect(textoDe(null)).toBe("");
    expect(textoDe(undefined)).toBe("");
    expect(textoDe(42)).toBe("42");
    expect(textoDe(false)).toBe("false");
  });

  test("no debería mandar un input opcional vacío", () => {
    expect(textoOpcional("")).toBeUndefined();
    expect(textoOpcional("Ana")).toBe("Ana");
    expect(numeroOpcional("")).toBeUndefined();
    expect(numeroOpcional("1.5")).toBe(1.5);
    expect(booleanoOpcional("")).toBeUndefined();
    expect(booleanoOpcional("true")).toBe(true);
    expect(booleanoOpcional("false")).toBe(false);
  });

  test("debería separar una lista por comas y descartar sus elementos vacíos", () => {
    expect(separarLista(" rojo, verde ,, azul ")).toEqual(["rojo", "verde", "azul"]);
    expect(separarLista("")).toEqual([]);
    expect(separarNumeros("1, 2")).toEqual([1, 2]);
  });

  test("debería escribir un mapa con una entrada por línea y leerlo de vuelta", () => {
    const entradas = { color: "rojo", consulta: "a=b" };

    const texto = formatearEntradas(entradas);

    expect(texto).toBe("color=rojo\nconsulta=a=b");
    expect(leerEntradas(`${texto}\n\n`)).toEqual(entradas);
    expect(formatearEntradas(entradas, ", ")).toBe("color=rojo, consulta=a=b");
  });

  test("debería ir y volver de un input datetime-local sin cambiar el instante", () => {
    const iso = "2026-01-01T10:00:00.000Z";

    const texto = aFechaHoraLocal(iso);

    expect(texto).toMatch(/^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}$/);
    expect(deFechaHoraLocal(texto)).toBe(iso);
    expect(aFechaHoraLocal(null)).toBe("");
    expect(aFechaHoraLocal(undefined)).toBe("");
    expect(fechaHoraOpcional("")).toBeUndefined();
  });

  test("debería ofrecer los objetivos de una referencia en cuanto se cargan", () => {
    const cargados = {
      estado: "listo" as const,
      datos: { elementos: [{ id: "c1", nombre: "Ana" }] },
    };

    expect(opcionesReferencia(cargados, (elemento) => elemento.nombre)).toEqual([
      { id: "c1", etiqueta: "Ana" },
    ]);
    expect(opcionesReferencia({ estado: "cargando" }, (e: { id: string }) => e.id)).toEqual([]);
  });
});
