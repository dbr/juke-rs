use std::sync::{Arc, Mutex};

use crate::common::{CommandResponse, SpotifyCommand, TaskID};

#[derive(Default, Debug)]
pub struct TaskQueue {
    queue: std::collections::VecDeque<SpotifyCommand>,
    responses: std::collections::VecDeque<CommandResponse>,
    last_task_id: u64,
}

impl TaskQueue {
    pub fn new() -> TaskQueue {
        TaskQueue::default()
    }
    pub fn get_task_id(&mut self) -> TaskID {
        self.last_task_id += 1;
        TaskID {
            id: self.last_task_id,
        }
    }
    pub fn queue(&mut self, c: SpotifyCommand) {
        self.queue.push_back(c);
    }
    pub fn wait(&mut self, task_id: TaskID) -> Option<CommandResponse> {
        let mut idx: Option<usize> = None;
        for (i, c) in self.responses.iter().enumerate() {
            if c.tid == task_id {
                idx = Some(i);
                break;
            }
        }
        if let Some(i) = idx {
            let thing = self.responses.remove(i).unwrap();
            return Some(thing);
        }
        None
    }
    pub fn respond(&mut self, value: CommandResponse) {
        self.responses.push_back(value)
    }
    pub fn pop(&mut self) -> Option<SpotifyCommand> {
        self.queue.pop_back()
    }
}

/// Commands being sent from web thread to Spotify controller thread
pub type LockedTaskQueue = Arc<Mutex<TaskQueue>>;
