import { describe, expect, it } from "vitest";

import type { EnginePlayerData, MatchEvent } from "../match/types";
import {
  compilePresentationTimeline,
  ballStateForEvent,
  filterPresentationTimeline,
  presentationFrame,
  playerVisualStatus,
  roleAbbreviation,
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

  it("makes authoritative play styles visibly change the formation shape", () => {
    const players = [player("gk", "Goalkeeper"), ...Array.from({ length: 10 }, (_, index) => player(`p${index}`, "Midfielder"))];
    const balanced = resolveTeamPositions("Home", "4-3-3", players, [], false, "Balanced");
    const attacking = resolveTeamPositions("Home", "4-3-3", players, [], false, "Attacking");
    const possession = resolveTeamPositions("Home", "4-3-3", players, [], false, "Possession");
    expect(attacking[4].point.x).toBeGreaterThan(balanced[4].point.x);
    expect(Math.abs(possession[1].point.y - 0.5)).toBeGreaterThan(Math.abs(balanced[1].point.y - 0.5));
  });

  it("orders source events deterministically and filters highlights without re-simulation", () => {
    const clips = compilePresentationTimeline([
      event(12, "PassCompleted", "Home"),
      event(8, "Goal", "Away", "AttackingBox"),
      event(15, "ShotOffTarget", "Home", "AttackingCentre"),
    ]);
    expect(clips.map((clip) => clip.event.minute)).toEqual([8, 12, 15]);
    expect(filterPresentationTimeline(clips, "key").map((clip) => clip.event.event_type)).toEqual(["Goal"]);
    expect(filterPresentationTimeline(clips, "extended").map((clip) => clip.event.event_type)).toEqual(["Goal", "ShotOffTarget"]);
  });

  it("classifies the canonical Rust shooting and set-piece event names", () => {
    const clips = compilePresentationTimeline([
      event(7, "PassCompleted", "Home"),
      event(8, "ShotOnTarget", "Home", "AttackingCentre"),
      event(9, "ShotSaved", "Away", "DefensiveBox"),
      event(10, "Corner", "Home", "AttackingRight"),
    ]);
    expect(filterPresentationTimeline(clips, "key").map((clip) => clip.event.event_type)).toEqual(["ShotOnTarget", "ShotSaved"]);
    expect(filterPresentationTimeline(clips, "extended").map((clip) => clip.event.event_type)).toEqual(["ShotOnTarget", "ShotSaved", "Corner"]);
  });

  it("maps authoritative event facts to distinct ball presentation states", () => {
    expect(ballStateForEvent(event(1, "PassCompleted", "Home"))).toBe("passing");
    expect(ballStateForEvent(event(2, "Cross", "Home"))).toBe("crossing");
    expect(ballStateForEvent(event(3, "ShotSaved", "Away"))).toBe("save");
    expect(ballStateForEvent(event(4, "Tackle", "Away"))).toBe("loose");
    expect(ballStateForEvent(event(5, "FreeKick", "Home"))).toBe("restart");
    expect(ballStateForEvent(event(6, "PenaltyGoal", "Home"))).toBe("penalty");
    expect(ballStateForEvent(event(7, "ShotOffTarget", "Away"))).toBe("out-of-play");
  });

  it("moves only semantic event participants in the presentation frame", () => {
    const players = [player("gk", "Goalkeeper"), player("runner", "Forward"), player("receiver", "Forward")];
    const input = {
      home_team: { id: "home", name: "Home", formation: "4-3-3", play_style: "Balanced", players },
      away_team: { id: "away", name: "Away", formation: "4-3-3", play_style: "Balanced", players: [player("away-gk", "Goalkeeper")] },
      sent_off: [], ball_zone: "MidfieldCentre", current_minute: 20,
      events: [{ ...event(10, "Dribble", "Home", "AttackingCentre"), player_id: "runner", secondary_player_id: "receiver" }],
    };
    const start = presentationFrame(input, 0);
    const mid = presentationFrame(input, 280);
    expect(mid.players.find((item) => item.id === "runner")?.point).not.toEqual(start.players.find((item) => item.id === "runner")?.point);
    expect(mid.players.find((item) => item.id === "gk")?.point).toEqual(start.players.find((item) => item.id === "gk")?.point);
  });

  it("keeps the ball close to its authoritative dribble actor", () => {
    const players = [player("gk", "Goalkeeper"), player("runner", "Forward")];
    const input = {
      home_team: { id: "home", name: "Home", formation: "4-3-3", play_style: "Balanced", players },
      away_team: { id: "away", name: "Away", formation: "4-3-3", play_style: "Balanced", players: [player("away-gk", "Goalkeeper")] },
      sent_off: [], ball_zone: "MidfieldCentre", current_minute: 20,
      events: [{ ...event(10, "Dribble", "Home", "AttackingCentre"), player_id: "runner" }],
    };
    const live = presentationFrame(input, 280);
    const runner = live.players.find((item) => item.id === "runner")!;
    const distanceToBall = Math.hypot(runner.point.x - live.ball.x, runner.point.y - live.ball.y);

    expect(distanceToBall).toBeLessThan(0.05);
  });

  it("brings the authoritative incoming substitute on from the touchline", () => {
    const incoming = player("incoming", "Forward");
    const input = {
      home_team: { id: "home", name: "Home", formation: "4-3-3", play_style: "Balanced", players: [player("gk", "Goalkeeper"), incoming] },
      away_team: { id: "away", name: "Away", formation: "4-3-3", play_style: "Balanced", players: [player("away-gk", "Goalkeeper")] },
      sent_off: [], ball_zone: "MidfieldCentre", current_minute: 66,
      events: [{ ...event(66, "Substitution", "Home"), player_id: "incoming", secondary_player_id: "outgoing" }],
    };
    const entering = presentationFrame(input, 0).players.find((item) => item.id === "incoming");
    const settled = presentationFrame(input, 559).players.find((item) => item.id === "incoming");

    expect(entering?.point.y).toBe(0.97);
    expect(settled?.point.y).not.toBe(0.97);
    expect(entering?.point.x).toBe(settled?.point.x);
  });

  it("moves the defending goalkeeper towards an authoritative save", () => {
    const input = {
      home_team: { id: "home", name: "Home", formation: "4-3-3", play_style: "Balanced", players: [player("home-gk", "Goalkeeper"), player("shooter", "Forward")] },
      away_team: { id: "away", name: "Away", formation: "4-3-3", play_style: "Balanced", players: [player("away-gk", "Goalkeeper")] },
      sent_off: [], ball_zone: "AttackingBox", current_minute: 31,
      events: [{ ...event(31, "ShotSaved", "Home", "AttackingBox"), player_id: "shooter" }],
    };
    const startFrame = presentationFrame(input, 0);
    const savingFrame = presentationFrame(input, 450);
    const start = startFrame.players.find((item) => item.id === "away-gk");
    const saving = savingFrame.players.find((item) => item.id === "away-gk");

    expect(saving?.point).not.toEqual(start?.point);
    expect(Math.abs((saving?.point.x ?? 0) - savingFrame.ball.x)).toBeLessThan(
      Math.abs((start?.point.x ?? 0) - startFrame.ball.x),
    );
  });

  it("keeps formation lanes while teams support and compress around live play", () => {
    const input = {
      home_team: { id: "home", name: "Home", formation: "4-3-3", play_style: "Balanced", players: [player("home-gk", "Goalkeeper"), player("home-defender", "Defender"), player("carrier", "Midfielder")] },
      away_team: { id: "away", name: "Away", formation: "4-3-3", play_style: "Balanced", players: [player("away-gk", "Goalkeeper"), player("away-defender", "Defender")] },
      sent_off: [], ball_zone: "AttackingRight", current_minute: 36,
      events: [{ ...event(36, "Dribble", "Home", "AttackingRight"), player_id: "carrier" }],
    };
    const start = presentationFrame(input, 0);
    const live = presentationFrame(input, 350);
    const homeDefenderAtStart = start.players.find((item) => item.id === "home-defender")!;
    const homeDefenderLive = live.players.find((item) => item.id === "home-defender")!;
    const awayDefenderAtStart = start.players.find((item) => item.id === "away-defender")!;
    const awayDefenderLive = live.players.find((item) => item.id === "away-defender")!;
    const eventTarget = zonePoint("AttackingRight", "Home");

    expect(homeDefenderLive.point).not.toEqual(homeDefenderAtStart.point);
    expect(awayDefenderLive.point).not.toEqual(awayDefenderAtStart.point);
    expect(Math.abs(awayDefenderLive.point.y - eventTarget.y)).toBeLessThan(
      Math.abs(awayDefenderAtStart.point.y - eventTarget.y),
    );
    expect(homeDefenderLive.point).not.toEqual(live.ball);
    expect(awayDefenderLive.point).not.toEqual(live.ball);
  });

  it("derives card and injury badges from authoritative snapshot facts", () => {
    const events = [event(64, "Injury", "Home")];
    events[0].player_id = "runner";
    expect(playerVisualStatus("runner", { runner: 1 }, events)).toEqual({ yellowCards: 1, injured: true });
    expect(playerVisualStatus("other", { runner: 1 }, events)).toEqual({ yellowCards: 0, injured: false });
  });

  it("uses concise deterministic role labels", () => {
    expect(roleAbbreviation("DefensiveMidfielder")).toBe("DM");
    expect(roleAbbreviation("Balanced")).toBe("BAL");
  });
});
