import { describe, expect, mock, test } from "bun:test";
import { fireEvent, render, screen } from "@testing-library/react";
import { ConfirmDialog } from "./confirm-dialog";

describe("ConfirmDialog", () => {
  test("should confirm when the confirm button is pressed", () => {
    // Arrange
    const onConfirm = mock(() => {});
    const onCancel = mock(() => {});
    render(
      <ConfirmDialog open message="Delete Alice?" onConfirm={onConfirm} onCancel={onCancel} />,
    );

    // Act
    fireEvent.click(screen.getByText("Delete"));

    // Assert
    expect(onConfirm).toHaveBeenCalledTimes(1);
    expect(onCancel).not.toHaveBeenCalled();
  });

  test("should cancel when the cancel button is pressed", () => {
    // Arrange
    const onConfirm = mock(() => {});
    const onCancel = mock(() => {});
    render(
      <ConfirmDialog open message="Delete Alice?" onConfirm={onConfirm} onCancel={onCancel} />,
    );

    // Act
    fireEvent.click(screen.getByText("Cancel"));

    // Assert
    expect(onCancel).toHaveBeenCalledTimes(1);
    expect(onConfirm).not.toHaveBeenCalled();
  });
});
