import { useEffect, useRef } from "react";
import type { MatchEvent, MatchSnapshot } from "../match/types";
import { presentationFrame } from "./presentation";
import { MATCH_2D_CONFIG, type CameraMode } from "./config";
import type { HighlightMode, PitchPoint, PresentationPlayer } from "./types";

type Match2DRendererProps = {
  snapshot: MatchSnapshot;
  homeColor: string;
  awayColor: string;
  speed: 1 | 2 | 4;
  highlightMode: HighlightMode;
  reducedMotion?: boolean;
  showNames?: boolean;
  /** A viewer-selected server event. This changes presentation playback only. */
  replayEvent?: MatchEvent | null;
  cameraMode?: CameraMode;
  zoom?: number;
  onRendererUnavailable?: () => void;
};

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
  for (const direction of [1, -1]) {
    const left = direction === 1 ? marginX : width - marginX - pitchWidth * 0.16;
    context.strokeRect(left, height * 0.24, pitchWidth * 0.16, height * 0.52);
    const sixLeft = direction === 1 ? marginX : width - marginX - pitchWidth * 0.07;
    context.strokeRect(sixLeft, height * 0.36, pitchWidth * 0.07, height * 0.28);
  }
}

function drawPlayer(
  context: CanvasRenderingContext2D,
  presentationPlayer: PresentationPlayer,
  width: number,
  height: number,
  color: string,
  showNames: boolean,
  highlighted: boolean,
): void {
  const [x, y] = toCanvas(presentationPlayer.point, width, height);
  const radius = Math.max(9, Math.min(width, height) * 0.026);
  context.save();
  context.fillStyle = presentationPlayer.goalkeeper ? "#f5b942" : color;
  context.strokeStyle = presentationPlayer.side === "Home" ? "#ffffff" : "#111827";
  context.lineWidth = 2;
  context.beginPath();
  context.arc(x, y, radius, 0, Math.PI * 2);
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
  context.fillText(String(presentationPlayer.player.id.slice(-2)), x, y + 0.5);
  if (showNames) {
    context.font = `${Math.max(9, radius * 0.75)}px Inter, sans-serif`;
    context.fillStyle = "#f8fafc";
    const names = presentationPlayer.player.name.split(" ");
    context.fillText(names[names.length - 1] ?? presentationPlayer.player.name, x, y - radius - 8);
  }
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
  replayEvent = null,
  cameraMode = "full",
  zoom = MATCH_2D_CONFIG.camera.minZoom,
  onRendererUnavailable,
}: Match2DRendererProps) {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const startedAt = useRef<number | null>(null);

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
      const rect = canvas.getBoundingClientRect();
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
        ? toCanvas(frame.ball, rect.width, rect.height)
        : [rect.width / 2, rect.height / 2];
      context.save();
      context.translate(rect.width / 2, rect.height / 2);
      context.scale(activeZoom, activeZoom);
      context.translate(-focusX, -focusY);
      drawPitch(context, rect.width, rect.height);
      frame.players.forEach((player) => {
        drawPlayer(
          context,
          player,
          rect.width,
          rect.height,
          player.side === "Home" ? homeColor : awayColor,
          showNames,
          player.id === frame.actorPlayerId || player.id === frame.targetPlayerId,
        );
      });
      const [ballX, ballY] = toCanvas(frame.ball, rect.width, rect.height);
      if (frame.activeClip && frame.ballTrajectory !== "ground") {
        const [fromX, fromY] = toCanvas(frame.activeClip.ballFrom, rect.width, rect.height);
        context.save();
        context.strokeStyle = frame.ballTrajectory === "shot" ? "rgba(250,204,21,0.8)" : "rgba(255,255,255,0.55)";
        context.lineWidth = 2;
        context.setLineDash([5, 5]);
        context.beginPath();
        context.moveTo(fromX, fromY);
        context.quadraticCurveTo((fromX + ballX) / 2, Math.min(fromY, ballY) - rect.height * 0.12, ballX, ballY);
        context.stroke();
        context.restore();
      }
      context.fillStyle = "#ffffff";
      context.strokeStyle = "#111827";
      context.lineWidth = 1.5;
      context.beginPath();
      context.arc(ballX, ballY, Math.max(4, Math.min(rect.width, rect.height) * 0.011), 0, Math.PI * 2);
      context.fill();
      context.stroke();
      context.restore();
      if (frame.activeClip && ["Goal", "PenaltyGoal", "RedCard", "Substitution"].includes(frame.activeClip.event.event_type)) {
        context.fillStyle = "rgba(15, 23, 42, 0.7)";
        context.fillRect(12, 12, 150, 28);
        context.fillStyle = "#f8fafc";
        context.font = "bold 13px Inter, sans-serif";
        context.fillText(`${frame.activeClip.event.minute}' ${frame.activeClip.event.event_type}`, 20, 31);
      }
      frameId = requestAnimationFrame(render);
    };
    frameId = requestAnimationFrame(render);
    return () => {
      cancelAnimationFrame(frameId);
      observer.disconnect();
    };
  }, [awayColor, cameraMode, highlightMode, homeColor, onRendererUnavailable, reducedMotion, replayEvent, showNames, snapshot, speed, zoom]);

  return <canvas ref={canvasRef} aria-label="2D live match pitch" className="block h-full min-h-80 w-full bg-emerald-800" />;
}
