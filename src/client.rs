use rspotify::spotify::client::Spotify;
use rspotify::spotify::model::device::Device;

use rand::seq::SliceRandom;
use std::collections::hash_map::Entry;
use std::time::{Instant, SystemTime};

use crate::commands::TaskQueue;
use crate::common::*;

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
pub struct Client<'a> {
    spotify: &'a Spotify,
    device: Option<Device>,
    the_list: TheList,
    last_status_check: Option<SystemTime>,
    pub status: PlaybackStatus,
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

impl<'a> Client<'a> {
    pub fn new(client: &'a Spotify) -> Client<'a> {
        Client {
            spotify: &client,
            device: None,
            the_list: TheList::new(),
            last_status_check: None,
            status: PlaybackStatus::default(),
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
        let start = Instant::now();
        let search = self.spotify.search_track(&params.title, 10, 0, None)?;
        let dur = start.elapsed();
        println!(
            // FIXME: Use logging
            "Search took {}",
            dur.as_secs() * 1000 + u64::from(dur.subsec_millis())
        );
        let mut sr = vec![];
        for s in search.tracks.items {
            sr.push(SearchResultSong {
                name: s.name,
                artists: s.artists.iter().map(|x| x.name.clone()).collect(),
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
            self.status.state = PlaybackState::EnqueuedAndWaiting; // TODO: Is this state necessary?
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

            // Enqueue song if needed
            if self.status.state == PlaybackState::NeedsSong {
                self.enqueue()?;
            }
        }
        Ok(())
    }
}
