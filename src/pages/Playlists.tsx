import { invoke } from "@tauri-apps/api/core";
import { useEffect, useState } from "react";
import Artwork from "../components/Artwork";
import PlaylistDetail from "./PlaylistDetail";
export interface Playlist { id:number; name:string; image_url:string|null; track_count:number; spotify_description:string|null; last_synced_at:string|null; last_error:string|null }
export default function Playlists() {
  const [rows,setRows]=useState<Playlist[]>([]);
  const [selected,setSelected]=useState<number|null>(null);
  const [loading,setLoading]=useState(true);
  const [error,setError]=useState("");
  const [syncing,setSyncing]=useState(false);
  const [message,setMessage]=useState("");
  const [attempt,setAttempt]=useState(0);
  useEffect(()=>{let active=true;setLoading(true);setError("");invoke<Playlist[]>("archived_playlists").then(r=>{if(active)setRows(r);}).catch(e=>{if(active)setError(String(e));}).finally(()=>{if(active)setLoading(false);});return()=>{active=false;};},[attempt]);
  async function sync(){setSyncing(true);setMessage("");try{const r=await invoke<{synced:number;unchanged:number;unavailable:number}>("spotify_sync_playlists");setMessage(`${r.synced} synced · ${r.unchanged} unchanged · ${r.unavailable} unavailable`);}catch(e){setMessage(`${String(e)} Your local archive remains available.`);}finally{setSyncing(false);setAttempt(a=>a+1);}}
  return <><section className="page" hidden={selected!==null}><h1>Playlists</h1><p className="muted">Spotify collections, with your own descriptions and tags.</p><button disabled={syncing} onClick={()=>void sync()}>{syncing?"Syncing playlists…":"Sync playlists"}</button><p className="muted">If playlist permission is missing, reconnect Spotify in Settings.</p>{message&&<p role="status">{message}</p>}{loading&&<p role="status">Loading playlists…</p>}{error&&<p role="alert">{error} <button onClick={()=>setAttempt(a=>a+1)}>Retry</button></p>}{!loading&&!error&&!rows.length&&<p>No playlists archived yet. Connect Spotify in Settings, then sync playlists.</p>}<div className="collection-grid">{rows.map(p=><button className="collection-card" key={p.id} onClick={()=>setSelected(p.id)}><Artwork className="collection-cover" url={p.image_url}/><strong>{p.name}</strong><span>{p.track_count} Spotify items</span>{p.last_error&&<small>Last sync incomplete · saved archive available</small>}</button>)}</div></section>{selected!==null&&<PlaylistDetail playlistId={selected} onBack={()=>setSelected(null)}/>}</>;
}
