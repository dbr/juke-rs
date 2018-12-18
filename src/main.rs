/*
Dodos spotify:track:0b05H1iP6hdx8ue7XQlC5J
Thao  spotify:track:67k9jnPe4dSqvAfrM902Z0
*/

use rand::seq::SliceRandom;
use std::collections::hash_map::Entry;
use std::sync::{Arc, Mutex};
use std::thread;
use std::thread::sleep;
use std::time::{Duration, SystemTime};

use failure::{format_err, Error};

use rouille::{router, Request, Response};

use rspotify::spotify::client::Spotify;
use rspotify::spotify::model::device::Device;
use rspotify::spotify::oauth2::{SpotifyClientCredentials, SpotifyOAuth};
use rspotify::spotify::util::get_token;

/// Shortcut for error return type
type ClientResult<T> = Result<T, Error>;

/// Commands being sent from web thread to Spotify controller thread
type LockedTaskQueue = Arc<Mutex<TaskQueue>>;

/// Handles the requested song queue, with weighting etc
struct TheList {
    songs: std::collections::HashMap<String, i64>,
}

impl TheList {
    fn new() -> TheList {
        TheList {
            songs: std::collections::HashMap::new(),
        }
    }

    fn add(&mut self, track_id: String) {
        self.songs.insert(track_id, 1);
    }

    fn nextup(&mut self) -> Option<String> {
        let hm = {
            let mut rng = rand::thread_rng();
            let keys: Vec<&String> = self.songs.keys().collect();
            let hm = keys.choose(&mut rng)?;
            hm.to_string().clone()
        };

        if let Entry::Occupied(o) = self.songs.entry(hm.to_string()) {
            o.remove_entry();
        };

        Some(hm.to_string())
    }
}

/// State of the Spotify client
#[derive(Debug, PartialEq)]
enum PlaybackState {
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
#[derive(Debug)]
pub struct PlaybackStatus {
    /// If song is playing etc
    state: PlaybackState,
    // TODO: Current song/volume etc
}

/// Handles playback/queue logic and commands Spotify
pub struct Client<'a> {
    spotify: &'a Spotify,
    device: Option<Device>,
    the_list: TheList,
    last_status_check: Option<SystemTime>,
    status: Option<PlaybackStatus>,
}

