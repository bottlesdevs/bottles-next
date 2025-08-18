use crate::DownloadError;
use std::path::PathBuf;
use tokio::sync::{oneshot, watch};

pub struct Download {
    status: watch::Receiver<Status>,
    result: oneshot::Receiver<Result<DownloadResult, DownloadError>>,
}

impl Download {
    pub fn new(
        status: watch::Receiver<Status>,
        result: oneshot::Receiver<Result<DownloadResult, DownloadError>>,
    ) -> Self {
        Download { status, result }
    }
}

pub enum Status {
    Queued,
    Running,
    Retrying(usize),
    Completed,
    Failed,
}

pub struct DownloadResult {
    pub path: PathBuf,
}
