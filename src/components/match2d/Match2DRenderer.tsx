import { useEffect, useRef } from "react";
import type { MatchSnapshot } from "../match/types";
import { presentationFrame } from "./presentation";
import type { HighlightMode, PitchPoint, PresentationPlayer } from "./types";

type Match2DRendererProps = {
  snapshot: MatchSnapshot;
  homeColor: string;
  awayColor: string;
  speed: 1 | 2 | 4;
  highlightMode: HighlightMode;
  reducedMotion?: boolean;
  showNames?: boolean;
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
  context.fillStyle = "#ffffff";
  context.font = `bold ${Math.max(9, radius)}px Inter, sans-serif`;
  context.textAlign = "center";
  context.textBaseline = "middle";
  context.fillText(String(presentationPlayer.player.id.slice(-2)), x, y + 0.5);
  if (showNames) {
    context.font = `${Math.max(9, radius * 0.75)}px Inter, sans-serif`;
    context.fillStyle = "#f8fafc";
    context.fillText(presentationPlayer.player.name.split(" ").at(-1) ?? presentationPlayer.player.name, x, y - radius - 8);
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
}: Match2DRendererProps) {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const startedAt = useRef<number | null>(null);

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return undefined;
    const context = canvas.getContext("2d");
    if (!context) return undefined;
    let frameId = 0;
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
      const frame = presentationFrame(snapshot, elapsed, highlightMode);
      context.clearRect(0, 0, rect.width, rect.height);
      drawPitch(context, rect.width, rect.height);
      frame.players.forEach((player) => drawPlayer(context, player, rect.width, rect.height, player.side === "Home" ? homeColor : awayColor, showNames));
      const [ballX, ballY] = toCanvas(frame.ball, rect.width, rect.height);
      context.fillStyle = "#ffffff";
      context.strokeStyle = "#111827";
      context.lineWidth = 1.5;
      context.beginPath();
      context.arc(ballX, ballY, Math.max(4, Math.min(rect.width, rect.height) * 0.011), 0, Math.PI * 2);
      context.fill();
      context.stroke();
      frameId = requestAnimationFrame(render);
    };
    frameId = requestAnimationFrame(render);
    return () => {
      cancelAnimationFrame(frameId);
      observer.disconnect();
    };
  }, [awayColor, highlightMode, homeColor, reducedMotion, showNames, snapshot, speed]);

  return <canvas ref={canvasRef} aria-label="2D live match pitch" className="block h-full min-h-80 w-full bg-emerald-800" />;
}
