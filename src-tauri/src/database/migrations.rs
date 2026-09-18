use std::path::Path;

use rusqlite::{params, Connection};

struct Migration {
    version: i64,
    name: &'static str,
    sql: &'static str,
}

const MIGRATIONS: &[Migration] = &[
    Migration {
        version: 1,
        name: "database foundation",
        sql: "",
    },
    Migration {
        version: 2,
        name: "core music database",
        sql: "
            CREATE TABLE artists (
                id INTEGER PRIMARY KEY,
                spotify_id TEXT NOT NULL UNIQUE,
                name TEXT NOT NULL,
                spotify_url TEXT,
                created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
                updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
            );

            CREATE TABLE tracks (
                id INTEGER PRIMARY KEY,
                spotify_id TEXT NOT NULL UNIQUE,
                title TEXT NOT NULL,
                album_name TEXT,
                album_spotify_id TEXT,
                album_art_url TEXT,
                duration_ms INTEGER,
                spotify_url TEXT,
                created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
                updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
                CHECK (duration_ms IS NULL OR duration_ms >= 0)
            );

            CREATE TABLE track_artists (
                track_id INTEGER NOT NULL,
                artist_id INTEGER NOT NULL,
                artist_order INTEGER NOT NULL,
                PRIMARY KEY (track_id, artist_id),
                UNIQUE (track_id, artist_order),
                CHECK (artist_order >= 0),
                FOREIGN KEY (track_id) REFERENCES tracks(id) ON DELETE CASCADE,
                FOREIGN KEY (artist_id) REFERENCES artists(id) ON DELETE CASCADE
            );

            CREATE TABLE play_history (
                id INTEGER PRIMARY KEY,
                track_id INTEGER NOT NULL,
                played_at TEXT NOT NULL,
                context_type TEXT,
                context_uri TEXT,
                synced_at TEXT NOT NULL,
                UNIQUE (track_id, played_at),
                FOREIGN KEY (track_id) REFERENCES tracks(id) ON DELETE CASCADE
            );

            CREATE TABLE sync_state (
                source TEXT PRIMARY KEY,
                last_successful_sync_at TEXT,
                last_processed_played_at TEXT,
                sync_status TEXT NOT NULL DEFAULT 'idle',
                last_error TEXT,
                CHECK (sync_status IN ('idle', 'running', 'success', 'failed'))
            );

            CREATE TABLE tags (
                id INTEGER PRIMARY KEY,
                name TEXT NOT NULL,
                category TEXT NOT NULL,
                created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
                UNIQUE (category, name),
                CHECK (category IN ('genre', 'mood', 'sound', 'vocal', 'free'))
            );

            CREATE TABLE track_tags (
                track_id INTEGER NOT NULL,
                tag_id INTEGER NOT NULL,
                created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
                PRIMARY KEY (track_id, tag_id),
                FOREIGN KEY (track_id) REFERENCES tracks(id) ON DELETE CASCADE,
                FOREIGN KEY (tag_id) REFERENCES tags(id) ON DELETE CASCADE
            );

            CREATE TABLE track_notes (
                track_id INTEGER PRIMARY KEY,
                note TEXT NOT NULL,
                created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
                updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
                FOREIGN KEY (track_id) REFERENCES tracks(id) ON DELETE CASCADE
            );

            CREATE TABLE playlists (
                id INTEGER PRIMARY KEY,
                spotify_id TEXT NOT NULL UNIQUE,
                name TEXT NOT NULL,
                spotify_description TEXT,
                image_url TEXT,
                owner_spotify_id TEXT,
                spotify_url TEXT,
                snapshot_id TEXT,
                is_owned INTEGER NOT NULL DEFAULT 0,
                is_collaborative INTEGER NOT NULL DEFAULT 0,
                track_count INTEGER NOT NULL DEFAULT 0,
                last_synced_at TEXT,
                CHECK (is_owned IN (0, 1)),
                CHECK (is_collaborative IN (0, 1)),
                CHECK (track_count >= 0)
            );

            CREATE TABLE playlist_tracks (
                id INTEGER PRIMARY KEY,
                playlist_id INTEGER NOT NULL,
                track_id INTEGER NOT NULL,
                position INTEGER NOT NULL,
                added_at TEXT,
                UNIQUE (playlist_id, position),
                CHECK (position >= 0),
                FOREIGN KEY (playlist_id) REFERENCES playlists(id) ON DELETE CASCADE,
                FOREIGN KEY (track_id) REFERENCES tracks(id) ON DELETE CASCADE
            );

            CREATE TABLE playlist_notes (
                playlist_id INTEGER PRIMARY KEY,
                personal_description TEXT,
                memo TEXT,
                updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
                FOREIGN KEY (playlist_id) REFERENCES playlists(id) ON DELETE CASCADE
            );

            CREATE TABLE playlist_tags (
                playlist_id INTEGER NOT NULL,
                tag_id INTEGER NOT NULL,
                PRIMARY KEY (playlist_id, tag_id),
                FOREIGN KEY (playlist_id) REFERENCES playlists(id) ON DELETE CASCADE,
                FOREIGN KEY (tag_id) REFERENCES tags(id) ON DELETE CASCADE
            );

            CREATE TABLE digging_sessions (
                id INTEGER PRIMARY KEY,
                name TEXT NOT NULL,
                start_date TEXT NOT NULL,
                end_date TEXT,
                description TEXT,
                created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
                updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
                CHECK (end_date IS NULL OR end_date >= start_date)
            );

            CREATE TABLE digging_tracks (
                digging_id INTEGER NOT NULL,
                track_id INTEGER NOT NULL,
                added_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
                PRIMARY KEY (digging_id, track_id),
                FOREIGN KEY (digging_id) REFERENCES digging_sessions(id) ON DELETE CASCADE,
                FOREIGN KEY (track_id) REFERENCES tracks(id) ON DELETE CASCADE
            );

            CREATE INDEX idx_play_history_track_id ON play_history(track_id);
            CREATE INDEX idx_play_history_played_at ON play_history(played_at);
            CREATE INDEX idx_track_tags_track_id ON track_tags(track_id);
            CREATE INDEX idx_playlist_tracks_playlist_id ON playlist_tracks(playlist_id);
            CREATE INDEX idx_digging_tracks_digging_id ON digging_tracks(digging_id);
        ",
    },
    Migration {
        version: 3,
        name: "playlist snapshot history",
        sql: "CREATE TABLE playlist_track_history (
            playlist_id INTEGER NOT NULL REFERENCES playlists(id) ON DELETE CASCADE,
            snapshot_id TEXT NOT NULL,
            position INTEGER NOT NULL,
            track_id INTEGER NOT NULL REFERENCES tracks(id),
            added_at TEXT,
            PRIMARY KEY(playlist_id,snapshot_id,position)
        );",
    },
];

