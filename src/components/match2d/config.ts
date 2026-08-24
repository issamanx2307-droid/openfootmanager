/** Versioned, presentation-only tuning defaults for the 2D Match Centre. */
export const MATCH_2D_CONFIG = {
  version: "albion-2d-1",
  camera: { minZoom: 1, maxZoom: 2.2, dynamicZoom: 1.3 },
} as const;

export type CameraMode = "full" | "follow-ball";

export function rendererDebugEnabled(): boolean {
  return import.meta.env.DEV && typeof window !== "undefined"
    && new URLSearchParams(window.location.search).get("albion2dDebug") === "1";
}
