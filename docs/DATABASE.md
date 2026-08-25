# Database

SQLite integration is planned for a later phase. Phase 0 does not implement a schema, migrations, or database access code.

## Planned Data Areas

- Artist
- Track
- Play History
- Sync State
- Tag
- Track Note
- Playlist
- Digging Session

## Principles

- Spotify metadata and personal metadata should be stored separately.
- Personal metadata must not be deleted or overwritten by Spotify sync.
- Store raw data where practical and calculate derived data when needed.
- Discovery should be derived from the earliest observed play history entry for a track.
- Schema changes should use migrations once SQLite is introduced.

The actual SQLite schema will be designed and implemented in a later phase.
