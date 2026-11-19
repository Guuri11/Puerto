import { describe, expect, test } from "bun:test";
import {
  aFechaHoraLocal,
  aImporte,
  booleanoOpcional,
  deFechaHoraLocal,
  deImporte,
  etiquetasReferencia,
  euros,
  fechaHoraOpcional,
  formatearEntradas,
  formatearFechaHora,
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

  test("debería escribir una fecha en castellano aunque el navegador esté en otro idioma", () => {
    // Las 21:00 del 15 de diciembre en la zona horaria de quien ejecuta el test.
    const instante = new Date(2026, 11, 15, 21, 0).toISOString();
    expect(formatearFechaHora(instante)).toBe("15 dic 2026, 21:00");
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

  test("debería dar la etiqueta de cada objetivo por su id en cuanto se cargan", () => {
    const datos = { elementos: [{ id: "c1", nombre: "Ana" }] };
    expect(etiquetasReferencia({ estado: "listo", datos }, "nombre").get("c1")).toBe("Ana");
    expect(etiquetasReferencia({ estado: "cargando" }, "id").size).toBe(0);
    expect(etiquetasReferencia({ estado: "error" }, "id").size).toBe(0);
  });

  test("debería escribir unos céntimos en euros con coma y leerlos de vuelta", () => {
    expect(aImporte(1250)).toBe("12,50");
    expect(aImporte(5)).toBe("0,05");
    expect(deImporte(aImporte(1250))).toBe(1250);
    expect(deImporte("12,5")).toBe(1250);
    expect(deImporte("12.50")).toBe(1250);
    expect(deImporte(" 12 € ")).toBe(1200);
    expect(deImporte("0,07")).toBe(7);
  });

  test("no debería leer como importe lo que no lo es", () => {
    for (const texto of ["", "doce", "12,505", "1.250,00", "12,"]) {
      expect(deImporte(texto)).toBeNaN();
    }
  });

  test("debería mostrar unos céntimos en euros, en castellano", () => {
    expect(euros(1250).replace(/\s/g, " ")).toBe("12,50 €");
  });
});
