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

/** Convert the ergonomic React command form into the Rust protocol's
 * internally-tagged `{ type, body }` representation. */
export function serializeAlbionCommand(command: unknown): unknown {
  if (!command || typeof command !== "object" || Array.isArray(command)) return command;
  const entries = Object.entries(command as Record<string, unknown>);
  if (entries.length !== 1) return command;
  const [type, body] = entries[0];
  return { type, body };
}

export type AlbionViewCache = {
  revision: number;
  views: ReadonlyMap<string, unknown>;
};

const LIVE_EVENT_LABELS: Record<string, { en: string; th: string }> = {
  Goal: { en: "Goal", th: "ประตู" }, PenaltyGoal: { en: "Penalty goal", th: "จุดโทษเข้า" },
  ShotOnTarget: { en: "Shot on target", th: "ยิงเข้ากรอบ" }, ShotSaved: { en: "Save", th: "ผู้รักษาประตูเซฟ" },
  YellowCard: { en: "Yellow card", th: "ใบเหลือง" }, RedCard: { en: "Red card", th: "ใบแดง" },
  Substitution: { en: "Substitution", th: "เปลี่ยนตัว" }, HalfTime: { en: "Half time", th: "จบครึ่งแรก" },
  FullTime: { en: "Full time", th: "จบการแข่งขัน" }, KickOff: { en: "Kick-off", th: "เริ่มการแข่งขัน" },
};

const MATCH_PHASE_LABELS: Record<string, { en: string; th: string }> = {
  PreKickOff: { en: "Pre-match", th: "ก่อนการแข่งขัน" },
  FirstHalf: { en: "First half", th: "ครึ่งแรก" },
  HalfTime: { en: "Half time", th: "พักครึ่ง" },
  SecondHalf: { en: "Second half", th: "ครึ่งหลัง" },
  FullTime: { en: "Full time", th: "จบการแข่งขัน" },
  ExtraTimeFirstHalf: { en: "Extra time · first half", th: "ต่อเวลาพิเศษ · ครึ่งแรก" },
  ExtraTimeHalfTime: { en: "Extra time half time", th: "พักต่อเวลาพิเศษ" },
  ExtraTimeSecondHalf: { en: "Extra time · second half", th: "ต่อเวลาพิเศษ · ครึ่งหลัง" },
  ExtraTimeEnd: { en: "End of extra time", th: "จบต่อเวลาพิเศษ" },
  PenaltyShootout: { en: "Penalty shootout", th: "ดวลจุดโทษ" },
  Finished: { en: "Final", th: "สิ้นสุด" },
};

const MATCH_SIDE_LABELS: Record<string, { en: string; th: string }> = {
  Home: { en: "Home", th: "เหย้า" },
  Away: { en: "Away", th: "เยือน" },
};

const ERROR_LABELS: Record<string, { en: string; th: string }> = {
  AUTH_INVALID: { en: "You cannot make that change for this club.", th: "คุณไม่มีสิทธิ์เปลี่ยนแปลงสโมสรนี้" },
  PROTOCOL_INCOMPATIBLE: { en: "Your game version is incompatible with this host.", th: "เวอร์ชันเกมของคุณไม่ตรงกับโฮสต์" },
  STALE_REVISION: { en: "The game changed. Your view has been refreshed; try again.", th: "เกมมีการเปลี่ยนแปลงแล้ว รีเฟรชข้อมูลและลองอีกครั้ง" },
  CLUB_ALREADY_CONTROLLED: { en: "Another manager already controls this club.", th: "มีผู้จัดการคนอื่นคุมสโมสรนี้อยู่แล้ว" },
  INVALID_LINEUP: { en: "Choose eleven unique, fit players.", th: "เลือกนักเตะที่พร้อมลงเล่น 11 คนและห้ามซ้ำ" },
  TRANSFER_WINDOW_CLOSED: { en: "The transfer window is closed.", th: "ตลาดซื้อขายปิดอยู่" },
  INSUFFICIENT_TRANSFER_BUDGET: { en: "The club does not have enough transfer budget.", th: "สโมสรมีงบซื้อขายไม่เพียงพอ" },
  INSUFFICIENT_WAGE_BUDGET: { en: "The club does not have enough wage budget.", th: "สโมสรมีงบค่าเหนื่อยไม่เพียงพอ" },
  REGISTRATION_INVALID: { en: "This registration is not valid.", th: "การลงทะเบียนนี้ไม่ถูกต้อง" },
  MATCH_COMMAND_NOT_ALLOWED: { en: "That command is not available now.", th: "ยังใช้คำสั่งนี้ไม่ได้ในตอนนี้" },
  SAVE_CORRUPT: { en: "The host save cannot be used.", th: "เซฟของโฮสต์ไม่สามารถใช้งานได้" },
  SNAPSHOT_INVALID: { en: "The current data snapshot is invalid.", th: "ชุดข้อมูลปัจจุบันไม่ถูกต้อง" },
};

/** Localizes stable protocol error codes; the server deliberately sends no prose. */
export function formatAlbionProtocolError(error: unknown, thai: boolean): string {
  const code = error && typeof error === "object" && typeof (error as Record<string, unknown>).code === "string"
    ? (error as Record<string, string>).code : "";
  return ERROR_LABELS[code]?.[thai ? "th" : "en"] ?? (thai ? "คำสั่งถูกปฏิเสธ" : "Command rejected");
}

/** Formats only facts received in the canonical event stream. */
export function formatAlbionLiveEvent(event: unknown, thai: boolean): string {
  if (!event || typeof event !== "object") return thai ? "เหตุการณ์การแข่งขัน" : "Match event";
  const record = event as Record<string, unknown>;
  const minute = typeof record.minute === "number" ? `${record.minute}′ ` : "";
  const type = typeof record.event_type === "string" ? record.event_type : "";
  const side = typeof record.side === "string" ? MATCH_SIDE_LABELS[record.side]?.[thai ? "th" : "en"] : undefined;
  const label = LIVE_EVENT_LABELS[type]?.[thai ? "th" : "en"] ?? (thai ? "เหตุการณ์การแข่งขัน" : "Match event");
  return `${minute}${label}${side ? ` · ${side}` : ""}`;
}

/** Localizes stable server phase codes without displaying protocol names to players. */
export function formatAlbionMatchPhase(phase: unknown, thai: boolean): string {
  const code = typeof phase === "string" ? phase : "";
  return MATCH_PHASE_LABELS[code]?.[thai ? "th" : "en"] ?? (thai ? "กำลังแข่งขัน" : "Match in progress");
}

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

  sendCommand(session: AlbionSession, command: unknown, expectedRevision = this.cache.revision): string {
    if (!this.socket || this.socket.readyState !== WebSocket.OPEN) {
      throw new Error("be.error.serverDisconnected");
    }
    const messageId = crypto.randomUUID();
    this.socket.send(JSON.stringify({
      protocol_version: 1,
      message_id: messageId,
      kind: "command",
      career_id: session.career_id,
      manager_id: session.manager_id,
      expected_revision: expectedRevision,
      payload: serializeAlbionCommand(command),
    }));
    return messageId;
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
