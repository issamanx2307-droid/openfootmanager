import { useEffect, useRef, useState } from "react";
import { useNavigate } from "react-router-dom";

import { startAlbionHost, stopAlbionHost } from "../services/albionHostService";
import { AlbionServerClient, type AlbionSession } from "../services/albionServerService";
import { useGameStore } from "../store/gameStore";

export default function MultiplayerLobby() {
  const navigate = useNavigate();
  const game = useGameStore((state) => state.gameState);
  const client = useRef(new AlbionServerClient());
  const hostedHere = useRef(false);
  const [serverUrl, setServerUrl] = useState("");
  const [joinSecret, setJoinSecret] = useState("");
  const [session, setSession] = useState<AlbionSession | null>(null);
  const [dashboard, setDashboard] = useState<unknown>(null);
  const [message, setMessage] = useState("เลือกเป็นโฮสต์หรือเข้าร่วมเกมส่วนตัว");
  const [busy, setBusy] = useState(false);

  useEffect(() => () => {
    client.current.disconnect();
    if (hostedHere.current) void stopAlbionHost();
  }, []);

  if (!game?.manager.team_id) {
    return <main className="min-h-screen p-8 bg-navy-900 text-white">ต้องเปิด career และเลือกสโมสรก่อนเริ่มเล่นร่วมกัน</main>;
  }

  const connect = async (url: string) => {
    const versions = await client.current.version(url);
    const joined = await client.current.join(url, {
      join_secret: joinSecret,
      manager_id: game.manager.id,
      club_id: game.manager.team_id,
      client_versions: versions,
    });
    client.current.connect(url, joined, (event, cache) => {
      if (event.type === "ViewSnapshot") setDashboard(cache.views.get("dashboard") ?? null);
      if (event.type === "CommandRejected") setMessage("คำสั่งถูกปฏิเสธ: กรุณารีเฟรชข้อมูลแล้วลองใหม่");
    });
    setSession(joined);
    setMessage(`เชื่อมต่อแล้วในฐานะ ${joined.slot === "host" ? "โฮสต์" : "ผู้ร่วมเล่น"}`);
  };

  const host = async () => {
    setBusy(true);
    try {
      const info = await startAlbionHost(joinSecret);
      hostedHere.current = true;
      setServerUrl(info.server_url);
      await connect(info.server_url);
    } catch {
      setMessage("ไม่สามารถเปิดเซิร์ฟเวอร์ได้ ตรวจสอบ save และรหัสเข้าร่วม");
    } finally {
      setBusy(false);
    }
  };

  const join = async () => {
    setBusy(true);
    try {
      await connect(serverUrl);
    } catch {
      setMessage("ไม่สามารถเข้าร่วมได้ ตรวจสอบ URL รหัส และเวอร์ชันของเกม");
    } finally {
      setBusy(false);
    }
  };

  return <main className="min-h-screen bg-navy-900 text-white p-6 sm:p-10">
    <div className="mx-auto max-w-xl space-y-5 rounded-2xl bg-navy-800 p-6 shadow-xl">
      <button type="button" onClick={() => navigate("/dashboard")} className="text-accent-300 hover:text-accent-100">← กลับสู่สโมสร</button>
      <h1 className="font-heading text-3xl font-bold">เล่นร่วมกัน</h1>
      <p className="text-gray-300">{message}</p>
      <label className="block">รหัสเข้าร่วม
        <input value={joinSecret} onChange={(event) => setJoinSecret(event.target.value)} className="mt-1 w-full rounded bg-navy-700 p-3" type="password" />
      </label>
      <label className="block">URL ของโฮสต์
        <input value={serverUrl} onChange={(event) => setServerUrl(event.target.value)} placeholder="http://192.168.1.10:38421" className="mt-1 w-full rounded bg-navy-700 p-3" />
      </label>
      <div className="flex gap-3">
        <button type="button" disabled={busy || !joinSecret} onClick={() => void host()} className="rounded bg-primary-500 px-4 py-3 font-bold disabled:opacity-50">เป็นโฮสต์</button>
        <button type="button" disabled={busy || !joinSecret || !serverUrl} onClick={() => void join()} className="rounded bg-accent-500 px-4 py-3 font-bold disabled:opacity-50">เข้าร่วม</button>
      </div>
      {session && <section className="rounded bg-navy-700 p-4" aria-live="polite">
        <p>เชื่อมต่อสำเร็จ · revision {session.current_revision}</p>
        {dashboard && <pre className="mt-3 overflow-auto text-xs text-gray-200">{JSON.stringify(dashboard, null, 2)}</pre>}
      </section>}
    </div>
  </main>;
}
