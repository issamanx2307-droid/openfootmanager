import { useEffect, useRef } from "react";
import type { MatchEvent, MatchSnapshot } from "../match/types";
import { playerVisualStatus, presentationFrame, roleAbbreviation } from "./presentation";
import { MATCH_2D_CONFIG, rendererDebugEnabled, type CameraMode } from "./config";
import type { HighlightMode, MatchPresentationFrame, PitchPoint, PresentationPlayer } from "./types";

type Match2DRendererProps = {
  snapshot: MatchSnapshot;
  homeColor: string;
  awayColor: string;
  speed: 1 | 2 | 4;
  highlightMode: HighlightMode;
  reducedMotion?: boolean;
  showNames?: boolean;
  showRoleLabels?: boolean;
  /** A viewer-selected server event. This changes presentation playback only. */
  replayEvent?: MatchEvent | null;
  cameraMode?: CameraMode;
  zoom?: number;
  onRendererUnavailable?: () => void;
  playerNumbers?: Readonly<Record<string, number>>;
  ariaLabel: string;
  /** Localized label supplied by the owning match screen. */
  eventLabel: (event: MatchEvent) => string;
};

type PitchViewport = { x: number; y: number; width: number; height: number };

/** Fits the canonical football-pitch ratio inside any Match Centre viewport. */
export function fitPitchViewport(width: number, height: number): PitchViewport {
  const aspectRatio = MATCH_2D_CONFIG.pitch.aspectRatio;
  if (width / height > aspectRatio) {
    const pitchWidth = height * aspectRatio;
    return { x: (width - pitchWidth) / 2, y: 0, width: pitchWidth, height };
  }
  const pitchHeight = width / aspectRatio;
  return { x: 0, y: (height - pitchHeight) / 2, width, height: pitchHeight };
}

function toCanvas(point: PitchPoint, width: number, height: number): [number, number] {
  return [point.x * width, point.y * height];
}

function drawPitch(context: CanvasRenderingContext2D, width: number, height: number): void {
  context.fillStyle = "#146c43";
  context.fillRect(0, 0, width, height);
  for (let index = 0; index < 10; index += 1) {
    context.fillStyle = index % 2 === 0 ? "rgba(255,255,255,0.035)" : "rgba(0,0,0,0.025)";
    context.fillRect((width / 10) * index, 0, width / 10, height);
  }
  const line = "rgba(255,255,255,0.82)";
  const marginX = width * 0.04;
  const marginY = height * 0.06;
  const pitchWidth = width - marginX * 2;
  const pitchHeight = height - marginY * 2;
  context.strokeStyle = line;
  context.lineWidth = Math.max(1, width * 0.002);
  context.strokeRect(marginX, marginY, pitchWidth, pitchHeight);
  context.beginPath();
  context.moveTo(width / 2, marginY);
  context.lineTo(width / 2, height - marginY);
  context.arc(width / 2, height / 2, height * 0.13, 0, Math.PI * 2);
  context.stroke();
  context.fillStyle = line;
  for (const direction of [1, -1]) {
    const spotX = direction === 1 ? marginX + pitchWidth * 0.11 : width - marginX - pitchWidth * 0.11;
    context.beginPath();
    context.arc(spotX, height / 2, Math.max(1.5, width * 0.003), 0, Math.PI * 2);
    context.fill();
    const goalDepth = pitchWidth * 0.025;
    context.strokeRect(direction === 1 ? marginX - goalDepth : width - marginX, height * 0.43, goalDepth, height * 0.14);
  }
  for (const direction of [1, -1]) {
    const left = direction === 1 ? marginX : width - marginX - pitchWidth * 0.16;
    context.strokeRect(left, height * 0.24, pitchWidth * 0.16, height * 0.52);
    const sixLeft = direction === 1 ? marginX : width - marginX - pitchWidth * 0.07;
    context.strokeRect(sixLeft, height * 0.36, pitchWidth * 0.07, height * 0.28);
  }
  for (const [x, y, start] of [[marginX, marginY, 0], [width - marginX, marginY, Math.PI / 2], [width - marginX, height - marginY, Math.PI], [marginX, height - marginY, Math.PI * 1.5]] as const) {
    context.beginPath();
    context.arc(x, y, height * 0.035, start, start + Math.PI / 2);
    context.stroke();
  }
}

