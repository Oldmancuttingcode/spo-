# MVP validation — 2026-09-18

## Implemented scope

| Phase | Result |
|---|---|
| 10 Track Notes | Inline load/save/edit/delete; explicit save; unsaved-change warning |
| 11 Calendar | Monthly navigation, local-day counts, discovery and per-day tracks |
| 12 Spotify Playlists | Paginated metadata/items import, snapshot checks, per-playlist transactions |
| 13 Playlist Archive | Local details, ordered repeated tracks, description/memo, tags, Spotify link |
| 14 Digging | Create/edit/end/delete sessions; add/remove tracks; active/past lists |
| 15 Home | Current-month summary, discoveries, active digging, recent tracks |
| 16 Sync/Error UX | Persisted sync status, safe API errors, reconnect, background command execution |
| 17 UI Polish | Shared track rows/artwork, light theme, spacing/focus/empty states, 800px layout |
| 18 Stabilization | Automated regression tests and isolated desktop/SQLite validation below |

## Checks actually run

- `cargo fmt`, `cargo check`, `cargo test`: 59 passed, 4 intentionally ignored.
- `npm run build`: passed with the default Vite configuration loader.
- `git diff --check`: passed.
- Tauri development app launched with identifier `com.local.musicarchive.mvpvalidation` and a copy of the original database.
- Actual WebView checks: note save with Korean/multiline text; Calendar; Digging creation, track assignment, and end date; playlist personal description; duplicate-safe playlist tags; local Home browsing offline; 800px Calendar without horizontal overflow.
- Complete app shutdown/restart preserved note, ended digging, track membership, playlist description and tags.
- Spotify reauthorization succeeded. Real import: **40 synced, 0 unavailable**. Repeat import: **0 synced, 40 unchanged, 0 unavailable**. Personal-data checks still passed after import.
- Independent read-only SQLite audit: integrity `ok`, no foreign-key violations, migrations `[1,2,3]`, 41 playlists (40 real + 1 fixture), 3,646 current membership rows.
- September 2026 Asia/Seoul totals: 151 plays and 116 discoveries, independently checked with SQL.
- Unit regressions cover UTC/local-midnight boundaries, 23-hour DST day, invalid dates, note replacement/removal, playlist repeated tracks, old snapshot preservation, failed-import rollback, and existing sync personal-data preservation.

## Data safety

The production archive was never migrated or edited during these checks. Its
SHA-256 remained:
`B77385DBF4ABA91834F6A3FF540B8FC0457078DCB8D70F654ECC5FD4BA5678FE`.
Migration 3 was applied only to the disposable copy. The normal app applies it
on its next launch, preserving previous playlist membership before refresh.
No dependencies were added. No commit or push was performed.

## Limits

- This validates the development app, not a packaged Windows installer.
- Spotify can restrict playlists the user does not own or collaborate on. A real 403 occurred before reauthorization; all 40 accessible playlists succeeded afterward.
- Previous snapshot membership is retained in SQLite; no historical snapshot browser is included.
- Every transient network/rate-limit path was not reproduced against the live service. No automated retry loop is used.
- The UI smoke script assumes the September 2026 fixture and the isolated app/port; it must never be pointed at the production app.

## Reproduction

Run the app with a separate identifier and database copy on dev port 1422,
WebView debugging port 9224. Then run `py -3 scripts/seed-validation.py` and
`node scripts/validate-mvp.mjs`. After a complete restart run with `--restart`.
Use `--connect` for user-approved OAuth, `--spotify` for live read-only import,
and `py -3 scripts/audit-validation.py` for a read-only SQLite audit.
