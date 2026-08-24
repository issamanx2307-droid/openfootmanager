/** Browser transport for a Project Albion authoritative career.
 *
 * This deliberately owns no football state. Pages consume its small view
 * cache, while mutations always travel to `albion_server` as envelopes.
 */

export type AlbionVersionSet = {
  app_version: string;
  protocol_version: number;
  save_schema_version: number;
  snapshot_schema_version: number;
  match_engine_version: string;
  rating_model_version: string;
  ruleset?: { ruleset_id: string; ruleset_version: number } | null;
};

export type AlbionJoinRequest = {
  join_secret: string;
  manager_id: string;
  club_id: string;
  client_versions: AlbionVersionSet;
};

export type AlbionSession = {
  career_id: string;
  manager_id: string;
  reconnect_token: string;
  current_revision: number;
  slot: "host" | "guest";
};

export type AlbionStoredSession = AlbionSession & { server_url: string };

const SESSION_STORAGE_KEY = "albion.active-session.v1";

export function saveAlbionSession(session: AlbionStoredSession): void {
  window.localStorage.setItem(SESSION_STORAGE_KEY, JSON.stringify(session));
}

export function loadAlbionSession(): AlbionStoredSession | null {
  try {
    const stored = window.localStorage.getItem(SESSION_STORAGE_KEY);
    return stored ? JSON.parse(stored) as AlbionStoredSession : null;
  } catch {
    window.localStorage.removeItem(SESSION_STORAGE_KEY);
    return null;
  }
}

export function clearAlbionSession(): void {
  window.localStorage.removeItem(SESSION_STORAGE_KEY);
}

export type AlbionServerEvent = {
  type: string;
  body?: Record<string, unknown>;
};

export type AlbionViewCache = {
  revision: number;
  views: ReadonlyMap<string, unknown>;
};

export function applyAlbionServerEvent(
  cache: AlbionViewCache,
  event: AlbionServerEvent,
): AlbionViewCache {
  const body = event.body ?? {};
  if (event.type === "ViewSnapshot" && typeof body.view_name === "string") {
    const revision = typeof body.revision === "number" ? body.revision : cache.revision;
    const views = new Map(cache.views);
    views.set(body.view_name, body.payload);
    return { revision, views };
  }
  if (event.type === "CommandAck" && typeof body.applied_revision === "number") {
    return { ...cache, revision: body.applied_revision };
  }
  if (event.type === "CommandRejected" && typeof body.current_revision === "number") {
    return { ...cache, revision: body.current_revision };
  }
  if (event.type === "GameTimeChanged" && typeof body.revision === "number") {
    return { ...cache, revision: body.revision };
  }
  return cache;
}

function websocketUrl(serverUrl: string, reconnectToken: string): string {
  const url = new URL(serverUrl);
  url.protocol = url.protocol === "https:" ? "wss:" : "ws:";
  url.pathname = "/ws";
  url.searchParams.set("reconnect_token", reconnectToken);
  return url.toString();
}

export class AlbionServerClient {
  private socket: WebSocket | null = null;
  private cache: AlbionViewCache = { revision: 0, views: new Map() };

  get viewCache(): AlbionViewCache {
    return this.cache;
  }

  async version(serverUrl: string): Promise<AlbionVersionSet> {
    const response = await fetch(new URL("/version", serverUrl));
    if (!response.ok) throw new Error(await response.text());
    return response.json() as Promise<AlbionVersionSet>;
  }

  async join(serverUrl: string, request: AlbionJoinRequest): Promise<AlbionSession> {
    return this.post<AlbionSession>(serverUrl, "/api/v1/session/join", request);
  }

  async reconnect(
    serverUrl: string,
    reconnect_token: string,
    client_versions: AlbionVersionSet,
  ): Promise<AlbionSession> {
    return this.post<AlbionSession>(serverUrl, "/api/v1/session/reconnect", {
      reconnect_token,
      client_versions,
    });
  }

  connect(
    serverUrl: string,
    session: AlbionSession,
    onEvent: (event: AlbionServerEvent, cache: AlbionViewCache) => void,
  ): void {
    this.disconnect();
    this.cache = { revision: session.current_revision, views: new Map() };
    const socket = new WebSocket(websocketUrl(serverUrl, session.reconnect_token));
    socket.addEventListener("message", (message) => {
      const event = JSON.parse(String(message.data)) as AlbionServerEvent;
      this.cache = applyAlbionServerEvent(this.cache, event);
      onEvent(event, this.cache);
    });
    socket.addEventListener("close", () => {
      if (this.socket === socket) this.socket = null;
    });
    this.socket = socket;
  }

  sendCommand(session: AlbionSession, command: unknown, expectedRevision = this.cache.revision): void {
    if (!this.socket || this.socket.readyState !== WebSocket.OPEN) {
      throw new Error("be.error.serverDisconnected");
    }
    this.socket.send(JSON.stringify({
      protocol_version: 1,
      message_id: crypto.randomUUID(),
      kind: "command",
      career_id: session.career_id,
      manager_id: session.manager_id,
      expected_revision: expectedRevision,
      payload: command,
    }));
  }

  disconnect(): void {
    this.socket?.close();
    this.socket = null;
  }

  private async post<T>(serverUrl: string, path: string, body: unknown): Promise<T> {
    const response = await fetch(new URL(path, serverUrl), {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify(body),
    });
    if (!response.ok) throw new Error(await response.text());
    return response.json() as Promise<T>;
  }
}
