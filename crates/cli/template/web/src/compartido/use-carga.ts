import { type DependencyList, useCallback, useEffect, useState } from "react";

export type EstadoCarga<T> =
  | { estado: "cargando" }
  | { estado: "error"; error: unknown }
  | { estado: "listo"; datos: T };

/**
 * Ejecuta `cargar` al montar y cada vez que cambian `deps`; `recargar()` lo ejecuta otra vez. Un
 * resultado que llega después de que cambien las entradas (o de desmontar) se descarta.
 */
export function useCarga<T>(
  cargar: () => Promise<T>,
  deps: DependencyList,
): EstadoCarga<T> & { recargar: () => void } {
  const [estado, setEstado] = useState<EstadoCarga<T>>({ estado: "cargando" });
  const [intento, setIntento] = useState(0);

  // biome-ignore lint/correctness/useExhaustiveDependencies: las `deps` de quien llama deciden cuándo cambia `cargar`
  useEffect(() => {
    let vigente = true;
    setEstado({ estado: "cargando" });
    cargar().then(
      (datos) => {
        if (vigente) setEstado({ estado: "listo", datos });
      },
      (error: unknown) => {
        if (vigente) setEstado({ estado: "error", error });
      },
    );
    return () => {
      vigente = false;
    };
  }, [...deps, intento]);

  const recargar = useCallback(() => setIntento((n) => n + 1), []);
  return { ...estado, recargar };
}
