import {invoke} from "@tauri-apps/api/core";
import {useEffect,useState} from "react";
interface Status {source:string;last_successful_sync_at:string|null;sync_status:string;last_error:string|null}
export default function SyncStatus({refresh}:{refresh:boolean}){
  const [rows,setRows]=useState<Status[]>([]);
  const [error,setError]=useState("");
  const [attempt,setAttempt]=useState(0);
  useEffect(()=>{let active=true;setError("");invoke<Status[]>("archive_sync_status").then(s=>{if(active)setRows(s);}).catch(e=>{if(active)setError(String(e));});return()=>{active=false;};},[refresh,attempt]);
  const history=rows.find(r=>r.source==="recently_played");
  return <section className="archive-section"><h2>Archive sync</h2>{error&&<p role="alert">{error}<button onClick={()=>setAttempt(a=>a+1)}>Retry status</button></p>}<p>Last listening sync: {history?.last_successful_sync_at?new Date(history.last_successful_sync_at).toLocaleString():"Not synced yet"}</p>{history?.last_error&&<p role="alert">{history.last_error}</p>}<p className="muted">Your local tracks, notes, tags, playlists, and digging stay available when Spotify is offline.</p><p className="muted">Playlist access requires reconnecting Spotify to grant playlist-read-private. Sync playlists from Playlists.</p></section>;
}
