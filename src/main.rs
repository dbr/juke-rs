extern crate rspotify;
#[macro_use] extern crate failure;

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

impl<'a> Client<'a>{
    pub fn new(client: &'a Spotify) -> Client<'a> {
        Client{
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


fn go() -> Result<(), Error> {

    println!("Creating OAuth token");
    let mut oauth = SpotifyOAuth::default()
        .scope("user-read-playback-state user-modify-playback-state")
        .build();

    let token_info = get_token(&mut oauth).unwrap();
    println!("Creating OAuth token");
    let client_credential = SpotifyClientCredentials::default()
        .token_info(token_info)
        .build();
    println!("Creating client");
    let spotify = Spotify::default()
        .client_credentials_manager(client_credential)
        .build();

    let mut c = Client::new(&spotify);
    let devices = c.list_devices()?;
    c.set_active_device(devices[0].clone())?;
    c.resume()?;
    c.pause()?;

    return Ok(());
}

fn main() {
    match go() {
        Ok(_) => (),
        Err(e) => println!("{:?}", e),
    }
}
