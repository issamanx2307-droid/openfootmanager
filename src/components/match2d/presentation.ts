import { buildFormationSlots } from "../match/FormationPitch";
import type { EnginePlayerData, MatchEvent } from "../match/types";
import type {
  HighlightMode,
  MatchPresentationFrame,
  MatchPresentationInput,
  PitchPoint,
  PresentationClip,
  PresentationPlayer,
  PresentationSide,
} from "./types";

const PITCH_MARGIN = 0.04;
const HOME_BALL: PitchPoint = { x: 0.5, y: 0.5 };

const ZONES: Record<string, PitchPoint> = {
  DefensiveBox: { x: 0.12, y: 0.5 },
  DefensiveLeft: { x: 0.27, y: 0.23 },
  DefensiveCentre: { x: 0.27, y: 0.5 },
  DefensiveRight: { x: 0.27, y: 0.77 },
  MidfieldLeft: { x: 0.5, y: 0.23 },
  MidfieldCentre: { x: 0.5, y: 0.5 },
  MidfieldRight: { x: 0.5, y: 0.77 },
  AttackingLeft: { x: 0.73, y: 0.23 },
  AttackingCentre: { x: 0.73, y: 0.5 },
  AttackingRight: { x: 0.73, y: 0.77 },
  AttackingBox: { x: 0.88, y: 0.5 },
};

function clamp(value: number): number {
  return Math.max(PITCH_MARGIN, Math.min(1 - PITCH_MARGIN, value));
}

function mirror(point: PitchPoint): PitchPoint {
  return { x: 1 - point.x, y: 1 - point.y };
}

function eventImportance(event: MatchEvent): PresentationClip["importance"] {
  if (["Goal", "PenaltyGoal", "PenaltyMiss", "RedCard", "SecondYellow", "Save"].includes(event.event_type)) return "key";
  if (["Shot", "Corner", "FreeKick", "Substitution", "YellowCard", "Injury"].includes(event.event_type)) return "extended";
  return "normal";
}

function eventDuration(event: MatchEvent): number {
  if (["Goal", "PenaltyGoal", "PenaltyMiss", "Shot", "Save"].includes(event.event_type)) return 900;
  if (["LongPass", "ThroughBall", "Cross"].includes(event.event_type)) return 720;
  if (["Tackle", "Interception", "Foul"].includes(event.event_type)) return 420;
  return 560;
}

export function zonePoint(zone: string, side: PresentationSide = "Home"): PitchPoint {
  const base = ZONES[zone] ?? HOME_BALL;
  return side === "Home" ? base : mirror(base);
}

/**
 * Places the existing deterministic formation slots on a horizontal pitch.
 * The second-half mirror is visual-only; stored event semantics remain stable.
 */
export function resolveTeamPositions(
  side: PresentationSide,
  formation: string,
  players: EnginePlayerData[],
  sentOff: string[],
  secondHalf: boolean,
): PresentationPlayer[] {
  const slots = buildFormationSlots(formation, players, sentOff);
  return slots.map(({ player, x, y }) => {
    const homePoint = { x: clamp(1 - y / 100), y: clamp(x / 100) };
    const sidePoint = side === "Home" ? homePoint : mirror(homePoint);
    const point = secondHalf ? mirror(sidePoint) : sidePoint;
    return {
      id: player.id,
      side,
      player,
      point,
      goalkeeper: player.position === "Goalkeeper",
      sentOff: sentOff.includes(player.id),
    };
  });
}

export function compilePresentationTimeline(events: MatchEvent[]): PresentationClip[] {
  let startMs = 0;
  const orderedEvents = [...events].sort((a, b) => a.minute - b.minute);
  return orderedEvents
    .map((event, index) => {
      const ballTo = zonePoint(event.zone, event.side);
      const durationMs = eventDuration(event);
      const clip: PresentationClip = {
        clipId: `${event.minute}-${event.event_type}-${index}`,
        event,
        startMs,
        durationMs,
        importance: eventImportance(event),
        ballFrom: index === 0 ? HOME_BALL : zonePoint(orderedEvents[index - 1].zone, orderedEvents[index - 1].side),
        ballTo,
      };
      startMs += durationMs;
      return clip;
    });
}

export function filterPresentationTimeline(
  clips: PresentationClip[],
  mode: HighlightMode,
): PresentationClip[] {
  if (mode === "full") return clips;
  if (mode === "extended") return clips.filter((clip) => clip.importance !== "normal");
  return clips.filter((clip) => clip.importance === "key");
}

export function presentationFrame(
  input: MatchPresentationInput,
  elapsedMs: number,
  mode: HighlightMode = "full",
): MatchPresentationFrame {
  const secondHalf = input.current_minute > 45;
  const players = [
    ...resolveTeamPositions("Home", input.home_team.formation, input.home_team.players, input.sent_off, secondHalf),
    ...resolveTeamPositions("Away", input.away_team.formation, input.away_team.players, input.sent_off, secondHalf),
  ];
  const clips = filterPresentationTimeline(compilePresentationTimeline(input.events), mode);
  const totalDuration = clips.reduce((total, clip) => total + clip.durationMs, 0);
  if (clips.length === 0 || totalDuration === 0) {
    return { players, ball: zonePoint(input.ball_zone), activeClip: null };
  }
  const playbackMs = elapsedMs % totalDuration;
  const activeClip = clips.find((clip) => playbackMs >= clip.startMs && playbackMs < clip.startMs + clip.durationMs) ?? clips.at(-1) ?? null;
  if (!activeClip) return { players, ball: zonePoint(input.ball_zone), activeClip: null };
  const progress = Math.max(0, Math.min(1, (playbackMs - activeClip.startMs) / activeClip.durationMs));
  const eased = progress * progress * (3 - 2 * progress);
  return {
    players,
    ball: {
      x: activeClip.ballFrom.x + (activeClip.ballTo.x - activeClip.ballFrom.x) * eased,
      y: activeClip.ballFrom.y + (activeClip.ballTo.y - activeClip.ballFrom.y) * eased,
    },
    activeClip,
  };
}
