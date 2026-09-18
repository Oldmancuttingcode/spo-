use super::{
    api::{RecentlyPlayedTrack, SpotifyTrack},
    sync::import_track,
    SpotifyAuth,
};
use crate::database::Database;
use reqwest::blocking::Client;
use rusqlite::{params, Connection, OptionalExtension};
use serde::Serialize;
use serde_json::Value;
use std::time::Duration;
use tauri::Manager;

fn get(client: &Client, token: &str, path: &str) -> Result<Value, String> {
    let response = client
        .get(format!("https://api.spotify.com/v1/{path}"))
        .bearer_auth(token)
        .send()
        .map_err(|_| "Spotify network request failed. Your archive is still available.")?;
    match response.status().as_u16() {
        200 => response.json().map_err(|_| "Spotify returned an unreadable response.".into()),
        401 => Err("Spotify authentication expired. Reconnect in Settings.".into()),
        403 => Err("Spotify denied access. Reconnect for playlist permission; only owned or collaborative playlist items may be available.".into()),
        429 => Err(format!("Spotify rate limit. Retry after {} seconds.",response.headers().get("Retry-After").and_then(|v|v.to_str().ok()).filter(|v|v.bytes().all(|b|b.is_ascii_digit())).unwrap_or("60"))),
        status => Err(format!("Spotify returned HTTP {status}. Your archive is unchanged.")),
    }
}
fn all_items(client: &Client, token: &str, path: &str) -> Result<Vec<Value>, String> {
    let mut result = Vec::new();
    for page in 0..2001 {
        let body = get(
            client,
            token,
            &format!("{path}?limit=50&offset={}", page * 50),
        )?;
        let items = body["items"]
            .as_array()
            .ok_or("Spotify playlist page has no items.")?;
        result.extend(items.iter().cloned());
        if body["next"].is_null() {
            return Ok(result);
        }
        if items.len() != 50 {
            return Err("Spotify returned incomplete pagination. Retry later.".into());
        }
    }
    Err("Playlist exceeds the supported pagination limit.".into())
}
fn text<'a>(v: &'a Value, key: &str) -> Option<&'a str> {
    v.get(key).and_then(Value::as_str)
}
fn valid_id(v: &str) -> bool {
    !v.is_empty() && v.bytes().all(|b| b.is_ascii_alphanumeric())
}

