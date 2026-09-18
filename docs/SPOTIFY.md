# Spotify

Music Archive uses Spotify as a source for listening and music metadata. Spotify
integration is backend-owned: React calls Tauri commands, and the Tauri backend
handles OAuth, token storage, future Spotify API calls, and future sync logic.

## Phase 4 Authentication

Phase 4 implements Spotify account connection only. It does not fetch Recently
Played, playlists, saved tracks, playback state, profile data, or any other
Spotify Web API content.

Authentication uses the Spotify Authorization Code Flow with PKCE:

```text
Connect Spotify
  |
Generate code_verifier, S256 code_challenge, and random state
  |
Bind one-shot loopback callback listener
  |
Open Spotify authorization in the system browser
  |
Validate callback state and authorization code
  |
Exchange code and verifier with Spotify Accounts
  |
Store credentials in OS secure credential storage
```

The PKCE challenge method is `S256`. State validation is required and must not be
disabled.

## Client ID

The Spotify Client ID is configured with the `SPOTIFY_CLIENT_ID` environment
variable before launching the Tauri app:

```sh
SPOTIFY_CLIENT_ID=your_spotify_client_id npm run tauri -- dev
```

PowerShell example:

```powershell
$env:SPOTIFY_CLIENT_ID = "your_spotify_client_id"
npm run tauri -- dev
```

The Client ID is not a secret, but it is kept in one explicit runtime
configuration point. If it is missing or empty, the Settings page reports that
Spotify cannot be connected. Music Archive does not use or configure a Spotify
Client Secret.

## Redirect URI

The registered Spotify redirect URI is:

```text
http://127.0.0.1:8888/callback
```

Use exactly this URI in the Spotify developer app configuration. The backend
binds only to `127.0.0.1` on port `8888` for the duration of the OAuth attempt.
If the port cannot be bound, authentication fails with a useful error instead of
silently switching to another redirect URI.

## Scope

Phase 4 requests only:

```text
user-read-recently-played
```

This is the minimum scope needed for the next planned Recently Played phase.
Additional scopes should be added only when their corresponding features are
implemented.

## Credential Storage

Spotify access and refresh credentials are stored with the Rust `keyring` crate,
using the operating system secure credential store. On Windows, this uses Windows
Credential Manager. Credentials are not stored in SQLite, `.env`, JSON config
files, frontend localStorage, frontend sessionStorage, or source code.

The stored credential record contains the access token, refresh token, expiration
time, and granted scope. Token values, authorization codes, and PKCE verifiers
must never be printed to logs or included in UI/backend error messages.

## Refresh Behavior

The backend exposes reusable logic for retrieving a valid Spotify access token.
If the cached access token is still valid, it is reused. If it is expired or near
expiration, the backend refreshes it through the Spotify Accounts token endpoint,
updates secure credential storage, and returns the new access token to backend
Spotify callers only.

Connection status does not perform a network refresh during normal startup, so
local archive access is not blocked by Spotify availability.

## Disconnect

Disconnect removes locally stored Spotify credentials and clears in-memory
authentication state. It does not delete or modify the Music Archive SQLite
database, tracks, play history, tags, notes, digging data, or playlist archive
data.

## Phase 5 Recently Played

Phase 5 adds read-only backend retrieval for the Spotify Recently Played
endpoint:

```text
GET https://api.spotify.com/v1/me/player/recently-played
```

The required scope remains:

```text
user-read-recently-played
```

No additional Spotify scopes are requested.

The backend reuses `SpotifyAuth::valid_access_token()` before calling the
Spotify Web API, so access token reuse and refresh stay owned by the Phase 4
authentication layer. Raw access tokens, refresh tokens, authorization codes,
and credential storage details are never returned to React.

### Request Parameters

Recently Played supports `limit`, `after`, and `before` for the backend command.
The Phase 5 development default is `limit = 50`, which is also Spotify's maximum
allowed value for this endpoint. Requests with `limit = 0`, `limit > 50`, or
both `after` and `before` are rejected before the HTTP request.

Spotify cursor parameters are Unix timestamps in milliseconds. Music Archive
passes those millisecond cursor values through without converting them to local
time.

### Response Subset

Music Archive parses only the subset currently needed for validation and future
archive persistence:

- play: `played_at`, optional context type, optional context URI
- track: Spotify track ID when present, title, duration, Spotify URL, local-track
  flag, unsupported reason when a track has no Spotify ID
- album: Spotify album ID when present, album name, first artwork URL when present
- artists: Spotify artist ID when present, name, Spotify URL when present
- cursors: `after`, `before`, plus Spotify's `next` value when returned

