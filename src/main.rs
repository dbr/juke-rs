extern crate rspotify;
#[macro_use]
extern crate failure;

use rspotify::spotify::client::Spotify;
use rspotify::spotify::model::offset::for_position;
use rspotify::spotify::oauth2::{SpotifyClientCredentials, SpotifyOAuth};
use rspotify::spotify::util::get_token;

use rspotify::spotify::model::device::Device;

use failure::Error;

type ClientResult<T> = Result<T, Error>;

pub struct Client<'a> {
    spotify: &'a Spotify,
    device: Option<Device>,
}

impl<'a> Client<'a> {
    pub fn new(client: &'a Spotify) -> Client<'a> {
        Client {
            spotify: &client,
            device: None,
        }
    }

    pub fn list_devices(&self) -> ClientResult<Vec<Device>> {
        let devices = self.spotify.device()?;
        Ok(devices.devices)
    }

    pub fn set_active_device(&mut self, device: Device) -> ClientResult<()> {
        self.device = Some(device);
        Ok(())
    }

    pub fn pause(&self) -> ClientResult<()> {
        self.spotify.pause_playback(None)?;
        Ok(())
    }

    pub fn resume(&self) -> ClientResult<()> {
        let id = self.device.clone().and_then(|x| Some(x.id));
        self.spotify.start_playback(id, None, None, None)?;

        Ok(())
    }
}

/*
fn go() -> Result<(), Error> {
    let mut c = Client::new(&spotify);
    let devices = c.list_devices()?;
    c.set_active_device(devices[0].clone())?;
    c.resume()?;
    c.pause()?;

    return Ok(());
}

fn _main() {
    match go() {
        Ok(_) => (),
        Err(e) => println!("{:?}", e),
    }
}
*/

#[macro_use]
extern crate rouille;
use rouille::{Request, Response};
use std::thread;
use std::thread::sleep;
use std::time::Duration;

use std::sync::{Arc, Mutex};

type CommandQueue = Arc<Mutex<std::collections::VecDeque<SpotifyCommand>>>;

fn handle_response(request: &Request, queue: CommandQueue) -> Response {
    router!(request,
        (GET) (/) => {
            Response::html("Hi")
        },
        (GET) (/ctrl/resume) => {
            queue.lock().unwrap().push_back(SpotifyCommand::Resume);
            Response::text("ok")
        },
        (GET) (/ctrl/pause) => {
            queue.lock().unwrap().push_back(SpotifyCommand::Pause);
            Response::text("ok")
        },

        // default route
        _ => Response::text("404 Not found").with_status_code(404)
    )
}

fn web(queue: CommandQueue) {
    rouille::start_server("0.0.0.0:8081", move |request| {
        handle_response(request, queue.clone())
    });
}

fn spotify_ctrl(queue: CommandQueue) -> Result<(), Error> {
    // Perform auth
    let mut oauth = SpotifyOAuth::default()
        .scope("user-read-playback-state user-modify-playback-state")
        .build();

    let token_info = get_token(&mut oauth).ok_or_else(|| format_err!("Failed to get token"))?;

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
    client.set_active_device(devices[0].clone())?;

    loop {
        let queue_content = queue.lock().unwrap().pop_front();
        if let Some(c) = queue_content {
            println!("Got command: {:?}", c);
            let cmd_result = match c {
                SpotifyCommand::Pause => client.pause()?,
                SpotifyCommand::Resume => client.resume()?,
            };
            println!("{:?}", cmd_result);
        } else {
            // Wait for new commands
            sleep(Duration::from_millis(50));
        }
    }
}

#[derive(Debug)]
enum SpotifyCommand {
    Resume,
    Pause,
}

fn main() {
    let queue = Arc::new(Mutex::new(
        std::collections::VecDeque::<SpotifyCommand>::new(),
    ));

    let q1 = queue.clone();
    let q2 = queue.clone();
    let w = thread::spawn(move || web(q1));

    let s = thread::spawn(move || spotify_ctrl(q2));
    s.join().unwrap().unwrap();
    w.join().unwrap();
}

/*
#[cfg(test)]
mod tests {
    use super::main;
    use std::io::Read;
    use std::thread;

    #[test]
    fn test_basic() {
        let _t = thread::spawn(main);
        let mut resp = reqwest::get("http://localhost:8081").unwrap();
        assert!(resp.status().is_success());
        let mut content = String::new();
        resp.read_to_string(&mut content).unwrap();
        assert_eq!(content, "hello world");
    }
}
*/
