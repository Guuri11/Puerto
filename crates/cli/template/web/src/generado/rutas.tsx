// GENERADO por puerto. Edita puerto.toml y después ejecuta `puerto generar bootstrap`.

import { Route, Switch } from "wouter";
import type { EntradaMenu } from "../compartido/ui/marco";
import { PaginaNoEncontrada } from "../paginas/no-encontrada";
import { PaginaPanel } from "../paginas/panel";
import { rutasPropias } from "../rutas";

/** Dónde va la app: en la raíz o, con portada, en `[proyecto] ruta_app` (`puerto nuevo --portada`). */
export const rutaApp = "/";

export const entradasMenu: EntradaMenu[] = [];

export function RutasApp() {
  return (
    <Switch>
      <Route path="/">
        <PaginaPanel entradas={entradasMenu} />
      </Route>
      {rutasPropias}
      <Route>
        <PaginaNoEncontrada />
      </Route>
    </Switch>
  );
}
