use crate::commands::LockedTaskQueue;
use crate::common::{SearchParams, SongRequestInfo, SpotifyCommand, TaskID};
use rouille::{router, Request, Response};
use std::thread::sleep;
use std::time::Duration;

fn handle_response(request: &Request, queue: &LockedTaskQueue) -> Response {
    router!(request,
        (GET) (/) => {
            // Index
            // FIXME: Serve status stuff
            Response::html("Hi")
        },
        (GET) (/ctrl/resume) => {
            // Play
            queue.lock().unwrap().queue(SpotifyCommand::Resume);
            Response::text("ok")
        },
        (GET) (/ctrl/pause) => {
            // Pause
            queue.lock().unwrap().queue(SpotifyCommand::Pause);
            Response::text("ok")
        },
        (GET) (/ctrl/request) => {
            // Add song to the list
            let id = request.get_param("track_id");
            if let Some(x) = id {
                queue.lock().unwrap().queue(SpotifyCommand::Request(SongRequestInfo{track_id: x}));
                Response::text("ok")
            } else {
                Response::text("missing track_id").with_status_code(500)
            }
        },
        (GET) (/search/track/{term:String}) => {
            // Queue search task and drop lock
            let tid: TaskID = {
                let mut q = queue.lock().unwrap();
                let tid = q.get_task_id();
                q.queue(SpotifyCommand::Search(SearchParams{tid: tid, title: term}));
                tid
            };

            loop {
                // TODO: Timeout?
                {
                    let response = queue.lock().unwrap()
                        .wait(tid);
                    if let Some(r) = response {
                        return Response::text(format!("kk: {:?}", r));
                    }
                    // Drop lock
                }

                sleep(Duration::from_millis(100));
            };
            unreachable!();
        },

        // default route
        _ => Response::text("404 Not found").with_status_code(404)
    )
}

/// Start web-server
pub fn web(queue: LockedTaskQueue) {
    rouille::start_server("0.0.0.0:8081", move |request| {
        handle_response(request, &queue.clone())
    });
}
