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

Phase 2 establishes the SQLite foundation. The app opens a local SQLite database during Tauri startup, runs migrations, and stores the database connection in backend-managed state.

The current implementation does not include Spotify integration, sync logic, Music Archive domain tables, or application workflows beyond the Phase 1 navigation shell.

## Database Layer

Database code lives under `src-tauri/src/database`:

- `connection.rs` resolves the app data path, opens SQLite, enables foreign keys, and owns the managed connection state.
- `migrations.rs` creates and maintains the migration tracking table and applies pending migrations in order.

React does not access SQLite directly. Future UI features should call Tauri commands, and those commands should use the backend database layer.

## Boundaries

- React UI should not query SQLite directly.
- React UI should not call the Spotify API directly.
- Database, Spotify integration, application logic, and UI should remain separate responsibilities.
- The architecture should stay simple for the current project size.