function drawPlayer(
  context: CanvasRenderingContext2D,
  presentationPlayer: PresentationPlayer,
  width: number,
  height: number,
  color: string,
  showNames: boolean,
  showRoleLabels: boolean,
  highlighted: boolean,
  markerLabel: string,
  yellowCards: number,
  injured: boolean,
): void {
  const [x, y] = toCanvas(presentationPlayer.point, width, height);
  const radius = Math.max(9, Math.min(width, height) * 0.026);
  context.save();
  context.fillStyle = presentationPlayer.goalkeeper ? "#f5b942" : color;
  context.strokeStyle = presentationPlayer.side === "Home" ? "#ffffff" : "#111827";
  context.lineWidth = 2;
  context.beginPath();
  if (presentationPlayer.goalkeeper) {
    context.moveTo(x, y - radius);
    context.lineTo(x + radius, y);
    context.lineTo(x, y + radius);
    context.lineTo(x - radius, y);
    context.closePath();
  } else if (presentationPlayer.side === "Away") {
    context.rect(x - radius, y - radius, radius * 2, radius * 2);
  } else {
    context.arc(x, y, radius, 0, Math.PI * 2);
  }
  context.fill();
  context.stroke();
  if (highlighted) {
    context.strokeStyle = "#facc15";
    context.lineWidth = 3;
    context.beginPath();
    context.arc(x, y, radius + 5, 0, Math.PI * 2);
    context.stroke();
  }
  context.fillStyle = "#ffffff";
  context.font = `bold ${Math.max(9, radius)}px Inter, sans-serif`;
  context.textAlign = "center";
  context.textBaseline = "middle";
  context.fillText(markerLabel, x, y + 0.5);
  if (yellowCards > 0) {
    context.fillStyle = "#facc15";
    context.strokeStyle = "#111827";
    context.lineWidth = 1;
    context.fillRect(x + radius * 0.55, y - radius * 1.15, radius * 0.52, radius * 0.72);
    context.strokeRect(x + radius * 0.55, y - radius * 1.15, radius * 0.52, radius * 0.72);
    if (yellowCards > 1) {
      context.fillStyle = "#ffffff";
      context.font = `bold ${Math.max(8, radius * 0.65)}px Inter, sans-serif`;
      context.fillText("2", x + radius * 0.81, y - radius * 0.79);
    }
  }
  if (injured) {
    context.strokeStyle = "#f97316";
    context.lineWidth = 3;
    context.beginPath();
    context.moveTo(x - radius * 0.35, y + radius * 1.15);
    context.lineTo(x + radius * 0.35, y + radius * 1.15);
    context.moveTo(x, y + radius * 0.8);
    context.lineTo(x, y + radius * 1.5);
    context.stroke();
  }
  if (showNames) {
    context.font = `${Math.max(9, radius * 0.75)}px Inter, sans-serif`;
    context.fillStyle = "#f8fafc";
    const names = presentationPlayer.player.name.split(" ");
    context.fillText(names[names.length - 1] ?? presentationPlayer.player.name, x, y - radius - 8);
  }
  if (showRoleLabels) {
    context.font = `bold ${Math.max(8, radius * 0.62)}px Inter, sans-serif`;
    context.fillStyle = "#f8fafc";
    context.fillText(roleAbbreviation(presentationPlayer.player.role), x, y + radius + 9);
  }
  context.restore();
}

/** Development-only shape and movement guides. They remain wholly visual. */
function drawDebugPitchGuides(
  context: CanvasRenderingContext2D,
  width: number,
  height: number,
  frame: MatchPresentationFrame,
): void {
  context.save();
  context.strokeStyle = "rgba(250, 204, 21, 0.28)";
  context.lineWidth = 1;
  for (let column = 1; column < MATCH_2D_CONFIG.debug.gridColumns; column += 1) {
    const x = (width / MATCH_2D_CONFIG.debug.gridColumns) * column;
    context.beginPath();
    context.moveTo(x, 0);
    context.lineTo(x, height);
    context.stroke();
  }
  for (let row = 1; row < MATCH_2D_CONFIG.debug.gridRows; row += 1) {
    const y = (height / MATCH_2D_CONFIG.debug.gridRows) * row;
    context.beginPath();
    context.moveTo(0, y);
    context.lineTo(width, y);
    context.stroke();
  }
  if (frame.activeClip) {
    const [fromX, fromY] = toCanvas(frame.activeClip.ballFrom, width, height);
    const [toX, toY] = toCanvas(frame.activeClip.ballTo, width, height);
    context.strokeStyle = "rgba(248, 250, 252, 0.85)";
    context.setLineDash([4, 3]);
    context.beginPath();
    context.moveTo(fromX, fromY);
    context.lineTo(toX, toY);
    context.stroke();
    context.setLineDash([]);
  }
  context.restore();
}

