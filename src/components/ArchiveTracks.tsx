import Artwork from "./Artwork";
export interface ArchiveTrack { id: number; title: string; artist_names: string; album_art_url: string | null; play_count?: number; discovered?: boolean }
export default function ArchiveTracks({ tracks, onOpen }: { tracks: ArchiveTrack[]; onOpen: (id: number) => void }) {
  return <ul className="archive-tracks">{tracks.map((track, index) => <li key={`${track.id}-${index}`}><button className="archive-track" onClick={() => onOpen(track.id)}><Artwork className="archive-cover" url={track.album_art_url} /><span><strong>{track.title}</strong><span>{track.artist_names || "Unknown artist"}</span></span>{!!track.play_count && <small>{track.play_count} plays</small>}</button></li>)}</ul>;
}
