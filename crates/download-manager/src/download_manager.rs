mod download;
mod error;
mod request;
mod worker;

pub use crate::{
    download::{Download, DownloadResult, Status},
    error::DownloadError,
    request::Request,
};
use crate::{request::RequestBuilder, worker::download_thread};
use reqwest::{Client, Url};
use std::path::Path;
use tokio::sync::mpsc;
use tokio_util::{sync::CancellationToken, task::TaskTracker};

pub struct DownloadManager {
    queue: mpsc::Sender<Request>,
    cancel_token: CancellationToken,
    tracker: TaskTracker,
}

impl Default for DownloadManager {
    fn default() -> Self {
        let (tx, rx) = mpsc::channel(100);
        let client = Client::new();
        let tracker = TaskTracker::new();
        let manager = Self {
            queue: tx,
            cancel_token: CancellationToken::new(),
            tracker: tracker.clone(),
        };

        tracker.spawn(dispatcher_thread(client, rx, tracker.clone()));
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
        self.cancel_token.cancel();
    }

    pub fn child_token(&self) -> CancellationToken {
        self.cancel_token.child_token()
    }
}

async fn dispatcher_thread(client: Client, mut rx: mpsc::Receiver<Request>, tracker: TaskTracker) {
    while let Some(request) = rx.recv().await {
        if request.is_cancelled() {
            continue;
        }

        tracker.spawn(download_thread(client.clone(), request));
    }
}
