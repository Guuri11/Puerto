import "./index.css";

import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { Marco } from "./compartido/ui/marco";
import { ProveedorDependencias } from "./dependencias";
import { crearDependencias } from "./generado/dependencias";
import { entradasMenu, RutasApp } from "./generado/rutas";
import { marca } from "./marca/marca";

const raiz = document.getElementById("root");
if (raiz === null) {
  throw new Error("index.html no tiene el elemento #root");
}
document.title = marca.nombre;

createRoot(raiz).render(
  <StrictMode>
    <ProveedorDependencias valor={crearDependencias()}>
      <Marco menu={entradasMenu}>
        <RutasApp />
      </Marco>
    </ProveedorDependencias>
  </StrictMode>,
);
