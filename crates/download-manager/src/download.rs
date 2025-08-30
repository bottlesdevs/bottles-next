use crate::{DownloadError, DownloadEvent, DownloadID, Progress};
use futures_core::Stream;
use std::path::PathBuf;
use tokio::sync::{broadcast, oneshot, watch};
use tokio_stream::wrappers::{BroadcastStream, WatchStream};
use tokio_util::sync::CancellationToken;

/// Handle for a single download scheduled by DownloadManager.
///
/// Behavior:
/// - Implements Future; awaiting resolves to DownloadResult or DownloadError.
/// - Exposes per-download streams via [Download::progress()] and [Download::events()].
/// - Cancellation is cooperative via [Download::cancel()]; the worker aborts the HTTP request and removes any partial file.
pub struct Download {
    id: DownloadID,
    progress: watch::Receiver<Progress>,
    events: broadcast::Receiver<DownloadEvent>,
    result: oneshot::Receiver<Result<DownloadResult, DownloadError>>,

    cancel_token: CancellationToken,
}

impl Download {
    pub(crate) fn new(
        id: DownloadID,
        progress: watch::Receiver<Progress>,
        events: broadcast::Receiver<DownloadEvent>,
        result: oneshot::Receiver<Result<DownloadResult, DownloadError>>,
        cancel_token: CancellationToken,
    ) -> Self {
        Download {
            id,
            progress,
            events,
            result,
            cancel_token,
        }
    }

    /// Unique identifier for this download, matching [DownloadEvent] IDs.
    pub fn id(&self) -> DownloadID {
        self.id
    }

    /// Request cooperative cancellation of this download.
    ///
    /// The scheduler/worker aborts the in-flight HTTP request and deletes any partially
    /// written file. Cancellation is best-effort and may race with completion.
    pub fn cancel(&self) {
        self.cancel_token.cancel();
    }

    pub fn progress_raw(&self) -> watch::Receiver<Progress> {
        self.progress.clone()
    }

    /// Stream of sampled Progress updates for this download.
    ///
    /// Backed by a watch channel: consumers receive the latest state immediately,
    /// and updates are coalesced according to sampling thresholds.
    pub fn progress(&self) -> impl Stream<Item = Progress> + 'static {
        WatchStream::new(self.progress_raw())
    }

    /// Stream of [DownloadEvent] values scoped to this download only.
    ///
    /// Backed by a broadcast channel; lagged consumers may drop messages.
    /// This stream filters events to those whose id matches this handle.
    pub fn events(&self) -> impl Stream<Item = DownloadEvent> + 'static {
        use tokio_stream::StreamExt as _;

        let download_id = self.id;
        BroadcastStream::new(self.events.resubscribe())
            .filter_map(|res| res.ok())
            .filter(move |event| {
                let matches = match event {
                    DownloadEvent::Queued { id, .. }
                    | DownloadEvent::Probed { id, .. }
                    | DownloadEvent::Started { id, .. }
                    | DownloadEvent::Retrying { id, .. }
                    | DownloadEvent::Completed { id, .. }
                    | DownloadEvent::Failed { id, .. }
                    | DownloadEvent::Cancelled { id, .. } => *id == download_id,
                };

                matches
            })
    }
}

impl std::future::Future for Download {
    type Output = Result<DownloadResult, DownloadError>;

    fn poll(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        use std::pin::Pin;
        use std::task::Poll;

        match Pin::new(&mut self.result).poll(cx) {
            Poll::Ready(Ok(result)) => Poll::Ready(result),
            Poll::Ready(Err(_)) => Poll::Ready(Err(DownloadError::ManagerShutdown)),
            Poll::Pending => Poll::Pending,
        }
    }
}

#[derive(Debug)]
pub struct DownloadResult {
    pub path: PathBuf,
    pub bytes_downloaded: u64,
}

#[derive(Debug, Clone)]
/// Remote metadata obtained via a best-effort `HEAD` probe prior to downloading.
/// Availability depends on server support; fields are None when not provided.
pub struct RemoteInfo {
    pub content_length: Option<u64>,
    pub accept_ranges: Option<String>,
    pub etag: Option<String>,
    pub last_modified: Option<String>,
    pub content_type: Option<String>,
}
