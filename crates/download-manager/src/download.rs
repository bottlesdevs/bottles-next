use crate::DownloadError;
use std::path::PathBuf;
use tokio::sync::{oneshot, watch};
use tokio_util::sync::CancellationToken;

pub struct Download {
    status: watch::Receiver<Status>,
    result: oneshot::Receiver<Result<DownloadResult, DownloadError>>,

    cancel_token: CancellationToken,
}

impl Download {
    pub fn new(
        status: watch::Receiver<Status>,
        result: oneshot::Receiver<Result<DownloadResult, DownloadError>>,
        cancel_token: CancellationToken,
    ) -> Self {
        Download {
            status,
            result,
            cancel_token,
        }
    }

    pub fn cancel(&self) {
        self.cancel_token.cancel();
    }
}

pub enum Status {
    Queued,
    Running,
    Retrying(u32),
    Completed,
    Cancelled,
    Failed,
}

pub struct DownloadResult {
    pub path: PathBuf,
}
