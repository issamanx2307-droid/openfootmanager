import { useEffect, useRef, useState } from "react";
import { useNavigate } from "react-router-dom";
import { useTranslation } from "react-i18next";

import { startAlbionHost, stopAlbionHost } from "../services/albionHostService";
import {
  AlbionServerClient,
  clearAlbionSession,
  formatAlbionLiveEvent,
  loadAlbionSession,
  saveAlbionSession,
  type AlbionSession,
} from "../services/albionServerService";
import { useGameStore } from "../store/gameStore";

type ManagerDashboard = {
  currentDate: string;
  club: { id: string; name: string; formation: string; playStyle: string };
  training?: { focus: string; intensity: string };
  nextFixture?: { date: string; competition: string; homeTeam: string; awayTeam: string };
};

function readManagerDashboard(value: unknown): ManagerDashboard | null {
  if (!value || typeof value !== "object") return null;
  const record = value as Record<string, unknown>;
  const club = record.club;
  if (typeof record.currentDate !== "string" || !club || typeof club !== "object") return null;
  const clubRecord = club as Record<string, unknown>;
  if (typeof clubRecord.id !== "string" || typeof clubRecord.name !== "string" || typeof clubRecord.formation !== "string" || typeof clubRecord.playStyle !== "string") return null;
  const trainingRecord = record.training;
  const training = trainingRecord && typeof trainingRecord === "object"
    && typeof (trainingRecord as Record<string, unknown>).focus === "string"
    && typeof (trainingRecord as Record<string, unknown>).intensity === "string"
    ? { focus: (trainingRecord as Record<string, string>).focus, intensity: (trainingRecord as Record<string, string>).intensity }
    : undefined;
  const fixtureRecord = record.nextFixture;
  const nextFixture = fixtureRecord && typeof fixtureRecord === "object"
    && typeof (fixtureRecord as Record<string, unknown>).date === "string"
    && typeof (fixtureRecord as Record<string, unknown>).competition === "string"
    && typeof (fixtureRecord as Record<string, unknown>).homeTeam === "string"
    && typeof (fixtureRecord as Record<string, unknown>).awayTeam === "string"
    ? fixtureRecord as ManagerDashboard["nextFixture"]
    : undefined;
  return { currentDate: record.currentDate, club: { id: clubRecord.id, name: clubRecord.name, formation: clubRecord.formation, playStyle: clubRecord.playStyle }, training, nextFixture };
}

function applyDashboardDelta(value: unknown, changes: unknown): unknown {
  const dashboard = readManagerDashboard(value);
  if (!dashboard || !changes || typeof changes !== "object") return value;
  const delta = changes as Record<string, unknown>;
  if (delta.teamId !== dashboard.club.id) return value;
  const next = { ...dashboard, club: { ...dashboard.club }, training: dashboard.training && { ...dashboard.training } };
  if (typeof delta.formation === "string") next.club.formation = delta.formation;
  if (typeof delta.mentality === "string") next.club.playStyle = delta.mentality;
  if (typeof delta.teamFocus === "string") next.training = { focus: delta.teamFocus, intensity: next.training?.intensity ?? "" };
  if (typeof delta.weeklyIntensity === "number") next.training = { focus: next.training?.focus ?? "", intensity: String(delta.weeklyIntensity) };
  return next;
}

function localizedCanonicalLabel(value: string, thai: boolean): string {
  if (!thai) return value;
  const labels: Record<string, string> = {
    balanced: "สมดุล", Balanced: "สมดุล", attacking: "เกมรุก", Attacking: "เกมรุก",
    defensive: "เกมรับ", Defensive: "เกมรับ", possession: "ครองบอล", Possession: "ครองบอล",
    counter: "สวนกลับ", Counter: "สวนกลับ", high_press: "เพรสซิ่งสูง", HighPress: "เพรสซิ่งสูง",
    physical: "ร่างกาย", Physical: "ร่างกาย", technical: "เทคนิค", Technical: "เทคนิค",
    tactical: "แท็กติก", Tactical: "แท็กติก", defending: "เกมรับ", Defending: "เกมรับ",
    recovery: "ฟื้นฟู", Recovery: "ฟื้นฟู", light: "เบา", Light: "เบา",
    medium: "ปานกลาง", Medium: "ปานกลาง", high: "สูง", High: "สูง",
  };
  return labels[value] ?? value;
}

