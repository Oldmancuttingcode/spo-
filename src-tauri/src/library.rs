use crate::database::Database;
use rusqlite::Connection;
use serde::Serialize;
use tauri::State;

#[derive(Debug, Serialize)]
pub struct LibraryTrack {
    id: i64,
    spotify_id: Option<String>,
    title: String,
    artist_names: String,
    album_name: Option<String>,
    album_art_url: Option<String>,
    discovered_at: Option<String>,
    last_played_at: Option<String>,
    play_count: i64,
}

#[tauri::command]
pub fn library_tracks(
    database: State<'_, Database>,
    search: Option<String>,
) -> Result<Vec<LibraryTrack>, String> {
    database.with_connection(|connection| {
        query_tracks(connection, search.as_deref()).map_err(|_| {
            "Could not load your music library. Your archive data has not been changed.".into()
        })
    })
}

fn query_tracks(
    connection: &Connection,
    search: Option<&str>,
) -> rusqlite::Result<Vec<LibraryTrack>> {
    // Aggregate each relation before joining to avoid multiplied plays and rows.
    let mut statement = connection.prepare(
        "WITH plays AS (
            SELECT track_id, MIN(played_at) AS discovered_at,
                   MAX(played_at) AS last_played_at, COUNT(*) AS play_count
            FROM play_history GROUP BY track_id
        ), credits AS (
            SELECT ta.track_id,
                   group_concat(a.name, ', ' ORDER BY ta.artist_order) AS artist_names
            FROM track_artists ta JOIN artists a ON a.id = ta.artist_id
            GROUP BY ta.track_id
        )
        SELECT t.id, t.spotify_id, t.title, COALESCE(c.artist_names, ''),
               t.album_name, t.album_art_url, p.discovered_at, p.last_played_at,
               COALESCE(p.play_count, 0)
        FROM tracks t
        LEFT JOIN plays p ON p.track_id = t.id
        LEFT JOIN credits c ON c.track_id = t.id
        ORDER BY p.discovered_at DESC, t.title COLLATE NOCASE, t.id",
    )?;
    let rows = statement.query_map([], |row| {
        Ok(LibraryTrack {
            id: row.get(0)?,
            spotify_id: row.get(1)?,
            title: row.get(2)?,
            artist_names: row.get(3)?,
            album_name: row.get(4)?,
            album_art_url: row.get(5)?,
            discovered_at: row.get(6)?,
            last_played_at: row.get(7)?,
            play_count: row.get(8)?,
        })
    })?;
    // Small archive: Unicode lowercase substring matching, with literal wildcard characters.
    let needle = search.unwrap_or_default().trim().to_lowercase();
    rows.filter_map(|row| match row {
        Ok(track) => {
            let matches = track.title.to_lowercase().contains(&needle)
                || track.artist_names.to_lowercase().contains(&needle)
                || track
                    .album_name
                    .as_deref()
                    .unwrap_or_default()
                    .to_lowercase()
                    .contains(&needle);
            matches.then_some(Ok(track))
        }
        Err(error) => Some(Err(error)),
    })
    .collect()
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
            "INSERT INTO tracks (id, spotify_id, title, album_name, album_art_url, created_at) VALUES
             (1, 'one', 'Night Walk', 'Blue Room', 'https://example.com/art.jpg', '2026-09-16T00:00:00Z'),
             (2, 'two', 'Dawn', 'Morning', NULL, '2020-01-01T00:00:00Z'),
             (3, 'three', 'Unplayed', NULL, NULL, '2026-09-17T00:00:00Z'),
             (4, 'four', 'Dawn', NULL, NULL, '2026-09-17T00:00:00Z');
             INSERT INTO artists (id, spotify_id, name) VALUES (1, 'a', 'Alpha'), (2, 'z', 'ZÉRO');
             INSERT INTO track_artists (track_id, artist_id, artist_order) VALUES (1, 1, 1), (1, 2, 0);
             INSERT INTO play_history (track_id, played_at, synced_at) VALUES
             (1, '2026-09-16T10:00:00Z', '2026-09-16T11:00:00Z'),
             (1, '2026-09-10T10:00:00Z', '2026-09-16T11:00:00Z'),
             (2, '2026-09-15T10:00:00Z', '2026-09-16T11:00:00Z'),
             (4, '2026-09-15T10:00:00Z', '2026-09-16T11:00:00Z');"
        ).unwrap();
        connection
    }

    #[test]
    fn basic_library_preserves_metadata_and_ordered_artists_without_duplicates() {
        let rows = query_tracks(&fixture(), None).unwrap();
        assert_eq!(rows.len(), 4);
        let track = rows.iter().find(|row| row.id == 1).unwrap();
        assert_eq!(track.spotify_id.as_deref(), Some("one"));
        assert_eq!(track.title, "Night Walk");
        assert_eq!(track.artist_names, "ZÉRO, Alpha");
        assert_eq!(track.album_name.as_deref(), Some("Blue Room"));
        assert_eq!(
            track.album_art_url.as_deref(),
            Some("https://example.com/art.jpg")
        );
        assert_eq!(track.play_count, 2);
        assert_eq!(track.discovered_at.as_deref(), Some("2026-09-10T10:00:00Z"));
        assert_eq!(
            track.last_played_at.as_deref(),
            Some("2026-09-16T10:00:00Z")
        );
    }

    #[test]
    fn discovery_order_is_deterministic_and_unplayed_tracks_are_last() {
        let rows = query_tracks(&fixture(), None).unwrap();
        assert_eq!(
            rows.iter().map(|row| row.id).collect::<Vec<_>>(),
            [2, 4, 1, 3]
        );
        let unplayed = rows.last().unwrap();
        assert!(unplayed.discovered_at.is_none());
        assert!(unplayed.last_played_at.is_none());
        assert_eq!(unplayed.play_count, 0);
        assert_eq!(unplayed.artist_names, "");
    }

    #[test]
    fn search_matches_title_each_artist_and_album_case_insensitively() {
        let connection = fixture();
        for search in [" NIGHT ", "zéro", "ALPHA", "blue room"] {
            let rows = query_tracks(&connection, Some(search)).unwrap();
            assert_eq!(rows.len(), 1, "{search}");
            assert_eq!(rows[0].id, 1);
            assert_eq!(rows[0].artist_names, "ZÉRO, Alpha");
            assert_eq!(rows[0].play_count, 2);
        }
        for search in ["missing", "%", "_", "' OR 1=1 --"] {
            assert!(query_tracks(&connection, Some(search)).unwrap().is_empty());
        }
        assert_eq!(query_tracks(&connection, Some("  ")).unwrap().len(), 4);
    }

    #[test]
    fn empty_archive_and_query_failure_are_distinct() {
        let mut connection = Connection::open_in_memory().unwrap();
        assert!(query_tracks(&connection, None).is_err());
        crate::database::migrations::run(&mut connection, ":memory:".as_ref()).unwrap();
        assert!(query_tracks(&connection, None).unwrap().is_empty());
    }

    #[test]
    #[ignore = "reads the existing local archive without modifying it"]
    fn validate_local_library_read_only() {
        let path = std::path::PathBuf::from(std::env::var("APPDATA").unwrap())
            .join("com.local.musicarchive")
            .join("music-archive.db");
        let connection =
            Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
        let rows = query_tracks(&connection, None).unwrap();
        let count: usize = connection
            .query_row("SELECT COUNT(*) FROM tracks", [], |row| row.get(0))
            .unwrap();
        assert_eq!(rows.len(), count);
        let ids: std::collections::HashSet<_> = rows.iter().map(|row| row.id).collect();
        assert_eq!(ids.len(), count);
        println!(
            "Local Library: {} tracks; {} artwork URLs",
            count,
            rows.iter()
                .filter(|row| row.album_art_url.is_some())
                .count()
        );
        for track in rows.iter().take(3) {
            println!(
                "{} | {} | {:?} | {:?}",
                track.title, track.artist_names, track.album_name, track.discovered_at
            );
            for search in [
                Some(track.title.as_str()),
                Some(track.artist_names.as_str()),
                track.album_name.as_deref(),
            ]
            .into_iter()
            .flatten()
            {
                assert!(query_tracks(&connection, Some(search))
                    .unwrap()
                    .iter()
                    .any(|row| row.id == track.id));
            }
        }
        println!("Title, artist and album searches passed with SQLite only.");
    }
}
