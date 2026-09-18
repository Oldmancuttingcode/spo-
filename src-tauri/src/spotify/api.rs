use std::{collections::HashMap, fmt};

use reqwest::{
    blocking::Client,
    header::{AUTHORIZATION, RETRY_AFTER},
    StatusCode, Url,
};
use serde::{Deserialize, Serialize};

const RECENTLY_PLAYED_URL: &str = "https://api.spotify.com/v1/me/player/recently-played";
const DEFAULT_RECENTLY_PLAYED_LIMIT: u32 = 50;
const MAX_RECENTLY_PLAYED_LIMIT: u32 = 50;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SpotifyApiError {
    InvalidRequest(String),
    Network(String),
    Unauthorized,
    Forbidden,
    RateLimited(Option<String>),
    SpotifyStatus(u16),
    InvalidResponse(String),
}

impl fmt::Display for SpotifyApiError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidRequest(message) => write!(formatter, "{message}"),
            Self::Network(message) => {
                write!(formatter, "Spotify network request failed: {message}")
            }
            Self::Unauthorized => write!(
                formatter,
                "Spotify rejected the access token. Try reconnecting Spotify."
            ),
            Self::Forbidden => write!(
                formatter,
                "Spotify did not allow this request. Check that the required scope is granted."
            ),
            Self::RateLimited(Some(retry_after)) => write!(
                formatter,
                "Spotify rate limited the request. Retry after {retry_after} seconds."
            ),
            Self::RateLimited(None) => {
                write!(
                    formatter,
                    "Spotify rate limited the request. Try again later."
                )
            }
            Self::SpotifyStatus(status) => {
                write!(formatter, "Spotify returned HTTP {status}.")
            }
            Self::InvalidResponse(message) => {
                write!(formatter, "Spotify response could not be read: {message}")
            }
        }
    }
}