`played_at` is preserved as Spotify's UTC RFC3339 timestamp string. The backend
does not convert it to local time; later UI surfaces can format it for display.

### Optional and Unsupported Data

Spotify fields such as playback context, album artwork, external URLs, album IDs,
artist IDs, and track IDs may be absent. The Phase 5 parser treats these fields
as optional and does not invent placeholder Spotify IDs.

Local or otherwise unsupported track objects without a Spotify track ID are
returned with an `unsupported_reason`. They are visible in the development
response, but Phase 6 persistence can deliberately skip them.

### Errors and Rate Limits

The API layer maps not-connected/token-refresh failures, network failures,
Spotify `401`, `403`, `429`, other non-success statuses, invalid request
parameters, and invalid response bodies into frontend-safe messages. For `429 Too
Many Requests`, the safe error includes Spotify's `Retry-After` value when one is
provided. Phase 5 does not automatically retry Recently Played requests.

## Phase 6 Listening History Sync

Phase 6 persists Spotify Recently Played responses into SQLite through the
backend command:

```text
spotify_sync_recently_played
```

React triggers the command and receives a safe summary. It does not receive
tokens, raw SQL, or internal database state.

The sync flow is:

```text
Read sync_state cursor
  |
Fetch Recently Played through the Phase 5 API layer
  |
Open SQLite transaction
  |
Upsert artists and tracks
  |
Refresh track_artists ordering
  |
Insert play_history rows with duplicate protection
  |
Update sync_state
  |
Commit
```

The Spotify network request is completed before the persistence transaction is
opened.

### First Sync

If no `sync_state` row exists for `recently_played`, Music Archive requests up
to Spotify's current Recently Played maximum of 50 items and archives the valid
playable items returned by Spotify. It does not invent older listening history or
attempt to recover plays that Spotify no longer exposes.

### Subsequent Syncs

After a successful sync, `sync_state.last_processed_played_at` stores the latest
Spotify `played_at` timestamp that was processed. Later syncs use that timestamp
to build the Spotify `after` cursor in milliseconds, with a tiny one-millisecond
overlap so boundary plays can be retried safely. Duplicate protection in SQLite
keeps the overlap idempotent.

`last_successful_sync_at` is the wall-clock time when the sync completed.
`last_processed_played_at` is the latest Spotify play timestamp processed. These
values are intentionally separate.

### Idempotency

`play_history` has a unique constraint on `(track_id, played_at)`. Phase 6 uses
that constraint intentionally with ignored duplicate inserts, so running sync
again with the same Spotify data creates no duplicate play rows and is treated
as a successful sync.

Artists and tracks are upserted by their Spotify IDs. Track artist relations are
refreshed for the current Spotify metadata while preserving Spotify artist
ordering.

### Unsupported Items

Local tracks or malformed Spotify track objects without a usable Spotify track
ID are skipped for archive persistence. Music Archive increments
`skipped_items`, continues syncing the remaining valid items, and never creates
fake Spotify identifiers.

### Error Behavior

If Spotify authentication, the Spotify API request, rate limiting, response
parsing, or database persistence fails, the frontend receives a safe error
message. Existing local archive data remains available. Database persistence
uses a transaction, so a failed batch does not leave partially inserted artists,
tracks, relations, plays, or successful sync-state updates behind.

### Availability Limitation

Music Archive can archive only listening events that Spotify's Recently Played
API still exposes when sync runs. If the app is not synced for a long period,
some Spotify listening history may no longer be available from Spotify and
cannot necessarily be reconstructed by Music Archive.

## Playlist import (MVP)

Read-only endpoints: GET /v1/me, GET /v1/me/playlists and
GET /v1/playlists/{id}/items. The importer uses 50-item pages and checks the
playlist snapshot again after fetching items. It supports current `item` and
legacy `track` payloads, skipping null, local, and non-track items. A failed
snapshot transaction rolls back. Playlist import never creates listening plays.

OAuth now requests user-read-recently-played and playlist-read-private. Existing
users can use Settings > Reconnect for playlist access to approve the added
read permission. Spotify may still deny items for playlists the user does not
own or collaborate on. Metadata and prior archive remain available.

Sources checked during implementation:
https://developer.spotify.com/documentation/web-api/reference/get-a-list-of-current-users-playlists
https://developer.spotify.com/documentation/web-api/reference/get-playlists-items

HTTP 401, 403, 429 (Retry-After), network, and malformed responses produce safe
errors. No automatic retry loop or Spotify write endpoint is used.
