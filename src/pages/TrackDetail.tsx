import { invoke } from "@tauri-apps/api/core";
import { useEffect, useRef, useState } from "react";
import Artwork from "../components/Artwork";
import ArchiveDate from "../components/ArchiveDate";
import TrackTags from "../components/TrackTags";
import TrackNote from "../components/TrackNote";
import { canLeave } from "../unsaved";
import "./TrackDetail.css";

interface TrackDetailData {
  id: number;
  spotify_id: string | null;
  title: string;
  album_name: string | null;
  album_art_url: string | null;
  duration_ms: number | null;
  spotify_url: string | null;
  artists: string[];
  play_count: number;
  discovered_at: string | null;
  last_played_at: string | null;
}

function durationLabel(milliseconds: number) {
  const seconds = Math.floor(milliseconds / 1000);
  return `${Math.floor(seconds / 60)}:${String(seconds % 60).padStart(2, "0")}`;
}

export default function TrackDetail({ trackId, onBack, backLabel = "Library" }: { trackId: number; onBack: () => void; backLabel?: string }) {
  const [track, setTrack] = useState<TrackDetailData | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState(false);
  const [attempt, setAttempt] = useState(0);
  const [opening, setOpening] = useState(false);
  const [openError, setOpenError] = useState<string | null>(null);
  const backButton = useRef<HTMLButtonElement>(null);

  useEffect(() => { backButton.current?.focus({ preventScroll: true }); }, []);

  useEffect(() => {
    let active = true;
    setLoading(true);
    setError(false);
    invoke<TrackDetailData | null>("track_detail", { trackId })
      .then((result) => { if (active) setTrack(result); })
      .catch(() => { if (active) setError(true); })
      .finally(() => { if (active) setLoading(false); });
    return () => { active = false; };
  }, [trackId, attempt]);

  async function openSpotify() {
    setOpening(true);
    setOpenError(null);
    try {
      await invoke("open_track_in_spotify", { trackId });
    } catch (error) {
      setOpenError(typeof error === "string" ? error : "Could not open Spotify. Please try again.");
    } finally {
      setOpening(false);
    }
  }

  return (
    <section className="page track-detail-page" aria-label="Track detail">
      <button ref={backButton} type="button" className="secondary-button track-back" onClick={() => { if (canLeave()) onBack(); }}>← {backLabel}</button>
      {loading && <p role="status">Loading track…</p>}
      {!loading && error && <div role="alert" className="track-detail-message">
        <p>Could not load this track.</p>
        <p>Your archive data has not been changed.</p>
        <button type="button" className="secondary-button" onClick={() => setAttempt((value) => value + 1)}>Retry</button>
      </div>}
      {!loading && !error && !track && <p role="status">Track not found.</p>}
      {!loading && !error && track && <>
        <header className="track-detail-header">
          <Artwork className="track-detail-artwork" url={track.album_art_url} />
          <div className="track-detail-metadata">
            <p className="track-detail-label">Track</p>
            <h1>{track.title}</h1>
            <p className="track-detail-artists">{track.artists.join(", ") || "Unknown artist"}</p>
            <p className="track-detail-album">{track.album_name || "Unknown album"}</p>
            {track.duration_ms !== null && <p className="track-detail-duration">{durationLabel(track.duration_ms)}</p>}
          </div>
        </header>
        <section className="track-listening" aria-labelledby="track-listening-heading">
          <h2 id="track-listening-heading">Listening history</h2>
          <dl>
            <div><dt>Play Count</dt><dd>{track.play_count} {track.play_count === 1 ? "play" : "plays"}</dd></div>
            <div><dt>First Discovered</dt><dd><ArchiveDate value={track.discovered_at} emptyText="Not yet discovered" /></dd></div>
            <div><dt>Last Played</dt><dd><ArchiveDate value={track.last_played_at} emptyText="Not yet played" /></dd></div>
          </dl>
        </section>
        <TrackTags key={track.id} trackId={track.id} />
        <TrackNote key={`note-${track.id}`} trackId={track.id} />
        {track.spotify_url && <div className="track-spotify-link">
          <button type="button" className="secondary-button" disabled={opening} onClick={openSpotify}>{opening ? "Opening Spotify…" : "Open in Spotify"}</button>
          {openError && <p role="alert">{openError}</p>}
        </div>}
      </>}
    </section>
  );
}
