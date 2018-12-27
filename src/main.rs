/*
Dodos spotify:track:0b05H1iP6hdx8ue7XQlC5J
Thao  spotify:track:67k9jnPe4dSqvAfrM902Z0
*/

use rspotify::spotify::oauth2::{SpotifyClientCredentials, SpotifyOAuth};
use rspotify::spotify::util::get_token;
use std::sync::RwLock;
use std::sync::{Arc, Mutex};
use std::thread;
use std::thread::sleep;
use std::time::Duration;

use failure::{format_err, Error};

use rspotify::spotify::client::Spotify;

mod client;
mod commands;
mod common;
mod web;

use crate::client::Client;
use crate::commands::{LockedTaskQueue, TaskQueue};
use crate::common::*;
use crate::web::web;

/// Spotify commander thread
fn spotify_ctrl(
    queue: &LockedTaskQueue,
    global_status: &Arc<RwLock<PlaybackStatus>>,
) -> Result<(), Error> {
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

        let mut s = global_status.write().unwrap();
        *s = client.status.clone();
    }
}

/// Start all threads
fn main() {
    let status: Arc<RwLock<PlaybackStatus>> = Arc::new(RwLock::new(PlaybackStatus::default()));

    let queue = Arc::new(Mutex::new(TaskQueue::new()));

    let q1 = queue.clone();
    let s1 = status.clone();
    let w = thread::spawn(move || web(q1, s1));

    let q2 = queue.clone();
    let s2 = status.clone();
    let s = thread::spawn(move || spotify_ctrl(&q2, &s2));
    s.join().unwrap().unwrap();
    w.join().unwrap();
}

#[cfg(test)]
mod tests {
    use super::main;
    use std::thread;

    #[test]
    fn test_basic() {
        let _t = thread::spawn(main);
        let mut resp =
            reqwest::get("http://localhost:8081/search/track/The Dodos Walking").unwrap();
        assert!(resp.status().is_success());
        let data: crate::web::WebResponse = resp.json().unwrap();
        println!("{:?}", data);
        if let crate::web::WebResponse::Search(s) = data {
            // Check more than one result, and that all contain the word "dodos"
            assert!(s.items.len() > 0);
            assert!(s
                .items
                .iter()
                .map(|x| format!("{} {}", x.name, x.artists.join(", ")))
                .any(|x| x.to_lowercase().contains("dodo")));
        }
    }
}
