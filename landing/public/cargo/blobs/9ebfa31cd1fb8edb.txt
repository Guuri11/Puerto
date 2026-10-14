import { describe, expect, test } from "bun:test";
import { render, screen } from "@testing-library/react";
import { ReferenceLink } from "./reference-link";

describe("ReferenceLink", () => {
  test("should link to the page of the referenced item", () => {
    // Act
    render(<ReferenceLink path="/customers" id="c1" />);

    // Assert
    expect(screen.getByRole("link", { name: "c1" }).getAttribute("href")).toBe("/customers/c1");
  });

  test("should show a dash when there is no reference", () => {
    // Act
    const { container } = render(<ReferenceLink path="/customers" id={null} />);

    // Assert
    expect(container.textContent).toBe("—");
  });
});
