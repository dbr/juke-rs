use failure::format_err;

use log::{debug, info, trace};
use rand::seq::SliceRandom;
use std::collections::hash_map::Entry;
use std::time::{Instant, SystemTime};

use serde_derive::{Deserialize, Serialize};

use rspotify::spotify::client::Spotify;
use rspotify::spotify::model::device::Device;
use rspotify::spotify::oauth2::{SpotifyClientCredentials, TokenInfo};

use crate::commands::TaskQueue;
use crate::common::*;

/// Handles the requested song queue, with weighting etc
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TheList {
    pub votes: std::collections::HashMap<String, i64>,
    pub songs: std::collections::HashMap<String, BasicSongInfo>,
}

impl TheList {
    pub fn new() -> TheList {
        TheList {
            votes: std::collections::HashMap::new(),
            songs: std::collections::HashMap::new(),
        }
    }

    fn add(&mut self, track_id: BasicSongInfo) {
        debug!("Added song {:?}", track_id);
        let key = &track_id.spotify_uri;
        self.votes
            .entry(key.clone())
            .and_modify(|x| *x += 1)
            .or_insert(1);
        self.songs.entry(key.clone()).or_insert(track_id);
        trace!("The list after: {:?}", self);
    }

    fn downvote(&mut self, track_id: String) {
        debug!("Downvoting song with ID {}", &track_id);
        self.votes.entry(track_id.clone()).and_modify(|e| *e -= 1);
        if let Some(c) = self.votes.get(&track_id) {
            if *c < -1 {
                debug!(
                    "Going to remove song from list because it now has {} votes",
                    c
                );
                self.votes.remove(&track_id);
                self.songs.remove(&track_id);
            }
        }
        if let Entry::Occupied(o) = self.votes.entry(track_id.clone()) {
            if *o.get() < -1 {
                o.remove();
            }
        }
        trace!("The list after: {:?}", self);
    }

    fn nextup(&mut self) -> Option<BasicSongInfo> {
        // TODO: This method can probably be simplified
        let hm = {
            let mut rng = rand::thread_rng();
            let keys: Vec<&String> = self.votes.keys().collect();
            let hm = keys.choose(&mut rng)?;
            hm.clone()
        };

        if let Entry::Occupied(o) = self.votes.entry(hm.to_string()) {
            let (key, _votes) = o.remove_entry();
            if let Entry::Occupied(o) = self.songs.entry(key) {
                let (_, value) = o.remove_entry();
                return Some(value);
            } else {
                return None;
            }
        } else {
            return None;
        };
    }
}

/// Handles playback/queue logic and commands Spotify
pub struct Client {
    spotify: Option<Spotify>,
    device: Option<Device>,
    pub the_list: TheList,
    last_status_check: Option<SystemTime>,
    last_token_refresh: Option<SystemTime>,
    pub status: PlaybackStatus,
    status_check_interval_ms: u32,
    token_refresh_interval_ms: u32,
}

/// Convert `Duration` into milliseconds (as u64), to be used until
/// the `as_millis` method is stable (returns u128). Max `u64` milliseconds
/// is only 49 days whereas `u128` is only 10^28 years..
/// Enough for our purposes
fn duration_as_millis(d: std::time::Duration) -> u64 {
    // TOOD: Replace when Duration::as_millis becomes stable
    (d.as_secs() * 1000) + u64::from(d.subsec_millis())
}

/// Turn Spotify API structure into internal `PlaybackStatus`
fn parse_playing_context(
    ctx: Option<rspotify::spotify::model::context::SimplifiedPlayingContext>,
) -> PlaybackStatus {
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

        let song = c.item.and_then(|t| Some(BasicSongInfo::from(t)));
        PlaybackStatus {
            state: current_state,
            song: song,
            progress_ms: c.progress_ms,
        }
    } else {
        PlaybackStatus::default()
    }
}

impl Client {
    pub fn new() -> Client {
        Client {
            spotify: None,
            device: None,
            the_list: TheList::new(),
            last_status_check: None,
            last_token_refresh: None,
            status: PlaybackStatus::default(),
            status_check_interval_ms: 1000,
            token_refresh_interval_ms: 1000 * 60 * 5,
        }
    }

    /// End session with Spotify
    pub fn clear_auth(&mut self) {
        self.spotify = None;
        self.device = None;
        self.status = PlaybackStatus::default();
    }

    pub fn set_auth_token(&mut self, token: &TokenInfo) {
        trace!("Setting auth token");
        let client_credential = SpotifyClientCredentials::default()
            .token_info(token.clone())
            .build();
        self.spotify = Some(
            Spotify::default()
                .client_credentials_manager(client_credential)
                .build(),
        );
    }

    /// Refresh auth token which expires every hour or so
    fn refresh_auth_token(&mut self) -> ClientResult<()> {
        let c = self.get_spotify()?;
        let oauth = rspotify::spotify::oauth2::SpotifyOAuth::default()
            .scope("user-read-playback-state user-modify-playback-state")
            .build();

        if let Some(ref ccm) = c.client_credentials_manager {
            let t = ccm.token_info.clone().unwrap();
            let rt = t.refresh_token.clone().unwrap();
            let newtoken = oauth.refresh_access_token(&rt).unwrap();
            self.set_auth_token(&newtoken);
        }
        Ok(())
    }

    fn get_spotify(&self) -> ClientResult<&Spotify> {
        match &self.spotify {
            None => Err(format_err!("Client not authenticated")),
            Some(c) => Ok(&c),
        }
    }

