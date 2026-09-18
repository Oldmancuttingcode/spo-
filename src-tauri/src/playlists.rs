use crate::{calendar::TrackRow, database::Database};
use rusqlite::{params, OptionalExtension};
use serde::Serialize;
use tauri::State;

#[derive(Serialize)]
pub struct Playlist {
    pub id: i64,
    pub name: String,
    image_url: Option<String>,
    track_count: i64,
    spotify_description: Option<String>,
    last_synced_at: Option<String>,
    last_error: Option<String>,
}
#[derive(Serialize)]
pub struct PlaylistTag {
    id: i64,
    name: String,
    category: String,
}
#[derive(Serialize)]
pub struct PlaylistDetail {
    playlist: Playlist,
    tracks: Vec<TrackRow>,
    personal_description: String,
    memo: String,
    tags: Vec<PlaylistTag>,
}
fn row(r: &rusqlite::Row<'_>) -> rusqlite::Result<Playlist> {
    Ok(Playlist {
        id: r.get(0)?,
        name: r.get(1)?,
        image_url: r.get(2)?,
        track_count: r.get(3)?,
        spotify_description: r.get(4)?,
        last_synced_at: r.get(5)?,
        last_error: r.get(6)?,
    })
}
const SELECT:&str="SELECT p.id,p.name,p.image_url,p.track_count,p.spotify_description,p.last_synced_at,s.last_error FROM playlists p LEFT JOIN sync_state s ON s.source='playlist:'||p.id";
#[tauri::command]
pub fn archived_playlists(database: State<'_, Database>) -> Result<Vec<Playlist>, String> {
    database.with_connection(|c| {
        let mut stmt = c
            .prepare(&format!("{SELECT} ORDER BY p.name COLLATE NOCASE,p.id"))
            .map_err(|_| "Could not load playlists.")?;
        let rows = stmt
            .query_map([], row)
            .map_err(|_| "Could not load playlists.")?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|_| "Could not load playlists.".into())
    })
}
#[tauri::command]
pub fn playlist_detail(
    database: State<'_, Database>,
    playlist_id: i64,
) -> Result<PlaylistDetail, String> {
    database.with_connection(|c|{
    let playlist=c.query_row(&format!("{SELECT} WHERE p.id=?1"),[playlist_id],row).optional().map_err(|_|"Could not load playlist.")?.ok_or("Playlist not found.")?;
    let (personal_description,memo)=c.query_row("SELECT COALESCE(personal_description,''),COALESCE(memo,'') FROM playlist_notes WHERE playlist_id=?1",[playlist_id],|r|Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?))).optional().map_err(|_|"Could not load playlist notes.")?.unwrap_or_default();
    let mut stmt=c.prepare("SELECT t.id,t.title,COALESCE((SELECT group_concat(a.name, ', ' ORDER BY ta.artist_order) FROM track_artists ta JOIN artists a ON a.id=ta.artist_id WHERE ta.track_id=t.id),''),t.album_art_url FROM playlist_tracks pt JOIN tracks t ON t.id=pt.track_id WHERE pt.playlist_id=?1 ORDER BY pt.position").map_err(|_|"Could not load playlist tracks.")?;
    let tracks=stmt.query_map([playlist_id],|r|Ok(TrackRow{id:r.get(0)?,title:r.get(1)?,artist_names:r.get(2)?,album_art_url:r.get(3)?,play_count:0,discovered:false})).map_err(|_|"Could not load playlist tracks.")?.collect::<Result<Vec<_>,_>>().map_err(|_|"Could not read playlist tracks.")?;
    let mut stmt=c.prepare("SELECT t.id,t.name,t.category FROM tags t JOIN playlist_tags pt ON pt.tag_id=t.id WHERE pt.playlist_id=?1 ORDER BY t.category,t.name").map_err(|_|"Could not load playlist tags.")?;
    let tags=stmt.query_map([playlist_id],|r|Ok(PlaylistTag{id:r.get(0)?,name:r.get(1)?,category:r.get(2)?})).map_err(|_|"Could not load playlist tags.")?.collect::<Result<Vec<_>,_>>().map_err(|_|"Could not read playlist tags.")?;
    Ok(PlaylistDetail{playlist,tracks,personal_description,memo,tags})
})
}
#[tauri::command]
pub fn save_playlist_notes(
    database: State<'_, Database>,
    playlist_id: i64,
    personal_description: String,
    memo: String,
) -> Result<(), String> {
    if personal_description.chars().count() > 50000 || memo.chars().count() > 50000 {
        return Err("Notes are limited to 50,000 characters.".into());
    }
    database.with_connection(|c|c.execute("INSERT INTO playlist_notes(playlist_id,personal_description,memo) VALUES(?1,?2,?3) ON CONFLICT(playlist_id) DO UPDATE SET personal_description=excluded.personal_description,memo=excluded.memo,updated_at=strftime('%Y-%m-%dT%H:%M:%fZ','now')",params![playlist_id,personal_description,memo]).map(|_|()).map_err(|_|"Could not save playlist notes.".into()))
}
#[tauri::command]
pub fn set_playlist_tag(
    database: State<'_, Database>,
    playlist_id: i64,
    tag_id: i64,
    assigned: bool,
) -> Result<(), String> {
    database.with_connection(|c| {
        c.execute(
            if assigned {
                "INSERT INTO playlist_tags(playlist_id,tag_id) VALUES(?1,?2) ON CONFLICT DO NOTHING"
            } else {
                "DELETE FROM playlist_tags WHERE playlist_id=?1 AND tag_id=?2"
            },
            params![playlist_id, tag_id],
        )
        .map(|_| ())
        .map_err(|_| "Could not update playlist tag.".into())
    })
}
#[tauri::command]
pub fn open_playlist_in_spotify(
    database: State<'_, Database>,
    playlist_id: i64,
) -> Result<(), String> {
    let id: String = database.with_connection(|c| {
        c.query_row(
            "SELECT spotify_id FROM playlists WHERE id=?1",
            [playlist_id],
            |r| r.get(0),
        )
        .map_err(|_| "Playlist not found.".into())
    })?;
    if id.is_empty() || !id.bytes().all(|b| b.is_ascii_alphanumeric()) {
        return Err("Invalid Spotify playlist ID.".into());
    }
    open::that(format!("https://open.spotify.com/playlist/{id}"))
        .map_err(|_| "Could not open Spotify.".into())
}
