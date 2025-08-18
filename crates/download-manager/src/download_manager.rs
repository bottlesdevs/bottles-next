mod context;
mod download;
mod error;
mod request;
mod worker;

use crate::{context::Context, request::RequestBuilder, worker::download_thread};
pub use crate::{
    download::{Download, DownloadResult, Status},
    error::DownloadError,
    request::Request,
};
use reqwest::Url;
use std::{
    path::Path,
    sync::{Arc, atomic::Ordering},
};
use tokio::sync::mpsc;
use tokio_util::{sync::CancellationToken, task::TaskTracker};

pub struct DownloadManager {
    queue: mpsc::Sender<Request>,
    ctx: Arc<Context>,
    tracker: TaskTracker,
}

impl Default for DownloadManager {
    fn default() -> Self {
        let (tx, rx) = mpsc::channel(100);
        let ctx = Context::new(3);
        let tracker = TaskTracker::new();

        let manager = Self {
            queue: tx,
            ctx: ctx.clone(),
            tracker: tracker.clone(),
        };

        tracker.spawn(dispatcher_thread(ctx, rx, tracker.clone()));
        manager
    }
}

impl DownloadManager {
    pub fn download_builder(&self, url: Url, destination: impl AsRef<Path>) -> RequestBuilder {
        let destination = destination.as_ref().to_path_buf();

        Request::builder(self).url(url).destination(destination)
    }

    fn queue_request(&self, request: Request) -> Result<(), DownloadError> {
        self.queue.try_send(request).map_err(|e| match e {
            mpsc::error::TrySendError::Full(_) => DownloadError::QueueFull,
            mpsc::error::TrySendError::Closed(_) => DownloadError::ManagerShutdown,
        })
    }

    /// Returns the count of pending requests still buffered in the internal mpsc channel.
    ///
    /// **Note**: This excludes any request already dequeued by the dispatcher but not yet started.
    ///
    /// Consider replacing with an explicit atomic counter.
    pub fn queued_downloads(&self) -> usize {
        self.queue.max_capacity() - self.queue.capacity()
    }

    /// Consider replacing with an explicit atomic counter.
    pub fn active_downloads(&self) -> usize {
        // -1 because the dispatcher thread is always running
        self.tracker.len() - 1
    }

    pub fn cancel_all(&self) {
        self.ctx.cancel_all();
    }

    pub fn child_token(&self) -> CancellationToken {
        self.ctx.child_token()
    }
}

async fn dispatcher_thread(
    ctx: Arc<Context>,
    mut rx: mpsc::Receiver<Request>,
    tracker: TaskTracker,
) {
    struct ActiveGuard {
        ctx: Arc<Context>,
        _permit: tokio::sync::OwnedSemaphorePermit,
    }

    impl Drop for ActiveGuard {
        fn drop(&mut self) {
            self.ctx
                .active
                .fetch_sub(1, std::sync::atomic::Ordering::Relaxed);
        }
    }

    while let Some(request) = rx.recv().await {
        let guard = match ctx.semaphore.clone().acquire_owned().await {
            Ok(p) => {
                ctx.active.fetch_add(1, Ordering::Relaxed);
                ActiveGuard {
                    ctx: ctx.clone(),
                    _permit: p,
                }
            }
            Err(_) => break,
        };
        let client = ctx.client.clone();

        tracker.spawn(async move {
            // Move the guard into the worker thread so it's automatically released when the thread finishes
            let _guard = guard;
            download_thread(client, request).await
        });
    }
}
