/**
 * Client-side projection of the authoritative live-match event stream.
 *
 * This module deliberately validates and sequences events only. It never
 * derives football results, positions, or events that the Albion server did
 * not publish, which makes it safe to rebuild after a reconnect.
 */
import type { MatchEvent, MatchSnapshot } from "../components/match/types";
import type { AlbionServerEvent } from "./albionServerService";

export type AlbionLiveMatchPresentation = {
  matchId: string | null;
  phase: string | null;
  matchSecond: number;
  homeScore: number;
  awayScore: number;
  lastSequence: number;
  events: MatchEvent[];
  snapshot: MatchSnapshot | null;
  finished: boolean;
};

export const EMPTY_ALBION_LIVE_MATCH: AlbionLiveMatchPresentation = {
  matchId: null,
  phase: null,
  matchSecond: 0,
  homeScore: 0,
  awayScore: 0,
  lastSequence: 0,
  events: [],
  snapshot: null,
  finished: false,
};

function isRecord(value: unknown): value is Record<string, unknown> {
  return Boolean(value) && typeof value === "object" && !Array.isArray(value);
}

/** Accept only the exact, semantic event shape emitted by the Rust engine. */
export function normalizeAlbionMatchEvent(value: unknown): MatchEvent | null {
  if (!isRecord(value)) return null;
  const { minute, event_type: eventType, side, zone, player_id: playerId, secondary_player_id: secondaryPlayerId, detail } = value;
  if (typeof minute !== "number" || !Number.isFinite(minute)
    || typeof eventType !== "string" || (side !== "Home" && side !== "Away") || typeof zone !== "string") return null;
  if (playerId !== null && playerId !== undefined && typeof playerId !== "string") return null;
  if (secondaryPlayerId !== null && secondaryPlayerId !== undefined && typeof secondaryPlayerId !== "string") return null;
  return {
    minute,
    event_type: eventType,
    side,
    zone,
    player_id: playerId ?? null,
    secondary_player_id: secondaryPlayerId ?? null,
    detail: isRecord(detail) ? detail as MatchEvent["detail"] : null,
  };
}

/** A lightweight runtime guard before trusted server JSON reaches the renderer. */
export function readAlbionMatchSnapshot(value: unknown): MatchSnapshot | null {
  if (!isRecord(value)) return null;
  const { phase, current_minute: currentMinute, home_score: homeScore, away_score: awayScore, possession, ball_zone: ballZone, home_team: homeTeam, away_team: awayTeam, events, sent_off: sentOff } = value;
  const validTeam = (team: unknown) => isRecord(team)
    && typeof team.name === "string" && typeof team.formation === "string" && Array.isArray(team.players);
  if (typeof phase !== "string" || typeof currentMinute !== "number" || typeof homeScore !== "number" || typeof awayScore !== "number"
    || (possession !== "Home" && possession !== "Away") || typeof ballZone !== "string" || !validTeam(homeTeam) || !validTeam(awayTeam)
    || !Array.isArray(events) || !Array.isArray(sentOff)) return null;
  return value as unknown as MatchSnapshot;
}

/**
 * Reduces reconnection and live updates into an idempotent display model.
 * Repeated event batches are ignored by their server-assigned sequence range.
 */
export function reduceAlbionLiveMatch(
  current: AlbionLiveMatchPresentation,
  event: AlbionServerEvent,
): AlbionLiveMatchPresentation {
  const body = event.body;
  if (!body) return current;
  if (event.type === "MatchOpened" && typeof body.match_id === "string") {
    return current.matchId === body.match_id
      ? { ...current, finished: false }
      : { ...EMPTY_ALBION_LIVE_MATCH, matchId: body.match_id };
  }
  if (typeof body.match_id !== "string" || body.match_id !== current.matchId) return current;
  if (event.type === "MatchState"
    && typeof body.phase === "string" && typeof body.match_second === "number"
    && typeof body.home_score === "number" && typeof body.away_score === "number") {
    const snapshot = readAlbionMatchSnapshot(body.snapshot) ?? current.snapshot;
    return {
      ...current,
      phase: body.phase,
      matchSecond: body.match_second,
      homeScore: body.home_score,
      awayScore: body.away_score,
      snapshot,
      // A reconnect receives a complete read-only engine snapshot. Replacing
      // the timeline here makes a fresh client immediately match the host.
      events: snapshot?.events ?? current.events,
    };
  }
  if (event.type === "MatchEventBatch" && typeof body.from_seq === "number" && typeof body.to_seq === "number" && Array.isArray(body.events)) {
    if (body.to_seq <= current.lastSequence || body.to_seq < body.from_seq) return current;
    const firstUnseen = Math.max(0, current.lastSequence - body.from_seq + 1);
    const canonicalEvents = body.events.slice(firstUnseen).flatMap((item) => {
      const normalized = normalizeAlbionMatchEvent(item);
      return normalized ? [normalized] : [];
    });
    return { ...current, lastSequence: body.to_seq, events: [...current.events, ...canonicalEvents] };
  }
  if (event.type === "MatchFinished" && typeof body.home_score === "number" && typeof body.away_score === "number") {
    return { ...current, homeScore: body.home_score, awayScore: body.away_score, finished: true };
  }
  return current;
}
