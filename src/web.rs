use std::sync::{Arc, RwLock};
use std::thread::sleep;
use std::time::Duration;

use serde_derive::{Deserialize, Serialize};
use serde_json;

use rouille::{router, try_or_400, websocket, Request, Response};

use crate::client::TheList;
use crate::commands::LockedTaskQueue;
use crate::common::{
    CommandResponse, CommandResponseDataType, PlaybackStatus, SearchParams, SearchResult,
    SongRequestInfo, SpotifyCommand, TaskID,
};

#[derive(Debug, Serialize, Deserialize)]
pub enum WebResponse {
    Success,
    Status(PlaybackStatus),
    Search(SearchResult),
    Queue(TheList),
}

#[derive(Debug, Serialize, Deserialize)]
enum MaybeWebResponse {
    Error(String),
    Response(WebResponse),
}

fn wait_for_task(queue: &LockedTaskQueue, tid: TaskID) -> CommandResponse {
    loop {
        // TODO: Timeout?
        {
            let response = queue.lock().unwrap().wait(tid);
            if let Some(r) = response {
                return r;
            }
            // Drop lock
        }

        sleep(Duration::from_millis(100));
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
                    let q = global_queue.read().unwrap().clone();
                    let info = WebResponse::Queue(q);
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
}

fn handle_response(
    request: &Request,
    queue: &LockedTaskQueue,
    global_status: &Arc<RwLock<PlaybackStatus>>,
    global_queue: &Arc<RwLock<TheList>>,
) -> Response {
    if let Some(request) = request.remove_prefix("/static") {
        // TODO: Maybe use std::include_str! instead to have binary self-contained?
        return rouille::match_assets(&request, "public");
    }

    // Main route
    router!(request,
        (GET) (/) => {
            // Index
            // FIXME: Serve status stuff
            Response::html("Hi")
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
            Response::text("ok")
        },
        (GET) (/api/pause) => {
            // Pause
            queue.lock().unwrap().queue(SpotifyCommand::Pause);
            Response::text("ok")
        },
        (GET) (/api/request/{track_id:String}) => {
            // Add song to the list
            queue.lock().unwrap().queue(SpotifyCommand::Request(SongRequestInfo{track_id: track_id}));
            Response::text("ok")
        },
        (GET) (/api/status) => {
            let s = global_status.read().unwrap().clone();
            Response::json(&WebResponse::Status(s))
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
                CommandResponseDataType::Search(d) => WebResponse::Search(d)
            };
            Response::json(&inner)
        },

        // default route
        _ => Response::text("404 Not found").with_status_code(404)
    )
}

/// Start web-server
pub fn web(
    queue: LockedTaskQueue,
    global_status: Arc<RwLock<PlaybackStatus>>,
    global_queue: Arc<RwLock<TheList>>,
) {
    rouille::start_server("0.0.0.0:8081", move |request| {
        handle_response(request, &queue.clone(), &global_status, &global_queue)
    });
}
