import { invoke } from "@tauri-apps/api/core";
import { useEffect, useState } from "react";
import ArchiveTracks, { type ArchiveTrack } from "../components/ArchiveTracks";
import TrackDetail from "./TrackDetail";
import { localDate, monthRanges } from "../archiveDates";
export interface CalendarDay { date: string; plays: number; discoveries: number; tracks: ArchiveTrack[] }
export default function Calendar() {
  const [month, setMonth] = useState(localDate().slice(0, 7));
  const [days, setDays] = useState<CalendarDay[]>([]);
  const [selected, setSelected] = useState<string | null>(localDate());
  const [track, setTrack] = useState<number | null>(null);
  const [error, setError] = useState("");
  const [loading, setLoading] = useState(true);
  const [attempt, setAttempt] = useState(0);
  useEffect(() => {
    let active = true; setLoading(true); setError("");
    invoke<CalendarDay[]>("calendar_days", { ranges: monthRanges(month) }).then(d => { if (active) setDays(d); }).catch(e => { if (active) setError(String(e)); }).finally(() => { if (active) setLoading(false); });
    return () => { active = false; };
  }, [month, attempt]);
  useEffect(() => {
    if (!selected || track !== null) return;
    function closeOnEscape(event: KeyboardEvent) {
      if (event.key === "Escape") setSelected(null);
    }
    window.addEventListener("keydown", closeOnEscape);
    return () => window.removeEventListener("keydown", closeOnEscape);
  }, [selected, track]);
  function move(delta: number) { const [y,m] = month.split("-").map(Number); const next=localDate(new Date(y,m-1+delta,1)); setMonth(next.slice(0,7)); setSelected(next); }
  const day=days.find(d => d.date===selected);
  const [year,m]=month.split("-").map(Number);
  const offset=(new Date(year,m-1,1).getDay()+6)%7;
  return <><section className="page" hidden={track!==null}>
    <h1>Calendar</h1><div className="action-row"><button aria-label="Previous month" onClick={() => move(-1)}>←</button><h2>{new Date(year,m-1,1).toLocaleDateString(undefined,{year:"numeric",month:"long"})}</h2><button aria-label="Next month" onClick={() => move(1)}>→</button><button onClick={() => { setMonth(localDate().slice(0,7)); setSelected(localDate()); }}>Today</button></div>
    <p className="muted">Dates use your system time zone ({Intl.DateTimeFormat().resolvedOptions().timeZone}).</p>
    {loading && <p role="status">Loading calendar…</p>}
    {error && <p role="alert">{error} <button onClick={() => setAttempt(a=>a+1)}>Retry</button></p>}
    {!loading && !error && <div className={`calendar-layout${day ? " detail-open" : ""}`}>
      <div className="calendar-grid">{["Mon","Tue","Wed","Thu","Fri","Sat","Sun"].map(d=><span key={d}>{d}</span>)}{Array.from({length:offset},(_,i)=><span key={`blank-${i}`} />)}{days.map(d=><button key={d.date} className={selected===d.date?"selected":""} aria-pressed={selected===d.date} onClick={()=>setSelected(d.date)}><strong>{Number(d.date.slice(-2))}</strong><span>{d.plays} plays</span><small>{d.discoveries} new</small></button>)}</div>
      {day && <aside className="calendar-day-detail" aria-labelledby="calendar-day-title">
        <header className="calendar-day-header"><div><p className="eyebrow">Day Detail</p><h2 id="calendar-day-title">{selected}</h2></div><button className="calendar-day-close" type="button" aria-label="Close day detail" onClick={()=>setSelected(null)}>Close</button></header>
        <p className="calendar-day-summary">{day.plays} plays · {day.tracks.length} tracks · {day.discoveries} discoveries</p>
        <section><h3>New Discoveries</h3>{day.discoveries ? <ArchiveTracks tracks={day.tracks.filter(t=>t.discovered)} onOpen={setTrack} />:<p className="muted">No new discoveries.</p>}</section>
        <section><h3>All Tracks</h3>{day.tracks.length ? <ArchiveTracks tracks={day.tracks} onOpen={setTrack} />:<p className="muted">No listening history for this day.</p>}</section>
      </aside>}
    </div>}
  </section>{track!==null && <TrackDetail trackId={track} onBack={()=>setTrack(null)} backLabel="Calendar" />}</>;
}
