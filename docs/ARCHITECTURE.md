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

Phase 0 only establishes the project foundation. Spotify integration, SQLite storage, sync logic, and application workflows are not implemented yet.

## Boundaries

- React UI should not query SQLite directly.
- React UI should not call the Spotify API directly.
- Database, Spotify integration, application logic, and UI should remain separate responsibilities.
- The architecture should stay simple for the current project size.
