use crate::database::Database;
use rusqlite::{Connection, OptionalExtension};
use serde::Serialize;
use tauri::State;
use url::Url;

#[derive(Debug, Serialize)]
pub struct TrackDetail {
    id: i64,
    spotify_id: Option<String>,
    title: String,
    album_name: Option<String>,
    album_art_url: Option<String>,
    duration_ms: Option<i64>,
    spotify_url: Option<String>,
    artists: Vec<String>,
    play_count: i64,
    discovered_at: Option<String>,
    last_played_at: Option<String>,
}

#[tauri::command]
pub fn track_detail(
    database: State<'_, Database>,
    track_id: i64,
) -> Result<Option<TrackDetail>, String> {
    database.with_connection(|connection| {
        query_track_detail(connection, track_id).map_err(|_| {
            "Could not load this track. Your archive data has not been changed.".into()
        })
    })
}

fn query_track_detail(
    connection: &Connection,
    track_id: i64,
) -> rusqlite::Result<Option<TrackDetail>> {
    // Artists are read separately so their number cannot multiply the play count.
    let detail = connection
        .query_row(
            "SELECT t.id, t.spotify_id, t.title, t.album_name, t.album_art_url,
                t.duration_ms, t.spotify_url, COUNT(p.id), MIN(p.played_at), MAX(p.played_at)
         FROM tracks t LEFT JOIN play_history p ON p.track_id = t.id
         WHERE t.id = ?1 GROUP BY t.id",
            [track_id],
            |row| {
                Ok(TrackDetail {
                    id: row.get(0)?,
                    spotify_id: row.get(1)?,
                    title: row.get(2)?,
                    album_name: row.get(3)?,
                    album_art_url: row.get(4)?,
                    duration_ms: row.get(5)?,
                    spotify_url: row.get(6)?,
                    artists: Vec::new(),
                    play_count: row.get(7)?,
                    discovered_at: row.get(8)?,
                    last_played_at: row.get(9)?,
                })
            },
        )
        .optional()?;
    let Some(mut detail) = detail else {
        return Ok(None);
    };
    let mut statement = connection.prepare(
        "SELECT a.name FROM track_artists ta JOIN artists a ON a.id = ta.artist_id
         WHERE ta.track_id = ?1 ORDER BY ta.artist_order",
    )?;
    detail.artists = statement
        .query_map([track_id], |row| row.get(0))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(Some(detail))
}

#[tauri::command]
pub fn open_track_in_spotify(database: State<'_, Database>, track_id: i64) -> Result<(), String> {
    // Accept an archive ID, never an arbitrary URL or shell command from React.
    let url = database.with_connection(|connection| stored_spotify_url(connection, track_id))?;
    open::that(url.as_str())
        .map_err(|_| "Could not open Spotify in your browser. Please try again.".into())
}

fn stored_spotify_url(connection: &Connection, track_id: i64) -> Result<Url, String> {
    let stored: Option<Option<String>> = connection
        .query_row(
            "SELECT spotify_url FROM tracks WHERE id = ?1",
            [track_id],
            |row| row.get(0),
        )
        .optional()
        .map_err(|_| "Could not read this track's Spotify link.".to_string())?;
    let url = stored
        .ok_or("This track was not found in your archive.")?
        .ok_or("This track has no Spotify link.")?;
    validate_spotify_url(&url).ok_or_else(|| "This track's Spotify link is not valid.".into())
}

