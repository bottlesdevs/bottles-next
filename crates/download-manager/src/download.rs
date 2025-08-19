use crate::{DownloadError, DownloadEvent, DownloadID, Progress};
use futures_core::Stream;
use std::path::PathBuf;
use tokio::sync::{broadcast, oneshot, watch};
use tokio_stream::wrappers::{BroadcastStream, WatchStream};
use tokio_util::sync::CancellationToken;

pub struct Download {
    id: DownloadID,
    progress: watch::Receiver<Progress>,
    events: broadcast::Receiver<DownloadEvent>,
    result: oneshot::Receiver<Result<DownloadResult, DownloadError>>,

    cancel_token: CancellationToken,
}

impl Download {
    pub fn new(
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

    pub fn id(&self) -> DownloadID {
        self.id
    }

    pub fn cancel(&self) {
        self.cancel_token.cancel();
    }

    pub fn progress_raw(&self) -> watch::Receiver<Progress> {
        self.progress.clone()
    }

    pub fn progress(&self) -> impl Stream<Item = Progress> + 'static {
        WatchStream::new(self.progress_raw())
    }

    pub fn events(&self) -> impl Stream<Item = DownloadEvent> + 'static {
        use tokio_stream::StreamExt as _;

        let download_id = self.id;
        BroadcastStream::new(self.events.resubscribe())
            .filter_map(|res| res.ok())
            .filter(move |event| {
                let matches = match event {
                    DownloadEvent::Queued { id, .. }
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
