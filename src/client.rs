use rspotify::spotify::client::Spotify;
use rspotify::spotify::model::device::Device;

use rand::seq::SliceRandom;
use std::collections::hash_map::Entry;
use std::time::SystemTime;

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
