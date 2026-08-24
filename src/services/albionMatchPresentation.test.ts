import { describe, expect, it } from "vitest";

import { EMPTY_ALBION_LIVE_MATCH, isReplayableAlbionEvent, needsAlbionIntermissionReady, normalizeAlbionMatchEvent, readAlbionMatchSnapshot, reduceAlbionLiveMatch } from "./albionMatchPresentation";

const opened = { type: "MatchOpened", body: { match_id: "match-1" } };
const goal = { minute: 12, event_type: "Goal", side: "Home", zone: "AttackingBox", player_id: "p-9", secondary_player_id: null };

describe("Albion live-match presentation adapter", () => {
  it("keeps only canonical engine events", () => {
    expect(normalizeAlbionMatchEvent(goal)).toMatchObject({ event_type: "Goal", player_id: "p-9" });
    expect(normalizeAlbionMatchEvent({ ...goal, side: "Neutral" })).toBeNull();
  });

  it("admits only a renderer-safe authoritative snapshot", () => {
    expect(readAlbionMatchSnapshot({ phase: "FirstHalf", current_minute: 12, home_score: 0, away_score: 0, possession: "Home", ball_zone: "MidfieldCentre", home_team: { name: "Home", formation: "4-3-3", players: [] }, away_team: { name: "Away", formation: "4-3-3", players: [] }, events: [], sent_off: [] })).toMatchObject({ current_minute: 12 });
    expect(readAlbionMatchSnapshot({ phase: "FirstHalf" })).toBeNull();
  });

  it("applies event batches once and preserves the server score", () => {
    const openState = reduceAlbionLiveMatch(EMPTY_ALBION_LIVE_MATCH, opened);
    const afterBatch = reduceAlbionLiveMatch(openState, { type: "MatchEventBatch", body: { match_id: "match-1", from_seq: 1, to_seq: 1, events: [goal] } });
    const repeated = reduceAlbionLiveMatch(afterBatch, { type: "MatchEventBatch", body: { match_id: "match-1", from_seq: 1, to_seq: 1, events: [goal] } });
    const afterState = reduceAlbionLiveMatch(repeated, { type: "MatchState", body: { match_id: "match-1", phase: "FirstHalf", match_second: 720, home_score: 1, away_score: 0 } });
    expect(repeated.events).toHaveLength(1);
    expect(afterState).toMatchObject({ homeScore: 1, awayScore: 0, matchSecond: 720 });
  });

  it("keeps the presentation timeline when reconnecting to the same match", () => {
    const state = reduceAlbionLiveMatch(
      reduceAlbionLiveMatch(EMPTY_ALBION_LIVE_MATCH, opened),
      { type: "MatchEventBatch", body: { match_id: "match-1", from_seq: 1, to_seq: 1, events: [goal] } },
    );
    expect(reduceAlbionLiveMatch(state, opened).events).toEqual([expect.objectContaining({ event_type: "Goal" })]);
  });

  it("rebuilds the event timeline from an authoritative reconnect snapshot", () => {
    const snapshot = { phase: "FirstHalf", current_minute: 12, home_score: 1, away_score: 0, possession: "Home", ball_zone: "AttackingBox", home_team: { name: "Home", formation: "4-3-3", players: [] }, away_team: { name: "Away", formation: "4-3-3", players: [] }, events: [goal], sent_off: [] };
    const reconnected = reduceAlbionLiveMatch(
      reduceAlbionLiveMatch(EMPTY_ALBION_LIVE_MATCH, opened),
      { type: "MatchState", body: { match_id: "match-1", phase: "FirstHalf", match_second: 720, home_score: 1, away_score: 0, snapshot } },
    );
    expect(reconnected.events).toEqual([goal]);
    expect(reconnected.snapshot).toMatchObject({ current_minute: 12 });
  });

  it("requires both managers to resume only at authoritative intervals", () => {
    expect(needsAlbionIntermissionReady("HalfTime")).toBe(true);
    expect(needsAlbionIntermissionReady("ExtraTimeHalfTime")).toBe(true);
    expect(needsAlbionIntermissionReady("SecondHalf")).toBe(false);
  });

  it("replays only already-published important events", () => {
    expect(isReplayableAlbionEvent(goal)).toBe(true);
    expect(isReplayableAlbionEvent({ ...goal, event_type: "PassCompleted" })).toBe(false);
  });
});
