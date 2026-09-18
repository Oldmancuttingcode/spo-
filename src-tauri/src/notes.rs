use crate::database::Database;
use rusqlite::{params, Connection, OptionalExtension};
use tauri::State;

pub(crate) fn save(connection: &Connection, track_id: i64, note: &str) -> Result<(), String> {
    let exists: bool = connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM tracks WHERE id=?1)",
            [track_id],
            |r| r.get(0),
        )
        .map_err(|_| "Could not read track.")?;
    if !exists {
        return Err("Track not found.".into());
    }
    if note.chars().count() > 50000 {
        return Err("Notes are limited to 50,000 characters.".into());
    }
    if note.trim().is_empty() {
        connection.execute("DELETE FROM track_notes WHERE track_id=?1", [track_id])
    } else {
        connection.execute("INSERT INTO track_notes(track_id,note) VALUES(?1,?2) ON CONFLICT(track_id) DO UPDATE SET note=excluded.note, updated_at=strftime('%Y-%m-%dT%H:%M:%fZ','now')", params![track_id,note])
    }.map_err(|_| "Could not save your note. Please retry.".to_string())?;
    Ok(())
}

#[tauri::command]
pub fn track_note(database: State<'_, Database>, track_id: i64) -> Result<String, String> {
    database.with_connection(|c| {
        c.query_row(
            "SELECT note FROM track_notes WHERE track_id=?1",
            [track_id],
            |r| r.get(0),
        )
        .optional()
        .map(|n| n.unwrap_or_default())
        .map_err(|_| "Could not load your note.".into())
    })
}

#[tauri::command]
pub fn save_track_note(
    database: State<'_, Database>,
    track_id: i64,
    note: String,
) -> Result<(), String> {
    database.with_connection(|c| save(c, track_id, &note))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn note_upsert_delete_preserves_track_and_other_notes() {
        let mut c = Connection::open_in_memory().unwrap();
        crate::database::migrations::run(&mut c, ":memory:".as_ref()).unwrap();
        c.execute_batch("INSERT INTO tracks(id,spotify_id,title) VALUES(1,'a','A'),(2,'b','B')")
            .unwrap();
        save(&c, 1, "한글\n  note").unwrap();
        save(&c, 2, "other").unwrap();
        save(&c, 1, "edited").unwrap();
        assert_eq!(
            c.query_row("SELECT note FROM track_notes WHERE track_id=1", [], |r| {
                r.get::<_, String>(0)
            })
            .unwrap(),
            "edited"
        );
        save(&c, 1, "").unwrap();
        assert_eq!(
            c.query_row("SELECT COUNT(*) FROM track_notes", [], |r| r
                .get::<_, i64>(0))
                .unwrap(),
            1
        );
        assert!(save(&c, 999, "bad").is_err());
        assert_eq!(
            c.query_row("SELECT COUNT(*) FROM tracks", [], |r| r.get::<_, i64>(0))
                .unwrap(),
            2
        );
    }
}
