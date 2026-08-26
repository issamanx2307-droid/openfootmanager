import { fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

import ContextMenu from "./ContextMenu";

afterEach(() => {
  vi.restoreAllMocks();
});

describe("ContextMenu", () => {
  it("renders repeated divider items without duplicate React keys", () => {
    const consoleError = vi.spyOn(console, "error").mockImplementation(() => {});

    render(
      <ContextMenu
        items={[
          { label: "First action", onClick: vi.fn() },
          { label: "", divider: true },
          { label: "Second action", onClick: vi.fn() },
          { label: "", divider: true },
          { label: "Third action", onClick: vi.fn() },
        ]}
      >
        <button type="button">Open menu</button>
      </ContextMenu>,
    );

    fireEvent.contextMenu(screen.getByRole("button", { name: "Open menu" }));

    expect(screen.getByRole("menu")).toBeInTheDocument();
    expect(consoleError).not.toHaveBeenCalledWith(
      expect.stringContaining("same key"),
    );
  });
});
