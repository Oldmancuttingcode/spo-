mod auth;
mod config;
mod credentials;

use serde::Serialize;
use tauri::State;

pub use auth::SpotifyAuth;

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
pub fn spotify_connect(
    spotify_auth: State<'_, SpotifyAuth>,
) -> Result<SpotifyConnectionStatus, String> {
    spotify_auth.connect()?;
    Ok(spotify_auth.connection_status())
}

#[tauri::command]
pub fn spotify_disconnect(
    spotify_auth: State<'_, SpotifyAuth>,
) -> Result<SpotifyConnectionStatus, String> {
    spotify_auth.disconnect()?;
    Ok(spotify_auth.connection_status())
}