    /// List available devices
    pub fn list_devices(
        &self,
        params: &DeviceListParams,
        queue: &mut TaskQueue,
    ) -> ClientResult<()> {
        trace!("Listing devices");
        let devices = self.get_spotify()?.device()?;
        queue.respond(CommandResponse {
            tid: params.tid,
            value: CommandResponseDataType::DeviceList(DeviceListResult {
                items: devices.devices,
            }),
        });
        Ok(())
    }

    /// Sets one of the devices from `list_devices` as the active one
    pub fn set_active_device(&mut self, id: String) -> ClientResult<()> {
        trace!("Setting {} as active device", id);
        let devices = self.get_spotify()?.device()?;
        for d in devices.devices {
            if d.id == id {
                info!("Device set as active: {:?}", d);
                self.device = Some(d);
                return Ok(());
            }
        }
        Err(format_err!("No device found with ID {}", id))
    }

    pub fn clear_device(&mut self) {
        self.device = None;
    }

    /// Pause playback
    pub fn pause(&self) -> ClientResult<()> {
        info!("Pausing");
        let id = self.device.clone().and_then(|x| Some(x.id));
        self.get_spotify()?.pause_playback(id)?;
        Ok(())
    }

    /// Clicks the play button
    pub fn resume(&self) -> ClientResult<()> {
        info!("Resume");
        let id = self.device.clone().and_then(|x| Some(x.id));
        self.get_spotify()?.start_playback(id, None, None, None)?;
        Ok(())
    }

    /// Pause playback
    pub fn skip(&mut self) -> ClientResult<()> {
        info!("Skipping track");
        if self.enqueue()? {
            // Loaded next song
            Ok(())
        } else {
            // No song in queue, just skip currently playing one in Spotify client
            let id = self.device.clone().and_then(|x| Some(x.id));
            self.get_spotify()?.next_track(id)?;
            Ok(())
        }
    }

    pub fn search(&self, params: &SearchParams, queue: &mut TaskQueue) -> ClientResult<()> {
        debug!("Searching for {:?}", params);
        let start = Instant::now();
        let search = self
            .get_spotify()?
            .search_track(&params.title, 40, 0, None)?;
        let dur = start.elapsed();
        trace!("Search took {}ms", duration_as_millis(dur));
        let mut sr = vec![];
        for s in search.tracks.items {
            sr.push(s.into());
        }
        queue.respond(CommandResponse {
            tid: params.tid,
            value: CommandResponseDataType::Search(SearchResult { items: sr }),
        });
        Ok(())
    }

    /// Update `status` field
    pub fn update_player_status(&mut self) -> ClientResult<()> {
        self.status = if let None = self.spotify {
            // No spotify API client
            PlaybackStatus {
                state: PlaybackState::NoAuth,
                ..PlaybackStatus::default()
            }
        } else if let None = self.device {
            // No active device
            PlaybackStatus {
                state: PlaybackState::NoDevice,
                ..PlaybackStatus::default()
            }
        } else {
            // Check what is playing
            trace!("Querying current playing");
            let x = self.get_spotify()?.current_playing(None)?;
            parse_playing_context(x)
        };
        Ok(())
    }

    /// Adds specified track to "the list for consideration"
    pub fn request(&mut self, track_id: String) -> ClientResult<()> {
        debug!("Requested song {}", track_id);
        let c = self.get_spotify()?;
        let track = c.track(&track_id)?;
        let x: BasicSongInfo = track.into();
        self.the_list.add(x);
        Ok(())
    }

    /// Reverse a request for song, possibly removing it from the queue
    pub fn downvote(&mut self, track_id: String) -> ClientResult<()> {
        debug!("Downvoted song {}", track_id);
        self.the_list.downvote(track_id);
        Ok(())
    }

    /// Make a song start playing, replacing anything currently playing
    pub fn load_song(&mut self, track: BasicSongInfo) -> ClientResult<()> {
        trace!("Starting playback of song");
        let id = self.device.clone().and_then(|x| Some(x.id));
        self.get_spotify()?
            .start_playback(id, None, Some(vec![track.spotify_uri]), None)?;
        Ok(())
    }

    /// Take a song from the list and make it go. Returns true if song was enqueued, false if not (e.g empty playlist)
    pub fn enqueue(&mut self) -> ClientResult<bool> {
        if let Some(t) = self.the_list.nextup() {
            trace!("Enqueuing song");
            self.load_song(t)?;
            self.status.state = PlaybackState::EnqueuedAndWaiting; // TODO: Is this state necessary?

            // Enqueued a song
            Ok(true)
        } else {
            // No song in queue
            Ok(false)
        }
    }

    /// Called very often, performs regular activities like checking if Spotify is ready to play next song
    pub fn routine(&mut self) -> ClientResult<()> {
        {
            // Wait a reasonable amount of time before pinging Spotify API for playback status
            let time_for_thing = if let Some(lc) = self.last_status_check {
                let x = lc.elapsed()?;
                duration_as_millis(x) > self.status_check_interval_ms.into()
            } else {
                true
            };

            if time_for_thing {
                trace!("Checking status");
                // Sufficent time has elapsed
                self.last_status_check = Some(SystemTime::now());

                // Update status
                self.update_player_status()?;

                // Enqueue song if needed
                if self.status.state == PlaybackState::NeedsSong {
                    self.enqueue()?;
                }
            }
        }
        {
            let time_for_refresh = if let Some(rt) = self.last_token_refresh {
                let x = rt.elapsed()?;
                duration_as_millis(x) > self.token_refresh_interval_ms.into()
            } else {
                true
            };
            if time_for_refresh {
                if let None = self.spotify {
                    // Not yet authenticated, so refreshign token is no-op
                } else {
                    debug!("Time to refresh auth token");
                    self.refresh_auth_token()?;
                }
                self.last_token_refresh = Some(SystemTime::now());
            }
        }
        Ok(())
    }
}
