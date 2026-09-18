use rusqlite::{params, Connection, OptionalExtension, Transaction};
use serde::Serialize;

use super::api::{RecentlyPlayedArtist, RecentlyPlayedItem, RecentlyPlayedResponse};

pub const RECENTLY_PLAYED_SYNC_SOURCE: &str = "recently_played";

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ListeningHistorySyncResult {
    pub processed_items: u32,
    pub new_play_history_rows: u32,
    pub new_track_rows: u32,
    pub updated_track_rows: u32,
    pub new_artist_rows: u32,
    pub skipped_items: u32,
    pub new_discoveries: u32,
    pub latest_processed_played_at: Option<String>,
    pub last_successful_sync_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyncCursor {
    pub last_processed_played_at: Option<String>,
    pub after_ms: Option<u64>,
}

#[derive(Debug, Default)]
struct SyncCounters {
    processed_items: u32,
    new_play_history_rows: u32,
    new_track_rows: u32,
    updated_track_rows: u32,
    new_artist_rows: u32,
    skipped_items: u32,
    new_discoveries: u32,
    latest_processed_played_at: Option<String>,
}

pub fn read_sync_cursor(connection: &Connection) -> Result<SyncCursor, String> {
    let last_processed_played_at = connection
        .query_row(
            "SELECT last_processed_played_at FROM sync_state WHERE source = ?1",
            params![RECENTLY_PLAYED_SYNC_SOURCE],
            |row| row.get::<_, Option<String>>(0),
        )
        .optional()
        .map_err(|error| format!("Failed to read Spotify sync state: {error}"))?
        .flatten();

    let after_ms = last_processed_played_at
        .as_deref()
        .map(|played_at| spotify_after_cursor_ms(connection, played_at))
        .transpose()?;

    Ok(SyncCursor {
        last_processed_played_at,
        after_ms,
    })
}

pub fn record_sync_failure(connection: &mut Connection, error_message: &str) -> Result<(), String> {
    connection
        .execute(
            "
            INSERT INTO sync_state (source, sync_status, last_error)
            VALUES (?1, 'failed', ?2)
            ON CONFLICT(source) DO UPDATE SET
                sync_status = 'failed',
                last_error = excluded.last_error
            ",
            params![RECENTLY_PLAYED_SYNC_SOURCE, error_message],
        )
        .map_err(|error| format!("Failed to record Spotify sync failure: {error}"))?;

    Ok(())
}

pub fn persist_recently_played(
    connection: &mut Connection,
    response: &RecentlyPlayedResponse,
) -> Result<ListeningHistorySyncResult, String> {
    let transaction = connection
        .transaction()
        .map_err(|error| format!("Failed to start Spotify sync transaction: {error}"))?;
    let mut counters = SyncCounters::default();

    for item in &response.items {
        persist_play_item(&transaction, item, &mut counters)?;
    }

    let last_successful_sync_at = sqlite_now(&transaction)?;
    update_successful_sync_state(&transaction, &counters, &last_successful_sync_at)?;

    transaction
        .commit()
        .map_err(|error| format!("Failed to commit Spotify sync transaction: {error}"))?;

    Ok(ListeningHistorySyncResult {
        processed_items: counters.processed_items,
        new_play_history_rows: counters.new_play_history_rows,
        new_track_rows: counters.new_track_rows,
        updated_track_rows: counters.updated_track_rows,
        new_artist_rows: counters.new_artist_rows,
        skipped_items: counters.skipped_items,
        new_discoveries: counters.new_discoveries,
        latest_processed_played_at: counters.latest_processed_played_at,
        last_successful_sync_at,
    })
}

pub(super) fn import_track(
    transaction: &Transaction<'_>,
    track: super::api::RecentlyPlayedTrack,
) -> Result<i64, String> {
    let artists = track
        .artists
        .iter()
        .filter(|a| a.spotify_id.is_some())
        .collect::<Vec<_>>();
    let artist_ids = artists
        .iter()
        .map(|a| upsert_artist(transaction, a).map(|r| r.id))
        .collect::<Result<Vec<_>, _>>()?;
    let item = RecentlyPlayedItem {
        track,
        played_at: String::new(),
        context: None,
    };
    let row = upsert_track(transaction, &item)?;
    replace_track_artists(transaction, row.id, &artist_ids)?;
    Ok(row.id)
}

fn persist_play_item(
    transaction: &Transaction<'_>,
    item: &RecentlyPlayedItem,
    counters: &mut SyncCounters,
) -> Result<(), String> {
    counters.processed_items += 1;

    let Some(track_spotify_id) = item.track.spotify_id.as_deref() else {
        counters.skipped_items += 1;
        return Ok(());
    };

    let valid_artists = item
        .track
        .artists
        .iter()
        .filter(|artist| artist.spotify_id.is_some())
        .collect::<Vec<_>>();
    let artist_ids = valid_artists
        .iter()
        .map(|artist| upsert_artist(transaction, artist))
        .collect::<Result<Vec<_>, _>>()?;
    counters.new_artist_rows += artist_ids
        .iter()
        .filter(|upsert| upsert.was_inserted)
        .count() as u32;

    let had_previous_play = track_has_any_play(transaction, track_spotify_id)?;
    let track_upsert = upsert_track(transaction, item)?;
    if track_upsert.was_inserted {
        counters.new_track_rows += 1;
    } else {
        counters.updated_track_rows += 1;
    }

    replace_track_artists(
        transaction,
        track_upsert.id,
        &artist_ids
            .iter()
            .map(|upsert| upsert.id)
            .collect::<Vec<_>>(),
    )?;

    let inserted_play = insert_play_history(transaction, track_upsert.id, item)?;
    if inserted_play {
        counters.new_play_history_rows += 1;
        if !had_previous_play {
            counters.new_discoveries += 1;
        }
    }

    counters.latest_processed_played_at = max_timestamp(
        counters.latest_processed_played_at.take(),
        Some(item.played_at.clone()),
    );

    Ok(())
}

#[derive(Debug, Clone, Copy)]
struct UpsertedRow {
    id: i64,
    was_inserted: bool,
}

fn upsert_artist(
    transaction: &Transaction<'_>,
    artist: &&RecentlyPlayedArtist,
) -> Result<UpsertedRow, String> {
    let spotify_id = artist
        .spotify_id
        .as_deref()
        .ok_or_else(|| "Artist upsert requires a Spotify artist ID.".to_string())?;
    let existing_id = find_id_by_spotify_id(transaction, "artists", spotify_id)?;

    transaction
        .execute(
            "
            INSERT INTO artists (spotify_id, name, spotify_url)
            VALUES (?1, ?2, ?3)
            ON CONFLICT(spotify_id) DO UPDATE SET
                name = excluded.name,
                spotify_url = excluded.spotify_url,
                updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
            ",
            params![spotify_id, artist.name, artist.spotify_url],
        )
        .map_err(|error| format!("Failed to upsert artist metadata: {error}"))?;

    Ok(UpsertedRow {
        id: existing_id.unwrap_or(
            find_id_by_spotify_id(transaction, "artists", spotify_id)?
                .ok_or_else(|| "Artist upsert did not return a row.".to_string())?,
        ),
        was_inserted: existing_id.is_none(),
    })
}

fn upsert_track(
    transaction: &Transaction<'_>,
    item: &RecentlyPlayedItem,
) -> Result<UpsertedRow, String> {
    let track = &item.track;
    let spotify_id = track
        .spotify_id
        .as_deref()
        .ok_or_else(|| "Track upsert requires a Spotify track ID.".to_string())?;
    let existing_id = find_id_by_spotify_id(transaction, "tracks", spotify_id)?;
    let album_name = track.album.as_ref().map(|album| album.name.as_str());
    let album_spotify_id = track
        .album
        .as_ref()
        .and_then(|album| album.spotify_id.as_deref());
    let album_art_url = track
        .album
        .as_ref()
        .and_then(|album| album.artwork_url.as_deref());

    transaction
        .execute(
            "
            INSERT INTO tracks (
                spotify_id,
                title,
                album_name,
                album_spotify_id,
                album_art_url,
                duration_ms,
                spotify_url
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
            ON CONFLICT(spotify_id) DO UPDATE SET
                title = excluded.title,
                album_name = excluded.album_name,
                album_spotify_id = excluded.album_spotify_id,
                album_art_url = excluded.album_art_url,
                duration_ms = excluded.duration_ms,
                spotify_url = excluded.spotify_url,
                updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
            ",
            params![
                spotify_id,
                track.title,
                album_name,
                album_spotify_id,
                album_art_url,
                track.duration_ms.map(i64::from),
                track.spotify_url
            ],
        )
        .map_err(|error| format!("Failed to upsert track metadata: {error}"))?;

    Ok(UpsertedRow {
        id: existing_id.unwrap_or(
            find_id_by_spotify_id(transaction, "tracks", spotify_id)?
                .ok_or_else(|| "Track upsert did not return a row.".to_string())?,
        ),
        was_inserted: existing_id.is_none(),
    })
}

fn replace_track_artists(
    transaction: &Transaction<'_>,
    track_id: i64,
    artist_ids: &[i64],
) -> Result<(), String> {
    transaction
        .execute(
            "DELETE FROM track_artists WHERE track_id = ?1",
            params![track_id],
        )
        .map_err(|error| format!("Failed to refresh track artist relations: {error}"))?;

    for (artist_order, artist_id) in artist_ids.iter().enumerate() {
        transaction
            .execute(
                "
                INSERT INTO track_artists (track_id, artist_id, artist_order)
                VALUES (?1, ?2, ?3)
                ",
                params![track_id, artist_id, artist_order as i64],
            )
            .map_err(|error| format!("Failed to store track artist relation: {error}"))?;
    }

    Ok(())
}

fn insert_play_history(
    transaction: &Transaction<'_>,
    track_id: i64,
    item: &RecentlyPlayedItem,
) -> Result<bool, String> {
    let context_type = item
        .context
        .as_ref()
        .and_then(|context| context.context_type.as_deref());
    let context_uri = item
        .context
        .as_ref()
        .and_then(|context| context.uri.as_deref());

    let inserted = transaction
        .execute(
            "
            INSERT OR IGNORE INTO play_history (
                track_id,
                played_at,
                context_type,
                context_uri,
                synced_at
            )
            VALUES (?1, ?2, ?3, ?4, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
            ",
            params![track_id, item.played_at, context_type, context_uri],
        )
        .map_err(|error| format!("Failed to insert play history: {error}"))?;

    Ok(inserted == 1)
}

fn update_successful_sync_state(
    transaction: &Transaction<'_>,
    counters: &SyncCounters,
    sync_completed_at: &str,
) -> Result<(), String> {
    transaction
        .execute(
            "
            INSERT INTO sync_state (
                source,
                last_successful_sync_at,
                last_processed_played_at,
                sync_status,
                last_error
            )
            VALUES (?1, ?2, ?3, 'success', NULL)
            ON CONFLICT(source) DO UPDATE SET
                last_successful_sync_at = excluded.last_successful_sync_at,
                last_processed_played_at = COALESCE(
                    excluded.last_processed_played_at,
                    sync_state.last_processed_played_at
                ),
                sync_status = 'success',
                last_error = NULL
            ",
            params![
                RECENTLY_PLAYED_SYNC_SOURCE,
                sync_completed_at,
                counters.latest_processed_played_at,
            ],
        )
        .map_err(|error| format!("Failed to update Spotify sync state: {error}"))?;

    Ok(())
}

fn track_has_any_play(transaction: &Transaction<'_>, spotify_id: &str) -> Result<bool, String> {
    let play_count = transaction
        .query_row(
            "
            SELECT COUNT(*)
            FROM play_history
            JOIN tracks ON tracks.id = play_history.track_id
            WHERE tracks.spotify_id = ?1
            ",
            params![spotify_id],
            |row| row.get::<_, i64>(0),
        )
        .map_err(|error| format!("Failed to inspect existing play history: {error}"))?;

    Ok(play_count > 0)
}

fn find_id_by_spotify_id(
    connection: &Connection,
    table_name: &str,
    spotify_id: &str,
) -> Result<Option<i64>, String> {
    let sql = format!("SELECT id FROM {table_name} WHERE spotify_id = ?1");
    connection
        .query_row(&sql, params![spotify_id], |row| row.get(0))
        .optional()
        .map_err(|error| format!("Failed to find {table_name} row by Spotify ID: {error}"))
}

fn sqlite_now(connection: &Connection) -> Result<String, String> {
    connection
        .query_row("SELECT strftime('%Y-%m-%dT%H:%M:%fZ', 'now')", [], |row| {
            row.get(0)
        })
        .map_err(|error| format!("Failed to read current SQLite timestamp: {error}"))
}

fn spotify_after_cursor_ms(connection: &Connection, played_at: &str) -> Result<u64, String> {
    let milliseconds = connection
        .query_row(
            "
            SELECT CAST(
                ROUND((julianday(?1) - julianday('1970-01-01T00:00:00Z')) * 86400000.0)
                AS INTEGER
            )
            ",
            params![played_at],
            |row| row.get::<_, i64>(0),
        )
        .map_err(|error| format!("Failed to convert Spotify sync cursor: {error}"))?;

    Ok(milliseconds.saturating_sub(1) as u64)
}

fn max_timestamp(left: Option<String>, right: Option<String>) -> Option<String> {
    match (left, right) {
        (Some(left), Some(right)) => Some(if left >= right { left } else { right }),
        (Some(left), None) => Some(left),
        (None, Some(right)) => Some(right),
        (None, None) => None,
    }
}

#[cfg(test)]
mod tests {
    use rusqlite::{params, Connection};

    use super::{persist_recently_played, read_sync_cursor};
    use crate::{
        database::migrations,
        spotify::api::{
            PlayContext, RecentlyPlayedAlbum, RecentlyPlayedArtist, RecentlyPlayedCursors,
            RecentlyPlayedItem, RecentlyPlayedResponse, RecentlyPlayedTrack,
        },
    };

    #[test]
    fn first_sync_persists_tracks_artists_plays_and_sync_state() {
        let mut connection = test_connection();
        let response = response(vec![
            item(
                "track-1",
                "Track One",
                vec![artist("artist-1", "Artist One")],
                "2026-09-09T10:00:00Z",
            ),
            item(
                "track-2",
                "Track Two",
                vec![artist("artist-2", "Artist Two")],
                "2026-09-09T10:05:00Z",
            ),
            item(
                "track-3",
                "Track Three",
                vec![artist("artist-3", "Artist Three")],
                "2026-09-09T10:10:00Z",
            ),
        ]);

        let result = persist_recently_played(&mut connection, &response).expect("sync persists");

        assert_eq!(result.processed_items, 3);
        assert_eq!(result.new_play_history_rows, 3);
        assert_eq!(result.new_track_rows, 3);
        assert_eq!(result.new_artist_rows, 3);
        assert_eq!(result.new_discoveries, 3);
        assert_eq!(table_count(&connection, "tracks"), 3);
        assert_eq!(table_count(&connection, "artists"), 3);
        assert_eq!(table_count(&connection, "play_history"), 3);
        assert_eq!(table_count(&connection, "sync_state"), 1);
    }

    #[test]
    fn repeated_sync_is_idempotent() {
        let mut connection = test_connection();
        let response = response(vec![
            item(
                "track-1",
                "Track One",
                vec![artist("artist-1", "Artist One")],
                "2026-09-09T10:00:00Z",
            ),
            item(
                "track-2",
                "Track Two",
                vec![artist("artist-2", "Artist Two")],
                "2026-09-09T10:05:00Z",
            ),
        ]);

        persist_recently_played(&mut connection, &response).expect("first sync persists");
        let second =
            persist_recently_played(&mut connection, &response).expect("second sync persists");

        assert_eq!(second.new_play_history_rows, 0);
        assert_eq!(second.new_track_rows, 0);
        assert_eq!(second.updated_track_rows, 2);
        assert_eq!(second.new_artist_rows, 0);
        assert_eq!(second.new_discoveries, 0);
        assert_eq!(table_count(&connection, "tracks"), 2);
        assert_eq!(table_count(&connection, "artists"), 2);
        assert_eq!(table_count(&connection, "play_history"), 2);
    }

    #[test]
    fn repeated_track_creates_one_track_and_multiple_play_events() {
        let mut connection = test_connection();
        let response = response(vec![
            item(
                "track-1",
                "Track One",
                vec![artist("artist-1", "Artist One")],
                "2026-09-09T10:00:00Z",
            ),
            item(
                "track-1",
                "Track One",
                vec![artist("artist-1", "Artist One")],
                "2026-09-09T10:15:00Z",
            ),
        ]);

        let result = persist_recently_played(&mut connection, &response).expect("sync persists");

        assert_eq!(result.new_track_rows, 1);
        assert_eq!(result.new_play_history_rows, 2);
        assert_eq!(result.new_discoveries, 1);
        assert_eq!(table_count(&connection, "tracks"), 1);
        assert_eq!(table_count(&connection, "play_history"), 2);
    }

    #[test]
    fn multiple_artist_order_is_preserved() {
        let mut connection = test_connection();
        let response = response(vec![item(
            "track-1",
            "Collaboration",
            vec![artist("artist-1", "First"), artist("artist-2", "Second")],
            "2026-09-09T10:00:00Z",
        )]);

        persist_recently_played(&mut connection, &response).expect("sync persists");
        let rows = connection
            .prepare(
                "
                SELECT artists.spotify_id, track_artists.artist_order
                FROM track_artists
                JOIN artists ON artists.id = track_artists.artist_id
                ORDER BY track_artists.artist_order
                ",
            )
            .expect("query prepares")
            .query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
            })
            .expect("query runs")
            .collect::<Result<Vec<_>, _>>()
            .expect("rows read");

        assert_eq!(
            rows,
            vec![("artist-1".to_string(), 0), ("artist-2".to_string(), 1)]
        );
    }

    #[test]
    fn discovery_count_only_includes_tracks_with_no_prior_play() {
        let mut connection = test_connection();
        insert_track(&connection, "track-old");
        connection
            .execute(
                "
                INSERT INTO play_history (track_id, played_at, synced_at)
                VALUES (1, '2026-09-08T10:00:00Z', '2026-09-08T10:01:00Z')
                ",
                [],
            )
            .expect("prior play inserts");
        insert_track(&connection, "track-known-no-play");

        let response = response(vec![
            item(
                "track-old",
                "Old Track",
                vec![artist("artist-1", "Artist One")],
                "2026-09-09T10:00:00Z",
            ),
            item(
                "track-known-no-play",
                "Known Track",
                vec![artist("artist-2", "Artist Two")],
                "2026-09-09T10:05:00Z",
            ),
        ]);

        let result = persist_recently_played(&mut connection, &response).expect("sync persists");

        assert_eq!(result.new_discoveries, 1);
    }

    #[test]
    fn unsupported_item_is_skipped_without_failing() {
        let mut connection = test_connection();
        let response = response(vec![RecentlyPlayedItem {
            played_at: "2026-09-09T10:00:00Z".to_string(),
            context: None,
            track: RecentlyPlayedTrack {
                spotify_id: None,
                title: "Local Demo".to_string(),
                duration_ms: None,
                spotify_url: None,
                album: None,
                artists: vec![],
                is_local: true,
                unsupported_reason: Some(
                    "Local Spotify track without a Spotify track ID.".to_string(),
                ),
            },
        }]);

        let result = persist_recently_played(&mut connection, &response).expect("sync persists");

        assert_eq!(result.processed_items, 1);
        assert_eq!(result.skipped_items, 1);
        assert_eq!(table_count(&connection, "tracks"), 0);
        assert_eq!(table_count(&connection, "play_history"), 0);
    }

    #[test]
    fn persistence_error_rolls_back_the_batch_and_does_not_advance_state() {
        let mut connection = test_connection();
        let response = response(vec![item(
            "track-1",
            "Broken Relation",
            vec![
                artist("artist-1", "First"),
                artist("artist-1", "Duplicate Artist"),
            ],
            "2026-09-09T10:00:00Z",
        )]);

        let result = persist_recently_played(&mut connection, &response);

        assert!(result.is_err());
        assert_eq!(table_count(&connection, "tracks"), 0);
        assert_eq!(table_count(&connection, "artists"), 0);
        assert_eq!(table_count(&connection, "play_history"), 0);
        assert_eq!(table_count(&connection, "sync_state"), 0);
    }

    #[test]
    fn personal_metadata_survives_track_metadata_refresh() {
        let mut connection = test_connection();
        insert_track(&connection, "track-1");
        connection
            .execute(
                "INSERT INTO tags (name, category) VALUES ('Warm', 'mood')",
                [],
            )
            .expect("tag inserts");
        connection
            .execute(
                "INSERT INTO track_tags (track_id, tag_id) VALUES (1, 1)",
                [],
            )
            .expect("track tag inserts");
        connection
            .execute(
                "INSERT INTO track_notes (track_id, note) VALUES (1, 'This one matters')",
                [],
            )
            .expect("note inserts");
        let response = response(vec![item(
            "track-1",
            "Refreshed Title",
            vec![artist("artist-1", "Artist One")],
            "2026-09-09T10:00:00Z",
        )]);

        persist_recently_played(&mut connection, &response).expect("sync persists");

        assert_eq!(table_count(&connection, "track_tags"), 1);
        let tag: (String, String, i64) = connection.query_row(
            "SELECT t.name, t.category, tt.track_id FROM tags t JOIN track_tags tt ON tt.tag_id = t.id WHERE t.id = 1",
            [], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?))
        ).expect("personal tag remains unchanged");
        assert_eq!(tag, ("Warm".into(), "mood".into(), 1));
        assert_eq!(table_count(&connection, "track_notes"), 1);
        let note: String = connection
            .query_row(
                "SELECT note FROM track_notes WHERE track_id = 1",
                [],
                |row| row.get(0),
            )
            .expect("note reads");
        assert_eq!(note, "This one matters");
    }

    #[test]
    fn sync_cursor_uses_last_processed_played_at_with_small_overlap() {
        let mut connection = test_connection();
        let response = response(vec![item(
            "track-1",
            "Track One",
            vec![artist("artist-1", "Artist One")],
            "2026-09-09T10:00:00Z",
        )]);

        persist_recently_played(&mut connection, &response).expect("sync persists");
        let cursor = read_sync_cursor(&connection).expect("cursor reads");

        assert_eq!(
            cursor.last_processed_played_at,
            Some("2026-09-09T10:00:00Z".to_string())
        );
        assert_eq!(cursor.after_ms, Some(1788947999999));
    }

    #[test]
    #[ignore = "uses the locally connected Spotify account and writes to the local app-data database"]
    fn real_spotify_sync_recently_played_persists_to_local_database() {
        let database_path =
            std::path::PathBuf::from(std::env::var("APPDATA").expect("APPDATA is set"))
                .join("com.local.musicarchive")
                .join("music-archive.db");
        let mut connection =
            Connection::open(&database_path).expect("local app-data database opens");
        connection
            .pragma_update(None, "foreign_keys", "ON")
            .expect("foreign keys are enabled");
        migrations::run(&mut connection, &database_path).expect("migrations run");

        let auth = crate::spotify::SpotifyAuth::new();
        let access_token = auth.valid_access_token().expect("Spotify is connected");
        let cursor = read_sync_cursor(&connection).expect("cursor reads");
        let response =
            crate::spotify::api::recently_played(&access_token, Some(50), cursor.after_ms, None)
                .expect("Spotify API works");
        let first =
            persist_recently_played(&mut connection, &response).expect("first sync persists");
        let second =
            persist_recently_played(&mut connection, &response).expect("second sync persists");

        println!(
            "first_sync processed={} new_plays={} new_tracks={} new_artists={} skipped={} latest_played_at={:?}",
            first.processed_items,
            first.new_play_history_rows,
            first.new_track_rows,
            first.new_artist_rows,
            first.skipped_items,
            first.latest_processed_played_at
        );
        println!(
            "second_identical_sync processed={} new_plays={} new_tracks={} new_artists={} skipped={} latest_played_at={:?}",
            second.processed_items,
            second.new_play_history_rows,
            second.new_track_rows,
            second.new_artist_rows,
            second.skipped_items,
            second.latest_processed_played_at
        );
        for table in [
            "artists",
            "tracks",
            "track_artists",
            "play_history",
            "sync_state",
        ] {
            println!("{table}={}", table_count(&connection, table));
        }
        let (earliest, latest): (Option<String>, Option<String>) = connection
            .query_row(
                "SELECT MIN(played_at), MAX(played_at) FROM play_history",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .expect("play range reads");
        println!("earliest_played_at={earliest:?}");
        println!("latest_played_at={latest:?}");
    }

    fn test_connection() -> Connection {
        let mut connection = Connection::open_in_memory().expect("in-memory db opens");
        connection
            .pragma_update(None, "foreign_keys", "ON")
            .expect("foreign keys are enabled");
        migrations::run(&mut connection, ":memory:".as_ref()).expect("migrations run");
        connection
    }

    fn response(items: Vec<RecentlyPlayedItem>) -> RecentlyPlayedResponse {
        RecentlyPlayedResponse {
            items,
            cursors: RecentlyPlayedCursors {
                after: None,
                before: None,
            },
            next: None,
            limit: 50,
        }
    }

    fn item(
        spotify_id: &str,
        title: &str,
        artists: Vec<RecentlyPlayedArtist>,
        played_at: &str,
    ) -> RecentlyPlayedItem {
        RecentlyPlayedItem {
            played_at: played_at.to_string(),
            context: Some(PlayContext {
                context_type: Some("playlist".to_string()),
                uri: Some("spotify:playlist:playlist-1".to_string()),
            }),
            track: RecentlyPlayedTrack {
                spotify_id: Some(spotify_id.to_string()),
                title: title.to_string(),
                duration_ms: Some(180000),
                spotify_url: Some(format!("https://open.spotify.com/track/{spotify_id}")),
                album: Some(RecentlyPlayedAlbum {
                    spotify_id: Some("album-1".to_string()),
                    name: "Album".to_string(),
                    artwork_url: Some("https://image.example/cover.jpg".to_string()),
                }),
                artists,
                is_local: false,
                unsupported_reason: None,
            },
        }
    }

    fn artist(spotify_id: &str, name: &str) -> RecentlyPlayedArtist {
        RecentlyPlayedArtist {
            spotify_id: Some(spotify_id.to_string()),
            name: name.to_string(),
            spotify_url: Some(format!("https://open.spotify.com/artist/{spotify_id}")),
        }
    }

    fn insert_track(connection: &Connection, spotify_id: &str) {
        connection
            .execute(
                "INSERT INTO tracks (spotify_id, title) VALUES (?1, ?2)",
                params![spotify_id, "Original Title"],
            )
            .expect("track inserts");
    }

    fn table_count(connection: &Connection, table_name: &str) -> i64 {
        let sql = format!("SELECT COUNT(*) FROM {table_name}");
        connection
            .query_row(&sql, [], |row| row.get(0))
            .expect("table count reads")
    }
}
