use failure::Error;
use serde_derive::{Deserialize, Serialize};

/// Shortcut for error return type
pub type ClientResult<T> = Result<T, Error>;

/// State of the Spotify client
#[derive(Debug, PartialEq, Clone, Copy, Serialize, Deserialize)]
pub enum PlaybackState {
    /// Awaiting a song to play
    NeedsSong,

    /// Currently making noise
    Playing,

    /// Client has been paused mid-song
    Paused,

    /// Was in `NeedsSong` but we have put a song in the queue
    EnqueuedAndWaiting,
}

/// What Spotify is currently playing
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlaybackStatus {
    /// If song is playing etc
    pub state: PlaybackState,
    pub song_title: Option<String>,
    // TODO: Current song/volume etc
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

#[derive(Debug)]
pub struct SearchResultSong {
    pub name: String,
    pub spotify_uri: String,
}

#[derive(Debug)]
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

#[derive(Debug)]
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
