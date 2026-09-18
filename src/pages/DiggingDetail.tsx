import { invoke } from "@tauri-apps/api/core";
import { useEffect, useState } from "react";
import type { DiggingSession } from "./Digging";
import ArchiveTracks, {type ArchiveTrack} from "../components/ArchiveTracks";
import TrackDetail from "./TrackDetail";
import {localDate} from "../archiveDates";
interface Detail {session:DiggingSession;tracks:ArchiveTrack[]}
export function DiggingForm({session,onSaved}:{session?:DiggingSession;onSaved:(id:number)=>void}){
  const [name,setName]=useState(session?.name??"");
  const [start,setStart]=useState(session?.start_date??localDate());
  const [end,setEnd]=useState(session?.end_date??"");
  const [description,setDescription]=useState(session?.description??"");
  const [error,setError]=useState("");
  const [pending,setPending]=useState(false);
  async function save(){setPending(true);setError("");try{const id=await invoke<number>("save_digging",{input:{id:session?.id??null,name,start_date:start,end_date:end||null,description}});onSaved(id);}catch(e){setError(String(e));}finally{setPending(false);}}
  return <form className="archive-form" onSubmit={e=>{e.preventDefault();void save();}}><label>Name<input required maxLength={200} value={name} disabled={pending} onChange={e=>setName(e.target.value)}/></label><div className="action-row"><label>Start date<input required type="date" value={start} disabled={pending} onChange={e=>setStart(e.target.value)}/></label><label>End date (optional)<input type="date" min={start} value={end} disabled={pending} onChange={e=>setEnd(e.target.value)}/></label>{session&&!end&&<button type="button" onClick={()=>setEnd(localDate())}>End today</button>}</div><label>Description<textarea rows={4} maxLength={50000} value={description} disabled={pending} onChange={e=>setDescription(e.target.value)}/></label>{error&&<p role="alert">{error}</p>}<button disabled={pending||!name.trim()}>{pending?"Saving…":"Save digging"}</button></form>;
}
export default function DiggingDetail({diggingId,onBack}:{diggingId:number;onBack:()=>void}){
  const [detail,setDetail]=useState<Detail|null>(null);
  const [attempt,setAttempt]=useState(0);
  const [error,setError]=useState("");
  const [editing,setEditing]=useState(false);
  const [search,setSearch]=useState("");
  const [matches,setMatches]=useState<ArchiveTrack[]>([]);
  const [searchError,setSearchError]=useState("");
  const [track,setTrack]=useState<number|null>(null);
  const [pending,setPending]=useState(false);
  useEffect(()=>{let active=true;setError("");invoke<Detail>("digging_detail",{diggingId}).then(d=>{if(active)setDetail(d);}).catch(e=>{if(active)setError(String(e));});return()=>{active=false;};},[diggingId,attempt]);
  useEffect(()=>{let active=true;setSearchError("");const timer=setTimeout(()=>{invoke<ArchiveTrack[]>("library_tracks",{search}).then(t=>{if(active)setMatches(t);}).catch(e=>{if(active)setSearchError(String(e));});},200);return()=>{active=false;clearTimeout(timer);};},[search,attempt]);
  async function assignment(trackId:number,assigned:boolean){setPending(true);setError("");try{await invoke("set_digging_track",{diggingId,trackId,assigned});setAttempt(a=>a+1);}catch(e){setError(String(e));}finally{setPending(false);}}
  async function remove(){if(!window.confirm("Delete this digging session? Tracks and listening history will remain."))return;setPending(true);try{await invoke("delete_digging",{diggingId});onBack();}catch(e){setError(String(e));}finally{setPending(false);}}
  return <><section className="page" hidden={track!==null}><button onClick={onBack}>← Digging</button>{error&&<p role="alert">{error}<button onClick={()=>setAttempt(a=>a+1)}>Retry</button></p>}{!detail&&!error&&<p role="status">Loading digging…</p>}{detail&&<><h1>{detail.session.name}</h1><p>{detail.session.start_date} — {detail.session.end_date||"Present"}</p><p>{detail.session.track_count} tracks · {detail.session.artist_count} artists</p><p className="preserve-lines">{detail.session.description}</p><div className="action-row"><button onClick={()=>setEditing(v=>!v)}>{editing?"Cancel editing":"Edit / End digging"}</button><button disabled={pending} onClick={()=>void remove()}>Delete session</button></div>{editing&&<DiggingForm session={detail.session} onSaved={()=>{setEditing(false);setAttempt(a=>a+1);}}/>}<section className="archive-section"><h2>Tracks</h2><ArchiveTracks tracks={detail.tracks} onOpen={setTrack}/>{detail.tracks.map(t=><button className="secondary-button" disabled={pending} key={t.id} onClick={()=>void assignment(t.id,false)}>Remove {t.title}</button>)}{!detail.tracks.length&&<p>No tracks yet. Search your library below.</p>}</section><section className="archive-section"><h2>Add Tracks</h2><label>Search library<input type="search" value={search} onChange={e=>setSearch(e.target.value)}/></label>{searchError&&<p role="alert">{searchError}<button onClick={()=>setAttempt(a=>a+1)}>Retry search</button></p>}<div className="track-picker">{matches.filter(t=>!detail.tracks.some(a=>a.id===t.id)).slice(0,50).map(t=><button disabled={pending} key={t.id} onClick={()=>void assignment(t.id,true)}>+ {t.title} — {t.artist_names}</button>)}</div></section></>}</section>{track!==null&&<TrackDetail trackId={track} onBack={()=>setTrack(null)} backLabel={detail?.session.name||"Digging"}/>}</>;
}
