use std::sync::{Arc, RwLock};
use std::thread::sleep;
use std::time::Duration;

use serde_derive::{Deserialize, Serialize};
use serde_json;

use rouille::{router, Request, Response};

use crate::commands::LockedTaskQueue;
use crate::common::{
    CommandResponse, PlaybackStatus, SearchParams, SongRequestInfo, SpotifyCommand, TaskID,
};

#[derive(Serialize, Deserialize)]
enum WebResponse {
    Success,
    Status(PlaybackStatus),
}

#[derive(Serialize, Deserialize)]
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

fn handle_response(
    request: &Request,
    queue: &LockedTaskQueue,
    global_status: &RwLock<Option<PlaybackStatus>>,
) -> Response {
    if let Some(request) = request.remove_prefix("/static") {
        return rouille::match_assets(&request, "public");
    }

    // Main route
    router!(request,
        (GET) (/) => {
            // Index
            // FIXME: Serve status stuff
            Response::html("Hi")
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
        (GET) (/api/request) => {
            // Add song to the list
            let id = request.get_param("track_id");
            if let Some(x) = id {
                queue.lock().unwrap().queue(SpotifyCommand::Request(SongRequestInfo{track_id: x}));
                Response::text("ok")
            } else {
                Response::text("missing track_id").with_status_code(500)
            }
        },
        (GET) (/api/status) => {
            let s = global_status.read().unwrap().clone();
            let info = match s {
                None => Response::text("Nope"),
                Some(t) => Response::json(&WebResponse::Status(t)),
            };
            info
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
            Response::json(&r.value)
        },

        // default route
        _ => Response::text("404 Not found").with_status_code(404)
    )
}

/// Start web-server
pub fn web(queue: LockedTaskQueue, global_status: Arc<RwLock<Option<PlaybackStatus>>>) {
    rouille::start_server("0.0.0.0:8081", move |request| {
        handle_response(request, &queue.clone(), &global_status.clone())
    });
}
