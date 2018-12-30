use failure::Error;
use serde_derive::{Deserialize, Serialize};

/// Shortcut for error return type
pub type ClientResult<T> = Result<T, Error>;

/// State of the Spotify client
#[derive(Debug, PartialEq, Clone, Copy, Serialize, Deserialize)]
pub enum PlaybackState {
    /// No client etc?
    Unknown,

    /// Awaiting a song to play
    NeedsSong,

    /// Currently making noise
    Playing,

    /// Client has been paused mid-song
    Paused,

    /// Was in `NeedsSong` but we have put a song in the queue
    EnqueuedAndWaiting,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BasicSongInfo {
    /// Song title
    pub title: String,
    /// Artist name
    pub artist: String, // FIXME: Keep as list
    /// Song duration in milliseconds
    pub duration_ms: u32,
    /// Album artwork
    pub album_image_url: Option<String>,
}

impl From<rspotify::spotify::model::track::FullTrack> for BasicSongInfo {
    fn from(ft: rspotify::spotify::model::track::FullTrack) -> BasicSongInfo {
        BasicSongInfo {
            title: ft.name,
            artist: ft
                .artists
                .iter()
                .map(|t| t.name.clone())
                .collect::<Vec<String>>()
                .join(", "),
            duration_ms: ft.duration_ms,
            album_image_url: ft.album.images.first().and_then(|i| Some(i.url.clone())),
        }
    }
}

/// What Spotify is currently playing
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlaybackStatus {
    /// If song is playing etc
    pub state: PlaybackState,
    pub song: Option<BasicSongInfo>,
    pub progress_ms: Option<u32>,
}

impl Default for PlaybackStatus {
    fn default() -> Self {
        PlaybackStatus {
            state: PlaybackState::Unknown,
            song: None,
            progress_ms: None,
        }
    }
}
/// Song ID to send over command-queue
#[derive(Debug, Serialize, Deserialize)]
pub struct SongRequestInfo {
    pub track_id: String,
}

#[derive(Debug)]
pub struct SearchParams {
    pub title: String,
    pub tid: TaskID,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SearchResultSong {
    pub name: String,
    pub artists: Vec<String>,
    pub spotify_uri: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SearchResult {
    pub items: Vec<SearchResultSong>,
}

/// Things web-server can ask Spotify thread to do
#[derive(Debug)]
pub enum SpotifyCommand {
    Resume,
    Pause,
    Request(SongRequestInfo),
    Search(SearchParams),
}

#[derive(Debug, Serialize)]
pub enum CommandResponseDataType {
    Search(SearchResult),
}

#[derive(Debug)]
pub struct CommandResponse {
    pub tid: TaskID,
    pub value: CommandResponseDataType,
}

#[derive(Debug, PartialEq, Copy, Clone)]
pub struct TaskID {
    pub id: u64,
}
