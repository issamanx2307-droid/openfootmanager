import { useEffect, useRef, useState } from "react";
import { useNavigate } from "react-router-dom";
import { useTranslation } from "react-i18next";

import { startAlbionHost, stopAlbionHost } from "../services/albionHostService";
import {
  AlbionServerClient,
  clearAlbionSession,
  loadAlbionSession,
  saveAlbionSession,
  type AlbionSession,
} from "../services/albionServerService";
import { useGameStore } from "../store/gameStore";

export default function MultiplayerLobby() {
  const navigate = useNavigate();
  const { i18n } = useTranslation();
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
          <label>{copy.mentality}<select value={mentality} onChange={(event) => setMentality(event.target.value)} className="ml-2 rounded bg-navy-800 p-2"><option value="balanced">Balanced</option><option value="attacking">Attacking</option><option value="defensive">Defensive</option><option value="possession">Possession</option><option value="counter">Counter</option><option value="high_press">High press</option></select></label>
          <button type="button" onClick={applyTactics} className="w-fit rounded bg-accent-500 px-3 py-2 font-bold">{copy.applyTactics}</button>
        </fieldset>
        <fieldset className="mt-4 grid gap-2 rounded border border-navy-600 p-3">
          <legend className="px-1 font-bold">{copy.training}</legend>
          <label>{copy.intensity}<input type="range" min="0" max="100" value={trainingIntensity} onChange={(event) => setTrainingIntensity(Number(event.target.value))} className="ml-2 align-middle" /><output className="ml-2">{trainingIntensity}</output></label>
          <label>{copy.focus}<select value={trainingFocus} onChange={(event) => setTrainingFocus(event.target.value)} className="ml-2 rounded bg-navy-800 p-2"><option value="physical">Physical</option><option value="technical">Technical</option><option value="tactical">Tactical</option><option value="defending">Defending</option><option value="attacking">Attacking</option><option value="recovery">Recovery</option></select></label>
          <button type="button" onClick={applyTraining} className="w-fit rounded bg-accent-500 px-3 py-2 font-bold">{copy.applyTraining}</button>
        </fieldset>
        {liveMatchId && <fieldset className="mt-4 grid gap-2 rounded border border-primary-500 p-3">
          <legend className="px-1 font-bold">{copy.liveMatch}</legend>
          {liveState && <p aria-live="polite">{copy.score}: {liveState.home}–{liveState.away} · {Math.floor(liveState.second / 60)}′ · {liveState.phase}</p>}
          <button type="button" onClick={applyLiveFormation} className="w-fit rounded bg-primary-500 px-3 py-2 font-bold">{copy.liveFormation}: {formation}</button>
        </fieldset>}
        {dashboard !== null && <pre className="mt-3 overflow-auto text-xs text-gray-200">{String(JSON.stringify(dashboard, null, 2))}</pre>}
      </section>}
    </div>
  </main>;
}
