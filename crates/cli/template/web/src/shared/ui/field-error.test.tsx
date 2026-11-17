import { describe, expect, test } from "bun:test";
import { render, screen } from "@testing-library/react";
import { FieldError } from "./field-error";

describe("FieldError", () => {
  test("should show the message when the problem is about its field", () => {
    // Act
    render(<FieldError problem={{ field: "name", text: "Name is required" }} field="name" />);

    // Assert
    expect(screen.getByText("Name is required")).toBeTruthy();
  });

  test("should show nothing when the problem is about another field", () => {
    // Act
    const { container } = render(
      <FieldError problem={{ field: "sku", text: "Sku is not valid" }} field="name" />,
    );

    // Assert
    expect(container.textContent).toBe("");
  });
});
