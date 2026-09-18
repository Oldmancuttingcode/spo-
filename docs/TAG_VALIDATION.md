# Phase 9 validation

## Automated checks

Run `cargo fmt --check`, `cargo check`, and `cargo test` in `src-tauri`,
then `npm run build` and `git diff --check` in the repository root.
Tag tests cover all five categories, trimmed names, Unicode lowercase duplicate
matching, category separation, invalid input, duplicate assignments, shared-tag
removal, and persistence after reopening SQLite. The sync regression test checks
that the tag name, category, and track relationship survive metadata refresh.

## Desktop checks

Use a disposable archive when testing creation and removal:

1. Open a track and verify Genre, Mood, Sound, Vocal, and Free Tag groups.
2. Create a tag in each category. Navigate away and back to verify persistence.
3. Create `Dark`, then ` dark ` in the same category; the existing tag is reused.
   The same name in a different category is allowed.
4. Assign an existing tag to two tracks. Remove it from one track and verify the
   other assignment and the reusable tag remain.
5. Restart the app and verify saved assignments and removals.
6. Test Enter for creation/exact-match assignment, Escape to close, pending
   controls, and retries after tag-list failures.
7. Repeat offline, with Spotify unavailable. Local tag operations must work.

## 2026-09-16 verification

Rust formatting/check/tests passed (53 passed, 4 intentionally ignored).
`git diff --check` passed. The default frontend build was blocked by the sandbox
denying esbuild access to a parent directory; `npm run build -- --configLoader
native` passed using Node 24's native configuration loader.

The initial sandboxed Tauri launch was denied by Windows. The follow-up dev run
succeeded outside that sandbox using a separate application identifier
(`com.local.musicarchive.phase9validation`), port 1421, and a copy of the archive.
Actual WebView UI checks passed for all five categories, creating and assigning
existing tags, case/whitespace duplicate reuse, duplicate assignment, navigation
persistence, removal, shared-tag preservation, and offline editing. A complete
app shutdown/restart preserved the saved assignments and removals. The rendered
Track Detail screenshot was inspected. Keyboard and injected-error scenarios
remain checklist items rather than verified manual results in this run.

The original archive SHA-256 remained unchanged:
`B77385DBF4ABA91834F6A3FF540B8FC0457078DCB8D70F654ECC5FD4BA5678FE`.
No migrations or dependencies were added.

