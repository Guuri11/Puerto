import { createContext, type ReactNode, useContext } from "react";
import type { Dependencias } from "./generado/dependencias";

const ContextoDependencias = createContext<Dependencias | null>(null);

export function ProveedorDependencias({
  valor,
  children,
}: {
  valor: Dependencias;
  children: ReactNode;
}) {
  return <ContextoDependencias value={valor}>{children}</ContextoDependencias>;
}

/** El cliente de la API (y la pasarela de auth). Los tests dan falsos con `ProveedorDependencias`. */
export function useDependencias(): Dependencias {
  const dependencias = useContext(ContextoDependencias);
  if (dependencias === null) {
    throw new Error("useDependencias() llamado fuera de <ProveedorDependencias>");
  }
  return dependencias;
}
