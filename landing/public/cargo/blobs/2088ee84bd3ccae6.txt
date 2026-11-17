import { render } from "@testing-library/react";
import type { ReactNode } from "react";
import { Router } from "wouter";
import { memoryLocation } from "wouter/memory-location";
import { ProveedorDependencias } from "../dependencias";
import type { Dependencias } from "../generado/dependencias";

/**
 * Pinta una pantalla en `ruta` con las dependencias que usa (`dependencias` solo necesita esas).
 * `historial` guarda cada ruta a la que navega la pantalla.
 */
export function renderizarPantalla(ui: ReactNode, dependencias: Partial<Dependencias>, ruta = "/") {
  const ubicacion = memoryLocation({ path: ruta, record: true });
  const resultado = render(
    // Una pantalla que pide una dependencia que no se le dio falla a la vista: es `undefined`.
    <ProveedorDependencias valor={dependencias as Dependencias}>
      <Router hook={ubicacion.hook}>{ui}</Router>
    </ProveedorDependencias>,
  );
  return { ...resultado, historial: ubicacion.history };
}
