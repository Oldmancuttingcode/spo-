import { invoke } from "@tauri-apps/api/core";
import { useEffect, useState } from "react";
import "./Library.css";

interface LibraryTrack {
  id: number;
  spotify_id: string | null;
  title: string;
  artist_names: string;
  album_name: string | null;
  album_art_url: string | null;
  discovered_at: string | null;
  last_played_at: string | null;
  play_count: number;
}

const dateFormat = new Intl.DateTimeFormat(undefined, {
  year: "numeric", month: "short", day: "numeric",
});

function DiscoveryDate({ value }: { value: string | null }) {
  if (!value) return <>Not yet discovered</>;
  const date = new Date(value);
  return Number.isNaN(date.getTime()) ? <>Unknown date</> : (
    <time dateTime={value} title={date.toLocaleString()}>{dateFormat.format(date)}</time>
  );
}

function Artwork({ url }: { url: string | null }) {
  const [failed, setFailed] = useState(false);
  return url && !failed ? (
    <img className="library-artwork" src={url} alt="" loading="lazy" onError={() => setFailed(true)} />
  ) : (
    <span className="library-artwork library-artwork-placeholder" role="img" aria-label="No artwork">♪</span>
  );
}

export default function Library() {
  const [rows, setRows] = useState<LibraryTrack[]>([]);
  const [search, setSearch] = useState("");
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState(false);
  const [attempt, setAttempt] = useState(0);

  useEffect(() => {
    let active = true;
    setLoading(true);
    setError(false);
    const timer = window.setTimeout(() => {
      invoke<LibraryTrack[]>("library_tracks", { search })
        .then((tracks) => { if (active) setRows(tracks); })
        .catch(() => { if (active) setError(true); })
        .finally(() => { if (active) setLoading(false); });
    }, search ? 200 : 0);
    return () => { active = false; window.clearTimeout(timer); };
  }, [search, attempt]);

  return (
    <section className="page library-page">
      <h1>Library</h1>
      <label className="library-search">
        <span>Search your archive</span>
        <input type="search" placeholder="Search tracks, artists, or albums…" value={search}
          onChange={(event) => setSearch(event.target.value)} />
      </label>
      <div className="library-heading">
        <h2>Recently Discovered</h2>
        {!loading && !error && <span>{rows.length} {rows.length === 1 ? "track" : "tracks"}</span>}
      </div>
      {loading && <p role="status">Loading tracks…</p>}
      {error && (
        <div className="library-message" role="alert">
          <p>Could not load your music library.</p>
          <p>Your archive data has not been changed.{rows.length > 0 && " Showing the previous results."}</p>
          <button className="secondary-button" onClick={() => setAttempt((value) => value + 1)}>Retry</button>
        </div>
      )}
      {!loading && !error && rows.length === 0 && (
        <div className="library-message" role="status">
          {search.trim() ? <p>No tracks match your search.</p> : <>
            <p>Your music archive is empty.</p>
            <p>Connect Spotify and sync your listening history in Settings to start building your archive.</p>
          </>}
        </div>
      )}
      {rows.length > 0 && (
        <div className="library-table-scroll" aria-busy={loading}>
          <table className="library-table">
            <thead><tr><th scope="col">Artwork</th><th scope="col">Track</th><th scope="col">Artist</th><th scope="col">Album</th><th scope="col">Discovered</th></tr></thead>
            <tbody>{rows.map((track) => (
              <tr key={track.id}>
                <td><Artwork key={track.album_art_url} url={track.album_art_url} /></td>
                <td className="library-track-title">{track.title}</td>
                <td>{track.artist_names || "Unknown artist"}</td>
                <td>{track.album_name || "Unknown album"}</td>
                <td><DiscoveryDate value={track.discovered_at} /></td>
              </tr>
            ))}</tbody>
          </table>
        </div>
      )}
    </section>
  );
}
