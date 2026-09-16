# Music Archive

Music Archive is a personal desktop application for recording music discovery, listening history, playlists, tags, notes, and how music taste changes over time.

## Current Status

Phase 8 - Track Detail

The Tauri, React, and TypeScript project foundation is in place, along with the navigation shell, backend-owned SQLite archive, core music database schema, Spotify account authentication, and read-only Spotify Recently Played retrieval for development validation. The backend creates a local `music-archive.db` in the app data directory, enables foreign keys, and runs migration tracking.

Spotify Recently Played can now be synced into SQLite as artists, tracks, track artist relations, play history, and sync progress. Duplicate play history rows are prevented by database constraints, and unsupported/local Spotify items are skipped without failing the rest of the sync.

Library now browses local archived tracks with artwork, ordered artist credits,
album names, Discovery dates, and track/artist/album search. It works independently
of Spotify authentication and API availability; remote artwork falls back to a
local placeholder when unavailable.

Select a Library track to view its archived metadata, ordered artists, duration,
play count, First Discovered, and Last Played. Returning preserves the Library
search and scroll position. The stored Spotify link opens externally on request.
Track Detail reads SQLite only and does not refresh Spotify metadata.

Playlist sync, tagging workflows, notes workflows, dashboards, and playback are not implemented yet.

## Tech Stack

- Tauri
- React
- TypeScript
- Vite
- Rust
- npm

## Requirements

- Node.js
- npm
- Rust
- Tauri prerequisites for Windows desktop development

## Development

Install frontend dependencies:

```sh
npm install
```

Configure Spotify authentication:

1. Create a Spotify app in the Spotify Developer Dashboard.
2. Add this redirect URI to the Spotify app:

```text
http://127.0.0.1:8888/callback
```

3. Set your Spotify app Client ID before launching Music Archive.

PowerShell:

```powershell
$env:SPOTIFY_CLIENT_ID = "your_spotify_client_id"
npm run tauri -- dev
```

The Spotify Client ID is not a secret. Do not configure a Client Secret for this
desktop app. If `SPOTIFY_CLIENT_ID` is missing, the Settings page shows a
connection error and no Spotify credentials are stored.

Run the Tauri development app:

```sh
npm run tauri -- dev
```

Run only the frontend dev server:

```sh
npm run dev
```

## Build

Build the frontend:

```sh
npm run build
```

Build the desktop application:

```sh
npm run tauri -- build
```