/// Turn Spotify API structure into internal `PlaybackStatus`
fn parse_playing_context(
    ctx: Option<rspotify::spotify::model::context::SimplifiedPlayingContext>,
) -> Option<PlaybackStatus> {
    /*
    Currently playing Some(SimplifiedPlayingContext { context: None, timestamp: 1544709885233, progress_ms: Some(4048), is_playing: true, item: Some(FullTrack { album: SimplifiedAlbum { artists: [SimplifiedArtist { external_urls: {"spotify": "https://open.spotify.com/artist/10tysauSA5JATqniBDu2Ed"}, href: "https://api.spotify.com/v1/artists/10tysauSA5JATqniBDu2Ed", id: "10tysauSA5JATqniBDu2Ed", name: "The Dodos", _type: artist, uri: "spotify:artist:10tysauSA5JATqniBDu2Ed" }], album_type: "album", available_markets: ["AD", "AE", "AR", "AT", "AU", "BE", "BG", "BH", "BO", "BR", "CA", "CH", "CL", "CO", "CR", "CY", "CZ", "DE", "DK", "DO", "DZ", "EC", "EE", "EG", "ES", "FI", "FR", "GB", "GR", "GT", "HK", "HN", "HU", "ID", "IE", "IL", "IS", "IT", "JO", "JP", "KW", "LB", "LI", "LT", "LU", "LV", "MA", "MC", "MT", "MX", "MY", "NI", "NL", "NO", "NZ", "OM", "PA", "PE", "PH", "PL", "PS", "PT", "PY", "QA", "RO", "SA", "SE", "SG", "SK", "SV", "TH", "TN", "TR", "TW", "US", "UY", "VN", "ZA"], external_urls: {"spotify": "https://open.spotify.com/album/0PCBeIzHUlDM2cxq3Eu8tC"}, href: "https://api.spotify.com/v1/albums/0PCBeIzHUlDM2cxq3Eu8tC", id: "0PCBeIzHUlDM2cxq3Eu8tC", images: [Image { height: Some(639), url: "https://i.scdn.co/image/8b578f53808b53d4364fc00d967b299537439a49", width: Some(640) }, Image { height: Some(300), url: "https://i.scdn.co/image/e5bad1bbf8cb032511bad0fa6737465bcecf3fc0", width: Some(300) }, Image { height: Some(64), url: "https://i.scdn.co/image/665b9e082a4d3eb71ce723bc391343f16756a2da", width: Some(64) }], name: "Visiter", _type: album, uri: "spotify:album:0PCBeIzHUlDM2cxq3Eu8tC" }, artists: [SimplifiedArtist { external_urls: {"spotify": "https://open.spotify.com/artist/10tysauSA5JATqniBDu2Ed"}, href: "https://api.spotify.com/v1/artists/10tysauSA5JATqniBDu2Ed", id: "10tysauSA5JATqniBDu2Ed", name: "The Dodos", _type: artist, uri: "spotify:artist:10tysauSA5JATqniBDu2Ed" }], available_markets: ["AD", "AE", "AR", "AT", "AU", "BE", "BG", "BH", "BO", "BR", "CA", "CH", "CL", "CO", "CR", "CY", "CZ", "DE", "DK", "DO", "DZ", "EC", "EE", "EG", "ES", "FI", "FR", "GB", "GR", "GT", "HK", "HN", "HU", "ID", "IE", "IL", "IS", "IT", "JO", "JP", "KW", "LB", "LI", "LT", "LU", "LV", "MA", "MC", "MT", "MX", "MY", "NI", "NL", "NO", "NZ", "OM", "PA", "PE", "PH", "PL", "PS", "PT", "PY", "QA", "RO", "SA", "SE", "SG", "SK", "SV", "TH", "TN", "TR", "TW", "US", "UY", "VN", "ZA"], disc_number: 1, duration_ms: 129213, external_ids: {"isrc": "USJMZ0800024"}, external_urls: {"spotify": "https://open.spotify.com/track/0b05H1iP6hdx8ue7XQlC5J"}, href: "https://api.spotify.com/v1/tracks/0b05H1iP6hdx8ue7XQlC5J", id: "0b05H1iP6hdx8ue7XQlC5J", name: "Walking", popularity: 26, preview_url: Some("https://p.scdn.co/mp3-preview/71d7038ab385245e4ac51d5eedb34d7606eb4d1e?cid=3c06f6e33b9444779340b90bd8638d2f"), track_number: 1, _type: track, uri: "spotify:track:0b05H1iP6hdx8ue7XQlC5J" }) })
    */
    if let Some(c) = ctx {
        let current_state = if c.is_playing {
            // Obvious case
            PlaybackState::Playing
        } else {
            // Not playing..
            if c.progress_ms.unwrap_or(0) > 0 {
                // Playing but midway through track means was paused
                PlaybackState::Paused
            } else {
                // Paused at 0ms means the current song stopped
                PlaybackState::NeedsSong
            }
        };

        Some(PlaybackStatus {
            state: current_state,
        })
    } else {
        None
    }
}

