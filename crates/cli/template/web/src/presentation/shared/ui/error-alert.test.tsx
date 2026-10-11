import { describe, expect, test } from "bun:test";
import { render, screen } from "@testing-library/react";
import { ErrorAlert } from "./error-alert";

describe("ErrorAlert", () => {
  test("should show the error code when the error carries one", () => {
    // Arrange
    const error = { code: "product.validation_error.name_empty" };

    // Act
    render(<ErrorAlert error={error} />);

    // Assert
    expect(screen.getByRole("alert").textContent).toBe("product.validation_error.name_empty");
  });

  test("should show the message when the request itself failed", () => {
    // Arrange
    const error = new TypeError("Failed to fetch");

    // Act
    render(<ErrorAlert error={error} />);

    // Assert
    expect(screen.getByRole("alert").textContent).toBe("Failed to fetch");
  });
});