pub(crate) fn persist(
    c: &mut Connection,
    p: &Value,
    items: Option<&[Value]>,
    owner: &str,
    error: Option<&str>,
) -> Result<(), String> {
    let sid = text(p, "id")
        .filter(|s| valid_id(s))
        .ok_or("Invalid playlist ID.")?;
    let name = text(p, "name").ok_or("Missing playlist name.")?;
    let snapshot = text(p, "snapshot_id");
    let tx = c
        .transaction()
        .map_err(|_| "Could not start playlist save.")?;
    let old: Option<(i64, Option<String>)> = tx
        .query_row(
            "SELECT id,snapshot_id FROM playlists WHERE spotify_id=?1",
            [sid],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()
        .map_err(|_| "Could not read playlist.")?;
    tx.execute("INSERT INTO playlists(spotify_id,name,spotify_description,image_url,owner_spotify_id,spotify_url,is_owned,is_collaborative,track_count) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9) ON CONFLICT(spotify_id) DO UPDATE SET name=excluded.name,spotify_description=excluded.spotify_description,image_url=excluded.image_url,owner_spotify_id=excluded.owner_spotify_id,spotify_url=excluded.spotify_url,is_owned=excluded.is_owned,is_collaborative=excluded.is_collaborative,track_count=excluded.track_count",params![sid,name,text(p,"description"),p["images"][0]["url"].as_str(),p["owner"]["id"].as_str(),p["external_urls"]["spotify"].as_str(),p["owner"]["id"].as_str()==Some(owner),p["collaborative"].as_bool().unwrap_or(false),p["items"]["total"].as_i64().or(p["tracks"]["total"].as_i64()).unwrap_or(0)]).map_err(|_|"Could not save playlist metadata.")?;
    let id: i64 = tx
        .query_row("SELECT id FROM playlists WHERE spotify_id=?1", [sid], |r| {
            r.get(0)
        })
        .map_err(|_| "Could not read playlist ID.")?;
    if let Some(items) = items {
        if let Some((_, old_snapshot)) = old {
            tx.execute("INSERT OR IGNORE INTO playlist_track_history(playlist_id,snapshot_id,position,track_id,added_at) SELECT playlist_id,?2,position,track_id,added_at FROM playlist_tracks WHERE playlist_id=?1",params![id,old_snapshot.unwrap_or_else(||"legacy".into())]).map_err(|_|"Could not preserve playlist history.")?;
        }
        tx.execute("DELETE FROM playlist_tracks WHERE playlist_id=?1", [id])
            .map_err(|_| "Could not refresh playlist tracks.")?;
        for (position, item) in items.iter().enumerate() {
            let raw = if item["item"].is_object() {
                &item["item"]
            } else {
                &item["track"]
            };
            if raw.is_null()
                || raw["type"].as_str() != Some("track")
                || raw["is_local"].as_bool() == Some(true)
                || item["is_local"].as_bool() == Some(true)
                || raw["id"].as_str().is_none()
            {
                continue;
            }
            let parsed: SpotifyTrack = serde_json::from_value(raw.clone())
                .map_err(|_| "Invalid playlist track; previous snapshot preserved.")?;
            let track: RecentlyPlayedTrack = parsed.into();
            if track.spotify_id.is_none() {
                continue;
            }
            let track_id = import_track(&tx, track)?;
            tx.execute("INSERT INTO playlist_tracks(playlist_id,track_id,position,added_at) VALUES(?1,?2,?3,?4)",params![id,track_id,position as i64,text(item,"added_at")]).map_err(|_|"Could not save playlist track.")?;
        }
        tx.execute("UPDATE playlists SET snapshot_id=?2,last_synced_at=strftime('%Y-%m-%dT%H:%M:%fZ','now') WHERE id=?1",params![id,snapshot]).map_err(|_|"Could not save playlist snapshot.")?;
    }
    tx.execute("INSERT INTO sync_state(source,sync_status,last_error,last_successful_sync_at) VALUES(?1,?2,?3,CASE WHEN ?3 IS NULL THEN strftime('%Y-%m-%dT%H:%M:%fZ','now') END) ON CONFLICT(source) DO UPDATE SET sync_status=excluded.sync_status,last_error=excluded.last_error,last_successful_sync_at=COALESCE(excluded.last_successful_sync_at,sync_state.last_successful_sync_at)",params![format!("playlist:{id}"),if error.is_some(){"failed"}else{"success"},error]).map_err(|_|"Could not save playlist sync state.")?;
    tx.commit()
        .map_err(|_| "Could not commit playlist archive.".into())
}

#[derive(Serialize)]
pub struct PlaylistSyncResult {
    synced: usize,
    unchanged: usize,
    unavailable: usize,
}
#[tauri::command]
pub async fn spotify_sync_playlists(app: tauri::AppHandle) -> Result<PlaylistSyncResult, String> {
    tauri::async_runtime::spawn_blocking(move || sync_playlists(&app))
        .await
        .map_err(|_| "Playlist sync stopped unexpectedly.".to_string())?
}
fn sync_playlists(app: &tauri::AppHandle) -> Result<PlaylistSyncResult, String> {
    static SYNC: std::sync::Mutex<()> = std::sync::Mutex::new(());
    let _guard = SYNC
        .try_lock()
        .map_err(|_| "Playlist sync is already running.".to_string())?;
    let spotify_auth = app.state::<SpotifyAuth>();
    let database = app.state::<Database>();
    let token = spotify_auth.valid_access_token()?;
    let client = Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(|_| "Could not initialize Spotify request.")?;
    let me = get(&client, &token, "me")?;
    let owner = text(&me, "id").ok_or("Spotify profile has no ID.")?;
    let playlists = all_items(&client, &token, "me/playlists")?;
    let mut result = PlaylistSyncResult {
        synced: 0,
        unchanged: 0,
        unavailable: 0,
    };
    for p in playlists {
        let sid = text(&p, "id")
            .filter(|s| valid_id(s))
            .ok_or("Invalid playlist ID.")?;
        let snapshot = text(&p, "snapshot_id");
        let unchanged=database.with_connection(|c| c.query_row("SELECT snapshot_id FROM playlists WHERE spotify_id=?1 AND last_synced_at IS NOT NULL",[sid],|r|r.get::<_,Option<String>>(0)).optional().map(|old|snapshot.is_some()&&old.flatten().as_deref()==snapshot).map_err(|_|"Could not read playlist snapshot.".into()))?;
        if unchanged {
            database.with_connection(|c| persist(c, &p, None, owner, None))?;
            result.unchanged += 1;
            continue;
        }
        match all_items(&client, &token, &format!("playlists/{sid}/items")) {
            Ok(items) => {
                // Recheck snapshot after pagination to avoid accepting a mixed version.
                let latest = get(&client, &token, &format!("playlists/{sid}"))?;
                if text(&latest, "snapshot_id") != snapshot {
                    database.with_connection(|c| {
                        persist(
                            c,
                            &p,
                            None,
                            owner,
                            Some("Playlist changed during sync. Retry later."),
                        )
                    })?;
                    result.unavailable += 1;
                } else {
                    database.with_connection(|c| persist(c, &p, Some(&items), owner, None))?;
                    result.synced += 1;
                }
            }
            Err(e) => {
                database.with_connection(|c| persist(c, &p, None, owner, Some(&e)))?;
                result.unavailable += 1;
                if e.contains("rate limit") || e.contains("authentication") || e.contains("network")
                {
                    return Err(e);
                }
            }
        }
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn malformed_track_rolls_back_metadata_and_snapshot() {
        let mut c = Connection::open_in_memory().unwrap();
        crate::database::migrations::run(&mut c, ":memory:".as_ref()).unwrap();
        let first = json!({"id":"abc","name":"Before","snapshot_id":"v1"});
        persist(&mut c, &first, Some(&[]), "me", None).unwrap();
        let next = json!({"id":"abc","name":"After","snapshot_id":"v2"});
        let malformed = json!({"item":{"id":"song","type":"track"}});
        assert!(persist(&mut c, &next, Some(&[malformed]), "me", None).is_err());
        let saved: (String, String) = c
            .query_row("SELECT name,snapshot_id FROM playlists", [], |r| {
                Ok((r.get(0)?, r.get(1)?))
            })
            .unwrap();
        assert_eq!(saved, ("Before".into(), "v1".into()));
    }
    #[test]
    fn snapshot_refresh_preserves_personal_data_old_membership_and_repeated_tracks() {
        let mut c = Connection::open_in_memory().unwrap();
        crate::database::migrations::run(&mut c, ":memory:".as_ref()).unwrap();
        let mut p = json!({"id":"abc","name":"List","snapshot_id":"v1","owner":{"id":"me"},"items":{"total":2}});
        let item = json!({"item":{"id":"track1","type":"track","name":"Song","duration_ms":2000,"artists":[],"is_local":false}});
        persist(&mut c, &p, Some(&[item.clone(), item]), "me", None).unwrap();
        assert_eq!(
            c.query_row("SELECT COUNT(*) FROM playlist_tracks", [], |r| r
                .get::<_, i64>(0))
                .unwrap(),
            2
        );
        c.execute_batch(
            "INSERT INTO playlist_notes(playlist_id,personal_description) VALUES(1,'Mine');",
        )
        .unwrap();
        p["snapshot_id"] = json!("v2");
        persist(&mut c, &p, None, "me", Some("Forbidden")).unwrap();
        assert_eq!(
            c.query_row("SELECT snapshot_id FROM playlists", [], |r| r
                .get::<_, String>(0))
                .unwrap(),
            "v1"
        );
        persist(&mut c, &p, Some(&[]), "me", None).unwrap();
        assert_eq!(
            c.query_row("SELECT COUNT(*) FROM playlist_track_history", [], |r| r
                .get::<_, i64>(0))
                .unwrap(),
            2
        );
        assert_eq!(
            c.query_row("SELECT personal_description FROM playlist_notes", [], |r| r
                .get::<_, String>(0))
                .unwrap(),
            "Mine"
        );
        assert_eq!(
            c.query_row("SELECT COUNT(*) FROM play_history", [], |r| r
                .get::<_, i64>(0))
                .unwrap(),
            0
        );
    }
}
