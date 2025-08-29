mod context;
mod download;
mod error;
mod events;
mod request;
mod scheduler;

use crate::{
    context::Context,
    request::RequestBuilder,
    scheduler::{Scheduler, SchedulerCmd},
};
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
    scheduler_tx: mpsc::Sender<SchedulerCmd>,
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

    pub fn cancel(&self, id: DownloadID) -> anyhow::Result<()> {
        self.scheduler_tx
            .try_send(SchedulerCmd::Cancel { id })
            .map_err(|e| anyhow::anyhow!("Failed to send cancel command: {}", e))
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

    pub async fn shutdown(&self) {
        self.cancel_all();
        self.tracker.close();
        self.tracker.wait().await;
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
        let scheduler = Scheduler::new(ctx.clone(), tracker.clone(), rx);

        let manager = DownloadManager {
            scheduler_tx: tx,
            ctx: ctx.clone(),
            tracker: tracker.clone(),
        };

        tracker.spawn(async move { scheduler.run().await });

        Ok(manager)
    }
}
