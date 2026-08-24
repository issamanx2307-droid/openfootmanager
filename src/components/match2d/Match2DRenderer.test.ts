import { describe, expect, it } from "vitest";

import { fitPitchViewport, shouldShowEventOverlay } from "./Match2DRenderer";

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

  it("shows an event cue for cards, injuries and substitutions", () => {
    const event = { minute: 61, side: "Home" as const, zone: "MidfieldCentre", player_id: "p1", secondary_player_id: null };
    expect(shouldShowEventOverlay({ ...event, event_type: "YellowCard" })).toBe(true);
    expect(shouldShowEventOverlay({ ...event, event_type: "Injury" })).toBe(true);
    expect(shouldShowEventOverlay({ ...event, event_type: "Substitution" })).toBe(true);
    expect(shouldShowEventOverlay({ ...event, event_type: "PassCompleted" })).toBe(false);
  });
});
