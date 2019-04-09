use log::{info, trace};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock};
use std::thread::sleep;
use std::time::Duration;

use serde_derive::{Deserialize, Serialize};
use serde_json;

use rouille::{router, try_or_400, websocket, Request, Response};

use crate::client::TheList;
use crate::commands::LockedTaskQueue;
use crate::common::{
    CommandResponse, CommandResponseDataType, Config, DeviceListParams, DeviceListResult,
    PlaybackStatus, SearchParams, SearchResult, SongRequestInfo, SpotifyCommand, TaskID,
};

#[derive(Debug, Serialize)]
pub enum WebResponse<'a> {
    Success,
    Status(PlaybackStatus),
    Search(SearchResult),
    Queue(&'a TheList),
    DeviceList(DeviceListResult),
    Error(String),
}

#[derive(Debug, Serialize)]
enum MaybeWebResponse<'a> {
    Error(String),
    Response(WebResponse<'a>),
}

/// Wait for given task ID
fn wait_for_task(queue: &LockedTaskQueue, tid: TaskID) -> CommandResponse {
    let duration_ms = 15_000;
    let step = 100;
    for _ in 0..(duration_ms / step) {
        {
            let response = queue.lock().unwrap().wait(tid);
            if let Some(r) = response {
                return r;
            }
            // Drop lock
        }

        sleep(Duration::from_millis(step));
    }

    // Waited too long
    CommandResponse {
        tid: tid,
        value: CommandResponseDataType::Error("Timed out".into()),
    }
}

fn websocket_handling_thread(
    mut websocket: websocket::Websocket,
    global_status: &Arc<RwLock<PlaybackStatus>>,
    global_queue: &Arc<RwLock<TheList>>,
) {
    // We wait for a new message to come from the websocket.
    while let Some(message) = websocket.next() {
        match message {
            websocket::Message::Text(txt) => {
                if txt == "status" {
                    let s = global_status.read().unwrap().clone();

                    let info = WebResponse::Status(s);

                    let t = serde_json::to_string(&info).unwrap();
                    websocket.send_text(&t).unwrap();
                } else if txt == "queue" {
                    let q = global_queue.read().unwrap();
                    let info = WebResponse::Queue(&q);
                    let t = serde_json::to_string(&info).unwrap();
                    websocket.send_text(&t).unwrap();
                } else {
                    websocket
                        .send_text("{\"error\": \"Unknown command\"}")
                        .unwrap();
                }
            }
            websocket::Message::Binary(_) => (),
        }
    }
    trace!("Web socket connection ended");
}

fn generate_random_string(length: usize) -> String {
    use rand::distributions::Alphanumeric;
    use rand::Rng;
    let mut rng = rand::thread_rng();
    std::iter::repeat(())
        .map(|()| rng.sample(Alphanumeric))
        .take(length)
        .collect()
}

static CONTENT_INDEX: &'static str = include_str!("../static/index.html");