pub fn recently_played(
    access_token: &str,
    limit: Option<u32>,
    after: Option<u64>,
    before: Option<u64>,
) -> Result<RecentlyPlayedResponse, SpotifyApiError> {
    let request = RecentlyPlayedRequest::new(limit, after, before)?;
    let url = request.url()?;
    let response = Client::new()
        .get(url)
        .header(AUTHORIZATION, format!("Bearer {access_token}"))
        .send()
        .map_err(|error| SpotifyApiError::Network(error.to_string()))?;

    let status = response.status();
    if status == StatusCode::UNAUTHORIZED {
        return Err(SpotifyApiError::Unauthorized);
    }
    if status == StatusCode::FORBIDDEN {
        return Err(SpotifyApiError::Forbidden);
    }
    if status == StatusCode::TOO_MANY_REQUESTS {
        let retry_after = response
            .headers()
            .get(RETRY_AFTER)
            .and_then(|value| value.to_str().ok())
            .map(ToString::to_string);
        return Err(SpotifyApiError::RateLimited(retry_after));
    }
    if !status.is_success() {
        return Err(SpotifyApiError::SpotifyStatus(status.as_u16()));
    }

    let spotify_response = response
        .json::<SpotifyRecentlyPlayedResponse>()
        .map_err(|error| SpotifyApiError::InvalidResponse(error.to_string()))?;

    spotify_response.try_into()
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RecentlyPlayedRequest {
    limit: u32,
    after: Option<u64>,
    before: Option<u64>,
}

impl RecentlyPlayedRequest {
    fn new(
        limit: Option<u32>,
        after: Option<u64>,
        before: Option<u64>,
    ) -> Result<Self, SpotifyApiError> {
        let limit = limit.unwrap_or(DEFAULT_RECENTLY_PLAYED_LIMIT);
        if limit == 0 || limit > MAX_RECENTLY_PLAYED_LIMIT {
            return Err(SpotifyApiError::InvalidRequest(format!(
                "Spotify Recently Played limit must be between 1 and {MAX_RECENTLY_PLAYED_LIMIT}."
            )));
        }

        if after.is_some() && before.is_some() {
            return Err(SpotifyApiError::InvalidRequest(
                "Spotify Recently Played accepts either after or before, not both.".to_string(),
            ));
        }

        Ok(Self {
            limit,
            after,
            before,
        })
    }

    fn url(&self) -> Result<Url, SpotifyApiError> {
        let mut url = Url::parse(RECENTLY_PLAYED_URL)
            .map_err(|_| SpotifyApiError::InvalidRequest("Spotify API URL is invalid.".into()))?;
        {
            let mut query = url.query_pairs_mut();
            query.append_pair("limit", &self.limit.to_string());
            if let Some(after) = self.after {
                query.append_pair("after", &after.to_string());
            }
            if let Some(before) = self.before {
                query.append_pair("before", &before.to_string());
            }
        }
        Ok(url)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecentlyPlayedResponse {
    pub items: Vec<RecentlyPlayedItem>,
    pub cursors: RecentlyPlayedCursors,
    pub next: Option<String>,
    pub limit: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecentlyPlayedItem {
    pub played_at: String,
    pub context: Option<PlayContext>,
    pub track: RecentlyPlayedTrack,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayContext {
    pub context_type: Option<String>,
    pub uri: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecentlyPlayedTrack {
    pub spotify_id: Option<String>,
    pub title: String,
    pub duration_ms: Option<u32>,
    pub spotify_url: Option<String>,
    pub album: Option<RecentlyPlayedAlbum>,
    pub artists: Vec<RecentlyPlayedArtist>,
    pub is_local: bool,
    pub unsupported_reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecentlyPlayedAlbum {
    pub spotify_id: Option<String>,
    pub name: String,
    pub artwork_url: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecentlyPlayedArtist {
    pub spotify_id: Option<String>,
    pub name: String,
    pub spotify_url: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RecentlyPlayedCursors {
    pub after: Option<String>,
    pub before: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SpotifyRecentlyPlayedResponse {
    items: Vec<SpotifyRecentlyPlayedItem>,
    cursors: Option<SpotifyCursors>,
    next: Option<String>,
    limit: Option<u32>,
}

impl TryFrom<SpotifyRecentlyPlayedResponse> for RecentlyPlayedResponse {
    type Error = SpotifyApiError;

    fn try_from(response: SpotifyRecentlyPlayedResponse) -> Result<Self, Self::Error> {
        let items = response
            .items
            .into_iter()
            .map(TryInto::try_into)
            .collect::<Result<Vec<_>, _>>()?;

        Ok(Self {
            items,
            cursors: response.cursors.unwrap_or_default().into(),
            next: response.next,
            limit: response.limit.unwrap_or(DEFAULT_RECENTLY_PLAYED_LIMIT),
        })
    }
}

#[derive(Debug, Deserialize)]
struct SpotifyRecentlyPlayedItem {
    track: SpotifyTrack,
    played_at: String,
    context: Option<SpotifyContext>,
}

impl TryFrom<SpotifyRecentlyPlayedItem> for RecentlyPlayedItem {
    type Error = SpotifyApiError;

    fn try_from(item: SpotifyRecentlyPlayedItem) -> Result<Self, Self::Error> {
        validate_spotify_timestamp(&item.played_at)?;

        Ok(Self {
            played_at: item.played_at,
            context: item.context.map(Into::into),
            track: item.track.into(),
        })
    }
}

#[derive(Debug, Deserialize)]
struct SpotifyContext {
    #[serde(rename = "type")]
    context_type: Option<String>,
    uri: Option<String>,
}

impl From<SpotifyContext> for PlayContext {
    fn from(context: SpotifyContext) -> Self {
        Self {
            context_type: context.context_type,
            uri: context.uri,
        }
    }
}

#[derive(Debug, Deserialize)]
pub(crate) struct SpotifyTrack {
    id: Option<String>,
    name: String,
    duration_ms: Option<u32>,
    external_urls: Option<HashMap<String, String>>,
    album: Option<SpotifyAlbum>,
    artists: Option<Vec<SpotifyArtist>>,
    is_local: Option<bool>,
}

impl From<SpotifyTrack> for RecentlyPlayedTrack {
    fn from(track: SpotifyTrack) -> Self {
        let is_local = track.is_local.unwrap_or(false);
        let spotify_id = non_empty(track.id);
        let unsupported_reason = if spotify_id.is_none() {
            Some(if is_local {
                "Local Spotify track without a Spotify track ID.".to_string()
            } else {
                "Spotify track is missing a Spotify track ID.".to_string()
            })
        } else {
            None
        };

        Self {
            spotify_id,
            title: track.name,
            duration_ms: track.duration_ms,
            spotify_url: external_spotify_url(track.external_urls),
            album: track.album.map(Into::into),
            artists: track
                .artists
                .unwrap_or_default()
                .into_iter()
                .map(Into::into)
                .collect(),
            is_local,
            unsupported_reason,
        }
    }
}

#[derive(Debug, Deserialize)]
struct SpotifyAlbum {
    id: Option<String>,
    name: String,
    images: Option<Vec<SpotifyImage>>,
}

impl From<SpotifyAlbum> for RecentlyPlayedAlbum {
    fn from(album: SpotifyAlbum) -> Self {
        Self {
            spotify_id: non_empty(album.id),
            name: album.name,
            artwork_url: album
                .images
                .unwrap_or_default()
                .into_iter()
                .next()
                .map(|image| image.url),
        }
    }
}

#[derive(Debug, Deserialize)]
struct SpotifyImage {
    url: String,
}

#[derive(Debug, Deserialize)]
struct SpotifyArtist {
    id: Option<String>,
    name: String,
    external_urls: Option<HashMap<String, String>>,
}

impl From<SpotifyArtist> for RecentlyPlayedArtist {
    fn from(artist: SpotifyArtist) -> Self {
        Self {
            spotify_id: non_empty(artist.id),
            name: artist.name,
            spotify_url: external_spotify_url(artist.external_urls),
        }
    }
}

#[derive(Debug, Default, Deserialize)]
struct SpotifyCursors {
    after: Option<String>,
    before: Option<String>,
}

impl From<SpotifyCursors> for RecentlyPlayedCursors {
    fn from(cursors: SpotifyCursors) -> Self {
        Self {
            after: cursors.after,
            before: cursors.before,
        }
    }
}

fn external_spotify_url(urls: Option<HashMap<String, String>>) -> Option<String> {
    urls.and_then(|urls| urls.get("spotify").cloned())
        .filter(|value| !value.trim().is_empty())
}

fn non_empty(value: Option<String>) -> Option<String> {
    value.filter(|value| !value.trim().is_empty())
}

fn validate_spotify_timestamp(timestamp: &str) -> Result<(), SpotifyApiError> {
    if timestamp.len() < 20 || !timestamp.ends_with('Z') || !timestamp.contains('T') {
        return Err(SpotifyApiError::InvalidResponse(
            "played_at was not an RFC3339 UTC timestamp.".to_string(),
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        RecentlyPlayedRequest, SpotifyApiError, SpotifyRecentlyPlayedResponse,
        DEFAULT_RECENTLY_PLAYED_LIMIT, MAX_RECENTLY_PLAYED_LIMIT,
    };

    const RECENTLY_PLAYED_FIXTURE: &str = r#"
    {
      "items": [
        {
          "track": {
            "id": "track-1",
            "name": "Jezebel",
            "duration_ms": 210000,
            "external_urls": { "spotify": "https://open.spotify.com/track/track-1" },
            "album": {
              "id": "album-1",
              "name": "Album One",
              "images": [
                { "url": "https://image.example/large.jpg", "height": 640, "width": 640 }
              ]
            },
            "artists": [
              {
                "id": "artist-1",
                "name": "Giveon",
                "external_urls": { "spotify": "https://open.spotify.com/artist/artist-1" }
              }
            ],
            "is_local": false
          },
          "played_at": "2026-09-09T10:00:00Z",
          "context": { "type": "playlist", "uri": "spotify:playlist:playlist-1" }
        }
      ],
      "next": "https://api.spotify.com/v1/me/player/recently-played?before=1",
      "cursors": { "after": "1799479200000", "before": "1799478000000" },
      "limit": 50
    }
    "#;

    #[test]
    fn request_defaults_to_phase_five_development_limit() {
        let request = RecentlyPlayedRequest::new(None, None, None).expect("request is valid");

        assert_eq!(request.limit, DEFAULT_RECENTLY_PLAYED_LIMIT);
    }

    #[test]
    fn request_rejects_invalid_limit() {
        assert!(matches!(
            RecentlyPlayedRequest::new(Some(0), None, None),
            Err(SpotifyApiError::InvalidRequest(_))
        ));
        assert!(matches!(
            RecentlyPlayedRequest::new(Some(MAX_RECENTLY_PLAYED_LIMIT + 1), None, None),
            Err(SpotifyApiError::InvalidRequest(_))
        ));
    }

    #[test]
    fn request_rejects_after_and_before_together() {
        assert!(matches!(
            RecentlyPlayedRequest::new(Some(50), Some(1), Some(2)),
            Err(SpotifyApiError::InvalidRequest(_))
        ));
    }

    #[test]
    fn request_url_uses_spotify_cursor_milliseconds() {
        let url = RecentlyPlayedRequest::new(Some(25), Some(1799479200000), None)
            .expect("request is valid")
            .url()
            .expect("url is valid");

        assert_eq!(url.query(), Some("limit=25&after=1799479200000"));
    }

    #[test]
    fn deserializes_recently_played_subset() {
        let spotify: SpotifyRecentlyPlayedResponse =
            serde_json::from_str(RECENTLY_PLAYED_FIXTURE).expect("fixture parses");
        let response = super::RecentlyPlayedResponse::try_from(spotify).expect("model maps");

        assert_eq!(response.items.len(), 1);
        assert_eq!(response.items[0].played_at, "2026-09-09T10:00:00Z");
        assert_eq!(
            response.items[0]
                .context
                .as_ref()
                .and_then(|context| context.context_type.clone()),
            Some("playlist".to_string())
        );
        assert_eq!(
            response.items[0].track.spotify_id,
            Some("track-1".to_string())
        );
        assert_eq!(response.items[0].track.artists[0].name, "Giveon");
        assert_eq!(
            response.items[0]
                .track
                .album
                .as_ref()
                .and_then(|album| album.artwork_url.clone()),
            Some("https://image.example/large.jpg".to_string())
        );
        assert_eq!(response.cursors.after, Some("1799479200000".to_string()));
        assert_eq!(response.cursors.before, Some("1799478000000".to_string()));
    }

    #[test]
    fn missing_optional_context_and_artwork_are_safe() {
        let spotify: SpotifyRecentlyPlayedResponse = serde_json::from_str(
            r#"
            {
              "items": [
                {
                  "track": {
                    "id": "track-2",
                    "name": "Catching Bodies",
                    "duration_ms": 190000,
                    "album": { "id": "album-2", "name": "Album Two", "images": [] },
                    "artists": [],
                    "is_local": false
                  },
                  "played_at": "2026-09-09T11:00:00Z",
                  "context": null
                }
              ],
              "cursors": {}
            }
            "#,
        )
        .expect("fixture parses");

        let response = super::RecentlyPlayedResponse::try_from(spotify).expect("model maps");

        assert!(response.items[0].context.is_none());
        assert!(response.items[0]
            .track
            .album
            .as_ref()
            .expect("album exists")
            .artwork_url
            .is_none());
        assert!(response.cursors.after.is_none());
        assert!(response.cursors.before.is_none());
    }

    #[test]
    fn local_track_without_spotify_id_is_marked_unsupported() {
        let spotify: SpotifyRecentlyPlayedResponse = serde_json::from_str(
            r#"
            {
              "items": [
                {
                  "track": {
                    "id": null,
                    "name": "Local Demo",
                    "is_local": true
                  },
                  "played_at": "2026-09-09T12:00:00Z"
                }
              ],
              "cursors": {}
            }
            "#,
        )
        .expect("fixture parses");

        let response = super::RecentlyPlayedResponse::try_from(spotify).expect("model maps");

        assert!(response.items[0].track.spotify_id.is_none());
        assert!(response.items[0].track.is_local);
        assert!(response.items[0].track.unsupported_reason.is_some());
    }

    #[test]
    fn invalid_played_at_timestamp_is_rejected() {
        let spotify: SpotifyRecentlyPlayedResponse = serde_json::from_str(
            r#"
            {
              "items": [
                {
                  "track": { "id": "track-1", "name": "Track" },
                  "played_at": "2026-09-09 12:00:00"
                }
              ],
              "cursors": {}
            }
            "#,
        )
        .expect("fixture parses");

        assert!(matches!(
            super::RecentlyPlayedResponse::try_from(spotify),
            Err(SpotifyApiError::InvalidResponse(_))
        ));
    }

    #[test]
    fn safe_error_messages_do_not_include_tokens() {
        assert_eq!(
            SpotifyApiError::Unauthorized.to_string(),
            "Spotify rejected the access token. Try reconnecting Spotify."
        );
        assert_eq!(
            SpotifyApiError::RateLimited(Some("5".to_string())).to_string(),
            "Spotify rate limited the request. Retry after 5 seconds."
        );
    }
}
