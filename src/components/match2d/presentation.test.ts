import { describe, expect, it } from "vitest";

import type { EnginePlayerData, MatchEvent } from "../match/types";
import {
  compilePresentationTimeline,
  filterPresentationTimeline,
  resolveTeamPositions,
  zonePoint,
} from "./presentation";

function player(id: string, position: string): EnginePlayerData {
  return {
    id,
    name: id,
    position,
    ovr: 70,
    condition: 100,
    pace: 70,
    stamina: 70,
    strength: 70,
    agility: 70,
    passing: 70,
    shooting: 70,
    tackling: 70,
    dribbling: 70,
    defending: 70,
    positioning: 70,
    vision: 70,
    decisions: 70,
    composure: 70,
    aggression: 70,
    teamwork: 70,
    leadership: 70,
    handling: 70,
    reflexes: 70,
    aerial: 70,
    traits: [],
    role: "Balanced",
  };
}

function event(minute: number, event_type: string, side: "Home" | "Away", zone = "MidfieldCentre"): MatchEvent {
  return { minute, event_type, side, zone, player_id: null, secondary_player_id: null };
}

describe("2D match presentation", () => {
  it("mirrors away zone positions while retaining a normalized pitch", () => {
    expect(zonePoint("AttackingBox", "Home")).toEqual({ x: 0.88, y: 0.5 });
    expect(zonePoint("AttackingBox", "Away")).toEqual({ x: 0.12, y: 0.5 });
  });

  it("keeps active formation markers in pitch bounds and excludes sent-off players", () => {
    const players = [player("gk", "Goalkeeper"), ...Array.from({ length: 10 }, (_, index) => player(`p${index}`, index < 4 ? "Defender" : index < 8 ? "Midfielder" : "Forward"))];
    const positions = resolveTeamPositions("Home", "4-3-3", players, ["p0"], false);
    expect(positions).toHaveLength(10);
    expect(positions.every(({ point }) => point.x >= 0.04 && point.x <= 0.96 && point.y >= 0.04 && point.y <= 0.96)).toBe(true);
    expect(positions.some(({ id }) => id === "p0")).toBe(false);
  });

  it("uses a pure halftime transform without changing player identity", () => {
    const players = [player("gk", "Goalkeeper"), ...Array.from({ length: 10 }, (_, index) => player(`p${index}`, "Midfielder"))];
    const firstHalf = resolveTeamPositions("Home", "4-3-3", players, [], false);
    const secondHalf = resolveTeamPositions("Home", "4-3-3", players, [], true);
    expect(secondHalf.map(({ id }) => id)).toEqual(firstHalf.map(({ id }) => id));
    expect(secondHalf[0].point).toEqual({ x: 1 - firstHalf[0].point.x, y: 1 - firstHalf[0].point.y });
  });

  it("orders source events deterministically and filters highlights without re-simulation", () => {
    const clips = compilePresentationTimeline([
      event(12, "Pass", "Home"),
      event(8, "Goal", "Away", "AttackingBox"),
      event(15, "Shot", "Home", "AttackingCentre"),
    ]);
    expect(clips.map((clip) => clip.event.minute)).toEqual([8, 12, 15]);
    expect(filterPresentationTimeline(clips, "key").map((clip) => clip.event.event_type)).toEqual(["Goal"]);
    expect(filterPresentationTimeline(clips, "extended").map((clip) => clip.event.event_type)).toEqual(["Goal", "Shot"]);
  });
});
