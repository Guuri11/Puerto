// Conversiones entre lo que guarda un input de formulario (texto) y lo que manda y recibe la API.
// Las llaman las vistas de formulario generadas; un input de texto guarda siempre un `string`.
//
// Un campo opcional es una propiedad opcional en la API, que responde `null` cuando falta: al
// leer se aceptan las dos cosas, y al escribir un input vacío no se manda (`undefined`).

/** El texto de un valor; `""` si no lo hay. */
export function textoDe(valor: string | number | boolean | null | undefined): string {
  return valor == null ? "" : String(valor);
}

/** No se manda si el input está vacío. */
export function textoOpcional(texto: string): string | undefined {
  return texto === "" ? undefined : texto;
}

export function numeroOpcional(texto: string): number | undefined {
  return texto === "" ? undefined : Number(texto);
}

/** Un `<select>` de `""` / `"true"` / `"false"`. */
export function booleanoOpcional(texto: string): boolean | undefined {
  return texto === "" ? undefined : texto === "true";
}

/** Elementos separados por comas, sin espacios alrededor; los vacíos se descartan. */
export function separarLista(texto: string): string[] {
  return texto
    .split(",")
    .map((elemento) => elemento.trim())
    .filter((elemento) => elemento !== "");
}

export function separarNumeros(texto: string): number[] {
  return separarLista(texto).map(Number);
}

/** Una entrada `clave=valor` por línea (o por `separador`). */
export function formatearEntradas(entradas: Record<string, string>, separador = "\n"): string {
  return Object.entries(entradas)
    .map(([clave, valor]) => `${clave}=${valor}`)
    .join(separador);
}

/** Líneas `clave=valor`; las vacías se descartan, y una sin `=` es una clave con el valor vacío. */
export function leerEntradas(texto: string): Record<string, string> {
  const entradas: Record<string, string> = {};
  for (const linea of texto.split("\n")) {
    if (linea.trim() === "") continue;
    const [clave = "", ...valor] = linea.split("=");
    entradas[clave.trim()] = valor.join("=").trim();
  }
  return entradas;
}

/** Un instante ISO 8601 como lo muestra un input `datetime-local`: hora local, al minuto. */
export function aFechaHoraLocal(iso: string | null | undefined): string {
  if (iso == null) return "";
  const fecha = new Date(iso);
  const local = new Date(fecha.getTime() - fecha.getTimezoneOffset() * 60_000);
  return local.toISOString().slice(0, 16);
}

/** El valor de un `datetime-local` (hora local) como un instante ISO 8601. */
export function deFechaHoraLocal(texto: string): string {
  return new Date(texto).toISOString();
}

export function fechaHoraOpcional(texto: string): string | undefined {
  return texto === "" ? undefined : deFechaHoraLocal(texto);
}

/** Un instante ISO 8601 en castellano, en la zona horaria de quien lo lee: «5 dic 2026, 21:00». */
export function formatearFechaHora(iso: string): string {
  return new Date(iso).toLocaleString("es-ES", { dateStyle: "medium", timeStyle: "short" });
}

/** Céntimos (un objeto valor `dinero`) como los escribe quien usa el formulario: 1250 → «12,50». */
export function aImporte(centimos: number): string {
  return (centimos / 100).toFixed(2).replace(".", ",");
}

/**
 * Un importe escrito en euros, en céntimos: «12,50», «12.5» y «12 €» son 1250. Lo que no es un
 * importe da `NaN`, que la API rechaza.
 */
export function deImporte(texto: string): number {
  const importe = texto.replace(/[\s€]/g, "").replace(",", ".");
  if (!/^-?\d+(\.\d{1,2})?$/.test(importe)) return Number.NaN;
  return Math.round(Number(importe) * 100);
}

/** Céntimos en euros, en castellano: 1250 → «12,50 €». */
export function euros(centimos: number): string {
  return (centimos / 100).toLocaleString("es-ES", { style: "currency", currency: "EUR" });
}

/** Una opción del `<select>` de una referencia: el id que manda y lo que muestra. */
export interface OpcionReferencia {
  id: string;
  etiqueta: string;
}

/** Las opciones del `<select>` de una referencia: ninguna hasta que sus objetivos se cargan. */
export function opcionesReferencia<T extends { id: string }>(
  cargados:
    | { estado: "cargando" }
    | { estado: "error" }
    | { estado: "listo"; datos: { elementos: T[] } },
  etiqueta: (elemento: T) => string,
): OpcionReferencia[] {
  return cargados.estado === "listo"
    ? cargados.datos.elementos.map((elemento) => ({
        id: elemento.id,
        etiqueta: etiqueta(elemento),
      }))
    : [];
}

/**
 * La etiqueta de cada objetivo cargado (su `campo`), por id: lo que nombra sus referencias en una
 * lista. Ninguna hasta que se cargan, ni si la carga falla: cada enlace muestra entonces el id.
 */
export function etiquetasReferencia<T extends { id: string }>(
  cargados:
    | { estado: "cargando" }
    | { estado: "error" }
    | { estado: "listo"; datos: { elementos: T[] } },
  campo: keyof T,
): Map<string, string> {
  const opciones = opcionesReferencia(cargados, (elemento) => String(elemento[campo]));
  return new Map(opciones.map((opcion) => [opcion.id, opcion.etiqueta]));
}
