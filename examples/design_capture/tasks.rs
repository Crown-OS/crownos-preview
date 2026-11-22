use std::any::Any;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Arc, Mutex};

use tokio::runtime::Handle;
use tokio::task::AbortHandle;
use view::{TaskExecutor, TaskFuture, TaskId};

pub type Finished = (TaskId, Box<dyn Any + Send>);

/// Runs the app's `cx.spawn` futures on a tokio runtime, which an offscreen app leaves unstarted.
pub struct Tasks {
    runtime: Handle,
    finished: Sender<Finished>,
    running: Arc<Mutex<Vec<AbortHandle>>>,
}

/// Stops every task of one mounted app when it is dropped.
pub struct TaskGuard(Arc<Mutex<Vec<AbortHandle>>>);

impl Tasks {
    pub fn new(runtime: Handle) -> (Self, Receiver<Finished>, TaskGuard) {
        let (finished, outputs) = channel();
        let running = Arc::new(Mutex::new(Vec::new()));
        let guard = TaskGuard(Arc::clone(&running));
        let tasks = Self {
            runtime,
            finished,
            running,
        };
        (tasks, outputs, guard)
    }
}

impl TaskExecutor for Tasks {
    fn spawn(&mut self, id: TaskId, future: TaskFuture) {
        let finished = self.finished.clone();
        let handle = self.runtime.spawn(async move {
            let _ = finished.send((id, future.await));
        });
        if let Ok(mut running) = self.running.lock() {
            running.retain(|task| !task.is_finished());
            running.push(handle.abort_handle());
        }
    }

    fn cancel(&mut self, _id: TaskId) {}
}

impl Drop for TaskGuard {
    fn drop(&mut self) {
        if let Ok(running) = self.0.lock() {
            running.iter().for_each(AbortHandle::abort);
        }
    }
}
