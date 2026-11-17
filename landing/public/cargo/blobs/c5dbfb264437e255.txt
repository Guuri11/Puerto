import { describe, expect, test } from "bun:test";
import { render, screen } from "@testing-library/react";
import { labelOf, ReferenceLink } from "./reference-link";

describe("ReferenceLink", () => {
  test("should link to the page of the referenced item", () => {
    // Act
    render(<ReferenceLink path="/customers" id="c1" />);

    // Assert
    expect(screen.getByRole("link", { name: "c1" }).getAttribute("href")).toBe("/customers/c1");
  });

  test("should name the referenced item by its label once it is loaded", async () => {
    // Arrange
    const customers = { get: async (id: string) => ({ id, name: "Ada" }) };

    // Act
    render(<ReferenceLink path="/customers" id="c1" load={labelOf(customers, (c) => c.name)} />);

    // Assert
    const link = await screen.findByRole("link", { name: "Ada" });
    expect(link.getAttribute("href")).toBe("/customers/c1");
  });

  test("should keep the id when the referenced item cannot be loaded", async () => {
    // Arrange
    const load = () => Promise.reject(new Error("gone"));

    // Act
    render(<ReferenceLink path="/customers" id="c1" load={load} />);

    // Assert
    expect((await screen.findByRole("link")).textContent).toBe("c1");
  });

  test("should show a dash when there is no reference", () => {
    // Act
    const { container } = render(<ReferenceLink path="/customers" id={null} />);

    // Assert
    expect(container.textContent).toBe("—");
  });
});