function drawDebugDiagnostics(
  context: CanvasRenderingContext2D,
  width: number,
  height: number,
  frame: MatchPresentationFrame,
  fps: number,
  frameMs: number,
  currentMinute: number,
): void {
  const inset = MATCH_2D_CONFIG.debug.panelInset;
  const lines = [
    `v${MATCH_2D_CONFIG.version} · ${fps.toFixed(0)} FPS · ${frameMs.toFixed(1)} ms`,
    `event ${frame.activeClip?.event.event_type ?? "static"} · clip ${frame.activeClip?.clipId ?? "none"}`,
    `match ${currentMinute}′ · ball ${frame.ball.x.toFixed(2)}, ${frame.ball.y.toFixed(2)} · entities ${frame.players.length + 1}`,
    `players ${frame.players.map((player) => player.id).join(", ") || "none"}`,
  ];
  context.save();
  context.fillStyle = "rgba(15, 23, 42, 0.88)";
  context.fillRect(inset, height - MATCH_2D_CONFIG.debug.panelHeight - inset, Math.min(MATCH_2D_CONFIG.debug.panelWidth, width - inset * 2), MATCH_2D_CONFIG.debug.panelHeight);
  context.fillStyle = "#f8fafc";
  context.font = "11px monospace";
  lines.forEach((line, index) => context.fillText(line, inset + 8, height - MATCH_2D_CONFIG.debug.panelHeight + 18 + index * 20));
  context.restore();
}

