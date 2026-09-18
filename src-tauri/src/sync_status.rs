use crate::database::Database;
use serde::Serialize;
use tauri::State;
#[derive(Serialize)]
pub struct SyncStatus {
    source: String,
    last_successful_sync_at: Option<String>,
    sync_status: String,
    last_error: Option<String>,
}
#[tauri::command]
pub fn archive_sync_status(database: State<'_, Database>) -> Result<Vec<SyncStatus>, String> {
    database.with_connection(|c|{let mut stmt=c.prepare("SELECT source,last_successful_sync_at,sync_status,last_error FROM sync_state ORDER BY source").map_err(|_|"Could not load sync status.")?;let rows=stmt.query_map([],|r|Ok(SyncStatus{source:r.get(0)?,last_successful_sync_at:r.get(1)?,sync_status:r.get(2)?,last_error:r.get(3)?})).map_err(|_|"Could not load sync status.")?;rows.collect::<Result<Vec<_>,_>>().map_err(|_|"Could not read sync status.".into())})
}
