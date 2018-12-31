use failure::format_err;

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
    pub songs: std::collections::HashMap<String, i64>,
}

impl TheList {
    pub fn new() -> TheList {
        TheList {
            songs: std::collections::HashMap::new(),
        }
    }

    fn add(&mut self, track_id: String) {
        let existing: i64 = *self.songs.get(&track_id).unwrap_or(&0);
        self.songs.insert(track_id, existing + 1);
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

/// Handles playback/queue logic and commands Spotify
pub struct Client {
    spotify: Option<Spotify>,
    device: Option<Device>,
    pub the_list: TheList,
    last_status_check: Option<SystemTime>,
    pub status: PlaybackStatus,
    status_check_interval_ms: u32,
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
            status: PlaybackStatus::default(),
            status_check_interval_ms: 1000,
        }
    }

    pub fn set_auth_token(&mut self, token: &TokenInfo) {
        let client_credential = SpotifyClientCredentials::default()
            .token_info(token.clone())
            .build();
        self.spotify = Some(
            Spotify::default()
                .client_credentials_manager(client_credential)
                .build(),
        );
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
        let devices = self.get_spotify()?.device()?;
        for d in devices.devices {
            if d.id == id {
                println!("Device set as active: {:?}", d);
                self.device = Some(d);
                return Ok(());
            }
        }
        Err(format_err!("No device found with ID {}", id))
    }

    /// Pause playback
    pub fn pause(&self) -> ClientResult<()> {
        self.get_spotify()?.pause_playback(None)?;
        Ok(())
    }

    /// Clicks the play button
    pub fn resume(&self) -> ClientResult<()> {
        let id = self.device.clone().and_then(|x| Some(x.id));
        self.get_spotify()?.start_playback(id, None, None, None)?;

        Ok(())
    }

    pub fn search(&self, params: &SearchParams, queue: &mut TaskQueue) -> ClientResult<()> {
        let start = Instant::now();
        let search = self
            .get_spotify()?
            .search_track(&params.title, 10, 0, None)?;
        let dur = start.elapsed();
        println!(
            // FIXME: Use logging
            "Search took {}",
            dur.as_secs() * 1000 + u64::from(dur.subsec_millis())
        );
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
            let x = self.get_spotify()?.current_playing(None)?;
            parse_playing_context(x)
        };
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
        self.get_spotify()?
            .start_playback(id, None, Some(vec![track_id]), None)?;
        Ok(())
    }

    /// Take a song from the list and make it go
    pub fn enqueue(&mut self) -> ClientResult<()> {
        if let Some(t) = self.the_list.nextup() {
            self.load_song(t)?;
            self.status.state = PlaybackState::EnqueuedAndWaiting; // TODO: Is this state necessary?
        }
        Ok(())
    }

    /// Called very often, performs regular activities like checking if Spotify is ready to play next song
    pub fn routine(&mut self) -> ClientResult<()> {
        // Wait a reasonable amount of time before pinging Spotify API for playback status
        let time_for_thing = if let Some(lc) = self.last_status_check {
            let x = lc.elapsed()?;
            x.as_secs() > 0 || x.subsec_millis() > self.status_check_interval_ms
        } else {
            true
        };

        if time_for_thing {
            // Sufficent time has elapsed
            self.last_status_check = Some(SystemTime::now());

            // Update status
            self.update_player_status()?;

            // Enqueue song if needed
            if self.status.state == PlaybackState::NeedsSong {
                self.enqueue()?;
            }
        }
        Ok(())
    }
}
