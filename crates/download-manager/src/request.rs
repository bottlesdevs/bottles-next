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
use tokio::sync::{oneshot, watch};
use tokio_util::sync::CancellationToken;

/// Immutable description of a single download request.
///
/// Built by [RequestBuilder] and executed by the scheduler. Holds destination,
/// headers, retry policy, and user callbacks. Most users should prefer creating
/// requests via [DownloadManager::download_builder()].
#[derive(Clone)]
pub struct Request {
    id: DownloadID,
    url: Url,
    destination: PathBuf,
    config: DownloadConfig,

    progress: watch::Sender<Progress>,
    events: EventBus,

    pub(crate) on_progress: Option<Arc<Box<dyn Fn(Progress) + Send + Sync>>>,
    pub(crate) on_event: Option<Arc<Box<dyn Fn(Event) + Send + Sync>>>,

    pub cancel_token: CancellationToken,
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
    pub fn builder<'a>(manager: &'a DownloadManager) -> RequestBuilder<'a> {
        RequestBuilder {
            url: None,
            destination: None,
            config: DownloadConfigBuilder::default(),
            on_progress: None,
            on_event: None,
            manager,
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

/// Builder for Request. Configure URL, destination, headers, retries, overwrite, and callbacks.
///
/// After configuration, call [RequestBuilder::start()] to enqueue the download and obtain a [Download] handle.
pub struct RequestBuilder<'a> {
    url: Option<Url>,
    destination: Option<PathBuf>,
    config: DownloadConfigBuilder,

    on_progress: Option<Arc<Box<dyn Fn(Progress) + Send + Sync>>>,
    on_event: Option<Arc<Box<dyn Fn(Event) + Send + Sync>>>,

    manager: &'a DownloadManager,
}

impl RequestBuilder<'_> {
    /// Set the source URL for the download.
    pub fn url(mut self, url: Url) -> Self {
        self.url = Some(url);
        self
    }

    /// Set the destination path. Parent directories are created when starting.
    ///
    /// If overwrite is false and the file exists, start() will error.
    pub fn destination(mut self, destination: impl AsRef<Path>) -> Self {
        self.destination = Some(destination.as_ref().to_path_buf());
        self
    }

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

    /// Register a callback invoked when sampled Progress updates are produced.
    ///
    /// Called on async worker context; keep the callback lightweight.
    pub fn on_progress<F>(mut self, callback: F) -> Self
    where
        F: Fn(Progress) + Send + Sync + 'static,
    {
        self.on_progress = Some(Arc::new(Box::new(callback)));
        self
    }

    /// Register a callback for per-download DownloadEvent notifications.
    ///
    /// Called for events emitted for this request only.
    pub fn on_event<F>(mut self, callback: F) -> Self
    where
        F: Fn(Event) + Send + Sync + 'static,
    {
        self.on_event = Some(Arc::new(Box::new(callback)));
        self
    }

    /// Finalize the request, enqueue it, and return a Download handle.
    ///
    /// Errors if url or destination are not set, or if the internal channel is unavailable.
    /// The returned handle implements [Future].
    pub fn start(self) -> anyhow::Result<Download> {
        if self.manager.shutdown_token.is_cancelled() {
            return Err(DownloadError::ManagerShutdown.into());
        }

        let url = self.url.ok_or_else(|| anyhow::anyhow!("URL must be set"))?;
        let destination = self
            .destination
            .ok_or_else(|| anyhow::anyhow!("Destination must be set"))?;
        let config = self.config.build()?;

        let id = self.manager.ctx.next_id();
        let (result_tx, result_rx) = oneshot::channel();
        let (progress_tx, progress_rx) = watch::channel(Progress::new(None));
        let cancel_token = self.manager.child_token();
        let event_bus = self.manager.ctx.events.clone();
        let event_rx = event_bus.subscribe();

        let on_progress = self.on_progress;
        let on_event = self.on_event;

        let request = Request {
            id,
            url: url.clone(),
            destination: destination.clone(),
            config,

            on_progress,
            on_event,

            events: event_bus,
            progress: progress_tx,
            cancel_token: cancel_token.clone(),
        };

        self.manager
            .scheduler_tx
            .try_send(SchedulerCmd::Enqueue { request, result_tx })?;

        Ok(Download::new(
            id,
            progress_rx,
            event_rx,
            result_rx,
            cancel_token,
        ))
    }
}