pub fn run(connection: &mut Connection, database_path: &Path) -> Result<(), String> {
    ensure_migration_table(connection, database_path)?;

    for migration in MIGRATIONS {
        if has_migration(connection, migration.version, database_path)? {
            continue;
        }

        let transaction = connection.transaction().map_err(|error| {
            format!(
                "Failed to start migration {} for {}: {error}",
                migration.version,
                database_path.display()
            )
        })?;

        if !migration.sql.trim().is_empty() {
            transaction.execute_batch(migration.sql).map_err(|error| {
                format!(
                    "Failed to apply migration {} ({}) for {}: {error}",
                    migration.version,
                    migration.name,
                    database_path.display()
                )
            })?;
        }

        transaction
            .execute(
                "INSERT INTO schema_migrations (version) VALUES (?1)",
                params![migration.version],
            )
            .map_err(|error| {
                format!(
                    "Failed to record migration {} for {}: {error}",
                    migration.version,
                    database_path.display()
                )
            })?;

        transaction.commit().map_err(|error| {
            format!(
                "Failed to commit migration {} for {}: {error}",
                migration.version,
                database_path.display()
            )
        })?;
    }

    Ok(())
}

fn ensure_migration_table(connection: &Connection, database_path: &Path) -> Result<(), String> {
    connection
        .execute_batch(
            "
            CREATE TABLE IF NOT EXISTS schema_migrations (
                version INTEGER PRIMARY KEY,
                applied_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
            );
            ",
        )
        .map_err(|error| {
            format!(
                "Failed to ensure schema_migrations table for {}: {error}",
                database_path.display()
            )
        })
}

