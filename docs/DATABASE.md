# Database

Music Archive uses SQLite for local persistent storage. Phase 2 adds the database foundation only; the Music Archive domain schema is still planned for Phase 3.

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

Initialization failures are returned with enough path context to diagnose the problem and should prevent the app from silently continuing with an invalid database state.

## Migrations

Migrations are tracked in `schema_migrations`:

- `version INTEGER PRIMARY KEY`
- `applied_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP`

The migration runner applies migrations in version order and skips versions that are already recorded. It does not delete or recreate an existing database.

Phase 2 records an initial foundation migration and creates no Music Archive domain tables.

## Foreign Keys

Every SQLite connection enables `PRAGMA foreign_keys = ON` during initialization so later schema phases can rely on foreign key enforcement.

## Phase 3

Phase 3 will add the core Music Archive tables for tracks, artists, play history, tags, playlists, notes, sync state, and digging sessions.
