mod context;
mod download;
mod error;
mod events;
mod request;
mod worker;

use crate::{context::Context, request::RequestBuilder, worker::download_thread};
pub use crate::{
    context::DownloadID,
    download::{Download, DownloadResult},
    error::DownloadError,
    events::{DownloadEvent, Progress},
    request::Request,
};
use futures_core::Stream;
use reqwest::Url;
use std::{
    path::Path,
    sync::{Arc, atomic::Ordering},
};
use tokio::sync::{broadcast, mpsc};
use tokio_stream::wrappers::BroadcastStream;
use tokio_util::{sync::CancellationToken, task::TaskTracker};

pub struct DownloadManager {
    queue: mpsc::Sender<Request>,
    ctx: Arc<Context>,
    tracker: TaskTracker,
}

impl Default for DownloadManager {
    fn default() -> Self {
        DownloadManager::builder()
            .max_concurrent(3)
            .queue_size(100)
            .build()
            .unwrap()
    }
}

impl DownloadManager {
    pub fn builder() -> DownloadManagerBuilder {
        DownloadManagerBuilder::new()
    }

    pub fn download(&self, url: Url, destination: impl AsRef<Path>) -> anyhow::Result<Download> {
        self.download_builder()
            .url(url)
            .destination(destination)
            .start()
    }

    pub fn download_builder(&self) -> RequestBuilder {
        Request::builder(self)
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

    pub fn active_downloads(&self) -> usize {
        self.ctx.active.load(Ordering::Relaxed)
    }

    pub fn cancel_all(&self) {
        self.ctx.cancel_all();
    }

    pub fn child_token(&self) -> CancellationToken {
        self.ctx.child_token()
    }

    pub fn subscribe(&self) -> broadcast::Receiver<DownloadEvent> {
        self.ctx.events.subscribe()
    }

    pub fn events(&self) -> impl Stream<Item = DownloadEvent> + 'static {
        use tokio_stream::StreamExt as _;

        BroadcastStream::new(self.subscribe()).filter_map(|res| res.ok())
    }
}

pub struct DownloadManagerBuilder {
    max_concurrent: Option<usize>,
    queue_size: Option<usize>,
}

impl DownloadManagerBuilder {
    pub fn new() -> Self {
        Self {
            max_concurrent: None,
            queue_size: None,
        }
    }

    pub fn max_concurrent(mut self, max: usize) -> Self {
        self.max_concurrent = Some(max);
        self
    }

    pub fn queue_size(mut self, size: usize) -> Self {
        self.queue_size = Some(size);
        self
    }

    pub fn build(self) -> anyhow::Result<DownloadManager> {
        let max_concurrent = self.max_concurrent.filter(|&n| n > 0).ok_or_else(|| {
            anyhow::anyhow!("Max concurrent downloads must be set and greater than 0")
        })?;
        let queue_size = self
            .queue_size
            .filter(|&n| n > 0)
            .ok_or_else(|| anyhow::anyhow!("Queue size must be set and greater than 0"))?;

        let (tx, rx) = mpsc::channel(queue_size);
        let ctx = Context::new(max_concurrent);
        let tracker = TaskTracker::new();

        let manager = DownloadManager {
            queue: tx,
            ctx: ctx.clone(),
            tracker: tracker.clone(),
        };

        tracker.spawn(dispatcher_thread(ctx, rx, tracker.clone()));

        Ok(manager)
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
