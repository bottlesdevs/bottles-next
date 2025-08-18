use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum DownloadError {
    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Download was cancelled")]
    Cancelled,
    #[error("Retry limit exceeded: {last_error}")]
    RetriesExhausted { last_error: anyhow::Error },
    #[error("Download queue is full")]
    QueueFull,
    #[error("Download manager has been shut down")]
    ManagerShutdown,
    #[error("File already exists: {path}")]
    FileExists { path: PathBuf },
    #[error("Invalid URL: {0}")]
    InvalidUrl(String),
}
