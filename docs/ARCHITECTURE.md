# Architecture

Music Archive is a local-first desktop application built with Tauri, React, and TypeScript.

## Planned Flow

```text
Spotify
   |
Tauri Backend / Sync Logic
   |
SQLite
   |
Application Logic
   |
React UI
```

## Current Scope

The MVP implementation includes the navigation shell, backend-owned SQLite archive, core music
database schema, Spotify authentication, read-only Recently Played retrieval,
manual listening-history sync into the archive, local Library browsing, Track Detail,
personal track tags and notes, Calendar, Playlist Archive, Digging, and Home.

Background sync and playback are out of scope.

## Database Layer

Database code lives under `src-tauri/src/database`:

- `connection.rs` resolves the app data path, opens SQLite, enables foreign keys, and owns the managed connection state.
- `migrations.rs` creates and maintains the migration tracking table and applies pending migrations in order.

React does not access SQLite directly. Future UI features should call Tauri commands, and those commands should use the backend database layer.

## Boundaries

Track tags use separate `track_tags`, `list_tags`, `create_tag`,
`assign_tag_to_track`, and `remove_tag_from_track` commands in `tags.rs`.
The TrackTags component owns category filtering and pending/error UI; the backend
validates input and owns all SQLite reads and writes. Successful mutations reload
only assigned tags. Spotify authentication and sync are not involved.

Library uses SQLite -> `library_tracks` -> Library DTO -> React. One SQL query
aggregates plays and ordered artist credits separately before joining tracks.
Discovery is `MIN(play_history.played_at)`, descending with undiscovered tracks
last, followed by title and ID for deterministic ordering. For the small archive,
Rust filters the loaded rows using lowercase substring matching for title,
artist names, and album. The optional search command argument allows later query
changes without coupling React to SQL. No Spotify request or sync is triggered;
only artwork images use their stored remote URLs, with a local fallback.

Track Detail uses the internal track ID with the `track_detail` command. A track
and its play aggregate (`COUNT`, `MIN`, `MAX`) are read separately from its ordered
artist names, avoiding count multiplication. A missing track returns `None`;
query failures return a safe error. Library owns the selected track ID and keeps
its list mounted but hidden during detail browsing, preserving search, rows, and
scroll position without a router. Artwork fallback and date display are shared.
The `open_track_in_spotify` command reads the stored URL by track ID, validates
an HTTPS `open.spotify.com/track/...` URL, and uses the existing OS browser opener.
Neither detail lookup nor rendering requires Spotify authentication or API calls.

- React UI should not query SQLite directly.
- React UI should not call the Spotify API directly.
- Database, Spotify integration, application logic, and UI should remain separate responsibilities.
- The architecture should stay simple for the current project size.

## Remaining MVP modules

- `notes.rs`: explicit track note upsert/delete; blank input removes only the note.
- `calendar.rs`: aggregates UTC plays within frontend-provided local-midnight bounds. Each day uses its own bounds for DST; discoveries use the earliest play across all history.
- `spotify/playlists.rs`: paginated read-only Spotify import with timeouts, snapshot recheck, and per-playlist transactions. Blocking HTTP runs off the UI thread. Concurrent sync requests are rejected.
- `playlists.rs`: local playlist detail, description/memo, tags, and validated external link.
- `digging.rs`: validated date ranges, active/past sessions, duplicate-safe track membership.
- Home composes existing Library, Calendar, and Digging commands; no summary tables.
- `sync_status.rs`: reads persisted sync success/error state for Settings.
- Shared `ArchiveTracks`, artwork, and archive styles provide consistent rows and responsive layouts. Track notes warn before abandoning unsaved text.

See `MVP_VALIDATION.md` for implementation status and actual validation limits.
