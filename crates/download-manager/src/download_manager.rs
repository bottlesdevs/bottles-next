mod context;
mod download;
mod error;
mod events;
mod request;
mod scheduler;
pub mod prelude {
    pub use crate::{
        context::DownloadID,
        download::{Download, DownloadResult},
        error::DownloadError,
        events::{DownloadEvent, Progress},
        request::Request,
    };
}

use crate::{
    context::Context,
    request::RequestBuilder,
    scheduler::{Scheduler, SchedulerCmd},
};
use futures_core::Stream;
use prelude::*;
use reqwest::Url;
use std::{
    path::Path,
    sync::{Arc, atomic::Ordering},
};
use tokio::sync::{broadcast, mpsc};
use tokio_stream::wrappers::BroadcastStream;
use tokio_util::{sync::CancellationToken, task::TaskTracker};

/// Entry point for scheduling, observing, and cancelling downloads.
///
/// Behavior
/// - Enforces a global concurrency limit across all downloads.
/// - Publishes global DownloadEvent notifications and exposes per-download streams via Download.
///
/// Notes
/// - Events are delivered over a broadcast channel with a bounded buffer; slow consumers can miss events.
/// - Use events() to get a fallible-safe stream that drops lagged messages.
/// - Use shutdown() for a graceful stop: it cancels all work and waits for workers to finish.
pub struct DownloadManager {
    scheduler_tx: mpsc::Sender<SchedulerCmd>,
    ctx: Arc<Context>,
    tracker: TaskTracker,
}

impl Default for DownloadManager {
    fn default() -> Self {
        DownloadManager::builder()
            .max_concurrent(3)
            .build()
            .unwrap()
    }
}

impl DownloadManager {
    /// Create a new builder for DownloadManager.
    ///
    /// You must set a positive max_concurrent on the builder before build().
    /// If you want a sensible default quickly, see [DownloadManager::default()].
    pub fn builder() -> DownloadManagerBuilder {
        DownloadManagerBuilder::new()
    }

    /// Start a download with default request settings.
    ///
    /// - Returns a [Download] handle which is also a Future yielding [DownloadResult] or [DownloadError].
    /// - You can stream progress and per-download events from the returned handle.
    /// - Cancellation: call [Download::cancel()] on the handle, or [DownloadManager::cancel(id)].
    pub fn download(&self, url: Url, destination: impl AsRef<Path>) -> anyhow::Result<Download> {
        self.download_builder()
            .url(url)
            .destination(destination)
            .start()
    }

    /// Create a [RequestBuilder] to customize a download (headers, retries, overwrite, callbacks).
    ///
    /// Use this if you need non-default behavior or want to hook into progress/event callbacks before start().
    pub fn download_builder(&self) -> RequestBuilder {
        Request::builder(self)
    }

    /// Request cancellation for a download by ID.
    ///
    /// - No-op if the job is already finished or missing.
    /// - Returns an error only if the internal command channel is unavailable.
    pub fn cancel(&self, id: DownloadID) -> anyhow::Result<()> {
        self.scheduler_tx
            .try_send(SchedulerCmd::Cancel { id })
            .map_err(|e| anyhow::anyhow!("Failed to send cancel command: {}", e))
    }

    /// Number of currently active (running) downloads.
    ///
    /// Does not include queued or delayed retries. Reflects active semaphore permits.
    pub fn active_downloads(&self) -> usize {
        self.ctx.active.load(Ordering::Relaxed)
    }

    /// Cancel all queued and in-flight downloads managed by this instance.
    ///
    /// This triggers cooperative cancellation for workers and removes partial files.
    pub fn cancel_all(&self) {
        self.ctx.cancel_all();
    }

    /// Return a child [CancellationToken] tied to the manager's root token.
    pub fn child_token(&self) -> CancellationToken {
        self.ctx.child_token()
    }

    /// Subscribe to all [DownloadEvent] notifications across the manager.
    ///
    /// The underlying broadcast channel has a bounded buffer (1024). Slow consumers may lag and
    /// miss events. Consider using [DownloadManager::events()] for a stream that skips lagged messages gracefully.
    pub fn subscribe(&self) -> broadcast::Receiver<DownloadEvent> {
        self.ctx.events.subscribe()
    }

    /// A fallible-safe stream of global [DownloadEvent] values.
    ///
    /// Internally wraps the broadcast receiver and filters out lagged/closed errors.
    pub fn events(&self) -> impl Stream<Item = DownloadEvent> + 'static {
        use tokio_stream::StreamExt as _;

        BroadcastStream::new(self.subscribe()).filter_map(|res| res.ok())
    }

    /// Gracefully stop the manager.
    ///
    /// - Cancels all in-flight work ([DownloadManager::cancel_all()]).
    /// - Prevents new tasks from being scheduled and waits for all worker tasks to finish.
    /// Call this before dropping the manager if you need deterministic teardown.
    pub async fn shutdown(&self) {
        self.cancel_all();
        self.tracker.close();
        self.tracker.wait().await;
    }
}

/// Builder for DownloadManager.
///
/// Requirements
/// - You must set a positive `max_concurrent` before [DownloadManagerBuilder::build()]; otherwise it will fails.
///
/// Notes
/// - [DownloadManagerBuilder::build()] spawns the internal scheduler onto the current Tokio runtime.
pub struct DownloadManagerBuilder {
    max_concurrent: Option<usize>,
}

impl DownloadManagerBuilder {
    /// Create a new builder with no defaults applied.
    ///
    /// You must call [DownloadManagerBuilder::max_concurrent()] before [DownloadManagerBuilder::build()].
    pub fn new() -> Self {
        Self {
            max_concurrent: None,
        }
    }

    /// Set the maximum number of concurrent downloads allowed.
    ///
    /// Must be greater than zero. This limit applies across the entire manager.
    pub fn max_concurrent(mut self, max: usize) -> Self {
        self.max_concurrent = Some(max);
        self
    }

    /// Build and start the [DownloadManager].
    ///
    /// Spawns the scheduler task and returns a ready-to-use manager.
    /// Fails if max_concurrent is not set or is zero.
    pub fn build(self) -> anyhow::Result<DownloadManager> {
        let max_concurrent = self.max_concurrent.filter(|&n| n > 0).ok_or_else(|| {
            anyhow::anyhow!("Max concurrent downloads must be set and greater than 0")
        })?;

        let (cmd_tx, cmd_rx) = mpsc::channel(1024);
        let ctx = Context::new(max_concurrent);
        let tracker = TaskTracker::new();
        let scheduler = Scheduler::new(ctx.clone(), tracker.clone(), cmd_rx);

        let manager = DownloadManager {
            scheduler_tx: cmd_tx,
            ctx: ctx.clone(),
            tracker: tracker.clone(),
        };

        tracker.spawn(async move { scheduler.run().await });

        Ok(manager)
    }
}
