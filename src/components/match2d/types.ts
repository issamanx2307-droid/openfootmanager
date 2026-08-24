import type { EnginePlayerData, MatchEvent, MatchSnapshot } from "../match/types";

export const PRESENTATION_VERSION = "albion-2d-1";

export type PitchPoint = { x: number; y: number };
export type PresentationSide = "Home" | "Away";
export type HighlightMode = "key" | "extended" | "full";
export type BallPresentationState = "controlled" | "passing" | "crossing" | "shot" | "loose" | "save" | "out-of-play" | "restart" | "penalty";

export type PresentationPlayer = {
  id: string;
  side: PresentationSide;
  player: EnginePlayerData;
  point: PitchPoint;
  goalkeeper: boolean;
  sentOff: boolean;
};

/** Visual-only status badges derived from the authoritative snapshot. */
export type PresentationPlayerStatus = {
  yellowCards: number;
  injured: boolean;
};

export type PresentationClip = {
  clipId: string;
  event: MatchEvent;
  startMs: number;
  durationMs: number;
  importance: "normal" | "extended" | "key";
  ballFrom: PitchPoint;
  ballTo: PitchPoint;
};

export type MatchPresentationFrame = {
  players: PresentationPlayer[];
  ball: PitchPoint;
  activeClip: PresentationClip | null;
  ballTrajectory: "ground" | "arc" | "shot";
  ballState: BallPresentationState;
  actorPlayerId: string | null;
  targetPlayerId: string | null;
};

export type MatchPresentationInput = Pick<
  MatchSnapshot,
  | "home_team"
  | "away_team"
  | "sent_off"
  | "ball_zone"
  | "current_minute"
  | "events"
>;
