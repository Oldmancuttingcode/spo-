import { invoke } from "@tauri-apps/api/core";
import { useEffect, useState } from "react";

export default function TrackNote({ trackId }: { trackId: number }) {
  const [note, setNote] = useState("");
  const [saved, setSaved] = useState("");
  const [loading, setLoading] = useState(true);
  const [pending, setPending] = useState(false);
  const [error, setError] = useState("");
  const [attempt, setAttempt] = useState(0);
  const [loaded, setLoaded] = useState(false);
  const [message, setMessage] = useState("");
  useEffect(() => {
    let active = true;
    setLoading(true); setError("");
    invoke<string>("track_note", { trackId }).then(value => {
      if (active) { setNote(value); setSaved(value); setLoaded(true); }
    }).catch(e => { if (active) setError(String(e)); }).finally(() => { if (active) setLoading(false); });
    return () => { active = false; };
  }, [trackId, attempt]);
  async function save(value: string) {
    setPending(true); setError(""); setMessage("");
    try { await invoke("save_track_note", { trackId, note: value }); setNote(value); setSaved(value); setMessage(value.trim() ? "Note saved." : "Note deleted."); }
    catch (e) { setError(String(e)); } finally { setPending(false); }
  }
  return <section className="archive-section" aria-label="My note" data-unsaved={note !== saved}>
    <h2>My Note</h2>
    {loading && <p role="status">Loading note…</p>}
    {error && <p role="alert">{error} {!loaded && <button onClick={() => setAttempt(a => a + 1)}>Retry note</button>}</p>}
    {loaded && <><label htmlFor="track-note">What do you notice about this track?</label>
      <textarea id="track-note" rows={5} maxLength={50000} disabled={pending} value={note} onChange={e => { setNote(e.target.value); setMessage(""); }} />
      <div className="action-row"><button disabled={pending || note === saved} onClick={() => void save(note)}>{pending ? "Saving…" : "Save note"}</button>
      <button className="secondary-button" disabled={pending || !saved} onClick={() => { if (window.confirm("Delete this track's note?")) void save(""); }}>Delete note</button>
      {note !== saved && <span>Unsaved changes — save before leaving.</span>}</div>
      {message && <p role="status">{message}</p>}</>}
  </section>;
}
