mod database;
mod library;
pub mod music;
pub mod spotify;

use tauri::Manager;
use tauri::State;

#[tauri::command]
fn database_health(database: State<'_, database::Database>) -> Result<String, String> {
    database.health()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let database = database::connect(app.handle())?;
            println!(
                "Music Archive database initialized at {}",
                database.path().display()
            );
            app.manage(database);
            app.manage(spotify::SpotifyAuth::new());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            database_health,
            library::library_tracks,
            spotify::spotify_connection_status,
            spotify::spotify_connect,
            spotify::spotify_disconnect,
            spotify::spotify_recently_played,
            spotify::spotify_sync_recently_played
        ])
        .run(tauri::generate_context!())
        .expect("error while running Tauri application");
}
