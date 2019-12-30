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
mod player;
mod web;

use crate::client::{Client, TheList};
use crate::commands::{LockedTaskQueue, TaskQueue};
use crate::common::*;
use crate::player::player;
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
                    SpotifyCommand::Pause => (),
                    SpotifyCommand::Resume => (),
                    SpotifyCommand::Skip => (),
                    SpotifyCommand::Request(ri) => {
                        client.request(ri.track_id, &mut queue.lock().unwrap())?
                    }
                    SpotifyCommand::Enqueue(si) => {
                        let mut gq = global_queue.write().unwrap();
                        gq.add(si);
                    }
                    SpotifyCommand::Search(sp) => client.search(&sp, &mut queue.lock().unwrap())?,
                    SpotifyCommand::SetAuthToken(t) => client.set_auth_token(&t),
                    SpotifyCommand::ClearAuth => client.clear_auth(),
                };
            } else {
                // Wait for new commands
                sleep(Duration::from_millis(50));
                client.routine()?;
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
    std::env::set_var("RUST_LOG", "juke=trace");
    env_logger::init();

    let running = Arc::new(AtomicBool::new(true));

    info!("Setup commencing");
    let status: Arc<RwLock<PlaybackStatus>> = Arc::new(RwLock::new(PlaybackStatus {
        state: PlaybackState::NoAuth,
        ..PlaybackStatus::default()
    }));

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

    info!("Starting player thread");
    let thread_player = {
        let l1 = thelist.clone();
        let gs = status.clone();
        thread::spawn(move || player(&l1, &gs))
    };

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

    thread_player.join().unwrap().unwrap();
    thread_spotify.join().unwrap().unwrap();
    thread_web.join().unwrap();

    // TODO: Save TheList etc?
}
