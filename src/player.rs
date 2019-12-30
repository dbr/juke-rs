extern crate librespot;
extern crate tokio_core;

use log::{debug, info, trace};
use std::sync::{Arc, RwLock};

use tokio_core::reactor::Core;

use librespot::core::authentication::Credentials;
use librespot::core::config::SessionConfig;
use librespot::core::session::Session;
use librespot::core::spotify_id::SpotifyId;
use librespot::playback::config::PlayerConfig;

use librespot::playback::audio_backend;
use librespot::playback::player::Player;

use failure::Error;

use crate::client::TheList;
use crate::common::{PlaybackState, PlaybackStatus};

pub fn player(
    queue: &Arc<RwLock<TheList>>,
    global_status: &Arc<RwLock<PlaybackStatus>>,
) -> Result<(), Error> {
    let mut core = Core::new().unwrap();
    let handle = core.handle();

    let session_config = SessionConfig::default();
    let player_config = PlayerConfig::default();

    let username = std::env::var("TEMP_SPOTIFY_USER").expect("Missing TEMP_SPOTIFY_USER"); // FIXME!
    let password = std::env::var("TEMP_SPOTIFY_PASS").expect("Missing TEMP_SPOTIFY_PASS");
    let credentials = Credentials::with_password(username, password);

    // Create backend
    let backend = audio_backend::find(None).unwrap();

    // Create session
    let session = core.run(Session::connect(session_config, credentials, None, handle))?;

    // Create player
    let (player, _) = Player::new(player_config, session.clone(), None, move || {
        (backend)(None)
    });

    println!("Starting play loop");
    loop {
        // Get next song from queue
        let song: Option<crate::common::BasicSongInfo> = {
            let mut q = queue.write().unwrap();
            q.nextup()
        };

        if let Some(s) = song {
            // Queue had song

            // First update status
            {
                info!("Starting to play");
                *global_status.write().unwrap() = PlaybackStatus {
                    state: PlaybackState::Playing,
                    song: Some(s.clone()),
                    ..PlaybackStatus::default()
                };
            }
            // Convert ID
            let sid = SpotifyId::from_uri(&s.spotify_uri).unwrap();

            // Play
            core.run(player.load(sid, true, 0)).unwrap();

            // Update status after song completes
            {
                *global_status.write().unwrap() = PlaybackStatus {
                    state: PlaybackState::NeedsSong,
                    ..PlaybackStatus::default()
                };
            }
        } else {
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
    }
}
