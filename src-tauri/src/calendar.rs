use crate::database::Database;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use tauri::State;

#[derive(Debug, Serialize)]
pub struct TrackRow {
    pub id: i64,
    pub title: String,
    pub artist_names: String,
    pub album_art_url: Option<String>,
    pub play_count: i64,
    pub discovered: bool,
}

pub(crate) fn track_rows(c: &Connection, ids_sql: &str, id: i64) -> Result<Vec<TrackRow>, String> {
    // ids_sql is a static application query, never frontend input.
    let sql = format!("SELECT t.id,t.title,COALESCE((SELECT group_concat(a.name, ', ' ORDER BY ta.artist_order) FROM track_artists ta JOIN artists a ON a.id=ta.artist_id WHERE ta.track_id=t.id),''),t.album_art_url FROM tracks t WHERE t.id IN ({ids_sql}) ORDER BY t.title COLLATE NOCASE,t.id");
    let mut stmt = c.prepare(&sql).map_err(|_| "Could not load tracks.")?;
    let rows = stmt
        .query_map([id], |r| {
            Ok(TrackRow {
                id: r.get(0)?,
                title: r.get(1)?,
                artist_names: r.get(2)?,
                album_art_url: r.get(3)?,
                play_count: 0,
                discovered: false,
            })
        })
        .map_err(|_| "Could not load tracks.")?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|_| "Could not load tracks.".into())
}
#[derive(Deserialize)]
pub struct DayRange {
    pub date: String,
    pub start: String,
    pub end: String,
}
#[derive(Debug, Serialize)]
pub struct Day {
    pub date: String,
    pub plays: i64,
    pub discoveries: i64,
    pub tracks: Vec<TrackRow>,
}
fn day(c: &Connection, range: &DayRange) -> Result<Day, String> {
    let valid: bool = c
        .query_row(
            "SELECT COALESCE(julianday(?2)>julianday(?1) AND julianday(?2)-julianday(?1)<=2,0)",
            params![range.start, range.end],
            |r| r.get(0),
        )
        .map_err(|_| "Could not read calendar.")?;
    if !valid {
        return Err("Invalid calendar date range.".into());
    }
    let mut stmt=c.prepare("SELECT t.id,t.title,COALESCE((SELECT group_concat(a.name, ', ' ORDER BY ta.artist_order) FROM track_artists ta JOIN artists a ON a.id=ta.artist_id WHERE ta.track_id=t.id),''),t.album_art_url,COUNT(p.id),(SELECT MIN(julianday(played_at)) FROM play_history WHERE track_id=t.id)>=julianday(?1) FROM play_history p JOIN tracks t ON t.id=p.track_id WHERE julianday(p.played_at)>=julianday(?1) AND julianday(p.played_at)<julianday(?2) GROUP BY t.id ORDER BY COUNT(p.id) DESC,t.title,t.id").map_err(|_| "Could not read calendar.")?;
    let tracks = stmt
        .query_map(params![range.start, range.end], |r| {
            Ok(TrackRow {
                id: r.get(0)?,
                title: r.get(1)?,
                artist_names: r.get(2)?,
                album_art_url: r.get(3)?,
                play_count: r.get(4)?,
                discovered: r.get(5)?,
            })
        })
        .map_err(|_| "Could not read calendar.")?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| "Could not read calendar.")?;
    Ok(Day {
        date: range.date.clone(),
        plays: tracks.iter().map(|t| t.play_count).sum(),
        discoveries: tracks.iter().filter(|t| t.discovered).count() as i64,
        tracks,
    })
}
#[tauri::command]
pub fn calendar_days(
    database: State<'_, Database>,
    ranges: Vec<DayRange>,
) -> Result<Vec<Day>, String> {
    if ranges.len() > 42 {
        return Err("Request at most 42 calendar days.".into());
    }
    database.with_connection(|c| ranges.iter().map(|r| day(c, r)).collect())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn daylight_saving_day_can_have_twenty_three_hours() {
        let mut c = Connection::open_in_memory().unwrap();
        crate::database::migrations::run(&mut c, ":memory:".as_ref()).unwrap();
        c.execute_batch("INSERT INTO tracks(id,spotify_id,title) VALUES(1,'a','A'); INSERT INTO play_history(track_id,played_at,synced_at) VALUES(1,'2026-03-08T05:00:00Z','now'),(1,'2026-03-09T03:59:59Z','now'),(1,'2026-03-09T04:00:00Z','now');").unwrap();
        let result = day(
            &c,
            &DayRange {
                date: "2026-03-08".into(),
                start: "2026-03-08T05:00:00Z".into(),
                end: "2026-03-09T04:00:00Z".into(),
            },
        )
        .unwrap();
        assert_eq!(result.plays, 2);
        assert_eq!(result.discoveries, 1);
    }
    #[test]
    fn local_day_boundary_and_discovery_use_entire_history() {
        let mut c = Connection::open_in_memory().unwrap();
        crate::database::migrations::run(&mut c, ":memory:".as_ref()).unwrap();
        c.execute_batch("INSERT INTO tracks(id,spotify_id,title) VALUES(1,'a','A'),(2,'b','B'); INSERT INTO play_history(track_id,played_at,synced_at) VALUES(1,'2026-09-14T15:00:00Z','now'),(1,'2026-09-15T15:00:00Z','now'),(2,'2026-09-15T14:59:59Z','now'),(2,'2026-09-16T15:00:00Z','now');").unwrap();
        let d = day(
            &c,
            &DayRange {
                date: "2026-09-16".into(),
                start: "2026-09-15T15:00:00Z".into(),
                end: "2026-09-16T15:00:00Z".into(),
            },
        )
        .unwrap();
        assert_eq!(d.plays, 1);
        assert_eq!(d.discoveries, 0);
        assert_eq!(d.tracks[0].id, 1);
        assert!(day(
            &c,
            &DayRange {
                date: "bad".into(),
                start: "bad".into(),
                end: "bad".into()
            }
        )
        .is_err());
    }
}
