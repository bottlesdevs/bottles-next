use reqwest::Client;
use std::sync::{
    Arc,
    atomic::{AtomicU64, AtomicUsize, Ordering},
};
use tokio::sync::Semaphore;
use tokio_util::sync::CancellationToken;

pub type DownloadID = u64;

#[derive(Debug)]
pub(crate) struct Context {
    pub semaphore: Arc<Semaphore>,
    pub cancel_root: CancellationToken,
    pub client: Client,

    // Counters
    pub id_counter: AtomicU64,
    pub active: AtomicUsize,
    pub max_concurrent: AtomicUsize,
}

impl Context {
    pub fn new(max_concurrent: usize) -> Arc<Self> {
        Arc::new(Self {
            semaphore: Arc::new(Semaphore::new(max_concurrent)),
            max_concurrent: AtomicUsize::new(max_concurrent),
            cancel_root: CancellationToken::new(),
            active: AtomicUsize::new(0),
            id_counter: AtomicU64::new(1),
            client: Client::new(),
        })
    }

    #[inline]
    pub fn next_id(&self) -> DownloadID {
        self.id_counter.fetch_add(1, Ordering::Relaxed)
    }

    #[inline]
    pub fn child_token(&self) -> CancellationToken {
        self.cancel_root.child_token()
    }

    pub fn cancel_all(&self) {
        self.cancel_root.cancel();
    }
}