fn handle_response(
    request: &Request,
    queue: &LockedTaskQueue,
    global_status: &Arc<RwLock<PlaybackStatus>>,
    global_queue: &Arc<RwLock<TheList>>,
) -> Response {
    if let Some(request) = request.remove_prefix("/static") {
        if !cfg!(debug_assertions) {
            // In release mode, bundle static stuff into binary via include_str!
            if &request.url() == "/thejuke.png" {
                return Response::from_data(
                    "image/png",
                    include_bytes!("../static/thejuke.png").to_vec(),
                );
            };
            let x = match request.url().as_ref() {
                "/app.jsx" => Some((include_str!("../static/app.jsx"), "application/javascript")),
                "/babel.min.js" => Some((
                    include_str!("../static/babel.min.js"),
                    "application/javascript",
                )),
                "/react-dom.production.min.js" => Some((
                    include_str!("../static/react-dom.production.min.js"),
                    "application/javascript",
                )),
                "/react.production.min.js" => Some((
                    include_str!("../static/react.production.min.js"),
                    "application/javascript",
                )),
                "/bootstrap.min.css" => {
                    Some((include_str!("../static/bootstrap.min.css"), "text/css"))
                }
                _ => None,
            };
            return match x {
                None => Response::text("404").with_status_code(404),
                Some((data, t)) => Response::from_data(t, data),
            };
        } else {
            // In debug build, read assets from folder for reloadability
            return rouille::match_assets(&request, "static");
        }
    }

    // Main route
    router!(request,
        (GET) (/) => {
            // Index
            // FIXME: Serve status stuff
            Response::html(CONTENT_INDEX)
        },
        (GET) (/ws) => {
            let (response, websocket) = try_or_400!(websocket::start(&request, Some("juke")));
            let gs = global_status.clone();
            let gq = global_queue.clone();
            std::thread::spawn(move || {
                let ws = websocket.recv().unwrap();
                websocket_handling_thread(ws, &gs, &gq);
            });
            response
        },
        (GET) (/api/resume) => {
            // Play
            queue.lock().unwrap().queue(SpotifyCommand::Resume);
            Response::json(&WebResponse::Success)
        },
        (GET) (/api/pause) => {
            // Pause
            queue.lock().unwrap().queue(SpotifyCommand::Pause);
            Response::json(&WebResponse::Success)
        },
        (GET) (/api/skip) => {
            // Skip song
            queue.lock().unwrap().queue(SpotifyCommand::Skip);
            Response::json(&WebResponse::Success)
        },

        (GET) (/api/request/{track_id:String}) => {
            // Add song to the list
            queue.lock().unwrap().queue(SpotifyCommand::Request(SongRequestInfo{track_id: track_id}));
            Response::json(&WebResponse::Success)
        },
        (GET) (/api/downvote/{track_id:String}) => {
            // Downvote (and possibly remove) song from queue
            queue.lock().unwrap().queue(SpotifyCommand::Downvote(SongRequestInfo{track_id: track_id}));
            Response::json(&WebResponse::Success)
        },

        (GET) (/api/status) => {
            let s = global_status.read().unwrap().clone();
            Response::json(&WebResponse::Status(s))
        },
        (GET) (/api/device/list) => {
            trace!("Request for device list");
            let tid: TaskID = {
                let mut q = queue.lock().unwrap();
                let tid = q.get_task_id();
                q.queue(SpotifyCommand::ListDevices(DeviceListParams{tid: tid}));
                tid
            };
            trace!("Awaiting task");
            let r = wait_for_task(&queue, tid);
            trace!("Got task list");
            let inner = match r.value {
                CommandResponseDataType::DeviceList(d) => WebResponse::DeviceList(d),
                _ => WebResponse::Error(format!("Unexpected response from command in /api/device/list")),
            };
            trace!("Responding with task list to web client");
            Response::json(&inner)
        },
        (GET) (/api/device/set/{id:String}) => {
            queue.lock().unwrap().queue(SpotifyCommand::SetActiveDevice(id));
            Response::text("{\"result\":\"ok\"}")
        },
        (GET) (/api/device/clear) => {
            queue.lock().unwrap().queue(SpotifyCommand::ClearDevice);
            Response::text("{\"result\":\"ok\"}")
        },
        (GET) (/search/track/{term:String}) => {
            // Queue search task and drop lock
            let tid: TaskID = {
                let mut q = queue.lock().unwrap();
                let tid = q.get_task_id();
                q.queue(SpotifyCommand::Search(SearchParams{tid: tid, title: term}));
                tid
            };

            let r = wait_for_task(&queue, tid);
            let inner = match r.value {
                CommandResponseDataType::Search(d) => WebResponse::Search(d),
                _ => WebResponse::Error(format!("Unexpected response from command in /search/track/...")),
            };
            Response::json(&inner)
        },
        (GET) (/auth) => {
            // FIXME: Move elsewhere
            let oauth = rspotify::spotify::oauth2::SpotifyOAuth::default()
                .scope("user-read-playback-state user-modify-playback-state")
                .build();

            let state = generate_random_string(16);
            let auth_url = oauth.get_authorize_url(Some(&state), None);
            Response::redirect_302(auth_url)
        },
        (GET) (/postauth) => {
            let so = rspotify::spotify::oauth2::SpotifyOAuth::default();
            let token = match request.get_param("code") {
                Some(c) => so.get_access_token(&c),
                None => None,
            };
            if let Some(t) = token {
                let mut q = queue.lock().unwrap();
                q.queue(SpotifyCommand::SetAuthToken(t));
                Response::redirect_302("/")
            } else {
                Response::text("Missing ?code= param").with_status_code(500)
            }
        },
        (GET) (/auth/destroy) => {
            {
                let mut q = queue.lock().unwrap();
                q.queue(SpotifyCommand::ClearAuth);
                Response::redirect_302("/")
            }
        },

        // Default route
        _ => Response::text("404 Not found").with_status_code(404)
    )
}

/// Start web-server
pub fn web(
    queue: LockedTaskQueue,
    global_status: Arc<RwLock<PlaybackStatus>>,
    global_queue: Arc<RwLock<TheList>>,
    running: Arc<AtomicBool>,
    cfg: &Config,
) {
    let addr = format!("{}:{}", cfg.web_host, cfg.web_port);
    info!("Listening on http://{}", &addr);
    let srv = rouille::Server::new(&addr, move |request| {
        handle_response(request, &queue.clone(), &global_status, &global_queue)
    })
    .unwrap();

    while running.load(Ordering::SeqCst) {
        srv.poll();
        sleep(Duration::from_millis(10)); // FIXME: https://github.com/tomaka/rouille/issues/200
    }
}
