use std::{
    collections::HashMap,
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    sync::Mutex,
    thread,
    time::{Duration, Instant},
};

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use rand::{rngs::OsRng, RngCore};
use reqwest::blocking::Client;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use url::Url;

use super::{
    config::{
        load_config, AUTHORIZATION_URL, CALLBACK_HOST, CALLBACK_PATH, CALLBACK_PORT,
        PHASE_FOUR_SCOPE, REDIRECT_URI, TOKEN_URL,
    },
    credentials::{now_unix_seconds, CredentialStore, KeyringCredentialStore, SpotifyCredentials},
    SpotifyConnectionStatus,
};

const OAUTH_TIMEOUT: Duration = Duration::from_secs(300);
const HTTP_POLL_INTERVAL: Duration = Duration::from_millis(100);

pub struct SpotifyAuth {
    credential_store: Box<dyn CredentialStore>,
    cached_credentials: Mutex<Option<SpotifyCredentials>>,
    last_error: Mutex<Option<String>>,
}

impl SpotifyAuth {
    pub fn new() -> Self {
        Self::with_store(Box::new(KeyringCredentialStore))
    }

    fn with_store(credential_store: Box<dyn CredentialStore>) -> Self {
        Self {
            credential_store,
            cached_credentials: Mutex::new(None),
            last_error: Mutex::new(None),
        }
    }

    pub fn connection_status(&self) -> SpotifyConnectionStatus {
        match self.load_credentials() {
            Ok(Some(_)) => SpotifyConnectionStatus::connected(),
            Ok(None) => {
                if let Some(error) = self.take_last_error() {
                    SpotifyConnectionStatus::authentication_error(error)
                } else {
                    SpotifyConnectionStatus::not_connected()
                }
            }
            Err(error) => SpotifyConnectionStatus::authentication_error(error),
        }
    }

    pub fn connect(&self) -> Result<(), String> {
        let result = self.connect_inner();
        match &result {
            Ok(()) => self.set_last_error(None),
            Err(error) => self.set_last_error(Some(error.clone())),
        }
        result
    }

    pub fn disconnect(&self) -> Result<(), String> {
        self.credential_store.delete()?;
        self.set_cached_credentials(None)?;
        self.set_last_error(None);
        Ok(())
    }

    pub fn valid_access_token(&self) -> Result<String, String> {
        let mut credentials = self
            .load_credentials()?
            .ok_or_else(|| "Spotify is not connected.".to_string())?;
        let now = now_unix_seconds()?;

        if credentials.access_token_is_valid_at(now) {
            return Ok(credentials.access_token);
        }

        credentials = self.refresh_credentials(&credentials)?;
        let access_token = credentials.access_token.clone();
        self.save_credentials(credentials)?;
        Ok(access_token)
    }

    fn connect_inner(&self) -> Result<(), String> {
        let config = load_config()?;
        let pkce = PkcePair::generate();
        let state = random_urlsafe_string(32);
        let listener = bind_callback_listener()?;
        let authorization_url = authorization_url(&config.client_id, &pkce.challenge, &state)?;

        open::that(authorization_url.as_str()).map_err(|error| {
            format!("Could not open Spotify authorization in your browser: {error}")
        })?;

        let callback = wait_for_callback(listener, &state)?;
        let token_response =
            exchange_authorization_code(&config.client_id, &callback.code, &pkce.verifier)?;
        let credentials = token_response.into_credentials(None, None)?;
        self.save_credentials(credentials)
    }

    fn refresh_credentials(
        &self,
        existing_credentials: &SpotifyCredentials,
    ) -> Result<SpotifyCredentials, String> {
        let config = load_config()?;
        let token_response =
            refresh_access_token(&config.client_id, &existing_credentials.refresh_token)?;
        token_response.into_credentials(
            Some(existing_credentials.refresh_token.clone()),
            Some(existing_credentials.scope.clone()),
        )
    }

    fn load_credentials(&self) -> Result<Option<SpotifyCredentials>, String> {
        let cached = self
            .cached_credentials
            .lock()
            .map_err(|_| "Spotify authentication state could not be read.".to_string())?;

        if cached.is_some() {
            return Ok(cached.clone());
        }

        drop(cached);
        let stored = self.credential_store.load()?;
        if stored.is_some() {
            self.set_cached_credentials(stored.clone())?;
        }
        Ok(stored)
    }

    fn save_credentials(&self, credentials: SpotifyCredentials) -> Result<(), String> {
        self.credential_store.save(&credentials)?;
        self.set_cached_credentials(Some(credentials))
    }

    fn set_cached_credentials(
        &self,
        credentials: Option<SpotifyCredentials>,
    ) -> Result<(), String> {
        let mut cached = self
            .cached_credentials
            .lock()
            .map_err(|_| "Spotify authentication state could not be updated.".to_string())?;
        *cached = credentials;
        Ok(())
    }

    fn set_last_error(&self, error: Option<String>) {
        if let Ok(mut last_error) = self.last_error.lock() {
            *last_error = error;
        }
    }

