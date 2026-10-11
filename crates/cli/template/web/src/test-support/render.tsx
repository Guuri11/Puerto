import { render } from "@testing-library/react";
import type { ReactNode } from "react";
import { Router } from "wouter";
import { memoryLocation } from "wouter/memory-location";
import { DependenciesProvider } from "../presentation/core/dependencies";
import type { Dependencies } from "../presentation/core/generated/dependencies";

/**
 * Renders a screen with the adapters it uses (`dependencies` needs only those) at `path`.
 * `history` records every location the screen navigates to.
 */
export function renderScreen(ui: ReactNode, dependencies: Partial<Dependencies>, path = "/") {
  const location = memoryLocation({ path, record: true });
  const result = render(
    // A screen that reaches for an adapter it was not given fails loudly: it is `undefined`.
    <DependenciesProvider value={dependencies as Dependencies}>
      <Router hook={location.hook}>{ui}</Router>
    </DependenciesProvider>,
  );
  return { ...result, history: location.history };
}
