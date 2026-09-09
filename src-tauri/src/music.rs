#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Artist {
    pub id: i64,
    pub spotify_id: String,
    pub name: String,
    pub spotify_url: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Track {
    pub id: i64,
    pub spotify_id: String,
    pub title: String,
    pub album_name: Option<String>,
    pub album_spotify_id: Option<String>,
    pub album_art_url: Option<String>,
    pub duration_ms: Option<i64>,
    pub spotify_url: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayHistory {
    pub id: i64,
    pub track_id: i64,
    pub played_at: String,
    pub context_type: Option<String>,
    pub context_uri: Option<String>,
    pub synced_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TagCategory {
    Genre,
    Mood,
    Sound,
    Vocal,
    Free,
}

impl TagCategory {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Genre => "genre",
            Self::Mood => "mood",
            Self::Sound => "sound",
            Self::Vocal => "vocal",
            Self::Free => "free",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tag {
    pub id: i64,
    pub name: String,
    pub category: TagCategory,
    pub created_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Playlist {
    pub id: i64,
    pub spotify_id: String,
    pub name: String,
    pub spotify_description: Option<String>,
    pub image_url: Option<String>,
    pub owner_spotify_id: Option<String>,
    pub spotify_url: Option<String>,
    pub snapshot_id: Option<String>,
    pub is_owned: bool,
    pub is_collaborative: bool,
    pub track_count: i64,
    pub last_synced_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiggingSession {
    pub id: i64,
    pub name: String,
    pub start_date: String,
    pub end_date: Option<String>,
    pub description: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}