fn validate_spotify_url(value: &str) -> Option<Url> {
    let url = Url::parse(value).ok()?;
    let segments: Vec<_> = url.path_segments()?.collect();
    (url.scheme() == "https"
        && url.host_str() == Some("open.spotify.com")
        && url.username().is_empty()
        && url.password().is_none()
        && url.port_or_known_default() == Some(443)
        && segments.len() == 2
        && segments[0] == "track"
        && !segments[1].is_empty()
        && segments[1].bytes().all(|byte| byte.is_ascii_alphanumeric()))
    .then_some(url)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Connection {
        let mut connection = Connection::open_in_memory().unwrap();
        connection
            .pragma_update(None, "foreign_keys", "ON")
            .unwrap();
        crate::database::migrations::run(&mut connection, ":memory:".as_ref()).unwrap();
        connection.execute_batch(
            "INSERT INTO tracks (id, spotify_id, title, album_name, album_art_url, duration_ms, spotify_url, created_at)
             VALUES (7, 'track7', 'Night Walk', 'Blue Room', 'https://example.com/art.jpg', 222000,
                     'https://open.spotify.com/track/track7', '2026-09-16T00:00:00Z');
             INSERT INTO tracks (id, spotify_id, title) VALUES (8, 'track8', 'Unplayed');
             INSERT INTO artists (id, spotify_id, name) VALUES (1, 'a', 'Alpha'), (2, 'z', 'Zéro');
             INSERT INTO track_artists (track_id, artist_id, artist_order) VALUES (7, 1, 1), (7, 2, 0);
             INSERT INTO play_history (track_id, played_at, synced_at) VALUES
             (7, '2026-09-16T10:00:00Z', '2026-09-16T11:00:00Z'),
             (7, '2026-09-10T10:00:00Z', '2026-09-16T11:00:00Z'),
             (7, '2026-09-12T10:00:00Z', '2026-09-16T11:00:00Z');"
        ).unwrap();
        connection
    }

    #[test]
    fn detail_returns_metadata_by_internal_id_and_ordered_artists() {
        let detail = query_track_detail(&fixture(), 7).unwrap().unwrap();
        assert_eq!(detail.id, 7);
        assert_eq!(detail.spotify_id.as_deref(), Some("track7"));
        assert_eq!(detail.title, "Night Walk");
        assert_eq!(detail.album_name.as_deref(), Some("Blue Room"));
        assert_eq!(
            detail.album_art_url.as_deref(),
            Some("https://example.com/art.jpg")
        );
        assert_eq!(detail.duration_ms, Some(222000));
        assert_eq!(
            detail.spotify_url.as_deref(),
            Some("https://open.spotify.com/track/track7")
        );
        assert_eq!(detail.artists, ["Zéro", "Alpha"]);
    }

    #[test]
    fn single_artist_track_returns_the_same_listening_summary() {
        let connection = fixture();
        connection
            .execute(
                "DELETE FROM track_artists WHERE track_id = 7 AND artist_id = 1",
                [],
            )
            .unwrap();
        let detail = query_track_detail(&connection, 7).unwrap().unwrap();
        assert_eq!(detail.title, "Night Walk");
        assert_eq!(detail.artists, ["Zéro"]);
        assert_eq!(detail.play_count, 3);
        assert_eq!(
            detail.discovered_at.as_deref(),
            Some("2026-09-10T10:00:00Z")
        );
        assert_eq!(
            detail.last_played_at.as_deref(),
            Some("2026-09-16T10:00:00Z")
        );
    }

    #[test]
    fn plays_are_not_multiplied_by_artists_and_discovery_uses_history() {
        let detail = query_track_detail(&fixture(), 7).unwrap().unwrap();
        assert_eq!(detail.play_count, 3);
        assert_eq!(
            detail.discovered_at.as_deref(),
            Some("2026-09-10T10:00:00Z")
        );
        assert_eq!(
            detail.last_played_at.as_deref(),
            Some("2026-09-16T10:00:00Z")
        );
    }

    #[test]
    fn unplayed_track_preserves_missing_metadata_and_has_zero_plays() {
        let detail = query_track_detail(&fixture(), 8).unwrap().unwrap();
        assert_eq!(detail.play_count, 0);
        assert!(detail.discovered_at.is_none());
        assert!(detail.last_played_at.is_none());
        assert!(detail.album_name.is_none());
        assert!(detail.album_art_url.is_none());
        assert!(detail.duration_ms.is_none());
        assert!(detail.spotify_url.is_none());
        assert!(detail.artists.is_empty());
    }

    #[test]
    fn unknown_id_is_distinct_from_database_failure() {
        let connection = fixture();
        for id in [0, -1, 999] {
            assert!(query_track_detail(&connection, id).unwrap().is_none());
        }
        assert!(query_track_detail(&Connection::open_in_memory().unwrap(), 7).is_err());
    }

    #[test]
    fn external_link_requires_a_stored_spotify_https_track_url() {
        let connection = fixture();
        assert_eq!(
            stored_spotify_url(&connection, 7).unwrap().as_str(),
            "https://open.spotify.com/track/track7"
        );
        assert!(stored_spotify_url(&connection, 8).is_err());
        assert!(stored_spotify_url(&connection, 999).is_err());
        assert!(validate_spotify_url("https://open.spotify.com/track/abc123?si=test").is_some());
        for url in [
            "",
            "file:///C:/Windows/system32/cmd.exe",
            "javascript:alert(1)",
            "http://open.spotify.com/track/abc",
            "https://open.spotify.com.evil.test/track/abc",
            "https://open.spotify.com@evil.test/track/abc",
            "https://user@open.spotify.com/track/abc",
            "https://open.spotify.com:444/track/abc",
            "https://open.spotify.com/track/",
            "https://open.spotify.com/playlist/abc",
            "https://open.spotify.com/track/abc/extra",
        ] {
            assert!(validate_spotify_url(url).is_none(), "{url}");
        }
        connection
            .execute(
                "UPDATE tracks SET spotify_url = 'file:///bad' WHERE id = 7",
                [],
            )
            .unwrap();
        assert!(stored_spotify_url(&connection, 7).is_err());
    }
}
