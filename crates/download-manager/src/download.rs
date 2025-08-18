use std::path::PathBuf;

pub struct Download {}

impl Download {
    pub fn new() -> Self {
        Download {}
    }
}

pub struct DownloadResult {
    pub path: PathBuf,
}
