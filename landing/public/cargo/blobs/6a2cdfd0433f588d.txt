import { type DependencyList, useCallback, useEffect, useState } from "react";

export type AsyncState<T> =
  | { status: "loading" }
  | { status: "error"; error: unknown }
  | { status: "ok"; data: T };

/**
 * Runs `run` on mount and again whenever `deps` change; `reload()` runs it once more. A result
 * that arrives after the inputs changed (or after unmount) is dropped.
 */
export function useAsync<T>(
  run: () => Promise<T>,
  deps: DependencyList,
): AsyncState<T> & { reload: () => void } {
  const [state, setState] = useState<AsyncState<T>>({ status: "loading" });
  const [attempt, setAttempt] = useState(0);

  // biome-ignore lint/correctness/useExhaustiveDependencies: the caller's `deps` decide when `run` changes
  useEffect(() => {
    let current = true;
    setState({ status: "loading" });
    run().then(
      (data) => {
        if (current) setState({ status: "ok", data });
      },
      (error: unknown) => {
        if (current) setState({ status: "error", error });
      },
    );
    return () => {
      current = false;
    };
  }, [...deps, attempt]);

  const reload = useCallback(() => setAttempt((n) => n + 1), []);
  return { ...state, reload };
}
