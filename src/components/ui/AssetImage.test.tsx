import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import AssetImage from "./AssetImage";

describe("AssetImage", () => {
  it("shows the fallback after an image fails to load", () => {
    render(
      <AssetImage
        alt="Club badge"
        fallback={<span>Fallback badge</span>}
        src="/broken-badge.svg"
      />,
    );

    fireEvent.error(screen.getByRole("img", { name: "Club badge" }));

    expect(screen.getByText("Fallback badge")).toBeInTheDocument();
    expect(screen.queryByRole("img", { name: "Club badge" })).not.toBeInTheDocument();
  });

  it("retries when a later render provides a different asset URL", () => {
    const { rerender } = render(
      <AssetImage
        alt="Club badge"
        fallback={<span>Fallback badge</span>}
        src="/broken-badge.svg"
      />,
    );

    fireEvent.error(screen.getByRole("img", { name: "Club badge" }));
    rerender(
      <AssetImage
        alt="Club badge"
        fallback={<span>Fallback badge</span>}
        src="/replacement-badge.svg"
      />,
    );

    expect(screen.getByRole("img", { name: "Club badge" })).toHaveAttribute(
      "src",
      "/replacement-badge.svg",
    );
  });
});
