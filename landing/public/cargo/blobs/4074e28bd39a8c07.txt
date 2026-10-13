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

  test("should show the given message instead of the code", () => {
    // Arrange
    const error = { code: "product.duplicate" };

    // Act
    render(<ErrorAlert error={error} message="A product with these values already exists" />);

    // Assert
    expect(screen.getByRole("alert").textContent).toBe(
      "A product with these values already exists",
    );
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
