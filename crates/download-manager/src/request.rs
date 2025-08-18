use crate::{Download, DownloadError, DownloadManager, DownloadResult, Status};
use anyhow::anyhow;
use reqwest::Url;
use std::path::{Path, PathBuf};
use tokio::sync::{oneshot, watch};

pub struct Request {
    url: Url,
    destination: PathBuf,

    status: watch::Sender<Status>,
    result: oneshot::Sender<Result<DownloadResult, DownloadError>>,
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

    fn mark_status(&self, status: Status) -> Result<(), DownloadError> {
        self.status
            .send(status)
            .map_err(|err| DownloadError::Channel(anyhow!(err)))
    }

    fn send_result(
        self,
        result: Result<DownloadResult, DownloadError>,
    ) -> Result<(), DownloadError> {
        self.result
            .send(result)
            .map_err(|_| DownloadError::Channel(anyhow!("Failed to send download result")))
    }

    pub fn mark_running(&self) -> Result<(), DownloadError> {
        self.mark_status(Status::Running)
    }

    pub fn mark_failed(self, error: DownloadError) -> Result<(), DownloadError> {
        self.mark_status(Status::Failed)?;
        self.send_result(Err(error))
    }

    pub fn mark_completed(self, result: DownloadResult) -> Result<(), DownloadError> {
        self.mark_status(Status::Completed)?;
        self.send_result(Ok(result))
    }

    pub fn mark_retrying(&self, retry_count: usize) -> Result<(), DownloadError> {
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

    pub fn start(self) -> Result<Download, DownloadError> {
        let url = self.url.ok_or_else(|| anyhow::anyhow!("URL must be set"))?;
        let destination = self
            .destination
            .ok_or_else(|| anyhow::anyhow!("Destination must be set"))?;

        let (status_tx, status_rx) = watch::channel(Status::Queued);
        let (result_tx, result_rx) = oneshot::channel();

        let request = Request {
            url,
            destination,
            status: status_tx,
            result: result_tx,
        };

        self.manager.queue_request(request)?;

        Ok(Download::new(status_rx, result_rx))
    }
}
