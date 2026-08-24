import { describe, expect, it } from "vitest";

import {
  applyAlbionServerEvent,
  clearAlbionSession,
  formatAlbionLiveEvent,
  loadAlbionSession,
  saveAlbionSession,
  serializeAlbionCommand,
} from "./albionServerService";

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

describe("formatAlbionLiveEvent", () => {
  it("renders canonical event facts in English and Thai", () => {
    const event = { minute: 72, event_type: "Goal", side: "Home" };
    expect(formatAlbionLiveEvent(event, false)).toBe("72′ Goal · Home");
    expect(formatAlbionLiveEvent(event, true)).toBe("72′ ประตู · Home");
  });
});

describe("serializeAlbionCommand", () => {
  it("uses the internally tagged Rust command wire shape", () => {
    expect(serializeAlbionCommand({ SetStartingXi: {
      fixture_id: "fpl-fixture-1", player_ids: ["fpl-1"], formation: "4-3-3",
    } })).toEqual({ type: "SetStartingXi", body: {
      fixture_id: "fpl-fixture-1", player_ids: ["fpl-1"], formation: "4-3-3",
    } });
  });
});

describe("Albion session storage", () => {
  it("restores the reconnect token and can clear it", () => {
    clearAlbionSession();
    saveAlbionSession({
      server_url: "http://127.0.0.1:38421",
      career_id: "career", manager_id: "manager", reconnect_token: "token",
      current_revision: 3, slot: "host",
    });
    expect(loadAlbionSession()?.reconnect_token).toBe("token");
    clearAlbionSession();
    expect(loadAlbionSession()).toBeNull();
  });
});
