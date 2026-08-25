# Spotify

Spotify integration is planned for a later phase. Phase 0 does not implement OAuth, Spotify API calls, login UI, or sync behavior.

## Planned Principles

- Use the Spotify Web API.
- Use Authorization Code with PKCE.
- Do not use a client secret in the desktop application.
- Do not hardcode secrets.
- Do not log access tokens or refresh tokens.
- Use a loopback OAuth callback based on `127.0.0.1`.
- Request the minimum OAuth scopes required for the current feature.
- Support incremental sync.
- Make sync idempotent.
- Ensure Spotify availability does not block use of the local archive.
- Protect personal metadata from Spotify sync changes.
