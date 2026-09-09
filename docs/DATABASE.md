# Database

Music Archive uses SQLite for local persistent storage. The database is owned by the Tauri backend; React must call backend commands and must not query SQLite directly.

## File Location

The production database is named `music-archive.db` and is stored in the operating system app data directory resolved by Tauri for Music Archive.

The app must not create the production database in the source repository and must not hardcode a user-specific path.

## Initialization

The Tauri backend initializes SQLite during application startup:

```text
Resolve Tauri app data directory
Create the directory if needed
Open music-archive.db
Enable SQLite foreign key enforcement
Run pending migrations
Register the database connection as backend state
```

Initialization failures include path context and should prevent the app from silently continuing with an invalid database state.

## Migrations

Migrations are tracked in `schema_migrations`:

- `version INTEGER PRIMARY KEY`
- `applied_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP`

The migration runner applies migrations in version order and skips versions that are already recorded. It does not delete or recreate an existing database.

Current migrations:

- `1` - database foundation
- `2` - core music database

## Timestamp Policy

Application timestamps are stored as UTC text. Listening timestamps such as `play_history.played_at`, sync timestamps, and lifecycle timestamps should use RFC3339 UTC text, for example `2026-09-09T10:00:00Z`.

Date-only values, such as `digging_sessions.start_date` and `digging_sessions.end_date`, are stored separately as `YYYY-MM-DD` text.

## Schema Overview

Spotify metadata tables:

- `artists` - Spotify artist metadata.
- `tracks` - Spotify track metadata. `created_at` is insertion time only and is not the discovery date.
- `track_artists` - many-to-many track/artist relationship with artist order.
- `play_history` - Spotify listening events.
- `sync_state` - future sync progress by source.
- `playlists` - Spotify playlist metadata.
- `playlist_tracks` - playlist membership and ordering.

Personal metadata tables:

- `tags` - user-created genre, mood, sound, vocal, and free tags.
- `track_tags` - track/tag relationship.
- `track_notes` - one personal note record per track.
- `playlist_notes` - personal playlist description and memo, separate from Spotify description.
- `playlist_tags` - playlist/tag relationship.
- `digging_sessions` - periods where the user was exploring a topic, sound, artist, genre, or style.
- `digging_tracks` - digging session/track relationship.

No discovery, calendar, or statistics tables are stored. Those values are derived from raw play history.

## Relationships

```text
artists <-> track_artists <-> tracks <-> play_history
                                  |
                                  +-> track_tags <-> tags
                                  +-> track_notes
                                  +-> playlist_tracks <-> playlists
                                  |                         |
                                  |                         +-> playlist_notes
                                  |                         +-> playlist_tags <-> tags
                                  |
                                  +-> digging_tracks <-> digging_sessions
```

Delete behavior:

- Deleting a track cascades to `track_artists`, `play_history`, `track_tags`, `track_notes`, `playlist_tracks`, and `digging_tracks`.
- Deleting an artist cascades to `track_artists`.
- Deleting a tag cascades to `track_tags` and `playlist_tags`.
- Deleting a playlist cascades to `playlist_tracks`, `playlist_notes`, and `playlist_tags`.
- Deleting a digging session cascades to `digging_tracks`.
- Deleting playlist or digging relation rows never deletes the underlying track.

Application-level track hard deletion is not supported in the MVP. Tracks, play history, track tags, track notes, playlist membership, and digging relations are archive data. Spotify synchronization must never delete a track, play, note, tag relation, playlist relation, or digging relation only because Spotify no longer returns it.

The track-level cascade rules exist as referential integrity protection for exceptional maintenance cases, not as normal product behavior. Feature code should preserve archived track data and avoid exposing routine hard-delete flows for tracks.

## Constraints

Important constraints:

- `artists.spotify_id`, `tracks.spotify_id`, and `playlists.spotify_id` are unique.
- `tags.category` is checked against `genre`, `mood`, `sound`, `vocal`, and `free`.
- Tags are unique by `(category, name)`, so the same name can exist in different categories.
- `track_artists`, `track_tags`, `playlist_tags`, and `digging_tracks` prevent duplicate relationships.
- `play_history` prevents duplicate plays by `(track_id, played_at)`.
- `playlist_tracks` uses its own `id` and does not make `(playlist_id, track_id)` unique, so repeated tracks in the same playlist remain possible.
- `digging_sessions` allows active sessions with `end_date = NULL`; when an end date exists, it must be on or after `start_date`.

## Indexes

Unique constraints provide indexes for Spotify ID lookups and duplicate prevention. Additional indexes support expected query paths:

- `idx_play_history_track_id`
- `idx_play_history_played_at`
- `idx_track_tags_track_id`
- `idx_playlist_tracks_playlist_id`
- `idx_digging_tracks_digging_id`

## Discovery

Discovery is calculated from the earliest listening event:

```sql
SELECT track_id, MIN(played_at) AS discovered_at
FROM play_history
GROUP BY track_id;
```

`tracks.created_at` must not be used as discovery time.

## Calendar and Statistics

Calendar views and statistics are derived from `play_history` and related tables. The database intentionally does not store duplicated calendar summaries, monthly statistics, play counts, most-played tables, or discovery tables.
