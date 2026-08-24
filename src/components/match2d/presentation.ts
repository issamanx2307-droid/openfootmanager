import { buildFormationSlots } from "../match/FormationPitch";
import type { EnginePlayerData, MatchEvent } from "../match/types";
import type {
  BallPresentationState,
  HighlightMode,
  MatchPresentationFrame,
  MatchPresentationInput,
  PitchPoint,
  PresentationClip,
  PresentationPlayer,
  PresentationPlayerStatus,
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

const KNOWN_EVENT_TYPES = new Set([
  "KickOff", "HalfTime", "SecondHalfStart", "FullTime", "PassCompleted", "PassIntercepted",
  "Dribble", "DribbleTackled", "Cross", "ShotOnTarget", "ShotOffTarget", "ShotBlocked",
  "ShotSaved", "Goal", "PenaltyAwarded", "PenaltyGoal", "PenaltyMiss", "ShootoutGoal",
  "ShootoutMiss", "Tackle", "Interception", "Clearance", "Foul", "YellowCard", "RedCard",
  "SecondYellow", "Corner", "FreeKick", "Injury", "GoalKick", "Substitution",
]);
const reportedUnknownEventTypes = new Set<string>();

const ROLE_ABBREVIATIONS: Readonly<Record<string, string>> = {
  Goalkeeper: "GK",
  SweeperKeeper: "SK",
  CentreBack: "CB",
  FullBack: "FB",
  WingBack: "WB",
  DefensiveMidfielder: "DM",
  CentralMidfielder: "CM",
  AttackingMidfielder: "AM",
  Winger: "WG",
  InsideForward: "IF",
  TargetForward: "TF",
  CompleteForward: "CF",
};

export function roleAbbreviation(role: string): string {
  return ROLE_ABBREVIATIONS[role] ?? role.replace(/[^A-Za-z]/g, "").slice(0, 3).toUpperCase();
}

function clamp(value: number): number {
  return Math.max(PITCH_MARGIN, Math.min(1 - PITCH_MARGIN, value));
}

function mirror(point: PitchPoint): PitchPoint {
  return { x: 1 - point.x, y: 1 - point.y };
}

/**
 * A deliberately small visual-only interpretation of the authoritative team
 * approach. Coordinates are adjusted before side/half mirroring so home and
 * away keep symmetric shapes; outcomes remain entirely engine-owned.
 */
function applyPlayStyleShape(point: PitchPoint, playStyle: string): PitchPoint {
  const attackingOffset = playStyle === "Attacking" || playStyle === "HighPress"
    ? 0.08
    : playStyle === "Defensive" ? -0.08 : 0;
  const widthFactor = playStyle === "Possession" ? 1.16 : playStyle === "HighPress" ? 0.9 : 1;
  return {
    x: clamp(point.x + attackingOffset),
    y: clamp(0.5 + (point.y - 0.5) * widthFactor),
  };
}

function eventImportance(event: MatchEvent): PresentationClip["importance"] {
  if (["Goal", "PenaltyGoal", "PenaltyMiss", "ShootoutGoal", "ShootoutMiss", "RedCard", "SecondYellow", "ShotOnTarget", "ShotSaved"].includes(event.event_type)) return "key";
  if (["ShotOffTarget", "ShotBlocked", "Corner", "FreeKick", "Substitution", "YellowCard", "Injury", "Cross"].includes(event.event_type)) return "extended";
  return "normal";
}

function eventDuration(event: MatchEvent): number {
  if (["Goal", "PenaltyGoal", "PenaltyMiss", "ShootoutGoal", "ShootoutMiss", "ShotOnTarget", "ShotOffTarget", "ShotBlocked", "ShotSaved"].includes(event.event_type)) return 900;
  if (["PassCompleted", "PassIntercepted", "Cross", "Corner", "FreeKick"].includes(event.event_type)) return 720;
  if (["Tackle", "Interception", "DribbleTackled", "Foul"].includes(event.event_type)) return 420;
  return 560;
}

function trajectoryForEvent(event: MatchEvent): MatchPresentationFrame["ballTrajectory"] {
  if (["Cross", "Corner", "FreeKick"].includes(event.event_type)) return "arc";
  if (["ShotOnTarget", "ShotOffTarget", "ShotBlocked", "ShotSaved", "Goal", "PenaltyGoal", "PenaltyMiss", "ShootoutGoal", "ShootoutMiss"].includes(event.event_type)) return "shot";
  return "ground";
}

/** Maps existing engine event facts into display-only ball treatment. */
export function ballStateForEvent(event: MatchEvent): BallPresentationState {
  if (["PenaltyAwarded", "PenaltyGoal", "PenaltyMiss", "ShootoutGoal", "ShootoutMiss"].includes(event.event_type)) return "penalty";
  if (event.event_type === "ShotSaved") return "save";
  if (["ShotOffTarget", "HalfTime", "FullTime"].includes(event.event_type)) return "out-of-play";
  if (["Cross", "Corner"].includes(event.event_type)) return "crossing";
  if (["ShotOnTarget", "ShotOffTarget", "ShotBlocked", "Goal"].includes(event.event_type)) return "shot";
  if (["GoalKick", "FreeKick", "KickOff", "SecondHalfStart"].includes(event.event_type)) return "restart";
  if (["Tackle", "Interception", "DribbleTackled", "PassIntercepted", "Clearance", "Foul"].includes(event.event_type)) return "loose";
  if (event.event_type === "PassCompleted") return "passing";
  return "controlled";
}

function moveTowards(from: PitchPoint, to: PitchPoint, amount: number): PitchPoint {
  return {
    x: from.x + (to.x - from.x) * amount,
    y: from.y + (to.y - from.y) * amount,
  };
}

/**
 * A sparse event stream has no per-player coordinates.  This small,
 * deterministic adjustment keeps the original formation legible while making
 * the possession side offer support and the defending side narrow toward the
 * ball side.  It deliberately stops well short of the ball so a whole team
 * never collapses into one marker cluster.
 */
function applyContextualTeamShape(
  players: PresentationPlayer[],
  clip: PresentationClip,
  ball: PitchPoint,
  progress: number,
): PresentationPlayer[] {
  return players.map((player) => {
    if (player.goalkeeper || player.id === clip.event.player_id || player.id === clip.event.secondary_player_id) {
      return player;
    }
    const hasPossession = player.side === clip.event.side;
    const roleFactor = player.player.position === "Forward" ? 0.85
      : player.player.position === "Midfielder" ? 1 : 0.75;
    // Defenders compress a little more laterally. Possession support remains
    // shallower, preserving lanes for a pass rather than chasing the carrier.
    const response = (hasPossession ? 0.1 : 0.16) * roleFactor * progress;
    const target = {
      x: player.point.x + (ball.x - player.point.x) * (hasPossession ? 0.42 : 0.22),
      y: ball.y,
    };
    return { ...player, point: moveTowards(player.point, target, response) };
  });
}

/**
 * Cards and injury events are already authoritative snapshot facts.  The
 * renderer only converts them into marker badges; it never changes who is
 * available or on the pitch.
 */
export function playerVisualStatus(
  playerId: string,
  yellowCards: Readonly<Record<string, number>>,
  events: readonly MatchEvent[],
): PresentationPlayerStatus {
  return {
    yellowCards: yellowCards[playerId] ?? 0,
    injured: events.some((event) => event.event_type === "Injury" && event.player_id === playerId),
  };
}

/**
 * Event payloads identify participants but not per-tick coordinates. These
 * small, deterministic offsets make those semantic actions legible without
 * inventing an additional football simulation.
 */
function applySemanticPlayerMovement(
  players: PresentationPlayer[],
  clip: PresentationClip,
  ball: PitchPoint,
  progress: number,
): PresentationPlayer[] {
  const eventType = clip.event.event_type;
  // The authoritative snapshot already has the incoming player in the XI and
  // the outgoing player on the bench.  Let the incoming player walk in from
  // the nearest touchline, rather than inventing a second on-pitch player or
  // modifying the recorded substitution.
  if (eventType === "Substitution") {
    return players.map((player) => {
      if (player.id !== clip.event.player_id) return player;
      const touchline = {
        x: player.point.x,
        y: player.side === "Home" ? 0.97 : 0.03,
      };
      return { ...player, point: moveTowards(touchline, player.point, progress) };
    });
  }
  const actorMovement = eventType === "Dribble" ? 0.8
    : ["Tackle", "Interception", "DribbleTackled", "PassIntercepted"].includes(eventType) ? 0.55
      : ["Cross", "PassCompleted", "Corner", "FreeKick"].includes(eventType) ? 0.25 : 0.12;
  const targetMovement = ["PassCompleted", "Cross", "Corner", "FreeKick"].includes(eventType) ? 0.38
    : ["Tackle", "Interception", "DribbleTackled", "PassIntercepted"].includes(eventType) ? 0.25 : 0.08;
  const goalkeeperMovement = eventType === "ShotSaved" ? 0.7
    : ["ShotOnTarget", "Goal", "PenaltyGoal"].includes(eventType) ? 0.35 : 0;
  return players.map((player) => {
    if (player.id === clip.event.player_id) {
      return { ...player, point: moveTowards(player.point, ball, actorMovement * progress) };
    }
    if (player.id === clip.event.secondary_player_id) {
      return { ...player, point: moveTowards(player.point, clip.ballTo, targetMovement * progress) };
    }
    // The engine's save payload names the shooter, not the goalkeeper. Use
    // the existing defensive goalkeeper marker as the visual participant.
    if (goalkeeperMovement > 0 && player.goalkeeper && player.side !== clip.event.side) {
      return { ...player, point: moveTowards(player.point, ball, goalkeeperMovement * progress) };
    }
    return player;
  });
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
  playStyle = "Balanced",
): PresentationPlayer[] {
  const slots = buildFormationSlots(formation, players, sentOff);
  return slots.map(({ player, x, y }) => {
    const homePoint = { x: clamp(1 - y / 100), y: clamp(x / 100) };
    const shapedHomePoint = applyPlayStyleShape(homePoint, playStyle);
    const sidePoint = side === "Home" ? shapedHomePoint : mirror(shapedHomePoint);
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
      if (!KNOWN_EVENT_TYPES.has(event.event_type) && !reportedUnknownEventTypes.has(event.event_type)) {
        reportedUnknownEventTypes.add(event.event_type);
        console.warn("[albion-2d] unknown semantic event; using generic movement", {
          eventType: event.event_type,
          minute: event.minute,
          side: event.side,
          zone: event.zone,
        });
      }
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
    ...resolveTeamPositions("Home", input.home_team.formation, input.home_team.players, input.sent_off, secondHalf, input.home_team.play_style),
    ...resolveTeamPositions("Away", input.away_team.formation, input.away_team.players, input.sent_off, secondHalf, input.away_team.play_style),
  ];
  const clips = filterPresentationTimeline(compilePresentationTimeline(input.events), mode);
  const totalDuration = clips.reduce((total, clip) => total + clip.durationMs, 0);
  if (clips.length === 0 || totalDuration === 0) {
    return {
      players,
      ball: zonePoint(input.ball_zone),
      activeClip: null,
      ballTrajectory: "ground",
      ballState: "controlled",
      actorPlayerId: null,
      targetPlayerId: null,
    };
  }
  const playbackMs = elapsedMs % totalDuration;
  const activeClip = clips.find((clip) => playbackMs >= clip.startMs && playbackMs < clip.startMs + clip.durationMs) ?? clips[clips.length - 1] ?? null;
  if (!activeClip) {
    return {
      players,
      ball: zonePoint(input.ball_zone),
      activeClip: null,
      ballTrajectory: "ground",
      ballState: "controlled",
      actorPlayerId: null,
      targetPlayerId: null,
    };
  }
  const progress = Math.max(0, Math.min(1, (playbackMs - activeClip.startMs) / activeClip.durationMs));
  const eased = progress * progress * (3 - 2 * progress);
  const ball = {
      x: activeClip.ballFrom.x + (activeClip.ballTo.x - activeClip.ballFrom.x) * eased,
      y: activeClip.ballFrom.y + (activeClip.ballTo.y - activeClip.ballFrom.y) * eased,
    };
  const shapedPlayers = applyContextualTeamShape(players, activeClip, ball, eased);
  const semanticPlayers = applySemanticPlayerMovement(shapedPlayers, activeClip, ball, eased);
  // A dribble is controlled possession, not a second independent ball path.
  // Keep the rendered ball close to the authoritative actor marker while the
  // underlying zone-to-zone interpolation remains available for every other
  // event type.
  const actor = activeClip.event.event_type === "Dribble"
    ? semanticPlayers.find((player) => player.id === activeClip.event.player_id)
    : undefined;
  const presentationBall = actor ? moveTowards(actor.point, ball, 0.15) : ball;
  return {
    players: semanticPlayers,
    ball: presentationBall,
    activeClip,
    ballTrajectory: trajectoryForEvent(activeClip.event),
    ballState: ballStateForEvent(activeClip.event),
    actorPlayerId: activeClip.event.player_id,
    targetPlayerId: activeClip.event.secondary_player_id,
  };
}
