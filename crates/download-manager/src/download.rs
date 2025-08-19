use crate::{DownloadError, DownloadEvent, DownloadID};
use std::path::PathBuf;
use tokio::sync::{broadcast, oneshot};
use tokio_util::sync::CancellationToken;

pub struct Download {
    id: DownloadID,
    events: broadcast::Receiver<DownloadEvent>,
    result: oneshot::Receiver<Result<DownloadResult, DownloadError>>,

    cancel_token: CancellationToken,
}

impl Download {
    pub fn new(
        id: DownloadID,
        events: broadcast::Receiver<DownloadEvent>,
        result: oneshot::Receiver<Result<DownloadResult, DownloadError>>,
        cancel_token: CancellationToken,
    ) -> Self {
        Download {
            id,
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
