use crate::{DownloadError, DownloadID};
use anyhow::anyhow;
use std::path::PathBuf;
use tokio::sync::{oneshot, watch};
use tokio_util::sync::CancellationToken;

pub struct Download {
    id: DownloadID,
    status: watch::Receiver<Status>,
    result: oneshot::Receiver<Result<DownloadResult, DownloadError>>,

    cancel_token: CancellationToken,
}

impl Download {
    pub fn new(
        id: DownloadID,
        status: watch::Receiver<Status>,
        result: oneshot::Receiver<Result<DownloadResult, DownloadError>>,
        cancel_token: CancellationToken,
    ) -> Self {
        Download {
            id,
            status,
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

    pub fn status(&self) -> Status {
        *self.status.borrow()
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    Queued,
    Running,
    Retrying(u32),
    Completed,
    Cancelled,
    Failed,
}

#[derive(Debug)]
pub struct DownloadResult {
    pub path: PathBuf,
}
