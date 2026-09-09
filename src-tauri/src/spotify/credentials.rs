use std::time::{SystemTime, UNIX_EPOCH};

use keyring::{Entry, Error as KeyringError};
use serde::{Deserialize, Serialize};

const KEYRING_SERVICE: &str = "Music Archive";
const KEYRING_ACCOUNT: &str = "spotify-oauth";
const EXPIRATION_LEEWAY_SECONDS: u64 = 60;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpotifyCredentials {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_at_unix_seconds: u64,
    pub scope: String,
}

impl SpotifyCredentials {
    pub fn access_token_is_valid_at(&self, now_unix_seconds: u64) -> bool {
        self.expires_at_unix_seconds > now_unix_seconds.saturating_add(EXPIRATION_LEEWAY_SECONDS)
    }
}

pub trait CredentialStore: Send + Sync {
    fn load(&self) -> Result<Option<SpotifyCredentials>, String>;
    fn save(&self, credentials: &SpotifyCredentials) -> Result<(), String>;
    fn delete(&self) -> Result<(), String>;
}

pub struct KeyringCredentialStore;

impl CredentialStore for KeyringCredentialStore {
    fn load(&self) -> Result<Option<SpotifyCredentials>, String> {
        let entry = credential_entry()?;
        match entry.get_password() {
            Ok(serialized) => serde_json::from_str(&serialized)
                .map(Some)
                .map_err(|_| "Stored Spotify credentials could not be read.".to_string()),
            Err(KeyringError::NoEntry) => Ok(None),
            Err(error) => Err(format!(
                "Could not read Spotify credentials from secure storage: {error}"
            )),
        }
    }

    fn save(&self, credentials: &SpotifyCredentials) -> Result<(), String> {
        let entry = credential_entry()?;
        let serialized = serde_json::to_string(credentials).map_err(|_| {
            "Spotify credentials could not be prepared for secure storage.".to_string()
        })?;

        entry
            .set_password(&serialized)
            .map_err(|error| format!("Could not save Spotify credentials securely: {error}"))
    }

    fn delete(&self) -> Result<(), String> {
        let entry = credential_entry()?;
        match entry.delete_credential() {
            Ok(()) | Err(KeyringError::NoEntry) => Ok(()),
            Err(error) => Err(format!(
                "Could not remove Spotify credentials from secure storage: {error}"
            )),
        }
    }
}

pub fn now_unix_seconds() -> Result<u64, String> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .map_err(|_| "System clock is before the Unix epoch.".to_string())
}

fn credential_entry() -> Result<Entry, String> {
    Entry::new(KEYRING_SERVICE, KEYRING_ACCOUNT)
        .map_err(|error| format!("Could not open secure Spotify credential storage: {error}"))
}

#[cfg(test)]
mod tests {
    use super::SpotifyCredentials;

    fn credentials(expires_at_unix_seconds: u64) -> SpotifyCredentials {
        SpotifyCredentials {
            access_token: "access".to_string(),
            refresh_token: "refresh".to_string(),
            expires_at_unix_seconds,
            scope: "user-read-recently-played".to_string(),
        }
    }

    #[test]
    fn access_token_validity_uses_expiration_leeway() {
        assert!(credentials(1_000).access_token_is_valid_at(900));
        assert!(!credentials(1_000).access_token_is_valid_at(940));
        assert!(!credentials(1_000).access_token_is_valid_at(1_000));
    }
}
