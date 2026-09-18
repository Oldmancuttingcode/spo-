mod calendar;
mod database;
mod digging;
mod library;
pub mod music;
mod notes;
mod playlists;
pub mod spotify;
mod sync_status;
mod tags;
mod track_detail;

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
            notes::track_note,
            sync_status::archive_sync_status,
            notes::save_track_note,
            calendar::calendar_days,
            spotify::playlists::spotify_sync_playlists,
            playlists::archived_playlists,
            playlists::playlist_detail,
            playlists::save_playlist_notes,
            playlists::set_playlist_tag,
            playlists::open_playlist_in_spotify,
            digging::digging_sessions,
            digging::digging_detail,
            digging::save_digging,
            digging::set_digging_track,
            digging::delete_digging,
            track_detail::track_detail,
            track_detail::open_track_in_spotify,
            tags::track_tags,
            tags::list_tags,
            tags::create_tag,
            tags::assign_tag_to_track,
            tags::remove_tag_from_track,
            spotify::spotify_connection_status,
            spotify::spotify_connect,
            spotify::spotify_disconnect,
            spotify::spotify_recently_played,
            spotify::spotify_sync_recently_played
        ])
        .run(tauri::generate_context!())
        .expect("error while running Tauri application");
}
