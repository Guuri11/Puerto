import { createContext, type ReactNode, useContext } from "react";
import type { Dependencies } from "./generated/dependencies";

const DependenciesContext = createContext<Dependencies | null>(null);

export function DependenciesProvider({
  value,
  children,
}: {
  value: Dependencies;
  children: ReactNode;
}) {
  return <DependenciesContext value={value}>{children}</DependenciesContext>;
}

/** The API client (and the auth gateway). Tests provide fakes through `DependenciesProvider`. */
export function useDependencies(): Dependencies {
  const dependencies = useContext(DependenciesContext);
  if (dependencies === null) {
    throw new Error("useDependencies() called outside <DependenciesProvider>");
  }
  return dependencies;
}
