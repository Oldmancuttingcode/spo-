pub const REDIRECT_URI: &str = "http://127.0.0.1:8888/callback";
pub const CALLBACK_HOST: &str = "127.0.0.1";
pub const CALLBACK_PORT: u16 = 8888;
pub const CALLBACK_PATH: &str = "/callback";
pub const AUTHORIZATION_URL: &str = "https://accounts.spotify.com/authorize";
pub const TOKEN_URL: &str = "https://accounts.spotify.com/api/token";
pub const PHASE_FOUR_SCOPE: &str = "user-read-recently-played";

const CLIENT_ID_ENV_VAR: &str = "SPOTIFY_CLIENT_ID";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpotifyConfig {
    pub client_id: String,
}

pub fn load_config() -> Result<SpotifyConfig, String> {
    let client_id = std::env::var(CLIENT_ID_ENV_VAR).map_err(|_| {
        format!(
            "Spotify Client ID is not configured. Set {CLIENT_ID_ENV_VAR} before connecting Spotify."
        )
    })?;

    validate_client_id(&client_id)?;
    Ok(SpotifyConfig { client_id })
}

pub fn validate_client_id(client_id: &str) -> Result<(), String> {
    if client_id.trim().is_empty() {
        return Err(format!(
            "Spotify Client ID is empty. Set {CLIENT_ID_ENV_VAR} to your Spotify app Client ID."
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::validate_client_id;

    #[test]
    fn client_id_must_not_be_empty() {
        assert!(validate_client_id("").is_err());
        assert!(validate_client_id("   ").is_err());
    }

    #[test]
    fn non_empty_client_id_is_valid() {
        assert!(validate_client_id("abc123").is_ok());
    }
}
