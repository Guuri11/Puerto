import { describe, expect, test } from "bun:test";
import { act, renderHook, waitFor } from "@testing-library/react";
import { useAsync } from "./use-async";

describe("useAsync", () => {
  test("should expose the data when the call resolves", async () => {
    // Arrange
    const run = () => Promise.resolve(["Alice", "Bob"]);

    // Act
    const { result } = renderHook(() => useAsync(run, []));

    // Assert
    expect(result.current.status).toBe("loading");
    await waitFor(() => expect(result.current.status).toBe("ok"));
    expect(result.current).toMatchObject({ data: ["Alice", "Bob"] });
  });

  test("should expose the error when the call rejects", async () => {
    // Arrange
    const failure = { code: "product.repository_error" };
    const run = () => Promise.reject(failure);

    // Act
    const { result } = renderHook(() => useAsync(run, []));

    // Assert
    await waitFor(() => expect(result.current.status).toBe("error"));
    expect(result.current).toMatchObject({ error: failure });
  });

  test("should run the call again when reloaded", async () => {
    // Arrange
    let calls = 0;
    const run = () => Promise.resolve(++calls);
    const { result } = renderHook(() => useAsync(run, []));
    await waitFor(() => expect(result.current).toMatchObject({ data: 1 }));

    // Act
    act(() => result.current.reload());

    // Assert
    await waitFor(() => expect(result.current).toMatchObject({ data: 2 }));
  });
});
