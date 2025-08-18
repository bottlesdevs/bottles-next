use std::path::PathBuf;

use tokio::sync::watch;

pub struct Download {
    status: watch::Receiver<Status>,
}

impl Download {
    pub fn new(status: watch::Receiver<Status>) -> Self {
        Download { status }
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
