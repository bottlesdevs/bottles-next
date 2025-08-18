mod download;
mod error;
mod request;
mod worker;

use crate::{
    download::{Download, DownloadResult},
    error::DownloadError,
    request::{Request, RequestBuilder},
    worker::download_thread,
};
use reqwest::{Client, Url};
use std::path::Path;
use tokio::sync::mpsc;
use tokio_util::task::TaskTracker;

pub struct DownloadManager {
    queue: mpsc::Sender<Request>,
    tracker: TaskTracker,
}

impl Default for DownloadManager {
    fn default() -> Self {
        let (tx, rx) = mpsc::channel(100);
        let client = Client::new();
        let tracker = TaskTracker::new();
        let manager = Self {
            queue: tx,
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
}

async fn dispatcher_thread(client: Client, mut rx: mpsc::Receiver<Request>, tracker: TaskTracker) {
    while let Some(request) = rx.recv().await {
        tracker.spawn(download_thread(client.clone(), request));
    }
}
