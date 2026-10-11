import { describe, expect, test } from "bun:test";
import { render, screen } from "@testing-library/react";
import { HomePage } from "./home";

describe("HomePage", () => {
  test("should link every entity when there are some", () => {
    // Arrange
    const entries = [{ label: "Products", href: "/products" }];

    // Act
    render(<HomePage entries={entries} />);

    // Assert
    expect(screen.getByText("Products").getAttribute("href")).toBe("/products");
  });

  test("should explain how to add an entity when there is none", () => {
    // Act
    render(<HomePage entries={[]} />);

    // Assert
    expect(screen.getByText(/puerto generate scaffold/)).toBeTruthy();
  });
});
