use std::{
    collections::{HashMap, VecDeque},
    sync::{Arc, atomic::Ordering},
    time::Duration,
};

use futures_util::StreamExt;
use tokio::sync::{mpsc, oneshot};
use tokio_util::{sync::CancellationToken, task::TaskTracker, time::DelayQueue};

use crate::{
    DownloadError, DownloadID, DownloadResult, Event, Request,
    context::Context,
    worker::{WorkerMsg, run},
};

pub struct ExponentialBackoff {
    pub base_delay: Duration,
    pub max_delay: Duration,
}

impl ExponentialBackoff {
    pub fn next_delay(&self, attempt: u32) -> Duration {
        let factor = 2f64.powi(attempt as i32);
        let delay = self.base_delay.mul_f64(factor);
        delay.min(self.max_delay)
    }
}

static BACKOFF_STRATEGY: ExponentialBackoff = ExponentialBackoff {
    base_delay: Duration::from_secs(1),
    max_delay: Duration::from_secs(10),
};

pub(crate) enum SchedulerCmd {
    Enqueue {
        request: Request,
        result_tx: oneshot::Sender<Result<DownloadResult, DownloadError>>,
    },
    Cancel {
        id: DownloadID,
    },
}

pub(crate) struct Scheduler {
    ctx: Arc<Context>,
    tracker: TaskTracker,
    shutdown_token: CancellationToken,

    cmd_rx: mpsc::Receiver<SchedulerCmd>,
    worker_tx: mpsc::Sender<WorkerMsg>,
    worker_rx: mpsc::Receiver<WorkerMsg>,

    jobs: HashMap<DownloadID, Job>,
    ready: VecDeque<DownloadID>,
    delayed: DelayQueue<DownloadID>,
}

impl Scheduler {
    pub fn new(
        shutdown_token: CancellationToken,
        ctx: Arc<Context>,
        tracker: TaskTracker,
        cmd_rx: mpsc::Receiver<SchedulerCmd>,
    ) -> Self {
        let (worker_tx, worker_rx) = mpsc::channel(1024);
        Self {
            ctx,
            tracker,
            shutdown_token,
            cmd_rx,
            worker_tx,
            worker_rx,
            ready: VecDeque::new(),
            delayed: DelayQueue::new(),
            jobs: HashMap::new(),
        }
    }

    fn schedule(&mut self, job: Job) {
        let request = &job.request;
        let id = job.id();
        request.emit(Event::Queued {
            id,
            url: request.url().clone(),
            destination: request.destination().to_path_buf(),
        });
        self.jobs.insert(id, job);
        self.ready.push_back(id);
    }

    pub async fn run(mut self) {
        loop {
            tokio::select! {
                Some(cmd) = self.cmd_rx.recv() => self.handle_cmd(cmd).await,
                Some(msg) = self.worker_rx.recv() =>self.handle_worker_msg(msg).await,
                expired = self.delayed.next(), if !self.delayed.is_empty() => {
                    if let Some(exp) = expired {
                        self.ready.push_back(exp.into_inner());
                    }
                }
                _ = self.shutdown_token.cancelled() => break,
            }
            self.try_dispatch();
        }
        self.jobs.drain().for_each(|(_, job)| job.cancel());
    }

    async fn handle_worker_msg(&mut self, msg: WorkerMsg) {
        match msg {
            WorkerMsg::Finish { id, result } => match result {
                Ok(result) => {
                    let Some(job) = self.jobs.remove(&id) else {
                        return;
                    };
                    job.finish(result)
                }
                Err(DownloadError::Cancelled) => {
                    let Some(job) = self.jobs.remove(&id) else {
                        return;
                    };
                    job.cancel()
                }
                Err(error) if error.is_retryable() => {
                    let Some(job) = self.jobs.get_mut(&id) else {
                        return;
                    };
                    if job.attempt >= job.request.config().retries() {
                        self.jobs.remove(&id).map(|job| job.fail(error));
                        return;
                    }
                    let delay = BACKOFF_STRATEGY.next_delay(job.attempt);
                    job.attempt += 1;
                    job.retry(delay);
                    self.delayed.insert(id, delay);
                }
                Err(error) => {
                    let Some(entry) = self.jobs.remove(&id) else {
                        return;
                    };
                    entry.fail(error)
                }
            },
        }
    }

    async fn handle_cmd(&mut self, cmd: SchedulerCmd) {
        match cmd {
            SchedulerCmd::Enqueue { request, result_tx } => {
                self.schedule(Job {
                    request: Arc::new(request),
                    result: Some(result_tx),
                    attempt: 0,
                });
            }
            SchedulerCmd::Cancel { id } => {
                self.jobs.remove(&id).map(|job| job.cancel());
            }
        }
    }

    fn try_dispatch(&mut self) {
        struct ActiveGuard {
            ctx: Arc<Context>,
            _permit: tokio::sync::OwnedSemaphorePermit,
        }

        impl ActiveGuard {
            fn new(ctx: Arc<Context>, permit: tokio::sync::OwnedSemaphorePermit) -> Self {
                ctx.active.fetch_add(1, Ordering::Relaxed);
                Self {
                    ctx,
                    _permit: permit,
                }
            }
        }

        impl Drop for ActiveGuard {
            fn drop(&mut self) {
                self.ctx.active.fetch_sub(1, Ordering::Relaxed);
            }
        }

        while let Some(id) = self.ready.pop_front() {
            if self.shutdown_token.is_cancelled() {
                return;
            }
            let permit = match self.ctx.semaphore.clone().try_acquire_owned() {
                Ok(p) => p,
                Err(_) => {
                    // No permits left; put the job back to the front and stop dispatching for now.
                    self.ready.push_front(id);
                    return;
                }
            };

            let Some(entry) = self.jobs.get_mut(&id) else {
                drop(permit);
                continue;
            };

            let request = entry.request.clone();
            let ctx = self.ctx.clone();
            let worker_tx = self.worker_tx.clone();

            self.tracker.spawn(async move {
                let _guard = ActiveGuard::new(ctx.clone(), permit);
                run(request, ctx, worker_tx).await;
            });
        }
    }
}

pub(crate) struct Job {
    request: Arc<Request>,
    attempt: u32,
    result: Option<oneshot::Sender<Result<DownloadResult, DownloadError>>>,
}

impl Job {
    fn id(&self) -> DownloadID {
        self.request.id()
    }

    fn send_result(self, result: Result<DownloadResult, DownloadError>) {
        if let Some(result_tx) = self.result {
            let _ = result_tx.send(result);
        }
    }

    fn fail(self, error: DownloadError) {
        self.request.emit(Event::Failed {
            id: self.id(),
            error: error.to_string(),
        });
        self.send_result(Err(error));
    }

    fn finish(self, result: DownloadResult) {
        self.request.emit(Event::Completed {
            id: self.id(),
            path: result.path.clone(),
            bytes_downloaded: result.bytes_downloaded,
        });
        self.send_result(Ok(result))
    }

    fn retry(&self, delay: Duration) {
        self.request.emit(Event::Retrying {
            id: self.id(),
            attempt: self.attempt,
            next_delay_ms: delay.as_millis() as u64,
        });
    }

    fn cancel(self) {
        self.request.cancel_token.cancel();
        self.request.emit(Event::Cancelled { id: self.id() });
        self.send_result(Err(DownloadError::Cancelled))
    }
}
