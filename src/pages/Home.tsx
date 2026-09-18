import {invoke} from "@tauri-apps/api/core";
import {useEffect,useState} from "react";
import ArchiveTracks,{type ArchiveTrack} from "../components/ArchiveTracks";
import type {CalendarDay} from "./Calendar";
import type {DiggingSession} from "./Digging";
import DiggingDetail from "./DiggingDetail";
import TrackDetail from "./TrackDetail";
import {localDate,monthRanges} from "../archiveDates";
interface HomeTrack extends ArchiveTrack {discovered_at:string|null;last_played_at:string|null}
export default function Home(){
  const [tracks,setTracks]=useState<HomeTrack[]>([]);
  const [days,setDays]=useState<CalendarDay[]>([]);
  const [digging,setDigging]=useState<DiggingSession[]>([]);
  const [selected,setSelected]=useState<number|null>(null);
  const [session,setSession]=useState<number|null>(null);
  const [error,setError]=useState("");
  const [loading,setLoading]=useState(true);
  const [attempt,setAttempt]=useState(0);
  useEffect(()=>{let active=true;setLoading(true);setError("");Promise.all([invoke<HomeTrack[]>("library_tracks"),invoke<CalendarDay[]>("calendar_days",{ranges:monthRanges(localDate().slice(0,7))}),invoke<DiggingSession[]>("digging_sessions")]).then(([t,d,s])=>{if(active){setTracks(t);setDays(d);setDigging(s);}}).catch(e=>{if(active)setError(String(e));}).finally(()=>{if(active)setLoading(false);});return()=>{active=false;};},[attempt]);
  return <><section className="page" hidden={selected!==null||session!==null}><p className="eyebrow">Your music, over time</p><h1>{new Date().toLocaleDateString(undefined,{month:"long",year:"numeric"})}</h1>{loading&&<p role="status">Loading your archive…</p>}{error&&<p role="alert">{error}<button onClick={()=>setAttempt(a=>a+1)}>Retry</button></p>}{!loading&&!error&&<><p className="home-summary"><strong>{days.reduce((n,d)=>n+d.plays,0)}</strong> plays · <strong>{days.reduce((n,d)=>n+d.discoveries,0)}</strong> discoveries this month</p>{!tracks.length&&<p>Your archive is empty. Connect Spotify and sync in Settings to begin.</p>}<section className="archive-section"><h2>Recently Discovered</h2><ArchiveTracks tracks={tracks.filter(t=>t.discovered_at).slice(0,6)} onOpen={setSelected}/></section><section className="archive-section"><h2>Currently Digging</h2>{digging.filter(d=>!d.end_date).map(d=><button className="digging-row" key={d.id} onClick={()=>setSession(d.id)}><strong>{d.name}</strong><span>{d.start_date} — Present · {d.track_count} tracks</span></button>)}{!digging.some(d=>!d.end_date)&&<p>Start a session in Digging to record what you are exploring.</p>}</section><section className="archive-section"><h2>Recently Played</h2><ArchiveTracks tracks={[...tracks].filter(t=>t.last_played_at).sort((a,b)=>b.last_played_at!.localeCompare(a.last_played_at!)).slice(0,8)} onOpen={setSelected}/></section></>}</section>{selected!==null&&<TrackDetail trackId={selected} onBack={()=>setSelected(null)} backLabel="Home"/>}{session!==null&&<DiggingDetail diggingId={session} onBack={()=>{setSession(null);setAttempt(a=>a+1);}}/>}</>;
}
