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