    fn take_last_error(&self) -> Option<String> {
        self.last_error.lock().ok().and_then(|error| error.clone())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PkcePair {
    verifier: String,
    challenge: String,
}

impl PkcePair {
    fn generate() -> Self {
        let verifier = random_urlsafe_string(64);
        let challenge = pkce_challenge(&verifier);

        Self {
            verifier,
            challenge,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CallbackResult {
    code: String,
}

#[derive(Debug, Deserialize)]
struct TokenResponse {
    access_token: String,
    refresh_token: Option<String>,
    expires_in: u64,
    scope: Option<String>,
}

impl TokenResponse {
    fn into_credentials(
        self,
        fallback_refresh_token: Option<String>,
        fallback_scope: Option<String>,
    ) -> Result<SpotifyCredentials, String> {
        let refresh_token = self
            .refresh_token
            .or(fallback_refresh_token)
            .ok_or_else(|| "Spotify did not return a refresh credential.".to_string())?;
        let now = now_unix_seconds()?;

        Ok(SpotifyCredentials {
            access_token: self.access_token,
            refresh_token,
            expires_at_unix_seconds: now.saturating_add(self.expires_in),
            scope: self.scope.or(fallback_scope).unwrap_or_default(),
        })
    }
}

fn authorization_url(client_id: &str, code_challenge: &str, state: &str) -> Result<Url, String> {
    let mut url = Url::parse(AUTHORIZATION_URL)
        .map_err(|_| "Spotify authorization URL is invalid.".to_string())?;
    url.query_pairs_mut()
        .append_pair("client_id", client_id)
        .append_pair("response_type", "code")
        .append_pair("redirect_uri", REDIRECT_URI)
        .append_pair("code_challenge_method", "S256")
        .append_pair("code_challenge", code_challenge)
        .append_pair("state", state)
        .append_pair("scope", PHASE_FOUR_SCOPE);
    Ok(url)
}

fn bind_callback_listener() -> Result<TcpListener, String> {
    let listener = TcpListener::bind((CALLBACK_HOST, CALLBACK_PORT)).map_err(|error| {
        format!(
            "Could not start Spotify callback listener on {REDIRECT_URI}. Make sure port {CALLBACK_PORT} is available: {error}"
        )
    })?;
    listener
        .set_nonblocking(true)
        .map_err(|error| format!("Could not prepare Spotify callback listener: {error}"))?;
    Ok(listener)
}

fn wait_for_callback(
    listener: TcpListener,
    expected_state: &str,
) -> Result<CallbackResult, String> {
    let deadline = Instant::now() + OAUTH_TIMEOUT;

    loop {
        match listener.accept() {
            Ok((stream, _)) => return handle_callback_stream(stream, expected_state),
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                if Instant::now() >= deadline {
                    return Err(
                        "Spotify authorization timed out before the callback was received."
                            .to_string(),
                    );
                }
                thread::sleep(HTTP_POLL_INTERVAL);
            }
            Err(error) => return Err(format!("Spotify callback listener failed: {error}")),
        }
    }
}

fn handle_callback_stream(
    mut stream: TcpStream,
    expected_state: &str,
) -> Result<CallbackResult, String> {
    let mut buffer = [0_u8; 4096];
    let bytes_read = stream
        .read(&mut buffer)
        .map_err(|error| format!("Could not read Spotify callback: {error}"))?;
    let request = String::from_utf8_lossy(&buffer[..bytes_read]);
    let result = parse_callback_request(&request, expected_state);

    let response_body = if result.is_ok() {
        "Spotify is connected. You can return to Music Archive."
    } else {
        "Spotify could not be connected. You can return to Music Archive."
    };
    let response = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: text/plain; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        response_body.len(),
        response_body
    );
    let _ = stream.write_all(response.as_bytes());
    let _ = stream.flush();

    result
}

fn parse_callback_request(request: &str, expected_state: &str) -> Result<CallbackResult, String> {
    let request_line = request
        .lines()
        .next()
        .ok_or_else(|| "Spotify callback request was empty.".to_string())?;
    let mut parts = request_line.split_whitespace();
    let method = parts.next().unwrap_or_default();
    let target = parts.next().unwrap_or_default();

    if method != "GET" {
        return Err("Spotify callback used an unexpected HTTP method.".to_string());
    }

    let callback_url = Url::parse(&format!("http://{CALLBACK_HOST}{target}"))
        .map_err(|_| "Spotify callback URL could not be parsed.".to_string())?;

    if callback_url.path() != CALLBACK_PATH {
        return Err("Spotify callback used an unexpected path.".to_string());
    }

    let query: HashMap<_, _> = callback_url.query_pairs().into_owned().collect();
    let returned_state = query
        .get("state")
        .ok_or_else(|| "Spotify callback was missing state.".to_string())?;
    if returned_state != expected_state {
        return Err("Spotify callback state did not match.".to_string());
    }

    if let Some(error) = query.get("error") {
        if error == "access_denied" {
            return Err("Spotify authorization was denied.".to_string());
        }

        return Err("Spotify returned an authorization error.".to_string());
    }

    let code = query
        .get("code")
        .filter(|code| !code.is_empty())
        .cloned()
        .ok_or_else(|| "Spotify callback was missing an authorization code.".to_string())?;

    Ok(CallbackResult { code })
}

fn exchange_authorization_code(
    client_id: &str,
    code: &str,
    code_verifier: &str,
) -> Result<TokenResponse, String> {
    let params = [
        ("client_id", client_id),
        ("grant_type", "authorization_code"),
        ("code", code),
        ("redirect_uri", REDIRECT_URI),
        ("code_verifier", code_verifier),
    ];

    post_token_request(&params, "Spotify token exchange failed")
}

fn refresh_access_token(client_id: &str, refresh_token: &str) -> Result<TokenResponse, String> {
    let params = [
        ("client_id", client_id),
        ("grant_type", "refresh_token"),
        ("refresh_token", refresh_token),
    ];

    post_token_request(&params, "Spotify token refresh failed")
}

fn post_token_request(
    params: &[(&str, &str)],
    failure_message: &str,
) -> Result<TokenResponse, String> {
    let response = Client::new()
        .post(TOKEN_URL)
        .form(params)
        .send()
        .map_err(|error| format!("{failure_message}: network request failed: {error}"))?;

    if !response.status().is_success() {
        return Err(format!(
            "{failure_message}: Spotify returned HTTP {}.",
            response.status().as_u16()
        ));
    }

    response
        .json::<TokenResponse>()
        .map_err(|error| format!("{failure_message}: response could not be read: {error}"))
}

fn random_urlsafe_string(byte_count: usize) -> String {
    let mut bytes = vec![0_u8; byte_count];
    OsRng.fill_bytes(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}

fn pkce_challenge(verifier: &str) -> String {
    let digest = Sha256::digest(verifier.as_bytes());
    URL_SAFE_NO_PAD.encode(digest)
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::{
        authorization_url, parse_callback_request, pkce_challenge, random_urlsafe_string,
        TokenResponse,
    };
    use crate::spotify::config::{PHASE_FOUR_SCOPE, REDIRECT_URI};

    #[test]
    fn pkce_challenge_uses_s256_without_padding() {
        let challenge = pkce_challenge("dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk");

        assert_eq!(challenge, "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM");
        assert!(!challenge.contains('='));
    }

    #[test]
    fn generated_pkce_values_are_urlsafe() {
        let value = random_urlsafe_string(64);

        assert!(value.len() >= 43);
        assert!(value
            .chars()
            .all(|character| character.is_ascii_alphanumeric()
                || character == '-'
                || character == '_'));
    }

    #[test]
    fn authorization_url_requests_only_phase_four_scope_and_pkce_s256() {
        let url = authorization_url("client-id", "challenge", "state").expect("url builds");
        let query: HashMap<_, _> = url.query_pairs().into_owned().collect();

        assert_eq!(query.get("client_id"), Some(&"client-id".to_string()));
        assert_eq!(query.get("redirect_uri"), Some(&REDIRECT_URI.to_string()));
        assert_eq!(
            query.get("code_challenge_method"),
            Some(&"S256".to_string())
        );
        assert_eq!(query.get("code_challenge"), Some(&"challenge".to_string()));
        assert_eq!(query.get("state"), Some(&"state".to_string()));
        assert_eq!(query.get("scope"), Some(&PHASE_FOUR_SCOPE.to_string()));
    }

    #[test]
    fn callback_requires_matching_state() {
        let request =
            "GET /callback?code=abc&state=expected HTTP/1.1\r\nHost: 127.0.0.1:8888\r\n\r\n";
        let result = parse_callback_request(request, "expected").expect("callback parses");

        assert_eq!(result.code, "abc");
        assert!(parse_callback_request(request, "other").is_err());
    }

    #[test]
    fn callback_handles_denial_and_missing_code() {
        let denied = "GET /callback?error=access_denied&state=expected HTTP/1.1\r\n\r\n";
        let missing_code = "GET /callback?state=expected HTTP/1.1\r\n\r\n";

        assert!(parse_callback_request(denied, "expected")
            .expect_err("denial fails")
            .contains("denied"));
        assert!(parse_callback_request(missing_code, "expected")
            .expect_err("missing code fails")
            .contains("authorization code"));
    }

    #[test]
    fn refreshed_credentials_keep_existing_refresh_token_and_scope_when_omitted() {
        let response = TokenResponse {
            access_token: "new-access".to_string(),
            refresh_token: None,
            expires_in: 3600,
            scope: None,
        };

        let credentials = response
            .into_credentials(
                Some("existing-refresh".to_string()),
                Some(PHASE_FOUR_SCOPE.to_string()),
            )
            .expect("fallback credential values are accepted");

        assert_eq!(credentials.access_token, "new-access");
        assert_eq!(credentials.refresh_token, "existing-refresh");
        assert_eq!(credentials.scope, PHASE_FOUR_SCOPE);
    }
}
