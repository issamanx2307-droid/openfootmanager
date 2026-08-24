import { describe, expect, it } from "vitest";

import { applyAlbionServerEvent } from "./albionServerService";

describe("applyAlbionServerEvent", () => {
  it("keeps a manager-specific view and advances its authoritative revision", () => {
    const initial = { revision: 2, views: new Map<string, unknown>() };
    const snapshot = applyAlbionServerEvent(initial, {
      type: "ViewSnapshot",
      body: { revision: 3, view_name: "dashboard", payload: { club: { name: "Albion" } } },
    });
    const acknowledged = applyAlbionServerEvent(snapshot, {
      type: "CommandAck",
      body: { applied_revision: 4 },
    });

    expect(snapshot.views.get("dashboard")).toEqual({ club: { name: "Albion" } });
    expect(acknowledged.revision).toBe(4);
  });

  it("uses a rejection revision to resync the next command", () => {
    const cache = applyAlbionServerEvent({ revision: 5, views: new Map() }, {
      type: "CommandRejected",
      body: { current_revision: 7 },
    });
    expect(cache.revision).toBe(7);
  });
});
