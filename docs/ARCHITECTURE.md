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

Phase 7 includes the navigation shell, backend-owned SQLite archive, core music
database schema, Spotify authentication, read-only Recently Played retrieval,
manual listening-history sync into the archive, and local Library browsing.

The current implementation does not include Calendar, Track Detail,
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

- React UI should not query SQLite directly.
- React UI should not call the Spotify API directly.
- Database, Spotify integration, application logic, and UI should remain separate responsibilities.
- The architecture should stay simple for the current project size.
