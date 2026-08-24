import { describe, expect, it } from "vitest";

import en from "./locales/en.json";
import th from "./locales/th.json";

describe("locale coverage", () => {
  it("keeps English and Thai translations available for primary navigation", () => {
    expect(en.menu.newGame).toBe("New Game");
    expect(th.menu.newGame).toBe("เริ่มเกมใหม่");
    expect(en.dashboard.multiplayer).toBe("Play together");
    expect(th.dashboard.multiplayer).toBe("เล่นร่วมกัน");
  });
});
