# Music Archive

Music Archive is a personal desktop application for recording music discovery, listening history, playlists, tags, notes, and how music taste changes over time.

## Current Status

Phase 0 - Project foundation

This phase only establishes the Tauri, React, and TypeScript project foundation. Spotify, SQLite, sync, library, playlist, tagging, notes, dashboards, and other product features are not implemented yet.

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
