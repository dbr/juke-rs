use log::{info, trace, warn};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::RwLock;
use std::sync::{Arc, Mutex};
use std::thread;
use std::thread::sleep;
use std::time::Duration;

use failure::Error;

mod client;
mod commands;
mod common;
mod web;

use crate::client::{Client, TheList};
use crate::commands::{LockedTaskQueue, TaskQueue};
use crate::common::*;
use crate::web::web;

/// Spotify commander thread
fn spotify_ctrl(
    queue: &LockedTaskQueue,
    global_status: &Arc<RwLock<PlaybackStatus>>,
    global_queue: &Arc<RwLock<TheList>>,
    running: Arc<AtomicBool>,
) -> Result<(), Error> {
    // Create client wrapper
    let mut client = Client::new();

    let mut innerloop = || -> Result<(), Error> {
        while running.load(Ordering::SeqCst) {
            // Wait for commands from the web-thread
            let queue_content = {
                let mut q = queue.lock().unwrap();
                q.pop()
            };
            if let Some(c) = queue_content {
                trace!("Got command: {:?}", c);
                match c {
                    SpotifyCommand::Pause => client.pause()?,
                    SpotifyCommand::Resume => client.resume()?,
                    SpotifyCommand::Skip => client.skip()?,
                    SpotifyCommand::Request(ri) => client.request(ri.track_id)?,
                    SpotifyCommand::Downvote(ri) => client.downvote(ri.track_id)?,
                    SpotifyCommand::Search(sp) => client.search(&sp, &mut queue.lock().unwrap())?,
                    SpotifyCommand::SetAuthToken(t) => client.set_auth_token(&t),
                    SpotifyCommand::ClearAuth => client.clear_auth(),
                    SpotifyCommand::ListDevices(lp) => {
                        client.list_devices(&lp, &mut queue.lock().unwrap())?
                    }
                    SpotifyCommand::SetActiveDevice(id) => client.set_active_device(id)?,
                    SpotifyCommand::ClearDevice => client.clear_device(),
                };
            } else {
                // Wait for new commands
                sleep(Duration::from_millis(50));
                client.routine()?;
            }

            // Update global status object if needed
            if *global_status.read().unwrap() != client.status {
                // TODO: Is this even necessary, could it just update always?
                trace!("Updating global status");
                let mut s = global_status.write().unwrap();
                *s = client.status.clone();
            }

            if *global_queue.read().unwrap() != client.the_list {
                // TODO: Is this even necessary, could it just update always?
                trace!("Updating global queue");
                let mut q = global_queue.write().unwrap();
                *q = client.the_list.clone();
            }
        }

        Ok(())
    };

    while running.load(Ordering::SeqCst) {
        let r = innerloop();
        match r {
            Ok(_) => (),
            Err(e) => warn!("{:?}", e),
        }
    }
    Ok(())
}

/// Start all threads
fn main() {
    let cfg = Config {
        web_host: "0.0.0.0".to_string(),
        web_port: std::env::var("PORT")
            .unwrap_or("8081".to_string())
            .parse::<u32>()
            .expect("Malformed $PORT value"),
    };
    std::env::set_var("RUST_LOG", "juke=debug");
    env_logger::init();

    let running = Arc::new(AtomicBool::new(true));

    info!("Setup commencing");
    let status: Arc<RwLock<PlaybackStatus>> = Arc::new(RwLock::new(PlaybackStatus::default()));

    let tasks = Arc::new(Mutex::new(TaskQueue::new()));
    let thelist = Arc::new(RwLock::new(TheList::new()));

    let r = running.clone();
    ctrlc::set_handler(move || {
        if r.load(Ordering::SeqCst) {
            // Set `running` to false, stops threads
            println!("Interrupt");
            r.store(false, Ordering::SeqCst);
        } else {
            // If pressing ctrl+c twice, immediately terminate in case threads only stop
            println!("Second interrupt, exiting");
            std::process::exit(1);
        }
    })
    .expect("Error setting Ctrl-C handler");

    info!("Starting web thread");
    let thread_web = {
        let q1 = tasks.clone();
        let s1 = status.clone();
        let l1 = thelist.clone();
        let r1 = running.clone();
        thread::spawn(move || web(q1, s1, l1, r1, &cfg))
    };

    info!("Starting Spotify thread");
    let thread_spotify = {
        let q2 = tasks.clone();
        let s2 = status.clone();
        let l2 = thelist.clone();
        let r2 = running.clone();
        thread::spawn(move || spotify_ctrl(&q2, &s2, &l2, r2))
    };

    thread_spotify.join().unwrap().unwrap();
    thread_web.join().unwrap();

    // TODO: Save TheList etc?
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
                .map(|x| format!("{} {}", x.title, x.artist))
                .any(|x| x.to_lowercase().contains("dodo")));
        }
    }
}
