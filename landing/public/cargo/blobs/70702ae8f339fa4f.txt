import "./index.css";

import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { DependenciesProvider } from "./presentation/core/dependencies";
import { createDependencies } from "./presentation/core/generated/dependencies";
import { AppRoutes, navEntries } from "./presentation/core/generated/routes";
import { Layout } from "./presentation/shared/ui/layout";

const root = document.getElementById("root");
if (root === null) {
  throw new Error("index.html has no #root element");
}

createRoot(root).render(
  <StrictMode>
    <DependenciesProvider value={createDependencies()}>
      <Layout title={document.title} nav={navEntries}>
        <AppRoutes />
      </Layout>
    </DependenciesProvider>
  </StrictMode>,
);
