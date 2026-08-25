# Music Archive — AGENTS.md

## Project

Music Archive is a personal desktop application for recording and organizing the user's music listening history and taste.

The main goal is to answer:

> What music did the user discover, when did they discover it, and what kind of music were they digging at that time?

This is not a Spotify clone or music player.

Spotify is used as a source of listening and music metadata.
Music Archive stores and organizes the user's long-term listening archive and personal music classification.

---

## Tech Stack

Use:

* Tauri
* React
* TypeScript
* SQLite
* Spotify Web API
* Git

Do not introduce another major framework or architecture unless there is a clear need.

---

## Core Concepts

### Track

A music track.

### Play

A Spotify listening event for a track at a specific time.

### Discovery

The earliest Play History record for a Track in Music Archive.

Do not use the date a Track was inserted into the database as the Discovery date.

### Library

All Tracks stored in Music Archive.

### Tag

Personal classification created by the user.

Tag categories:

* Genre
* Mood
* Sound
* Vocal
* Free

### Playlist

Spotify playlist information combined with Music Archive personal metadata.

### Digging

A music topic, sound, genre, artist, or style the user was actively exploring during a specific period.

Digging is not the same as Genre.

---

## Architecture

Keep responsibilities separated.

Preferred flow:

```text
Spotify
   ↓
Tauri Backend / Sync Logic
   ↓
SQLite
   ↓
Application Logic
   ↓
React UI
```

### React

React should handle:

* UI rendering
* user input
* navigation
* loading state
* error state
* calling backend commands

React must not directly query SQLite.

React must not directly call Spotify APIs.

### Backend

The Tauri backend should handle:

* Spotify authentication
* Spotify API calls
* Spotify sync
* SQLite access
* application logic
* data validation
* statistics derived from stored data

---

## Data Rules

Separate Spotify metadata from user-created metadata.

### Spotify Metadata

Examples:

* Track title
* Artist
* Album
* Artwork
* Spotify URL
* Playlist metadata
* Played time

Spotify sync may update this data.

### Personal Metadata

Examples:

* Tags
* Notes
* Digging
* Personal playlist descriptions

Spotify sync must never silently delete or overwrite Personal Metadata.

SQLite is the long-term source of truth for the user's archive.

---

## Discovery and Derived Data

Prefer storing raw data and calculating derived data.

Do not create unnecessary duplicated tables for:

* Discovery
* Calendar summaries
* Monthly statistics
* Most played tracks

Discovery should be calculated from the earliest Play History record.

Calendar data should be derived from Play History and related stored data.

---

## Spotify Authentication

When Spotify integration is implemented:

* Use Authorization Code with PKCE.
* Do not use a Spotify Client Secret.
* Do not hardcode secrets.
* Do not log access tokens.
* Do not log refresh tokens.
* Use a loopback callback based on `127.0.0.1`.
* Request only the minimum Spotify OAuth scopes required for the current feature.

Tokens should be stored using an appropriate secure OS credential storage mechanism when possible.

---

## Spotify Sync

Spotify sync must be:

### Incremental

Do not repeatedly download and process the entire history when avoidable.

### Idempotent

Running the same sync multiple times must not create duplicate Play History records.

### Failure-safe

Spotify API or network failure must not prevent the user from accessing existing local archive data.

### Personal-data-safe

Spotify sync must not overwrite Tags, Notes, Digging, or other user-created metadata.

Avoid tight retry loops when API requests fail or are rate limited.

---

## Database

Use SQLite for local persistent storage.

Database schema changes must use migrations.

Do not solve migration problems by deleting the user's database.

Preserve existing user data whenever schema changes are made.

React should not contain raw SQL.

---

## MVP Scope

The MVP includes:

* Spotify connection
* Recently Played sync
* Play History
* Calendar
* Library
* Track Detail
* Tags
* Track Notes
* Spotify Playlist archive
* Playlist personal metadata
* Digging
* Home summary
* Sync and error states

---

## Out of Scope Unless Explicitly Requested

Do not implement these automatically:

* music playback
* audio streaming
* lyrics
* social features
* public profiles
* AI recommendations
* automatic playlist generation
* machine learning
* track similarity
* Last.fm integration
* background Windows service
* tray application
* startup sync
* cloud database
* multi-user system
* complex analytics

Do not build future features early unless they are required for the current task.

---

## UI Principles

The app should feel like a personal music archive, not an enterprise analytics dashboard.

Prioritize:

* album artwork
* music content
* typography
* simple navigation
* clear hierarchy
* comfortable browsing

Avoid:

* excessive cards
* unnecessary charts
* excessive gradients
* overly dense information
* unnecessary settings

Do not redesign unrelated pages while implementing a feature.

---

## Coding Principles

Follow these rules:

1. Keep implementations simple.
2. Implement only the requested task or phase.
3. Prefer small incremental changes.
4. Preserve existing working functionality.
5. Avoid unnecessary abstractions.
6. Avoid premature optimization.
7. Avoid giant components and giant services.
8. Do not introduce unnecessary dependencies.
9. Prefer clear code over clever code.
10. Keep architecture proportional to the size of the project.

Do not over-engineer the application.

---

## Before Starting a Task

Before implementation:

1. Read this `AGENTS.md`.
2. Read the relevant documents in `/docs`.
3. Inspect the existing project structure and implementation.
4. Understand what already works.
5. Confirm the scope of the current task.
6. Avoid implementing unrelated future phases.

If the requested change conflicts with existing project decisions, report the conflict before making a major architectural change.

---

## During a Task

* Modify files relevant to the task.
* Avoid unrelated refactoring.
* Do not redesign unrelated UI.
* Preserve user data.
* Handle relevant failure states.
* Add dependencies only when justified.
* Do not silently change core architecture decisions.

---

## Validation

After implementation, run the appropriate available checks.

Examples may include:

* TypeScript type checking
* frontend build
* Rust `cargo check`
* Tauri build or dev launch
* relevant automated tests

Fix errors introduced by the current change before considering the task complete.

---

## Completion Report

At the end of every task, report:

### What Changed

A concise summary of the implementation.

### Files Changed

Important files created or modified.

### Validation

Commands or checks actually performed and their result.

### Remaining Issues

Known limitations or unfinished items.

If there are no remaining issues, state `None`.

Do not automatically begin the next phase.
