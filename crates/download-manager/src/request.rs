use crate::{
    Download, DownloadEvent, DownloadID, DownloadManager, Progress, scheduler::SchedulerCmd,
};
use derive_builder::Builder;
use reqwest::{
    Url,
    header::{HeaderMap, IntoHeaderName},
};
use std::path::{Path, PathBuf};
use tokio::sync::{broadcast, oneshot, watch};
use tokio_util::sync::CancellationToken;

#[derive(Debug, Clone)]
pub struct Request {
    id: DownloadID,
    url: Url,
    destination: PathBuf,
    config: DownloadConfig,

    progress: watch::Sender<Progress>,
    events: broadcast::Sender<DownloadEvent>,

    pub cancel_token: CancellationToken,
}

#[derive(Debug, Builder, Clone)]
#[builder(pattern = "owned")]
pub struct DownloadConfig {
    #[builder(default = "3")]
    retries: u32,
    #[builder(default = "false")]
    overwrite: bool,
    #[builder(field(ty = "HeaderMap"), setter(custom))]
    headers: HeaderMap,
}

impl DownloadConfigBuilder {
    pub fn header(mut self, header: impl IntoHeaderName, value: impl AsRef<str>) -> Self {
        self.headers.insert(header, value.as_ref().parse().unwrap());
        self
    }
}

impl Default for DownloadConfig {
    fn default() -> Self {
        DownloadConfig {
            retries: 3,
            overwrite: false,
            headers: HeaderMap::new(),
        }
    }
}

impl DownloadConfig {
    pub fn retries(&self) -> u32 {
        self.retries
    }

    pub fn overwrite(&self) -> bool {
        self.overwrite
    }

    pub fn headers(&self) -> &HeaderMap {
        &self.headers
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

    pub fn emit(&self, event: DownloadEvent) {
        // TODO: Log the error
        let _ = self.events.send(event);
    }

    pub fn update_progress(&self, progress: Progress) {
        // TODO: Log the error
        let _ = self.progress.send(progress);
    }

    pub fn start(&self) {
        self.emit(DownloadEvent::Started {
            id: self.id(),
            url: self.url().clone(),
            destination: self.destination().to_path_buf(),
            total_bytes: None,
        });
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

    pub fn user_agent(self, user_agent: impl AsRef<str>) -> Self {
        self.header(reqwest::header::USER_AGENT, user_agent)
    }

    pub fn overwrite(mut self, overwrite: bool) -> Self {
        self.config = self.config.overwrite(overwrite);
        self
    }

    pub fn header(mut self, header: impl IntoHeaderName, value: impl AsRef<str>) -> Self {
        self.config = self.config.header(header, value);
        self
    }

    pub fn start(self) -> anyhow::Result<Download> {
        let url = self.url.ok_or_else(|| anyhow::anyhow!("URL must be set"))?;
        let destination = self
            .destination
            .ok_or_else(|| anyhow::anyhow!("Destination must be set"))?;
        let config = self.config.build()?;

        let id = self.manager.ctx.next_id();
        let (result_tx, result_rx) = oneshot::channel();
        let (progress_tx, progress_rx) = watch::channel(Progress::new(None));
        let cancel_token = self.manager.child_token();
        let event_tx = self.manager.ctx.events.clone();
        let event_rx = event_tx.subscribe();

        let request = Request {
            id,
            url: url.clone(),
            destination: destination.clone(),
            config,

            events: event_tx,
            progress: progress_tx,
            cancel_token: cancel_token.clone(),
        };

        self.manager
            .scheduler_tx
            .try_send(SchedulerCmd::Enqueue { request, result_tx });

        Ok(Download::new(
            id,
            progress_rx,
            event_rx,
            result_rx,
            cancel_token,
        ))
    }
}