export default function Match2DRenderer({
  snapshot,
  homeColor,
  awayColor,
  speed,
  highlightMode,
  reducedMotion = false,
  showNames = false,
  showRoleLabels = false,
  replayEvent = null,
  cameraMode = "full",
  zoom = MATCH_2D_CONFIG.camera.minZoom,
  onRendererUnavailable,
  playerNumbers,
  ariaLabel,
  eventLabel,
}: Match2DRendererProps) {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const startedAt = useRef<number | null>(null);
  const previousFrameAt = useRef<number | null>(null);
  const fps = useRef(0);
  const frameMs = useRef(0);

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return undefined;
    const context = canvas.getContext("2d");
    if (!context) {
      onRendererUnavailable?.();
      return undefined;
    }
    let frameId = 0;
    startedAt.current = null;
    const resize = () => {
      const rect = canvas.getBoundingClientRect();
      const ratio = window.devicePixelRatio || 1;
      canvas.width = Math.max(1, Math.round(rect.width * ratio));
      canvas.height = Math.max(1, Math.round(rect.height * ratio));
      context.setTransform(ratio, 0, 0, ratio, 0, 0);
    };
    const observer = new ResizeObserver(resize);
    observer.observe(canvas);
    resize();
    const render = (now: number) => {
      if (startedAt.current === null) startedAt.current = now;
      if (previousFrameAt.current !== null) {
        frameMs.current = Math.max(1, now - previousFrameAt.current);
        fps.current = 1000 / frameMs.current;
      }
      previousFrameAt.current = now;
      const rect = canvas.getBoundingClientRect();
      const pitchViewport = fitPitchViewport(rect.width, rect.height);
      const elapsed = reducedMotion ? 0 : (now - startedAt.current) * speed;
      const frame = presentationFrame(
        replayEvent ? { ...snapshot, events: [replayEvent] } : snapshot,
        elapsed,
        replayEvent ? "full" : highlightMode,
      );
      context.clearRect(0, 0, rect.width, rect.height);
      const boundedZoom = Math.max(MATCH_2D_CONFIG.camera.minZoom, Math.min(MATCH_2D_CONFIG.camera.maxZoom, zoom));
      const activeZoom = cameraMode === "follow-ball" ? Math.max(boundedZoom, MATCH_2D_CONFIG.camera.dynamicZoom) : boundedZoom;
      const [focusX, focusY] = cameraMode === "follow-ball"
        ? toCanvas(frame.ball, pitchViewport.width, pitchViewport.height)
        : [pitchViewport.width / 2, pitchViewport.height / 2];
      context.save();
      context.beginPath();
      context.rect(pitchViewport.x, pitchViewport.y, pitchViewport.width, pitchViewport.height);
      context.clip();
      context.translate(pitchViewport.x + pitchViewport.width / 2, pitchViewport.y + pitchViewport.height / 2);
      context.scale(activeZoom, activeZoom);
      context.translate(-focusX, -focusY);
      drawPitch(context, pitchViewport.width, pitchViewport.height);
      let homeMarkerNumber = 0;
      let awayMarkerNumber = 0;
      frame.players.forEach((player) => {
        const fallbackNumber = player.side === "Home" ? ++homeMarkerNumber : ++awayMarkerNumber;
        const status = playerVisualStatus(
          player.id,
          player.side === "Home" ? snapshot.home_yellows : snapshot.away_yellows,
          snapshot.events,
        );
        drawPlayer(
          context,
          player,
          pitchViewport.width,
          pitchViewport.height,
          player.side === "Home" ? homeColor : awayColor,
          showNames,
          showRoleLabels,
          player.id === frame.actorPlayerId || player.id === frame.targetPlayerId,
          String(playerNumbers?.[player.id] ?? fallbackNumber),
          status.yellowCards,
          status.injured,
        );
      });
      const [ballX, ballY] = toCanvas(frame.ball, pitchViewport.width, pitchViewport.height);
      if (frame.activeClip && frame.ballTrajectory !== "ground") {
        const [fromX, fromY] = toCanvas(frame.activeClip.ballFrom, pitchViewport.width, pitchViewport.height);
        context.save();
        context.strokeStyle = frame.ballTrajectory === "shot" ? "rgba(250,204,21,0.8)" : "rgba(255,255,255,0.55)";
        context.lineWidth = 2;
        context.setLineDash([5, 5]);
        context.beginPath();
        context.moveTo(fromX, fromY);
        context.quadraticCurveTo((fromX + ballX) / 2, Math.min(fromY, ballY) - pitchViewport.height * 0.12, ballX, ballY);
        context.stroke();
        context.restore();
      }
      context.fillStyle = "#ffffff";
      context.strokeStyle = "#111827";
      context.lineWidth = 1.5;
      context.beginPath();
      context.arc(ballX, ballY, Math.max(4, Math.min(pitchViewport.width, pitchViewport.height) * 0.011), 0, Math.PI * 2);
      context.fill();
      context.stroke();
      if (rendererDebugEnabled()) {
        drawDebugPitchGuides(context, pitchViewport.width, pitchViewport.height, frame);
      }
      context.restore();
      if (frame.activeClip && ["Goal", "PenaltyGoal", "RedCard", "Substitution"].includes(frame.activeClip.event.event_type)) {
        const label = eventLabel(frame.activeClip.event);
        context.fillStyle = "rgba(15, 23, 42, 0.7)";
        context.font = "bold 13px Inter, sans-serif";
        context.fillRect(12, 12, Math.max(150, context.measureText(label).width + 24), 28);
        context.fillStyle = "#f8fafc";
        context.fillText(label, 20, 31);
      }
      if (rendererDebugEnabled()) {
        drawDebugDiagnostics(context, rect.width, rect.height, frame, fps.current, frameMs.current, snapshot.current_minute);
      }
      frameId = requestAnimationFrame(render);
    };
    frameId = requestAnimationFrame(render);
    return () => {
      cancelAnimationFrame(frameId);
      observer.disconnect();
    };
  }, [awayColor, cameraMode, eventLabel, highlightMode, homeColor, onRendererUnavailable, playerNumbers, reducedMotion, replayEvent, showNames, showRoleLabels, snapshot, speed, zoom]);

  return <canvas ref={canvasRef} aria-label={ariaLabel} className="block h-full min-h-80 w-full bg-slate-950" />;
}
