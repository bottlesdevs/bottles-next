use crate::{
    Download, DownloadError, DownloadEvent, DownloadID, DownloadManager, DownloadResult, Progress,
};
use derive_builder::Builder;
use reqwest::Url;
use std::{
    path::{Path, PathBuf},
    time::Duration,
};
use tokio::sync::{broadcast, oneshot, watch};
use tokio_util::sync::CancellationToken;

pub struct Request {
    id: DownloadID,
    url: Url,
    destination: PathBuf,
    config: DownloadConfig,

    progress: watch::Sender<Progress>,
    events: broadcast::Sender<DownloadEvent>,
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

    pub fn id(&self) -> DownloadID {
        self.id
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

    fn emit(&self, event: DownloadEvent) {
        // TODO: Log the error
        let _ = self.events.send(event);
    }

    fn send_result(self, result: Result<DownloadResult, DownloadError>) {
        // TODO: Log the error
        let _ = self.result.send(result);
    }

    pub fn update_progress(&self, progress: Progress) {
        // TODO: Log the error
        let _ = self.progress.send(progress);
    }

    pub fn start(&self) {
        self.emit(DownloadEvent::Started {
            id: self.id(),
            url: self.url().clone(),
            destination: self.destination.clone(),
            total_bytes: None,
        });
    }

    pub fn fail(self, error: DownloadError) {
        self.send_result(Err(error));
    }

    pub fn finish(self, result: DownloadResult) {
        self.emit(DownloadEvent::Completed {
            id: self.id(),
            path: result.path.clone(),
            bytes_downloaded: result.bytes_downloaded,
        });
        self.send_result(Ok(result))
    }

    pub fn retry(&self, attempt: u32, delay: Duration) {
        self.emit(DownloadEvent::Retrying {
            id: self.id(),
            attempt,
            next_delay_ms: delay.as_millis() as u64,
        });
    }

    pub fn cancel(self) {
        self.emit(DownloadEvent::Cancelled { id: self.id() });
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

    pub fn start(self) -> anyhow::Result<Download> {
        let url = self.url.ok_or_else(|| anyhow::anyhow!("URL must be set"))?;
        let destination = self
            .destination
            .ok_or_else(|| anyhow::anyhow!("Destination must be set"))?;
        let config = self.config.build()?;

        let (progress_tx, progress_rx) = watch::channel(Progress::new(None));
        let (result_tx, result_rx) = oneshot::channel();
        let cancel_token = self.manager.child_token();

        let event_tx = self.manager.ctx.events.clone();
        let event_rx = event_tx.subscribe();
        let id = self.manager.ctx.next_id();
        let request = Request {
            id,
            url: url.clone(),
            destination: destination.clone(),
            config,
            progress: progress_tx,
            events: event_tx.clone(),
            result: result_tx,
            cancel_token: cancel_token.clone(),
        };

        self.manager.queue_request(request)?;
        event_tx.send(DownloadEvent::Queued {
            id,
            url,
            destination,
        })?;

        Ok(Download::new(
            id,
            progress_rx,
            event_rx,
            result_rx,
            cancel_token,
        ))
    }
}
