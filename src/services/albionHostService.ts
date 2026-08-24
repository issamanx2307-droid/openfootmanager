import { invoke } from "@tauri-apps/api/core";

export type AlbionHostInfo = {
  server_url: string;
};

/** Starts/stops the bundled private server; game commands still use WebSocket. */
export function startAlbionHost(joinSecret: string): Promise<AlbionHostInfo> {
  return invoke<AlbionHostInfo>("start_albion_host", { joinSecret });
}

export function stopAlbionHost(): Promise<void> {
  return invoke<void>("stop_albion_host");
}
