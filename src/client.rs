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
    pub songs: std::collections::HashMap<String, BasicSongInfo>,
}

impl TheList {
    pub fn new() -> TheList {
        TheList {
            songs: std::collections::HashMap::new(),
        }
    }

    pub fn add(&mut self, track_id: BasicSongInfo) {
        debug!("Added song {:?}", track_id);
        let key = &track_id.spotify_uri;
        self.songs.entry(key.clone()).or_insert(track_id);
        trace!("The list after: {:?}", self);
    }

    pub fn nextup(&mut self) -> Option<BasicSongInfo> {
        debug!("Getting next song from the list");
        let key = {
            let mut rng = rand::thread_rng();
            let all_keys: Vec<&String> = self.songs.iter().map(|x| x.0).collect();
            let k = all_keys.choose(&mut rng)?.clone();
            k.clone()
        };

        if let Entry::Occupied(o) = self.songs.entry(key) {
            let (_, value) = o.remove_entry();
            trace!("Dropping song from list, new list is {:?}", self.songs);
            return Some(value);
        } else {
            return None;
        };
    }
}

impl Drop for TheList {
    fn drop(&mut self) {
        trace!("Dropping the list!");
    }
}

/// Handles playback/queue logic and commands Spotify
pub struct Client {
    spotify: Option<Spotify>,
    last_token_refresh: Option<SystemTime>,
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

impl Client {
    pub fn new() -> Client {
        Client {
            spotify: None,
            last_token_refresh: None,
            token_refresh_interval_ms: 1000 * 60 * 5,
        }
    }

    /// End session with Spotify
    pub fn clear_auth(&mut self) {
        self.spotify = None;
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

    /// Adds specified track to "the list for consideration"
    pub fn request(&mut self, track_id: String, queue: &mut TaskQueue) -> ClientResult<()> {
        debug!("Requested song {}", track_id);
        let c = self.get_spotify()?;
        let track = c.track(&track_id)?;
        let x: BasicSongInfo = track.into();
        if x.title.to_lowercase().contains("scatman")
            || x.title.to_lowercase().contains("freestyler")
        {
            return Ok(());
        }
        queue.queue(SpotifyCommand::Enqueue(x));
        Ok(())
    }

    /// Called very often, performs regular activities like checking if Spotify is ready to play next song
    pub fn routine(&mut self) -> ClientResult<()> {
        {
            // Auth token refresh check
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