fn has_migration(
    connection: &Connection,
    version: i64,
    database_path: &Path,
) -> Result<bool, String> {
    let count: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM schema_migrations WHERE version = ?1",
            params![version],
            |row| row.get(0),
        )
        .map_err(|error| {
            format!(
                "Failed to inspect migration {} for {}: {error}",
                version,
                database_path.display()
            )
        })?;

    Ok(count > 0)
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        time::{SystemTime, UNIX_EPOCH},
    };

    use rusqlite::{params, Connection};

    use super::run;

    #[test]
    fn migrations_are_recorded_once() {
        let database_path = test_database_path("migration-test");
        let mut connection = open_test_database(&database_path);

        run(&mut connection, &database_path).expect("first migration run succeeds");
        run(&mut connection, &database_path).expect("second migration run succeeds");

        let migration_count: i64 = connection
            .query_row("SELECT COUNT(*) FROM schema_migrations", [], |row| {
                row.get(0)
            })
            .expect("migration count can be read");

        assert_eq!(migration_count, 3);

        drop(connection);
        fs::remove_file(database_path).expect("test database can be removed");
    }

    #[test]
    fn core_tables_are_created() {
        let database_path = test_database_path("tables-test");
        let mut connection = open_test_database(&database_path);

        run(&mut connection, &database_path).expect("migrations succeed");

        let expected_tables = [
            "schema_migrations",
            "artists",
            "tracks",
            "track_artists",
            "play_history",
            "sync_state",
            "tags",
            "track_tags",
            "track_notes",
            "playlists",
            "playlist_tracks",
            "playlist_notes",
            "playlist_tags",
            "digging_sessions",
            "digging_tracks",
        ];

        for table in expected_tables {
            let count: i64 = connection
                .query_row(
                    "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = ?1",
                    params![table],
                    |row| row.get(0),
                )
                .expect("table lookup succeeds");

            assert_eq!(count, 1, "{table} table should exist");
        }

        drop(connection);
        fs::remove_file(database_path).expect("test database can be removed");
    }

    #[test]
    fn derived_tables_are_not_created() {
        let database_path = test_database_path("derived-tables-test");
        let mut connection = open_test_database(&database_path);

        run(&mut connection, &database_path).expect("migrations succeed");

        for table in ["discoveries", "calendar", "calendar_days", "statistics"] {
            let count: i64 = connection
                .query_row(
                    "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = ?1",
                    params![table],
                    |row| row.get(0),
                )
                .expect("table lookup succeeds");
            assert_eq!(count, 0, "{table} table should not exist");
        }

        drop(connection);
        fs::remove_file(database_path).expect("test database can be removed");
    }

    #[test]
    fn required_indexes_are_created() {
        let database_path = test_database_path("indexes-test");
        let mut connection = open_test_database(&database_path);

        run(&mut connection, &database_path).expect("migrations succeed");

        for index in [
            "idx_play_history_track_id",
            "idx_play_history_played_at",
            "idx_track_tags_track_id",
            "idx_playlist_tracks_playlist_id",
            "idx_digging_tracks_digging_id",
        ] {
            let count: i64 = connection
                .query_row(
                    "SELECT COUNT(*) FROM sqlite_master WHERE type = 'index' AND name = ?1",
                    params![index],
                    |row| row.get(0),
                )
                .expect("index lookup succeeds");
            assert_eq!(count, 1, "{index} should exist");
        }

        drop(connection);
        fs::remove_file(database_path).expect("test database can be removed");
    }

    #[test]
    fn phase_two_database_upgrades_to_phase_three() {
        let database_path = test_database_path("phase-two-upgrade-test");
        let mut connection = open_test_database(&database_path);

        connection
            .execute_batch(
                "
                CREATE TABLE schema_migrations (
                    version INTEGER PRIMARY KEY,
                    applied_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
                );
                INSERT INTO schema_migrations (version) VALUES (1);
                ",
            )
            .expect("phase two migration state can be created");

        run(&mut connection, &database_path).expect("phase two database upgrades");

        let migrations: Vec<i64> = connection
            .prepare("SELECT version FROM schema_migrations ORDER BY version")
            .expect("migration query prepares")
            .query_map([], |row| row.get(0))
            .expect("migration query runs")
            .collect::<Result<Vec<_>, _>>()
            .expect("migration rows can be read");
        assert_eq!(migrations, vec![1, 2, 3]);

        let tracks_table_count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = 'tracks'",
                [],
                |row| row.get(0),
            )
            .expect("tracks table lookup succeeds");
        assert_eq!(tracks_table_count, 1);

        drop(connection);
        fs::remove_file(database_path).expect("test database can be removed");
    }

    #[test]
    fn core_constraints_are_enforced() {
        let database_path = test_database_path("constraints-test");
        let mut connection = open_test_database(&database_path);

        run(&mut connection, &database_path).expect("migrations succeed");
        insert_track(&connection, "spotify-track-1");

        let duplicate_track = connection.execute(
            "INSERT INTO tracks (spotify_id, title) VALUES (?1, ?2)",
            params!["spotify-track-1", "Duplicate"],
        );
        assert!(duplicate_track.is_err());

        let invalid_tag = connection.execute(
            "INSERT INTO tags (name, category) VALUES (?1, ?2)",
            params!["Dark", "invalid"],
        );
        assert!(invalid_tag.is_err());

        connection
            .execute(
                "INSERT INTO tags (name, category) VALUES (?1, ?2)",
                params!["Dark", "mood"],
            )
            .expect("valid tag inserts");
        connection
            .execute(
                "INSERT INTO tags (name, category) VALUES (?1, ?2)",
                params!["Dark", "free"],
            )
            .expect("same name in a different tag category inserts");

        let duplicate_tag_category = connection.execute(
            "INSERT INTO tags (name, category) VALUES (?1, ?2)",
            params!["Dark", "mood"],
        );
        assert!(duplicate_tag_category.is_err());

        connection
            .execute(
                "INSERT INTO track_tags (track_id, tag_id) VALUES (1, 1)",
                [],
            )
            .expect("track tag relation inserts");
        let duplicate_track_tag = connection.execute(
            "INSERT INTO track_tags (track_id, tag_id) VALUES (1, 1)",
            [],
        );
        assert!(duplicate_track_tag.is_err());

        connection
            .execute(
                "
                INSERT INTO play_history (track_id, played_at, synced_at)
                VALUES (1, '2026-09-09T00:00:00Z', '2026-09-09T00:01:00Z')
                ",
                [],
            )
            .expect("play inserts");
        let duplicate_play = connection.execute(
            "
            INSERT INTO play_history (track_id, played_at, synced_at)
            VALUES (1, '2026-09-09T00:00:00Z', '2026-09-09T00:02:00Z')
            ",
            [],
        );
        assert!(duplicate_play.is_err());

        let foreign_key_violation = connection.execute(
            "
            INSERT INTO play_history (track_id, played_at, synced_at)
            VALUES (999, '2026-09-09T00:00:00Z', '2026-09-09T00:01:00Z')
            ",
            [],
        );
        assert!(foreign_key_violation.is_err());

        let invalid_digging_dates = connection.execute(
            "
            INSERT INTO digging_sessions (name, start_date, end_date)
            VALUES ('Earlier end', '2026-09-09', '2026-09-08')
            ",
            [],
        );
        assert!(invalid_digging_dates.is_err());

        drop(connection);
        fs::remove_file(database_path).expect("test database can be removed");
    }

    #[test]
    fn playlist_tracks_allow_repeated_tracks_at_different_positions() {
        let database_path = test_database_path("playlist-repeated-track-test");
        let mut connection = open_test_database(&database_path);

        run(&mut connection, &database_path).expect("migrations succeed");
        insert_track(&connection, "spotify-track-1");
        connection
            .execute(
                "INSERT INTO playlists (spotify_id, name) VALUES ('spotify-playlist-1', 'Playlist')",
                [],
            )
            .expect("playlist inserts");
        connection
            .execute(
                "
                INSERT INTO playlist_tracks (playlist_id, track_id, position)
                VALUES (1, 1, 0), (1, 1, 1)
                ",
                [],
            )
            .expect("same track can appear twice at different playlist positions");

        let duplicate_position = connection.execute(
            "INSERT INTO playlist_tracks (playlist_id, track_id, position) VALUES (1, 1, 1)",
            [],
        );
        assert!(duplicate_position.is_err());

        drop(connection);
        fs::remove_file(database_path).expect("test database can be removed");
    }

    #[test]
    fn relation_rows_cascade_when_parents_are_deleted() {
        let database_path = test_database_path("cascade-test");
        let mut connection = open_test_database(&database_path);

        run(&mut connection, &database_path).expect("migrations succeed");
        insert_track(&connection, "spotify-track-1");
        connection
            .execute(
                "INSERT INTO artists (spotify_id, name) VALUES ('spotify-artist-1', 'Artist')",
                [],
            )
            .expect("artist inserts");
        connection
            .execute(
                "INSERT INTO track_artists (track_id, artist_id, artist_order) VALUES (1, 1, 0)",
                [],
            )
            .expect("track artist relation inserts");
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
            .expect("track tag relation inserts");
        connection
            .execute(
                "INSERT INTO track_notes (track_id, note) VALUES (1, 'keeper')",
                [],
            )
            .expect("track note inserts");
        connection
            .execute(
                "INSERT INTO playlists (spotify_id, name) VALUES ('spotify-playlist-1', 'Playlist')",
                [],
            )
            .expect("playlist inserts");
        connection
            .execute(
                "INSERT INTO playlist_tracks (playlist_id, track_id, position) VALUES (1, 1, 0)",
                [],
            )
            .expect("playlist track inserts");
        connection
            .execute(
                "INSERT INTO digging_sessions (name, start_date) VALUES ('Discovery week', '2026-09-09')",
                [],
            )
            .expect("digging session inserts");
        connection
            .execute(
                "INSERT INTO digging_tracks (digging_id, track_id) VALUES (1, 1)",
                [],
            )
            .expect("digging track inserts");

        connection
            .execute("DELETE FROM tracks WHERE id = 1", [])
            .expect("track deletes");

        for table in [
            "track_artists",
            "track_tags",
            "track_notes",
            "playlist_tracks",
            "digging_tracks",
        ] {
            let count: i64 = connection
                .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
                    row.get(0)
                })
                .expect("relation table count can be read");
            assert_eq!(count, 0, "{table} should cascade");
        }

        let playlist_count: i64 = connection
            .query_row("SELECT COUNT(*) FROM playlists", [], |row| row.get(0))
            .expect("playlist count can be read");
        let digging_count: i64 = connection
            .query_row("SELECT COUNT(*) FROM digging_sessions", [], |row| {
                row.get(0)
            })
            .expect("digging count can be read");

        assert_eq!(playlist_count, 1);
        assert_eq!(digging_count, 1);

        drop(connection);
        fs::remove_file(database_path).expect("test database can be removed");
    }

    #[test]
    fn existing_rows_survive_repeated_migration_runs() {
        let database_path = test_database_path("preservation-test");
        let mut connection = open_test_database(&database_path);

        run(&mut connection, &database_path).expect("first migration run succeeds");
        insert_track(&connection, "spotify-track-1");

        run(&mut connection, &database_path).expect("second migration run succeeds");

        let track_count: i64 = connection
            .query_row("SELECT COUNT(*) FROM tracks", [], |row| row.get(0))
            .expect("track count can be read");
        assert_eq!(track_count, 1);

        drop(connection);
        fs::remove_file(database_path).expect("test database can be removed");
    }

    #[test]
    fn discovery_is_derived_from_earliest_play() {
        let database_path = test_database_path("discovery-test");
        let mut connection = open_test_database(&database_path);

        run(&mut connection, &database_path).expect("migrations succeed");
        insert_track(&connection, "spotify-track-1");
        connection
            .execute(
                "
                INSERT INTO play_history (track_id, played_at, synced_at)
                VALUES
                    (1, '2026-09-09T10:00:00Z', '2026-09-09T10:01:00Z'),
                    (1, '2026-09-08T10:00:00Z', '2026-09-09T10:01:00Z')
                ",
                [],
            )
            .expect("plays insert");

        let discovery: String = connection
            .query_row(
                "SELECT MIN(played_at) FROM play_history WHERE track_id = 1",
                [],
                |row| row.get(0),
            )
            .expect("discovery query succeeds");

        assert_eq!(discovery, "2026-09-08T10:00:00Z");

        drop(connection);
        fs::remove_file(database_path).expect("test database can be removed");
    }

    fn test_database_path(test_name: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!(
            "music-archive-{test_name}-{}.db",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system time is valid")
                .as_nanos()
        ))
    }

    fn open_test_database(database_path: &std::path::Path) -> Connection {
        let connection = Connection::open(database_path).expect("test database opens");
        connection
            .pragma_update(None, "foreign_keys", "ON")
            .expect("foreign keys are enabled");
        connection
    }

    fn insert_track(connection: &Connection, spotify_id: &str) {
        connection
            .execute(
                "INSERT INTO tracks (spotify_id, title) VALUES (?1, ?2)",
                params![spotify_id, "Track"],
            )
            .expect("track inserts");
    }
}
