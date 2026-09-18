import { invoke } from "@tauri-apps/api/core";
import { useEffect, useState } from "react";
import type { Playlist } from "./Playlists";
import Artwork from "../components/Artwork";
import ArchiveTracks, {type ArchiveTrack} from "../components/ArchiveTracks";
import TrackDetail from "./TrackDetail";
interface Tag {id:number;name:string;category:string}
interface Detail {playlist:Playlist;tracks:ArchiveTrack[];personal_description:string;memo:string;tags:Tag[]}
export default function PlaylistDetail({playlistId,onBack}:{playlistId:number;onBack:()=>void}){
  const [detail,setDetail]=useState<Detail|null>(null);
  const [error,setError]=useState("");
  const [attempt,setAttempt]=useState(0);
  useEffect(()=>{let active=true;setError("");invoke<Detail>("playlist_detail",{playlistId}).then(d=>{if(active)setDetail(d);}).catch(e=>{if(active)setError(String(e));});return()=>{active=false;};},[playlistId,attempt]);
  return <><div hidden={!!detail}><button onClick={onBack}>← Playlists</button>{!error&&<p role="status">Loading playlist…</p>}</div>{error&&<p role="alert">{error}<button onClick={()=>setAttempt(a=>a+1)}>Retry</button></p>}{detail&&<Editor key={playlistId} detail={detail} onBack={onBack} onRefresh={()=>setAttempt(a=>a+1)}/>}</>;
}
function Editor({detail,onBack,onRefresh}:{detail:Detail;onBack:()=>void;onRefresh:()=>void}){
  const {playlist:p}=detail;
  const [description,setDescription]=useState(detail.personal_description);
  const [memo,setMemo]=useState(detail.memo);
  const [pending,setPending]=useState(false);
  const [error,setError]=useState("");
  const [message,setMessage]=useState("");
  const [track,setTrack]=useState<number|null>(null);
  const [available,setAvailable]=useState<Tag[]>([]);
  const [category,setCategory]=useState("mood");
  const [name,setName]=useState("");
  const [tagError,setTagError]=useState("");
  const [tagAttempt,setTagAttempt]=useState(0);
  useEffect(()=>{let active=true;setTagError("");invoke<Tag[]>("list_tags").then(t=>{if(active)setAvailable(t);}).catch(e=>{if(active)setTagError(String(e));});return()=>{active=false;};},[detail.tags,tagAttempt]);
  async function act(action:()=>Promise<unknown>,message:string){setPending(true);setError("");setMessage("");try{await action();setMessage(message);onRefresh();}catch(e){setError(String(e));}finally{setPending(false);}}
  async function add(){const tag=await invoke<Tag>("create_tag",{name,category});await invoke("set_playlist_tag",{playlistId:p.id,tagId:tag.id,assigned:true});setName("");}
  return <><section className="page" hidden={track!==null}><button onClick={onBack}>← Playlists</button><header className="collection-header"><Artwork className="collection-cover" url={p.image_url}/><div><h1>{p.name}</h1><p>{p.track_count} Spotify items · {detail.tracks.length} archived tracks</p><button onClick={()=>void act(()=>invoke("open_playlist_in_spotify",{playlistId:p.id}),"")}>Open in Spotify</button></div></header>{p.spotify_description&&<p>{p.spotify_description}</p>}{p.last_error&&<p role="alert">{p.last_error} Showing the last saved track list.</p>}{!p.last_synced_at&&<p>Track list not yet available from Spotify.</p>}{error&&<p role="alert">{error}</p>}{message&&<p role="status">{message}</p>}
    <section className="archive-section"><h2>My Description</h2><label htmlFor="playlist-description">Your interpretation of this collection</label><textarea id="playlist-description" rows={4} maxLength={50000} disabled={pending} value={description} onChange={e=>setDescription(e.target.value)}/><label htmlFor="playlist-memo">Memo</label><textarea id="playlist-memo" rows={3} maxLength={50000} disabled={pending} value={memo} onChange={e=>setMemo(e.target.value)}/><button disabled={pending} onClick={()=>void act(()=>invoke("save_playlist_notes",{playlistId:p.id,personalDescription:description,memo}),"Notes saved.")}>Save notes</button></section>
    <section className="archive-section"><h2>Tags</h2><div className="tag-chips">{detail.tags.map(t=><span className="tag-chip" key={t.id}>{t.category}: {t.name}<button disabled={pending} aria-label={`Remove ${t.name}`} onClick={()=>void act(()=>invoke("set_playlist_tag",{playlistId:p.id,tagId:t.id,assigned:false}),"Tag removed.")}>×</button></span>)}</div>{tagError&&<p role="alert">{tagError}<button onClick={()=>setTagAttempt(a=>a+1)}>Retry tags</button></p>}<form className="action-row" onSubmit={e=>{e.preventDefault();void act(add,"Tag added.");}}><label>Category<select value={category} disabled={pending} onChange={e=>setCategory(e.target.value)}>{["genre","mood","sound","vocal","free"].map(c=><option key={c}>{c}</option>)}</select></label><label>Find or create tag<input value={name} disabled={pending} onChange={e=>setName(e.target.value)}/></label><button disabled={pending||!name.trim()}>Add tag</button></form><div className="action-row">{available.filter(t=>t.category===category&&t.name.toLowerCase().includes(name.trim().toLowerCase())&&!detail.tags.some(a=>a.id===t.id)).map(t=><button disabled={pending} key={t.id} onClick={()=>void act(()=>invoke("set_playlist_tag",{playlistId:p.id,tagId:t.id,assigned:true}),"Tag added.")}>{t.name}</button>)}</div></section>
    <section className="archive-section"><h2>Tracks</h2><ArchiveTracks tracks={detail.tracks} onOpen={setTrack}/>{!detail.tracks.length&&<p>No archived tracks.</p>}</section></section>{track!==null&&<TrackDetail trackId={track} onBack={()=>setTrack(null)} backLabel={p.name}/>}</>;
}
