//! Coordinated lifecycle for background workers.
//!
//! `CoreRuntime` collects spawned `JoinHandle`s and a `Notify`-based cancel
//! token. Workers are expected to `select!` on `runtime.cancel.notified()` at
//! their loop boundary so a single `runtime.shutdown(timeout)` call drains
//! them gracefully (Round 2 P1-3 + Round 3 P1-2).

use std::sync::Arc;
use std::time::Duration;

use tokio::sync::Notify;
use tokio::task::JoinHandle;

pub struct CoreRuntime {
    workers: Vec<JoinHandle<()>>,
    cancel: Arc<Notify>,
}

impl CoreRuntime {
    pub fn new() -> Self {
        Self {
            workers: Vec::new(),
            cancel: Arc::new(Notify::new()),
        }
    }

    /// Returns a clone of the cancel handle. Workers should hold this and
    /// call `cancel.notified().await` from their `tokio::select!` arm.
    pub fn cancel_handle(&self) -> Arc<Notify> {
        Arc::clone(&self.cancel)
    }

    /// Tracks an already-spawned worker handle.
    pub fn track(&mut self, handle: JoinHandle<()>) {
        self.workers.push(handle);
    }

    /// Spawns a future onto the current runtime and tracks its handle.
    pub fn spawn<F>(&mut self, future: F)
    where
        F: std::future::Future<Output = ()> + Send + 'static,
    {
        self.workers.push(tokio::spawn(future));
    }

    /// Signal cancel + wait for all workers up to `timeout`. Workers that
    /// do not finish in time are aborted — the OS reclaims their resources.
    pub async fn shutdown(self, timeout: Duration) {
        self.cancel.notify_waiters();
        let join_all = futures::future::join_all(self.workers);
        if tokio::time::timeout(timeout, join_all).await.is_err() {
            log::warn!(
                "CoreRuntime.shutdown timed out after {:?} — outstanding workers will be aborted",
                timeout
            );
        }
    }
}

impl Default for CoreRuntime {
    fn default() -> Self {
        Self::new()
    }
}
