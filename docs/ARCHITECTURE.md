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

Phase 8 includes the navigation shell, backend-owned SQLite archive, core music
database schema, Spotify authentication, read-only Recently Played retrieval,
manual listening-history sync into the archive, local Library browsing, and Track Detail.

The current implementation does not include Calendar,
Tags UI, Notes UI, playlist sync, Digging UI, background sync, or playback.

## Database Layer

Database code lives under `src-tauri/src/database`:

- `connection.rs` resolves the app data path, opens SQLite, enables foreign keys, and owns the managed connection state.
- `migrations.rs` creates and maintains the migration tracking table and applies pending migrations in order.

React does not access SQLite directly. Future UI features should call Tauri commands, and those commands should use the backend database layer.

## Boundaries

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
