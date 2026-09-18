import { invoke } from "@tauri-apps/api/core";
import { useEffect, useState } from "react";
import DiggingDetail, { DiggingForm } from "./DiggingDetail";
export interface DiggingSession {id:number;name:string;start_date:string;end_date:string|null;description:string;track_count:number;artist_count:number}
export default function Digging({ initialId }: { initialId?: number } = {}) {
  const [rows,setRows]=useState<DiggingSession[]>([]);
  const [selected,setSelected]=useState<number|null>(initialId??null);
  const [creating,setCreating]=useState(false);
  const [error,setError]=useState("");
  const [loading,setLoading]=useState(true);
  const [attempt,setAttempt]=useState(0);
  useEffect(()=>{let active=true;setLoading(true);setError("");invoke<DiggingSession[]>("digging_sessions").then(r=>{if(active)setRows(r);}).catch(e=>{if(active)setError(String(e));}).finally(()=>{if(active)setLoading(false);});return()=>{active=false;};},[attempt]);
  function back(){setSelected(null);setAttempt(a=>a+1);}
  return <><section className="page" hidden={selected!==null}><h1>Digging</h1><p className="muted">Keep a record of the sounds, scenes, and artists you are exploring.</p><button onClick={()=>setCreating(v=>!v)}>{creating?"Cancel":"New digging"}</button>{creating&&<DiggingForm onSaved={id=>{setCreating(false);setSelected(id);}}/>}{loading&&<p role="status">Loading digging…</p>}{error&&<p role="alert">{error}<button onClick={()=>setAttempt(a=>a+1)}>Retry</button></p>}{[false,true].map(past=><section className="archive-section" key={String(past)}><h2>{past?"Past Digging":"Currently Digging"}</h2>{rows.filter(d=>!!d.end_date===past).map(d=><button className="digging-row" key={d.id} onClick={()=>setSelected(d.id)}><strong>{d.name}</strong><span>{d.start_date} — {d.end_date||"Present"}</span><small>{d.track_count} tracks · {d.artist_count} artists</small></button>)}{!loading&&!rows.some(d=>!!d.end_date===past)&&<p>{past?"No past sessions yet.":"No active sessions. Start one when a new sound catches your attention."}</p>}</section>)}</section>{selected!==null&&<DiggingDetail key={selected} diggingId={selected} onBack={back}/>}</>;
}
