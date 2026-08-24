import { describe, expect, it } from "vitest";

import { fitPitchViewport } from "./Match2DRenderer";

describe("fitPitchViewport", () => {
  it("letterboxes a wide container without stretching the pitch", () => {
    expect(fitPitchViewport(1600, 600)).toEqual({
      x: 336.7647058823529,
      y: 0,
      width: 926.4705882352941,
      height: 600,
    });
  });

  it("letterboxes a tall container without stretching the pitch", () => {
    expect(fitPitchViewport(600, 900)).toEqual({
      x: 0,
      y: 255.71428571428572,
      width: 600,
      height: 388.57142857142856,
    });
  });
});
