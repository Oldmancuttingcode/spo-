mod api;
mod auth;
mod config;
mod credentials;
pub mod playlists;
mod sync;

use serde::Serialize;
use tauri::{Manager, State};

use crate::database::Database;

pub use api::{recently_played, RecentlyPlayedResponse};
pub use auth::SpotifyAuth;
pub use sync::ListeningHistorySyncResult;

#[derive(Debug, Serialize)]
pub struct SpotifyConnectionStatus {
    state: &'static str,
    message: Option<String>,
}

impl SpotifyConnectionStatus {
    fn connected() -> Self {
        Self {
            state: "connected",
            message: None,
        }
    }

    fn not_connected() -> Self {
        Self {
            state: "not_connected",
            message: None,
        }
    }

    fn authentication_error(message: impl Into<String>) -> Self {
        Self {
            state: "authentication_error",
            message: Some(message.into()),
        }
    }
}

#[tauri::command]
pub fn spotify_connection_status(spotify_auth: State<'_, SpotifyAuth>) -> SpotifyConnectionStatus {
    spotify_auth.connection_status()
}

#[tauri::command]
pub async fn spotify_connect(app: tauri::AppHandle) -> Result<SpotifyConnectionStatus, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let auth = app.state::<SpotifyAuth>();
        auth.connect()?;
        Ok(auth.connection_status())
    })
    .await
    .map_err(|_| "Spotify connection stopped unexpectedly.".to_string())?
}

#[tauri::command]
pub fn spotify_disconnect(
    spotify_auth: State<'_, SpotifyAuth>,
) -> Result<SpotifyConnectionStatus, String> {
    spotify_auth.disconnect()?;
    Ok(spotify_auth.connection_status())
}

#[tauri::command]
pub fn spotify_recently_played(
    spotify_auth: State<'_, SpotifyAuth>,
    limit: Option<u32>,
    after: Option<u64>,
    before: Option<u64>,
) -> Result<RecentlyPlayedResponse, String> {
    let access_token = spotify_auth.valid_access_token()?;
    recently_played(&access_token, limit, after, before).map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn spotify_sync_recently_played(
    app: tauri::AppHandle,
) -> Result<ListeningHistorySyncResult, String> {
    tauri::async_runtime::spawn_blocking(move || {
        static SYNC: std::sync::Mutex<()> = std::sync::Mutex::new(());
        let _guard = SYNC
            .try_lock()
            .map_err(|_| "Listening history sync is already running.".to_string())?;
        sync_recently_played(app.state::<SpotifyAuth>(), app.state::<Database>())
    })
    .await
    .map_err(|_| "Listening sync stopped unexpectedly.".to_string())?
}

fn sync_recently_played(
    spotify_auth: State<'_, SpotifyAuth>,
    database: State<'_, Database>,
) -> Result<ListeningHistorySyncResult, String> {
    let cursor = database.with_connection(|connection| sync::read_sync_cursor(connection))?;
    let access_token = spotify_auth.valid_access_token().map_err(|error| {
        let _ =
            database.with_connection(|connection| sync::record_sync_failure(connection, &error));
        error
    })?;

    let response =
        api::recently_played(&access_token, Some(50), cursor.after_ms, None).map_err(|error| {
            let error_message = error.to_string();
            let _ = database.with_connection(|connection| {
                sync::record_sync_failure(connection, &error_message)
            });
            error_message
        })?;

    match database
        .with_connection(|connection| sync::persist_recently_played(connection, &response))
    {
        Ok(result) => Ok(result),
        Err(error) => {
            let _ = database
                .with_connection(|connection| sync::record_sync_failure(connection, &error));
            Err(error)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{api, SpotifyAuth};

    #[test]
    #[ignore = "uses the locally connected Spotify account and real Spotify Web API"]
    fn real_spotify_recently_played_returns_tracks() {
        let auth = SpotifyAuth::new();
        let access_token = auth.valid_access_token().expect("Spotify is connected");
        let response =
            api::recently_played(&access_token, Some(10), None, None).expect("Spotify API works");

        assert!(
            !response.items.is_empty(),
            "recent plays should be returned"
        );
        for item in &response.items {
            assert!(item.played_at.ends_with('Z'));
            assert!(!item.track.title.trim().is_empty());
        }

        println!("recent_play_count={}", response.items.len());
        for item in response.items.iter().take(3) {
            let artists = item
                .track
                .artists
                .iter()
                .map(|artist| artist.name.as_str())
                .collect::<Vec<_>>()
                .join(", ");
            println!(
                "recent_play title={:?} artists={:?} played_at={}",
                item.track.title, artists, item.played_at
            );
        }
    }
}
