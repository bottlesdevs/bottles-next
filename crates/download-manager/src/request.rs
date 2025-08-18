use crate::{Download, DownloadManager, download::Status};
use anyhow::{Result, anyhow};
use reqwest::Url;
use std::path::{Path, PathBuf};
use tokio::sync::watch;

pub struct Request {
    url: Url,
    destination: PathBuf,

    status: watch::Sender<Status>,
}

impl Request {
    pub fn builder<'a>(manager: &'a DownloadManager) -> RequestBuilder<'a> {
        RequestBuilder {
            url: None,
            destination: None,
            manager,
        }
    }

    pub fn url(&self) -> &Url {
        &self.url
    }

    pub fn destination(&self) -> &Path {
        self.destination.as_path()
    }

    pub fn mark_status(&self, status: Status) -> Result<()> {
        self.status.send(status).map_err(|err| anyhow!(err))
    }
    pub fn mark_running(&self) -> Result<()> {
        self.mark_status(Status::Running)
    }

    pub fn mark_failed(&self) -> Result<()> {
        self.mark_status(Status::Failed)
    }

    pub fn mark_completed(&self) -> Result<()> {
        self.mark_status(Status::Completed)
    }

    pub fn mark_retrying(&self, retry_count: usize) -> Result<()> {
        self.mark_status(Status::Retrying(retry_count))
    }
}

pub struct RequestBuilder<'a> {
    url: Option<Url>,
    destination: Option<PathBuf>,

    manager: &'a DownloadManager,
}

impl RequestBuilder<'_> {
    pub fn url(mut self, url: Url) -> Self {
        self.url = Some(url);
        self
    }

    pub fn destination(mut self, destination: impl AsRef<Path>) -> Self {
        self.destination = Some(destination.as_ref().to_path_buf());
        self
    }

    pub fn start(self) -> Result<Download> {
        let url = self.url.ok_or_else(|| anyhow::anyhow!("URL must be set"))?;
        let destination = self
            .destination
            .ok_or_else(|| anyhow::anyhow!("Destination must be set"))?;

        let (status_tx, status_rx) = watch::channel(Status::Queued);
        let request = Request {
            url,
            destination,
            status: status_tx,
        };

        self.manager.queue_request(request)?;

        Ok(Download::new(status_rx))
    }
}
