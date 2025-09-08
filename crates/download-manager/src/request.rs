use crate::{
    Download, DownloadID, DownloadManager, Event, Progress, error::DownloadError, events::EventBus,
    scheduler::SchedulerCmd,
};
use derive_builder::Builder;
use reqwest::{
    Url,
    header::{HeaderMap, IntoHeaderName},
};
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};
use tokio::sync::{mpsc, oneshot, watch};
use tokio_util::sync::CancellationToken;

/// Immutable description of a single download request.
///
/// Built by [RequestBuilder] and executed by the scheduler. Holds destination,
/// headers, retry policy, and user callbacks. Most users should prefer creating
/// requests via [DownloadManager::download_builder()].
#[derive(Clone, Builder)]
#[builder(pattern = "owned")]
#[builder(build_fn(skip))]
pub struct Request {
    #[builder(field(ty = "DownloadID"))]
    id: DownloadID,
    url: Url,
    #[builder(setter(into))]
    destination: PathBuf,
    #[builder(field(ty = "DownloadConfigBuilder"))]
    config: DownloadConfig,

    progress: watch::Sender<Progress>,
    events: EventBus,

    #[builder(
        field(ty = "Option<Arc<dyn Fn(Progress) + Send + Sync>>"),
        setter(strip_option)
    )]
    pub(crate) on_progress: Option<Arc<dyn Fn(Progress) + Send + Sync>>,
    #[builder(
        field(ty = "Option<Arc<dyn Fn(Event) + Send + Sync>>"),
        setter(strip_option)
    )]
    pub(crate) on_event: Option<Arc<dyn Fn(Event) + Send + Sync>>,

    pub cancel_token: CancellationToken,

    #[builder(field(ty = "Option<mpsc::Sender<SchedulerCmd>>"), setter(custom))]
    _sched_tx: (),
}

/// Per-request configuration for retries, overwrite behavior, and headers.
///
/// Behavior
/// - `retries`: maximum retry attempts for retryable network errors (default 3).
/// - `overwrite`: when false, existing destination paths cause FileExists errors.
/// - `headers`: extra HTTP headers (e.g., User-Agent).
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
    /// Add an HTTP header to the request configuration.
    ///
    /// The value must be a valid HTTP header value; invalid values will panic during parsing.
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
    /// Maximum retry attempts for retryable network errors.
    pub fn retries(&self) -> u32 {
        self.retries
    }

    /// Whether an existing destination file may be overwritten.
    pub fn overwrite(&self) -> bool {
        self.overwrite
    }

    /// Additional headers applied to both the HEAD probe and the GET request.
    pub fn headers(&self) -> &HeaderMap {
        &self.headers
    }
}

impl Request {
    pub fn builder(manager: &DownloadManager) -> RequestBuilder {
        RequestBuilder {
            id: manager.ctx.next_id(),
            url: None,
            destination: None,
            config: DownloadConfigBuilder::default(),
            progress: None,
            on_progress: None,
            on_event: None,
            events: Some(manager.ctx.events.clone()),
            cancel_token: Some(manager.child_token()),
            _sched_tx: Some(manager.scheduler_tx.clone()),
        }
    }

    pub fn id(&self) -> DownloadID {
        self.id
    }

    pub fn url(&self) -> &Url {
        &self.url
    }

    pub fn destination(&self) -> &Path {
        self.destination.as_path()
    }

    pub fn config(&self) -> &DownloadConfig {
        &self.config
    }

    pub fn emit(&self, event: Event) {
        self.events.send(event.clone());
        self.on_event.as_ref().map(|cb| cb(event));
    }

    pub fn update_progress(&self, progress: Progress) {
        // TODO: Log the error
        let _ = self.progress.send(progress);
        self.on_progress.as_ref().map(|cb| cb(progress));
    }
}

impl RequestBuilder {
    /// Set the maximum retry attempts for retryable network errors.
    pub fn retries(mut self, retries: u32) -> Self {
        self.config = self.config.retries(retries);
        self
    }

    /// Convenience for setting the User-Agent header.
    pub fn user_agent(self, user_agent: impl AsRef<str>) -> Self {
        self.header(reqwest::header::USER_AGENT, user_agent)
    }

    /// Control whether an existing destination file may be overwritten.
    pub fn overwrite(mut self, overwrite: bool) -> Self {
        self.config = self.config.overwrite(overwrite);
        self
    }

    /// Add an HTTP header (e.g., Authorization, Range).
    ///
    /// Note: value must be a valid header value; invalid values cause a panic during build.
    pub fn header(mut self, header: impl IntoHeaderName, value: impl AsRef<str>) -> Self {
        self.config = self.config.header(header, value);
        self
    }

    pub fn start(self) -> anyhow::Result<Download> {
        let cancel_token = self.cancel_token.expect("Cancel token must be set");
        if cancel_token.is_cancelled() {
            return Err(DownloadError::ManagerShutdown.into());
        }

        let url = self.url.ok_or_else(|| anyhow::anyhow!("URL must be set"))?;
        let destination = self
            .destination
            .ok_or_else(|| anyhow::anyhow!("Destination must be set"))?;
        let config = self.config.build()?;

        let (result_tx, result_rx) = oneshot::channel();
        let (progress_tx, progress_rx) = watch::channel(Progress::new(None));
        let events = self.events.unwrap();
        let event_rx = events.subscribe();
        let id = self.id;

        let request = Request {
            id,
            url: url.clone(),
            destination: destination.clone(),
            config,

            on_progress: self.on_progress,
            on_event: self.on_event,

            events,
            progress: progress_tx,
            cancel_token: cancel_token.clone(),

            _sched_tx: (),
        };

        let sched_tx = self._sched_tx.expect("sched_tx must be set");
        sched_tx.try_send(SchedulerCmd::Enqueue { request, result_tx })?;

        Ok(Download::new(
            id,
            progress_rx,
            event_rx,
            result_rx,
            cancel_token,
        ))
    }
}
