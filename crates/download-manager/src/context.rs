use reqwest::Client;
use std::sync::{
    Arc,
    atomic::{AtomicU64, AtomicUsize, Ordering},
};
use tokio::sync::{Semaphore, broadcast};
use tokio_util::sync::CancellationToken;

use crate::DownloadEvent;

/// Unique identifier for a download; monotonically increasing u64.
pub type DownloadID = u64;

/// Shared runtime context for coordinating downloads. Internal to the crate.
/// Holds the concurrency semaphore, root cancellation token, HTTP client,
/// atomic counters, and the global [DownloadEvent] broadcast sender.
/// Cloned and shared across scheduler and workers.
#[derive(Debug)]
pub(crate) struct Context {
    /// Semaphore limiting concurrent active downloads.
    pub semaphore: Arc<Semaphore>,
    /// Root cancellation token; children inherit via [Context::child_token()].
    pub cancel_root: CancellationToken,
    /// Shared reqwest client reused across attempts.
    pub client: Client,

    // Counters
    /// Monotonic counter for generating DownloadID values.
    pub id_counter: AtomicU64,
    /// Number of currently active (running) downloads.
    pub active: AtomicUsize,
    /// Configured maximum concurrency. Not automatically updated if semaphore changes.
    pub max_concurrent: AtomicUsize,

    /// Global [DownloadEvent] broadcaster (buffered). Slow subscribers may miss events.
    pub events: broadcast::Sender<DownloadEvent>,
}

impl Context {
    /// Create a new shared Context.
    /// - Initializes the semaphore with `max_concurrent` permits.
    /// - Creates a root [CancellationToken] and a broadcast channel (capacity 1024).
    /// - Constructs a shared [reqwest::Client].
    pub fn new(max_concurrent: usize) -> Arc<Self> {
        let (events, _) = broadcast::channel(1024);
        Arc::new(Self {
            semaphore: Arc::new(Semaphore::new(max_concurrent)),
            max_concurrent: AtomicUsize::new(max_concurrent),
            cancel_root: CancellationToken::new(),
            active: AtomicUsize::new(0),
            id_counter: AtomicU64::new(1),
            client: Client::new(),
            events,
        })
    }

    /// Atomically generate the next [DownloadID] (relaxed ordering).
    /// Unique within the lifetime of this Context; starts at 1.
    #[inline]
    pub fn next_id(&self) -> DownloadID {
        self.id_counter.fetch_add(1, Ordering::Relaxed)
    }

    /// Create a child [CancellationToken] tied to the manager's root token.
    /// Cancelling the root cascades to all children.
    #[inline]
    pub fn child_token(&self) -> CancellationToken {
        self.cancel_root.child_token()
    }

    /// Cancel the root token, cooperatively cancelling all in-flight downloads.
    pub fn cancel_all(&self) {
        self.cancel_root.cancel();
    }
}
