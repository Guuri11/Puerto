import "./index.css";

import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { DependenciesProvider } from "./dependencies";
import { createDependencies } from "./generated/dependencies";
import { AppRoutes, navEntries } from "./generated/routes";
import { Layout } from "./shared/ui/layout";

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
