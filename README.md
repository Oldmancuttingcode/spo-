# Music Archive

Music Archive is a personal desktop application for recording music discovery, listening history, playlists, tags, notes, and how music taste changes over time.

## Current Status

Phase 2 - SQLite foundation

The Tauri, React, and TypeScript project foundation is in place, along with the Phase 1 navigation shell and the Phase 2 SQLite foundation. The backend creates a local `music-archive.db` in the app data directory, enables foreign keys, and runs migration tracking.

Spotify, sync, library, playlist, tagging, notes, dashboards, and the core Music Archive schema are not implemented yet.

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
