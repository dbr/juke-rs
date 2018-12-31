use failure::Error;
use rspotify::spotify::oauth2::TokenInfo;
use serde_derive::{Deserialize, Serialize};

/// Shortcut for error return type
pub type ClientResult<T> = Result<T, Error>;

/// State of the Spotify client
#[derive(Debug, PartialEq, Clone, Copy, Serialize, Deserialize)]
pub enum PlaybackState {
    /// No client etc?
    Unknown,

    /// Requires an auth token
    NoAuth,

    /// Missing a device to control, set via `Client::set_active_device`
    NoDevice,

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
    /// Spotify ID
    pub spotify_uri: String,
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
            spotify_uri: ft.uri,
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
pub struct SearchResult {
    pub items: Vec<BasicSongInfo>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Device {
    name: String,
    id: String,
}

#[derive(Debug)]
pub struct DeviceListParams {
    pub tid: TaskID,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DeviceListResult {
    pub items: Vec<rspotify::spotify::model::device::Device>,
}

/// Things web-server can ask Spotify thread to do
#[derive(Debug)]
pub enum SpotifyCommand {
    Resume,
    Pause,
    Request(SongRequestInfo),
    Search(SearchParams),
    SetAuthToken(TokenInfo),
    ListDevices(DeviceListParams),
    SetActiveDevice(String),
}

/// Types of things a Spotify thread can respond to a command with
#[derive(Debug, Serialize)]
pub enum CommandResponseDataType {
    Search(SearchResult),
    DeviceList(DeviceListResult),
}

/// Spotify commands can respond with this type
#[derive(Debug)]
pub struct CommandResponse {
    pub tid: TaskID,
    pub value: CommandResponseDataType,
}

/// Identifier for a task, used to match up return values
#[derive(Debug, PartialEq, Copy, Clone)]
pub struct TaskID {
    pub id: u64,
}