impl<'a> Client<'a> {
    pub fn new(client: &'a Spotify) -> Client<'a> {
        Client {
            spotify: &client,
            device: None,
            the_list: TheList::new(),
            last_status_check: None,
            status: None,
        }
    }

    /// List available devices
    pub fn list_devices(&self) -> ClientResult<Vec<Device>> {
        let devices = self.spotify.device()?;
        Ok(devices.devices)
    }

    /// Sets one of the devices from `list_devices` as the active one
    pub fn set_active_device(&mut self, device: Device) -> ClientResult<()> {
        self.device = Some(device);
        Ok(())
    }

    /// Pause playback
    pub fn pause(&self) -> ClientResult<()> {
        self.spotify.pause_playback(None)?;
        Ok(())
    }

    /// Clicks the play button
    pub fn resume(&self) -> ClientResult<()> {
        let id = self.device.clone().and_then(|x| Some(x.id));
        self.spotify.start_playback(id, None, None, None)?;

        Ok(())
    }

    pub fn search(&self, params: &SearchParams, queue: &mut TaskQueue) -> ClientResult<()> {
        let search = self.spotify.search_track(&params.title, 10, 0, None)?;
        let mut sr = vec![];
        for s in search.tracks.items {
            sr.push(SearchResultSong {
                name: s.name,
                spotify_uri: s.uri,
            });
        }
        queue.respond(CommandResponse {
            tid: params.tid,
            value: CommandResponseDataType::Search(SearchResult { items: sr }),
        });
        Ok(())
    }

    /// Update `status` field
    pub fn update_player_status(&mut self) -> ClientResult<()> {
        let x = self.spotify.current_playing(None)?;
        self.status = parse_playing_context(x);
        Ok(())
    }

    /// Adds specified track to "the list for consideration"
    pub fn request(&mut self, track_id: String) -> ClientResult<()> {
        self.the_list.add(track_id);
        Ok(())
    }

    /// Make a song start playing, replacing anything currently playing
    pub fn load_song(&mut self, track_id: String) -> ClientResult<()> {
        let id = self.device.clone().and_then(|x| Some(x.id));
        self.spotify
            .start_playback(id, None, Some(vec![track_id]), None)?;
        Ok(())
    }

    /// Take a song from the list and make it go
    pub fn enqueue(&mut self) -> ClientResult<()> {
        if let Some(t) = self.the_list.nextup() {
            self.load_song(t)?;
            if let Some(ref mut x) = self.status {
                // TODO: Is this state necessary?
                x.state = PlaybackState::EnqueuedAndWaiting;
            }
        }
        Ok(())
    }

    /// Called very often, performs regular activities like checking if Spotify is ready to play next song
    pub fn routine(&mut self) -> ClientResult<()> {
        // Wait a reasonable amount of time before pinging Spotify API for playback status
        let time_for_thing = if let Some(lc) = self.last_status_check {
            let x = lc.elapsed()?;
            x.as_secs() > 0 || x.subsec_millis() > 1000
        } else {
            true
        };

        if time_for_thing {
            // Sufficent time has elapsed
            self.last_status_check = Some(SystemTime::now());

            // Update status
            self.update_player_status()?;

            // FIXME: Quiet
            println!("Currently playing {:?}", self.status);
            println!("Songs in queue {:?}", self.the_list.songs);

            // Check if client needs
            let mut need_song = false;
            if let Some(ref s) = self.status {
                if s.state == PlaybackState::NeedsSong {
                    need_song = true;
                }
            }
            if need_song {
                self.enqueue()?;
            }
        }
        Ok(())
    }
}

fn handle_response(request: &Request, queue: &LockedTaskQueue) -> Response {
    router!(request,
        (GET) (/) => {
            // Index
            // FIXME: Serve status stuff
            Response::html("Hi")
        },
        (GET) (/ctrl/resume) => {
            // Play
            queue.lock().unwrap().queue(SpotifyCommand::Resume);
            Response::text("ok")
        },
        (GET) (/ctrl/pause) => {
            // Pause
            queue.lock().unwrap().queue(SpotifyCommand::Pause);
            Response::text("ok")
        },
        (GET) (/ctrl/request) => {
            // Add song to the list
            let id = request.get_param("track_id");
            if let Some(x) = id {
                queue.lock().unwrap().queue(SpotifyCommand::Request(SongRequestInfo{track_id: x}));
                Response::text("ok")
            } else {
                Response::text("missing track_id").with_status_code(500)
            }
        },
        (GET) (/search/track/{term:String}) => {
            // Queue search task and drop lock
            let tid: TaskID = {
                let mut q = queue.lock().unwrap();
                let tid = q.get_task_id();
                q.queue(SpotifyCommand::Search(SearchParams{tid: tid, title: term}));
                tid
            };

            loop {
                // TODO: Timeout?
                {
                    let response = queue.lock().unwrap()
                        .wait(tid);
                    if let Some(r) = response {
                        return Response::text(format!("kk: {:?}", r));
                    }
                    // Drop lock
                }

                sleep(Duration::from_millis(100));
            };
            unreachable!();
        },

        // default route
        _ => Response::text("404 Not found").with_status_code(404)
    )
}

/// Start web-server
fn web(queue: LockedTaskQueue) {
    rouille::start_server("0.0.0.0:8081", move |request| {
        handle_response(request, &queue.clone())
    });
}

/// Spotify commander thread
fn spotify_ctrl(queue: &LockedTaskQueue) -> Result<(), Error> {
    println!("Starting auth");

    // Perform auth
    let mut oauth = SpotifyOAuth::default()
        .scope("user-read-playback-state user-modify-playback-state")
        .build();

    let token_info = get_token(&mut oauth).ok_or_else(|| format_err!("Failed to get token"))?;

    println!("Got token");

    // Create client
    let client_credential = SpotifyClientCredentials::default()
        .token_info(token_info)
        .build();
    let spotify = Spotify::default()
        .client_credentials_manager(client_credential)
        .build();

    // Create client wrapper
    let mut client = Client::new(&spotify);
    let devices = client.list_devices()?;
    // FIXME: Handle no devices!
    client.set_active_device(devices[0].clone())?;

    // Wait for commands from the web-thread
    loop {
        let mut q = queue.lock().unwrap();
        let queue_content = q.pop();
        if let Some(c) = queue_content {
            println!("Got command: {:?}", c);
            match c {
                SpotifyCommand::Pause => client.pause()?,
                SpotifyCommand::Resume => client.resume()?,
                SpotifyCommand::Request(ri) => client.request(ri.track_id)?,
                SpotifyCommand::Search(sp) => client.search(&sp, &mut q)?,
            };
        } else {
            // Wait for new commands
            sleep(Duration::from_millis(50));
            client.routine()?;
        }
    }
}

/// Song ID to send over command-queue
#[derive(Debug)]
pub struct SongRequestInfo {
    track_id: String,
}

#[derive(Debug)]
pub struct SearchParams {
    title: String,
    tid: TaskID,
}

#[derive(Debug)]
pub struct SearchResultSong {
    name: String,
    spotify_uri: String,
}

#[derive(Debug)]
pub struct SearchResult {
    items: Vec<SearchResultSong>,
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
enum CommandResponseDataType {
    Search(SearchResult),
}

#[derive(Debug)]
pub struct CommandResponse {
    tid: TaskID,
    value: CommandResponseDataType,
}

#[derive(Default, Debug)]
pub struct TaskQueue {
    queue: std::collections::VecDeque<SpotifyCommand>,
    responses: std::collections::VecDeque<CommandResponse>,
    last_task_id: u64,
}
#[derive(Debug, PartialEq, Copy, Clone)]
pub struct TaskID {
    id: u64,
}

impl TaskQueue {
    pub fn new() -> TaskQueue {
        TaskQueue::default()
    }
    pub fn get_task_id(&mut self) -> TaskID {
        self.last_task_id += 1;
        TaskID {
            id: self.last_task_id,
        }
    }
    pub fn queue(&mut self, c: SpotifyCommand) {
        self.queue.push_back(c);
    }
    pub fn wait(&mut self, task_id: TaskID) -> Option<CommandResponse> {
        let mut idx: Option<usize> = None;
        for (i, c) in self.responses.iter().enumerate() {
            if c.tid == task_id {
                idx = Some(i);
                break;
            }
        }
        if let Some(i) = idx {
            let thing = self.responses.remove(i).unwrap();
            return Some(thing);
        }
        None
    }
    pub fn respond(&mut self, value: CommandResponse) {
        self.responses.push_back(value)
    }
    pub fn pop(&mut self) -> Option<SpotifyCommand> {
        self.queue.pop_back()
    }
}

/// Start all threads
fn main() {
    let queue = Arc::new(Mutex::new(TaskQueue::new()));

    let q1 = queue.clone();
    let w = thread::spawn(move || web(q1));

    let q2 = queue.clone();
    let s = thread::spawn(move || spotify_ctrl(&q2));
    s.join().unwrap().unwrap();
    w.join().unwrap();
}

#[cfg(test)]
mod tests {
    use super::main;
    use std::io::Read;
    use std::thread;

    #[test]
    fn test_basic() {
        let _t = thread::spawn(main);
        let mut resp =
            reqwest::get("http://localhost:8081/search/track/The Dodos Walking").unwrap();
        assert!(resp.status().is_success());
        let mut content = String::new();
        resp.read_to_string(&mut content).unwrap();
        assert_eq!(content, "kk");
        assert!(false);
    }
}
