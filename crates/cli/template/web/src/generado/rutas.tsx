// GENERADO por puerto. Edita puerto.toml y después ejecuta `puerto generar bootstrap`.

import { Route, Switch } from "wouter";
import type { EntradaMenu } from "../compartido/ui/marco";
import { PaginaInicio } from "../paginas/inicio";
import { PaginaNoEncontrada } from "../paginas/no-encontrada";
import { rutasPropias } from "../rutas";

export const entradasMenu: EntradaMenu[] = [];

export function RutasApp() {
  return (
    <Switch>
      <Route path="/">
        <PaginaInicio entradas={entradasMenu} />
      </Route>
      {rutasPropias}
      <Route>
        <PaginaNoEncontrada />
      </Route>
    </Switch>
  );
}