export default function MultiplayerLobby() {
  const navigate = useNavigate();
  const { i18n } = useTranslation();
  const thai = i18n.language.startsWith("th");
  const copy = i18n.language.startsWith("th") ? {
    initial: "เลือกเป็นโฮสต์หรือเข้าร่วมเกมส่วนตัว", noCareer: "ต้องเปิด career และเลือกสโมสรก่อนเริ่มเล่นร่วมกัน",
    rejected: "คำสั่งถูกปฏิเสธ: กรุณารีเฟรชข้อมูลแล้วลองใหม่", readyState: "อัปเดตสถานะพร้อมแล้ว รอผู้จัดการอีกฝ่าย",
    matchOpened: "ถึงวันแข่งขันแล้ว กำลังเปิดศูนย์การแข่งขัน", connected: (slot: string) => `เชื่อมต่อแล้วในฐานะ ${slot === "host" ? "โฮสต์" : "ผู้ร่วมเล่น"}`,
    hostFailed: "ไม่สามารถเปิดเซิร์ฟเวอร์ได้ ตรวจสอบ save และรหัสเข้าร่วม", joinFailed: "ไม่สามารถเข้าร่วมได้ ตรวจสอบ URL รหัส และเวอร์ชันของเกม",
    restored: "เชื่อมต่อ session เดิมสำเร็จ", stale: "session เดิมหมดอายุหรือเซิร์ฟเวอร์ไม่พร้อม กรุณาเข้าร่วมใหม่", readySent: "ส่งสถานะพร้อมแล้ว",
    disconnected: "การเชื่อมต่อขาดหาย กรุณาเชื่อมต่อใหม่", back: "← กลับสู่สโมสร", title: "เล่นร่วมกัน", secret: "รหัสเข้าร่วม",
    hostUrl: "URL ของโฮสต์", host: "เป็นโฮสต์", join: "เข้าร่วม", reconnect: "เชื่อมต่อเดิม", connectedAt: "เชื่อมต่อสำเร็จ · revision", ready: "พร้อมดำเนินเกม",
    tactics: "แท็กติก", formation: "แผนการเล่น", mentality: "แนวทาง", applyTactics: "บันทึกแท็กติก", tacticsSent: "ส่งแท็กติกไปยังเซิร์ฟเวอร์แล้ว",
    training: "การฝึกซ้อม", intensity: "ความเข้มข้น", focus: "จุดเน้น", applyTraining: "บันทึกแผนฝึก", trainingSent: "ส่งแผนฝึกไปยังเซิร์ฟเวอร์แล้ว",
    liveMatch: "ศูนย์การแข่งขัน", liveFormation: "เปลี่ยนแผนระหว่างแข่ง", liveSent: "ส่งคำสั่งระหว่างแข่งไปยังเซิร์ฟเวอร์แล้ว",
    matchFinished: "การแข่งขันจบแล้ว", score: "สกอร์",
    events: "เหตุการณ์ล่าสุด",
    clubView: "ข้อมูลสโมสรจากเซิร์ฟเวอร์", date: "วันในเกม", playStyle: "แนวทาง",
    nextFixture: "นัดถัดไป", noFixture: "ยังไม่มีนัดที่กำหนด",
    hostHint: "โฮสต์: แทนที่ 127.0.0.1 ด้วย IP LAN หรือ Tailscale ของคุณก่อนส่ง URL ให้เพื่อน",
  } : {
    initial: "Choose to host or join a private game", noCareer: "Open a career and choose a club before playing together",
    rejected: "Command rejected: refresh the view and try again", readyState: "Ready state updated; waiting for the other manager",
    matchOpened: "Match day is here. Opening the match centre.", connected: (slot: string) => `Connected as ${slot === "host" ? "host" : "guest"}`,
    hostFailed: "Could not start the server. Check the save and join code.", joinFailed: "Could not join. Check the URL, code, and game version.",
    restored: "Previous session restored", stale: "The previous session expired or the server is unavailable. Please join again.", readySent: "Ready status sent.",
    disconnected: "Connection lost. Please reconnect.", back: "← Back to club", title: "Play Together", secret: "Join code",
    hostUrl: "Host URL", host: "Host game", join: "Join game", reconnect: "Reconnect", connectedAt: "Connected · revision", ready: "Ready to continue",
    tactics: "Tactics", formation: "Formation", mentality: "Approach", applyTactics: "Save tactics", tacticsSent: "Tactics sent to the server.",
    training: "Training", intensity: "Intensity", focus: "Focus", applyTraining: "Save training", trainingSent: "Training plan sent to the server.",
    liveMatch: "Match centre", liveFormation: "Change live formation", liveSent: "Live-match command sent to the server.",
    matchFinished: "Match finished", score: "Score",
    events: "Latest events",
    clubView: "Server club view", date: "Game date", playStyle: "Approach",
    nextFixture: "Next fixture", noFixture: "No scheduled fixture",
    hostHint: "Host: replace 127.0.0.1 with your LAN or Tailscale IP before sharing the URL.",
  };
  const game = useGameStore((state) => state.gameState);
  const client = useRef(new AlbionServerClient());
  const hostedHere = useRef(false);
  const [serverUrl, setServerUrl] = useState("");
  const [joinSecret, setJoinSecret] = useState("");
  const [session, setSession] = useState<AlbionSession | null>(null);
  const [dashboard, setDashboard] = useState<unknown>(null);
  const [message, setMessage] = useState(copy.initial);
  const [busy, setBusy] = useState(false);
  const [formation, setFormation] = useState("4-3-3");
  const [mentality, setMentality] = useState("balanced");
  const [trainingIntensity, setTrainingIntensity] = useState(60);
  const [trainingFocus, setTrainingFocus] = useState("tactical");
  const [liveMatchId, setLiveMatchId] = useState<string | null>(null);
  const [liveState, setLiveState] = useState<{ phase: string; second: number; home: number; away: number } | null>(null);
  const [liveEvents, setLiveEvents] = useState<string[]>([]);
  const dashboardView = readManagerDashboard(dashboard);

  useEffect(() => () => {
    client.current.disconnect();
    if (hostedHere.current) void stopAlbionHost();
  }, []);

  useEffect(() => {
    const stored = loadAlbionSession();
    if (stored) setServerUrl(stored.server_url);
  }, []);

  const managerClubId = game?.manager.team_id;
  if (!game || !managerClubId) {
    return <main className="min-h-screen p-8 bg-navy-900 text-white">{copy.noCareer}</main>;
  }

  const connect = async (url: string) => {
    const versions = await client.current.version(url);
    const joined = await client.current.join(url, {
      join_secret: joinSecret,
      manager_id: game.manager.id,
      club_id: managerClubId,
      client_versions: versions,
    });
    client.current.connect(url, joined, (event, cache) => {
      if (event.type === "ViewSnapshot") setDashboard(cache.views.get("dashboard") ?? null);
      if (event.type === "StateDelta") setDashboard((previous) => applyDashboardDelta(previous, event.body?.changes));
      if (event.type === "CommandRejected") setMessage(copy.rejected);
      if (event.type === "ReadyStateChanged") setMessage(copy.readyState);
      if (event.type === "MatchOpened") {
        const matchId = event.body?.match_id;
        if (typeof matchId === "string") setLiveMatchId(matchId);
        setMessage(copy.matchOpened);
      }
      if (event.type === "MatchState") {
        const body = event.body;
        if (typeof body?.phase === "string" && typeof body.match_second === "number" && typeof body.home_score === "number" && typeof body.away_score === "number") {
          setLiveState({ phase: body.phase, second: body.match_second, home: body.home_score, away: body.away_score });
        }
      }
      const batchEvents = event.body?.events;
      if (event.type === "MatchEventBatch" && Array.isArray(batchEvents)) {
        setLiveEvents((previous) => [...previous, ...batchEvents.map((item) => formatAlbionLiveEvent(item, thai))].slice(-6));
      }
      if (event.type === "MatchFinished") {
        setLiveMatchId(null);
        setMessage(copy.matchFinished);
      }
    });
    saveAlbionSession({ ...joined, server_url: url });
    setSession(joined);
    setMessage(copy.connected(joined.slot));
  };

  const host = async () => {
    setBusy(true);
    try {
      const info = await startAlbionHost(joinSecret);
      hostedHere.current = true;
      setServerUrl(info.server_url);
      await connect(info.server_url);
    } catch {
      setMessage(copy.hostFailed);
    } finally {
      setBusy(false);
    }
  };

  const join = async () => {
    setBusy(true);
    try {
      await connect(serverUrl);
    } catch {
      setMessage(copy.joinFailed);
    } finally {
      setBusy(false);
    }
  };

  const reconnect = async () => {
    const stored = loadAlbionSession();
    if (!stored) return;
    setBusy(true);
    try {
      const versions = await client.current.version(stored.server_url);
      const restored = await client.current.reconnect(stored.server_url, stored.reconnect_token, versions);
      client.current.connect(stored.server_url, restored, (event, cache) => {
        if (event.type === "ViewSnapshot") setDashboard(cache.views.get("dashboard") ?? null);
        if (event.type === "StateDelta") setDashboard((previous) => applyDashboardDelta(previous, event.body?.changes));
        if (event.type === "MatchOpened") {
          const matchId = event.body?.match_id;
          if (typeof matchId === "string") setLiveMatchId(matchId);
          setMessage(copy.matchOpened);
        }
        if (event.type === "MatchState") {
          const body = event.body;
          if (typeof body?.phase === "string" && typeof body.match_second === "number" && typeof body.home_score === "number" && typeof body.away_score === "number") {
            setLiveState({ phase: body.phase, second: body.match_second, home: body.home_score, away: body.away_score });
          }
        }
        const batchEvents = event.body?.events;
        if (event.type === "MatchEventBatch" && Array.isArray(batchEvents)) {
          setLiveEvents((previous) => [...previous, ...batchEvents.map((item) => formatAlbionLiveEvent(item, thai))].slice(-6));
        }
        if (event.type === "MatchFinished") {
          setLiveMatchId(null);
          setMessage(copy.matchFinished);
        }
      });
      setSession(restored);
      setMessage(copy.restored);
    } catch {
      clearAlbionSession();
      setMessage(copy.stale);
    } finally {
      setBusy(false);
    }
  };

  const markReady = () => {
    if (!session) return;
    try {
      client.current.sendCommand(session, { MarkReady: {} });
      setMessage(copy.readySent);
    } catch {
      setMessage(copy.disconnected);
    }
  };

  const applyTactics = () => {
    if (!session) return;
    try {
      client.current.sendCommand(session, { SetTactics: { formation, mentality } });
      setMessage(copy.tacticsSent);
    } catch {
      setMessage(copy.disconnected);
    }
  };

  const applyTraining = () => {
    if (!session) return;
    try {
      client.current.sendCommand(session, { SetTrainingPlan: { weekly_intensity: trainingIntensity, team_focus: trainingFocus } });
      setMessage(copy.trainingSent);
    } catch {
      setMessage(copy.disconnected);
    }
  };

  const applyLiveFormation = () => {
    if (!session || !liveMatchId) return;
    try {
      client.current.sendCommand(session, {
        ApplyLiveMatchCommand: {
          match_id: liveMatchId,
          command: { type: "ChangeFormation", body: { formation } },
        },
      });
      setMessage(copy.liveSent);
    } catch {
      setMessage(copy.disconnected);
    }
  };

  return <main className="min-h-screen bg-navy-900 text-white p-6 sm:p-10">
    <div className="mx-auto max-w-xl space-y-5 rounded-2xl bg-navy-800 p-6 shadow-xl">
      <button type="button" onClick={() => navigate("/dashboard")} className="text-accent-300 hover:text-accent-100">{copy.back}</button>
      <h1 className="font-heading text-3xl font-bold">{copy.title}</h1>
      <p className="text-gray-300">{message}</p>
      <label className="block">{copy.secret}
        <input value={joinSecret} onChange={(event) => setJoinSecret(event.target.value)} className="mt-1 w-full rounded bg-navy-700 p-3" type="password" />
      </label>
      <label className="block">{copy.hostUrl}
        <input value={serverUrl} onChange={(event) => setServerUrl(event.target.value)} placeholder="http://192.168.1.10:38421" className="mt-1 w-full rounded bg-navy-700 p-3" />
      </label>
      <p className="text-sm text-gray-300">{copy.hostHint}</p>
      <div className="flex gap-3">
        <button type="button" disabled={busy || !joinSecret} onClick={() => void host()} className="rounded bg-primary-500 px-4 py-3 font-bold disabled:opacity-50">{copy.host}</button>
        <button type="button" disabled={busy || !joinSecret || !serverUrl} onClick={() => void join()} className="rounded bg-accent-500 px-4 py-3 font-bold disabled:opacity-50">{copy.join}</button>
        {loadAlbionSession() && <button type="button" disabled={busy} onClick={() => void reconnect()} className="rounded bg-navy-600 px-4 py-3 font-bold disabled:opacity-50">{copy.reconnect}</button>}
      </div>
      {session && <section className="rounded bg-navy-700 p-4" aria-live="polite">
        <p>{copy.connectedAt} {session.current_revision}</p>
        <button type="button" onClick={markReady} className="mt-3 rounded bg-primary-500 px-4 py-2 font-bold">{copy.ready}</button>
        <fieldset className="mt-4 grid gap-2 rounded border border-navy-600 p-3">
          <legend className="px-1 font-bold">{copy.tactics}</legend>
          <label>{copy.formation}<input value={formation} onChange={(event) => setFormation(event.target.value)} className="ml-2 rounded bg-navy-800 p-2" /></label>
          <label>{copy.mentality}<select value={mentality} onChange={(event) => setMentality(event.target.value)} className="ml-2 rounded bg-navy-800 p-2">{["balanced", "attacking", "defensive", "possession", "counter", "high_press"].map((value) => <option key={value} value={value}>{localizedCanonicalLabel(value, thai)}</option>)}</select></label>
          <button type="button" onClick={applyTactics} className="w-fit rounded bg-accent-500 px-3 py-2 font-bold">{copy.applyTactics}</button>
        </fieldset>
        <fieldset className="mt-4 grid gap-2 rounded border border-navy-600 p-3">
          <legend className="px-1 font-bold">{copy.training}</legend>
          <label>{copy.intensity}<input type="range" min="0" max="100" value={trainingIntensity} onChange={(event) => setTrainingIntensity(Number(event.target.value))} className="ml-2 align-middle" /><output className="ml-2">{trainingIntensity}</output></label>
          <label>{copy.focus}<select value={trainingFocus} onChange={(event) => setTrainingFocus(event.target.value)} className="ml-2 rounded bg-navy-800 p-2">{["physical", "technical", "tactical", "defending", "attacking", "recovery"].map((value) => <option key={value} value={value}>{localizedCanonicalLabel(value, thai)}</option>)}</select></label>
          <button type="button" onClick={applyTraining} className="w-fit rounded bg-accent-500 px-3 py-2 font-bold">{copy.applyTraining}</button>
        </fieldset>
        {liveMatchId && <fieldset className="mt-4 grid gap-2 rounded border border-primary-500 p-3">
          <legend className="px-1 font-bold">{copy.liveMatch}</legend>
          {liveState && <p aria-live="polite">{copy.score}: {liveState.home}–{liveState.away} · {Math.floor(liveState.second / 60)}′ · {liveState.phase}</p>}
          <button type="button" onClick={applyLiveFormation} className="w-fit rounded bg-primary-500 px-3 py-2 font-bold">{copy.liveFormation}: {formation}</button>
          {liveEvents.length > 0 && <div aria-live="polite"><p className="font-semibold">{copy.events}</p><ul className="list-disc pl-5 text-sm">{liveEvents.map((event, index) => <li key={index}>{event}</li>)}</ul></div>}
        </fieldset>}
        {dashboardView && <section className="mt-4 rounded border border-navy-600 p-3" aria-label={copy.clubView}>
          <h2 className="font-bold">{copy.clubView} · {dashboardView.club.name}</h2>
          <dl className="mt-2 grid grid-cols-2 gap-2 text-sm">
            <dt className="text-gray-300">{copy.date}</dt><dd>{dashboardView.currentDate}</dd>
            <dt className="text-gray-300">{copy.tactics}</dt><dd>{dashboardView.club.formation}</dd>
            <dt className="text-gray-300">{copy.playStyle}</dt><dd>{localizedCanonicalLabel(dashboardView.club.playStyle, thai)}</dd>
            {dashboardView.training && <><dt className="text-gray-300">{copy.training}</dt><dd>{localizedCanonicalLabel(dashboardView.training.focus, thai)} · {localizedCanonicalLabel(dashboardView.training.intensity, thai)}</dd></>}
          </dl>
          <div className="mt-3 border-t border-navy-600 pt-3 text-sm">
            <p className="font-semibold">{copy.nextFixture}</p>
            {dashboardView.nextFixture
              ? <p>{dashboardView.nextFixture.date} · {dashboardView.nextFixture.homeTeam}–{dashboardView.nextFixture.awayTeam} · {dashboardView.nextFixture.competition}</p>
              : <p className="text-gray-300">{copy.noFixture}</p>}
          </div>
        </section>}
      </section>}
    </div>
  </main>;
}
