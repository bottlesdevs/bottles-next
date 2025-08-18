use crate::{Download, DownloadError, DownloadManager, DownloadResult, Status};
use anyhow::{Result, anyhow};
use derive_builder::Builder;
use reqwest::Url;
use std::path::{Path, PathBuf};
use tokio::sync::{oneshot, watch};
use tokio_util::sync::CancellationToken;

pub struct Request {
    url: Url,
    destination: PathBuf,
    config: DownloadConfig,

    status: watch::Sender<Status>,
    result: oneshot::Sender<Result<DownloadResult, DownloadError>>,

    pub cancel_token: CancellationToken,
}

#[derive(Debug, Builder)]
#[builder(pattern = "owned")]
pub struct DownloadConfig {
    #[builder(default = "3")]
    retries: u32,
    #[builder(default, setter(strip_option))]
    user_agent: Option<String>,
    #[builder(default = "false")]
    overwrite: bool,
}

impl Default for DownloadConfig {
    fn default() -> Self {
        DownloadConfig {
            retries: 3,
            user_agent: None,
            overwrite: false,
        }
    }
}

impl DownloadConfig {
    pub fn retries(&self) -> u32 {
        self.retries
    }

    pub fn user_agent(&self) -> Option<&str> {
        self.user_agent.as_deref()
    }

    pub fn overwrite(&self) -> bool {
        self.overwrite
    }
}

impl Request {
    pub fn builder<'a>(manager: &'a DownloadManager) -> RequestBuilder<'a> {
        RequestBuilder {
            url: None,
            destination: None,
            config: DownloadConfigBuilder::default(),
            manager,
        }
    }

    /* TODO:
     * Add callbacks like `on_update`, `on_progress`, `on_complete`, etc.
     */
    pub fn url(&self) -> &Url {
        &self.url
    }

    pub fn destination(&self) -> &Path {
        self.destination.as_path()
    }

    pub fn config(&self) -> &DownloadConfig {
        &self.config
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancel_token.is_cancelled()
    }

    fn mark_status(&self, status: Status) -> Result<()> {
        self.status.send(status).map_err(|err| anyhow!(err))
    }

    fn send_result(self, result: Result<DownloadResult, DownloadError>) -> Result<()> {
        self.result
            .send(result)
            .map_err(|_| anyhow!("Failed to send download result"))
    }

    pub fn mark_running(&self) -> Result<()> {
        self.mark_status(Status::Running)
    }

    pub fn mark_failed(self, error: DownloadError) -> Result<()> {
        match error {
            DownloadError::Cancelled => self.mark_status(Status::Cancelled)?,
            _ => self.mark_status(Status::Failed)?,
        }
        self.send_result(Err(error))
    }

    pub fn mark_completed(self, result: DownloadResult) -> Result<()> {
        self.mark_status(Status::Completed)?;
        self.send_result(Ok(result))
    }

    pub fn mark_retrying(&self, retry_count: u32) -> Result<()> {
        self.mark_status(Status::Retrying(retry_count))
    }

    pub fn mark_cancelled(self) -> Result<()> {
        self.mark_status(Status::Cancelled)?;
        self.send_result(Err(DownloadError::Cancelled))
    }
}

pub struct RequestBuilder<'a> {
    url: Option<Url>,
    destination: Option<PathBuf>,
    config: DownloadConfigBuilder,

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

    pub fn retries(mut self, retries: u32) -> Self {
        self.config = self.config.retries(retries);
        self
    }

    pub fn user_agent(mut self, user_agent: impl AsRef<str>) -> Self {
        self.config = self.config.user_agent(user_agent.as_ref().into());
        self
    }

    pub fn overwrite(mut self, overwrite: bool) -> Self {
        self.config = self.config.overwrite(overwrite);
        self
    }

    pub fn start(self) -> Result<Download> {
        let url = self.url.ok_or_else(|| anyhow::anyhow!("URL must be set"))?;
        let destination = self
            .destination
            .ok_or_else(|| anyhow::anyhow!("Destination must be set"))?;
        let config = self.config.build()?;

        let (status_tx, status_rx) = watch::channel(Status::Queued);
        let (result_tx, result_rx) = oneshot::channel();
        let cancel_token = self.manager.child_token();

        let request = Request {
            url,
            destination,
            config,
            status: status_tx,
            result: result_tx,
            cancel_token: cancel_token.clone(),
        };

        self.manager.queue_request(request)?;

        Ok(Download::new(status_rx, result_rx, cancel_token))
    }
}
