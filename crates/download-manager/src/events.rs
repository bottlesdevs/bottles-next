use crate::DownloadID;
use reqwest::Url;
use std::path::PathBuf;
use std::time::Instant;

#[derive(Debug, Clone)]
pub enum DownloadEvent {
    Queued {
        id: DownloadID,
        url: Url,
        destination: PathBuf,
    },
    Started {
        id: DownloadID,
        url: Url,
        destination: PathBuf,
        total_bytes: Option<u64>,
    },
    Retrying {
        id: DownloadID,
        attempt: u32,
        next_delay_ms: u64,
    },
    Completed {
        id: DownloadID,
        path: PathBuf,
        bytes_downloaded: u64,
    },
    Failed {
        id: DownloadID,
        error: String,
    },
    Cancelled {
        id: DownloadID,
    },
}

#[derive(Debug, Clone, Copy)]
pub struct Progress {
    pub bytes_downloaded: u64,
    pub total_bytes: Option<u64>,
    pub instantaneous_bps: f64,
    pub avg_bps: f64,

    started_at: Instant,
    updated_at: Instant,
}

impl Progress {
    pub fn new() -> Self {
        let now = Instant::now();
        Progress {
            bytes_downloaded: 0,
            total_bytes: None,
            started_at: now,
            updated_at: now,
            instantaneous_bps: 0.0,
            avg_bps: 0.0,
        }
    }
}
