use crate::{
    calendar::{track_rows, TrackRow},
    database::Database,
};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use tauri::State;

#[derive(Serialize)]
pub struct Digging {
    id: i64,
    name: String,
    start_date: String,
    end_date: Option<String>,
    description: String,
    track_count: i64,
    artist_count: i64,
}
#[derive(Deserialize)]
pub struct DiggingInput {
    id: Option<i64>,
    name: String,
    start_date: String,
    end_date: Option<String>,
    description: String,
}
#[derive(Serialize)]
pub struct DiggingDetail {
    session: Digging,
    tracks: Vec<TrackRow>,
}
const SELECT:&str="SELECT d.id,d.name,d.start_date,d.end_date,COALESCE(d.description,''),(SELECT COUNT(*) FROM digging_tracks WHERE digging_id=d.id),(SELECT COUNT(DISTINCT ta.artist_id) FROM digging_tracks dt JOIN track_artists ta ON ta.track_id=dt.track_id WHERE dt.digging_id=d.id) FROM digging_sessions d";
fn row(r: &rusqlite::Row<'_>) -> rusqlite::Result<Digging> {
    Ok(Digging {
        id: r.get(0)?,
        name: r.get(1)?,
        start_date: r.get(2)?,
        end_date: r.get(3)?,
        description: r.get(4)?,
        track_count: r.get(5)?,
        artist_count: r.get(6)?,
    })
}
fn valid_date(c: &Connection, date: &str) -> bool {
    date.len() == 10
        && date.as_bytes()[4] == b'-'
        && date.as_bytes()[7] == b'-'
        && date
            .chars()
            .enumerate()
            .all(|(i, ch)| i == 4 || i == 7 || ch.is_ascii_digit())
        && &date[..4] != "0000"
        && c.query_row("SELECT COALESCE(date(?1,'+0 days')=?1,0)", [date], |r| {
            r.get::<_, bool>(0)
        })
        .unwrap_or(false)
}
fn save(c: &Connection, input: &DiggingInput) -> Result<i64, String> {
    if input.name.trim().is_empty() || input.name.chars().count() > 200 {
        return Err("Enter a digging name (up to 200 characters).".into());
    }
    if input.description.chars().count() > 50000 {
        return Err("Description is limited to 50,000 characters.".into());
    }
    if !valid_date(c, &input.start_date)
        || input
            .end_date
            .as_deref()
            .is_some_and(|d| !valid_date(c, d) || d < input.start_date.as_str())
    {
        return Err("Use valid dates; end date must be on or after start date.".into());
    }
    if let Some(id) = input.id {
        let n=c.execute("UPDATE digging_sessions SET name=?2,start_date=?3,end_date=?4,description=?5,updated_at=strftime('%Y-%m-%dT%H:%M:%fZ','now') WHERE id=?1",params![id,input.name.trim(),input.start_date,input.end_date,input.description]).map_err(|_|"Could not save digging.")?;
        if n == 0 {
            return Err("Digging session not found.".into());
        }
        Ok(id)
    } else {
        c.execute("INSERT INTO digging_sessions(name,start_date,end_date,description) VALUES(?1,?2,?3,?4)",params![input.name.trim(),input.start_date,input.end_date,input.description]).map_err(|_|"Could not create digging.")?;
        Ok(c.last_insert_rowid())
    }
}
#[tauri::command]
pub fn digging_sessions(database: State<'_, Database>) -> Result<Vec<Digging>, String> {
    database.with_connection(|c| {
        let mut stmt = c
            .prepare(&format!(
                "{SELECT} ORDER BY d.end_date IS NOT NULL,d.start_date DESC,d.id DESC"
            ))
            .map_err(|_| "Could not load digging sessions.")?;
        let rows = stmt
            .query_map([], row)
            .map_err(|_| "Could not load digging sessions.")?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|_| "Could not read digging sessions.".into())
    })
}
#[tauri::command]
pub fn digging_detail(
    database: State<'_, Database>,
    digging_id: i64,
) -> Result<DiggingDetail, String> {
    database.with_connection(|c| {
        let session = c
            .query_row(&format!("{SELECT} WHERE d.id=?1"), [digging_id], row)
            .optional()
            .map_err(|_| "Could not load digging.")?
            .ok_or("Digging session not found.")?;
        let tracks = track_rows(
            c,
            "SELECT track_id FROM digging_tracks WHERE digging_id=?1",
            digging_id,
        )?;
        Ok(DiggingDetail { session, tracks })
    })
}
#[tauri::command]
pub fn save_digging(database: State<'_, Database>, input: DiggingInput) -> Result<i64, String> {
    database.with_connection(|c| save(c, &input))
}
#[tauri::command]
pub fn set_digging_track(
    database: State<'_, Database>,
    digging_id: i64,
    track_id: i64,
    assigned: bool,
) -> Result<(), String> {
    database.with_connection(|c|c.execute(if assigned{"INSERT INTO digging_tracks(digging_id,track_id) VALUES(?1,?2) ON CONFLICT DO NOTHING"}else{"DELETE FROM digging_tracks WHERE digging_id=?1 AND track_id=?2"},params![digging_id,track_id]).map(|_|()).map_err(|_|"Could not update digging track.".into()))
}
#[tauri::command]
pub fn delete_digging(database: State<'_, Database>, digging_id: i64) -> Result<(), String> {
    database.with_connection(|c| {
        c.execute("DELETE FROM digging_sessions WHERE id=?1", [digging_id])
            .map(|_| ())
            .map_err(|_| "Could not delete digging session.".into())
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn dates_and_end_preserve_membership() {
        let mut c = Connection::open_in_memory().unwrap();
        crate::database::migrations::run(&mut c, ":memory:".as_ref()).unwrap();
        let mut input = DiggingInput {
            id: None,
            name: " Soul ".into(),
            start_date: "2026-02-28".into(),
            end_date: None,
            description: "Exploring".into(),
        };
        assert!(!valid_date(&c, "2026-02-29"));
        assert!(valid_date(&c, "2024-02-29"));
        let id = save(&c, &input).unwrap();
        input.id = Some(id);
        input.end_date = Some("2026-02-27".into());
        assert!(save(&c, &input).is_err());
        c.execute_batch("INSERT INTO tracks(id,spotify_id,title) VALUES(1,'a','A');INSERT INTO digging_tracks(digging_id,track_id) VALUES(1,1);").unwrap();
        input.end_date = Some("2026-03-01".into());
        save(&c, &input).unwrap();
        assert_eq!(
            track_rows(
                &c,
                "SELECT track_id FROM digging_tracks WHERE digging_id=?1",
                id
            )
            .unwrap()
            .len(),
            1
        );
    }
}
