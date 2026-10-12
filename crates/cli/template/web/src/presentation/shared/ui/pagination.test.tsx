import { describe, expect, mock, test } from "bun:test";
import { fireEvent, render, screen } from "@testing-library/react";
import { offsetFromSearch, Pagination, pageSearch } from "./pagination";

function button(name: string): HTMLButtonElement {
  return screen.getByRole("button", { name }) as HTMLButtonElement;
}

describe("Pagination", () => {
  test("should show the range and disable Previous on the first page", () => {
    // Arrange
    const onOffsetChange = mock((_: number) => {});

    // Act
    render(<Pagination total={45} limit={20} offset={0} onOffsetChange={onOffsetChange} />);

    // Assert
    expect(screen.getByText("1–20 of 45")).toBeTruthy();
    expect(button("Previous").disabled).toBe(true);
    expect(button("Next").disabled).toBe(false);
  });

  test("should disable Next on the last page", () => {
    // Act
    render(<Pagination total={45} limit={20} offset={40} onOffsetChange={() => {}} />);

    // Assert
    expect(screen.getByText("41–45 of 45")).toBeTruthy();
    expect(button("Next").disabled).toBe(true);
  });

  test("should ask for the next offset when Next is pressed", () => {
    // Arrange
    const onOffsetChange = mock((_: number) => {});
    render(<Pagination total={45} limit={20} offset={20} onOffsetChange={onOffsetChange} />);

    // Act
    fireEvent.click(button("Next"));

    // Assert
    expect(onOffsetChange).toHaveBeenCalledWith(40);
  });

  test("should render nothing when everything fits on one page", () => {
    // Act
    const { container } = render(
      <Pagination total={3} limit={20} offset={0} onOffsetChange={() => {}} />,
    );

    // Assert
    expect(container.textContent).toBe("");
  });

  test("should read the page of the URL as an offset", () => {
    expect(offsetFromSearch("?page=3", 20)).toBe(40);
    expect(offsetFromSearch("", 20)).toBe(0);
    expect(offsetFromSearch("?page=abc", 20)).toBe(0);
    expect(offsetFromSearch("?page=0", 20)).toBe(0);
  });

  test("should write an offset as the page of the URL, none for the first", () => {
    expect(pageSearch(40, 20)).toBe("?page=3");
    expect(pageSearch(0, 20)).toBe("");
  });
});
